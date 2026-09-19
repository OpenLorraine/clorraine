//! Statement checking for the ownership stage.
//!
//! Every block opens a scope, a `let` declares its binding after the
//! initializer is walked, and control flow applies the move,
//! initialization, and null-narrowing rules across branches with a
//! conservative join. The same walk lowers each statement into the typed
//! control-flow IR.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    // -- statement checking --------------------------------------------------

    /// Checks a block in a fresh scope.
    pub(super) fn check_block(&mut self, block: &Block) {
        self.push_scope();
        for statement in &block.statements {
            self.check_stmt(statement);
        }
        self.pop_scope();
    }

    /// Checks one statement.
    pub(super) fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(decl) => self.check_let(decl),
            Stmt::Const(decl) => self.check_const(decl),
            Stmt::Expr(expr) => {
                self.eval(expr);
            }
            Stmt::If(decl) => self.check_if(decl),
            Stmt::While(decl) => self.check_while(decl),
            Stmt::DoWhile(decl) => self.check_do_while(decl),
            Stmt::ForC(decl) => self.check_for_c(decl),
            Stmt::ForIn(decl) => self.check_for_in(decl),
            Stmt::Return { value, span } => self.check_return(value.as_ref(), *span),
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Switch(decl) => self.check_switch(decl),
            Stmt::Defer(block) => self.check_defer(block),
            Stmt::Unsafe(block) => {
                self.unsafe_depth += 1;
                self.check_block(block);
                self.unsafe_depth -= 1;
            }
            Stmt::Block(block) => self.check_block(block),
        }
    }

    /// Checks a `let` binding and plans its initialization.
    pub(super) fn check_let(&mut self, decl: &LetStmt) {
        let declared = decl.ty.as_ref().map(|ty| self.resolve(ty));
        if let Some(ty) = &declared {
            self.check_nullable_decl(ty, decl.span);
        }
        let value_ty = decl.value.as_ref().map(|value| self.eval(value));
        let target = declared
            .clone()
            .or(value_ty)
            .unwrap_or_else(Ty::unknown);
        let id = self.declare_local(&decl.name, target.clone(), false);
        if let Some(value) = &decl.value {
            let init = PlaceState {
                initialized: true,
                ..PlaceState::default()
            };
            self.set_place(id, init);
            self.check_value_into(value, &target, decl.span);
            self.bind_value(id, &target, value, decl.span);
        }
        self.emit(Statement::Nop);
    }

    /// Checks a `const` binding.
    pub(super) fn check_const(&mut self, decl: &ConstStmt) {
        let declared = decl.ty.as_ref().map(|ty| self.resolve(ty));
        if let Some(ty) = &declared {
            self.check_nullable_decl(ty, decl.span);
        }
        let value_ty = self.eval(&decl.value);
        let target = declared.or(Some(value_ty)).unwrap_or_else(Ty::unknown);
        let id = self.declare_local(&decl.name, target.clone(), true);
        self.set_place(
            id,
            PlaceState {
                initialized: true,
                ..PlaceState::default()
            },
        );
        self.check_value_into(&decl.value, &target, decl.span);
        self.bind_value(id, &target, &decl.value, decl.span);
    }

    /// Checks a binding's initializer against nullability and moves or loans.
    pub(super) fn bind_value(&mut self, id: usize, target: &Ty, value: &Expr, span: Span) {
        if is_view(target) {
            let provenance = self.view_provenance(value);
            if let Provenance::Local(owner) = provenance {
                let owner_scope = self.bindings[owner].scope;
                let target_scope = self.bindings[id].scope;
                if target_scope < owner_scope {
                    let name = self.bindings[owner].name.clone();
                    self.error(
                        "E10030",
                        format!("borrow of '{name}' does not live long enough"),
                        "finish the use before the owner's scope ends",
                        span,
                    );
                } else {
                    self.state
                        .loans
                        .add(owner, LoanKind::Shared, target_scope, span);
                }
            }
            self.bindings[id].provenance = provenance;
            return;
        }
        if self.sema.table.duplicable(target) {
            return;
        }
        let Some(target_place) = self.place_target(value) else {
            return;
        };
        self.move_target(target_place, span);
    }

    /// Checks an `if`, applying null narrowing to each branch.
    pub(super) fn check_if(&mut self, decl: &IfStmt) {
        self.eval(&decl.condition);
        let narrow = self.narrowing(&decl.condition);
        let before = self.state.clone();

        let then_block = self.new_block();
        let else_block = self.new_block();
        let join = self.new_block();
        let condition = self.lower_value(&decl.condition);
        self.terminate(Terminator::Branch {
            condition,
            on_true: then_block,
            on_false: else_block,
        });

        self.switch_to(then_block);
        if let Some((id, non_null)) = narrow {
            self.with_place(id, |place| place.narrowed = non_null);
        }
        self.check_block(&decl.then_block);
        let then_state = self.state.clone();
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(join));
        }

        self.state = before;
        self.switch_to(else_block);
        if let Some((id, non_null)) = narrow {
            self.with_place(id, |place| place.narrowed = !non_null);
        }
        match &decl.else_branch {
            Some(ElseBranch::If(nested)) => self.check_if(nested),
            Some(ElseBranch::Block(block)) => self.check_block(block),
            None => {}
        }
        let else_state = self.state.clone();
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(join));
        }

        self.state = Self::merge(then_state, else_state);
        self.switch_to(join);
    }

    /// Checks a `while` loop conservatively: the body may not run.
    pub(super) fn check_while(&mut self, decl: &crate::ast::WhileStmt) {
        let head = self.new_block();
        self.terminate(Terminator::Goto(head));
        self.switch_to(head);
        self.eval(&decl.condition);
        let body_block = self.new_block();
        let exit = self.new_block();
        let condition = self.lower_value(&decl.condition);
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
        });
        let before = self.state.clone();
        self.switch_to(body_block);
        self.check_block(&decl.body);
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(head));
        }
        let body_state = self.state.clone();
        self.state = Self::merge_loop(before, body_state);
        self.switch_to(exit);
    }

    /// Checks a `do ... while` loop, whose body runs at least once.
    pub(super) fn check_do_while(&mut self, decl: &crate::ast::DoWhileStmt) {
        let body_block = self.new_block();
        self.terminate(Terminator::Goto(body_block));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        self.eval(&decl.condition);
        let head = body_block;
        let exit = self.new_block();
        let condition = self.lower_value(&decl.condition);
        self.terminate(Terminator::Branch {
            condition,
            on_true: head,
            on_false: exit,
        });
        self.switch_to(exit);
    }

    /// Checks a three-clause `for` loop.
    pub(super) fn check_for_c(&mut self, decl: &ForCStmt) {
        self.push_scope();
        match &decl.init {
            Some(ForInit::Let(binding)) => self.check_let(binding),
            Some(ForInit::Const(binding)) => self.check_const(binding),
            Some(ForInit::Expr(expressions)) => {
                for expression in expressions {
                    self.eval(expression);
                }
            }
            None => {}
        }
        if let Some(condition) = &decl.condition {
            self.eval(condition);
        }
        for step in &decl.step {
            self.eval(step);
        }
        self.check_block(&decl.body);
        self.pop_scope();
    }

    /// Checks an element loop, requiring `borrow` for non-duplicable elements.
    pub(super) fn check_for_in(&mut self, decl: &ForInStmt) {
        let iterable_ty = self.eval(&decl.iterable);
        let element = match &iterable_ty.kind {
            TyKind::Array { element, .. } => (**element).clone(),
            _ => Ty::unknown(),
        };
        if !decl.borrow && !self.sema.table.duplicable(&element) {
            self.error(
                "E10040",
                format!(
                    "element loop requires 'borrow' for non-duplicable '{}'",
                    element.display()
                ),
                "write `for (borrow e in xs)`",
                decl.span,
            );
        }
        self.push_scope();
        let kind = if decl.borrow {
            BindingKind::OutSlot
        } else {
            BindingKind::Local
        };
        let id = self.declare(
            &decl.name,
            element,
            kind,
            if decl.borrow { Mode::Borrow } else { Mode::None },
            false,
            false,
        );
        self.set_place(
            id,
            PlaceState {
                initialized: true,
                ..PlaceState::default()
            },
        );
        self.check_block(&decl.body);
        self.pop_scope();
    }

    /// Checks a `switch` statement.
    pub(super) fn check_switch(&mut self, decl: &SwitchStmt) {
        self.eval(&decl.scrutinee);
        for clause in &decl.clauses {
            self.push_scope();
            if let SwitchLabel::Case(value) = &clause.label {
                self.eval(value);
            }
            for statement in &clause.statements {
                self.check_stmt(statement);
            }
            self.pop_scope();
        }
    }

    /// Checks a deferred block in its own IR block and scope.
    pub(super) fn check_defer(&mut self, block: &Block) {
        let saved = self.builder.current();
        let defer_block = self.new_block();
        self.emit(Statement::Defer { block: defer_block });
        self.switch_to(defer_block);
        self.check_block(block);
        self.switch_to(saved);
    }

    /// Checks a `return`.
    pub(super) fn check_return(&mut self, value: Option<&Expr>, span: Span) {
        if let Some(value) = value {
            self.eval(value);
            let returns = self.returns.clone();
            self.check_value_into(value, &returns, span);
            if is_view(&returns) {
                let provenance = self.view_provenance(value);
                match provenance {
                    Provenance::Local(id) => {
                        let name = self.bindings[id].name.clone();
                        self.error(
                            "E10035",
                            format!("cannot return a view of local '{name}'"),
                            "return an owned value or a view of a parameter",
                            span,
                        );
                        self.suppress_elision = true;
                    }
                    Provenance::Static => self.suppress_elision = true,
                    _ => {}
                }
            }
            let lowered = self.lower_value(value);
            self.emit_return_cleanup();
            self.terminate(Terminator::Return(Some(lowered)));
        } else {
            self.emit_return_cleanup();
            self.terminate(Terminator::Return(None));
        }
    }

}

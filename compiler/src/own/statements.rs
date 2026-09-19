//! Statement checking for the ownership stage.
//!
//! Every block opens a scope, a `let` declares its binding after the
//! initializer is walked, and control flow applies the move,
//! initialization, and null-narrowing rules across branches with a
//! conservative join. Loops are analyzed to a fixed point so a value moved in
//! one iteration is seen as moved in the next. The same walk lowers each
//! statement into the typed control-flow IR.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    // -- statement checking --------------------------------------------------

    /// Checks a block in a fresh scope.
    ///
    /// Statements after a terminator are unreachable, so the walk stops; the
    /// lowering would drop them anyway, and analyzing them could report on
    /// code no path can reach.
    pub(super) fn check_block(&mut self, block: &'a Block) {
        self.push_scope();
        for statement in &block.statements {
            if self.is_terminated() {
                break;
            }
            self.check_stmt(statement);
        }
        self.pop_scope();
    }

    /// Checks one statement.
    pub(super) fn check_stmt(&mut self, stmt: &'a Stmt) {
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
            Stmt::Break(span) => self.check_break(*span),
            Stmt::Continue(span) => self.check_continue(*span),
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
    ///
    /// A view bound from a local or a parameter records a loan of that owner:
    /// a `take` or `borrow` parameter is storage the callee may not move or
    /// write while the derived view lives. A writable view derived from a
    /// read-only loan is `E10041`.
    pub(super) fn bind_value(&mut self, id: usize, target: &Ty, value: &Expr, span: Span) {
        if is_view(target) {
            let provenance = self.view_provenance(value);
            let kind = self.view_loan_kind(value);
            if let Provenance::Local(owner) | Provenance::Param(owner) = &provenance {
                let owner = *owner;
                let target_scope = self.bindings[id].scope;
                if matches!(provenance, Provenance::Local(_)) {
                    let owner_scope = self.bindings[owner].scope;
                    if target_scope < owner_scope {
                        let name = self.bindings[owner].name.clone();
                        self.error(
                            "E10030",
                            format!("borrow of '{name}' does not live long enough"),
                            "finish the use before the owner's scope ends",
                            span,
                        );
                        self.bindings[id].provenance = provenance;
                        return;
                    }
                }
                self.record_view_loan(owner, kind, target_scope, span);
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

    /// Records a view loan, reporting the two ways it can be illegal.
    fn record_view_loan(&mut self, owner: usize, kind: LoanKind, origin: usize, span: Span) {
        let name = self.bindings[owner].name.clone();
        if kind == LoanKind::Exclusive && self.is_readonly_borrow(owner) {
            self.error(
                "E10041",
                format!("cannot write through a view of read-only borrow '{name}'"),
                "take the owner by `borrow`",
                span,
            );
            return;
        }
        if self.state.loans.conflict(owner, kind) {
            self.error(
                "E10032",
                format!("'{name}' is already borrowed"),
                "end the first borrow first",
                span,
            );
            return;
        }
        self.state.loans.add(owner, kind, origin, span);
    }

    /// Checks an `if`, applying null narrowing to each branch.
    ///
    /// A branch that cannot fall through does not contribute to the join: the
    /// state after the `if` is the state of the branches that still reach it.
    pub(super) fn check_if(&mut self, decl: &'a IfStmt) {
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
        if let Some((id, projections, non_null)) = &narrow {
            self.apply_narrowing(*id, projections, *non_null);
        }
        self.check_block(&decl.then_block);
        let then_terminated = self.is_terminated();
        let then_state = self.state.clone();
        if !then_terminated {
            self.terminate(Terminator::Goto(join));
        }

        self.state = before;
        self.switch_to(else_block);
        if let Some((id, projections, non_null)) = &narrow {
            self.apply_narrowing(*id, projections, !*non_null);
        }
        match &decl.else_branch {
            Some(ElseBranch::If(nested)) => self.check_if(nested),
            Some(ElseBranch::Block(block)) => self.check_block(block),
            None => {}
        }
        let else_terminated = self.is_terminated();
        let else_state = self.state.clone();
        if !else_terminated {
            self.terminate(Terminator::Goto(join));
        }

        self.state = match (then_terminated, else_terminated) {
            (true, false) => else_state,
            (false, true) => then_state,
            _ => Self::merge(then_state, else_state),
        };
        self.switch_to(join);
    }

    /// Records that a place path is (or is not) known to be not null.
    fn apply_narrowing(&mut self, id: usize, projections: &[ir::Projection], non_null: bool) {
        let key = projections.to_vec();
        self.with_place(id, |place| {
            if non_null {
                place.narrowed.insert(key);
            } else {
                place.narrowed.remove(&key);
            }
        });
    }

    /// Checks a `while` loop.
    ///
    /// The body may run zero or more times, so the state at the head is the
    /// join of the pre-state and the body's effect, iterated to a fixed point.
    /// `break` leaves through the exit block and `continue` returns to the
    /// head, each running the scopes it leaves.
    pub(super) fn check_while(&mut self, decl: &'a crate::ast::WhileStmt) {
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

        let pre = self.state.clone();
        let floor = self.scopes.len();
        let (entry, post) = self.loop_entry_state(pre.clone(), |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(head),
                floor,
                floor,
            ));
            checker.check_block(&decl.body);
            checker.exits.pop();
        });

        self.state = entry.clone();
        self.exits
            .push(ExitFrame::new(exit, Some(head), floor, floor));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        let body_terminated = self.is_terminated();
        if !body_terminated {
            self.terminate(Terminator::Goto(head));
        }
        let frame = self.exits.pop().expect("loop frame");
        let always_true = condition_is_true(&decl.condition);
        self.state = loop_exit_state(entry, post, body_terminated, always_true, &frame);
        self.switch_to(exit);
    }

    /// Checks a `do ... while` loop, whose body runs at least once.
    pub(super) fn check_do_while(&mut self, decl: &'a crate::ast::DoWhileStmt) {
        let body_block = self.new_block();
        self.terminate(Terminator::Goto(body_block));
        let condition_block = self.new_block();
        let exit = self.new_block();

        let pre = self.state.clone();
        let floor = self.scopes.len();
        let (entry, post) = self.loop_entry_state(pre, |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(condition_block),
                floor,
                floor,
            ));
            checker.check_block(&decl.body);
            if !checker.is_terminated() {
                checker.eval(&decl.condition);
            }
            checker.exits.pop();
        });

        self.state = entry;
        self.exits.push(ExitFrame::new(
            exit,
            Some(condition_block),
            floor,
            floor,
        ));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(condition_block));
        }
        self.switch_to(condition_block);
        self.eval(&decl.condition);
        let condition = self.lower_value(&decl.condition);
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
        });
        let frame = self.exits.pop().expect("loop frame");
        self.state = join_breaks(post, &frame);
        self.switch_to(exit);
    }

    /// Checks a three-clause `for` loop.
    ///
    /// The header is lowered to an init (in the loop's own scope), a head that
    /// tests the condition, a body, and a step block. `continue` runs the step,
    /// so the entry state joins the pre-state, which covers the zero-iteration
    /// case.
    pub(super) fn check_for_c(&mut self, decl: &'a ForCStmt) {
        let floor = self.scopes.len();
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
        let head = self.new_block();
        self.terminate(Terminator::Goto(head));
        self.switch_to(head);
        if let Some(condition) = &decl.condition {
            self.eval(condition);
        }
        let body_block = self.new_block();
        let step_block = self.new_block();
        let exit = self.new_block();
        let condition = match &decl.condition {
            Some(condition) => self.lower_value(condition),
            None => Value::Bool(true),
        };
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
        });

        let pre = self.state.clone();
        let continue_floor = floor + 1;
        let (entry, post) = self.loop_entry_state(pre.clone(), |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(step_block),
                floor,
                continue_floor,
            ));
            checker.check_block(&decl.body);
            if !checker.is_terminated() {
                for step in &decl.step {
                    checker.eval(step);
                }
            }
            checker.exits.pop();
        });

        self.state = entry.clone();
        self.exits.push(ExitFrame::new(
            exit,
            Some(step_block),
            floor,
            continue_floor,
        ));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        let body_terminated = self.is_terminated();
        if !body_terminated {
            self.terminate(Terminator::Goto(step_block));
        }
        self.switch_to(step_block);
        for step in &decl.step {
            self.eval(step);
        }
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(head));
        }
        let frame = self.exits.pop().expect("loop frame");
        let always_true = decl
            .condition
            .as_ref()
            .map_or(true, condition_is_true);
        self.state = loop_exit_state(entry, post, body_terminated, always_true, &frame);
        self.switch_to(exit);
        self.pop_scope();
    }

    /// Declares the loop element in its own scope and returns its binding.
    ///
    /// The element is bound fresh on every iteration, so the declaration and
    /// its initial state live inside the loop's speculative body and are re-run
    /// on each fixpoint pass.
    fn declare_for_in_element(&mut self, decl: &'a ForInStmt, element: &Ty) -> usize {
        self.push_scope();
        let kind = if decl.borrow {
            BindingKind::OutSlot
        } else {
            BindingKind::Local
        };
        let id = self.declare(
            &decl.name,
            element.clone(),
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
        id
    }

    /// Checks an element loop, requiring `borrow` for non-duplicable elements.
    ///
    /// The element binding and the body are analyzed to a fixed point just like
    /// the other loops, so a value moved in one iteration is seen as moved in
    /// the next. The loop may run zero times, so the head also joins the
    /// pre-loop state.
    pub(super) fn check_for_in(&mut self, decl: &'a ForInStmt) {
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
        let pre = self.state.clone();
        let floor = self.scopes.len();
        let body_block = self.new_block();
        let exit = self.new_block();
        self.terminate(Terminator::Goto(body_block));

        let (entry, _post) = self.loop_entry_state(pre.clone(), |checker| {
            checker.declare_for_in_element(decl, &element);
            checker
                .exits
                .push(ExitFrame::new(exit, Some(body_block), floor, floor));
            checker.switch_to(body_block);
            checker.check_block(&decl.body);
            checker.exits.pop();
            checker.pop_scope();
        });

        self.state = entry.clone();
        self.declare_for_in_element(decl, &element);
        self.exits
            .push(ExitFrame::new(exit, Some(body_block), floor, floor));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(body_block));
        }
        let frame = self.exits.pop().expect("loop frame");
        self.pop_scope();
        self.state = join_breaks(entry, &frame);
        self.switch_to(exit);
    }

    /// Checks a `switch` statement.
    ///
    /// Each clause starts from the join of the pre-switch state and the
    /// previous clause's fall-through, so a value assigned only on one case
    /// does not count as assigned after the switch. A missing default adds the
    /// pre-switch state as an exit path.
    pub(super) fn check_switch(&mut self, decl: &'a SwitchStmt) {
        self.eval(&decl.scrutinee);
        let scrutinee = self.lower_value(&decl.scrutinee);
        let pre = self.state.clone();
        let floor = self.scopes.len();
        let exit = self.new_block();

        let mut arms: Vec<(i128, BlockId)> = Vec::new();
        let mut clause_blocks: Vec<BlockId> = Vec::new();
        let mut default_block = exit;
        let mut has_default = false;
        for clause in &decl.clauses {
            let block = self.new_block();
            clause_blocks.push(block);
            match &clause.label {
                SwitchLabel::Case(value) => {
                    if let Some(literal) = int_literal_value(value) {
                        arms.push((literal, block));
                    }
                }
                SwitchLabel::Default => {
                    default_block = block;
                    has_default = true;
                }
            }
        }
        self.terminate(Terminator::Switch {
            value: scrutinee,
            arms,
            default: default_block,
        });

        let mut exit_states: Vec<FlowState> = Vec::new();
        if !has_default {
            exit_states.push(pre.clone());
        }
        let mut fallthrough: Option<FlowState> = None;
        for (index, clause) in decl.clauses.iter().enumerate() {
            let mut entry = pre.clone();
            if let Some(previous) = &fallthrough {
                entry = Self::merge(entry, previous.clone());
            }
            self.state = entry;
            self.switch_to(clause_blocks[index]);
            self.exits.push(ExitFrame::new(exit, None, floor, floor));
            self.push_scope();
            if let SwitchLabel::Case(value) = &clause.label {
                self.eval(value);
            }
            for statement in &clause.statements {
                if self.is_terminated() {
                    break;
                }
                self.check_stmt(statement);
            }
            self.pop_scope();
            let terminated = self.is_terminated();
            let frame = self.exits.pop().expect("switch frame");
            if frame.saw_break {
                exit_states.push(frame.break_state);
            }
            if !terminated {
                let target = clause_blocks.get(index + 1).copied().unwrap_or(exit);
                self.terminate(Terminator::Goto(target));
                // A clause with no following clause falls straight out of the
                // switch, so its state is an exit path and must join the code
                // after the switch.
                if target == exit {
                    exit_states.push(self.state.clone());
                }
                fallthrough = Some(self.state.clone());
            } else {
                fallthrough = None;
            }
        }

        self.state = exit_states
            .into_iter()
            .reduce(Self::merge)
            .unwrap_or_else(|| pre.clone());
        self.switch_to(exit);
    }

    /// Checks a deferred block.
    ///
    /// The block's IR is lowered at registration, but its ownership effects
    /// are checked on each scope exit edge (by `run_defers_from`) and restored
    /// here, so they do not leak into the enclosing flow.
    pub(super) fn check_defer(&mut self, block: &'a Block) {
        let saved_block = self.builder.current();
        let defer_block = self.new_block();
        self.emit(Statement::Defer { block: defer_block });

        let saved_state = self.state.clone();
        let saved_bindings = self.bindings.len();
        let saved_scopes = self.scopes.len();
        let saved_defers = self.defer_entries.len();
        let saved_report = self.report_diagnostics;
        self.report_diagnostics = false;
        self.switch_to(defer_block);
        self.check_block(block);
        self.report_diagnostics = saved_report;
        self.state = saved_state;
        self.bindings.truncate(saved_bindings);
        self.scopes.truncate(saved_scopes);
        self.defer_entries.truncate(saved_defers);
        self.switch_to(saved_block);

        self.defer_entries.push(DeferEntry {
            depth: self.scopes.len(),
            block,
        });
    }

    /// Checks a `break`, recording the edge and running the scopes it leaves.
    pub(super) fn check_break(&mut self, _span: Span) {
        let state = self.state.clone();
        let Some(frame) = self.exits.last_mut() else {
            return;
        };
        let break_block = frame.break_block;
        let floor = frame.scope_floor;
        frame.record_break(state);
        self.emit_exit_cleanup(floor);
        self.terminate(Terminator::Goto(break_block));
    }

    /// Checks a `continue`, recording the edge and running the scopes it
    /// leaves on the way back to the loop head.
    pub(super) fn check_continue(&mut self, _span: Span) {
        let found = self
            .exits
            .iter()
            .rev()
            .find_map(|frame| frame.continue_block.map(|block| (block, frame.continue_floor)));
        let Some((continue_block, floor)) = found else {
            return;
        };
        self.emit_exit_cleanup(floor);
        self.terminate(Terminator::Goto(continue_block));
    }

    /// Emits the defers and drops for the scopes deeper than `floor`.
    fn emit_exit_cleanup(&mut self, floor: usize) {
        self.run_defers_from(floor + 1);
        let depth = self.scopes.len();
        for index in (floor..depth).rev() {
            let ids: Vec<usize> = self.scopes[index]
                .order
                .iter()
                .rev()
                .copied()
                .collect();
            for id in ids {
                if self.bindings[id].needs_drop {
                    let place = self.binding_place(id);
                    self.emit(Statement::Drop(place));
                }
            }
        }
    }

    /// Checks a `return`.
    ///
    /// A non-duplicable return value is an ownership transfer, so it runs the
    /// move rules: returning a lend parameter is `E10034`, and returning a
    /// moved value is `E10020`.
    pub(super) fn check_return(&mut self, value: Option<&Expr>, span: Span) {
        if let Some(value) = value {
            self.eval(value);
            let returns = self.returns.clone();
            self.check_value_into(value, &returns, span);
            if !self.sema.table.duplicable(&returns) && !returns.is_unknown() {
                if let Some(target) = self.place_target(value) {
                    self.move_target(target, span);
                }
            }
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

/// Joins the loop-entry state with the state of every `break` edge.
fn join_breaks(entry: FlowState, frame: &ExitFrame) -> FlowState {
    if frame.saw_break {
        FunctionChecker::merge(entry, frame.break_state.clone())
    } else {
        entry
    }
}

/// True when a loop condition is the literal `true`.
///
/// Such a loop can only leave through a `break` or a `return`, never through a
/// false condition, so its condition edge does not reach the code after it.
fn condition_is_true(condition: &Expr) -> bool {
    matches!(condition.kind, ExprKind::Bool(true))
}

/// The state that reaches the code after a loop.
///
/// Normally the exit joins the loop-entry state with every `break` edge,
/// because the condition may be false before the body runs. When the body
/// always terminates and the condition is the literal `true`, the loop cannot
/// leave through its condition, so the only reachable exit is the body's
/// terminator, whose state is `post`.
fn loop_exit_state(
    entry: FlowState,
    post: FlowState,
    body_terminated: bool,
    condition_always_true: bool,
    frame: &ExitFrame,
) -> FlowState {
    if body_terminated && condition_always_true && frame.saw_break {
        post
    } else {
        join_breaks(entry, frame)
    }
}

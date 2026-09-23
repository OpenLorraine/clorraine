//! Statement checking for the ownership stage.
//!
//! Every block opens a scope, a `let` declares its binding after the
//! initializer is walked, and control flow applies the move,
//! initialization, and null-narrowing rules across branches with a
//! conservative join. The same walk lowers each statement into the typed
//! control-flow IR; the loops live in the sibling `loops` module.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    // -- statement checking --------------------------------------------------

    /// Checks a block in a fresh scope.
    ///
    /// Statements after a terminator are unreachable, so the walk stops; the
    /// lowering would drop them anyway, and analyzing them could report on
    /// code no path can reach. A block that falls through cleanly ends with a
    /// scope-end marker; a block that leaves through `return`, `break`, or
    /// `continue` already carries its cleanup on that edge, so no marker is
    /// emitted.
    pub(super) fn check_block(&mut self, block: &'a Block) {
        let scope = self.push_scope(block.span);
        for statement in &block.statements {
            if self.is_terminated() {
                // The walk stopped at a terminator, so this statement and
                // every one after it is unreachable. One warning names the
                // first such statement.
                self.warn("W10003", "unreachable code", stmt_span(statement));
                break;
            }
            self.check_stmt(statement);
        }
        if !self.is_terminated() {
            self.emit(Statement::ScopeEnd { scope });
        }
        self.pop_scope();
    }

    /// Checks one statement.
    pub(super) fn check_stmt(&mut self, stmt: &'a Stmt) {
        self.current_span = stmt_span(stmt);
        match stmt {
            Stmt::Let(decl) => self.check_let(decl),
            Stmt::Const(decl) => self.check_const(decl),
            Stmt::Expr(expr) => self.check_expr_statement(expr),
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

    /// Checks an expression used as a statement and emits its run-time effect.
    ///
    /// An assignment emitted its store while it was walked; an increment or
    /// decrement emits the same read-modify-write as the equivalent compound
    /// assignment; every other effectful expression lowers to a discarded call.
    /// This is the statement path, so a `j++` here increments `j` even though
    /// the same expression in value position yields the old value.
    pub(super) fn check_expr_statement(&mut self, expr: &'a Expr) {
        self.eval(expr);
        match &expr.kind {
            ExprKind::Assign { .. } => {}
            ExprKind::Unary { op, operand } if op.is_update() => {
                let op = match op {
                    UnaryOp::PreIncrement | UnaryOp::PostIncrement => AssignOp::Add,
                    _ => AssignOp::Sub,
                };
                self.check_update(op, operand);
            }
            _ => self.emit_expr_statement(expr),
        }
    }

    /// Lowers an expression statement whose result is discarded.
    ///
    /// A bare call and the `assert`/`panic` intrinsics lower to a discarded
    /// call. Any other value expression still has to run an effect nested inside
    /// it — the call in `f() + 1;` or the update in `j++ + 1;` — so it lowers to
    /// a discarded value, which is skipped when the value is pure. The callee
    /// keeps its simple name, so the emitter resolves the C symbol from the
    /// collected tables at render time.
    fn emit_expr_statement(&mut self, expr: &'a Expr) {
        match &expr.kind {
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                let call = self.lower_call(callee, type_args, args);
                let move_outs = self.take_move_out_places(&call.name, &call.args);
                self.emit(Statement::Call {
                    callee: call.name,
                    type_args: call.type_args,
                    args: call.args,
                    receiver: call.receiver.map(Box::new),
                    result: None,
                });
                for place in move_outs {
                    self.emit(Statement::MoveOut { place });
                }
            }
            ExprKind::IntrinsicCall { name, args } if name == "assert" || name == "panic" => {
                let name = name.clone();
                let args = args
                    .iter()
                    .filter_map(|arg| match arg {
                        IntrinsicArg::Expr(inner) => Some(self.lower_value(inner)),
                        IntrinsicArg::Type(_) => None,
                    })
                    .collect();
                self.emit(Statement::Call {
                    callee: name,
                    type_args: Vec::new(),
                    args,
                    receiver: None,
                    result: None,
                });
            }
            _ => self.lower_discarded(expr),
        }
    }

    /// Checks a `let` binding and plans its initialization.
    pub(super) fn check_let(&mut self, decl: &LetStmt) {
        self.current_span = decl.span;
        let declared = decl.ty.as_ref().map(|ty| self.resolve(ty));
        if let Some(ty) = &declared {
            self.check_nullable_decl(ty, decl.span);
        }
        let value_ty = decl.value.as_ref().map(|value| self.eval(value));
        let target = declared
            .clone()
            .or(value_ty)
            .unwrap_or_else(Ty::unknown);
        let id = self.declare_local(&decl.name, target.clone(), false, decl.name_span);
        if let Some(value) = &decl.value {
            let init = PlaceState {
                initialized: true,
                ..PlaceState::default()
            };
            self.set_place(id, init);
            self.check_value_into(value, &target, decl.span);
            self.bind_value(id, &target, value, decl.span);
            let place = self.binding_place(id);
            self.emit_store(place, &target, value);
        } else {
            self.emit(Statement::Nop);
        }
    }

    /// Emits the IR statement that stores an initializer or assignment value.
    ///
    /// A non-duplicable value that already names a place is an ownership
    /// transfer, so it lowers to a move that copies and then zeroes the
    /// source. Every other value lowers to an assign of the lowered value,
    /// which covers duplicable copies and fresh call results.
    pub(super) fn emit_store(&mut self, target: Place, ty: &Ty, value: &Expr) {
        if !self.sema.table.duplicable(ty) {
            if let Some(source) = self.place_target(value) {
                let source = self.target_place(&source);
                self.emit(Statement::Move { target, source });
                return;
            }
        }
        let lowered = self.lower_value(value);
        self.emit(Statement::Assign { target, value: lowered });
    }

    /// Checks a `const` binding.
    pub(super) fn check_const(&mut self, decl: &ConstStmt) {
        self.current_span = decl.span;
        let declared = decl.ty.as_ref().map(|ty| self.resolve(ty));
        if let Some(ty) = &declared {
            self.check_nullable_decl(ty, decl.span);
        }
        let value_ty = self.eval(&decl.value);
        let target = declared.or(Some(value_ty)).unwrap_or_else(Ty::unknown);
        let id = self.declare_local(&decl.name, target.clone(), true, decl.name_span);
        self.set_place(
            id,
            PlaceState {
                initialized: true,
                ..PlaceState::default()
            },
        );
        self.check_value_into(&decl.value, &target, decl.span);
        self.bind_value(id, &target, &decl.value, decl.span);
        let place = self.binding_place(id);
        self.emit_store(place, &target, &decl.value);
    }

    /// Reports W10000 for every declared local that was never used.
    ///
    /// A local counts as used once it is read, moved, or written; only the
    /// declaration's own initializer store does not mark it, so `let x = f();`
    /// with no later use warns while a value written through by index does not.
    /// Parameters, out-param slots, globals, and temporaries are outside the
    /// rule, and the warning is suppressed during a speculative pass that only
    /// computes flow state.
    pub(super) fn report_unused_locals(&mut self) {
        let unused: Vec<(String, Span)> = self
            .bindings
            .iter()
            .filter(|binding| binding.kind == BindingKind::Local && !binding.used)
            .map(|binding| (binding.name.clone(), binding.span))
            .collect();
        for (name, span) in unused {
            self.warn("W10000", format!("unused local '{name}'"), span);
        }
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
            shape: BranchShape::If { join },
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

        let mut arms: Vec<(Value, BlockId)> = Vec::new();
        let mut clause_blocks: Vec<BlockId> = Vec::new();
        let mut default_block = exit;
        let mut has_default = false;
        for clause in &decl.clauses {
            let block = self.new_block();
            clause_blocks.push(block);
            match &clause.label {
                SwitchLabel::Case(value) => {
                    // Keep the written label as a value so an enum member is
                    // rendered with its `E.V` spelling rather than folded.
                    let label = self.lower_value(value);
                    arms.push((label, block));
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
            join: exit,
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
            self.push_scope(decl.span);
            if let SwitchLabel::Case(value) = &clause.label {
                self.eval(value);
            }
            for statement in &clause.statements {
                if self.is_terminated() {
                    // The clause body already left through a terminator, so
                    // this statement cannot be reached.
                    self.warn("W10003", "unreachable code", stmt_span(statement));
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
                // A non-empty body that can complete normally falls into the
                // next label; a stacked empty label does not warn.
                let has_next_label = index + 1 < decl.clauses.len();
                if has_next_label && !clause.statements.is_empty() {
                    self.warn(
                        "W10002",
                        "case falls through to the next label",
                        clause.span,
                    );
                }
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
        let scope = self.current_scope();
        let defer_block = self.new_block();
        let registration = self.register_cleanup();
        self.emit(Statement::Defer {
            block: defer_block,
            scope,
            order: registration,
        });

        let saved_state = self.state.clone();
        let saved_bindings = self.bindings.len();
        let saved_scopes = self.scopes.len();
        let saved_defers = self.defer_entries.len();
        let saved_report = self.report_diagnostics;
        let saved_owned = self.materialize_owned_temporaries;
        self.report_diagnostics = false;
        self.materialize_owned_temporaries = false;
        self.switch_to(defer_block);
        self.check_block(block);
        self.report_diagnostics = saved_report;
        self.materialize_owned_temporaries = saved_owned;
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
    ///
    /// A `for (borrow e in xs)` loop being broken writes its element back before
    /// the jump, so the last write through the element view is not lost.
    pub(super) fn check_break(&mut self, _span: Span) {
        let state = self.state.clone();
        let Some(frame) = self.exits.last_mut() else {
            return;
        };
        let break_block = frame.break_block;
        let floor = frame.scope_floor;
        let write = frame.for_in.clone();
        frame.record_break(state);
        self.run_for_in_write(write);
        self.emit_exit_cleanup(floor);
        self.terminate(Terminator::Goto(break_block));
    }

    /// Checks a `continue`, recording the edge and running the scopes it
    /// leaves on the way back to the loop head.
    ///
    /// As with `break`, a `borrow` element loop writes its element back before
    /// continuing so the iteration's write reaches the iterable.
    pub(super) fn check_continue(&mut self, _span: Span) {
        let found = self.exits.iter().rev().find_map(|frame| {
            frame
                .continue_block
                .map(|block| (block, frame.continue_floor, frame.for_in.clone()))
        });
        let Some((continue_block, floor, write)) = found else {
            return;
        };
        self.run_for_in_write(write);
        self.emit_exit_cleanup(floor);
        self.terminate(Terminator::Goto(continue_block));
    }

    /// Runs a recorded element write-back, when there is one.
    fn run_for_in_write(&mut self, write: Option<ForInWrite>) {
        let Some(write) = write else {
            return;
        };
        self.store_for_in_element(&write.element, &write.iterable, &write.index, write.id);
    }

    /// Emits the defers and drops for the scopes deeper than `floor`.
    ///
    /// A `ScopeExit` marker precedes each scope's drops, so the emitter routes
    /// the edge through that scope's cleanup even when the scope owns only a
    /// `defer`.
    fn emit_exit_cleanup(&mut self, floor: usize) {
        self.run_defers_from(floor + 1);
        let depth = self.scopes.len();
        for index in (floor..depth).rev() {
            let scope_id = self.scopes[index].id;
            self.emit(Statement::ScopeExit { scope: scope_id });
            let ids: Vec<usize> = self.scopes[index]
                .order
                .iter()
                .rev()
                .copied()
                .collect();
            for id in ids {
                if self.bindings[id].needs_drop {
                    let place = self.binding_place(id);
                    let order = self.bindings[id].registration;
                    self.emit(Statement::Drop {
                        place,
                        scope: scope_id,
                        order,
                    });
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


/// The source span of a statement, used for the IR's `#line` facts.
fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Let(decl) => decl.span,
        Stmt::Const(decl) => decl.span,
        Stmt::Expr(expr) => expr.span,
        Stmt::If(decl) => decl.span,
        Stmt::While(decl) => decl.span,
        Stmt::DoWhile(decl) => decl.span,
        Stmt::ForC(decl) => decl.span,
        Stmt::ForIn(decl) => decl.span,
        Stmt::Return { span, .. } => *span,
        Stmt::Break(span) => *span,
        Stmt::Continue(span) => *span,
        Stmt::Switch(decl) => decl.span,
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => block.span,
    }
}

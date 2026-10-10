//! Statement checking for the ownership stage.
//!
//! Every block opens a scope, a `let` declares its binding after the
//! initializer is walked, and control flow applies the move,
//! initialization, and null-narrowing rules across branches with a
//! conservative join. The same walk lowers each statement into the typed
//! control-flow IR; the loops live in the sibling `loops` module.

use super::*;

/// The two IR shapes a value store can take.
///
/// Preparing the shape before the target's binding exists keeps name
/// resolution at the initializer's position: the target place is attached
/// later, once the declaration has recorded the new binding.
enum PreparedStore {
    /// A non-duplicable value that names a place, stored as a move.
    Move { source: Place },
    /// A lowered value stored by assignment.
    Assign { value: Value },
    /// A lowered value assigned from statement-owned storage, followed by a
    /// move-out of the source slot.
    ///
    /// A projection over a deferred materialization is assigned first, because
    /// the assignment is what fills the materialized temporary; the move-out
    /// then empties the selected slot so the temporary's release cannot free
    /// the storage the destination now holds.
    AssignThenMoveOut { value: Value, source: Place },
}

impl PreparedStore {
    /// The lowered value of an assignment store, for ownership reasoning.
    ///
    /// A move store's value travels as a place, not as a lowered value, so it
    /// has none.
    fn lowered(&self) -> Option<&Value> {
        match self {
            Self::Assign { value } | Self::AssignThenMoveOut { value, .. } => Some(value),
            Self::Move { .. } => None,
        }
    }
}

/// The ownership effect an initializer has on the binding it initializes.
///
/// Like [`PreparedStore`], the effect is resolved before the binding is
/// declared, so a name the initializer reads ties to the outer binding the
/// declaration may shadow. Applying it later keeps the flow state and the
/// emitted store agreed on which binding was moved or loaned.
enum BindingEffect {
    /// A view binding: where its storage comes from, the loans it shares, and
    /// the kind of loan a fresh derivation creates.
    View {
        provenance: Provenance,
        kind: LoanKind,
        sources: BTreeSet<LoanId>,
        owners: BTreeSet<LoanOwner>,
    },
    /// A non-duplicable value that names an existing place.
    Move { source: PlaceTarget },
    /// A conditional that consumes one of its owned place arms.
    ///
    /// The runtime move is invisible to the walk, so every owned place arm
    /// becomes maybe-moved: a later use of either reports the moved-value
    /// error, exactly as if the value had been moved unconditionally.
    ConditionalMove { sources: Vec<PlaceTarget> },
    /// A maybe-owning value consumed from one or more places.
    ///
    /// The store runs unconditionally, so every whole-binding source is marked
    /// as consumed: an owning instance moves it and a later use is rejected by
    /// the per-instance audit, while a duplicable instance copies and the use
    /// stays accepted.
    MoveIfOwning { sources: Vec<PlaceTarget> },
    /// A duplicable copy or a fresh value: no ownership effect.
    None,
}

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
            self.pop_scope();
        } else {
            // The block ended in `return`, `break`, or `continue`, and that
            // edge already ran the scope's cleanup (or the block diverged, so
            // no cleanup runs). The scope is left without checking its
            // deferred blocks again: a second check would read state the edge
            // already released.
            self.close_scope_after_edge();
        }
    }

    /// Checks one statement.
    ///
    /// The statement's own destination temporaries are released as soon as it
    /// finishes: a nested statement records its own starting point, so it only
    /// drains what it created and an enclosing statement's temporary stays
    /// pending until that statement ends. A statement that leaves through a
    /// terminator carries its releases itself, so the drain here finds nothing
    /// left.
    pub(super) fn check_stmt(&mut self, stmt: &'a Stmt) {
        self.current_span = stmt_span(stmt);
        let release_floor = self.pending_releases.len();
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
        self.drain_pending_releases(release_floor);
    }

    /// Checks an expression used as a statement and emits its run-time effect.
    ///
    /// An assignment emitted its store while it was walked; an increment or
    /// decrement emits the same read-modify-write as the equivalent compound
    /// assignment; every other effectful expression lowers to a discarded call.
    /// This is the statement path, so a `j++` here increments `j` even though
    /// the same expression in value position yields the old value.
    pub(super) fn check_expr_statement(&mut self, expr: &'a Expr) {
        // An update used as a statement performs its write through the
        // statement write path; the value-position walk owns the same
        // expression when it yields a value, so evaluating the operand
        // directly here keeps the site judged exactly once.
        if let ExprKind::Unary { op, operand } = &expr.kind {
            if op.is_update() {
                self.eval(operand);
                let op = match op {
                    UnaryOp::PreIncrement | UnaryOp::PostIncrement => AssignOp::Add,
                    _ => AssignOp::Sub,
                };
                self.check_update(op, operand);
                return;
            }
        }
        self.eval(expr);
        match &expr.kind {
            ExprKind::Assign { .. } => {}
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
                // A directly invoked lambda is an ephemeral closure call, not a
                // named function call. The whole expression lowers to the
                // two-word call, whose result is discarded.
                if self.is_closure_callee(callee) {
                    let value = self.lower_value(expr);
                    self.emit(Statement::Discard { value });
                    return;
                }
                let call = self.lower_call(callee, type_args, args);
                let move_outs = self.take_move_out_places(call.modes.as_deref(), &call.args);
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
    pub(super) fn check_let(&mut self, decl: &'a LetStmt) {
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
        // The initializer is prepared and checked before the binding is
        // declared: a name the initializer reads must resolve to the outer
        // binding the declaration may shadow, because the new name is not yet
        // in scope in its own initializer. The null check consults the same
        // pre-declaration scope, so a shadowing initializer under a narrowing
        // sees the narrowed outer binding.
        let prepared = decl
            .value
            .as_ref()
            .map(|value| self.prepare_store(&target, value));
        let lowered = prepared
            .as_ref()
            .and_then(PreparedStore::lowered);
        let effect = decl
            .value
            .as_ref()
            .map(|value| self.prepare_binding_effect(&target, value, lowered));
        if let Some(value) = &decl.value {
            self.check_value_into(value, &target, decl.span);
        }
        let id = self.declare_local(&decl.name, target.clone(), false, decl.name_span);
        if let (Some(prepared), Some(effect)) = (prepared, effect) {
            let init = PlaceState {
                initialized: true,
                ..PlaceState::default()
            };
            self.set_place(id, init);
            self.apply_binding_effect(id, effect, decl.span);
            let place = self.binding_place(id);
            self.emit_prepared_store(place, prepared);
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
    pub(super) fn emit_store(&mut self, target: Place, ty: &Ty, value: &'a Expr) -> Option<Value> {
        let prepared = self.prepare_store(ty, value);
        let lowered = prepared.lowered().cloned();
        self.emit_prepared_store(target, prepared);
        lowered
    }

    /// Prepares a store before its target place exists.
    ///
    /// A `let` or `const` initializer must be resolved before the binding it
    /// initializes is declared, so a name the initializer reads resolves to
    /// the outer binding the declaration may shadow. The caller emits the
    /// prepared shape once the target place is built.
    fn prepare_store(&mut self, ty: &Ty, value: &'a Expr) -> PreparedStore {
        // A type that may own at an instantiation stores as a move too: the
        // emitted move checks the substituted source type and falls back to a
        // plain copy when the instance is duplicable.
        let owning = !self.sema.table.duplicable(ty) || self.maybe_owns(ty);
        if owning {
            if let Some(source) = self.place_target(value) {
                return PreparedStore::Move {
                    source: self.target_place(&source),
                };
            }
            // An element read from a call result has no place to move out of,
            // so storing it into an owned destination is the v1 element-move
            // limit rather than an assign of a borrowed element. A conditional
            // whose arms are such reads hits the same limit. A maybe-owning
            // destination cannot judge the limit here, so it stays a copy.
            if !self.maybe_owns(ty)
                && (self.is_ownerless_element_move(value)
                    || self.conditional_ownerless_element_move(value))
            {
                self.report_element_move_limit(value.span);
            }
        }
        let lowered = self.lower_value(value);
        // A registered destination temporary is adopted by moving its value
        // out: the store copies it into the destination and the move-out
        // empties the temporary, so the statement-end release this temporary
        // joined becomes a no-op. Without the move-out the temporary and the
        // destination would both own the value and the release would free it
        // twice (the whole-result adoption of a take-call result).
        if owning {
            if let ValueKind::Temp(id) = &lowered.kind {
                if self.is_destination_temp(*id) {
                    let source = Place {
                        base: ir::PlaceBase::Temp(*id),
                        projections: Vec::new(),
                        ty: lowered.ty.clone(),
                    };
                    return PreparedStore::AssignThenMoveOut {
                        value: lowered,
                        source,
                    };
                }
            }
        }
        // A consumed projection of fresh statement-owned storage is a move:
        // a call result's projection is bound to an owned temporary first,
        // while a projection over an existing destination temporary (a
        // materialized call result or a transfer temporary) is moved out
        // directly, so the temporary's release sees the emptied slot instead
        // of freeing what the destination now owns. A run-time element index is
        // bound once, so the move empties the selected slot. A projection over
        // a caller place keeps the ordinary copy: the caller's container owns
        // the element and the existing move paths already cover a named place.
        if owning
            && matches!(
                lowered.kind,
                ValueKind::Member { .. } | ValueKind::Index { .. }
            )
        {
            if self.projection_roots_at_temp(&lowered) {
                // The base runs before the index, so the materialized base is
                // filled before the index is pinned.
                let lowered = self.settle_projection_base(lowered);
                let lowered = self.materialize_index_chain(lowered);
                if let Some(source) = self.value_place(&lowered) {
                    return PreparedStore::AssignThenMoveOut {
                        value: lowered,
                        source,
                    };
                }
                return PreparedStore::Assign { value: lowered };
            }
            if !self.projects_from_place(&lowered) {
                let rebound = self.bind_ownerless_projection_root(lowered);
                if let Some(source) = self.value_place(&rebound) {
                    return PreparedStore::Move { source };
                }
                return PreparedStore::Assign { value: rebound };
            }
        }
        // A view bound to a projection of fresh ownerless storage — a call
        // result, a struct literal, or a conditional whose value is such an
        // aggregate — binds the backing to a local in the view's scope, so the
        // scope's cleanup releases it after the view's last use and an escaped
        // view is rejected. A chain that already roots at a place, or a value
        // that owns nothing, is returned unchanged.
        //
        // A conditional whose arms project an ownerless literal directly
        // (`c ? (P { a, b }).a : (Q { a, b }).a`) is not bound here: its value
        // is the projected view, and binding each arm's root eagerly would run
        // both arms. Such a shape keeps its unreleased backing.
        let lowered = if is_view(ty) && !self.projects_from_place(&lowered) {
            self.bind_view_projection_root(lowered)
        } else {
            lowered
        };
        PreparedStore::Assign { value: lowered }
    }

    /// Emits a store prepared by [`Self::prepare_store`].
    fn emit_prepared_store(&mut self, target: Place, prepared: PreparedStore) {
        match prepared {
            PreparedStore::Move { source } => self.emit(Statement::Move { target, source }),
            PreparedStore::Assign { value } => self.emit(Statement::Assign { target, value }),
            PreparedStore::AssignThenMoveOut { value, source } => {
                self.emit(Statement::Assign { target, value });
                self.emit(Statement::MoveOut { place: source });
            }
        }
    }

    /// Checks a `const` binding.
    pub(super) fn check_const(&mut self, decl: &'a ConstStmt) {
        self.current_span = decl.span;
        let declared = decl.ty.as_ref().map(|ty| self.resolve(ty));
        if let Some(ty) = &declared {
            self.check_nullable_decl(ty, decl.span);
        }
        let value_ty = self.eval(&decl.value);
        let target = declared.or(Some(value_ty)).unwrap_or_else(Ty::unknown);
        // As in `check_let`, the store, the ownership effect, and the null
        // check resolve before the binding is declared, so the initializer's
        // names tie to the outer bindings they may shadow.
        let prepared = self.prepare_store(&target, &decl.value);
        let lowered = prepared.lowered();
        let effect = self.prepare_binding_effect(&target, &decl.value, lowered);
        self.check_value_into(&decl.value, &target, decl.span);
        let id = self.declare_local(&decl.name, target.clone(), true, decl.name_span);
        self.set_place(
            id,
            PlaceState {
                initialized: true,
                ..PlaceState::default()
            },
        );
        self.apply_binding_effect(id, effect, decl.span);
        let place = self.binding_place(id);
        self.emit_prepared_store(place, prepared);
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

    /// Applies an initializer's ownership effect to the binding it initializes.
    ///
    /// A view bound from a local or a parameter carries a loan of that owner:
    /// a `take` or `borrow` parameter is storage the callee may not move or
    /// write while a use of the derived view is still reachable. A view value
    /// that already carries a loan shares it; a fresh derivation creates one.
    /// A writable view derived from a read-only loan is `E10041`.
    pub(super) fn bind_value(
        &mut self,
        id: usize,
        target: &Ty,
        value: &Expr,
        lowered: Option<&Value>,
        span: Span,
    ) {
        let effect = self.prepare_binding_effect(target, value, lowered);
        self.apply_binding_effect(id, effect, span);
    }

    /// Resolves the ownership effect an initializer has on the new binding.
    ///
    /// A view binding ties to the owner its provenance names and shares or
    /// creates a loan; a non-duplicable value that names a place is a move
    /// from that place. Resolving the effect before the binding is declared
    /// keeps a name the initializer reads tied to the outer binding the
    /// declaration may shadow, so the flow state and the emitted store agree
    /// on which binding was moved or loaned. `lowered` is the initializer's
    /// lowered value when it has one: a view over an ownerless value derives
    /// from the statement temporary the lowering created, and only the lowered
    /// value knows that temporary's identity.
    fn prepare_binding_effect(
        &self,
        target: &Ty,
        value: &Expr,
        lowered: Option<&Value>,
    ) -> BindingEffect {
        if self.maybe_owns(target) {
            // The flow keeps copy semantics for a maybe-owning target; the
            // sources are only marked as consumed so a later use deposits the
            // duplicability duty.
            let mut sources = Vec::new();
            if let Some(source) = self.place_target(value) {
                sources.push(source);
            } else {
                sources.extend(self.maybe_owning_move_sources(value));
            }
            if sources.is_empty() {
                return BindingEffect::None;
            }
            return BindingEffect::MoveIfOwning { sources };
        }
        if is_view(target) {
            return BindingEffect::View {
                provenance: match self.view_provenance(value) {
                    Provenance::None => lowered
                        .map(|lowered| self.lowered_view_provenance(lowered))
                        .unwrap_or(Provenance::None),
                    provenance => provenance,
                },
                kind: self.view_loan_kind(value),
                sources: self.view_loans(value),
                owners: self.view_owners(value, lowered),
            };
        }
        if self.sema.table.duplicable(target) {
            return BindingEffect::None;
        }
        if let Some(source) = self.place_target(value) {
            return BindingEffect::Move { source };
        }
        let sources = self.conditional_move_sources(value);
        if !sources.is_empty() {
            return BindingEffect::ConditionalMove { sources };
        }
        BindingEffect::None
    }

    /// Records a prepared ownership effect on the binding that just declared.
    fn apply_binding_effect(&mut self, id: usize, effect: BindingEffect, span: Span) {
        match effect {
            BindingEffect::None => {}
            BindingEffect::Move { source } => self.move_target(source, span),
            BindingEffect::ConditionalMove { sources } => {
                for source in sources {
                    // The assignment target is written by the conditional, so
                    // it stays initialized; only the other arms are moves.
                    if let PlaceTarget::Local(source_id, projections) = &source {
                        if *source_id == id && projections.is_empty() {
                            continue;
                        }
                    }
                    self.move_target(source, span);
                }
            }
            BindingEffect::MoveIfOwning { sources } => {
                // A move whose destination is a writable parameter slot may be
                // the `borrow T?` out-slot carve-out at some instance, so the
                // per-instance audit exempts a nullable substitution there.
                // Every other destination keeps the strict lend judgment.
                let into_slot = matches!(
                    self.bindings[id].kind,
                    BindingKind::Param | BindingKind::OutSlot
                ) && self.bindings[id].mode == Mode::Borrow;
                for source in sources {
                    if let PlaceTarget::Local(source_id, projections) = &source {
                        if *source_id == id && projections.is_empty() {
                            continue;
                        }
                    }
                    if into_slot {
                        self.mark_moved_into_slot_if_owning(source, span);
                    } else {
                        self.mark_moved_if_owning(source, span);
                    }
                }
            }
            BindingEffect::View {
                provenance,
                kind,
                sources,
                owners,
            } => {
                let mut loans: BTreeSet<LoanId> = BTreeSet::new();
                if sources.is_empty() {
                    // A fresh derivation creates one loan per owner: a direct
                    // owner, every arm of a conditional, and every statement
                    // temporary the lowered value reads through.
                    for owner in owners {
                        if let Some(loan) = self.record_view_loan(owner, kind, span) {
                            loans.insert(loan);
                        }
                    }
                } else {
                    // An existing loan is shared; the binding's use of it keeps
                    // the loan live. A binding owner behind the shared loans
                    // needs no second loan; a temporary owner still does.
                    for loan in &sources {
                        self.record_event(LoanEventKind::Used { loan: *loan }, span);
                    }
                    loans.extend(sources);
                    for owner in owners {
                        if matches!(owner, LoanOwner::Temp(_)) {
                            if let Some(loan) = self.record_view_loan(owner, kind, span) {
                                loans.insert(loan);
                            }
                        }
                    }
                }
                self.set_binding_loans(id, loans);
                self.bindings[id].provenance = provenance;
            }
        }
    }

    /// Creates a persistent loan of an owner and reports the one inline error.
    ///
    /// A writable view derived from a read-only borrow is `E10041`. The loan's
    /// id is interned from its origin, so a speculative pass and the reporting
    /// pass agree on it and the loop fixpoint does not multiply ids once per
    /// iteration. The loan request and the loan itself are recorded as events
    /// only on the reporting pass; whether the request conflicts with a live
    /// loan is decided by the liveness post-pass.
    fn record_view_loan(&mut self, owner: LoanOwner, kind: LoanKind, span: Span) -> Option<LoanId> {
        let loan = self.intern_loan(LoanOrigin::new(owner, kind, span));
        if let LoanOwner::Binding(owner) = owner {
            if self.report_diagnostics
                && kind == LoanKind::Exclusive
                && self.is_readonly_borrow(owner)
            {
                let name = self.bindings[owner].name.clone();
                self.error(
                    "E10041",
                    format!("cannot write through a view of read-only borrow '{name}'"),
                    "take the owner by `borrow`",
                    span,
                );
                return None;
            }
        }
        if self.report_diagnostics {
            self.record_event(LoanEventKind::Request { owner, kind }, span);
            self.record_event(LoanEventKind::Created { loan, owner, kind }, span);
        }
        Some(loan)
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
            if self.is_terminated() {
                // The clause left through `return`, `break`, or `continue`,
                // and that edge already ran the clause scope's cleanup (or the
                // clause diverged). Leaving it must not check its deferred
                // blocks again.
                self.close_scope_after_edge();
            } else {
                self.pop_scope();
            }
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
    /// The block's IR is lowered at registration, but its ownership effects are
    /// checked on each scope exit edge, in the scope's reverse registration
    /// order, so they interleave with the scope's drops exactly as the emitter
    /// runs them.
    pub(super) fn check_defer(&mut self, block: &'a Block) {
        let saved_block = self.builder.current();
        let scope = self.current_scope();
        let defer_block = self.new_block();
        let registration = self.register_cleanup();

        let saved_state = self.state.clone();
        let saved_scopes = self.scopes.len();
        let saved_defers = self.defer_entries.len();
        let saved_report = self.report_diagnostics;
        let saved_owned = self.materialize_owned_temporaries;
        self.report_diagnostics = false;
        self.materialize_owned_temporaries = false;
        self.switch_to(defer_block);
        self.check_block(block);
        // The body's blocks occupy the contiguous arena range from the defer
        // block to `region_end`, so the range is recorded for the emitter's
        // bounded copy.
        let region_end = self.builder.function().blocks.len();
        self.report_diagnostics = saved_report;
        self.materialize_owned_temporaries = saved_owned;
        self.state = saved_state;
        // The deferred block's own locals are kept: the IR lowered here refers
        // to them, so they must stay in the function's local list for the
        // emitter to declare them. Their cleanup is the block's own.
        self.scopes.truncate(saved_scopes);
        self.defer_entries.truncate(saved_defers);
        self.switch_to(saved_block);

        // The registration marker is emitted once the body's range is known; a
        // `defer` runs from its scope's cleanup, not from this position, so the
        // marker's place among the block's statements carries no runtime
        // meaning.
        self.emit(Statement::Defer {
            block: defer_block,
            scope,
            order: registration,
            end: region_end,
        });

        self.defer_entries.push(DeferEntry {
            depth: self.scopes.len(),
            block,
            registration,
        });
    }

    /// Checks a `break`, recording the edge and running the scopes it leaves.
    pub(super) fn check_break(&mut self, _span: Span) {
        let Some(frame) = self.exits.last() else {
            return;
        };
        let break_block = frame.break_block;
        let floor = frame.scope_floor;
        // The cleanup the break runs is part of the edge state, so it runs
        // before the edge is recorded; otherwise a loan a defer creates would
        // be missing from the state that joins after the loop.
        self.emit_exit_cleanup(floor);
        // The edge also carries the statement temporaries still pending: it
        // leaves the statements that own them, so they die here on this path.
        self.emit_pending_release_copies();
        let state = self.edge_state(floor);
        if let Some(frame) = self.exits.last_mut() {
            frame.record_break(state);
        }
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
        // The continue edge leaves the statements that own the pending
        // temporaries, so their releases run here; the loop head re-assigns
        // its own temporary on the next iteration.
        self.emit_pending_release_copies();
        // The cleanup the continue runs belongs to its edge, and the edge
        // returns to the loop head, so its state is recorded against the head's
        // block for the loop fixpoint to join into the entry.
        let state = self.edge_state(floor);
        self.record_continue(continue_block, state);
        self.terminate(Terminator::Goto(continue_block));
    }

    /// Emits the cleanup of the scopes deeper than `floor`, innermost first.
    ///
    /// A `ScopeExit` marker precedes each scope's cleanup, so the emitter
    /// routes the edge through that scope even when it owns only a `defer`, and
    /// then the scope's drops and defers run in reverse registration order.
    fn emit_exit_cleanup(&mut self, floor: usize) {
        let depth = self.scopes.len();
        for index in (floor..depth).rev() {
            let scope_id = self.scopes[index].id;
            self.emit(Statement::ScopeExit { scope: scope_id });
            self.run_scope_cleanup(index);
        }
    }

    /// Checks a `return`.
    ///
    /// A non-duplicable return value is an ownership transfer, so it runs the
    /// move rules: returning a lend parameter is `E10034`, and returning a
    /// moved value is `E10020`. The evaluated value runs before the statement
    /// temporaries it read are released: the terminator carries a copy of the
    /// pending releases, while the pending list stays for the statement's
    /// other paths.
    pub(super) fn check_return(&mut self, value: Option<&'a Expr>, span: Span) {
        if let Some(value) = value {
            self.eval(value);
            let returns = self.returns.clone();
            self.check_value_into(value, &returns, span);
            let maybe_owning = self.maybe_owns(&returns);
            if (!self.sema.table.duplicable(&returns) || maybe_owning) && !returns.is_unknown() {
                if let Some(target) = self.place_target(value) {
                    if maybe_owning {
                        self.mark_escape_move_if_owning(target, span);
                    } else {
                        self.move_target(target, span);
                    }
                } else {
                    let sources = if maybe_owning {
                        self.maybe_owning_move_sources(value)
                    } else {
                        self.conditional_move_sources(value)
                    };
                    if sources.is_empty() {
                        if !maybe_owning
                            && (self.is_ownerless_element_move(value)
                                || self.conditional_ownerless_element_move(value))
                        {
                            // The return type owns the value it receives, and
                            // an element of a call result has no owner that
                            // outlives the statement.
                            self.report_element_move_limit(span);
                        }
                    } else {
                        // A conditional return moves whichever owned arm the
                        // condition selects, so either arm may be gone; a
                        // maybe-owning return records the duty instead.
                        for source in sources {
                            if maybe_owning {
                                self.mark_escape_move_if_owning(source, span);
                            } else {
                                self.move_target(source, span);
                            }
                        }
                    }
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
            let lowered = self.bind_member_receiver(lowered);
            // A consumed projection of a fresh call result is bound and
            // re-rooted, so the return emission empties the selected field and
            // the temporary releases only the remaining fields. A whole call
            // result keeps its own destination handling.
            let lowered = if (!self.sema.table.duplicable(&returns) || maybe_owning)
                && !is_view(&returns)
                && matches!(
                    lowered.kind,
                    ValueKind::Member { .. } | ValueKind::Index { .. }
                )
                && !self.projects_from_place(&lowered)
            {
                self.bind_ownerless_projection_root(lowered)
            } else {
                lowered
            };
            // A moved-out return reads its run-time index once: the source move
            // empties a slot the returned copy also read, so both must use the
            // same index temporary. The base is filled first because it runs
            // before the index.
            let lowered = if (!self.sema.table.duplicable(&returns) || maybe_owning)
                && !is_view(&returns)
                && matches!(
                    lowered.kind,
                    ValueKind::Member { .. } | ValueKind::Index { .. }
                )
            {
                let lowered = self.settle_projection_base(lowered);
                self.materialize_index_chain(lowered)
            } else {
                lowered
            };
            // A view derived from a statement temporary cannot leave the
            // function: the caller's use lies after the temporary's release.
            // The loans are created at the return, the releases follow, and
            // the use is recorded last, so the liveness rule reports `E10030`.
            let temp_loans: Vec<LoanId> = if is_view(&returns) {
                self.lowered_temp_owners(&lowered)
                    .into_iter()
                    .filter_map(|temp| {
                        self.record_view_loan(LoanOwner::Temp(temp), LoanKind::Shared, span)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            self.emit_return_cleanup();
            let releases = self.pending_release_copies();
            self.record_release_events(&releases);
            for loan in temp_loans {
                self.record_event(LoanEventKind::Used { loan }, span);
            }
            self.terminate(Terminator::Return {
                value: Some(lowered),
                releases,
            });
        } else {
            self.emit_return_cleanup();
            let releases = self.pending_release_copies();
            self.record_release_events(&releases);
            self.terminate(Terminator::Return {
                value: None,
                releases,
            });
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

//! Call checking for the ownership stage.
//!
//! This walk checks calls against the declared parameter modes, the `ffi`
//! contracts, and the standard-library view helpers. A dot-call receiver is
//! checked like an argument at index zero, so a receiver-only read counts as a
//! use and a live view of the receiver blocks a reallocating call.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {

    /// Checks a call expression.
    ///
    /// A `func`-typed field call is not a method taking the receiver: the
    /// field's own signature (already checked by the type stage) types it, so
    /// the ownership walk only reads the field and its arguments.
    pub(super) fn check_call_expr(&mut self, callee: &'a Expr, args: &'a [Arg], span: Span) {
        if let Some(field_ty) = self.callee_field_type(callee) {
            if let TyKind::Func(signature) = &field_ty.kind {
                // The callee is a field, not a named function, so the
                // receiver is only read and the field's own signature types
                // the call. A call-site marker must still match the declared
                // mode. The modes carry no ownership semantics here, so a
                // `take` argument is not marked as moved: the lowering does
                // not emit a move-out for a field call.
                self.eval_dot_receiver(callee);
                let params: Vec<(Mode, Ty)> = signature
                    .params
                    .iter()
                    .map(|param| (param.mode, param.ty.clone()))
                    .collect();
                if let Some(name) = self.callee_name(callee) {
                    self.check_marker_matches(&name, args, &params);
                }
                for arg in args {
                    self.eval(&arg.value);
                }
                return;
            }
        }
        let callee_name = self.callee_name(callee);
        let Some(name) = callee_name else {
            self.eval(callee);
            for arg in args {
                self.eval(&arg.value);
            }
            return;
        };
        let Some(resolved) = self.program.callee(&name) else {
            // The callee may be an intrinsic reached as a dot-call, such as
            // `xs.len()`; its receiver is still read, so a local used only
            // there must not be reported unused. A `func`-typed local or a
            // closure parameter used as the callee is marked read directly, so
            // a variable used only as a callable is not reported unused.
            if self.is_dot_call(callee) {
                self.eval_dot_receiver(callee);
            } else if let Some(PlaceTarget::Local(id, _)) = self.place_target(callee) {
                self.bindings[id].used = true;
            }
            for arg in args {
                self.eval(&arg.value);
            }
            return;
        };
        match resolved {
            Callee::User(decl) => {
                let type_params: Vec<String> =
                    decl.type_params.iter().map(|p| p.name.clone()).collect();
                let params: Vec<(Mode, Ty)> = decl
                    .params
                    .iter()
                    .map(|param| {
                        (
                            param.mode,
                            resolve_ty(&param.ty, &type_params, &self.sema.table),
                        )
                    })
                    .collect();
                // A dot-call's receiver is the first parameter; the written
                // arguments line up with the parameters after it. The receiver
                // is walked and borrow-checked like an argument, so a local
                // used only as a receiver counts as used and a live view of it
                // blocks a reallocating call.
                let is_dot = self.is_dot_call(callee);
                let skip = usize::from(is_dot).min(params.len());
                let written_params = &params[skip..];
                self.check_marker_matches(&name, args, written_params);
                let mut lent: BTreeSet<usize> = BTreeSet::new();
                if is_dot {
                    self.check_dot_receiver(callee, params.first(), &mut lent);
                }
                self.check_arguments(args, written_params, span, &mut lent);
            }
            Callee::Ffi(signature) => {
                if self.is_dot_call(callee) {
                    self.eval_dot_receiver(callee);
                }
                self.check_ffi_call(&name, signature, args, span);
            }
            Callee::Stdlib { effect, .. } => {
                if self.is_dot_call(callee) {
                    self.eval_dot_receiver(callee);
                }
                self.check_stdlib_call(effect, args);
            }
        }
    }

    /// Walks a dot-call receiver for its reads and raw dereferences.
    ///
    /// The receiver expression is not a written argument, so this only records
    /// the read; the borrow effect is applied by [`Self::check_dot_receiver`]
    /// for a resolved user function.
    pub(super) fn eval_dot_receiver(&mut self, callee: &'a Expr) {
        match &callee.kind {
            ExprKind::Member { base, .. } => {
                self.eval(base);
            }
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let last = name.segments.len() - 1;
                let path = crate::ast::QualifiedName {
                    segments: name.segments[..last].to_vec(),
                    span: name.span,
                };
                self.eval_qualified(&path, name.span);
            }
            _ => {}
        }
    }

    /// The simple name a call resolves through, when the callee is a name.
    pub(super) fn callee_name(&self, callee: &Expr) -> Option<String> {
        match &callee.kind {
            ExprKind::Name(name) => Some(name.clone()),
            ExprKind::Qualified(name) => name.segments.last().cloned(),
            ExprKind::Member { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    /// Reports `E10039` when a call-site marker contradicts a declaration.
    pub(super) fn check_marker_matches(&mut self, name: &str, args: &[Arg], params: &[(Mode, Ty)]) {
        for (arg, (mode, _)) in args.iter().zip(params) {
            if arg.marker != Mode::None && marker_kind(arg.marker) != marker_kind(*mode) {
                self.error(
                    "E10039",
                    format!("call-site marker does not match the declaration of '{name}'"),
                    "remove the marker or correct it",
                    arg.span,
                );
                return;
            }
        }
    }

    /// Checks a call's arguments against user-declared parameter modes.
    ///
    /// An exclusive lend is call-scoped, so `lent` accumulates the owners
    /// already lent in this one call (including the dot-call receiver): lending
    /// the same place twice in one call is a conflicting loan even though two
    /// separate calls are fine.
    pub(super) fn check_arguments(
        &mut self,
        args: &'a [Arg],
        params: &[(Mode, Ty)],
        _span: Span,
        lent: &mut BTreeSet<usize>,
    ) {
        for (index, arg) in args.iter().enumerate() {
            let expected = params.get(index).map(|(_, ty)| ty.clone());
            let arg_ty = self.eval(&arg.value);
            if let Some(expected) = &expected {
                self.check_value_into(&arg.value, expected, arg.span);
            }
            let mode = params.get(index).map(|(mode, _)| *mode).unwrap_or(Mode::None);
            self.check_argument_place(&arg.value, arg.span, mode, expected.as_ref(), &arg_ty, lent);
        }
    }

    /// Walks and borrow-checks a dot-call's receiver as parameter zero.
    ///
    /// The receiver is not written at the call site, so its type comes from the
    /// checked expression or the head binding rather than from a call argument.
    /// It shares the call's `lent` set, so lending the receiver twice in one
    /// call is rejected even when the second lend is a written argument.
    pub(super) fn check_dot_receiver(
        &mut self,
        callee: &'a Expr,
        expected: Option<&(Mode, Ty)>,
        lent: &mut BTreeSet<usize>,
    ) {
        let arg_ty = self.dot_receiver_type(callee);
        let (mode, param_ty) = match expected {
            Some((mode, ty)) => (*mode, Some(ty.clone())),
            None => (Mode::None, None),
        };
        self.eval_dot_receiver(callee);
        let receiver = match &callee.kind {
            ExprKind::Member { .. } => None,
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let last = name.segments.len() - 1;
                let path = crate::ast::QualifiedName {
                    segments: name.segments[..last].to_vec(),
                    span: name.span,
                };
                Some(Expr {
                    kind: ExprKind::Qualified(path),
                    span: name.span,
                })
            }
            _ => return,
        };
        // The walk above already read the receiver; this borrow check only
        // needs its place and provenance, so a synthesized qualified path is
        // enough and the type-stage lookup is never consulted.
        match &receiver {
            None => {
                let base = match &callee.kind {
                    ExprKind::Member { base, .. } => base,
                    _ => return,
                };
                self.check_argument_place(
                    base,
                    base.span,
                    mode,
                    param_ty.as_ref(),
                    &arg_ty,
                    lent,
                );
            }
            Some(receiver) => {
                self.check_argument_place(
                    receiver,
                    receiver.span,
                    mode,
                    param_ty.as_ref(),
                    &arg_ty,
                    lent,
                );
            }
        }
    }

    /// The type of a dot-call receiver, without evaluating it.
    pub(super) fn dot_receiver_type(&self, callee: &Expr) -> Ty {
        match &callee.kind {
            ExprKind::Member { base, .. } => self.ty(base),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let Some(head) = name.segments.first() else {
                    return Ty::unknown();
                };
                let mut ty = if let Some(id) = self.lookup(head) {
                    self.bindings[id].ty.clone()
                } else if let Some(ty) = self.global_type(head) {
                    ty
                } else {
                    return Ty::unknown();
                };
                let last = name.segments.len() - 1;
                for segment in &name.segments[1..last] {
                    ty = self.member_type(&ty, segment);
                }
                ty
            }
            _ => Ty::unknown(),
        }
    }

    /// Applies the ownership effect of one argument.
    pub(super) fn check_argument_place(
        &mut self,
        expr: &Expr,
        span: Span,
        mode: Mode,
        expected: Option<&Ty>,
        arg_ty: &Ty,
        lent: &mut BTreeSet<usize>,
    ) {
        let target = self.place_target(expr);
        match mode {
            Mode::Take => {
                let maybe_owning = self.maybe_owns(arg_ty);
                if !self.sema.table.duplicable(arg_ty) || maybe_owning {
                    if let Some(target) = target {
                        if maybe_owning {
                            self.mark_moved_if_owning(target, span);
                        } else {
                            self.move_target(target, span);
                        }
                    } else {
                        let sources = if maybe_owning {
                            self.maybe_owning_move_sources(expr)
                        } else {
                            self.conditional_move_sources(expr)
                        };
                        if sources.is_empty() {
                            if !maybe_owning
                                && (self.is_ownerless_element_move(expr)
                                    || self.conditional_ownerless_element_move(expr))
                            {
                                // A `take` argument owns the value it receives,
                                // and an element of a call result has no owner
                                // that outlives the statement.
                                self.report_element_move_limit(span);
                            }
                        } else {
                            // The callee consumes whichever arm the condition
                            // selects, so either arm may be gone; a maybe-owning
                            // argument records the duplicability duty instead.
                            for source in sources {
                                if maybe_owning {
                                    self.mark_moved_if_owning(source, span);
                                } else {
                                    self.move_target(source, span);
                                }
                            }
                        }
                    }
                }
            }
            Mode::Borrow => {
                // A borrow reads the place, so a later use of a consumed
                // maybe-owning path records the duplicability duty here.
                if let Some(PlaceTarget::Local(id, projections)) = &target {
                    let (id, projections) = (*id, projections.clone());
                    self.note_maybe_moved_use(id, &projections, span);
                }
                if let Some(PlaceTarget::Global(id, _)) = &target {
                    if !self.global_is_const(id) {
                        self.error(
                            "E10038",
                            format!("cannot borrow global '{}'", id.name),
                            "borrow from a local owner",
                            span,
                        );
                    }
                    return;
                }
                if self.sema.table.duplicable(arg_ty) && !is_view(arg_ty) {
                    // A writable lend of a duplicable value binds a copy, so
                    // the write reaches the copy only.
                    let name = match &target {
                        Some(PlaceTarget::Local(id, _)) => self.bindings[*id].name.clone(),
                        _ => String::from("value"),
                    };
                    self.warn(
                        "W10001",
                        format!("unnecessary lend of duplicable '{name}'"),
                        span,
                    );
                }
                // The owner may be a direct place or a view derived from a
                // call such as `slice(buf, ...)`; both lend the base storage.
                let owner = match &target {
                    Some(PlaceTarget::Local(id, _)) => Some(*id),
                    _ => self.argument_owner(expr),
                };
                if let Some(owner) = owner {
                    if expected.is_some_and(is_view) && self.is_readonly_borrow(owner) {
                        let name = self.bindings[owner].name.clone();
                        self.error(
                            "E10041",
                            format!("cannot write through a view of read-only borrow '{name}'"),
                            "take the owner by `borrow`",
                            span,
                        );
                        return;
                    }
                    if !self.sema.table.duplicable(arg_ty) {
                        // Two lends of one owner in the same call are a
                        // duplicate; whether the lend conflicts with a
                        // persistent loan is decided by the liveness post-pass.
                        if !lent.insert(owner) {
                            let name = self.bindings[owner].name.clone();
                            self.error(
                                "E10032",
                                format!("'{name}' is already borrowed"),
                                "end the first borrow first",
                                span,
                            );
                        }
                        self.record_event(
                            LoanEventKind::Request {
                                owner: LoanOwner::Binding(owner),
                                kind: LoanKind::Exclusive,
                            },
                            span,
                        );
                    }
                    // A writable lend can only invalidate a null narrowing when
                    // the lent type admits `null`: a `borrow T` of a
                    // non-nullable `T` cannot write `null` back. Fall back to
                    // the owner's type when no parameter type is known.
                    let writes_nullable = match expected {
                        Some(ty) => ty.nullable,
                        None => self.bindings[owner].ty.nullable,
                    };
                    if writes_nullable {
                        self.with_place(owner, |place| place.narrowed.clear());
                    }
                }
            }
            Mode::None | Mode::Const => {
                if let Some(PlaceTarget::Global(id, _)) = &target {
                    if !self.global_is_const(id) {
                        self.error(
                            "E10038",
                            format!("cannot borrow global '{}'", id.name),
                            "borrow from a local owner",
                            span,
                        );
                    }
                }
            }
        }
    }

    /// The binding that owns the storage a view value points into.
    pub(super) fn argument_owner(&self, expr: &Expr) -> Option<usize> {
        match self.view_provenance(expr) {
            Provenance::Local(id) | Provenance::Param(id) => Some(id),
            _ => None,
        }
    }

    /// True when the module-level binding is a `const`.
    pub(super) fn global_is_const(&self, id: &ir::GlobalId) -> bool {
        self.program
            .globals
            .get(id)
            .is_some_and(|global| global.is_const)
    }

    /// True when the binding is a read-only borrow of view storage.
    ///
    /// A local view is read-only when the parameter it was derived from is a
    /// read-only lend; a view derived from a `borrow` or `take` parameter is
    /// writable, because that parameter owns or may write the storage.
    pub(super) fn is_readonly_borrow(&self, id: usize) -> bool {
        let binding = &self.bindings[id];
        if binding.kind == BindingKind::Param {
            return matches!(binding.mode, Mode::None | Mode::Const);
        }
        matches!(
            &binding.provenance,
            Provenance::Param(owner)
                if matches!(self.bindings[*owner].mode, Mode::None | Mode::Const)
        )
    }

    /// Checks a call to an `ffi`-declared function.
    pub(super) fn check_ffi_call(
        &mut self,
        name: &str,
        signature: &crate::ast::FuncSignature,
        args: &'a [Arg],
        span: Span,
    ) {
        let uncontracted = ffi::is_uncontracted(signature, &self.sema.table);
        if uncontracted && self.unsafe_depth == 0 {
            self.error(
                "E10100",
                format!("uncontracted call to '{name}' requires 'unsafe'"),
                "wrap the call, or add a contract declaration",
                span,
            );
        }
        let mut params: Vec<(Mode, Ty)> = Vec::new();
        for param in &signature.params {
            params.push((
                param.mode,
                ffi::signature_param_type(&param.ty, &self.sema.table),
            ));
        }
        self.check_marker_matches(name, args, &params);
        let mut lent: BTreeSet<usize> = BTreeSet::new();
        for (index, arg) in args.iter().enumerate() {
            let arg_ty = self.eval(&arg.value);
            let Some((mode, expected)) = params.get(index) else {
                continue;
            };
            if ffi::is_uncontracted_param(*mode, expected) {
                // An uncontracted parameter accepts only scalars, `raw`,
                // `cstring`, views, and a literal `null`. A handle or
                // aggregate is rejected whether or not it is nullable: a
                // narrowed value still owns storage.
                if is_handle_or_aggregate(&arg_ty) && !matches!(arg.value.kind, ExprKind::Null) {
                    let param_name = signature.params[index].name.clone();
                    self.error(
                        "E10101",
                        format!("'{name}' has no contract for parameter '{param_name}'"),
                        "pass a `raw` pointer derived inside `unsafe`",
                        arg.span,
                    );
                }
                continue;
            }
            self.check_value_into(&arg.value, expected, arg.span);
            self.check_argument_place(&arg.value, arg.span, *mode, Some(expected), &arg_ty, &mut lent);
        }
    }

    /// Applies the ownership effect of a standard-library helper.
    pub(super) fn check_stdlib_call(&mut self, effect: StdlibEffect, args: &'a [Arg]) {
        if let StdlibEffect::ViewOf { arg, .. } = effect {
            for (index, written) in args.iter().enumerate() {
                self.eval(&written.value);
                if index == arg {
                    if let Some(PlaceTarget::Global(id, _)) = self.place_target(&written.value) {
                        if !self.global_is_const(&id) {
                            self.error(
                                "E10038",
                                format!("cannot borrow global '{}'", id.name),
                                "borrow from a local owner",
                                written.span,
                            );
                        }
                    }
                }
            }
            return;
        }
        for written in args {
            self.eval(&written.value);
        }
    }
}

/// The exclusive/shared class a call-site or declaration marker denotes.
///
/// An unmarked parameter and an explicit `const` both grant read-only access,
/// so they are treated as the same assertion.
fn marker_kind(mode: Mode) -> Mode {
    match mode {
        Mode::None | Mode::Const => Mode::Const,
        other => other,
    }
}

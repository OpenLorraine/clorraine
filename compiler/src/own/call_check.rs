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
    /// the ownership walk reads the field and applies the signature's
    /// parameter modes to the arguments — a `take` marks its move, a `borrow`
    /// records its lend, and a plain lend only reads.
    pub(super) fn check_call_expr(
        &mut self,
        callee: &'a Expr,
        type_args: &'a [crate::ast::Type],
        args: &'a [Arg],
        span: Span,
    ) {
        if let Some(field_ty) = self.callee_field_type(callee) {
            if let Some(params) = self.callable_signature(&field_ty) {
                // The callee is a field, not a named function, so the
                // receiver is only read and the field's own signature types
                // the call: its `take` arguments are marked moved, its
                // `borrow` arguments record their lend, and a same-named
                // module function's modes are never applied to it. A
                // call-site marker must still match the declared mode.
                self.eval_dot_receiver(callee);
                if let Some(name) = self.callee_name(callee) {
                    self.check_marker_matches(&name, args, &params);
                }
                let effective: Vec<Ty> = params.iter().map(|(_, ty)| ty.clone()).collect();
                let mut lent: BTreeSet<usize> = BTreeSet::new();
                self.check_arguments(args, &params, &effective, span, &mut lent, None);
                return;
            }
        }
        // A callee that is itself a call is called through the function value
        // it returns; the recorded result type carries the signature, so the
        // arguments are checked exactly like a func-typed binding's. The
        // inner call is evaluated first, exactly as the lowering lowers it.
        if matches!(callee.kind, ExprKind::Call { .. }) {
            let result_ty = self.ty(callee);
            if let Some(params) = self.callable_signature(&result_ty) {
                self.eval(callee);
                // The returned signature declares the parameter modes, so a
                // written marker must match it exactly like any other
                // value callee's; the inner callee names the declaration.
                if let ExprKind::Call { callee: inner, .. } = &callee.kind {
                    if let Some(name) = self.callee_name(inner) {
                        self.check_marker_matches(&name, args, &params);
                    }
                }
                let effective: Vec<Ty> = params.iter().map(|(_, ty)| ty.clone()).collect();
                let mut lent: BTreeSet<usize> = BTreeSet::new();
                self.check_arguments(args, &params, &effective, span, &mut lent, None);
                return;
            }
        }
        // A name that resolves to a `func`-typed local, parameter, or capture
        // is called through its binding: the binding's own signature types the
        // call and carries its parameter modes, and a same-named module
        // function keeps its own identity. The arguments are walked like a
        // module call's, so a `take` argument is marked moved and a `borrow`
        // argument records its lend.
        if let Some(id) = self.func_binding_callee(callee) {
            self.bindings[id].used = true;
            if let Some(params) = self.callable_signature(&self.bindings[id].ty) {
                let name = self.callee_name(callee).unwrap_or_default();
                self.check_marker_matches(&name, args, &params);
                let effective: Vec<Ty> = params.iter().map(|(_, ty)| ty.clone()).collect();
                let mut lent: BTreeSet<usize> = BTreeSet::new();
                self.check_arguments(args, &params, &effective, span, &mut lent, None);
                return;
            }
        }
        // A non-name callee whose recorded type is callable — an inline lambda
        // or a conditional over func values — is called through its value. The
        // expression's own signature types the call and carries its parameter
        // modes exactly as a func-typed binding's does.
        if self.callee_name(callee).is_none() {
            if let Some(params) = self.value_callee_signature(&self.ty(callee)) {
                self.eval(callee);
                self.mark_written_captures(callee);
                self.check_marker_matches("the callable value", args, &params);
                // A closure's parameters are unmarked read-only lends. The
                // closure-call lowering materializes an owned argument itself,
                // so only the marker check and the argument reads run here.
                if matches!(self.ty(callee).kind, TyKind::Closure(_)) {
                    for arg in args {
                        self.eval(&arg.value);
                    }
                    return;
                }
                let effective: Vec<Ty> = params.iter().map(|(_, ty)| ty.clone()).collect();
                let mut lent: BTreeSet<usize> = BTreeSet::new();
                self.check_arguments(args, &params, &effective, span, &mut lent, None);
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
        let Some(resolved) = self.resolved_callee(&name) else {
            // A bare name may resolve to a module-level func-typed global:
            // the global's value is the callable, so its signature types the
            // call exactly as any other callable's does.
            if let Some(params) = self
                .value_callee_global_type(callee)
                .and_then(|ty| self.callable_signature(&ty))
            {
                if let Some(name) = self.callee_name(callee) {
                    self.check_marker_matches(&name, args, &params);
                }
                let effective: Vec<Ty> = params.iter().map(|(_, ty)| ty.clone()).collect();
                let mut lent: BTreeSet<usize> = BTreeSet::new();
                self.check_arguments(args, &params, &effective, span, &mut lent, None);
                return;
            }
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
                // An explicit instantiation fixes the callee's parameter types,
                // so a nullable slot argument is not read as a possibly-null
                // value. An inferred call keeps the declared parameter types.
                let mut bindings: BTreeMap<String, Ty> = decl
                    .type_params
                    .iter()
                    .zip(type_args)
                    .map(|(param, ty)| (param.name.clone(), self.resolve(ty)))
                    .collect();
                let params: Vec<(Mode, Ty)> = decl
                    .params
                    .iter()
                    .map(|param| {
                        let ty = self.resolve_with(&param.ty, &type_params);
                        (param.mode, crate::types::substitute(&ty, &bindings))
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
                // The effective parameter types also fold in the inferred
                // bindings of a generic call, so a slot that resolves to a
                // nullable type is recognized even when the call spells no
                // type arguments. Every other check keeps the declared types,
                // which defer a parameter-typed slot to its instances.
                let mut inferred = false;
                if bindings.is_empty() && !decl.type_params.is_empty() {
                    let mut arg_types: Vec<Ty> = Vec::new();
                    if is_dot {
                        arg_types.push(self.dot_receiver_type(callee));
                    }
                    arg_types.extend(args.iter().map(|arg| self.ty(&arg.value)));
                    if let Some(found) = self.infer_call_types(&name, &arg_types) {
                        for (param, ty) in decl.type_params.iter().zip(found) {
                            bindings.insert(param.name.clone(), ty);
                        }
                        inferred = true;
                    }
                }
                let effective_params: Vec<Ty> = if inferred {
                    decl.params
                        .iter()
                        .map(|param| {
                            let ty = self.resolve_with(&param.ty, &type_params);
                            crate::types::substitute(&ty, &bindings)
                        })
                        .collect()
                } else {
                    params.iter().map(|(_, ty)| ty.clone()).collect()
                };
                self.check_marker_matches(&name, args, written_params);
                let mut lent: BTreeSet<usize> = BTreeSet::new();
                if is_dot {
                    self.check_dot_receiver(
                        callee,
                        params.first(),
                        effective_params.first(),
                        &mut lent,
                    );
                }
                self.check_arguments(
                    args,
                    written_params,
                    &effective_params[skip..],
                    span,
                    &mut lent,
                    if is_dot {
                        params.first().map(|param| (callee, param))
                    } else {
                        None
                    },
                );
                // A slot parameter whose declared type still mentions a type
                // parameter may become a nullable out-slot only at the
                // instance; the audit applies the exact-place rule once the
                // type argument is known.
                for (index, arg) in args.iter().enumerate() {
                    let Some((mode, slot)) = written_params.get(index) else {
                        continue;
                    };
                    if *mode != Mode::Borrow || !crate::types::contains_param(slot) {
                        continue;
                    }
                    self.out_slot_duties.push(ir::RequiresOutSlotPlace {
                        function: self.decl.name.clone(),
                        argument: self.ty(&arg.value),
                        slot: slot.clone(),
                        place: self.place_target(&arg.value).is_some(),
                        span: arg.span,
                    });
                }
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
                // The receiver is read for the call, so a possibly-null
                // receiver reports like any other member read — the check
                // covers the view-consuming intrinsics reached as dot calls
                // (`s.len()`), whose receiver otherwise bypasses the member
                // read path.
                if self.ty(base).nullable {
                    self.check_nonnull_expr(base, callee.span);
                }
            }
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let last = name.segments.len() - 1;
                let path = crate::ast::QualifiedName {
                    segments: name.segments[..last].to_vec(),
                    span: name.span,
                };
                self.eval_qualified(&path, name.span);
                // The receiver is read for the call, so a possibly-null
                // receiver reports like any other member read — the check
                // covers the view-consuming intrinsics reached as dot calls
                // (`s.len()`, `h.s.len()`), whose receiver otherwise bypasses
                // the member read path. The gate tests the receiver path's
                // type: a nullable owner under a non-nullable receiver, or a
                // non-nullable field under a nullable-typed head, is judged
                // by what the call actually reads.
                if self.dot_receiver_type(callee).nullable {
                    let receiver = if path.segments.len() == 1 {
                        Expr {
                            kind: ExprKind::Name(path.segments[0].clone()),
                            span: name.span,
                        }
                    } else {
                        Expr {
                            kind: ExprKind::Qualified(path),
                            span: name.span,
                        }
                    };
                    self.check_nonnull_expr(&receiver, name.span);
                }
            }
            _ => {}
        }
    }

    /// The name a callee carries into the ownership and emission stages.
    ///
    /// A bare name and a member name keep their spelling. A module-qualified
    /// path names the loaded module and its declaration, so it keeps every
    /// segment: the caller's same-named function must never displace it. A
    /// qualified path whose receiver names a lexical binding or a module-level
    /// binding is a member access (a dot call), and its callee is the last
    /// segment; the receiver may itself be a module-path binding, so the whole
    /// receiver path decides the question.
    pub(super) fn callee_name(&self, callee: &Expr) -> Option<String> {
        match &callee.kind {
            ExprKind::Name(name) => Some(name.clone()),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let receiver = &name.segments[..name.segments.len() - 1];
                let receiver_is_value = receiver
                    .first()
                    .is_some_and(|head| self.lookup(head).is_some())
                    || (!(receiver.len() >= 2 && self.loaded.has_module(&receiver.join(".")))
                        && self.dotted_global(receiver).is_some());
                if receiver_is_value {
                    name.segments.last().cloned()
                } else {
                    Some(name.segments.join("."))
                }
            }
            ExprKind::Qualified(name) => name.segments.last().cloned(),
            ExprKind::Member { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    /// The binding a callee name resolves to when that binding is callable.
    ///
    /// A local, a parameter, or a lambda capture can hold a `func` value or a
    /// callable-bound type parameter, and such a call goes through the
    /// binding's own signature — a parameter-typed callee through its bound's.
    /// A module function of the same written name must keep its own identity,
    /// so this resolves through the scope lookup rather than the module tables.
    pub(super) fn func_binding_callee(&self, callee: &Expr) -> Option<usize> {
        let ExprKind::Name(written) = &callee.kind else {
            return None;
        };
        let id = self.lookup(written)?;
        matches!(
            self.bindings[id].ty.kind,
            TyKind::Func(_) | TyKind::Param(_)
        )
        .then_some(id)
    }

    /// Marks an enclosing local written through a lambda capture as unknown.
    ///
    /// A write-borrow capture can replace the local with a view the closure
    /// computed. The closure body is checked in its lifted function, so the
    /// enclosing checker would otherwise keep the local's initial provenance
    /// and miss a later escape. Conservatively marking the local's own frame as
    /// its backing rejects that escape; a later read inside the frame is
    /// unaffected.
    fn mark_written_captures(&mut self, callee: &Expr) {
        if !matches!(
            super::expr::structural_wrapper_inner(callee).kind,
            ExprKind::Lambda { .. }
        ) {
            return;
        }
        let TyKind::Closure(closure) = &self.ty(callee).kind else {
            return;
        };
        let captures = self
            .sema
            .closures
            .get(&closure.id)
            .map(|site| site.captures.clone())
            .unwrap_or_default();
        for capture in captures {
            if capture.mode != CaptureMode::WriteBorrow {
                continue;
            }
            let Some(id) = self.lookup(&capture.name) else {
                continue;
            };
            let binding = &self.bindings[id];
            let owns_frame_storage = binding.kind == BindingKind::Local
                || (binding.kind == BindingKind::Param && binding.mode == Mode::Take);
            if owns_frame_storage {
                self.bindings[id].provenance = Provenance::Local(id);
            }
        }
    }

    /// The parameters of a value callee's signature.
    ///
    /// A `func` type and a callable-bound type parameter carry their modes
    /// directly. A closure's parameters are unmarked read-only lends, exactly
    /// like a lifted function's; the marker check and argument reads use that
    /// spelling.
    fn value_callee_signature(&self, ty: &Ty) -> Option<Vec<(Mode, Ty)>> {
        if let TyKind::Closure(closure) = &ty.kind {
            return Some(
                closure
                    .params
                    .iter()
                    .map(|ty| (Mode::None, ty.clone()))
                    .collect(),
            );
        }
        self.callable_signature(ty)
    }

    /// The parameters of the callable a type spells, when it is one.
    ///
    /// A `func` type carries its parameters and their modes directly. A bare
    /// type parameter is callable exactly when its callable bound is a
    /// `func(...)` signature: the bound's parameter types type the call its
    /// value receives, and a bound declares no modes, so they are read-only
    /// lends. Every other type — a closure, a scalar, a handle — spells no
    /// `func` parameters and yields `None`.
    pub(super) fn callable_signature(&self, ty: &Ty) -> Option<Vec<(Mode, Ty)>> {
        match &ty.kind {
            TyKind::Func(signature) => Some(
                signature
                    .params
                    .iter()
                    .map(|param| (param.mode, param.ty.clone()))
                    .collect(),
            ),
            TyKind::Param(name) => {
                let bound = self
                    .decl
                    .type_params
                    .iter()
                    .find(|param| &param.name == name)?
                    .bound
                    .as_ref()?;
                Some(
                    bound
                        .params
                        .iter()
                        .map(|ty| (Mode::None, self.resolve_with(ty, &self.type_params)))
                        .collect(),
                )
            }
            _ => None,
        }
    }

    /// The parameter modes of the callable a type spells, when it is one.
    pub(super) fn callable_modes(&self, ty: &Ty) -> Option<Vec<Mode>> {
        self.callable_signature(ty)
            .map(|params| params.into_iter().map(|(mode, _)| mode).collect())
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
        effective: &[Ty],
        span: Span,
        lent: &mut BTreeSet<usize>,
        receiver: Option<(&'a Expr, &(Mode, Ty))>,
    ) {
        for (index, arg) in args.iter().enumerate() {
            let expected = params.get(index).map(|(_, ty)| ty.clone());
            let resolved = effective.get(index);
            let arg_ty = self.eval(&arg.value);
            if let Some(expected) = &expected {
                // An inferred parameter that resolved to the null shape is not
                // a type an instance can carry: the argument reports the frozen
                // null rule exactly as a concrete parameter would, instead of
                // deriving a null-shaped instance key. The effective type is
                // the substituted one; the declared type drives every other
                // judgment.
                let null_parameter = resolved.is_some_and(crate::types::is_null_shaped);
                let null_argument = crate::types::is_null_shaped(&arg_ty);
                // A sibling argument can fix the parameter concretely, so the
                // frozen null rule fires at every use site whose instance
                // resolved the parameter to a concrete non-nullable type,
                // exactly as a concrete parameter would; a null-capable
                // resolution keeps the legal null use.
                // Only a concrete resolution counts: a parameter-typed one is
                // judged by the deferred record and its audit, so a nullable
                // instance stays legal and a concrete one reports once.
                let sibling_fixed = resolved.is_some_and(|ty| {
                    !ty.is_unknown() && !ty.nullable && !crate::types::contains_param(ty)
                });
                if null_argument && (null_parameter || sibling_fixed) {
                    self.report_null_rule(arg.span);
                } else {
                    self.check_value_into(&arg.value, expected, arg.span);
                }
            }
            let mode = params.get(index).map(|(mode, _)| *mode).unwrap_or(Mode::None);
            self.check_argument_place(
                &arg.value,
                arg.span,
                mode,
                expected.as_ref(),
                resolved,
                &arg_ty,
                lent,
            );
        }
        self.check_cross_frame_pass_through(args, params, receiver, span);
    }

    /// Judges a view pass-through into a slot that lives further out.
    ///
    /// The callee cannot see the frame relation between one of its lend
    /// parameters and the caller's frame: `fill(b.v, borrow slot)` hands a
    /// view of the caller's local into a slot that belongs to the caller's
    /// caller, and the local dies before that slot's reader runs. The call
    /// site sees both argument relations, so it judges the pass-through
    /// exactly like the direct store: a view whose backing dies with this
    /// frame, at a call that also passes a borrow slot rooted in a further-out
    /// frame, reports one `E10030`. The receiver is parameter zero, so a
    /// dot-call's slot counts too. A parameter-backed view and a same-frame
    /// slot keep their acceptance.
    fn check_cross_frame_pass_through(
        &mut self,
        args: &'a [Arg],
        params: &[(Mode, Ty)],
        receiver: Option<(&'a Expr, &(Mode, Ty))>,
        span: Span,
    ) {
        // A borrow parameter whose type can hold a view is a slot the callee
        // may write one into — a view-like slot directly, or a struct or
        // array slot with a view-like position. The slot's own root decides
        // where it lives: a parameter's storage belongs to this function's
        // caller — a further-out frame — while a local's dies with this frame.
        let mut slot_lives_further_out = false;
        for (index, arg) in args.iter().enumerate() {
            let Some((mode, param_ty)) = params.get(index) else {
                continue;
            };
            if *mode != Mode::Borrow || !self.may_carry_view(param_ty) {
                continue;
            }
            if self.place_roots_in_a_further_out_frame(&arg.value) {
                slot_lives_further_out = true;
                break;
            }
        }
        if !slot_lives_further_out {
            if let Some((callee, (mode, param_ty))) = receiver {
                if *mode == Mode::Borrow && self.may_carry_view(param_ty) {
                    slot_lives_further_out = self.receiver_roots_in_a_further_out_frame(callee);
                }
            }
        }
        if !slot_lives_further_out {
            return;
        }
        // One report per call site: the first view source whose backing dies
        // with this frame names the escape. A dot call's receiver is parameter
        // zero and is not a written argument, so it is judged first.
        let mut failing: Option<Option<usize>> = None;
        if let Some((callee, (mode, param_ty))) = receiver {
            let view_parameter =
                self.may_carry_view(param_ty) || self.may_view_from_storage(param_ty);
            if *mode != Mode::Take || is_view_like(param_ty) {
                if view_parameter {
                    failing = match &callee.kind {
                        ExprKind::Member { base, .. } => self.frame_owned_view_owner(base),
                        // A dotted receiver names the head binding and the
                        // fields selected from it. Judge the receiver's
                        // backing, not the head's lifetime: a view backed by a
                        // parameter, a module global, or static storage stays
                        // accepted, while an owned local aggregate or a view
                        // of this frame's storage reports.
                        ExprKind::Qualified(name) => {
                            self.qualified_receiver_frame_owner(name)
                        }
                        _ => None,
                    };
                }
            }
        }
        for (index, arg) in args.iter().enumerate() {
            if failing.is_some() {
                break;
            }
            // An owned value widens to the view its parameter declares, so the
            // parameter type decides too: `b.v: String` passed as `str` hands
            // the callee a view of `b`'s storage exactly as a `str` argument
            // would.
            // An aggregate parameter can carry a view in one of its fields
            // (`Box { v: String }` passed to a callee that reads `h.v`), so
            // the whole aggregate is a view source for this judgment.
            let Some((mode, param_ty)) = params.get(index) else {
                continue;
            };
            // A take aggregate's storage belongs to the callee, so the
            // callee-side out-slot rule owns the judgment; the caller does not
            // add a second report for the same argument. A taken view is
            // different: its backing is still the caller's storage, so the
            // caller's escape report stays.
            if *mode == Mode::Take && !is_view_like(param_ty) {
                continue;
            }
            let view_parameter =
                self.may_carry_view(param_ty) || self.may_view_from_storage(param_ty);
            if !view_parameter && !is_view_like(&self.ty(&arg.value)) {
                continue;
            }
            let owner = self.frame_owned_view_owner(&arg.value).or_else(|| {
                // A whole aggregate carries no view itself, but a view of one
                // of its retaining fields (`Box.v: String`) derives from the
                // aggregate's storage. A local aggregate dies with this frame,
                // so the callee's out-slot store would dangle; a parameter or
                // a module value outlives the call and stays accepted.
                // Only an owned aggregate takes this fallback. A view-carrying
                // aggregate names its backings through the recorded positions
                // above; a caller-backed backing keeps its acceptance.
                if !is_view_like(&self.ty(&arg.value))
                    && !self.may_carry_view(&self.ty(&arg.value))
                    && self.may_view_from_storage(param_ty)
                    && self.has_owned_viewable_storage(param_ty)
                {
                    self.frame_owned_place_owner(&arg.value)
                } else {
                    None
                }
            });
            if let Some(owner) = owner {
                failing = Some(owner);
                break;
            }
        }
        let Some(failing) = failing else { return };
        let subject = match failing {
            Some(id) => super::liveness::owner_subject(
                &self.bindings[id].name,
                self.bindings[id].generated,
            ),
            None => "a statement temporary".to_owned(),
        };
        self.error(
            "E10030",
            format!("borrow of {subject} does not live long enough"),
            "finish the use before the owner's scope ends",
            span,
        );
    }

    /// True when a place's root binding is a parameter or an out-slot.
    ///
    /// Such a place lives in a further-out frame: a parameter's storage
    /// belongs to this function's caller and an out-slot's to its caller —
    /// either way the frame reading it outlives this one.
    fn place_roots_in_a_further_out_frame(&self, expr: &Expr) -> bool {
        match self.place_target(expr) {
            // A loop alias has no storage of its own; its place reaches the
            // container, whose kind decides whether the slot lives further out.
            Some(PlaceTarget::Local(id, _)) => {
                let id = self.alias_resolved_root(id);
                matches!(
                    self.bindings[id].kind,
                    BindingKind::Param | BindingKind::OutSlot
                )
            }
            _ => false,
        }
    }

    /// True when a dot call's receiver roots in a further-out frame.
    ///
    /// A member receiver is a place like any other; a qualified receiver names
    /// its head binding, so that binding's kind decides.
    fn receiver_roots_in_a_further_out_frame(&self, callee: &Expr) -> bool {
        match &callee.kind {
            ExprKind::Member { base, .. } => self.place_roots_in_a_further_out_frame(base),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let Some(id) = name.segments.first().and_then(|first| self.lookup(first)) else {
                    return false;
                };
                let id = self.alias_resolved_root(id);
                matches!(
                    self.bindings[id].kind,
                    BindingKind::Param | BindingKind::OutSlot
                )
            }
            _ => false,
        }
    }

    /// The frame-owned storage a view argument reads, when it reads any.
    ///
    /// The view's owners are resolved in order: a single-lend call walks its
    /// source; a binding's recorded view positions are authoritative — the
    /// storage its views actually read — so a position loaned to this frame's
    /// storage reports while one loaned to the caller's parameters or module
    /// storage keeps the acceptance; a view-only aggregate with no owned handle
    /// or raw field owns no frame storage of its own; a single view binding
    /// consults the loans its generated or temporary leaf owners record; a
    /// value with no place of its own carries its loans on the value itself; a
    /// binding without recorded positions is judged by its own provenance; and
    /// an ownerless chain that roots at a fresh call result, a struct literal,
    /// or a conditional or array of either reads statement storage, which dies
    /// with the frame. `Some(None)` names statement storage, `Some(Some(id))` a
    /// frame binding, and `None` no frame-owned storage at all.
    pub(super) fn frame_owned_view_owner(&self, expr: &Expr) -> Option<Option<usize>> {
        // A wrapper contributes no storage of its own: an `unsafe` block, a
        // cast, and a conditional each hand back what their inner value reads,
        // so the judgment descends to the wrapped value. A conditional may
        // select either arm, so both are judged.
        match &expr.kind {
            ExprKind::Unsafe(inner) => return self.frame_owned_view_owner(inner),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                return self.frame_owned_view_owner(value);
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                return self
                    .frame_owned_view_owner(then_value)
                    .or_else(|| self.frame_owned_view_owner(else_value));
            }
            // A projection chain rooted at a carrier-returning call reads the
            // returned view positions exactly as the whole call does. The call
            // at the root decides the backing; a chain rooted anywhere else
            // keeps the ordinary place judgment below.
            ExprKind::Member { .. } | ExprKind::Index { .. } => {
                if let Some((callee, args)) = self.carrier_call_root(expr) {
                    if let Some(owner) = self.frame_owned_carrier_source_owner(callee, args) {
                        return Some(owner);
                    }
                }
            }
            // A literal materializes its fields in this frame. A view-carrying
            // position whose value derives from frame storage hands that
            // backing to the destination, so the same judgment descends each
            // such field or element.
            ExprKind::StructLiteral { name, fields, .. } => {
                let struct_name = name.segments.last().cloned().unwrap_or_default();
                let declared = self.instance_fields(&self.ty(expr), &struct_name);
                for field in fields {
                    let position = declared
                        .iter()
                        .find(|(declared, _)| declared == &field.name)
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or_else(|| self.ty(&field.value));
                    if !self.may_carry_view(&position) {
                        continue;
                    }
                    // A field whose provenance already names a frame binding is
                    // reported by the literal's own field judgment; adding the
                    // destination's report would repeat one site. Only an
                    // ownerless derivation — a carrier call behind a
                    // conditional — needs this walk.
                    if self.provenance_dies_with_frame(&self.view_provenance(&field.value)) {
                        continue;
                    }
                    if let Some(owner) = self.frame_owned_view_owner(&field.value) {
                        return Some(owner);
                    }
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                if self.may_carry_view(&self.ty(expr).iterated_element()) {
                    for element in elements {
                        if self.provenance_dies_with_frame(&self.view_provenance(element)) {
                            continue;
                        }
                        if let Some(owner) = self.frame_owned_view_owner(element) {
                            return Some(owner);
                        }
                    }
                    if let Some(fill) = fill {
                        if let Some(owner) = self.frame_owned_view_owner(fill) {
                            return Some(owner);
                        }
                    }
                }
            }
            _ => {}
        }
        // A view returned through a single-lend call reads its source's
        // storage; the source decides whether the backing dies with this
        // frame. Walk it here so a call-mediated view of an inline temporary
        // reports at the caller's out-slot store. A carrier-returning call
        // hands back the view positions of every source argument it may read,
        // so the same walk names the backing.
        if let ExprKind::Call { callee, args, .. } = &expr.kind {
            for source in self.call_view_sources(callee, args) {
                match source {
                    super::place::ViewCallSource::Expr(source) => {
                        if let Some(owner) = self.frame_owned_view_owner(source) {
                            return Some(owner);
                        }
                    }
                    super::place::ViewCallSource::Receiver(provenance)
                    | super::place::ViewCallSource::Capture(provenance) => {
                        if let Some(owner) = self.frame_owner_of_provenance(&provenance) {
                            return Some(owner);
                        }
                    }
                }
            }
            if let Some(owner) = self.frame_owned_carrier_source_owner(callee, args) {
                return Some(owner);
            }
        }
        if let Some(owners) = self.recorded_position_frame_owners(expr) {
            if let Some(owner) = owners.into_iter().next() {
                return Some(self.frame_view_owner(owner));
            }
            // Every recorded view position is known-safe, but an aggregate can
            // still own storage a view reads: a `tag: static str` position
            // accepted above does not cover the owned `name: String` field.
            // The whole-place judgment names the aggregate's frame when it
            // owns reachable storage, and stays silent for a parameter- or
            // module-backed carrier.
            if self.has_owned_viewable_storage(&self.ty(expr)) {
                if let Some(owner) = self.frame_owned_place_owner(expr) {
                    return Some(owner);
                }
            }
            return None;
        }
        // A single view binding records its loans in the binding's loan set,
        // not in the carrier's element-position map. A call-mediated view of a
        // generated owner (`pick([mk().v; ...], 0)`) names that owner here, so
        // the binding's own loans must be judged too.
        if let Some(PlaceTarget::Local(id, _)) = self.place_target(expr) {
            if is_view(&self.bindings[id].ty) {
                if let Some(owner) = self
                    .binding_loan_owners(id)
                    .into_iter()
                    .find(|owner| match owner {
                        // A generated owner is the leaf storage behind a
                        // call-mediated view. A loan of a named local keeps
                        // the existing one-report behavior of its own write
                        // or direct-argument check.
                        LoanOwner::Binding(owner_id) => {
                            self.bindings[*owner_id].generated
                                && self.owner_dies_with_frame(*owner)
                        }
                        LoanOwner::Temp(_) => self.owner_dies_with_frame(*owner),
                    })
                {
                    return Some(self.frame_view_owner(owner));
                }
            }
        }
        // A value that names no place carries its loans on the value itself:
        // a call result reborrows its argument's storage, and a conditional or
        // an array of such results carries every arm's or element's loans.
        // A binding records the same loans as positions above, so only a
        // non-place value consults the carried set here; a binding's union
        // spans every element its container holds and would name storage the
        // selected element does not read.
        if self.place_target(expr).is_none() {
            if let Some(owner) = self.value_loan_frame_owners(expr).into_iter().next() {
                return Some(self.frame_view_owner(owner));
            }
        }
        // An alias reads the iterable element, so its storage is the
        // container's: the container binding's provenance names it.
        let provenance = match self.alias_container_binding(expr) {
            Some(container) => self.place_provenance(container),
            None => self.view_provenance(expr),
        };
        for atom in Self::storage_atoms(&provenance) {
            match atom {
                Provenance::Local(id) | Provenance::Param(id) => {
                    // A projected loop alias (`e.v`) names the iterable's
                    // storage, not the alias binding: resolve it to the
                    // container before the frame judgment.
                    let resolved = self.alias_resolved_root(id);
                    let id = resolved;
                    if self.owner_dies_with_frame(LoanOwner::Binding(id)) {
                        return Some(Some(id));
                    }
                    // A caller-backed component survives this frame, but a
                    // later component or the value's own fresh storage still
                    // reports: the walk continues past it.
                }
                Provenance::Temp(_) => return Some(None),
                _ => {}
            }
        }
        if self.view_reads_fresh_root(expr) {
            return Some(None);
        }
        None
    }

    /// The frame-owned backing a carrier-returning call's sources name.
    ///
    /// Every source the returned positions may read is judged; the first one
    /// whose storage dies with this frame supplies the report. A dot call's
    /// receiver is a source at parameter zero, and a receiver whose own
    /// provenance is static still reads its binding's element loans, so its
    /// place is judged as well.
    fn frame_owned_carrier_source_owner(
        &self,
        callee: &Expr,
        args: &[Arg],
    ) -> Option<Option<usize>> {
        for source in self.call_carrier_sources(callee, args) {
            match source {
                super::place::ViewCallSource::Expr(source) => {
                    if let Some(owner) = self.frame_owned_view_owner(source) {
                        return Some(owner);
                    }
                }
                super::place::ViewCallSource::Receiver(provenance) => {
                    if let Some(owner) = self.frame_owner_of_provenance(&provenance) {
                        return Some(owner);
                    }
                    match &callee.kind {
                        ExprKind::Member { base, .. } => {
                            if let Some(owner) = self.frame_owned_view_owner(base) {
                                return Some(owner);
                            }
                        }
                        ExprKind::Qualified(name) => {
                            if let Some(owner) = self.qualified_receiver_frame_owner(name) {
                                return Some(owner);
                            }
                        }
                        _ => {}
                    }
                }
                // A capturing lambda's returned view reads the captured
                // binding's backing; the enclosing provenance decides whether
                // that storage dies with this frame.
                super::place::ViewCallSource::Capture(provenance) => {
                    if let Some(owner) = self.frame_owner_of_provenance(&provenance) {
                        return Some(owner);
                    }
                }
            }
        }
        None
    }

    /// True when a type contains an owned handle or raw field a view can
    /// retain, so a local aggregate of this type may back a view that dies
    /// with the frame.
    ///
    /// A view-only struct (`str[N]`) names storage it does not own: a
    /// static-only or parameter-backed field contributes no frame owner and
    /// keeps its acceptance. An owned handle (`String`) or a raw field does
    /// own reachable storage.
    fn has_owned_viewable_storage(&self, ty: &Ty) -> bool {
        match &ty.kind {
            TyKind::Handle(_) | TyKind::Raw(_) => true,
            TyKind::Struct(name, _) => self
                .instance_fields(ty, name)
                .iter()
                .any(|(_, field_ty)| self.has_owned_viewable_storage(field_ty)),
            TyKind::Array { element, .. } => self.has_owned_viewable_storage(element),
            _ => false,
        }
    }

    /// True when a type owns storage a view can be derived from.
    ///
    /// A handle (`String`), a view, a raw pointer, a struct containing any of
    /// them, or an array of them is a possible source of a view the callee can
    /// store into an out-slot. The caller-side pass-through check cannot see
    /// which field the callee views, so it treats the owned argument as a
    /// potential source.
    pub(super) fn may_view_from_storage(&self, ty: &Ty) -> bool {
        match &ty.kind {
            TyKind::Str | TyKind::CString | TyKind::StaticStr | TyKind::Handle(_) => true,
            TyKind::Raw(_) => true,
            TyKind::Struct(name, _) => self
                .instance_fields(ty, name)
                .iter()
                .any(|(_, field_ty)| self.may_view_from_storage(field_ty)),
            TyKind::Array { element, .. } => self.may_view_from_storage(element),
            _ => false,
        }
    }

    /// The owner of a place whose storage dies with this frame, when the
    /// place is a local aggregate handed to a view-carrying parameter.
    ///
    /// A parameter or out-slot roots in a further-out frame and stays
    /// accepted; a module value names no binding and owns no frame storage.
    fn frame_owned_place_owner(&self, expr: &Expr) -> Option<Option<usize>> {
        let Some(PlaceTarget::Local(id, _)) = self.place_target(expr) else {
            return None;
        };
        // A `for (borrow e in xs)` alias names the iterable's storage, not its
        // own binding: a parameter- or module-backed container outlives this
        // frame and keeps its acceptance. Resolve the chain for a projected
        // alias too (`e.v`).
        let id = self.alias_resolved_root(id);
        self.owner_dies_with_frame(LoanOwner::Binding(id))
            .then_some(Some(id))
    }

    /// True when a provenance names storage that dies with this frame.
    ///
    /// A local always dies; a parameter dies only when it is a `take` parameter
    /// or a by-value capture. A plain parameter belongs to the caller and stays
    /// live, so a literal field backed by one still needs the destination walk
    /// for its other arm.
    fn provenance_dies_with_frame(&self, provenance: &Provenance) -> bool {
        Self::storage_atoms(provenance).iter().any(|atom| match atom {
            Provenance::Local(_) => true,
            Provenance::Param(id) => self.binding_dies_with_frame(*id),
            _ => false,
        })
    }

    /// The frame-owned storage a provenance names, when it reads any.
    ///
    /// A local or a `take` parameter dies with this frame; a plain or `borrow`
    /// parameter, a module global, and a static literal survive it. A
    /// statement temporary always dies at the end of its statement.
    fn frame_owner_of_provenance(&self, provenance: &Provenance) -> Option<Option<usize>> {
        for atom in Self::storage_atoms(provenance) {
            match atom {
                Provenance::Local(id) | Provenance::Param(id) => {
                    if self.owner_dies_with_frame(LoanOwner::Binding(id)) {
                        return Some(self.frame_view_owner(LoanOwner::Binding(id)));
                    }
                }
                Provenance::Temp(_) => return Some(None),
                _ => {}
            }
        }
        None
    }

    /// The frame-owned backing of a dotted receiver path.
    ///
    /// The head names a binding and the trailing segments select fields from
    /// it. A view receiver is judged by the loans it carries, so a `str` backed
    /// by a parameter, a module global, or static storage stays accepted. An
    /// aggregate is judged by its recorded loans and then by whether it owns
    /// viewable storage; every other receiver is the head's own storage. A
    /// head that resolves to no local is module-level and survives every
    /// frame.
    fn qualified_receiver_frame_owner(
        &self,
        name: &crate::ast::QualifiedName,
    ) -> Option<Option<usize>> {
        let id = name.segments.first().and_then(|first| self.lookup(first))?;
        let id = self.alias_resolved_root(id);
        if !self.owner_dies_with_frame(LoanOwner::Binding(id)) {
            return None;
        }
        // The final segment is the method name, not a field of the receiver.
        let method = name.segments.len().saturating_sub(1);
        let mut ty = self.bindings[id].ty.clone();
        if method > 1 {
            for segment in &name.segments[1..method] {
                ty = self.member_type(&ty, segment);
            }
        }
        if is_view(&ty) || self.may_carry_view(&ty) {
            if let Some(owner) = self
                .binding_loan_owners(id)
                .into_iter()
                .find(|owner| self.owner_dies_with_frame(*owner))
            {
                return Some(self.frame_view_owner(owner));
            }
            if is_view(&ty) || !self.has_owned_viewable_storage(&ty) {
                return None;
            }
        }
        Some(Some(id))
    }

    /// The report subject a frame-dying view owner names.
    ///
    /// A compiler-generated owner is backing the checker created for a
    /// statement's value. It has no written name, so it renders as the same
    /// statement-temporary phrase a lowered temporary does; a written binding
    /// keeps its own identity.
    fn frame_view_owner(&self, owner: LoanOwner) -> Option<usize> {
        match owner {
            LoanOwner::Binding(id) if !self.bindings[id].generated => Some(id),
            _ => None,
        }
    }

    /// True when a view expression reads into fresh statement storage.
    ///
    /// A projection chain that roots at a call whose result owns storage reads
    /// storage the statement owns: `mk().v` names the member of a result that
    /// dies when the statement ends. A conditional or an array contributes
    /// every arm or element, because either may be chosen or stored, and a
    /// struct literal reads the selected position's own value — storing it
    /// directly reads frame storage, while a view-typed position reads the
    /// storage its initializer names. A chain rooted at a place or a call that
    /// owns nothing names no statement storage here.
    fn view_reads_fresh_root(&self, expr: &Expr) -> bool {
        let mut cursor = expr;
        let mut member: Option<&str> = None;
        loop {
            match &cursor.kind {
                ExprKind::Member { base, name } => {
                    member = Some(name);
                    cursor = base;
                }
                ExprKind::Index { base, .. } => {
                    member = None;
                    cursor = base;
                }
                ExprKind::Unsafe(inner) => cursor = inner,
                ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => cursor = value,
                ExprKind::Conditional {
                    then_value,
                    else_value,
                    ..
                } => {
                    return self.view_reads_fresh_root(then_value)
                        || self.view_reads_fresh_root(else_value);
                }
                ExprKind::ArrayLiteral { elements, fill } => {
                    return elements.iter().any(|element| self.view_reads_fresh_root(element))
                        || fill
                            .as_deref()
                            .is_some_and(|fill| self.view_reads_fresh_root(fill));
                }
                ExprKind::Call { .. } => {
                    let ty = self.ty(cursor);
                    return self.owns_drop(&ty) || self.maybe_owns(&ty);
                }
                ExprKind::StructLiteral { fields, .. } => {
                    let selected = member.and_then(|name| {
                        fields
                            .iter()
                            .find(|field| field.name == name)
                            .map(|field| &field.value)
                    });
                    return match selected {
                        // The selected position reads the storage its own
                        // initializer names: a static or caller-backed value is
                        // safe, while a fresh or local-backed one dies with the
                        // frame.
                        Some(value) => self.frame_owned_view_owner(value).is_some(),
                        // A whole-literal or positional read names the
                        // literal's own frame storage.
                        None => true,
                    };
                }
                _ => return false,
            }
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
        effective: Option<&Ty>,
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
                    effective,
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
                    effective,
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

    /// True when a writable lend is a needless lend of a duplicable value.
    ///
    /// The decision uses the parameter's effective type — its declared type
    /// with the call's explicit or inferred substitutions — so a slot that
    /// resolves to a nullable type is recognized as an out-parameter slot no
    /// matter how the argument is spelled. A slot whose declared type is
    /// nullable is exempt for every instance, and a type that still mentions
    /// an unresolved parameter falls back to the argument's type, matching the
    /// declared-parameter behavior.
    fn warns_needless_lend(&self, effective: Option<&Ty>, arg_ty: &Ty) -> bool {
        match effective {
            Some(ty) if !crate::types::contains_param(ty) => {
                !ty.nullable && self.sema.table.duplicable(ty) && !is_view(ty)
            }
            Some(ty) if ty.nullable => false,
            _ => self.sema.table.duplicable(arg_ty) && !is_view(arg_ty),
        }
    }

    /// Applies the ownership effect of one argument.
    ///
    /// `effective` is the parameter's declared type with the call's explicit
    /// or inferred substitutions, when the call resolved one; the warning
    /// decision uses it so a slot that resolves to nullable stays exempt.
    pub(super) fn check_argument_place(
        &mut self,
        expr: &Expr,
        span: Span,
        mode: Mode,
        expected: Option<&Ty>,
        effective: Option<&Ty>,
        arg_ty: &Ty,
        lent: &mut BTreeSet<usize>,
    ) {
        let target = self.place_target(expr);
        match mode {
            Mode::Take => {
                let maybe_owning = self.maybe_owns(arg_ty);
                // A move through a borrow element view is the v1 element-move
                // limit once the moved place owns; while its type is still a
                // parameter the site records the duty for the per-instance
                // audit.
                if let Some(PlaceTarget::Local(id, projections)) = &target {
                    self.note_borrowed_element_move(*id, projections, arg_ty, span);
                }
                // An unresolved conditional argument selects one arm at run
                // time, so each arm is judged per instance: an owned arm is
                // marked for its move (a later use deposits the duplicability
                // duty), and a borrow-element arm records the element-move
                // duty, exactly as its concrete twin judges the arm.
                if arg_ty.is_unknown() && target.is_none() {
                    if let Some(arms) = self.unresolved_conditional_place_arms(expr) {
                        for (source, arm_ty) in arms {
                            if self.root_owns_moves(&source) {
                                self.mark_moved_if_owning(source, span);
                                continue;
                            }
                            if let PlaceTarget::Local(id, projections) = &source {
                                self.note_borrowed_element_move(
                                    *id, projections, &arm_ty, span,
                                );
                            }
                        }
                    }
                }
                // A conditional arm rooted at a type parameter is judged at
                // its instance: a projected member records the lend duty and a
                // whole arm records the maybe-owning move, exactly as the same
                // arm does at a return or a binding.
                for source in self.type_param_conditional_sources(expr) {
                    self.mark_moved_if_owning(source, span);
                }
                // A projection over a conditional names each inner lend arm
                // even when the projected conditional is the whole argument.
                for source in self.maybe_owning_projection_move_sources(expr) {
                    self.mark_moved_if_owning(source, span);
                }
                if !self.sema.table.duplicable(arg_ty) || maybe_owning {
                    if let Some(target) = target {
                        if maybe_owning {
                            self.mark_moved_if_owning(target, span);
                        } else {
                            self.move_target(target, span);
                        }
                    } else {
                        let sources = if maybe_owning {
                            let mut sources = self.maybe_owning_move_sources(expr);
                            if sources.is_empty() {
                                sources = self.maybe_owning_projection_move_sources(expr);
                            }
                            sources
                        } else {
                            let mut sources = self.conditional_move_sources(expr);
                            if sources.is_empty() {
                                sources = self.conditional_projection_move_sources(expr);
                            }
                            sources
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
                if self.warns_needless_lend(effective, arg_ty) {
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
            self.check_argument_place(
                &arg.value,
                arg.span,
                *mode,
                Some(expected),
                Some(expected),
                &arg_ty,
                &mut lent,
            );
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
                    // A view builder reads its source through the view, so a
                    // possibly-null source widens its handle before that read;
                    // the widening is a use and reports like a direct use.
                    if self.ty(&written.value).nullable {
                        self.check_nonnull_expr(&written.value, written.span);
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

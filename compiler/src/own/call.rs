//! Call lowering for the ownership stage.
//!
//! A call lowers to its callee name, its concrete type arguments, and its
//! arguments in signature order. A dot-call carries its receiver as the first
//! argument, matching the free-function signature the specification fixes. An
//! inferred generic call gets its concrete type arguments from the argument
//! types, so the emitter can name the monomorphized instance. The compiler-known
//! view builders are lowered to the intrinsic value the emitter renders
//! directly.

use super::*;
use super::lower::fixed_array_extent;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    /// True when a call's callee is a dot-call with a value receiver.
    ///
    /// A `Member` callee always has one. A `Qualified` callee is a dot-call
    /// when its head names a value in scope, matching how the type stage
    /// decides between member access and a module path.
    pub(super) fn is_dot_call(&self, callee: &Expr) -> bool {
        match &callee.kind {
            ExprKind::Member { .. } => true,
            ExprKind::Qualified(name) => {
                name.segments.len() >= 2
                    && (self.lookup(&name.segments[0]).is_some()
                        || self.resolve_global_id(&name.segments[0]).is_some())
            }
            _ => false,
        }
    }

    /// Lowers the receiver of a dot-call to a place value, when it has one.
    fn lower_dot_receiver(&mut self, callee: &'a Expr) -> Option<Value> {
        match &callee.kind {
            ExprKind::Member { base, .. } => Some(self.lower_value(base)),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let method = name.segments.len() - 1;
                let head = &name.segments[0];
                let projections: Vec<ir::Projection> = name.segments[1..method]
                    .iter()
                    .map(|segment| ir::Projection::Field(segment.clone()))
                    .collect();
                if let Some(id) = self.lookup(head) {
                    let mut ty = self.bindings[id].ty.clone();
                    for segment in &name.segments[1..method] {
                        ty = self.member_type(&ty, segment);
                    }
                    return Some(Value::place(Place {
                        base: self.binding_place(id).base,
                        projections,
                        ty,
                    }));
                }
                if let Some(id) = self.resolve_global_id(head) {
                    let mut ty = self
                        .program
                        .globals
                        .get(&id)
                        .map(|global| global.ty.clone())
                        .unwrap_or_else(Ty::unknown);
                    for segment in &name.segments[1..method] {
                        ty = self.member_type(&ty, segment);
                    }
                    return Some(Value::place(Place {
                        base: ir::PlaceBase::Global(id),
                        projections,
                        ty,
                    }));
                }
                None
            }
            _ => None,
        }
    }

    /// The member type a dot-call callee selects, when it is a field access.
    ///
    /// A `Member` callee is always a field access. A `Qualified` callee is a
    /// field access only when its head names a value in scope; its final
    /// segment is the field and the segments before it are field selections.
    pub(super) fn callee_field_type(&self, callee: &Expr) -> Option<Ty> {
        match &callee.kind {
            ExprKind::Member { base, name } => {
                let base_ty = self.ty(base);
                Some(self.member_type(&base_ty, name))
            }
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let head = name.segments.first()?;
                let mut ty = if let Some(id) = self.lookup(head) {
                    self.bindings[id].ty.clone()
                } else {
                    self.global_type(head)?
                };
                let last = name.segments.len() - 1;
                for segment in &name.segments[1..last] {
                    ty = self.member_type(&ty, segment);
                }
                Some(self.member_type(&ty, &name.segments[last]))
            }
            _ => None,
        }
    }

    /// The callable value of a `func`-typed field call.
    ///
    /// A call such as `c.f(0)` where `f` is a `func` field is a call through
    /// the field, not a method taking the receiver: the field itself is the
    /// callable value. A call whose callee is itself a func-typed call
    /// (`make()(4)`) is called through the returned value the same way.
    /// Returning `None` for every other callee keeps the ordinary dot-call
    /// path unchanged.
    pub(super) fn func_field_receiver(&mut self, callee: &'a Expr) -> Option<Value> {
        // A callee that is itself a call has no stored field type; its own
        // recorded result type decides whether it is called through.
        if matches!(callee.kind, ExprKind::Call { .. }) {
            if matches!(self.ty(callee).kind, TyKind::Func(_)) {
                return Some(self.lower_value(callee));
            }
            return None;
        }
        let field_ty = self.callee_field_type(callee)?;
        // A field whose type is a bare type parameter is callable exactly when
        // its bound is `func(...)`: the checker rejects the unbound call, so an
        // accepted program reaches this lowering through the field's own
        // value.
        if !matches!(field_ty.kind, TyKind::Func(_) | TyKind::Param(_)) {
            return None;
        }
        // The field itself is the callable value. It is built directly rather
        // than through `lower_value`, which would look the field expression's
        // recorded type up and find none: the type stage records the base's
        // type and the call's result, not an internal callee. The receiver
        // aggregate is only read for that field, so its other owning fields
        // need an owner: a fresh literal or call result routes through the
        // ownerless-root binder, whose cleanup releases the siblings. A named
        // place stays untouched, and a conditionally evaluated receiver keeps
        // its deferred form.
        let kind = match &callee.kind {
            ExprKind::Member { base, name } => {
                let base = self.lower_value(base);
                let base = if self.defer_move_outs == 0 {
                    self.bind_ownerless_projection_root(base)
                } else {
                    self.bind_guarded_projection_root(base)
                };
                // The receiver is the call's argument position zero, and the
                // field designator reads it. A receiver whose own rendering
                // nests an arm's fill into the designator settles into a
                // temporary, so its evaluation — a conditional's selected arm,
                // a fresh result — becomes a fill the designator reads. A
                // place, a temporary, and a value the deferred fill already
                // bound keep their spelling: the call reads them directly.
                let settled = match &base.kind {
                    ValueKind::Place(_)
                    | ValueKind::Temp(_)
                    | ValueKind::Materialize { .. } => true,
                    ValueKind::Member { .. } | ValueKind::Index { .. } => {
                        self.projects_from_place(&base)
                    }
                    _ => false,
                };
                let base = if settled {
                    base
                } else {
                    self.materialize_guard_operand(base)
                };
                ValueKind::Member {
                    base: Box::new(base),
                    name: name.clone(),
                }
            }
            ExprKind::Qualified(name) => ValueKind::Path(name.segments.clone()),
            _ => return None,
        };
        Some(Value::new(field_ty, kind))
    }

    /// Lowers a call to its callable value, name, type arguments, and arguments.
    ///
    /// A dot-call's receiver becomes the first argument, matching the
    /// free-function signature; a `func`-typed field call instead carries the
    /// field as its callable value and passes no receiver. An inferred generic
    /// call gets its concrete type arguments from the argument types so the
    /// emitter can name the instance symbol; an explicit list is used as
    /// written.
    pub(super) fn lower_call(
        &mut self,
        callee: &'a Expr,
        type_args: &'a [crate::ast::Type],
        args: &'a [Arg],
    ) -> LoweredCall {
        // A name that resolves to a func-typed local or parameter is called
        // through its binding, so the IR carries the binding's own spelling;
        // that keeps a shadowing or sanitized local distinct from any module
        // function of the same written name.
        let mut local_callee = false;
        let mut local_callee_id: Option<usize> = None;
        let name = match &callee.kind {
            ExprKind::Name(written) => match self.lookup(written) {
                Some(id)
                    if matches!(
                        self.bindings[id].ty.kind,
                        TyKind::Func(_) | TyKind::Param(_)
                    ) =>
                {
                    local_callee = true;
                    local_callee_id = Some(id);
                    // Calling the binding reads it, so an unused-local warning
                    // would be spurious.
                    self.bindings[id].used = true;
                    self.bindings[id].ir_name.clone()
                }
                _ => written.clone(),
            },
            _ => self.callee_name(callee).unwrap_or_default(),
        };
        let receiver = self.func_field_receiver(callee);
        // The dot receiver is the call's argument position zero, so its
        // evaluation position matters exactly like a written argument's: an
        // update in the receiver must run where it is written, and an update in
        // any argument must follow the receiver's read.
        let dot_receiver = match &callee.kind {
            ExprKind::Member { base, .. } => Some(base.as_ref()),
            _ => None,
        };
        let pin_updates = dot_receiver.is_some_and(super::lower::contains_update)
            || args
                .iter()
                .any(|arg| super::lower::contains_update(&arg.value));
        let mut values: Vec<Value> = Vec::new();
        let mut arg_types: Vec<Ty> = Vec::new();
        // The modes come from the callable the call actually reaches: a
        // `func`-typed field or call result, a `func`-typed or callable-bound
        // local, parameter, or capture, a module-level func-typed global, or
        // the declared function the name resolves to. A same-named module
        // function's modes are never applied to a value callee.
        let modes: Option<Vec<Mode>> = if let Some(callable) = &receiver {
            self.callable_modes(&callable.ty)
        } else {
            match local_callee_id {
                Some(id) => self.callable_modes(&self.bindings[id].ty),
                None => self.callee_modes(&name).or_else(|| {
                    self.value_callee_global_type(callee)
                        .and_then(|ty| self.callable_modes(&ty))
                }),
            }
        };
        // The callee expression of a func-field call is that call's position
        // zero: its element locations settle before the written arguments, so
        // a receiver index that updates a later argument's place runs first.
        let receiver = match receiver {
            Some(value) if pin_updates => Some(self.pin_operand_location(value)),
            other => other,
        };
        if receiver.is_none() {
            if let Some(value) = self.lower_dot_receiver(callee) {
                // The dot receiver is the call's argument position zero: its
                // element locations settle here, before any written argument,
                // and an owning receiver binds at the same position exactly as
                // a written argument does. A `take` receiver moves to the
                // callee, so it keeps its place.
                let value = if pin_updates {
                    self.pin_operand_location(value)
                } else {
                    value
                };
                let take = modes
                    .as_ref()
                    .and_then(|modes| modes.first())
                    .is_some_and(|mode| *mode == Mode::Take);
                // A bare update receiver pins wherever it stands: a loop
                // condition defers the pin to the per-iteration fill (no
                // statement before the loop) and a guarded operand keeps its
                // deferred spelling. The eager ownership binding has to be a
                // statement, so it runs only where a statement has a position.
                let eager_binding = self.defer_move_outs == 0 && !self.in_loop_condition;
                let value = if pin_updates {
                    if take {
                        if eager_binding && modes.is_some() {
                            self.materialize_owned_argument(value, true)
                        } else {
                            value
                        }
                    } else {
                        let value = self.pin_update_value(value);
                        if eager_binding && modes.is_some() {
                            self.materialize_owned_argument(value, false)
                        } else {
                            value
                        }
                    }
                } else {
                    value
                };
                arg_types.push(value.ty.clone());
                values.push(value);
            }
        }
        // An update inside an argument runs at its own position: evaluation
        // is strictly left-to-right, and C leaves the order of argument
        // evaluations unspecified. Each position-dependent argument binds to
        // a statement temporary where it is lowered, so a later argument's
        // update never reorders an earlier argument. An owning argument binds
        // through the ownership-safe binding at the same position, because
        // the mode pass below would otherwise run its binding after a later
        // argument's update; a plain copy would carry its storage, so the
        // pin leaves it to that binding. A `take` argument keeps its place —
        // the post-call move-out empties caller storage, which a temporary
        // would hide — and a constant has no position to lose.
        for arg in args {
            let value = self.lower_value(&arg.value);
            // A view-element array argument adopts each temporary element's
            // backing in this frame, exactly as a view binding does: the
            // elements are views of their sources, so a temporary element's
            // backing must live until the call has read the array. A
            // conditionally evaluated argument keeps its element sources in
            // place instead: the backing's call must run only on the path the
            // guard reaches, so nothing is adopted before the guard.
            let value = if self.defer_move_outs == 0
                && matches!(value.kind, ValueKind::ArrayLit { .. })
                && is_view_like(&value.ty)
            {
                self.adopt_view_backings(value)
            } else {
                value
            };
            let value = self.bind_member_receiver(value);
            // The argument's element locations settle at its own position, so
            // an indexed place argument's index update runs before a later
            // argument's pin instead of at the call's guards.
            let value = if pin_updates {
                self.pin_operand_location(value)
            } else {
                value
            };
            let position = values.len();
            let take = modes
                .as_ref()
                .and_then(|modes| modes.get(position))
                .is_some_and(|mode| *mode == Mode::Take);
            // A guarded operand keeps its existing deferred binding path; an
            // eager argument binds here, at its own position. The update pin
            // runs for every update-bearing argument: under a loop condition it
            // defers to the per-iteration fill.
            let eager_binding = self.defer_move_outs == 0 && !self.in_loop_condition;
            let value = if pin_updates {
                if take {
                    // A `take` argument moves to the callee, so the plain pin
                    // would hide the caller's slot; its ownership-safe
                    // binding (the move carrier or the argument temporary)
                    // runs here instead, before a later argument's update.
                    if eager_binding && modes.is_some() {
                        self.materialize_owned_argument(value, true)
                    } else {
                        value
                    }
                } else {
                    let value = self.pin_update_value(value);
                    if eager_binding && modes.is_some() {
                        self.materialize_owned_argument(value, false)
                    } else {
                        value
                    }
                }
            } else {
                value
            };
            arg_types.push(value.ty.clone());
            values.push(value);
        }
        let explicit: Vec<Ty> = type_args.iter().map(|arg| self.resolve(arg)).collect();
        let resolved = if explicit.is_empty() {
            // A func-typed binding has no declared generic parameters, so no
            // type inference runs for its call even when a same-named module
            // function would infer one.
            if local_callee {
                Vec::new()
            } else {
                self.infer_call_types(&name, &arg_types).unwrap_or_default()
            }
        } else {
            explicit
        };
        if let Some(modes) = &modes {
            for (value, mode) in values.iter_mut().zip(modes) {
                let pending =
                    std::mem::replace(value, Value::new(Ty::unknown(), ValueKind::Aggregate));
                *value = self.materialize_owned_argument(pending, *mode == Mode::Take);
            }
        }
        LoweredCall {
            receiver,
            name,
            type_args: resolved,
            args: values,
            modes,
        }
    }

    /// Infers the type arguments of a generic call from its argument types.
    ///
    /// Returns `None` when the callee is not generic or some parameter stays
    /// unbound; the emitter then treats the call as opaque rather than guessing.
    pub(super) fn infer_call_types(&self, name: &str, arg_types: &[Ty]) -> Option<Vec<Ty>> {
        let Some(Callee::User(decl)) = self.program.callee(name) else {
            return None;
        };
        if decl.type_params.is_empty() {
            return None;
        }
        let declared: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        let param_types: Vec<Ty> = decl
            .params
            .iter()
            .map(|param| self.resolve_with(&param.ty, &declared))
            .collect();
        let mut inferred: BTreeMap<String, Ty> = BTreeMap::new();
        for (param, arg) in param_types.iter().zip(arg_types) {
            crate::sema::call::match_param(param, arg, &declared, &mut inferred);
        }
        let mut result = Vec::with_capacity(declared.len());
        for param in &declared {
            result.push(inferred.get(param).cloned()?);
        }
        Some(result)
    }

    /// The structural extent a dot-`len` receiver names, when it names one.
    ///
    /// The receiver of `xs.len()` is a fixed array exactly when its own type
    /// says so; the length is then a compile-time constant and nothing is
    /// bound or evaluated.
    fn structural_len_extent(&self, callee: &Expr) -> Option<usize> {
        match &callee.kind {
            ExprKind::Member { base, .. } => fixed_array_extent(&self.ty(base)),
            ExprKind::Qualified(name) => self.qualified_structural_len(name),
            _ => None,
        }
    }

    /// Lowers a compiler-known intrinsic call, when the callee names one.
    ///
    /// The view builders `rawSlice`, `rawStr`, and `slice` become an intrinsic
    /// value so the emitter can spell the view struct and place the bounds
    /// guard. The dot-call `xs.len()` is the receiver-taking spelling of the
    /// `len` intrinsic, so its receiver becomes the intrinsic's argument.
    /// Returns `None` for every other call, which stays an ordinary call.
    pub(super) fn lower_intrinsic_call(
        &mut self,
        callee: &'a Expr,
        type_args: &'a [crate::ast::Type],
        args: &'a [Arg],
    ) -> Option<(Ty, ValueKind)> {
        let name = self.callee_name(callee)?;
        // Only the compiler-known helper lowers as an intrinsic. A user or
        // `ffi` declaration that happens to share the name must resolve to its
        // own symbol, never be silently replaced by the built-in lowering.
        if matches!(
            self.program.callee(&name),
            Some(Callee::User(_)) | Some(Callee::Ffi(_))
        ) {
            return None;
        }
        // A callable field shadows the built-in name too: `h.len(5)` is a call
        // through the field, not the length intrinsic.
        if let Some(field_ty) = self.callee_field_type(callee) {
            if matches!(field_ty.kind, TyKind::Func(_) | TyKind::Param(_)) {
                return None;
            }
        }
        if name == "len" && type_args.is_empty() {
            // A fixed array's `len` is its structural extent: the length is a
            // compile-time constant, so nothing is bound and the receiver
            // reads no storage.
            if let Some(extent) = self.structural_len_extent(callee) {
                return Some((Ty::usize_ty(), ValueKind::Int(extent as i128)));
            }
            // `xs.len()` is the dot-call spelling of `len(xs)`; the receiver
            // becomes the intrinsic's argument. The intrinsic borrows the
            // receiver, so an ownerless owned result needs the caller-side
            // binding its cleanup uses.
            let receiver = self.lower_dot_receiver(callee)?;
            let receiver = self.bind_intrinsic_argument(receiver);
            return Some((
                Ty::usize_ty(),
                ValueKind::Intrinsic {
                    name: "len".to_owned(),
                    args: vec![receiver],
                    type_arg: None,
                },
            ));
        }
        if name != "rawSlice" && name != "rawStr" && name != "slice" {
            return None;
        }
        let explicit: Vec<Ty> = type_args.iter().map(|arg| self.resolve(arg)).collect();
        let mut lowered: Vec<Value> = args.iter().map(|arg| self.lower_value(&arg.value)).collect();
        if name == "slice" {
            // The slice guard and the slice builder both read the source and
            // the two offsets, so a side-effecting one is stored once.
            for value in lowered.iter_mut() {
                let pending = std::mem::replace(value, Value::new(Ty::unknown(), ValueKind::Aggregate));
                *value = self.materialize_guard_operand(pending);
            }
        }
        // The view's element is one step of iteration over the source: whole
        // rows for a multi-extent source, the scalar for a single-extent one.
        let source_element = lowered
            .first()
            .map(|value| self.materialized_ty(value).iterated_element());
        // An explicit type argument fixes the view's element, so it must name
        // the source's element exactly. The `slice` intrinsic is registered
        // without a declared signature, so the agreement check runs here, at
        // the call site, and reports the ordinary implicit-conversion error.
        if name == "slice" {
            if let (Some(explicit_ty), Some(source)) = (explicit.first(), lowered.first()) {
                if let Some(source_element) = &source_element {
                    if !source_element.is_unknown() && explicit_ty != source_element {
                        if self.report_diagnostics {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    "E10200",
                                    format!(
                                        "cannot convert '{}' to '{}' implicitly",
                                        self.materialized_ty(source).display(),
                                        view_type(explicit_ty.clone()).display()
                                    ),
                                    type_args[0].span,
                                )
                                .with_help("use an explicit cast `T(expr)`"),
                            );
                        }
                    }
                }
            }
        }
        let type_arg = explicit
            .first()
            .cloned()
            .or_else(|| source_element.filter(|element| !element.is_unknown()))
            .or_else(|| match &lowered.first()?.ty.kind {
                TyKind::Raw(inner) => Some((**inner).clone()),
                _ => None,
            });
        let is_raw_str = name == "rawStr";
        let result = if is_raw_str {
            Ty {
                kind: TyKind::Str,
                nullable: false,
            }
        } else {
            view_type(type_arg.clone().unwrap_or_else(Ty::unknown))
        };
        Some((
            result,
            ValueKind::Intrinsic {
                name,
                args: lowered,
                type_arg: if is_raw_str { None } else { type_arg },
            },
        ))
    }

    /// The declared parameter modes of a known user or `ffi` function.
    pub(super) fn callee_modes(&self, name: &str) -> Option<Vec<Mode>> {
        match self.program.callee(name) {
            Some(Callee::User(decl)) => Some(decl.params.iter().map(|param| param.mode).collect()),
            Some(Callee::Ffi(signature)) => {
                Some(signature.params.iter().map(|param| param.mode).collect())
            }
            _ => None,
        }
    }

    /// The places a call moves out of through non-duplicable `take` arguments.
    ///
    /// The modes are the callee's declared parameter modes: a module or `ffi`
    /// callee's from its declaration, and a value callee's — a `func`-typed
    /// binding, field, call result, or module global — from its own signature.
    /// A duplicable `take`
    /// argument is copied, so the caller keeps its value; a handle or
    /// aggregate is moved and its place must be emptied after the call. A
    /// plain place is the caller's binding; a conditional argument (or any
    /// materialized operand) travels through a compiler temporary, and the
    /// caller's copy of the value in that temporary is emptied the same way
    /// once the callee owns it. A fresh call result already transfers
    /// ownership and has no caller storage to clear.
    pub(super) fn take_move_out_places(
        &self,
        modes: Option<&[Mode]>,
        args: &[Value],
    ) -> Vec<Place> {
        let Some(modes) = modes else {
            return Vec::new();
        };
        let mut places = Vec::new();
        for (arg, mode) in args.iter().zip(modes) {
            // A maybe-owning argument travels as a move for an owning instance
            // and as a copy for a duplicable one; the emitted move-out chooses
            // by the substituted type.
            if *mode != Mode::Take
                || (self.sema.table.duplicable(&arg.ty) && !self.maybe_owns(&arg.ty))
            {
                continue;
            }
            match &arg.kind {
                ValueKind::Place(place) => places.push(place.clone()),
                ValueKind::Temp(temp) | ValueKind::Materialize { temp, .. } => {
                    places.push(Place {
                        base: ir::PlaceBase::Temp(*temp),
                        projections: Vec::new(),
                        ty: arg.ty.clone(),
                    });
                }
                // A projected place rooted in caller storage (`p.v`, `xs[0]`)
                // is emptied the same way; a projection over a fresh call
                // result has no caller storage and is skipped.
                _ => {
                    if let Some(place) = self.value_place(arg) {
                        places.push(place);
                    }
                }
            }
        }
        places
    }

}

/// A call lowered for the IR: its callable value, symbol name, concrete type
/// arguments, and arguments.
pub(super) struct LoweredCall {
    /// The `func`-typed field that is called, when the callee is a field.
    pub receiver: Option<Value>,
    /// Simple name of the callee, for symbol resolution.
    pub name: String,
    /// Concrete type arguments of a generic call.
    pub type_args: Vec<Ty>,
    /// Lowered arguments in source order.
    pub args: Vec<Value>,
    /// The callee's declared parameter modes, when this call has any.
    ///
    /// A module or `ffi` callee's modes come from its declaration; a value
    /// callee's — a `func`-typed binding, field, call result, or module
    /// global — from its own signature. The modes drive the argument
    /// materialization and the post-call move-outs, so a value call treats
    /// its owned temporary arguments and `take` arguments exactly like a
    /// module call's.
    pub modes: Option<Vec<Mode>>,
}

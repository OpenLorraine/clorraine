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
        if !matches!(field_ty.kind, TyKind::Func(_)) {
            return None;
        }
        // The field itself is the callable value. It is built directly rather
        // than through `lower_value`, which would look the field expression's
        // recorded type up and find none: the type stage records the base's
        // type and the call's result, not an internal callee.
        let kind = match &callee.kind {
            ExprKind::Member { base, name } => ValueKind::Member {
                base: Box::new(self.lower_value(base)),
                name: name.clone(),
            },
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
        let name = match &callee.kind {
            ExprKind::Name(written) => match self.lookup(written) {
                Some(id) if matches!(self.bindings[id].ty.kind, TyKind::Func(_)) => {
                    local_callee = true;
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
        let mut values: Vec<Value> = Vec::new();
        let mut arg_types: Vec<Ty> = Vec::new();
        if receiver.is_none() {
            if let Some(value) = self.lower_dot_receiver(callee) {
                arg_types.push(value.ty.clone());
                values.push(value);
            }
        }
        for arg in args {
            let value = self.lower_value(&arg.value);
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
        // A call result the callee only borrows stays owned by the caller; an
        // unnamed one is bound to a scope local so the cleanup drops it, since
        // the callee never does. A `take` argument is skipped: the callee owns
        // it, and the move-out empties the caller's binding. A call through a
        // `func`-typed field has no declared parameter modes, so the same-named
        // global function's modes must not be applied to it; a func-typed
        // local or parameter has none either.
        if receiver.is_none() && !local_callee {
            if let Some(modes) = self.callee_modes(&name) {
                for (value, mode) in values.iter_mut().zip(modes) {
                    let pending =
                        std::mem::replace(value, Value::new(Ty::unknown(), ValueKind::Aggregate));
                    *value = self.materialize_owned_argument(pending, mode == Mode::Take);
                }
            }
        }
        LoweredCall {
            receiver,
            name,
            type_args: resolved,
            args: values,
        }
    }

    /// Infers the type arguments of a generic call from its argument types.
    ///
    /// Returns `None` when the callee is not generic or some parameter stays
    /// unbound; the emitter then treats the call as opaque rather than guessing.
    fn infer_call_types(&self, name: &str, arg_types: &[Ty]) -> Option<Vec<Ty>> {
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
            .map(|param| resolve_ty(&param.ty, &declared, &self.sema.table))
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
        if name == "len" && type_args.is_empty() {
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
                                        Ty::array(explicit_ty.clone(), vec![None]).display()
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
            Ty::array(type_arg.clone().unwrap_or_else(Ty::unknown), vec![None])
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
    /// A duplicable `take` argument is copied, so the caller keeps its value;
    /// a handle or aggregate is moved and its place must be emptied after the
    /// call. A plain place is the caller's binding; a conditional argument (or
    /// any materialized operand) travels through a compiler temporary, and the
    /// caller's copy of the value in that temporary is emptied the same way
    /// once the callee owns it. A fresh call result already transfers ownership
    /// and has no caller storage to clear.
    pub(super) fn take_move_out_places(&self, name: &str, args: &[Value]) -> Vec<Place> {
        let Some(modes) = self.callee_modes(name) else {
            return Vec::new();
        };
        let mut places = Vec::new();
        for (arg, mode) in args.iter().zip(modes) {
            // A maybe-owning argument travels as a move for an owning instance
            // and as a copy for a duplicable one; the emitted move-out chooses
            // by the substituted type.
            if mode != Mode::Take
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
}

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
use super::place::deref_rooted;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    /// The length of the longest leading module path in a qualified name.
    ///
    /// A module path must name a loaded module; a single-segment module (`m`)
    /// counts as much as a nested one (`a.b`). The search runs from the longest
    /// candidate down so a nested module wins over its parent's prefix.
    pub(super) fn module_prefix_len(&self, segments: &[String]) -> Option<usize> {
        (1..segments.len())
            .rev()
            .find(|split| self.loaded.has_module(&segments[..*split].join(".")))
    }

    /// True when a call's callee is a dot-call with a value receiver.
    ///
    /// A `Member` callee always has one. A `Qualified` callee is a dot-call
    /// when the leading segments name a binding and a later segment is the
    /// called member. A module path that reaches the final segment directly
    /// names a module declaration instead, even when an imported value shares
    /// the head's spelling; a lexical binding of the head always shadows the
    /// path. When the module prefix ends before the final segment, the next
    /// segment is a module binding and the rest are its members, so the call
    /// is a dot-call on that binding.
    pub(super) fn is_dot_call(&self, callee: &Expr) -> bool {
        match &callee.kind {
            ExprKind::Member { .. } => true,
            ExprKind::Qualified(name) => {
                if name.segments.len() < 2 {
                    return false;
                }
                let head = &name.segments[0];
                if self.lookup(head).is_some() {
                    return true;
                }
                if let Some(split) = self.module_prefix_len(&name.segments) {
                    return split + 1 < name.segments.len();
                }
                // No module path: the head is a module-level value and the
                // rest are its members. A path that resolves no binding stays
                // on the ordinary unresolved route.
                self.resolve_global_id(head).is_some()
                    || self
                        .dotted_global(&name.segments[..name.segments.len() - 1])
                        .is_some()
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
                // The module path wins for the receiver exactly as it does for
                // a read (Ch 10 MOD-13); a lexical binding still shadows it.
                if self.lookup(head).is_none() {
                    // A receiver path that names a loaded module is a module
                    // path, not a binding: a parent module's same-spelled
                    // export must not become the receiver of the call.
                    if name.segments[..method].len() >= 2
                        && self.loaded.has_module(&name.segments[..method].join("."))
                    {
                        return None;
                    }
                    if let Some((id, fields)) = self.dotted_global(&name.segments[..method]) {
                        if let Some(global_ty) =
                            self.program.globals.get(&id).map(|global| global.ty.clone())
                        {
                            let mut ty = global_ty;
                            for field in &fields {
                                ty = self.member_type(&ty, field);
                            }
                            let projections = fields
                                .iter()
                                .map(|field| ir::Projection::Field(field.clone()))
                                .collect();
                            return Some(Value::place(Place {
                                base: ir::PlaceBase::Global(id),
                                projections,
                                ty,
                            }));
                        }
                    }
                }
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
                // A module-qualified declaration is not a field access, even
                // when an imported value shares the head's spelling. The
                // module path wins, exactly as it does for a read.
                if !self.is_dot_call(callee) {
                    return None;
                }
                let last = name.segments.len() - 1;
                let ty = self.receiver_path_type(&name.segments[..last])?;
                Some(self.member_type(&ty, &name.segments[last]))
            }
            _ => None,
        }
    }

    /// The type of a dotted receiver path.
    ///
    /// A local or module binding is the head and the rest are its fields. A
    /// path whose leading segments spell a module resolves the binding after
    /// them, so `m.a.holder` reads module `m.a`'s binding even though the path
    /// itself mentions no name.
    fn receiver_path_type(&self, segments: &[String]) -> Option<Ty> {
        let (first, rest) = segments.split_first()?;
        // A receiver path that names a loaded module is a module qualifier, not
        // a binding, unless a lexical binding shadows the head.
        if self.lookup(first).is_none() && self.loaded.has_module(&segments.join(".")) {
            return None;
        }
        // The module path wins for a receiver exactly as it does for a read
        // (Ch 10 MOD-13); a lexical binding of the head's name still shadows
        // the path.
        if self.lookup(first).is_none() {
            if let Some((id, fields)) = self.dotted_global(segments) {
                if let Some(ty) = self.program.globals.get(&id).map(|global| global.ty.clone()) {
                    let mut ty = ty;
                    for field in &fields {
                        ty = self.member_type(&ty, field);
                    }
                    return Some(ty);
                }
            }
        }
        if let Some(id) = self.lookup(first) {
            let mut ty = self.bindings[id].ty.clone();
            for segment in rest {
                ty = self.member_type(&ty, segment);
            }
            return Some(ty);
        }
        if let Some(ty) = self.global_type(first) {
            let mut ty = ty;
            for segment in rest {
                ty = self.member_type(&ty, segment);
            }
            return Some(ty);
        }
        None
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
        // An indexed callee such as `a[0](3)` is the element value the access
        // yields: the access's recorded type is the callable, exactly as a
        // called field's is, and the checker rejects the call of an element
        // that is not callable.
        if matches!(callee.kind, ExprKind::Index { .. }) {
            if matches!(
                self.ty(callee).kind,
                TyKind::Func(_) | TyKind::Param(_)
            ) {
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
                // temporary, so its evaluation — a fresh result, or a
                // conditional of fresh results — becomes a fill the designator
                // reads. A place, a temporary, and a value the deferred fill
                // already bound keep their spelling, and a conditional whose
                // reachable arms borrow places stays inline: settling it would
                // move the chosen arm out of the place it borrows, while the
                // call only reads the callable field.
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
                } else if self.conditional_borrows_place(&base) {
                    // A receiver conditional that borrows a place keeps the
                    // place a read. Outside a loop condition each ownerless arm
                    // binds its own temporary the statement releases, so the
                    // fresh result does not leak and the designator reads the
                    // selected arm. A loop condition keeps its deferred
                    // spelling: the condition's own per-iteration placement
                    // owns those fills, and no statement can be hoisted out of
                    // it.
                    if self.in_loop_condition {
                        base
                    } else {
                        self.hoist_ownerless_conditional_arms(base)
                    }
                } else {
                    self.materialize_guard_operand(base)
                };
                ValueKind::Member {
                    base: Box::new(base),
                    name: name.clone(),
                }
            }
            ExprKind::Qualified(name) => {
                // The receiver resolves module-aware and the last segment is
                // the called member, exactly as the checker splits the two:
                // `m.holder.g` reads the module path's binding `holder` and
                // calls its field `g`, while a plain head keeps its binding.
                let last = name.segments.len() - 1;
                let mut segments = self.global_head_segments(&name.segments[..last]);
                segments.push(name.segments[last].clone());
                ValueKind::Path(segments)
            }
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
        // A callee that resolves to a module-level func global carries the
        // binding's identity: the declaring module is spelled with the name,
        // so two modules exporting the same-named global stay apart. A local
        // binding, a declared or `ffi` function, a library helper, and a
        // callable field keep their own spelling.
        let name = if local_callee {
            name
        } else {
            self.func_global_callee_path(callee).unwrap_or(name)
        };
        // A callee expression with no name but a callable type — a conditional
        // over func values, a cast, or an indexed callable — is the callable
        // itself. Lower it as a value and attach it as the call's receiver, so
        // the emitter calls the value the callee produced instead of spelling
        // the raw expression as if it were a function name. A closure value
        // keeps its own two-word call path.
        let expression_callee = self.callee_name(callee).is_none()
            && !self.is_closure_callee(callee)
            && self.callable_modes(&self.ty(callee)).is_some();
        let receiver = if expression_callee {
            Some(self.lower_value(callee))
        } else {
            self.func_field_receiver(callee)
        };
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
        if receiver.is_none() && self.is_dot_call(callee) {
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
        // Every value position with a literal that needs a statement store is
        // bound to a temporary where it stands: a call argument or receiver
        // has no C value form for an order-mismatched literal, and materializing
        // it here keeps the written field order on every path.
        let mut value_literal = |value: Value| {
            if matches!(
                value.kind,
                ValueKind::StructLit { .. } | ValueKind::ArrayLit { .. }
            ) && self.literal_fields_need_stores(&value)
            {
                self.materialize_guard_literal(value)
            } else {
                value
            }
        };
        let receiver = receiver.map(&mut value_literal);
        for value in values.iter_mut() {
            let pending =
                std::mem::replace(value, Value::new(Ty::unknown(), ValueKind::Aggregate));
            *value = value_literal(pending);
        }
        LoweredCall {
            receiver,
            name,
            type_args: resolved,
            args: values,
            modes,
        }
    }

    /// The identity-qualified spelling of a module-level func-global callee.
    ///
    /// A call whose callee is a whole module-level binding of function type
    /// reaches that binding, exactly as a read of it does, so its declaring
    /// module is spelled with the name. Two modules may export the same-named
    /// global, and the qualified spelling keeps the call on the binding the
    /// front end resolved instead of a name match. A function the caller's
    /// module declares or one of its imports binds is the callee instead; a
    /// function of the same simple name elsewhere is not in scope, so it never
    /// displaces the binding. A declared or `ffi` function, a library helper,
    /// a callable field, and a callee that names no whole binding yield
    /// `None`.
    fn func_global_callee_path(&self, callee: &Expr) -> Option<String> {
        // A function bound in the caller's module or by one of its imports
        // wins over a module-level value of the same written name. A
        // qualified path names its binding directly, so the same-named
        // function of another module cannot shadow it.
        if let ExprKind::Name(written) = &callee.kind {
            if self.function_in_scope(written) {
                return None;
            }
        }
        let ty = self.value_callee_global_type(callee)?;
        if !matches!(ty.kind, TyKind::Func(_)) {
            return None;
        }
        let id = match &callee.kind {
            ExprKind::Name(written) => self.resolve_global_id(written)?,
            ExprKind::Qualified(name) => {
                let global = crate::init_graph::resolve_global(self.loaded, self.module, name)?;
                ir::GlobalId::new(global.module, global.name)
            }
            _ => return None,
        };
        Some(format!("{}.{}", id.module, id.name))
    }

    /// True when the caller's own module declares a function of the name, or
    /// one of the caller's imports binds the name to a function.
    ///
    /// The scope rule call resolution follows: a function bound in the
    /// caller's module or by its imports is the callee, while a function of
    /// the same simple name elsewhere is not in scope. An `ffi` declaration
    /// is a function for this purpose, matching the checker.
    fn function_in_scope(&self, name: &str) -> bool {
        let is_function = |decl: &crate::load::Decl| {
            matches!(
                decl.kind,
                crate::load::DeclKind::Func | crate::load::DeclKind::Ffi
            )
        };
        if self.module.decls.get(name).is_some_and(is_function) {
            return true;
        }
        self.module.import(name).is_some_and(|binding| {
            self.loaded
                .module(&binding.module)
                .and_then(|target| target.decls.get(&binding.original))
                .is_some_and(is_function)
        })
    }

    /// Infers the type arguments of a generic call from its argument types.
    ///
    /// Returns `None` when the callee is not generic or some parameter stays
    /// unbound; the emitter then treats the call as opaque rather than guessing.
    pub(super) fn infer_call_types(&self, name: &str, arg_types: &[Ty]) -> Option<Vec<Ty>> {
        let Some(Callee::User(decl)) = self.resolved_callee(name) else {
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
        crate::sema::call::infer_type_args(&declared, &param_types, arg_types)
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
        let written = self.callee_name(callee)?;
        // An imported alias names its original declaration. The intrinsic
        // resolver keys on the declaration name, while the shadow check still
        // uses the written spelling so an alias to a user function keeps its
        // own symbol.
        let resolved = if written.contains('.') {
            written.clone()
        } else {
            self.module
                .import(&written)
                .filter(|binding| !binding.poisoned)
                .map(|binding| binding.original.clone())
                .unwrap_or_else(|| written.clone())
        };
        // A module-qualified spelling keeps its declaration identity for the
        // shadow check, while the compiler-known helper is named by its last
        // segment.
        let name = resolved
            .rsplit_once('.')
            .map(|(_, simple)| simple.to_owned())
            .unwrap_or_else(|| resolved.clone());
        // Only the compiler-known helper lowers as an intrinsic. A user or
        // `ffi` declaration that happens to share the name must resolve to its
        // own symbol, never be silently replaced by the built-in lowering.
        for candidate in [&written, &resolved] {
            if matches!(
                self.resolved_callee(candidate),
                Some(Callee::User(_)) | Some(Callee::Ffi(_))
            ) {
                return None;
            }
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
                if let ExprKind::Member { base, .. } = &callee.kind {
                    if !super::is_pure_place(base) {
                        if matches!(base.kind, ExprKind::Call { .. }) {
                            let value = self.lower_value(base);
                            let value = self.materialize_guard_operand(value);
                            return Some((
                                Ty::usize_ty(),
                                ValueKind::Intrinsic {
                                    name: "len".to_owned(),
                                    args: vec![value],
                                    type_arg: None,
                                },
                            ));
                        }
                        if self.defer_move_outs == 0 && !self.in_loop_condition {
                            self.evaluate_len_operand_effects(base);
                        }
                    }
                }
                return Some((Ty::usize_ty(), ValueKind::Int(extent as i128)));
            }
            // A qualified free call (`mod.len(value)`) takes its argument
            // directly; the dot spelling (`xs.len()`) takes the receiver.
            if !self.is_dot_call(callee) {
                // A fixed-array place has a structural extent: folding it
                // reads no storage. Skipping the lowering keeps an
                // otherwise-unused local unused, so the emitter still marks
                // it and the strict C build stays warning-free. A call or
                // literal is still lowered because it may have effects or
                // own a temporary.
                if let Some(arg) = args.first() {
                    if let Some(extent) = fixed_array_extent(&self.ty(&arg.value)) {
                        if super::is_pure_place(&arg.value) {
                            return Some((Ty::usize_ty(), ValueKind::Int(extent as i128)));
                        }
                        // A call operand may have effects. Lower it through
                        // the guard materialization so a conditional or
                        // short-circuit operand runs only on the path that
                        // reaches this expression, and an owning result is
                        // released there. The emitter reads the extent from
                        // the fixed-array type.
                        //
                        // A conditional, array literal, or run-time index is
                        // evaluated effects-only: a fresh arm or element is
                        // discarded path-aware, while a caller place is never
                        // moved by the structural fold. The evaluated position
                        // must be unconditional; a conditionally evaluated
                        // operand keeps the fold and its effects are the
                        // recorded residual.
                        if matches!(arg.value.kind, ExprKind::Call { .. }) {
                            let value = self.lower_value(&arg.value);
                            let value = self.materialize_guard_operand(value);
                            return Some((
                                Ty::usize_ty(),
                                ValueKind::Intrinsic {
                                    name: "len".to_owned(),
                                    args: vec![value],
                                    type_arg: None,
                                },
                            ));
                        }
                        // A conditional, array literal, or run-time index
                        // has effects the extent fold would drop. In an
                        // unconditional position, discard the operand first
                        // so the effects run at this point and only the
                        // selected arm or elements evaluate. A statement in a
                        // conditionally evaluated position would run on a path
                        // that skips the call, so that shape keeps the fold.
                        if self.defer_move_outs == 0 && !self.in_loop_condition {
                            self.evaluate_len_operand_effects(&arg.value);
                        }
                        return Some((Ty::usize_ty(), ValueKind::Int(extent as i128)));
                    }
                }
                let value = match args.first() {
                    Some(arg) => self.lower_value(&arg.value),
                    None => Value::new(Ty::unknown(), ValueKind::Aggregate),
                };
                let value = self.bind_intrinsic_argument(value);
                return Some((
                    Ty::usize_ty(),
                    ValueKind::Intrinsic {
                        name: "len".to_owned(),
                        args: vec![value],
                        type_arg: None,
                    },
                ));
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

    /// The declaration a written callee name resolves to, by scope.
    ///
    /// The written name resolves the way the checker resolves it: the caller's
    /// own module first, then an import binding under its original name, and
    /// only then the program-wide simple-name map. A module-qualified name is
    /// resolved exactly instead: its leading segments name the module and its
    /// last segment the declaration there, so a same-named function of the
    /// caller's module never displaces it. Every mode and ownership judgment of
    /// the call reads this one resolution, so an aliased or same-named spelling
    /// judges exactly like its plain twin.
    pub(super) fn resolved_callee(&self, name: &str) -> Option<Callee<'a>> {
        if let Some((module, simple)) = name.rsplit_once('.') {
            if let Some(target) = self.loaded.module(module) {
                if let Some(decl) = declared_function(target, simple) {
                    return Some(Callee::User(decl));
                }
                if let Some(signature) = declared_ffi(target, simple) {
                    return Some(Callee::Ffi(signature));
                }
            }
        }
        if let Some(decl) = declared_function(self.module, name) {
            return Some(Callee::User(decl));
        }
        if let Some(signature) = declared_ffi(self.module, name) {
            return Some(Callee::Ffi(signature));
        }
        if let Some(binding) = self.module.import(name) {
            if !binding.poisoned {
                if let Some(target) = self.loaded.module(&binding.module) {
                    if let Some(decl) = declared_function(target, &binding.original) {
                        return Some(Callee::User(decl));
                    }
                    if let Some(signature) = declared_ffi(target, &binding.original) {
                        return Some(Callee::Ffi(signature));
                    }
                }
            }
        }
        self.program.callee(name)
    }

    /// The declared parameter modes of a known user or `ffi` function.
    pub(super) fn callee_modes(&self, name: &str) -> Option<Vec<Mode>> {
        match self.resolved_callee(name) {
            Some(Callee::User(decl)) => Some(decl.params.iter().map(|param| param.mode).collect()),
            Some(Callee::Ffi(signature)) => {
                Some(signature.params.iter().map(|param| param.mode).collect())
            }
            _ => None,
        }
    }

    /// The `take` arguments whose pointee the callee consumes.
    ///
    /// A `take` argument that reads through a raw pointer names no place the
    /// ordinary move-out can empty, so its value travels here and the emitter
    /// empties the pointee through the pointer after the call, exactly as the
    /// store source is emptied. A projected member (`(*p).name`) records the
    /// member value, not the whole pointee, so the emitter empties only the
    /// selected member and the pointee's sibling fields stay owned. A
    /// duplicable pointee copies and the emitted emptying is a no-op.
    pub(super) fn take_pointee_move_outs(
        &self,
        modes: Option<&[Mode]>,
        args: &[Value],
    ) -> Vec<Value> {
        let Some(modes) = modes else {
            return Vec::new();
        };
        let mut values = Vec::new();
        for (arg, mode) in args.iter().zip(modes) {
            if *mode == Mode::Take && deref_rooted(arg) {
                values.push(arg.clone());
            }
        }
        values
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
            // by the substituted type. An argument recorded with an unknown
            // type takes the same route: its substituted shape may own once
            // the instance is known, so the move-out is emitted and the
            // emptied check decides per instance.
            let maybe_owning = self.maybe_owns(&arg.ty) || arg.ty.is_unknown();
            if *mode != Mode::Take
                || (self.sema.table.duplicable(&arg.ty) && !maybe_owning)
            {
                continue;
            }
            match &arg.kind {
                ValueKind::Place(place) => places.push(place.clone()),
                ValueKind::Materialize {
                    destination: Some(place),
                    ..
                } => places.push(place.clone()),
                ValueKind::Temp(temp)
                | ValueKind::Materialize {
                    temp: Some(temp),
                    ..
                } => {
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

/// The `func` declaration a module declares under a name.
fn declared_function<'a>(module: &'a crate::load::Module, name: &str) -> Option<&'a FuncDecl> {
    module.items.iter().find_map(|item| match item {
        crate::ast::Item::Func(decl) if decl.name == name => Some(decl),
        _ => None,
    })
}

/// The `ffi` signature a module declares under a name.
fn declared_ffi<'a>(
    module: &'a crate::load::Module,
    name: &str,
) -> Option<&'a crate::ast::FuncSignature> {
    module.items.iter().find_map(|item| match item {
        crate::ast::Item::Ffi(block) => block.decls.iter().find(|decl| decl.name == name),
        _ => None,
    })
}

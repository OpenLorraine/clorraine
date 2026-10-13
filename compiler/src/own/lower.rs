//! Expression lowering and temporary materialization for the ownership stage.
//!
//! The lowering turns a typed expression into the typed control-flow IR value
//! the emitter renders. A side-effecting guarded operand is stored in a
//! compiler temporary so the guard and the operation read the same value, and
//! an owned call result the callee only borrows is bound to a scope local so
//! the scope cleanup releases it.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    /// Lowers an expression to a typed IR value.
    ///
    /// Every value records the type the type stage computed for its expression,
    /// and a form the checker does not need to name explicitly becomes a fresh
    /// temporary whose type joins the function's temporary list.
    pub(super) fn lower_value(&mut self, expr: &'a Expr) -> Value {
        let mut ty = self.ty(expr);
        let kind = match &expr.kind {
            ExprKind::Int(value) => ValueKind::Int(*value as i128),
            ExprKind::Float(value) => ValueKind::Float(*value),
            ExprKind::Char(byte) => ValueKind::Char(*byte),
            ExprKind::Bool(value) => ValueKind::Bool(*value),
            ExprKind::Null => ValueKind::Null,
            ExprKind::Str(bytes) => ValueKind::Str(String::from_utf8_lossy(bytes).into_owned()),
            ExprKind::Name(name) => match self.place_target(expr) {
                Some(target) => ValueKind::Place(self.target_place(&target)),
                None => ValueKind::Path(vec![name.clone()]),
            },
            ExprKind::Qualified(name) => {
                // A fixed array's `.len` is its structural extent: the length
                // is a compile-time constant and the base reads no storage,
                // exactly as the `len` intrinsic folds it.
                if let Some(extent) = self.qualified_structural_len(name) {
                    return Value {
                        ty: Ty::usize_ty(),
                        kind: ValueKind::Int(extent as i128),
                    };
                }
                match self.place_target(expr) {
                    Some(target) => ValueKind::Place(self.target_place(&target)),
                    None => ValueKind::Path(name.segments.clone()),
                }
            }
            ExprKind::Member { base, name } => {
                // A fixed array's `.len` is its structural extent: the length
                // is a compile-time constant and the base reads no storage,
                // exactly as the `len` intrinsic folds it.
                if name == "len" {
                    if let TyKind::Array { extents, .. } = &self.ty(base).kind {
                        if extents.iter().all(Option::is_some) {
                            let extent = extents.first().and_then(|size| *size).unwrap_or(0);
                            return Value {
                                ty: Ty::usize_ty(),
                                kind: ValueKind::Int(extent as i128),
                            };
                        }
                    }
                }
                ValueKind::Member {
                    base: Box::new(self.lower_value(base)),
                    name: name.clone(),
                }
            }
            ExprKind::Index { base, index } => {
                // The guard and the access both read these operands. A
                // side-effecting operand (a call such as `a[next()]`) is
                // stored once so the guard and the access inspect the same
                // value instead of re-running the call.
                // A conditional base is materialized into a temporary by the
                // read path, and that store moves the chosen owned arm.
                self.mark_read_conditional_move(base);
                let base = self.lower_value(base);
                let index = self.lower_value(index);
                let base = self.materialize_guard_operand(base);
                let base = self.materialize_read_place(base);
                let index = self.materialize_guard_operand(index);
                // The type stage does not type the element of a direct
                // `slice(...)` call, so the access type can still be unknown.
                // The settled base knows its element type, and the element one
                // step yields is the accessed type; recovering it lets the
                // move-out path see the element's drop need instead of skipping
                // it as unknown. A view's element is the same step the emitted
                // access yields, so the recovery always agrees with the view.
                if ty.is_unknown() {
                    let element = self.materialized_ty(&base).iterated_element();
                    if !element.is_unknown() {
                        ty = element;
                    }
                }
                ValueKind::Index {
                    base: Box::new(base),
                    index: Box::new(index),
                }
            }
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                // A callee that lowers to a closure value is the ephemeral
                // two-word callable, not a named function: it is called through
                // its function pointer with the environment as the first
                // argument. A generic body's `f(x)` is typed by a parameter and
                // reaches the same shape only after instantiation, so the
                // emitter handles that case instead.
                if self.is_closure_callee(callee) {
                    let callable = self.lower_value(callee);
                    // A closure's parameters are unmarked, so they are
                    // read-only lends exactly like a module function's: an
                    // owned temporary argument binds to a scope owner the
                    // cleanup releases, since the lifted function never does.
                    let values: Vec<Value> = args
                        .iter()
                        .map(|arg| {
                            let value = self.lower_value(&arg.value);
                            self.materialize_owned_argument(value, false)
                        })
                        .collect();
                    ty = self.ty(expr);
                    ValueKind::ClosureCall {
                        callable: Box::new(callable),
                        args: values,
                    }
                } else if let Some((intrinsic_ty, intrinsic_kind)) =
                    self.lower_intrinsic_call(callee, type_args, args)
                {
                    ty = intrinsic_ty;
                    intrinsic_kind
                } else {
                    let call = self.lower_call(callee, type_args, args);
                    // A call consumes the places its callee's own signature
                    // marks `take`, whether the callee is a declared function,
                    // a func-typed binding, field, or global, or a call result.
                    let move_outs =
                        self.take_move_out_places(call.modes.as_deref(), &call.args);
                    let kind = ValueKind::Call {
                        callee: call.name,
                        type_args: call.type_args,
                        args: call.args,
                        receiver: call.receiver.map(Box::new),
                    };
                    if move_outs.is_empty() {
                        kind
                    } else if self.defer_move_outs > 0 {
                        // The operand is evaluated only on its guard's path,
                        // so the consumed places must not be emptied now: the
                        // move-outs travel on the materialized value and the
                        // guard emitter runs them after the call, on the
                        // evaluated path only. The temporary joins the
                        // statement's release set, so a path that discards the
                        // value releases it exactly once; an adopting
                        // destination moves the value out and empties the
                        // temporary, which makes that release a no-op.
                        let temp = self.new_temp(ty.clone());
                        self.register_pending_release(temp, &ty);
                        let value = Value::new(ty.clone(), kind);
                        return Value::new(
                            ty,
                            ValueKind::Materialize {
                                temp,
                                value: Box::new(value),
                                move_outs,
                                adopting: false,
                            },
                        );
                    } else {
                        // The call consumes a `take` argument before the
                        // enclosing statement continues, so the result is
                        // stored in a destination temporary the statement end
                        // releases, and the consumed places are emptied
                        // immediately afterward. An adopting store moves the
                        // whole value out of the temporary (emptying it, so
                        // the release is a no-op); a projection of the
                        // temporary reads it and the release drops the fields
                        // the take did not move out.
                        let temp =
                            self.materialize_value_type(Value::new(ty.clone(), kind), ty.clone());
                        for place in move_outs {
                            self.emit(Statement::MoveOut { place });
                        }
                        return temp;
                    }
                }
            }
            ExprKind::Unary { op, operand } => ValueKind::Unary {
                op: *op,
                operand: Box::new(self.lower_value(operand)),
            },
            ExprKind::Binary { op, left, right } => {
                // A division guard re-reads both operands, so a
                // side-effecting one is stored once and the guard and the
                // operation share it.
                let left = self.lower_value(left);
                // The right operand of `&&`/`||` runs only when the left
                // decides to evaluate it, so a `take` move-out inside it must
                // wait for that path.
                let right = if matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                    self.defer_move_outs += 1;
                    let right = self.lower_value(right);
                    self.defer_move_outs -= 1;
                    right
                } else {
                    self.lower_value(right)
                };
                let (left, right) = if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                    (
                        self.materialize_guard_operand(left),
                        self.materialize_guard_operand(right),
                    )
                } else {
                    (left, right)
                };
                ValueKind::Binary {
                    op: *op,
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
                // A float-to-integer cast guard re-reads its operand, so a
                // side-effecting one is stored once.
                let inner = self.lower_value(value);
                let inner = self.bind_member_receiver(inner);
                let inner_ty = self.ty(value);
                let inner = if ty.is_integer() && inner_ty.is_numeric() && !inner_ty.is_integer() {
                    self.materialize_guard_operand(inner)
                } else {
                    inner
                };
                ValueKind::Cast {
                    to: ty.display(),
                    value: Box::new(inner),
                }
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let condition = self.lower_value(condition);
                // Each arm runs only when the condition selects it, so a
                // `take` move-out inside an arm must wait for that path.
                self.defer_move_outs += 1;
                let then_value = self.lower_value(then_value);
                let then_value = self.defer_discarded_arm(then_value);
                let else_value = self.lower_value(else_value);
                let else_value = self.defer_discarded_arm(else_value);
                self.defer_move_outs -= 1;
                ValueKind::Conditional {
                    condition: Box::new(condition),
                    then_value: Box::new(then_value),
                    else_value: Box::new(else_value),
                }
            }
            ExprKind::StructLiteral { name, fields, .. } => {
                // A literal inside a conditionally evaluated operand must not
                // move its field sources while the enclosing expression is
                // lowered: only the chosen arm runs, and the arm's store empties
                // the source it moves. An eagerly emitted move would leave the
                // skipped arm's value in an unreleased temporary.
                let deferred = self.defer_move_outs > 0;
                let mut lowered = Vec::with_capacity(fields.len());
                for field in fields {
                    let value = self.lower_value(&field.value);
                    let value = if deferred {
                        value
                    } else {
                        self.consume_place(value)
                    };
                    lowered.push((field.name.clone(), value));
                }
                ValueKind::StructLit {
                    name: name.segments.last().cloned().unwrap_or_default(),
                    fields: lowered,
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => ValueKind::ArrayLit {
                // As for a struct literal, a deferred operand leaves its
                // element sources in place; the arm's store moves the chosen
                // source and empties it. A view-element array keeps its
                // sources in place too: an element is a view of its source,
                // so the binding loans or adopts the backing rather than
                // moving it out of the source.
                elements: elements
                    .iter()
                    .map(|element| {
                        let value = self.lower_value(element);
                        if self.defer_move_outs > 0 || self.view_array_literal_depth > 0 {
                            value
                        } else {
                            self.consume_place(value)
                        }
                    })
                    .collect(),
                fill: fill
                    .as_ref()
                    .map(|fill| Box::new(self.lower_value(fill))),
            },
            ExprKind::Unsafe(inner) => return self.lower_value(inner),
            ExprKind::IntrinsicCall { name, args } => {
                // A fixed array's `len` is its structural extent: the length is
                // a compile-time constant, so nothing is bound and the
                // argument reads no storage.
                if name == "len" {
                    if let Some(IntrinsicArg::Expr(inner)) = args.first() {
                        if let Some(extent) = fixed_array_extent(&self.ty(inner)) {
                            return Value {
                                ty: Ty::usize_ty(),
                                kind: ValueKind::Int(extent as i128),
                            };
                        }
                    }
                }
                let mut values = Vec::new();
                let mut type_arg = None;
                for arg in args {
                    match arg {
                        IntrinsicArg::Expr(inner) => {
                            // An intrinsic only borrows its expression
                            // arguments, so an unnamed owned result passed here
                            // needs the caller-side binding its cleanup uses.
                            // A fixed array's `len` is structural: the length
                            // needs no storage, so the argument keeps its form
                            // and nothing runs for it.
                            let value = self.lower_value(inner);
                            let value = self.bind_member_receiver(value);
                            let value = if name == "len" && value.ty.is_fixed_array() {
                                value
                            } else {
                                self.bind_intrinsic_argument(value)
                            };
                            values.push(value);
                        }
                        IntrinsicArg::Type(written) => type_arg = Some(self.resolve(written)),
                    }
                }
                ValueKind::Intrinsic {
                    name: name.clone(),
                    args: values,
                    type_arg,
                }
            }
            ExprKind::Template(pieces) => ValueKind::Template(
                pieces
                    .iter()
                    .map(|piece| match piece {
                        TemplatePiece::Literal(bytes) => ValuePiece::Literal(
                            String::from_utf8_lossy(bytes).into_owned(),
                        ),
                        TemplatePiece::Placeholder(inner) => {
                            // An owned placeholder is copied into the template
                            // builder, so the source is released by the scope
                            // cleanup; otherwise it leaks. A view or scalar
                            // placeholder owns nothing and is left inline.
                            let value = self.lower_value(inner);
                            ValuePiece::Placeholder(self.materialize_owned_argument(value, false))
                        }
                    })
                    .collect(),
            ),
            ExprKind::Lambda { params, body, .. } => {
                return self.lower_lambda(expr, &ty, params, body);
            }
            _ => ValueKind::Aggregate,
        };
        Value { ty, kind }
    }

    /// Lowers a lambda expression to its lifted function and closure value.
    ///
    /// The body becomes a file-scope `static` C function. A capture-less lambda
    /// is a plain function pointer, so its value is the lifted function's name.
    /// A capturing lambda is a two-word value whose environment holds one
    /// expression per capture; the value's captures are read or addressed in
    /// the enclosing frame at the point the lambda is written.
    fn lower_lambda(
        &mut self,
        expr: &Expr,
        ty: &Ty,
        params: &'a [LambdaParam],
        body: &'a LambdaBody,
    ) -> Value {
        // A capturing lambda's site comes from the semantic layer, which also
        // recorded its capture list. A capture-less lambda has no site there,
        // so it takes the next free lifted id; the enclosing stage reserves ids
        // above every recorded capturing site.
        let (site, capturing) = match &ty.kind {
            TyKind::Closure(closure) => (closure.id, true),
            _ => {
                let site = *self.next_lifted;
                *self.next_lifted += 1;
                (site, false)
            }
        };
        // The lambda's own signature fixes the lifted function. A capturing
        // closure carries it on its type; a capture-less one is a `func`.
        let (param_tys, ret) = match &ty.kind {
            TyKind::Closure(closure) => (closure.params.clone(), (*closure.returns).clone()),
            TyKind::Func(signature) => (
                signature.params.iter().map(|param| param.ty.clone()).collect(),
                (*signature.returns).clone(),
            ),
            _ => (Vec::new(), Ty::void()),
        };
        self.lift_lambda(site, params, &param_tys, ret.clone(), body);

        let captures = if capturing {
            self.lower_captures(site, expr.span)
        } else {
            Vec::new()
        };
        Value::new(ty.clone(), ValueKind::Closure { site, captures })
    }

    /// Lifts every capture-less lambda a module initializer spells.
    ///
    /// A module initializer has no enclosing body, so a lambda used there has
    /// no lift; this walk gives each one a site from the same counter the body
    /// path uses and lowers it through [`Self::lift_lambda`]. The emitter
    /// resolves the initializer's callable through the returned node-address
    /// map. A capture entry can only name a local (none exist in an
    /// initializer) or a module-level name (already `E10406`), so only
    /// capture-less lambdas reach the lift.
    pub(super) fn lift_initializer_lambdas(
        &mut self,
        values: &'a [&'a Expr],
    ) -> (Vec<(usize, usize)>, Vec<ir::Function>) {
        let mut sites = Vec::new();
        for value in values {
            self.collect_initializer_lambdas(value, &mut sites);
        }
        (sites, std::mem::take(&mut self.lifted))
    }

    /// Records and lifts the lambda literals one initializer expression spells.
    ///
    /// A lambda body is not walked: the lift lowers it with the same body
    /// machinery, so its own lambdas lift in place.
    fn collect_initializer_lambdas(&mut self, expr: &'a Expr, sites: &mut Vec<(usize, usize)>) {
        if let ExprKind::Lambda { params, body, .. } = &expr.kind {
            let ty = self.ty(expr);
            if let TyKind::Func(signature) = &ty.kind {
                let site = *self.next_lifted;
                *self.next_lifted += 1;
                let param_tys: Vec<Ty> = signature
                    .params
                    .iter()
                    .map(|param| param.ty.clone())
                    .collect();
                let ret = (*signature.returns).clone();
                sites.push((expr as *const Expr as usize, site));
                self.lift_lambda(site, params, &param_tys, ret, body);
            }
            return;
        }
        match &expr.kind {
            ExprKind::Call { callee, args, .. } => {
                self.collect_initializer_lambdas(callee, sites);
                for arg in args {
                    self.collect_initializer_lambdas(&arg.value, sites);
                }
            }
            ExprKind::Index { base, index } => {
                self.collect_initializer_lambdas(base, sites);
                self.collect_initializer_lambdas(index, sites);
            }
            ExprKind::Member { base, .. } => self.collect_initializer_lambdas(base, sites),
            ExprKind::Unary { operand, .. } => self.collect_initializer_lambdas(operand, sites),
            ExprKind::Binary { left, right, .. } => {
                self.collect_initializer_lambdas(left, sites);
                self.collect_initializer_lambdas(right, sites);
            }
            ExprKind::Assign { target, value, .. } => {
                self.collect_initializer_lambdas(target, sites);
                self.collect_initializer_lambdas(value, sites);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.collect_initializer_lambdas(condition, sites);
                self.collect_initializer_lambdas(then_value, sites);
                self.collect_initializer_lambdas(else_value, sites);
            }
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
                self.collect_initializer_lambdas(value, sites)
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.collect_initializer_lambdas(&field.value, sites);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.collect_initializer_lambdas(element, sites);
                }
                if let Some(fill) = fill {
                    self.collect_initializer_lambdas(fill, sites);
                }
            }
            ExprKind::Unsafe(inner) => self.collect_initializer_lambdas(inner, sites),
            ExprKind::IntrinsicCall { args, .. } => {
                for arg in args {
                    if let crate::ast::IntrinsicArg::Expr(inner) = arg {
                        self.collect_initializer_lambdas(inner, sites);
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let crate::ast::TemplatePiece::Placeholder(inner) = piece {
                        self.collect_initializer_lambdas(inner, sites);
                    }
                }
            }
            _ => {}
        }
    }

    /// Reports the write-mutability errors a module initializer carries.
    ///
    /// A module initializer has no enclosing body, so the statement path's
    /// checks never see it: a write through a `const` module binding or an
    /// element of an immutable view is judged here with the same rules and
    /// messages the function path reports. Only updates and assignments are
    /// judged, and a lambda body is skipped because the lift checks it with
    /// the ordinary body rules.
    pub(super) fn check_initializer_writes(&mut self, values: &'a [&'a Expr]) {
        for value in values {
            self.walk_initializer_writes(value);
        }
    }

    /// Reports a write through a `const` module binding or an immutable view.
    ///
    /// Mirrors the update path's module-level judgment: a `const` binding
    /// reports `E10023` first, and the immutable-view element rule runs
    /// otherwise, so a rejected write keeps exactly one diagnostic.
    fn check_initializer_write_target(&mut self, expr: &'a Expr) {
        let Some(PlaceTarget::Global(id, projections)) = self.place_target(expr) else {
            return;
        };
        let is_const = self
            .program
            .globals
            .get(&id)
            .is_some_and(|global| global.is_const);
        // A field update through a `raw` global reaches the pointee, not the
        // constant pointer binding.
        let through_pointer = self.writes_through_raw_global(&id, &projections);
        if is_const && !through_pointer {
            self.error(
                "E10023",
                format!("cannot assign to immutable '{}'", id.name),
                "declare it with `let` (mutable)",
                expr.span,
            );
            return;
        }
        self.check_global_immutable_view_write(&id, &projections, expr.span);
    }

    /// Walks one initializer expression for its write forms.
    fn walk_initializer_writes(&mut self, expr: &'a Expr) {
        match &expr.kind {
            ExprKind::Unary { op, operand } if op.is_update() => {
                if self.structural_len_target(operand) {
                    // A fixed array's structural `.len` names no storage, so
                    // the update reports the body's own rejection.
                    self.reject_structural_len_target(operand);
                } else {
                    self.check_initializer_write_target(operand);
                }
                self.walk_initializer_writes(operand);
            }
            ExprKind::Unary { operand, .. } => self.walk_initializer_writes(operand),
            ExprKind::Assign { target, value, .. } => {
                self.check_initializer_write_target(target);
                self.walk_initializer_writes(target);
                self.walk_initializer_writes(value);
            }
            ExprKind::Call { callee, args, .. } => {
                self.walk_initializer_writes(callee);
                for arg in args {
                    self.walk_initializer_writes(&arg.value);
                }
            }
            ExprKind::Index { base, index } => {
                self.walk_initializer_writes(base);
                self.walk_initializer_writes(index);
            }
            ExprKind::Member { base, .. } => self.walk_initializer_writes(base),
            ExprKind::Binary { left, right, .. } => {
                self.walk_initializer_writes(left);
                self.walk_initializer_writes(right);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.walk_initializer_writes(condition);
                self.walk_initializer_writes(then_value);
                self.walk_initializer_writes(else_value);
            }
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
                self.walk_initializer_writes(value)
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.walk_initializer_writes(&field.value);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.walk_initializer_writes(element);
                }
                if let Some(fill) = fill {
                    self.walk_initializer_writes(fill);
                }
            }
            ExprKind::Unsafe(inner) => self.walk_initializer_writes(inner),
            ExprKind::IntrinsicCall { args, .. } => {
                for arg in args {
                    if let crate::ast::IntrinsicArg::Expr(inner) = arg {
                        self.walk_initializer_writes(inner);
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let crate::ast::TemplatePiece::Placeholder(inner) = piece {
                        self.walk_initializer_writes(inner);
                    }
                }
            }
            _ => {}
        }
    }

    /// Lowers one capture expression per recorded capture, in field order.
    ///
    /// A `Copy` capture reads the enclosing binding's value, so the environment
    /// holds its own copy. A borrow capture takes the enclosing place's address,
    /// so the lifted function reads or writes the original owner; a
    /// non-duplicable capture is therefore never moved.
    fn lower_captures(&mut self, site: usize, span: Span) -> Vec<Value> {
        let Some(closure) = self.sema.closures.get(&site) else {
            return Vec::new();
        };
        let infos = closure.captures.clone();
        let mut captures = Vec::new();
        for info in &infos {
            let Some(id) = self.lookup(&info.name) else {
                continue;
            };
            let place = self.binding_place(id);
            match info.mode {
                CaptureMode::Copy => {
                    self.read_place(id, &[], span);
                    captures.push(Value::place(place));
                }
                CaptureMode::ReadOnlyBorrow | CaptureMode::WriteBorrow => {
                    self.bindings[id].used = true;
                    for loan in self.binding_loans(id) {
                        self.record_event(LoanEventKind::Used { loan }, span);
                    }
                    captures.push(Value::new(
                        Ty::raw(info.ty.clone()),
                        ValueKind::Unary {
                            op: UnaryOp::AddressOf,
                            operand: Box::new(Value::place(place)),
                        },
                    ));
                }
            }
        }
        captures
    }

    /// Builds the lifted IR function for one lambda body and records it.
    ///
    /// The checker state is saved, replaced with a fresh function context, and
    /// restored afterward, so the enclosing body's graph, scopes, and binding
    /// table are untouched. Each captured name is bound to a place rooted at a
    /// new [`ir::PlaceBase::EnvField`], so reads and writes render through the
    /// environment and a `[borrow]` capture reaches the enclosing frame.
    fn lift_lambda(
        &mut self,
        site: usize,
        params: &'a [LambdaParam],
        param_tys: &[Ty],
        ret: Ty,
        body: &'a LambdaBody,
    ) {
        let lambda_params: Vec<ir::Param> = params
            .iter()
            .zip(param_tys)
            .map(|(param, ty)| ir::Param {
                name: param.name.clone(),
                ty: ty.clone(),
                mode: Mode::None,
            })
            .collect();
        let saved_builder = std::mem::replace(
            &mut self.builder,
            Builder::new(format!("cl_lambda_{site}"), lambda_params, ret.clone()),
        );
        let saved_scopes = std::mem::take(&mut self.scopes);
        let saved_bindings = std::mem::take(&mut self.bindings);
        let saved_state = std::mem::replace(&mut self.state, FlowState::default());
        let saved_temps = std::mem::take(&mut self.temps);
        let saved_destinations = std::mem::take(&mut self.destination_temps);
        let saved_releases = std::mem::take(&mut self.pending_releases);
        let saved_exits = std::mem::take(&mut self.exits);
        let saved_defers = std::mem::take(&mut self.defer_entries);
        let saved_events = std::mem::take(&mut self.events);
        let saved_ordinals = std::mem::take(&mut self.next_ordinal);
        let saved_loans = std::mem::take(&mut self.loan_ids);
        let saved_names = std::mem::take(&mut self.binding_names);
        let saved_state_reports = std::mem::take(&mut self.state_reports);
        let saved_continues = std::mem::take(&mut self.continue_states);
        let saved_defer = self.defer_anchor;
        let saved_move_outs = self.defer_move_outs;
        let saved_loop_condition = self.in_loop_condition;
        let saved_registration = self.next_registration;
        let saved_owned_temp = self.next_owned_temp;
        let saved_returns = self.returns.clone();
        let saved_current_span = self.current_span;
        let saved_elision_candidate = self.elision_candidate.clone();
        let saved_derivation_mismatch = self.return_derivation_mismatch;
        let saved_view_array_depth = self.view_array_literal_depth;
        let saved_view_destination = self.view_destination_adopts;
        let saved_discard_conditional = self.discard_conditional_depth;
        self.next_registration = 0;
        self.next_owned_temp = 0;
        self.defer_anchor = None;
        // A lifted lambda body is evaluated when the closure is called, not
        // where the closure literal is written, so the enclosing operand's
        // deferred move-out and loop-condition contexts do not apply inside
        // it.
        self.defer_move_outs = 0;
        self.in_loop_condition = false;
        self.returns = ret.clone();
        // The elision judgment belongs to the enclosing function, not to a
        // lifted lambda body; the body's own view returns must not decide it.
        self.return_derivation_mismatch = false;
        self.view_array_literal_depth = 0;
        // A lifted lambda body is a function of its own, so its local
        // destinations may adopt their view backings again.
        self.view_destination_adopts = true;
        self.discard_conditional_depth = 0;

        let body_span = match body {
            LambdaBody::Expr(value) => value.span,
            LambdaBody::Block(block) => block.span,
        };
        self.push_scope(body_span);
        for (param, ty) in params.iter().zip(param_tys) {
            self.declare_lambda_param(&param.name, ty.clone());
        }
        self.declare_captures(site);
        match body {
            LambdaBody::Expr(value) => {
                // An expression body is the lifted function's return, so it
                // owes the same move and lend-storage judgement a `return`
                // statement runs before its lowering: without it an owned
                // value would escape without a source move, and a lend
                // parameter stored or returned through the body would slip
                // past the rule that rejects its block-bodied spelling.
                self.eval(value);
                self.judge_return_moves(value, value.span);
                let returned = self.lower_value(value);
                if !self.is_terminated() {
                    // Mirror `Stmt::Return`: the scope cleanup becomes a
                    // cleanup ladder the emitter runs after the returned
                    // value has been read. Running the cleanup before the
                    // terminator would release the temporary the returned
                    // expression still reads.
                    self.emit_return_cleanup();
                    let releases = self.pending_release_copies();
                    self.record_release_events(&releases);
                    self.terminate(Terminator::Return {
                        value: Some(returned),
                        releases,
                    });
                }
                // The cleanup, when it ran, already emitted the scope's drops;
                // popping without the cleanup keeps them from being emitted
                // twice.
                self.scopes.pop();
            }
            LambdaBody::Block(block) => {
                self.check_block(block);
                self.pop_scope();
                if !self.is_terminated() {
                    let releases = self.pending_release_copies();
                    self.record_release_events(&releases);
                    self.terminate(Terminator::Return {
                        value: None,
                        releases,
                    });
                }
            }
        }

        let lambda_builder = std::mem::replace(
            &mut self.builder,
            Builder::new("__cl_lifted_placeholder", Vec::new(), Ty::void()),
        );
        let mut function = lambda_builder.finish();
        Self::sync_param_names(&mut function, &self.bindings);
        function.locals = self
            .bindings
            .iter()
            // A base-overridden binding (a lambda capture or a borrow-element
            // alias) never declares a C local: its places root at the override.
            .filter(|binding| binding.kind == BindingKind::Local && binding.base.is_none())
            .map(|binding| ir::Local {
                name: binding.ir_name.clone(),
                ty: binding.ty.clone(),
            })
            .collect();
        function.temps = self.temps.clone();
        function.module = self.module.name.clone();
        function.lambda_site = Some(site);
        function.enclosing = Some(self.decl.name.clone());
        let generated = self.generated_owner_ids();
        liveness::check(
            &function,
            &self.events,
            &self.binding_names,
            &generated,
            self.diagnostics,
        );
        // Only a discarded speculative pass skips the record: that pass lowers
        // the body with owned-temporary materialization suppressed, and its IR
        // is re-lowered by the real pass. A `defer` registration lowering also
        // reports no diagnostics but emits its own IR, so its record — the one
        // the emitted body's call sites reference — must be kept.
        if !self.speculative_lowering {
            self.lifted.push(function);
        }

        self.builder = saved_builder;
        self.scopes = saved_scopes;
        self.bindings = saved_bindings;
        self.state = saved_state;
        self.temps = saved_temps;
        self.destination_temps = saved_destinations;
        self.pending_releases = saved_releases;
        self.exits = saved_exits;
        self.defer_entries = saved_defers;
        self.events = saved_events;
        self.next_ordinal = saved_ordinals;
        self.loan_ids = saved_loans;
        self.binding_names = saved_names;
        self.state_reports = saved_state_reports;
        self.continue_states = saved_continues;
        self.defer_anchor = saved_defer;
        self.defer_move_outs = saved_move_outs;
        self.in_loop_condition = saved_loop_condition;
        self.next_registration = saved_registration;
        self.next_owned_temp = saved_owned_temp;
        self.returns = saved_returns;
        self.current_span = saved_current_span;
        self.elision_candidate = saved_elision_candidate;
        self.return_derivation_mismatch = saved_derivation_mismatch;
        self.view_array_literal_depth = saved_view_array_depth;
        self.view_destination_adopts = saved_view_destination;
        self.discard_conditional_depth = saved_discard_conditional;
    }

    /// Declares one lambda parameter in the lifted function.
    fn declare_lambda_param(&mut self, name: &str, ty: Ty) {
        let id = self.declare(
            name,
            Span::default(),
            ty,
            BindingKind::Param,
            Mode::None,
            false,
            false,
            true,
        );
        self.set_place(
            id,
            PlaceState {
                initialized: true,
                ..PlaceState::default()
            },
        );
    }

    /// Binds each recorded capture to its environment field.
    ///
    /// A `Copy` capture is a by-value field; a read-only or writable borrow is
    /// a pointer field, and a writable borrow is declared `borrow` so the body
    /// may assign through it.
    fn declare_captures(&mut self, site: usize) {
        let infos: Vec<CaptureInfo> = self
            .sema
            .closures
            .get(&site)
            .map(|closure| closure.captures.clone())
            .unwrap_or_default();
        for info in infos {
            let by_pointer = !matches!(info.mode, CaptureMode::Copy);
            let mode = match info.mode {
                CaptureMode::WriteBorrow => Mode::Borrow,
                _ => Mode::None,
            };
            let base = ir::PlaceBase::EnvField {
                field: info.name.clone(),
                by_pointer,
                ty: info.ty.clone(),
            };
            let id = self.bindings.len();
            let scope = self.scopes.len();
            let registration = self.register_cleanup();
            self.bindings.push(Binding {
                name: info.name.clone(),
                // A capture's places root at its environment field, so its IR
                // spelling is never read; keep the source name for diagnostics.
                ir_name: info.name.clone(),
                ty: info.ty.clone(),
                span: Span::default(),
                used: true,
                scope,
                kind: BindingKind::Param,
                mode,
                is_const: false,
                needs_drop: false,
                registration,
                provenance: Provenance::None,
                base: Some(base),
                generated: false,
            });
            if let Some(scope) = self.scopes.last_mut() {
                scope.names.insert(info.name.clone(), id);
                scope.order.push(id);
            }
            self.set_place(
                id,
                PlaceState {
                    initialized: true,
                    ..PlaceState::default()
                },
            );
        }
    }

    /// Lowers a discarded expression statement, keeping any side effect it has.
    ///
    /// An expression statement is normally written for its side effect (Ch 05
    /// §4), so a call or update embedded in a wider expression must still run
    /// even though the result is dropped (`f() + 1;` calls `f`). The whole value
    /// is lowered and emitted as a discard, which preserves the short-circuit
    /// behavior of `&&`, `||`, and `?:` instead of running an embedded effect
    /// ahead of its guard. A value with no side effect lowers to nothing.
    ///
    /// A conditional lowers with discard-aware arms: either arm's fresh owning
    /// result materializes into a statement temporary the guard fills on that
    /// arm's path, so the statement's release frees exactly the arm that ran.
    pub(super) fn lower_discarded(&mut self, expr: &'a Expr) {
        let conditional = matches!(&expr.kind, ExprKind::Conditional { .. });
        if conditional {
            self.discard_conditional_depth += 1;
        }
        let value = self.lower_value(expr);
        if conditional {
            self.discard_conditional_depth -= 1;
        }
        if is_pure_value(&value) {
            return;
        }
        self.emit(Statement::Discard { value });
    }

    /// Defers a discarded conditional arm's fresh owning result.
    ///
    /// A conditional statement discards whichever arm it selects, so a fresh
    /// result the arm allocated must bind to a statement temporary the guard
    /// fills on that arm's path; the temporary joins the statement's release
    /// set and is freed exactly once. A place-derived or already bound arm
    /// keeps reading its storage instead of moving it, because the discard
    /// never takes ownership of the arm's value.
    fn defer_discarded_arm(&mut self, value: Value) -> Value {
        if self.discard_conditional_depth == 0
            || !self.discard_binds_owner(&value.ty)
            || !matches!(
                &value.kind,
                ValueKind::Call { .. }
                    | ValueKind::ClosureCall { .. }
                    | ValueKind::StructLit { .. }
            )
        {
            return value;
        }
        // A struct literal materializes only when every owning field is a
        // fresh result. A field that reads a place, an already bound
        // temporary, or an aggregate built from either is copied by the
        // aggregate's store, so releasing the aggregate alongside the field's
        // own owner would free the same storage twice; the literal stays an
        // inline value instead.
        if let ValueKind::StructLit { fields, .. } = &value.kind {
            if fields
                .iter()
                .any(|(_, field)| self.field_aliases_owned_storage(field))
            {
                return value;
            }
        }
        let ty = value.ty.clone();
        let temp = self.new_temp(ty.clone());
        self.register_pending_release(temp, &ty);
        Value::new(
            ty,
            ValueKind::Materialize {
                temp,
                value: Box::new(value),
                move_outs: Vec::new(),
                adopting: false,
            },
        )
    }

    /// True when a guard-materialized field reads storage another owner holds.
    ///
    /// A field that reads a place, an already bound temporary, or a projection
    /// of either is copied by the aggregate store, so releasing the
    /// materialized aggregate would free storage the source still owns. An
    /// aggregate built from such fields is covered too, because the same copy
    /// reaches them through the field's own store; a fresh call or a literal
    /// of fresh results aliases nothing. A duplicable field carries no
    /// storage, so it never counts.
    fn field_aliases_owned_storage(&self, value: &Value) -> bool {
        if !(self.owns_drop(&value.ty) || self.maybe_owns(&value.ty)) {
            return false;
        }
        if self.projects_from_place(value) {
            return true;
        }
        match &value.kind {
            ValueKind::StructLit { fields, .. } => fields
                .iter()
                .any(|(_, field)| self.field_aliases_owned_storage(field)),
            ValueKind::ArrayLit { elements, fill } => {
                elements
                    .iter()
                    .any(|element| self.field_aliases_owned_storage(element))
                    || fill
                        .as_deref()
                        .is_some_and(|fill| self.field_aliases_owned_storage(fill))
            }
            // Whichever arm the condition selects supplies the field's value,
            // so an aliasing arm aliases it on that path; the aggregate stays
            // inline when either arm reads owned storage.
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.field_aliases_owned_storage(then_value)
                    || self.field_aliases_owned_storage(else_value)
            }
            _ => false,
        }
    }

    /// True when a discarded result needs a statement temporary to release.
    ///
    /// An owned or maybe-owning result keeps storage alive past its
    /// expression, so the statement needs a destination the pending release
    /// can free; a duplicable result owns nothing, and a fixed-array result
    /// already releases through the discard path's own destination.
    pub(super) fn discard_binds_owner(&self, ty: &Ty) -> bool {
        !ty.is_unknown()
            && !ty.is_fixed_array()
            && (self.owns_drop(ty) || self.maybe_owns(ty))
    }

    /// Binds a discarded owning result to a declared statement temporary.
    ///
    /// The result assigns into the temporary and the pending release runs at
    /// the statement's end, so a discarded expression's storage is freed
    /// exactly once on every path the statement reaches.
    pub(super) fn bind_discarded_owner(&mut self, value: Value) {
        let ty = value.ty.clone();
        let temp = self.new_temp(ty.clone());
        self.register_pending_release(temp, &ty);
        let target = Place {
            base: ir::PlaceBase::Temp(temp),
            projections: Vec::new(),
            ty,
        };
        self.emit(Statement::Assign { target, value });
    }

    /// Defers a side-effecting guarded operand into a temporary, evaluated once.
    ///
    /// The declared type of a materialized temporary.
    ///
    /// A call expression's recorded type may still name the callee's type
    /// parameters (a dot call on a generic receiver), so the temporary would
    /// be declared with the unsubstituted type. The call's concrete type
    /// arguments substitute it; every other value keeps its type.
    pub(super) fn materialized_ty(&self, value: &Value) -> Ty {
        let ty = value.ty.clone();
        if !mentions_type_param(&ty) {
            return ty;
        }
        match &value.kind {
            ValueKind::Call {
                callee, type_args, ..
            } => {
                let Some(Callee::User(decl)) = self.program.callee(callee) else {
                    return ty;
                };
                if type_args.is_empty() || decl.type_params.len() != type_args.len() {
                    return ty;
                }
                let bindings: BTreeMap<String, Ty> = decl
                    .type_params
                    .iter()
                    .map(|param| param.name.clone())
                    .zip(type_args.iter().cloned())
                    .collect();
                crate::types::substitute(&ty, &bindings)
            }
            // The recorded element type of a dot-call result still names the
            // callee's parameter; the base's resolved type supplies the
            // concrete element instead.
            ValueKind::Index { base, .. } => {
                let element = self.materialized_ty(base).iterated_element();
                if !element.is_unknown() {
                    return element;
                }
                ty
            }
            _ => ty,
        }
    }

    /// A runtime guard and the operation it protects both read the operand. An
    /// operand with no side effect is safe to render twice, so it is returned
    /// unchanged. A side-effecting one (a call such as `a[next()]` or
    /// `10/d()`) is wrapped in a [`ValueKind::Materialize`] so both reads see
    /// the same value and the call runs once. The assignment that fills the
    /// temporary is emitted by the guard, which keeps it on the path where the
    /// operand is actually evaluated. A fixed array is left alone unless the
    /// operand is a call: C cannot assign a place read to a temporary, but a
    /// by-value fixed-array call is written through its hidden trailing
    /// out-parameter, which the materialize guard arms emit.
    pub(super) fn materialize_guard_operand(&mut self, value: Value) -> Value {
        if is_pure_value(&value) || value.ty.is_closure() {
            return value;
        }
        if matches!(
            &value.ty.kind,
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some)
        ) && !matches!(
            value.kind,
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
        )
        {
            return value;
        }
        // A type that mentions a generic parameter materializes like any
        // other: the temporary is declared with the substituted type per
        // instance, and a maybe-owning release is a no-op for a duplicable one.
        // The value is assigned to the temporary where the guard runs, not
        // here: an operand inside `&&`/`||`/`?:` is evaluated only on the path
        // that reaches it, so a statement emitted now would run it even when
        // the left operand short-circuits.
        let ty = self.materialized_ty(&value);
        let temp = self.new_temp(ty.clone());
        self.mark_destination_temp(temp);
        self.register_pending_release(temp, &ty);
        Value::new(
            ty,
            ValueKind::Materialize {
                temp,
                value: Box::new(value),
                move_outs: Vec::new(),
                adopting: false,
            },
        )
    }

    /// Fills an eager materialized projection base before an index is pinned.
    ///
    /// A projection base expression runs before its index (`m()[i]` evaluates
    /// `m()` first), and the base may change the index. A base that is already
    /// materialized defers its fill to the statement that renders the value, so
    /// when a separate index statement must follow the base, the fill is emitted
    /// here first. A guarded operand keeps its deferred form — its fill and its
    /// index already share one comma expression inside the guard, in the right
    /// order.
    pub(super) fn settle_projection_base(&mut self, value: Value) -> Value {
        match value.kind {
            ValueKind::Member { base, name } => {
                let base = self.settle_projection_base(*base);
                Value::new(
                    value.ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.settle_projection_base(*base);
                Value::new(
                    value.ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index,
                    },
                )
            }
            ValueKind::Materialize {
                temp,
                value: inner,
                move_outs,
                adopting: _,
            } if self.defer_move_outs == 0 && move_outs.is_empty() => {
                let place = Place {
                    base: ir::PlaceBase::Temp(temp),
                    projections: Vec::new(),
                    ty: inner.ty.clone(),
                };
                self.emit(Statement::Assign {
                    target: place,
                    value: *inner,
                });
                Value::new(value.ty, ValueKind::Temp(temp))
            }
            other => Value::new(value.ty, other),
        }
    }

    /// Binds every non-constant element index in a value's projection chain.
    ///
    /// A `take` move-out must empty the slot the callee received, so a run-time
    /// index has to survive on the place. Each level of a nested chain is
    /// handled (`mk()[i][j]` keeps both indices), and a `Member` chain recurses
    /// to its element base. A constant index and an index that already names
    /// storage are left alone.
    pub(super) fn materialize_index_chain(&mut self, value: Value) -> Value {
        match value.kind {
            ValueKind::Member { base, name } => {
                let base = self.materialize_index_chain(*base);
                Value::new(
                    value.ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.materialize_index_chain(*base);
                let index = self.materialize_index(*index);
                Value::new(
                    value.ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index: Box::new(index),
                    },
                )
            }
            other => Value::new(value.ty, other),
        }
    }

    /// Stores one index expression so the call and its move-out share it.
    ///
    /// The index is evaluated exactly once. Outside a guard it is assigned to a
    /// compiler temporary now; under a guard the assignment is deferred to the
    /// path that evaluates the operand, so a short-circuited index never runs.
    /// A constant or an index that already names storage is returned unchanged.
    fn materialize_index(&mut self, index: Value) -> Value {
        if matches!(
            index.kind,
            ValueKind::Int(_) | ValueKind::Temp(_) | ValueKind::Materialize { .. }
        ) {
            return index;
        }
        if self.defer_move_outs > 0 {
            let ty = self.materialized_ty(&index);
            let temp = self.new_temp(ty.clone());
            Value::new(
                ty,
                ValueKind::Materialize {
                    temp,
                    value: Box::new(index),
                    move_outs: Vec::new(),
                    adopting: false,
                },
            )
        } else {
            self.materialize_value(index)
        }
    }

    /// Materializes a `take` argument that has no C place into a non-releasing
    /// temporary.
    ///
    /// The callee's parameter cleanup owns the argument, so the caller never
    /// releases the temporary; the call empties it through a move-out after the
    /// call runs. That move-out also leaves the empty state the branch store's
    /// own save and release expect. An operand that is evaluated only sometimes
    /// (the right side of `&&`/`||`, or an arm of `?:`) defers the store to its
    /// guard, so a skipped path never runs an arm or the call. A struct literal
    /// stores eagerly, because only its statement form exists.
    fn materialize_take_place(&mut self, value: Value) -> Value {
        let ty = self.materialized_ty(&value);
        // An owned conditional has an expression form in a guard; a
        // duplicable struct conditional does not, so it stores eagerly.
        let deferred = self.defer_move_outs > 0
            && match &value.kind {
                ValueKind::Conditional { .. } => !self.sema.table.duplicable(&value.ty),
                ValueKind::ArrayLit { .. }
                | ValueKind::Call { .. }
                | ValueKind::ClosureCall { .. } => true,
                _ => false,
            };
        if deferred {
            let temp = self.new_temp(ty.clone());
            return Value::new(
                ty,
                ValueKind::Materialize {
                    temp,
                    value: Box::new(value),
                    move_outs: Vec::new(),
                    adopting: false,
                },
            );
        }
        self.materialize_value(value)
    }

    /// Stores a value in a compiler temporary and returns the temporary value.
    ///
    /// The assignment is emitted into the current block, so the temporary is
    /// computed before the construct that reads it. The temporary is an
    /// unnamed compiler slot, so it takes no part in the ownership walk.
    pub(super) fn materialize_value(&mut self, value: Value) -> Value {
        let ty = self.materialized_ty(&value);
        let id = self.new_temp(ty.clone());
        let place = Place {
            base: ir::PlaceBase::Temp(id),
            projections: Vec::new(),
            ty: ty.clone(),
        };
        self.emit(Statement::Assign { target: place, value });
        Value::new(ty, ValueKind::Temp(id))
    }

    /// Consumes an owned place value as a deferred move, emptying its source.
    ///
    /// A place value whose type is non-duplicable and does not mention a generic
    /// parameter travels through a deferred materialization: the temporary's
    /// fill copies the value at the consuming point and the carried move-out
    /// zeroes the source, so a later drop of the source cannot free the value
    /// the destination now owns. Every other value is returned unchanged. This
    /// realizes the "a move is a copy plus a zero of the source" rule for a
    /// value consumed inside an aggregate literal or stored into an element.
    ///
    /// The preparation stays deferred so the fill runs where the value is
    /// consumed: a fresh call root is bound through the guard-emitter
    /// materialization, a run-time index is pinned, and the field or element
    /// projection then moves out of the bound temporary. The consuming
    /// destination adopts the value and empties the temporary; without that,
    /// a loop's reassigned-temp drop would free what the destination owns.
    /// A value that already names no storage is returned unchanged.
    pub(super) fn consume_place(&mut self, value: Value) -> Value {
        self.consume_place_into(value, None)
    }

    /// Consumes an owned place into a chosen destination temporary.
    ///
    /// [`Self::consume_place`] allocates a fresh temporary for the consumed
    /// value. A caller that already owns a destination — the shared carrier a
    /// `take` argument fills — passes that temporary here instead, so no
    /// unused temporary is created.
    fn consume_place_into(&mut self, value: Value, target: Option<ir::TempId>) -> Value {
        // A maybe-owning value moves through the same shape: the emitted move
        // copies and zeroes only for an owning instance.
        if !(self.owns_drop(&value.ty) || self.maybe_owns(&value.ty)) {
            return value;
        }
        let value = if matches!(
            value.kind,
            ValueKind::Index { .. } | ValueKind::Member { .. }
        ) {
            let value = self.bind_guarded_projection_root(value);
            self.materialize_index_chain(value)
        } else {
            value
        };
        let Some(source) = self.value_place(&value) else {
            return value;
        };
        let ty = self.materialized_ty(&value);
        let temp = match target {
            Some(temp) => temp,
            None => self.new_temp(ty.clone()),
        };
        Value::new(
            ty,
            ValueKind::Materialize {
                temp,
                value: Box::new(value),
                move_outs: vec![source],
                adopting: true,
            },
        )
    }

    /// Lowers a consumed projection of a conditional per arm.
    ///
    /// A member or element projection whose root is a conditional that mixes a
    /// place arm with an ownerless one cannot be consumed as one copy: the
    /// chosen arm's member must move out of its root, or the root's release
    /// frees the storage the consumer now owns (a double free) and the place
    /// arm's own cleanup frees it again at scope exit. The condition is
    /// evaluated once into a temporary; each arm runs in its own branch, where
    /// the projected member is copied into an adopting temporary and zeroed in
    /// the chosen root; the consumer then selects between the two temporaries.
    /// The place arm keeps ownership of its remaining fields, and a fresh
    /// arm's root releases only its un-moved fields. Evaluation order is
    /// preserved: the condition evaluates once, then the chosen arm alone.
    ///
    /// Only an eager statement position (a store, a return, a `take` argument,
    /// or a deferred body) reaches here; a conditionally evaluated operand
    /// keeps its own deferred shapes.
    pub(super) fn consume_conditional_projection(&mut self, value: Value) -> Value {
        self.consume_conditional_projection_into(value, None)
    }

    /// Lowers a `take` projection of a conditional into one caller carrier.
    ///
    /// The callee owns a `take` argument, so the caller's copy of the value
    /// must be emptied once the call returns. A per-arm temporary would keep
    /// the pointer the callee freed, and a statement that runs again — a loop
    /// iteration or a duplicated deferred copy — would free it a second time.
    /// Every arm therefore fills one shared carrier temporary that the
    /// ordinary post-call move-out empties, exactly as a plain-place `take`
    /// argument empties its source.
    pub(super) fn materialize_take_conditional_projection(&mut self, value: Value) -> Value {
        let ty = value.ty.clone();
        let carrier = self.new_temp(ty.clone());
        self.consume_conditional_projection_into(value, Some(carrier));
        Value::new(ty, ValueKind::Temp(carrier))
    }

    /// Lowers a consumed projection of a conditional, optionally into a
    /// shared carrier.
    ///
    /// Without a carrier each arm fills its own adopting temporary and the
    /// result selects between them; with one every arm fills that temporary
    /// and the result is the carrier itself.
    fn consume_conditional_projection_into(
        &mut self,
        value: Value,
        carrier: Option<ir::TempId>,
    ) -> Value {
        let result_ty = value.ty.clone();
        let mut steps: Vec<ConditionalProjectionStep> = Vec::new();
        let mut cursor = value;
        let (condition, then_value, else_value) = loop {
            match cursor.kind {
                ValueKind::Member { base, name } => {
                    steps.push(ConditionalProjectionStep::Field {
                        name,
                        ty: cursor.ty.clone(),
                    });
                    cursor = *base;
                }
                ValueKind::Index { base, index } => {
                    steps.push(ConditionalProjectionStep::Index {
                        index: *index,
                        ty: cursor.ty.clone(),
                    });
                    cursor = *base;
                }
                ValueKind::Conditional {
                    condition,
                    then_value,
                    else_value,
                } => break (condition, then_value, else_value),
                other => return Value::new(cursor.ty, other),
            }
        };
        let cond_ty = condition.ty.clone();
        let cond_id = self.new_temp(cond_ty.clone());
        let cond_place = Place {
            base: ir::PlaceBase::Temp(cond_id),
            projections: Vec::new(),
            ty: cond_ty.clone(),
        };
        self.emit(Statement::Assign {
            target: cond_place.clone(),
            value: *condition,
        });
        let cond_value =
            || Value::new(cond_ty.clone(), ValueKind::Place(cond_place.clone()));
        let then_block = self.new_block();
        let else_block = self.new_block();
        let join = self.new_block();
        self.terminate(Terminator::Branch {
            condition: cond_value(),
            on_true: then_block,
            on_false: else_block,
            shape: BranchShape::If { join },
        });
        self.switch_to(then_block);
        let then_value = self.consume_conditional_arm(*then_value, &steps, carrier);
        self.terminate(Terminator::Goto(join));
        self.switch_to(else_block);
        let else_value = self.consume_conditional_arm(*else_value, &steps, carrier);
        self.terminate(Terminator::Goto(join));
        self.switch_to(join);
        match carrier {
            Some(carrier) => Value::new(result_ty, ValueKind::Temp(carrier)),
            None => Value::new(
                result_ty,
                ValueKind::Conditional {
                    condition: Box::new(cond_value()),
                    then_value: Box::new(then_value),
                    else_value: Box::new(else_value),
                },
            ),
        }
    }

    /// True when a consumed value carries a projection over a conditional.
    ///
    /// Only the shapes this predicate names route into
    /// [`Self::consume_conditional_projection`]: a member or element chain
    /// rooted at a conditional with a place arm, or a conditional with such a
    /// chain in one of its arms. A chain rooted anywhere else keeps its
    /// existing lowering.
    pub(super) fn consume_needs_conditional_arms(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.consume_needs_conditional_arms(then_value)
                    || self.consume_needs_conditional_arms(else_value)
            }
            ValueKind::Member { .. } | ValueKind::Index { .. } => {
                self.conditional_borrows_place_root(value)
            }
            _ => false,
        }
    }

    /// Consumes one arm's projection in its own branch.
    ///
    /// The projection below the conditional is rebuilt on the arm, then the
    /// selected member moves out of the arm's root into the destination. A
    /// nested conditional arm recurses, so every level moves out its own
    /// chosen member.
    fn consume_conditional_arm(
        &mut self,
        arm: Value,
        steps: &[ConditionalProjectionStep],
        carrier: Option<ir::TempId>,
    ) -> Value {
        let projected = apply_conditional_steps(arm, steps);
        if self.consume_needs_conditional_arms(&projected) {
            self.consume_conditional_projection_into(projected, carrier)
        } else {
            self.settle_consumed_arm(projected, carrier)
        }
    }

    /// Fills a consumed arm's destination in the current branch.
    ///
    /// The projection's root settles first — a fresh call root fills its own
    /// temporary — then the selected member is copied into the destination and
    /// the source is zeroed. The root keeps ownership of its remaining fields
    /// and the consumer owns the moved value exactly once. With a carrier the
    /// destination is the shared temporary the callee's `take` empties, and a
    /// value that owns nothing fills the carrier as a plain store; without
    /// one, such a value keeps its existing form.
    fn settle_consumed_arm(&mut self, value: Value, carrier: Option<ir::TempId>) -> Value {
        let value = self.consume_place_into(value, carrier);
        let kind = value.kind;
        let ValueKind::Materialize {
            temp,
            value: inner,
            move_outs,
            ..
        } = kind
        else {
            let value = Value::new(value.ty, kind);
            let Some(carrier) = carrier else {
                return value;
            };
            let ty = value.ty.clone();
            let place = Place {
                base: ir::PlaceBase::Temp(carrier),
                projections: Vec::new(),
                ty: ty.clone(),
            };
            self.emit(Statement::Assign {
                target: place,
                value,
            });
            return Value::new(ty, ValueKind::Temp(carrier));
        };
        let inner = self.settle_projection_base(*inner);
        let ty = inner.ty.clone();
        let place = Place {
            base: ir::PlaceBase::Temp(temp),
            projections: Vec::new(),
            ty: ty.clone(),
        };
        self.emit(Statement::Assign {
            target: place,
            value: inner,
        });
        for source in move_outs {
            self.emit(Statement::MoveOut { place: source });
        }
        Value::new(ty, ValueKind::Temp(temp))
    }

    /// The place a value reads, when it is a field reached through a place.
    ///
    /// A `Member` value keeps a run-time base for a call result, but when its
    /// base is itself a place (`(b).value`) the whole path is a place and can be
    /// moved. The field projection is appended so the resolved place matches the
    /// ownership walk's `place_target`.
    pub(super) fn value_place(&self, value: &Value) -> Option<Place> {
        match &value.kind {
            ValueKind::Place(place) => Some(place.clone()),
            // A compiler temporary is storage the current statement owns, so a
            // projected read of it names the same object a move-out empties.
            ValueKind::Temp(id) => Some(Place {
                base: ir::PlaceBase::Temp(*id),
                projections: Vec::new(),
                ty: value.ty.clone(),
            }),
            ValueKind::Materialize { temp, .. } => Some(Place {
                base: ir::PlaceBase::Temp(*temp),
                projections: Vec::new(),
                ty: value.ty.clone(),
            }),
            ValueKind::Member { base, name } => {
                // A fixed array's `.len` is its structural extent: it names no
                // storage, so it never becomes a place projection and its
                // base does not evaluate.
                if name == "len" && base.ty.is_fixed_array() {
                    return None;
                }
                let mut place = self.value_place(base)?;
                place.projections.push(ir::Projection::Field(name.clone()));
                place.ty = value.ty.clone();
                Some(place)
            }
            ValueKind::Index { base, index } => {
                let mut place = self.value_place(base)?;
                let projection = match &index.kind {
                    ValueKind::Int(value) => {
                        ir::Projection::Element(usize::try_from(*value).ok())
                    }
                    // A materialized run-time index survives on the place, so a
                    // move-out through it empties the slot the read selected.
                    ValueKind::Temp(id) => ir::Projection::ElementTemp(*id),
                    ValueKind::Materialize { temp, .. } => ir::Projection::ElementTemp(*temp),
                    _ => ir::Projection::Element(None),
                };
                place.projections.push(projection);
                place.ty = value.ty.clone();
                Some(place)
            }
            _ => None,
        }
    }

    /// True when a value has no C form at a position that needs a place.
    ///
    /// A fixed-array value must live in storage before it is passed or
    /// returned: C cannot copy an array from a `?:` expression or a brace
    /// list, and a by-value fixed-array call writes through a hidden
    /// out-parameter instead of returning a value. A struct literal needs a
    /// place when a field is a fixed array (a C aggregate initializer cannot
    /// name one) or when a nested literal needs stores. A value that already
    /// names storage — a place, a compiler temporary, or a deferred
    /// materialization — needs nothing.
    fn value_needs_place(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Place(_) | ValueKind::Temp(_) | ValueKind::Materialize { .. } => false,
            ValueKind::ArrayLit { .. }
            | ValueKind::Call { .. }
            | ValueKind::ClosureCall { .. } => value.ty.is_fixed_array(),
            // A fixed-array conditional has no C value form. A struct
            // conditional does not either when an arm stores its fields,
            // because a C `?:` copies bytes and cannot name a fixed-array
            // field; the value is stored per arm instead.
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                if value.ty.is_fixed_array() {
                    return true;
                }
                matches!(&value.ty.kind, TyKind::Struct(..))
                    && (self.value_needs_place(then_value) || self.value_needs_place(else_value))
            }
            ValueKind::StructLit { fields, .. } => fields.iter().any(|(_, field)| {
                field.ty.is_fixed_array() || self.value_needs_place(field)
            }),
            _ => false,
        }
    }

    /// True when a projection chain roots at statement-owned compiler storage.
    ///
    /// A store that reads an owning element or field out of such a temporary
    /// must move it out before the temporary's release, exactly as a `take`
    /// does: the temporary owns the value until the end of the statement, so a
    /// copy would leave the stored value aliasing storage the release frees. A
    /// chain that roots at a caller place is not covered — the caller's
    /// container keeps ownership and the store stays a copy.
    pub(super) fn projection_roots_at_temp(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Temp(_) | ValueKind::Materialize { .. } => true,
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.projection_roots_at_temp(base)
            }
            _ => false,
        }
    }

    /// True when the value reads storage an owned container is responsible for.
    ///
    /// A place, or a field or element projection of one at any index, names
    /// storage something else owns: the container's cleanup releases it, so
    /// binding a scope temporary for such a value would create a second owner
    /// and the two drops would free the same storage twice. A
    /// `for (borrow e in xs)` element is covered because its binding is a place
    /// rooted at the element address, and so is any projection over a local, a
    /// parameter, or a call-returned array stored in a place.
    ///
    /// A destination materialization is covered too: the compiler releases it
    /// at the end of the statement that created it, so a projection of it is
    /// storage that release owns. The same holds for a conditional whose arms
    /// both point into such storage, because the chosen arm is a projection of
    /// it. An ownerless temporary that is not a destination (a transfer
    /// temporary) is not covered: its value travels to a binding that owns it,
    /// so a projection still needs that binding.
    pub(super) fn projects_from_place(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Place(_) => true,
            ValueKind::Materialize { .. } => true,
            ValueKind::Temp(id) => self.is_destination_temp(*id),
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.projects_from_place(base)
            }
            // A conditional whose arms both point into existing storage is a
            // forwarding value: whichever arm the condition chooses lends
            // storage the caller already owns, so binding an owning copy for
            // it would create a second owner and double-free.
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.projects_from_place(then_value) && self.projects_from_place(else_value)
            }
            _ => false,
        }
    }

    /// The provenance of a view whose storage a lowered value reads.
    ///
    /// A projection over a materialized value reads storage the compiler owns
    /// through a statement temporary, so the view must carry a loan of that
    /// temporary: releasing the temporary at the statement's end would
    /// otherwise leave the view pointing into freed storage. A projection over
    /// a place keeps the place's own provenance, which the source expression
    /// already reports.
    pub(super) fn lowered_view_provenance(&self, value: &Value) -> Provenance {
        match &value.kind {
            ValueKind::Materialize { temp, .. } | ValueKind::Temp(temp) => {
                Provenance::Temp(*temp)
            }
            ValueKind::Index { base, .. } | ValueKind::Member { base, .. } => {
                self.lowered_view_provenance(base)
            }
            _ => Provenance::None,
        }
    }

    /// The compiler temporaries a lowered view value reads its storage from.
    ///
    /// A view may derive from a statement temporary directly (a projection of
    /// a materialized value), through a reborrow call whose single lend source
    /// is such a value, or through a conditional whose arms derive from
    /// different temporaries. Every such temporary is an owner: the view
    /// carries a loan of it, so a use after the temporary's statement release
    /// reports `E10030`.
    pub(super) fn lowered_temp_owners(&self, value: &Value) -> BTreeSet<ir::TempId> {
        let mut owners = BTreeSet::new();
        self.collect_lowered_temp_owners(value, &mut owners);
        owners
    }

    /// Collects the statement temporaries a lowered value reads through.
    fn collect_lowered_temp_owners(&self, value: &Value, owners: &mut BTreeSet<ir::TempId>) {
        match &value.kind {
            ValueKind::Materialize {
                temp,
                value: inner,
                move_outs: _,
                adopting: _,
            } => {
                // A view-typed temporary owns no storage and is never released,
                // so only an owning temporary is a loan owner; a type that may
                // own once substituted is one too, because an owning instance
                // releases the temporary and every view derived from it must
                // not outlive that release. The wrapped value may itself read
                // through a temporary (a slice of an element of a call result),
                // so the walk continues into it.
                if self.owns_drop(&value.ty) || self.maybe_owns(&value.ty) {
                    owners.insert(*temp);
                }
                self.collect_lowered_temp_owners(inner, owners);
            }
            ValueKind::Temp(temp) => {
                // A transfer temporary's value travels to a binding that owns
                // it, so only a destination temporary is a loan owner.
                if self.is_destination_temp(*temp) {
                    owners.insert(*temp);
                }
            }
            ValueKind::Index { base, .. } | ValueKind::Member { base, .. } => {
                self.collect_lowered_temp_owners(base, owners);
            }
            ValueKind::Cast { value: inner, .. } => {
                self.collect_lowered_temp_owners(inner, owners);
            }
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.collect_lowered_temp_owners(then_value, owners);
                self.collect_lowered_temp_owners(else_value, owners);
            }
            ValueKind::Call {
                callee,
                args,
                receiver: None,
                ..
            } => {
                if let Some(source) = self
                    .lowered_lend_source(callee)
                    .and_then(|index| args.get(index))
                {
                    self.collect_lowered_temp_owners(source, owners);
                }
            }
            ValueKind::Intrinsic { name, args, .. } if name == "slice" => {
                if let Some(source) = args.first() {
                    self.collect_lowered_temp_owners(source, owners);
                }
            }
            // An array literal's elements and fill value are the value's
            // storage, so a view element of a temporary contributes the
            // temporary like any other projection.
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.collect_lowered_temp_owners(element, owners);
                }
                if let Some(fill) = fill {
                    self.collect_lowered_temp_owners(fill, owners);
                }
            }
            _ => {}
        }
    }

    /// The compiler-generated owner bindings a lowered view reads through.
    ///
    /// A generated owner (`__cl_ownedN`) lives in the checker's scope, so a
    /// view returned through it dies with the function exactly like a view of
    /// a named local: the caller's use lies after the owner's release. The
    /// walk mirrors [`Self::collect_lowered_temp_owners`] and reports the
    /// bindings so the return can record their loans and the liveness rule
    /// reports the escape once.
    pub(super) fn lowered_owner_bindings(&self, value: &Value) -> BTreeSet<usize> {
        let mut owners = BTreeSet::new();
        self.collect_lowered_owner_bindings(value, &mut owners);
        owners
    }

    /// The binding id whose IR spelling names a generated owner.
    ///
    /// A place names its base by IR spelling, so the generated-owner check
    /// maps the spelling back to the binding table entry.
    fn generated_owner_id(&self, name: &str) -> Option<usize> {
        self.bindings
            .iter()
            .position(|binding| binding.generated && binding.ir_name == name)
    }

    /// Collects the generated owner bindings a value reads through.
    fn collect_lowered_owner_bindings(&self, value: &Value, owners: &mut BTreeSet<usize>) {
        match &value.kind {
            ValueKind::Place(place) => {
                if let ir::PlaceBase::Local(name) = &place.base {
                    if let Some(id) = self.generated_owner_id(name) {
                        if self.bindings[id].needs_drop {
                            owners.insert(id);
                        }
                    }
                }
            }
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.collect_lowered_owner_bindings(base, owners);
            }
            ValueKind::Cast { value: inner, .. } => {
                self.collect_lowered_owner_bindings(inner, owners);
            }
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.collect_lowered_owner_bindings(then_value, owners);
                self.collect_lowered_owner_bindings(else_value, owners);
            }
            ValueKind::Call {
                callee,
                args,
                receiver: None,
                ..
            } => {
                if let Some(source) = self
                    .lowered_lend_source(callee)
                    .and_then(|index| args.get(index))
                {
                    self.collect_lowered_owner_bindings(source, owners);
                }
            }
            ValueKind::Intrinsic { name, args, .. } if name == "slice" => {
                if let Some(source) = args.first() {
                    self.collect_lowered_owner_bindings(source, owners);
                }
            }
            // An array literal's elements and fill value are the value's
            // storage, so a view element read through a generated owner
            // contributes that owner like any other projection.
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.collect_lowered_owner_bindings(element, owners);
                }
                if let Some(fill) = fill {
                    self.collect_lowered_owner_bindings(fill, owners);
                }
            }
            _ => {}
        }
    }

    /// The argument a view-returning call reborrows, when it elides one.
    ///
    /// Mirrors the source-level provenance rule on the lowered call: a user
    /// callee that returns an elided view and has exactly one lend parameter
    /// reborrows that argument; a standard-library `ViewOf` effect names its
    /// source argument directly. The lowered argument list already carries a
    /// dot call's receiver first, so the parameter index is the argument
    /// index.
    fn lowered_lend_source(&self, callee: &str) -> Option<usize> {
        // A callee that names a local binding is a call through its func
        // value, so the value's own type decides any reborrow; a same-named
        // module function must not claim the call, or its declared lend would
        // suppress the result's binding and leak the owned result.
        if self.lookup(callee).is_some() {
            return None;
        }
        match self.program.callee(callee) {
            Some(Callee::User(decl)) => {
                let type_params: Vec<String> = decl
                    .type_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect();
                let returns = self.resolve_with(&decl.returns, &type_params);
                // A generic reborrow returns its type parameter, which is the
                // view at the call site; only a call whose result is a view
                // reaches this collector, so the parameter return qualifies.
                if !is_elided_view(&returns)
                    && !returns.is_param()
                    && !mentions_type_param(&returns)
                {
                    return None;
                }
                let candidates: Vec<usize> = decl
                    .params
                    .iter()
                    .enumerate()
                    .filter(|(_, param)| {
                        param.mode != Mode::Take
                            && (is_lend_candidate(&self.resolve_with(&param.ty, &type_params))
                                || mentions_type_param(&self.resolve_with(
                                    &param.ty,
                                    &type_params,
                                )))
                    })
                    .map(|(index, _)| index)
                    .collect();
                if candidates.len() == 1 {
                    Some(candidates[0])
                } else {
                    None
                }
            }
            Some(Callee::Stdlib {
                effect: StdlibEffect::ViewOf { arg, .. },
                ..
            }) => Some(arg),
            _ => None,
        }
    }

    /// Materializes a fixed-array read source that needs a place.
    ///
    /// An index base must name storage: a conditional base carries the chosen
    /// arm's ownership decisions, and a brace list has no C place at all, so
    /// neither can be indexed directly. Both are stored through the ordinary
    /// assignment into a fresh temporary and the read goes through that
    /// temporary. A value that already names storage, a non-array type, or a
    /// type that still mentions a generic parameter is returned unchanged.
    pub(super) fn materialize_read_place(&mut self, mut value: Value) -> Value {
        let needs_place = value.ty.is_fixed_array()
            && matches!(
                value.kind,
                ValueKind::Conditional { .. } | ValueKind::ArrayLit { .. }
            );
        if !needs_place {
            return value;
        }
        let ty = self.materialized_ty(&value);
        // An array literal inside a conditionally evaluated operand must not be
        // filled while the enclosing expression is lowered: the guard emitter
        // stores it where the operand is actually evaluated, so a short circuit
        // leaves its elements unread and unevaluated.
        if self.defer_move_outs > 0 && matches!(value.kind, ValueKind::ArrayLit { .. }) {
            let temp = self.new_temp(ty.clone());
            self.mark_destination_temp(temp);
            self.register_pending_release(temp, &ty);
            return Value::new(
                ty,
                ValueKind::Materialize {
                    temp,
                    value: Box::new(value),
                    move_outs: Vec::new(),
                    adopting: false,
                },
            );
        }
        // The eager store fills the temporary through a statement, but an owned
        // call trapped inside the stored value still needs its own temporary:
        // its guarded store drops the previous value before a re-store and its
        // release runs after the enclosing loop.
        self.materialize_owned_calls(&mut value);
        self.materialize_value_type(value, ty)
    }

    /// Stores a value into a fresh temporary and returns the temporary value.
    ///
    /// The assignment is emitted eagerly into the current block, so it runs
    /// where the lowered expression is evaluated. When the temporary's type
    /// owns storage it joins the statement's pending release list.
    pub(super) fn materialize_value_type(&mut self, value: Value, ty: Ty) -> Value {
        let temp = self.new_temp(ty.clone());
        self.mark_destination_temp(temp);
        self.register_pending_release(temp, &ty);
        let place = Place {
            base: ir::PlaceBase::Temp(temp),
            projections: Vec::new(),
            ty: ty.clone(),
        };
        self.emit(Statement::Assign {
            target: place,
            value,
        });
        Value::new(ty, ValueKind::Temp(temp))
    }

    /// Stores an impure operand in a compiler temporary before it is used twice.
    ///
    /// Unlike [`Self::materialize_guard_operand`], which defers the assignment
    /// to the guarded construct, this emits the assignment immediately. Use it
    /// for an operand that appears in two places of one statement (the element
    /// read and the element address of a compound store): each appearance is
    /// guarded separately, so an operand left inline would run its side effect
    /// twice.
    pub(super) fn materialize_operand(&mut self, value: Value) -> Value {
        if is_pure_value(&value) || value.ty.is_closure() {
            return value;
        }
        // A fixed-array call is let through for the same reason as in
        // `materialize_guard_operand`: the call writes into the temporary
        // through its hidden out-parameter, so no array assignment is emitted.
        if matches!(
            &value.ty.kind,
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some)
        ) && !matches!(
            value.kind,
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
        )
        {
            return value;
        }
        self.materialize_value(value)
    }

    /// Binds a consumed projection's ownerless call root to an owned temporary.
    ///
    /// A field or element read of a fresh call result names no storage that
    /// outlives the statement, so consuming it (a return, an owned binding, or a
    /// `take` argument) would copy the selected part while the result's other
    /// fields are never released. The root call is bound to an owned local and
    /// the projection is re-rooted at that local, so the ordinary move and
    /// return emission empties the selected field and the local's scope release
    /// drops only the remaining fields. A chain that already roots in storage,
    /// or a root that owns nothing, is returned unchanged.
    pub(super) fn bind_ownerless_projection_root(&mut self, value: Value) -> Value {
        match value.kind {
            // A fixed array's `.len` is its structural extent: the member
            // reads no storage, so the chain needs no owner and its root does
            // not evaluate.
            ValueKind::Member { ref base, ref name }
                if name == "len" && base.ty.is_fixed_array() =>
            {
                value
            }
            ValueKind::Member { base, name } => {
                let base = self.bind_ownerless_projection_root(*base);
                Value::new(
                    value.ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.bind_ownerless_projection_root(*base);
                Value::new(
                    value.ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index,
                    },
                )
            }
            // The root of the chain: a fresh call result owns its storage only
            // until the statement ends, so it is bound to a scope local. A
            // result that owns nothing to release is left inline.
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. } => {
                let owning = !self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty);
                if owning && !is_view(&value.ty) && !value.ty.is_unknown() {
                    self.materialize_owned_argument(value, false)
                } else {
                    value
                }
            }
            // A struct literal with owning fields is a storage root of its
            // own: its C compound literal has block lifetime and no cleanup,
            // so a projection of it must read through a bound local the
            // binding's scope releases.
            ValueKind::StructLit { .. } => {
                let owning = !self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty);
                if owning && !is_view(&value.ty) && !value.ty.is_unknown() {
                    self.materialize_owned_argument(value, false)
                } else {
                    value
                }
            }
            // A conditional whose arms own their values is bound as one value:
            // the owned-conditional store adopts the chosen arm, so every
            // arm's result is released exactly once. A conditional with a
            // place-rooted arm keeps its inline read, because adopting it would
            // empty storage the caller still owns. Binding each arm's root
            // instead would release the selected member a consuming store
            // copies, so the per-arm ownership stays with the view binder,
            // whose binding only reads.
            ValueKind::Conditional { .. } => {
                if self.conditional_borrows_place(&value) {
                    value
                } else {
                    let owning =
                        !self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty);
                    if owning && !is_view(&value.ty) && !value.ty.is_unknown() {
                        self.materialize_owned_argument(value, false)
                    } else {
                        value
                    }
                }
            }
            _ => value,
        }
    }

    /// Re-roots an ownerless projection chain at a local in the view's scope.
    ///
    /// Mirrors [`Self::bind_ownerless_projection_root`] for a view binding: the
    /// same roots bind — a fresh call result, a struct literal with owning
    /// fields, or a conditional of either — but the owner stays in the scope
    /// that declares the view. A view must not outlive the storage it reads,
    /// so the scope cleanup releases the backing after the binding's last use
    /// and the liveness pass rejects a view that escapes the scope. A chain
    /// that already roots in storage, or a root that owns nothing, is returned
    /// unchanged.
    pub(super) fn bind_view_projection_root(&mut self, value: Value) -> Value {
        match value.kind {
            // A fixed array's `.len` is its structural extent: the member
            // reads no storage, so the chain needs no owner.
            ValueKind::Member { ref base, ref name }
                if name == "len" && base.ty.is_fixed_array() =>
            {
                value
            }
            ValueKind::Member { base, name } => {
                let base = self.bind_view_projection_root(*base);
                Value::new(
                    value.ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.bind_view_projection_root(*base);
                Value::new(
                    value.ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index,
                    },
                )
            }
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. } | ValueKind::StructLit { .. } => {
                let owning = !self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty);
                if owning && !is_view(&value.ty) {
                    self.materialize_scoped_owner(value)
                } else {
                    value
                }
            }
            // A conditional whose arms own their values is bound as one scoped
            // owner: the owned-conditional store adopts the chosen arm once. An
            // arm that projects out of an ownerless root, a place-rooted arm,
            // and a view-typed result all keep the conditional inline and own
            // each ownerless arm's root in its own branch instead: the
            // projected root's other fields release, the place stays a borrow,
            // and the backing lives in the view binding's scope.
            ValueKind::Conditional { .. } => {
                let whole_owned = (!self.sema.table.duplicable(&value.ty)
                    || self.maybe_owns(&value.ty))
                    && !is_view(&value.ty)
                    && !value.ty.is_unknown();
                let arm_fill = self.conditional_needs_arm_fill(&value)
                    && self.materialize_owned_temporaries;
                if self.conditional_has_ownerless_projection_arm(&value) {
                    if arm_fill {
                        self.hoist_view_conditional_arms(value)
                    } else {
                        value
                    }
                } else if whole_owned && !self.conditional_borrows_place(&value) {
                    self.materialize_scoped_owner(value)
                } else if arm_fill {
                    self.hoist_view_conditional_arms(value)
                } else {
                    value
                }
            }
            // A view-element array is a list of views, so each element's
            // storage binds exactly as a single view's would: a place element
            // keeps its loan, a temporary element's backing adopts a scoped
            // owner, and a static element needs none.
            ValueKind::ArrayLit { elements, fill } => {
                let elements = elements
                    .into_iter()
                    .map(|element| self.bind_view_projection_root(element))
                    .collect();
                let fill = fill.map(|fill| Box::new(self.bind_view_projection_root(*fill)));
                Value::new(value.ty, ValueKind::ArrayLit { elements, fill })
            }
            _ => value,
        }
    }

    /// Binds a consumed projection's call root for a conditionally evaluated
    /// operand.
    ///
    /// An operand inside `&&`/`||`/`?:` runs only on the path that reaches it, so
    /// the ownerless call root cannot be bound to an eagerly assigned scope
    /// local: the binding would run the root call even when the operand is
    /// skipped. The root is wrapped in the deferred [`ValueKind::Materialize`]
    /// shape the guard emitter fills on the evaluated path, and the temporary's
    /// registered release frees every field the take does not move out. The
    /// recursion and the ownership rules match
    /// [`Self::bind_ownerless_projection_root`]; a chain that already roots in
    /// storage, or a root that owns nothing, is returned unchanged.
    pub(super) fn bind_guarded_projection_root(&mut self, value: Value) -> Value {
        match value.kind {
            ValueKind::Member { base, name } => {
                let base = self.bind_guarded_projection_root(*base);
                Value::new(
                    value.ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.bind_guarded_projection_root(*base);
                Value::new(
                    value.ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index,
                    },
                )
            }
            // The root of the chain: a fresh call result owns its storage only
            // until the statement ends. The deferred materialization fills the
            // temporary where the operand is evaluated and releases it with the
            // statement, so a skipped path never runs the call.
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. } => {
                let owning = !self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty);
                if owning && !is_view(&value.ty) && !value.ty.is_unknown() {
                    self.materialize_guard_operand(value)
                } else {
                    value
                }
            }
            // A struct literal with owning fields is a storage root of its
            // own: its C compound literal has block lifetime and no cleanup,
            // so the deferred materialization fills a temporary with it where
            // the operand is evaluated — the taken path only — and the
            // statement's release frees the fields the consumer does not
            // move. A literal that owns nothing stays inline.
            ValueKind::StructLit { .. } => {
                let owning = !self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty);
                if owning && !is_view(&value.ty) && !value.ty.is_unknown() {
                    self.materialize_guard_operand(value)
                } else {
                    value
                }
            }
            // A conditional whose arms own their values is bound as one value
            // through the deferred materialization: the guard emitter fills it
            // with the chosen arm on the evaluated path only, and its release
            // drops that arm's storage. An arm that projects out of an
            // ownerless root, a place-rooted arm, and a view-typed result all
            // keep the conditional inline and bind each ownerless arm's root
            // through the deferred fill instead: the projected root's other
            // fields release, the place stays a borrow, and the fresh root
            // releases on its own path.
            ValueKind::Conditional { .. } => {
                let whole_owned = (!self.sema.table.duplicable(&value.ty)
                    || self.maybe_owns(&value.ty))
                    && !is_view(&value.ty)
                    && !value.ty.is_unknown();
                let arm_fill = self.conditional_needs_arm_fill(&value)
                    && !self.speculative_lowering
                    && self.materialize_owned_temporaries;
                if self.conditional_has_ownerless_projection_arm(&value) {
                    if arm_fill {
                        self.own_ownerless_conditional_arms(value)
                    } else {
                        value
                    }
                } else if whole_owned && !self.conditional_borrows_place(&value) {
                    self.materialize_guard_operand(value)
                } else if arm_fill {
                    self.own_ownerless_conditional_arms(value)
                } else {
                    value
                }
            }
            _ => value,
        }
    }

    /// Binds an ownerless owned result that an intrinsic only borrows.
    ///
    /// An intrinsic such as `len` reads its argument without taking ownership,
    /// so the argument's storage is released by the enclosing statement's
    /// cleanup rather than by the callee. A named local already has an owner;
    /// an unnamed call result has none and leaks unless it is bound. The value
    /// is bound exactly like a named call's borrowed argument, so the same
    /// release paths apply. A conditionally evaluated argument (the right side
    /// of `&&` or `||`, or a `?:` arm) is bound through the deferred
    /// materialization instead: its fill runs only on the path that evaluates
    /// the argument, and the statement's registered release frees it there.
    pub(super) fn bind_intrinsic_argument(&mut self, value: Value) -> Value {
        if is_view(&value.ty) || value.ty.is_unknown() || self.projects_from_place(&value) {
            return value;
        }
        // A projection chain rooted at a conditional that needs an arm fill
        // must not be bound as one value: the owned-conditional store adopts
        // the chosen arm, which would empty storage a place-borrowing arm only
        // reads, and an all-ownerless conditional's un-moved root fields would
        // leak. Each arm is bound on its own path instead — evaluated under a
        // branch at this statement position when there is one — so a place
        // stays readable, a fresh arm's root releases exactly once, and the
        // argument keeps its position in the evaluation order. A member or
        // element projection (`(c ? b : mk()).v`) walks down to the
        // conditional the same way, so the argument reads through the bound
        // roots instead of materializing the projection into a released
        // temporary that aliases the place arm or leaks the root's other
        // fields.
        if self.conditional_root_needs_arm_fill(&value)
            && !self.speculative_lowering
            && self.materialize_owned_temporaries
            && !value.ty.is_unknown()
        {
            return if self.defer_move_outs == 0 {
                self.hoist_ownerless_arm(value)
            } else {
                self.own_ownerless_arm(value)
            };
        }
        if !(self.owns_drop(&value.ty) || self.maybe_owns(&value.ty)) {
            return value;
        }
        // A call that may reborrow an owned argument returns storage that
        // argument's owner already releases; binding the result would create a
        // second owner for the same storage. The intrinsic only borrows, so the
        // aliased value needs no binding of its own.
        if self.value_may_lend(&value) {
            return value;
        }
        // A transfer temporary that carries ownership is released by the
        // enclosing statement; registering it keeps a single owner without a
        // second binding. A temporary that already has a registered release is
        // left alone so the release is not repeated, and the temporary's
        // re-assignment pre-release covers a statement that runs again.
        if let ValueKind::Temp(id) = &value.kind {
            if !self.has_pending_release(*id, &value.ty) {
                self.register_pending_release(*id, &value.ty);
            }
            return value;
        }
        if self.defer_move_outs > 0 {
            self.materialize_guard_operand(value)
        } else {
            self.materialize_owned_argument(value, false)
        }
    }

    /// Binds an owned member receiver at a read position.
    ///
    /// A plain member read of an ownerless owned value (`spawn().count`,
    /// `mk().b.c`) leaves the projection root without an owner, so its
    /// storage leaks; only intrinsic arguments and dot-`len` receivers were
    /// bound before. The projection chain is bound at its root exactly like a
    /// consumed projection, so the enclosing construct's cleanup releases the
    /// whole result and the chain reads through the bound root. The bind runs
    /// only at the read positions that take the member value (casts, intrinsic
    /// arguments, call arguments, and return reads); consume preparation,
    /// conditionally evaluated operands, and loop conditions keep their own
    /// machinery and are excluded. A chain that already roots in storage, or a
    /// root that owns nothing, is returned unchanged.
    pub(super) fn bind_member_receiver(&mut self, value: Value) -> Value {
        if self.defer_move_outs > 0 || self.in_loop_condition {
            return value;
        }
        match value.kind {
            ValueKind::Member { .. } => self.bind_ownerless_projection_root(value),
            other => Value::new(value.ty, other),
        }
    }

    /// True when any reachable arm of a conditional roots in a place.
    ///
    /// The owned-conditional store adopts the chosen arm, so a conditional
    /// whose arm tree contains a place-rooted leaf must stay an inline read.
    /// Every non-conditional leaf is judged by the same predicate that tells a
    /// place projection from a fresh value, so a nested conditional is covered
    /// at any depth.
    fn conditional_borrows_place(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.conditional_borrows_place(then_value)
                    || self.conditional_borrows_place(else_value)
            }
            _ => self.projects_from_place(value),
        }
    }

    /// True when a value is a projection of a conditional with a place arm.
    ///
    /// The conditional stays an inline read (its place arm must not be
    /// adopted), so adopting the projected member would copy storage the
    /// place arm still owns and release it twice; the value passes through
    /// instead, and the fresh arm's storage stays with the enclosing
    /// statement. A bare conditional is not a projection: the conditional
    /// paths handle it (the per-arm fill, the owned-conditional store, or the
    /// array-decay spelling).
    fn projects_from_conditional_borrow(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.conditional_borrows_place_root(base)
            }
            _ => false,
        }
    }

    /// True when a chain roots at a conditional with a place arm.
    fn conditional_borrows_place_root(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.conditional_borrows_place_root(base)
            }
            ValueKind::Conditional { .. } => self.conditional_borrows_place(value),
            _ => false,
        }
    }

    /// True when a value is a chain rooted at a conditional whose arms need a
    /// fill.
    ///
    /// A member or element chain descends to its root; a root that is not a
    /// conditional names no arms to bind. A projection chain fires whenever
    /// its root's arms need a fill — the mixed conditional with a
    /// place-borrowing arm and the all-ownerless conditional alike, so an
    /// intrinsic argument never materializes the projection alone. A bare
    /// conditional fires only when an arm borrows a place: an ownerless bare
    /// conditional is handled by the whole-value binder, which owns its
    /// chosen arm itself.
    fn conditional_root_needs_arm_fill(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.chain_conditional_needs_arm_fill(base)
            }
            ValueKind::Conditional { .. } => {
                self.conditional_borrows_place(value) && self.conditional_needs_arm_fill(value)
            }
            _ => false,
        }
    }

    /// True when the base under a projection is a conditional whose arms need
    /// a fill.
    fn chain_conditional_needs_arm_fill(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.chain_conditional_needs_arm_fill(base)
            }
            ValueKind::Conditional { .. } => self.conditional_needs_arm_fill(value),
            _ => false,
        }
    }

    /// Binds the ownerless arms of a conditional lend argument.
    ///
    /// A place-rooted arm stays a borrow; every other arm — a fresh call
    /// result, a struct literal, or a nested conditional of either — binds
    /// through the deferred fill, so its storage is released by the
    /// statement's registered release only on the path that evaluates it. A
    /// conditional whose arms all stand on existing storage is returned
    /// unchanged: every leaf already lends the caller's storage.
    fn own_ownerless_conditional_arms(&mut self, value: Value) -> Value {
        match value.kind {
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let then_value = self.own_ownerless_arm(*then_value);
                let else_value = self.own_ownerless_arm(*else_value);
                Value::new(
                    value.ty,
                    ValueKind::Conditional {
                        condition,
                        then_value: Box::new(then_value),
                        else_value: Box::new(else_value),
                    },
                )
            }
            other => Value::new(value.ty, other),
        }
    }

    /// Binds one arm of a conditional lend argument when it owns storage.
    ///
    /// A place-rooted arm, or a conditional of place-rooted arms, stays
    /// inline. A temporary that already carries a registered release keeps
    /// that single owner. A projection arm binds its own root, so a selected
    /// member read does not release alone while the root's other fields keep
    /// their storage. Every other arm binds through the deferred fill.
    fn own_ownerless_arm(&mut self, value: Value) -> Value {
        if self.projects_from_place(&value) {
            return value;
        }
        match value.kind {
            ValueKind::Member { base, name } => {
                let base = self.own_ownerless_arm(*base);
                return Value::new(
                    value.ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                );
            }
            ValueKind::Index { base, index } => {
                let base = self.own_ownerless_arm(*base);
                return Value::new(
                    value.ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index,
                    },
                );
            }
            ValueKind::Conditional { .. } => {
                return self.own_ownerless_conditional_arms(value);
            }
            ValueKind::Temp(id) => {
                if !self.has_pending_release(id, &value.ty) {
                    self.register_pending_release(id, &value.ty);
                }
                return value;
            }
            _ => {}
        }
        self.materialize_guard_operand(value)
    }

    /// Evaluates a view conditional's ownerless arm roots on their own paths.
    ///
    /// A view binding of a conditional projection (`(c ? b : mk()).v`) must
    /// keep a place arm a borrow while every ownerless arm's root is owned for
    /// the view's scope: each root binds inside a branch only its own path
    /// reaches, the scope's cleanup releases it after the binding's last use,
    /// and the binding's view reads through the selected root. The condition
    /// is evaluated once, and a nested conditional recurses into its own
    /// branch.
    fn hoist_view_conditional_arms(&mut self, value: Value) -> Value {
        match value.kind {
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let cond_ty = condition.ty.clone();
                let cond_id = self.new_temp(cond_ty.clone());
                let cond_place = Place {
                    base: ir::PlaceBase::Temp(cond_id),
                    projections: Vec::new(),
                    ty: cond_ty.clone(),
                };
                self.emit(Statement::Assign {
                    target: cond_place.clone(),
                    value: *condition,
                });
                let cond_value =
                    || Value::new(cond_ty.clone(), ValueKind::Place(cond_place.clone()));
                let then_block = self.new_block();
                let else_block = self.new_block();
                let join = self.new_block();
                self.terminate(Terminator::Branch {
                    condition: cond_value(),
                    on_true: then_block,
                    on_false: else_block,
                    shape: BranchShape::If { join },
                });
                self.switch_to(then_block);
                let then_value = self.own_view_arm(*then_value);
                self.terminate(Terminator::Goto(join));
                self.switch_to(else_block);
                let else_value = self.own_view_arm(*else_value);
                self.terminate(Terminator::Goto(join));
                self.switch_to(join);
                Value::new(
                    value.ty,
                    ValueKind::Conditional {
                        condition: Box::new(cond_value()),
                        then_value: Box::new(then_value),
                        else_value: Box::new(else_value),
                    },
                )
            }
            other => Value::new(value.ty, other),
        }
    }

    /// Binds one view conditional arm's owning root in its own branch.
    ///
    /// Mirrors [`Self::bind_view_projection_root`] for one arm: a place-rooted
    /// arm stays a borrow, a nested conditional recurses, and a fresh call or
    /// literal root binds to a scope owner the view's scope releases.
    fn own_view_arm(&mut self, value: Value) -> Value {
        match value.kind {
            ValueKind::Member { base, name } => {
                let base = self.own_view_arm(*base);
                Value::new(
                    value.ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.own_view_arm(*base);
                Value::new(
                    value.ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index,
                    },
                )
            }
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. } | ValueKind::StructLit { .. } => {
                let owning = !self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty);
                if owning && !is_view(&value.ty) {
                    self.materialize_scoped_owner(value)
                } else {
                    value
                }
            }
            ValueKind::Conditional { .. } => self.hoist_view_conditional_arms(value),
            _ => value,
        }
    }

    /// True when any reachable arm of a conditional owns storage to fill.
    ///
    /// A place-rooted arm stays an inline read; every other arm — a fresh call
    /// result, a literal, or a nested conditional of either — needs its own
    /// evaluated temporary.
    fn conditional_needs_arm_fill(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => self.arm_needs_fill(then_value) || self.arm_needs_fill(else_value),
            _ => false,
        }
    }

    /// True when one conditional arm needs an evaluated temporary.
    ///
    /// The chain walks to the arm's root: a place-rooted arm stays inline, a
    /// temporary or an already-filled value needs no second owner, and any
    /// other root needs a fill exactly when it owns storage to release — a
    /// duplicable root (such as a struct of scalars) owns nothing.
    fn arm_needs_fill(&self, value: &Value) -> bool {
        if self.projects_from_place(value) {
            return false;
        }
        match &value.kind {
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.arm_needs_fill(base)
            }
            ValueKind::Conditional { .. } => self.conditional_needs_arm_fill(value),
            ValueKind::Temp(_) | ValueKind::Materialize { .. } => false,
            _ => !self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty),
        }
    }

    /// True when a conditional has an arm that projects out of an ownerless
    /// root.
    ///
    /// The arm's value is a selected member or element, so binding the arm
    /// value alone would copy the selected part and leave the root's other
    /// owning fields unowned. Such an arm binds its root instead.
    fn conditional_has_ownerless_projection_arm(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.arm_has_ownerless_projection(then_value)
                    || self.arm_has_ownerless_projection(else_value)
            }
            _ => false,
        }
    }

    /// True when one conditional arm projects out of an ownerless root.
    fn arm_has_ownerless_projection(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Member { .. } | ValueKind::Index { .. } => {
                !self.projects_from_place(value)
            }
            ValueKind::Conditional { .. } => self.conditional_has_ownerless_projection_arm(value),
            _ => false,
        }
    }

    /// Evaluates a conditional lend argument's ownerless arms at its position.
    ///
    /// The condition is evaluated once into a temporary; each ownerless arm
    /// runs inside a branch only its own path reaches, filling a registered
    /// temporary the statement's release frees. Place-rooted arms stay inline
    /// reads, and the argument expression selects between the filled
    /// temporaries and the places — so the argument keeps its position in the
    /// left-to-right evaluation order and every argument ABI sees plain places
    /// and a select of places.
    fn hoist_ownerless_conditional_arms(&mut self, value: Value) -> Value {
        match value.kind {
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let cond_ty = condition.ty.clone();
                let cond_id = self.new_temp(cond_ty.clone());
                let cond_place = Place {
                    base: ir::PlaceBase::Temp(cond_id),
                    projections: Vec::new(),
                    ty: cond_ty.clone(),
                };
                self.emit(Statement::Assign {
                    target: cond_place.clone(),
                    value: *condition,
                });
                let cond_value =
                    || Value::new(cond_ty.clone(), ValueKind::Place(cond_place.clone()));
                let then_block = self.new_block();
                let else_block = self.new_block();
                let join = self.new_block();
                self.terminate(Terminator::Branch {
                    condition: cond_value(),
                    on_true: then_block,
                    on_false: else_block,
                    shape: BranchShape::If { join },
                });
                self.switch_to(then_block);
                let then_value = self.hoist_ownerless_arm(*then_value);
                self.terminate(Terminator::Goto(join));
                self.switch_to(else_block);
                let else_value = self.hoist_ownerless_arm(*else_value);
                self.terminate(Terminator::Goto(join));
                self.switch_to(join);
                Value::new(
                    value.ty,
                    ValueKind::Conditional {
                        condition: Box::new(cond_value()),
                        then_value: Box::new(then_value),
                        else_value: Box::new(else_value),
                    },
                )
            }
            other => Value::new(value.ty, other),
        }
    }

    /// Evaluates one conditional arm at its own branch position.
    ///
    /// A place-rooted arm stays an inline read; a nested conditional recurses
    /// into its own branch; a projection arm binds its own root, so a selected
    /// member read does not release alone while the root's other fields keep
    /// their storage; every other arm fills a registered temporary whose
    /// release the statement's cleanup runs.
    fn hoist_ownerless_arm(&mut self, value: Value) -> Value {
        if self.projects_from_place(&value) {
            return value;
        }
        match value.kind {
            ValueKind::Member { base, name } => {
                let base = self.hoist_ownerless_arm(*base);
                return Value::new(
                    value.ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                );
            }
            ValueKind::Index { base, index } => {
                let base = self.hoist_ownerless_arm(*base);
                return Value::new(
                    value.ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index,
                    },
                );
            }
            ValueKind::Conditional { .. } => {
                return self.hoist_ownerless_conditional_arms(value);
            }
            ValueKind::Temp(id) => {
                if !self.has_pending_release(id, &value.ty) {
                    self.register_pending_release(id, &value.ty);
                }
                return value;
            }
            _ => {}
        }
        let ty = self.materialized_ty(&value);
        let filled = self.materialize_value(value);
        if let ValueKind::Temp(id) = filled.kind {
            self.mark_destination_temp(id);
            self.register_pending_release(id, &ty);
        }
        filled
    }

    /// True when a value may hand back storage one of its arguments owns.
    ///
    /// A receiver-less call reborrows an owned argument when the callee can
    /// return a lend of it, so the result aliases storage the argument's owner
    /// already releases. The check follows a conditional's arms because either
    /// arm may be the lender; a func-typed field carries no declared lend
    /// parameter and is never consulted.
    fn value_may_lend(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Call {
                callee,
                args,
                receiver,
                ..
            } if receiver.is_none() => self.lowered_lend_source(callee).is_some_and(|index| {
                args.get(index)
                    .is_some_and(|arg| self.owns_drop(&arg.ty) || self.maybe_owns(&arg.ty))
            }),
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => self.value_may_lend(then_value) || self.value_may_lend(else_value),
            _ => false,
        }
    }

    /// Binds an unnamed owned call result so the scope cleanup can release it.
    ///
    /// A call result the callee only borrows is owned by the caller, but an
    /// unnamed temporary has no binding to drop, so it leaks. The value is
    /// bound to a compiler-created local; the scope's normal cleanup releases
    /// it. A `take` argument is skipped because the callee owns it, and a
    /// duplicable or view value owns nothing to release. The speculative passes
    /// and a deferred block are skipped: their bindings do not survive into the
    /// function's local list.
    ///
    /// The binding is placed in the function body scope rather than the scope
    /// of the statement that produced it. A view derived from the value (for
    /// example a slice or byte view stored in an outer struct) can outlive the
    /// block that produced it, and a drop at the block's end would free the
    /// storage while the view still points at it. The body scope's cleanup
    /// outlives every such view. A loop still assigns the one temporary each
    /// iteration, and the assignment releases the previous value, so the wider
    /// scope does not accumulate storage.
    pub(super) fn materialize_owned_argument(&mut self, value: Value, take: bool) -> Value {
        if take {
            // A `take` projection of a mixed conditional moves the selected
            // member out of the chosen arm's root before the call runs: the
            // caller's place is emptied, the fresh arm's root releases only
            // its un-moved fields, and the callee owns the argument exactly
            // once. Every arm fills one shared carrier the post-call move-out
            // empties, so a re-executed statement never frees the transferred
            // value twice. A conditionally evaluated argument keeps its
            // existing deferred handling.
            if self.defer_move_outs == 0
                && !self.sema.table.duplicable(&value.ty)
                && !mentions_type_param(&value.ty)
                && !is_view(&value.ty)
                && !value.ty.is_unknown()
                && self.consume_needs_conditional_arms(&value)
            {
                return self.materialize_take_conditional_projection(value);
            }
            // The base expression evaluates before the index (`m()[i]` runs
            // `m()` first and it may change the index), so an eager base is
            // filled before the index is pinned. A guarded operand keeps its
            // deferred form and pins the index inside the guard instead. A
            // guarded call root is bound to the deferred guard temporary, which
            // keeps the call on the evaluated path while the take's move-out
            // still empties the selected member.
            let value = if self.defer_move_outs == 0 {
                self.bind_ownerless_projection_root(value)
            } else {
                self.bind_guarded_projection_root(value)
            };
            let value = self.settle_projection_base(value);
            let value = self.materialize_index_chain(value);
            // A `take` argument that reads a projection of a fresh call result
            // has no caller storage to empty: the root is bound to an owned
            // temporary and the projection re-roots there, so the ordinary
            // move-out after the call empties the selected part and the
            // temporary releases the remaining fields. A conditionally
            // evaluated operand is left inline: the root call must run only on
            // the path that evaluates the operand.
            let value = if self.defer_move_outs == 0
                && !self.projects_from_place(&value)
                && matches!(
                    value.kind,
                    ValueKind::Member { .. } | ValueKind::Index { .. }
                )
                && (!self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty))
                && !is_view(&value.ty)
                && !value.ty.is_unknown()
            {
                self.bind_ownerless_projection_root(value)
            } else {
                value
            };
            // A `take` argument must reach the callee with a C place. A value
            // that has none — a fixed-array literal, a by-value fixed-array
            // call, or a struct literal whose field is a fixed array — is
            // stored into an argument temporary. A conditional fixed-array
            // value keeps its array-decay spelling, which is already a C
            // expression.
            // A fixed-array conditional keeps its array-decay spelling, which
            // is already a C expression; every other place-less value is
            // stored into an argument temporary first.
            if self.value_needs_place(&value)
                && !(value.ty.is_fixed_array()
                    && matches!(value.kind, ValueKind::Conditional { .. }))
                && !value.ty.is_unknown()
            {
                return self.materialize_take_place(value);
            }
            // A `take` argument transfers ownership to the callee, so an owned
            // conditional must not pass one arm directly: the chosen arm has to
            // move into an argument temporary and the caller's arm be emptied.
            // The temporary is not released by the caller; the callee's
            // parameter cleanup owns the value.
            if matches!(value.kind, ValueKind::Conditional { .. })
                && !value.ty.is_fixed_array()
                && (!self.sema.table.duplicable(&value.ty) || self.maybe_owns(&value.ty))
                && !is_view(&value.ty)
                && !value.ty.is_unknown()
            {
                return self.materialize_take_place(value);
            }
            return value;
        }
        // A conditional lend argument may mix a place-rooted arm with an
        // ownerless one. Binding the whole conditional would adopt the chosen
        // arm — the store empties a place the callee only borrows — while
        // leaving it entirely inline leaks the ownerless arm's storage. Each
        // ownerless arm is evaluated under a branch instead: the fill runs
        // only on the path that evaluates that arm, and the statement's
        // registered release frees it there, so the place stays readable and
        // the fresh value releases. A normal argument position evaluates the
        // fills in branches at this statement position, so a later argument's
        // evaluation still follows this one; a genuinely conditionally
        // evaluated operand keeps the expression form, because it has no
        // statement position of its own.
        if matches!(value.kind, ValueKind::Conditional { .. })
            && self.conditional_borrows_place(&value)
            && self.conditional_needs_arm_fill(&value)
            && !self.speculative_lowering
            && self.materialize_owned_temporaries
            && !value.ty.is_unknown()
        {
            return if self.defer_move_outs == 0 {
                self.hoist_ownerless_conditional_arms(value)
            } else {
                self.own_ownerless_conditional_arms(value)
            };
        }
        if value.ty.is_closure()
            || (self.sema.table.duplicable(&value.ty)
                && !self.maybe_owns(&value.ty)
                && !self.value_needs_place(&value))
            || is_view(&value.ty)
            || self.projects_from_place(&value)
            || self.projects_from_conditional_borrow(&value)
            || self.speculative_lowering
            || !self.materialize_owned_temporaries
            || value.ty.is_unknown()
        {
            return value;
        }
        let name = format!("__cl_owned{}", self.next_owned_temp);
        self.next_owned_temp += 1;
        let decl_ty = self.materialized_ty(&value);
        let id = self.declare_generated_local(&name, decl_ty, self.current_span);
        self.move_binding_to_body_scope(id);
        self.set_place(
            id,
            PlaceState {
                initialized: true,
                ..PlaceState::default()
            },
        );
        self.bindings[id].used = true;
        let place = self.binding_place(id);
        self.emit(Statement::Assign {
            target: place.clone(),
            value,
        });
        Value::new(place.ty.clone(), ValueKind::Place(place))
    }

    /// Binds an owned view backing to a local in the declaring scope.
    ///
    /// The storage must outlive the view binding that reads it and no longer:
    /// the local stays in the current scope instead of moving to the function
    /// body, so the scope's cleanup releases the backing after the binding's
    /// last use and the liveness pass rejects a view that escapes the scope
    /// (`E10030`), exactly as it rejects an escaped view of a named local.
    ///
    /// Unlike [`Self::materialize_owned_argument`], the binding is declared
    /// even in a speculative pass that reports no diagnostics: the view's
    /// owner and provenance must be the same in every pass over the body, or
    /// the loan a loop's entry state carries would name a different owner than
    /// the real pass creates. A body retained for a deferred registration and
    /// an unknown type are returned unchanged.
    pub(super) fn materialize_scoped_owner(&mut self, value: Value) -> Value {
        if self.projects_from_place(&value)
            || !self.materialize_owned_temporaries
            || value.ty.is_unknown()
        {
            return value;
        }
        let name = format!("__cl_owned{}", self.next_owned_temp);
        self.next_owned_temp += 1;
        let decl_ty = self.materialized_ty(&value);
        let id = self.declare_generated_local(&name, decl_ty, self.current_span);
        self.set_place(
            id,
            PlaceState {
                initialized: true,
                ..PlaceState::default()
            },
        );
        self.bindings[id].used = true;
        let place = self.binding_place(id);
        self.emit(Statement::Assign {
            target: place.clone(),
            value,
        });
        Value::new(place.ty.clone(), ValueKind::Place(place))
    }

    /// Moves a just-declared generated local into the function body scope.
    ///
    /// The binding is removed from the innermost scope's drop order and
    /// recorded in the body scope instead, so its cleanup runs at the end of
    /// the function rather than at the end of the block that declared it. The
    /// body scope is the first scope and has depth one; a binding already
    /// declared there is left alone. The generated binding has no name-map
    /// entry to move: it is referenced by id only.
    fn move_binding_to_body_scope(&mut self, id: usize) {
        // A defer body's owner keeps the block's own cleanup: the deferred
        // copy runs from every cleanup edge, so a body-scope release would run
        // once for the whole program instead of once per execution.
        if self.scopes.len() <= 1 || self.defer_body {
            return;
        }
        let innermost = self.scopes.len() - 1;
        if let Some(scope) = self.scopes.get_mut(innermost) {
            scope.order.retain(|entry| *entry != id);
        }
        self.bindings[id].scope = 1;
        if let Some(scope) = self.scopes.get_mut(0) {
            scope.order.push(id);
        }
    }
}

/// True when a type mentions a generic type parameter anywhere.
///
/// Such a type has no C spelling until its instantiation substitutes the
/// parameter, so it cannot be declared as a temporary in the uninstantiated
/// body that produced it.
pub(super) fn mentions_type_param(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Param(_) => true,
        TyKind::Raw(inner) => mentions_type_param(inner),
        TyKind::Array { element, .. } => mentions_type_param(element),
        TyKind::Struct(_, args) => args.iter().any(mentions_type_param),
        TyKind::Func(signature) => {
            signature
                .params
                .iter()
                .any(|param| mentions_type_param(&param.ty))
                || mentions_type_param(&signature.returns)
        }
        TyKind::Closure(closure) => {
            closure.params.iter().any(mentions_type_param)
                || mentions_type_param(&closure.returns)
        }
        _ => false,
    }
}

/// One projection segment between a consumed value and its conditional root.
///
/// The steps are recorded from the source's outermost segment inward, together
/// with the type each segment selects, so an arm can rebuild the same chain
/// over its own value without consulting the type table again.
enum ConditionalProjectionStep {
    /// A field selection and the selected field's type.
    Field { name: String, ty: Ty },
    /// An element selection with its index value and element type.
    Index { index: Value, ty: Ty },
}

/// Rebuilds a value by applying projection steps from the innermost outward.
///
/// The steps were recorded outermost first, so applying them in reverse order
/// reconstructs the source's own chain shape over the given value.
fn apply_conditional_steps(mut value: Value, steps: &[ConditionalProjectionStep]) -> Value {
    for step in steps.iter().rev() {
        value = match step {
            ConditionalProjectionStep::Field { name, ty } => Value::new(
                ty.clone(),
                ValueKind::Member {
                    base: Box::new(value),
                    name: name.clone(),
                },
            ),
            ConditionalProjectionStep::Index { index, ty } => Value::new(
                ty.clone(),
                ValueKind::Index {
                    base: Box::new(value),
                    index: Box::new(index.clone()),
                },
            ),
        };
    }
    value
}

/// True when evaluating a value has no side effect and may be rendered twice.
///
/// A call, an intrinsic, or a template can allocate, read, or write, so it is
/// not safe to re-evaluate; every other shape is a read of a place or a pure
/// operator over pure operands. A discarded expression statement with a pure
/// value has nothing to run and lowers to nothing.
pub(super) fn is_pure_value(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Call { .. }
        | ValueKind::Intrinsic { .. }
        | ValueKind::Template(_)
        | ValueKind::ClosureCall { .. }
        | ValueKind::Materialize { .. } => false,
        ValueKind::Closure { captures, .. } => captures.iter().all(is_pure_value),
        ValueKind::Unary { op, operand } => !op.is_update() && is_pure_value(operand),
        ValueKind::Binary { left, right, .. } => is_pure_value(left) && is_pure_value(right),
        ValueKind::Cast { value: inner, .. } => is_pure_value(inner),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            is_pure_value(condition) && is_pure_value(then_value) && is_pure_value(else_value)
        }
        ValueKind::Member { base, .. } => is_pure_value(base),
        ValueKind::Index { base, index } => is_pure_value(base) && is_pure_value(index),
        ValueKind::StructLit { fields, .. } => {
            fields.iter().all(|(_, field)| is_pure_value(field))
        }
        ValueKind::ArrayLit { elements, fill } => {
            elements.iter().all(is_pure_value) && fill.as_deref().map_or(true, is_pure_value)
        }
        _ => true,
    }
}

/// The outermost extent of a fixed array, when the type is one.
///
/// A fixed array's length is a compile-time constant, so `len` of one needs no
/// value: nothing is bound and the base reads no storage.
pub(super) fn fixed_array_extent(ty: &Ty) -> Option<usize> {
    match &ty.kind {
        TyKind::Array { extents, .. } if extents.iter().all(Option::is_some) => {
            extents.first().and_then(|size| *size)
        }
        _ => None,
    }
}

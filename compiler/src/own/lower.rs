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
            ExprKind::Qualified(name) => match self.place_target(expr) {
                Some(target) => ValueKind::Place(self.target_place(&target)),
                None => ValueKind::Path(name.segments.clone()),
            },
            ExprKind::Member { base, name } => ValueKind::Member {
                base: Box::new(self.lower_value(base)),
                name: name.clone(),
            },
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
                    let values: Vec<Value> = args
                        .iter()
                        .map(|arg| self.lower_value(&arg.value))
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
                    // A call through a `func`-typed field has no declared
                    // parameter modes, so it never moves an argument out even
                    // when a same-named global function would.
                    let move_outs = if call.receiver.is_none() {
                        self.take_move_out_places(&call.name, &call.args)
                    } else {
                        Vec::new()
                    };
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
                            },
                        );
                    } else {
                        // The call consumes a `take` argument before the
                        // enclosing statement continues, so the result is
                        // stored in a temporary and the consumed places are
                        // emptied immediately afterward.
                        let temp = self.materialize_value(Value::new(ty.clone(), kind));
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
                let else_value = self.lower_value(else_value);
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
                // source and empties it.
                elements: elements
                    .iter()
                    .map(|element| {
                        let value = self.lower_value(element);
                        if self.defer_move_outs > 0 {
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
                let mut values = Vec::new();
                let mut type_arg = None;
                for arg in args {
                    match arg {
                        IntrinsicArg::Expr(inner) => values.push(self.lower_value(inner)),
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
        let saved_continues = std::mem::take(&mut self.continue_states);
        let saved_defer = self.defer_anchor;
        let saved_move_outs = self.defer_move_outs;
        let saved_registration = self.next_registration;
        let saved_owned_temp = self.next_owned_temp;
        let saved_returns = self.returns.clone();
        let saved_current_span = self.current_span;
        self.next_registration = 0;
        self.next_owned_temp = 0;
        self.defer_anchor = None;
        // A lifted lambda body is evaluated when the closure is called, not
        // where the closure literal is written, so the enclosing operand's
        // deferred move-out context does not apply inside it.
        self.defer_move_outs = 0;
        self.returns = ret.clone();

        let body_span = match body {
            LambdaBody::Expr(value) => value.span,
            LambdaBody::Block(block) => block.span,
        };
        self.push_scope(body_span);
        for (param, ty) in params.iter().zip(param_tys) {
            self.declare_lambda_param(&param.name, ty.clone());
        }
        self.declare_captures(site);
        let returned = match body {
            LambdaBody::Expr(value) => Some(self.lower_value(value)),
            LambdaBody::Block(block) => {
                self.check_block(block);
                None
            }
        };
        self.pop_scope();
        if !self.is_terminated() {
            let releases = self.pending_release_copies();
            self.record_release_events(&releases);
            self.terminate(Terminator::Return {
                value: returned,
                releases,
            });
        }

        let lambda_builder = std::mem::replace(
            &mut self.builder,
            Builder::new("__cl_lifted_placeholder", Vec::new(), Ty::void()),
        );
        let mut function = lambda_builder.finish();
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
        liveness::check(&function, &self.events, &self.binding_names, self.diagnostics);
        self.lifted.push(function);

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
        self.continue_states = saved_continues;
        self.defer_anchor = saved_defer;
        self.defer_move_outs = saved_move_outs;
        self.next_registration = saved_registration;
        self.next_owned_temp = saved_owned_temp;
        self.returns = saved_returns;
        self.current_span = saved_current_span;
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
    pub(super) fn lower_discarded(&mut self, expr: &'a Expr) {
        let value = self.lower_value(expr);
        if is_pure_value(&value) {
            return;
        }
        self.emit(Statement::Discard { value });
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

    /// Consumes an owned place value as a move, emptying its source.
    ///
    /// A place value whose type is non-duplicable and does not mention a generic
    /// parameter is moved into a compiler temporary: `Statement::Move` copies
    /// the value and then zeros the source, so a later drop of the source cannot
    /// free the value the destination now owns. Every other value is returned
    /// unchanged. This realizes the "a move is a copy plus a zero of the source"
    /// rule for a value consumed inside an aggregate literal.
    ///
    /// A projection is prepared like a store before its place is derived: a
    /// fresh call root is bound to an owned temporary, an eager base is filled,
    /// and a run-time index is pinned. Without that preparation the root call
    /// would never run and a run-time index would fall back to the element-zero
    /// form, so the move could empty the wrong slot. A projection over caller
    /// storage keeps the ordinary move.
    pub(super) fn consume_place(&mut self, value: Value) -> Value {
        // A maybe-owning value moves through the same shape: the emitted move
        // copies and zeroes only for an owning instance.
        if !(self.owns_drop(&value.ty) || self.maybe_owns(&value.ty)) {
            return value;
        }
        let value = if matches!(
            value.kind,
            ValueKind::Member { .. } | ValueKind::Index { .. }
        ) {
            let value = self.bind_ownerless_projection_root(value);
            let value = self.settle_projection_base(value);
            self.materialize_index_chain(value)
        } else {
            value
        };
        let Some(source) = self.value_place(&value) else {
            return value;
        };
        let ty = self.materialized_ty(&value);
        let id = self.new_temp(ty.clone());
        let target = Place {
            base: ir::PlaceBase::Temp(id),
            projections: Vec::new(),
            ty: ty.clone(),
        };
        self.emit(Statement::Move {
            target,
            source,
        });
        Value::new(ty, ValueKind::Temp(id))
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
            ValueKind::Call { callee, args, .. } => {
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
        match self.program.callee(callee) {
            Some(Callee::User(decl)) => {
                let type_params: Vec<String> = decl
                    .type_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect();
                let returns = resolve_ty(&decl.returns, &type_params, &self.sema.table);
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
                            && (is_lend_candidate(&resolve_ty(
                                &param.ty,
                                &type_params,
                                &self.sema.table,
                            )) || mentions_type_param(&resolve_ty(
                                &param.ty,
                                &type_params,
                                &self.sema.table,
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
    fn bind_guarded_projection_root(&mut self, value: Value) -> Value {
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
            _ => value,
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
        if value.ty.is_closure()
            || (self.sema.table.duplicable(&value.ty)
                && !self.maybe_owns(&value.ty)
                && !self.value_needs_place(&value))
            || is_view(&value.ty)
            || self.projects_from_place(&value)
            || !self.report_diagnostics
            || !self.materialize_owned_temporaries
            || value.ty.is_unknown()
        {
            return value;
        }
        let name = format!("__cl_owned{}", self.next_owned_temp);
        self.next_owned_temp += 1;
        let decl_ty = self.materialized_ty(&value);
        let id = self.declare_local(&name, decl_ty, false, self.current_span);
        self.move_binding_to_body_scope(id, &name);
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

    /// Moves a just-declared local into the function body scope.
    ///
    /// The binding is removed from the innermost scope's name map and drop
    /// order and recorded in the body scope instead, so its cleanup runs at the
    /// end of the function rather than at the end of the block that declared
    /// it. The body scope is the first scope and has depth one; a binding
    /// already declared there is left alone.
    fn move_binding_to_body_scope(&mut self, id: usize, name: &str) {
        if self.scopes.len() <= 1 {
            return;
        }
        let innermost = self.scopes.len() - 1;
        if let Some(scope) = self.scopes.get_mut(innermost) {
            scope.order.retain(|entry| *entry != id);
            scope.names.remove(name);
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

/// True when evaluating a value has no side effect and may be rendered twice.
///
/// A call, an intrinsic, or a template can allocate, read, or write, so it is
/// not safe to re-evaluate; every other shape is a read of a place or a pure
/// operator over pure operands.
fn is_pure_value(value: &Value) -> bool {
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

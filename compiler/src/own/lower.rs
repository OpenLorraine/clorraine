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
    pub(super) fn lower_value(&mut self, expr: &Expr) -> Value {
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
                let base = self.lower_value(base);
                let index = self.lower_value(index);
                let base = self.materialize_guard_operand(base);
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
                if let Some((intrinsic_ty, intrinsic_kind)) =
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
                let right = self.lower_value(right);
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
            } => ValueKind::Conditional {
                condition: Box::new(self.lower_value(condition)),
                then_value: Box::new(self.lower_value(then_value)),
                else_value: Box::new(self.lower_value(else_value)),
            },
            ExprKind::StructLiteral { name, fields, .. } => ValueKind::StructLit {
                name: name.segments.last().cloned().unwrap_or_default(),
                fields: fields
                    .iter()
                    .map(|field| (field.name.clone(), self.lower_value(&field.value)))
                    .collect(),
            },
            ExprKind::ArrayLiteral { elements, fill } => ValueKind::ArrayLit {
                elements: elements.iter().map(|element| self.lower_value(element)).collect(),
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
            _ => ValueKind::Aggregate,
        };
        Value { ty, kind }
    }

    /// Defers a side-effecting guarded operand into a temporary, evaluated once.
    ///
    /// A runtime guard and the operation it protects both read the operand. An
    /// operand with no side effect is safe to render twice, so it is returned
    /// unchanged. A side-effecting one (a call such as `a[next()]` or
    /// `10/d()`) is wrapped in a [`ValueKind::Materialize`] so both reads see
    /// the same value and the call runs once. The assignment that fills the
    /// temporary is emitted by the guard, which keeps it on the path where the
    /// operand is actually evaluated. A fixed array cannot be assigned to a
    /// temporary in C, so it is left alone as well.
    pub(super) fn materialize_guard_operand(&mut self, value: Value) -> Value {
        if is_pure_value(&value) {
            return value;
        }
        if matches!(
            &value.ty.kind,
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some)
        ) {
            return value;
        }
        // A type that still names a generic parameter cannot be declared as a
        // C temporary in a caller that has no substitution for it; the operand
        // is left inline instead, which is safe when it has no side effect.
        if mentions_type_param(&value.ty) {
            return value;
        }
        // The value is assigned to the temporary where the guard runs, not
        // here: an operand inside `&&`/`||`/`?:` is evaluated only on the path
        // that reaches it, so a statement emitted now would run it even when
        // the left operand short-circuits.
        let ty = value.ty.clone();
        let temp = self.new_temp(ty.clone());
        Value::new(
            ty,
            ValueKind::Materialize {
                temp,
                value: Box::new(value),
            },
        )
    }

    /// Stores a value in a compiler temporary and returns the temporary value.
    ///
    /// The assignment is emitted into the current block, so the temporary is
    /// computed before the construct that reads it. The temporary is an
    /// unnamed compiler slot, so it takes no part in the ownership walk.
    pub(super) fn materialize_value(&mut self, value: Value) -> Value {
        let ty = value.ty.clone();
        let id = self.new_temp(ty.clone());
        let place = Place {
            base: ir::PlaceBase::Temp(id),
            projections: Vec::new(),
            ty: ty.clone(),
        };
        self.emit(Statement::Assign { target: place, value });
        Value::new(ty, ValueKind::Temp(id))
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
        if is_pure_value(&value) {
            return value;
        }
        if matches!(
            &value.ty.kind,
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some)
        ) {
            return value;
        }
        if mentions_type_param(&value.ty) {
            return value;
        }
        self.materialize_value(value)
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
        if take
            || self.sema.table.duplicable(&value.ty)
            || is_view(&value.ty)
            || matches!(value.kind, ValueKind::Place(_))
            || !self.report_diagnostics
            || !self.materialize_owned_temporaries
            || value.ty.is_unknown()
            || mentions_type_param(&value.ty)
        {
            return value;
        }
        let name = format!("__cl_owned{}", self.next_owned_temp);
        self.next_owned_temp += 1;
        let id = self.declare_local(&name, value.ty.clone(), false, self.current_span);
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
fn mentions_type_param(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Param(_) => true,
        TyKind::Raw(inner) => mentions_type_param(inner),
        TyKind::Array { element, .. } => mentions_type_param(element),
        TyKind::Struct(_, args) => args.iter().any(mentions_type_param),
        TyKind::Func(signature) => {
            signature.params.iter().any(mentions_type_param)
                || mentions_type_param(&signature.returns)
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
        | ValueKind::Materialize { .. } => false,
        ValueKind::Unary { operand, .. } => is_pure_value(operand),
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

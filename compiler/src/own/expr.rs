//! Expression checking and call handling for the ownership stage.
//!
//! This walk reads every expression with the types the type stage
//! recorded, applies the move and initialization rules to reads, assigns
//! each assignment its place rules, and checks calls against the declared
//! parameter modes, the `ffi` contracts, and the standard-library view
//! helpers.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    // -- expressions ---------------------------------------------------------

    /// The recorded type of an expression.
    ///
    /// The type stage records every expression the ownership walk can reach,
    /// so a miss means the two stages disagree about the tree shape. That is
    /// an internal error: it must not silently become the opaque type, because
    /// the opaque type disables every ownership check.
    pub(super) fn ty(&self, expr: &Expr) -> Ty {
        self.sema
            .expr_types
            .get(&(expr as *const Expr as usize))
            .cloned()
            .expect("internal error: the type stage recorded no type for this expression")
    }

    /// The recorded type of an expression, when the type stage recorded one.
    ///
    /// Unlike [`Self::ty`], this never panics: a callee expression such as a
    /// function name is not always recorded, so a caller that only asks whether
    /// the callee is a closure reads it through this method.
    pub(super) fn recorded_ty(&self, expr: &Expr) -> Option<Ty> {
        self.sema
            .expr_types
            .get(&(expr as *const Expr as usize))
            .cloned()
    }

    /// True when a call's callee expression lowers to a closure value.
    ///
    /// A capturing lambda's recorded type is a closure, but a capture-less
    /// lambda is a `func`; its literal form is recognized by shape so a direct
    /// `((x: i32) => x + 1)(4)` still becomes a closure call.
    pub(super) fn is_closure_callee(&self, callee: &Expr) -> bool {
        matches!(callee.kind, ExprKind::Lambda { .. })
            || self.recorded_ty(callee).is_some_and(|ty| ty.is_closure())
    }

    /// Resolves a written type in the current generic scope.
    pub(super) fn resolve(&self, ty: &crate::ast::Type) -> Ty {
        resolve_ty(ty, &self.type_params, &self.sema.table)
    }

    /// Checks and walks an expression, returning its type.
    pub(super) fn eval(&mut self, expr: &'a Expr) -> Ty {
        let ty = self.ty(expr);
        match &expr.kind {
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Char(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Error
            | ExprKind::Lambda { .. } => {}
            ExprKind::Name(name) => {
                if let Some(id) = self.lookup(name) {
                    self.read_place(id, &[], expr.span);
                }
            }
            ExprKind::Qualified(name) => self.eval_qualified(name, expr.span),
            ExprKind::Member { base, .. } => {
                let base_ty = self.eval(base);
                if base_ty.nullable {
                    self.check_nonnull_expr(base, expr.span);
                }
                self.check_member_raw(&base_ty, expr.span);
            }
            ExprKind::Index { base, index } => {
                let base_ty = self.eval(base);
                if matches!(base_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
                    self.report_unsafe(UnsafeOp::PointerArithmetic, expr.span);
                } else if base_ty.nullable && self.unsafe_depth == 0 {
                    self.check_nonnull_expr(base, expr.span);
                }
                self.eval(index);
            }
            ExprKind::Unary { op, operand } => {
                let operand_ty = self.eval(operand);
                if matches!(op, UnaryOp::AddressOf) && self.unsafe_depth == 0 {
                    self.report_unsafe(UnsafeOp::AddressOf, expr.span);
                }
                if matches!(op, UnaryOp::Deref) {
                    if self.unsafe_depth == 0 {
                        self.report_unsafe(UnsafeOp::Deref, expr.span);
                    }
                    if operand_ty.nullable {
                        self.check_nonnull_expr(operand, expr.span);
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                let left_ty = self.eval(left);
                let right_ty = self.eval(right);
                if matches!(op, BinaryOp::Add | BinaryOp::Sub)
                    && (matches!(left_ty.kind, TyKind::Raw(_))
                        || matches!(right_ty.kind, TyKind::Raw(_)))
                    && self.unsafe_depth == 0
                {
                    self.report_unsafe(UnsafeOp::PointerArithmetic, expr.span);
                }
            }
            ExprKind::Assign { op, target, value } => self.check_assign(*op, target, value),
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.eval(condition);
                self.eval(then_value);
                self.eval(else_value);
            }
            ExprKind::Cast { value, .. } => {
                self.eval(value);
            }
            ExprKind::RawCast { value, .. } => {
                let value_ty = self.eval(value);
                if matches!(value_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
                    self.report_unsafe(UnsafeOp::PointerCast, expr.span);
                }
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.eval(&field.value);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.eval(element);
                }
                if let Some(fill) = fill {
                    self.eval(fill);
                }
            }
            ExprKind::Unsafe(inner) => {
                self.unsafe_depth += 1;
                self.eval(inner);
                self.unsafe_depth -= 1;
            }
            ExprKind::IntrinsicCall { name, args } => {
                if ffi::is_volatile_intrinsic(name) && self.unsafe_depth == 0 {
                    self.report_unsafe(UnsafeOp::Volatile, expr.span);
                }
                for arg in args {
                    if let crate::ast::IntrinsicArg::Expr(inner) = arg {
                        self.eval(inner);
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let crate::ast::TemplatePiece::Placeholder(inner) = piece {
                        self.eval(inner);
                    }
                }
            }
            ExprKind::Call {
                callee,
                type_args: _,
                args,
            } => {
                self.check_call_expr(callee, args, expr.span);
            }
        }
        ty
    }

    /// Walks a dotted name, checking each hop's read and raw dereference.
    ///
    /// Every segment is walked, not just the first, so a raw pointer reached
    /// part way through a chain (`o.inner.x` where `inner: raw Inner`) still
    /// reports the implied dereference.
    pub(super) fn eval_qualified(&mut self, name: &crate::ast::QualifiedName, span: Span) {
        let Some(first) = name.segments.first() else {
            return;
        };
        let Some(id) = self.lookup(first) else {
            return;
        };
        let projections: Vec<ir::Projection> = name.segments[1..]
            .iter()
            .map(|segment| ir::Projection::Field(segment.clone()))
            .collect();
        self.read_place(id, &projections, span);
        let mut current = self.bindings[id].ty.clone();
        for segment in &name.segments[1..] {
            self.check_member_raw(&current, span);
            current = self.member_type(&current, segment);
        }
    }

    /// The type of a field selected from `base`, following a raw pointer.
    pub(super) fn member_type(&self, base: &Ty, field: &str) -> Ty {
        match &base.kind {
            TyKind::Struct(name, _) => self
                .sema
                .table
                .struct_fields(name)
                .and_then(|fields| {
                    fields
                        .iter()
                        .find(|(declared, _)| declared == field)
                        .map(|(_, ty)| ty.clone())
                })
                .unwrap_or_else(Ty::unknown),
            TyKind::Raw(inner) => self.member_type(inner, field),
            _ => Ty::unknown(),
        }
    }

    /// Reports `E10600` when a member is reached through a raw pointer.
    pub(super) fn check_member_raw(&mut self, base_ty: &Ty, span: Span) {
        if matches!(base_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
            self.report_unsafe(UnsafeOp::Deref, span);
        }
    }

    /// Reports a nullable place used where a non-null value is required.
    ///
    /// The check covers projected places too, so a nullable struct field is
    /// reported unless that exact field path was narrowed.
    pub(super) fn check_nonnull_expr(&mut self, expr: &Expr, span: Span) {
        if let Some((id, projections)) = self.plain_binding(expr) {
            if !self.state.place(id).is_narrowed(&projections) {
                let name = self.bindings[id].name.clone();
                self.error(
                    "E10010",
                    format!("value '{name}' may be null here"),
                    "check `!= null` before the use",
                    span,
                );
            }
        }
    }

    /// Checks an expression against an expected type for nullability.
    pub(super) fn check_value_into(&mut self, expr: &Expr, expected: &Ty, span: Span) {
        if matches!(expr.kind, ExprKind::Null) {
            if !expected.nullable && !expected.is_unknown() {
                self.error(
                    "E10012",
                    "'null' is not allowed here",
                    "declare the type `T?` or avoid the empty state",
                    span,
                );
            }
            return;
        }
        let value_ty = self.ty(expr);
        if value_ty.nullable && !expected.nullable && !expected.is_unknown() {
            self.check_nonnull_expr(expr, span);
        }
    }

    /// Checks an assignment expression.
    ///
    /// A compound assignment `x op= y` is `x = x op y` (Ch 04 §4), so it emits
    /// the same read-modify-write as the equivalent plain assignment: the
    /// current value of the place is read, combined with the right-hand side,
    /// and stored back. The place is lowered once, so a side-effecting element
    /// index runs a single time.
    pub(super) fn check_assign(&mut self, op: AssignOp, target: &'a Expr, value: &'a Expr) {
        let target_ty = self.ty(target);
        let compound = op.binary_op();
        if compound.is_some() && matches!(target_ty.kind, TyKind::Raw(_)) {
            self.report_unsafe(UnsafeOp::PointerArithmetic, target.span);
        }
        self.check_place_raw(target);
        match self.place_target(target) {
            Some(PlaceTarget::Local(id, projections)) => {
                self.eval(value);
                if compound.is_some() {
                    self.read_place(id, &projections, target.span);
                }
                if !projections.is_empty() && self.is_loan_value(value) {
                    let name = self.bindings[id].name.clone();
                    self.error(
                        "E10033",
                        format!("borrow of '{name}' escapes its scope"),
                        "derive the view from a parameter or take ownership",
                        target.span,
                    );
                }
                self.write_place(id, &projections, target.span);
                if has_runtime_element(&projections) {
                    self.emit_indexed_store(target, value, compound);
                } else {
                    let place = self.target_place(&PlaceTarget::Local(id, projections.clone()));
                    self.emit_assigned(place, &target_ty, value, compound);
                }
                if projections.is_empty() {
                    self.bind_value(id, &target_ty, value, target.span);
                }
            }
            Some(PlaceTarget::Global(id, projections)) => {
                let is_const = self
                    .program
                    .globals
                    .get(&id)
                    .is_some_and(|global| global.is_const);
                // A field write through a `raw` global reaches the pointee, not
                // the constant pointer binding, so the const does not forbid it.
                let through_pointer = self.writes_through_raw_global(&id, &projections);
                if is_const && !through_pointer {
                    self.error(
                        "E10023",
                        format!("cannot assign to immutable '{}'", id.name),
                        "declare it with `let` (mutable)",
                        target.span,
                    );
                }
                self.eval(value);
                if self.is_loan_value(value) {
                    self.error(
                        "E10033",
                        format!("borrow of '{}' escapes its scope", id.name),
                        "derive the view from a parameter or take ownership",
                        target.span,
                    );
                }
                if !is_const || through_pointer {
                    if has_runtime_element(&projections) {
                        self.emit_indexed_store(target, value, compound);
                    } else {
                        let place = self.target_place(&PlaceTarget::Global(id, projections));
                        self.emit_assigned(place, &target_ty, value, compound);
                    }
                }
            }
            None => {
                self.eval(target);
                self.eval(value);
            }
        }
        self.check_value_into(value, &target_ty, value.span);
    }

    /// Checks an increment or decrement used as a statement.
    ///
    /// `x++`, `++x`, `x--`, and `--x` all reduce to `x = x op 1` (Ch 04 §4), so
    /// they emit the same read-modify-write as a compound assignment. The result
    /// value is discarded, so a postfix and a prefix form lower identically
    /// here; the value forms go through the unary value path instead.
    pub(super) fn check_update(&mut self, op: AssignOp, operand: &'a Expr) {
        let ty = self.ty(operand);
        self.check_place_raw(operand);
        let binary = op.binary_op().expect("the update operators are compound");
        let one = Value::new(ty.clone(), ValueKind::Int(1));
        match self.place_target(operand) {
            Some(PlaceTarget::Local(id, projections)) => {
                self.read_place(id, &projections, operand.span);
                self.write_place(id, &projections, operand.span);
                if has_runtime_element(&projections) {
                    self.emit_indexed_update(operand, binary, one);
                } else {
                    let place = self.target_place(&PlaceTarget::Local(id, projections));
                    self.emit_compound_store(binary, place, &ty, one);
                }
            }
            Some(PlaceTarget::Global(id, projections)) => {
                let is_const = self
                    .program
                    .globals
                    .get(&id)
                    .is_some_and(|global| global.is_const);
                // A field update through a `raw` global reaches the pointee,
                // not the constant pointer binding.
                let through_pointer = self.writes_through_raw_global(&id, &projections);
                if is_const && !through_pointer {
                    self.error(
                        "E10023",
                        format!("cannot assign to immutable '{}'", id.name),
                        "declare it with `let` (mutable)",
                        operand.span,
                    );
                    return;
                }
                if has_runtime_element(&projections) {
                    self.emit_indexed_update(operand, binary, one);
                } else {
                    let place = self.target_place(&PlaceTarget::Global(id, projections));
                    self.emit_compound_store(binary, place, &ty, one);
                }
            }
            None => {
                self.eval(operand);
            }
        }
    }

    /// True when an assignment to a global place goes through a `raw` pointer.
    ///
    /// A module-level `const` of `raw T` is an immutable pointer binding, but
    /// the memory it names is not part of the binding: `BASE.field = ...`
    /// writes the pointee, which the const does not forbid. Only a write with
    /// no projection reaches the binding itself.
    fn writes_through_raw_global(&self, id: &ir::GlobalId, projections: &[ir::Projection]) -> bool {
        !projections.is_empty()
            && self
                .program
                .globals
                .get(id)
                .is_some_and(|global| matches!(global.ty.kind, TyKind::Raw(_)))
    }

    /// Emits the store for an assignment, plain or compound.
    fn emit_assigned(
        &mut self,
        place: Place,
        ty: &Ty,
        value: &'a Expr,
        compound: Option<BinaryOp>,
    ) {
        match compound {
            Some(binary) => {
                let right = self.lower_value(value);
                self.emit_compound_store(binary, place, ty, right);
            }
            None => self.emit_store(place, ty, value),
        }
    }

    /// Emits a read-modify-write `place = place op right`.
    fn emit_compound_store(&mut self, op: BinaryOp, place: Place, ty: &Ty, right: Value) {
        let left = Value::place(place.clone());
        let value = self.combine(op, left, right, ty);
        self.emit(Statement::Assign { target: place, value });
    }

    /// Builds a binary value, materializing a guarded operand once.
    ///
    /// The emitted guard for `/` and `%` re-reads both operands, so a
    /// side-effecting operand is stored in a temporary the guard and the
    /// operation share.
    fn combine(&mut self, op: BinaryOp, left: Value, right: Value, ty: &Ty) -> Value {
        let (left, right) = if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
            (
                self.materialize_guard_operand(left),
                self.materialize_guard_operand(right),
            )
        } else {
            (left, right)
        };
        Value::new(
            ty.clone(),
            ValueKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
        )
    }

    /// Lowers an indexed target to its element read and its address.
    ///
    /// The base and the run-time index are materialized once so the element
    /// read and the store address share one evaluation: a side-effecting index
    /// such as `a[next()]` cannot pass a bounds guard for one value and store
    /// through another. The address keeps the index, so the emitter still
    /// emits the element's bounds check; a plain element projection would lose
    /// the index and hardcode element zero.
    ///
    /// When `shared` is set the element value is used twice (read and address),
    /// so an impure operand is computed into a real temporary to keep its side
    /// effect to one run; a plain store passes `false` and leaves the operand
    /// in its single guarded position.
    fn indexed_target(&mut self, target: &'a Expr, shared: bool) -> Option<(Value, Value, Ty)> {
        let target = match &target.kind {
            ExprKind::Unsafe(inner) => inner,
            _ => target,
        };
        let ExprKind::Index { base, index } = &target.kind else {
            return None;
        };
        let element_ty = self.ty(target);
        let base_value = self.lower_value(base);
        let index_value = self.lower_value(index);
        // A compound store reads the element and takes its address, so the
        // index and base appear in two guarded positions. An impure operand is
        // computed into a real temporary first so it runs once; the plain store
        // leaves the operand inline, where its single guarded position suffices.
        let (base_value, index_value) = if shared {
            (
                self.materialize_operand(base_value),
                self.materialize_operand(index_value),
            )
        } else {
            (
                self.materialize_guard_operand(base_value),
                self.materialize_guard_operand(index_value),
            )
        };
        let element = Value::new(
            element_ty.clone(),
            ValueKind::Index {
                base: Box::new(base_value),
                index: Box::new(index_value),
            },
        );
        let pointer = Value::new(
            element_ty.clone(),
            ValueKind::Unary {
                op: UnaryOp::AddressOf,
                operand: Box::new(element.clone()),
            },
        );
        Some((element, pointer, element_ty))
    }

    /// Lowers an indexed assignment, plain or compound.
    fn emit_indexed_store(&mut self, target: &'a Expr, value: &'a Expr, compound: Option<BinaryOp>) {
        let Some((element, pointer, element_ty)) = self.indexed_target(target, compound.is_some())
        else {
            return;
        };
        let stored = match compound {
            Some(binary) => {
                let right = self.lower_value(value);
                self.combine(binary, element, right, &element_ty)
            }
            None => self.lower_value(value),
        };
        self.emit(Statement::StorePointer {
            pointer,
            value: stored,
        });
    }

    /// Lowers an indexed increment or decrement as a read-modify-write.
    fn emit_indexed_update(&mut self, target: &'a Expr, binary: BinaryOp, one: Value) {
        let Some((element, pointer, element_ty)) = self.indexed_target(target, true) else {
            return;
        };
        let stored = self.combine(binary, element, one, &element_ty);
        self.emit(Statement::StorePointer {
            pointer,
            value: stored,
        });
    }

    /// Reports raw operations implied by an assignment target's projections.
    ///
    /// Walking the projections rather than the whole expression keeps the
    /// target from being read as a value, which would be wrong for a field of
    /// a partially initialized or partially moved struct.
    pub(super) fn check_place_raw(&mut self, target: &'a Expr) {
        let Some(base) = self.place_target(target) else {
            return;
        };
        // A field write through a raw pointer is a dereference whether the
        // pointer is a local or a module-level binding, so both roots are
        // walked for the unsafe gate.
        let (mut current, projections) = match base {
            PlaceTarget::Local(id, projections) => (self.bindings[id].ty.clone(), projections),
            PlaceTarget::Global(id, projections) => {
                let ty = self
                    .program
                    .globals
                    .get(&id)
                    .map(|global| global.ty.clone())
                    .unwrap_or_else(Ty::unknown);
                (ty, projections)
            }
        };
        for projection in &projections {
            match projection {
                ir::Projection::Field(field) => {
                    self.check_member_raw(&current, target.span);
                    current = self.member_type(&current, field);
                }
                ir::Projection::Element(_) => {
                    if matches!(current.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
                        self.report_unsafe(UnsafeOp::PointerArithmetic, target.span);
                    }
                    current = Ty::unknown();
                }
            }
        }
    }


}

/// True when a projection chain selects an element with a run-time index.
fn has_runtime_element(projections: &[ir::Projection]) -> bool {
    projections
        .iter()
        .any(|projection| matches!(projection, ir::Projection::Element(None)))
}

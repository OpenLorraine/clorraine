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

    /// Resolves a written type in the current generic scope.
    pub(super) fn resolve(&self, ty: &crate::ast::Type) -> Ty {
        resolve_ty(ty, &self.type_params, &self.sema.table)
    }

    /// Checks and walks an expression, returning its type.
    pub(super) fn eval(&mut self, expr: &Expr) -> Ty {
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
            TyKind::Struct(name) => self
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
    pub(super) fn check_assign(&mut self, op: AssignOp, target: &Expr, value: &Expr) {
        let target_ty = self.ty(target);
        if op != AssignOp::Assign && matches!(target_ty.kind, TyKind::Raw(_)) {
            self.report_unsafe(UnsafeOp::PointerArithmetic, target.span);
        }
        self.check_place_raw(target);
        match self.place_target(target) {
            Some(PlaceTarget::Local(id, projections)) => {
                self.eval(value);
                if op != AssignOp::Assign {
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
                    self.emit_indexed_store(target, value);
                } else {
                    let place = self.target_place(&PlaceTarget::Local(id, projections.clone()));
                    self.emit_store(place, &target_ty, value);
                }
                if projections.is_empty() {
                    self.bind_value(id, &target_ty, value, target.span);
                }
            }
            Some(PlaceTarget::Global(name, projections)) => {
                let is_const = self
                    .program
                    .globals
                    .get(&name)
                    .is_some_and(|global| global.is_const);
                if is_const {
                    self.error(
                        "E10023",
                        format!("cannot assign to immutable '{name}'"),
                        "declare it with `let` (mutable)",
                        target.span,
                    );
                }
                self.eval(value);
                if self.is_loan_value(value) {
                    self.error(
                        "E10033",
                        format!("borrow of '{name}' escapes its scope"),
                        "derive the view from a parameter or take ownership",
                        target.span,
                    );
                }
                if !is_const {
                    if has_runtime_element(&projections) {
                        self.emit_indexed_store(target, value);
                    } else {
                        let place = self.target_place(&PlaceTarget::Global(name, projections));
                        self.emit_store(place, &target_ty, value);
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

    /// Lowers an assignment whose target has a run-time index.
    ///
    /// The target is lowered as the address of the selected element plus a
    /// store through it. The address keeps the index, so the emitter can emit
    /// the element's bounds check; a plain element projection would have lost
    /// the index and hardcoded element zero.
    fn emit_indexed_store(&mut self, target: &Expr, value: &Expr) {
        let target = match &target.kind {
            ExprKind::Unsafe(inner) => inner,
            _ => target,
        };
        let ExprKind::Index { base, index } = &target.kind else {
            return;
        };
        let element_ty = self.ty(target);
        let base_value = self.lower_value(base);
        let index_value = self.lower_value(index);
        let element = Value::new(
            element_ty.clone(),
            ValueKind::Index {
                base: Box::new(base_value),
                index: Box::new(index_value),
            },
        );
        let pointer = Value::new(
            element_ty,
            ValueKind::Unary {
                op: UnaryOp::AddressOf,
                operand: Box::new(element),
            },
        );
        let lowered = self.lower_value(value);
        self.emit(Statement::StorePointer {
            pointer,
            value: lowered,
        });
    }

    /// Reports raw operations implied by an assignment target's projections.
    ///
    /// Walking the projections rather than the whole expression keeps the
    /// target from being read as a value, which would be wrong for a field of
    /// a partially initialized or partially moved struct.
    pub(super) fn check_place_raw(&mut self, target: &Expr) {
        let Some(PlaceTarget::Local(id, projections)) = self.place_target(target) else {
            return;
        };
        let mut current = self.bindings[id].ty.clone();
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

    // -- calls ---------------------------------------------------------------

    /// Checks a call expression.
    pub(super) fn check_call_expr(&mut self, callee: &Expr, args: &[Arg], span: Span) {
        let callee_name = self.callee_name(callee);
        let Some(name) = callee_name else {
            self.eval(callee);
            for arg in args {
                self.eval(&arg.value);
            }
            return;
        };
        let Some(resolved) = self.program.callee(&name) else {
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
                self.check_marker_matches(&name, args, &params);
                self.check_arguments(args, &params, span);
            }
            Callee::Ffi(signature) => self.check_ffi_call(&name, signature, args, span),
            Callee::Stdlib { effect, .. } => self.check_stdlib_call(effect, args),
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
    /// already lent in this one call: lending the same place twice in one call
    /// is a conflicting loan even though two separate calls are fine.
    pub(super) fn check_arguments(&mut self, args: &[Arg], params: &[(Mode, Ty)], _span: Span) {
        let mut lent: BTreeSet<usize> = BTreeSet::new();
        for (index, arg) in args.iter().enumerate() {
            let expected = params.get(index).map(|(_, ty)| ty.clone());
            let arg_ty = self.eval(&arg.value);
            if let Some(expected) = &expected {
                self.check_value_into(&arg.value, expected, arg.span);
            }
            let mode = params.get(index).map(|(mode, _)| *mode).unwrap_or(Mode::None);
            self.check_argument_place(arg, mode, expected.as_ref(), &arg_ty, &mut lent);
        }
    }

    /// Applies the ownership effect of one argument.
    pub(super) fn check_argument_place(
        &mut self,
        arg: &Arg,
        mode: Mode,
        expected: Option<&Ty>,
        arg_ty: &Ty,
        lent: &mut BTreeSet<usize>,
    ) {
        let target = self.place_target(&arg.value);
        match mode {
            Mode::Take => {
                if !self.sema.table.duplicable(arg_ty) {
                    if let Some(target) = target {
                        self.move_target(target, arg.span);
                    }
                }
            }
            Mode::Borrow => {
                if let Some(PlaceTarget::Global(name, _)) = &target {
                    if !self.global_is_const(name) {
                        self.error(
                            "E10038",
                            format!("cannot borrow global '{name}'"),
                            "borrow from a local owner",
                            arg.span,
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
                        arg.span,
                    );
                }
                // The owner may be a direct place or a view derived from a
                // call such as `slice(buf, ...)`; both lend the base storage.
                let owner = match &target {
                    Some(PlaceTarget::Local(id, _)) => Some(*id),
                    _ => self.argument_owner(&arg.value),
                };
                if let Some(owner) = owner {
                    if expected.is_some_and(is_view) && self.is_readonly_borrow(owner) {
                        let name = self.bindings[owner].name.clone();
                        self.error(
                            "E10041",
                            format!("cannot write through a view of read-only borrow '{name}'"),
                            "take the owner by `borrow`",
                            arg.span,
                        );
                        return;
                    }
                    if !self.sema.table.duplicable(arg_ty) {
                        let duplicate = !lent.insert(owner);
                        if duplicate || self.state.loans.conflict(owner, LoanKind::Exclusive) {
                            let name = self.bindings[owner].name.clone();
                            self.error(
                                "E10032",
                                format!("'{name}' is already borrowed"),
                                "end the first borrow first",
                                arg.span,
                            );
                        }
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
                if let Some(PlaceTarget::Global(name, _)) = &target {
                    if !self.global_is_const(name) {
                        self.error(
                            "E10038",
                            format!("cannot borrow global '{name}'"),
                            "borrow from a local owner",
                            arg.span,
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
    pub(super) fn global_is_const(&self, name: &str) -> bool {
        self.program
            .globals
            .get(name)
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
        args: &[Arg],
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
            self.check_argument_place(arg, *mode, Some(expected), &arg_ty, &mut lent);
        }
    }

    /// Applies the ownership effect of a standard-library helper.
    pub(super) fn check_stdlib_call(&mut self, effect: StdlibEffect, args: &[Arg]) {
        if let StdlibEffect::ViewOf { arg, .. } = effect {
            for (index, written) in args.iter().enumerate() {
                self.eval(&written.value);
                if index == arg {
                    if let Some(PlaceTarget::Global(name, _)) = self.place_target(&written.value) {
                        if !self.global_is_const(&name) {
                            self.error(
                                "E10038",
                                format!("cannot borrow global '{name}'"),
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

/// True when a projection chain selects an element with a run-time index.
fn has_runtime_element(projections: &[ir::Projection]) -> bool {
    projections
        .iter()
        .any(|projection| matches!(projection, ir::Projection::Element(None)))
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

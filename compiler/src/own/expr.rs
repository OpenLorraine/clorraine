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

    /// The recorded type of an expression, or the opaque type.
    pub(super) fn ty(&self, expr: &Expr) -> Ty {
        self.sema
            .expr_types
            .get(&(expr as *const Expr as usize))
            .cloned()
            .unwrap_or_else(Ty::unknown)
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
                    self.read_place(id, expr.span);
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

    /// Walks a dotted name, checking reads and implied raw dereferences.
    pub(super) fn eval_qualified(&mut self, name: &crate::ast::QualifiedName, span: Span) {
        let Some(first) = name.segments.first() else {
            return;
        };
        if let Some(id) = self.lookup(first) {
            self.read_place(id, span);
            if name.segments.len() >= 2 {
                let base_ty = self.bindings[id].ty.clone();
                self.check_member_raw(&base_ty, span);
            }
        } else if let Some(global) = self.program.globals.get(first) {
            let _ = global;
        }
    }

    /// Reports `E10600` when a member is reached through a raw pointer.
    pub(super) fn check_member_raw(&mut self, base_ty: &Ty, span: Span) {
        if matches!(base_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
            self.report_unsafe(UnsafeOp::Deref, span);
        }
    }

    /// Reports a nullable place used where a non-null value is required.
    pub(super) fn check_nonnull_expr(&mut self, expr: &Expr, span: Span) {
        if let Some((id, projections)) = self.plain_binding(expr) {
            if projections.is_empty() && !self.state.place(id).narrowed {
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
        match self.place_target(target) {
            Some(PlaceTarget::Local(id, projections)) => {
                self.eval(value);
                if op != AssignOp::Assign {
                    self.read_place(id, target.span);
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
                let whole = projections.is_empty();
                self.write_place(id, whole, target.span);
                if whole {
                    let place = self.binding_place(id);
                    let lowered = self.lower_value(value);
                    self.emit(Statement::Assign {
                        target: place,
                        value: lowered,
                    });
                    self.bind_value(id, &target_ty, value, target.span);
                }
            }
            Some(PlaceTarget::Global(name, _projections)) => {
                if self
                    .program
                    .globals
                    .get(&name)
                    .is_some_and(|global| global.is_const)
                {
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
            }
            None => {
                self.eval(target);
                self.eval(value);
            }
        }
        self.check_value_into(value, &target_ty, value.span);
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
                self.emit(Statement::Call {
                    callee: name,
                    args: args.iter().map(|arg| self.lower_value(&arg.value)).collect(),
                    result: None,
                });
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
    pub(super) fn check_arguments(&mut self, args: &[Arg], params: &[(Mode, Ty)], _span: Span) {
        for (index, arg) in args.iter().enumerate() {
            let expected = params.get(index).map(|(_, ty)| ty.clone());
            let arg_ty = self.eval(&arg.value);
            if let Some(expected) = &expected {
                self.check_value_into(&arg.value, expected, arg.span);
            }
            let mode = params.get(index).map(|(mode, _)| *mode).unwrap_or(Mode::None);
            self.check_argument_place(arg, mode, expected.as_ref(), &arg_ty);
        }
    }

    /// Applies the ownership effect of one argument.
    pub(super) fn check_argument_place(
        &mut self,
        arg: &Arg,
        mode: Mode,
        expected: Option<&Ty>,
        arg_ty: &Ty,
    ) {
        let Some(target) = self.place_target(&arg.value) else {
            return;
        };
        match mode {
            Mode::Take => {
                if !self.sema.table.duplicable(arg_ty) {
                    self.move_target(target, arg.span);
                }
            }
            Mode::Borrow => {
                if let PlaceTarget::Global(name, _) = &target {
                    self.error(
                        "E10038",
                        format!("cannot borrow global '{name}'"),
                        "borrow from a local owner",
                        arg.span,
                    );
                    return;
                }
                if self.sema.table.duplicable(arg_ty) && !is_view(arg_ty) {
                    // A writable lend of a duplicable value binds a copy, so
                    // the write reaches the copy only.
                    let name = match &target {
                        PlaceTarget::Local(id, _) => self.bindings[*id].name.clone(),
                        PlaceTarget::Global(name, _) => name.clone(),
                    };
                    self.diagnostics.push(Diagnostic::warning(
                        "W10001",
                        format!("unnecessary lend of duplicable '{name}'"),
                        arg.span,
                    ));
                }
                if let PlaceTarget::Local(id, _) = target {
                    if expected.is_some_and(is_view) && self.is_readonly_borrow(id) {
                        let name = self.bindings[id].name.clone();
                        self.error(
                            "E10041",
                            format!("cannot write through a view of read-only borrow '{name}'"),
                            "take the owner by `borrow`",
                            arg.span,
                        );
                        return;
                    }
                    if !self.sema.table.duplicable(arg_ty)
                        && self.state.loans.conflict(id, LoanKind::Exclusive)
                    {
                        let name = self.bindings[id].name.clone();
                        self.error(
                            "E10032",
                            format!("'{name}' is already borrowed"),
                            "end the first borrow first",
                            arg.span,
                        );
                    }
                }
            }
            Mode::None | Mode::Const => {
                if let PlaceTarget::Global(name, _) = &target {
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

    /// True when the binding is a read-only borrow of view storage.
    pub(super) fn is_readonly_borrow(&self, id: usize) -> bool {
        let binding = &self.bindings[id];
        if binding.kind == BindingKind::Param {
            return matches!(binding.mode, Mode::None | Mode::Const);
        }
        matches!(&binding.provenance, Provenance::Param(owner) if self.bindings[*owner].mode != Mode::Borrow)
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
        for (index, arg) in args.iter().enumerate() {
            let arg_ty = self.eval(&arg.value);
            let Some((mode, expected)) = params.get(index) else {
                continue;
            };
            if ffi::is_uncontracted_param(*mode, expected) {
                if is_handle_or_aggregate(&arg_ty)
                    && !arg_ty.nullable
                    && !matches!(arg.value.kind, ExprKind::Null)
                {
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
            self.check_argument_place(arg, *mode, Some(expected), &arg_ty);
        }
    }

    /// Applies the ownership effect of a standard-library helper.
    pub(super) fn check_stdlib_call(&mut self, effect: StdlibEffect, args: &[Arg]) {
        if let StdlibEffect::ViewOf { arg, .. } = effect {
            for (index, written) in args.iter().enumerate() {
                self.eval(&written.value);
                if index == arg {
                    if let Some(PlaceTarget::Global(name, _)) = self.place_target(&written.value) {
                        self.error(
                            "E10038",
                            format!("cannot borrow global '{name}'"),
                            "borrow from a local owner",
                            written.span,
                        );
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

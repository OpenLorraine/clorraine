//! Operator, assignment, and conditional checking.
//!
//! Binary and unary operators unify their operands, `bool` conditions are
//! enforced, assignment targets must be places, and the conditional operator
//! types its branches. The split from plain expression checking keeps both
//! files focused: this one owns the operator table, the other owns names,
//! calls, literals, and aggregates.

use crate::ast::{AssignOp, BinaryOp, Expr, ExprKind, ScalarType, UnaryOp};
use crate::borrow::is_view;
use crate::span::Span;
use crate::types::{
    contains_param, int_fits, int_literal_value, is_integer_scalar, is_null_shaped, unify_numeric,
    Ty, TyKind,
};

use super::Checker;

impl Checker<'_, '_> {
    /// Types a unary operator application.
    ///
    /// A negated integer literal is folded before its context is consulted, so
    /// `-5` takes the destination's integer type when the folded value fits,
    /// exactly as the positive literal does. The operand is never context-typed:
    /// the unary minus produces its own type, and only the folded value's fit
    /// decides acceptance. Any other shape keeps the ordinary unary rules.
    pub(super) fn check_unary(
        &mut self,
        expr: &Expr,
        op: UnaryOp,
        operand: &Expr,
        expected: Option<&Ty>,
    ) -> Ty {
        if op == UnaryOp::Neg {
            if let Some(value) = int_literal_value(expr) {
                if let Some(scalar) = expected.and_then(Self::integer_context) {
                    if !int_fits(value, scalar) {
                        self.error(
                            "E10202",
                            format!("literal does not fit '{}'", Ty::scalar(scalar).display()),
                            "use a wider type or a hex/binary form",
                            expr.span,
                        );
                    }
                    let ty = self.check_value(operand, None);
                    self.require_numeric(&ty, "-", operand.span);
                    return Ty::scalar(scalar);
                }
            }
        }
        let ty = self.check_value(operand, None);
        match op {
            UnaryOp::Not => {
                self.require_bool(&ty, operand.span);
                Ty::boolean()
            }
            UnaryOp::BitNot => {
                self.require_integer(&ty, "~", operand.span);
                ty
            }
            UnaryOp::Neg => {
                self.require_numeric(&ty, "-", operand.span);
                ty
            }
            UnaryOp::AddressOf => Ty {
                kind: TyKind::Raw(Box::new(ty)),
                nullable: false,
            },
            UnaryOp::Deref => match ty.kind {
                TyKind::Raw(inner) => *inner,
                _ => Ty::unknown(),
            },
            UnaryOp::PreIncrement
            | UnaryOp::PreDecrement
            | UnaryOp::PostIncrement
            | UnaryOp::PostDecrement => {
                self.require_numeric(&ty, unary_spelling(op), operand.span);
                // `x++` is `x = x + 1`, so its operand must be a place just as
                // an assignment's left side must be.
                if !self.is_place(operand) {
                    self.error(
                        "E10221",
                        "left side of an assignment must be a place",
                        "increment a variable, field, or element",
                        operand.span,
                    );
                }
                // An increment writes its operand, so the same capture rule
                // applies as for an assignment.
                self.reject_capture_write(operand);
                ty
            }
        }
    }

    /// Types a binary operator application.
    pub(super) fn check_binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Ty {
        let left_ty = self.check_value(left, None);
        let right_ty = self.check_value(right, None);
        let (left_ty, right_ty) = self.adapt_operands(left, left_ty, right, right_ty);
        // A conditional operand with a null arm is judged against the other
        // operand's non-nullable base type. The nullable flag a literal
        // inherits from the other side is stripped: the arithmetic result
        // cannot carry a null arm.
        let mut left_expected = right_ty.clone();
        left_expected.nullable = false;
        self.judge_conditional_argument(left, &left_expected);
        let mut right_expected = left_ty.clone();
        right_expected.nullable = false;
        self.judge_conditional_argument(right, &right_expected);
        self.binary_result(op, &left_ty, left.span, &right_ty, right.span)
    }

    /// Gives an integer literal the type of the other operand when it fits.
    fn adapt_operands(
        &mut self,
        left: &Expr,
        left_ty: Ty,
        right: &Expr,
        right_ty: Ty,
    ) -> (Ty, Ty) {
        let mut left_ty = left_ty;
        let mut right_ty = right_ty;
        if let Some(adapted) = self.adapt_literal(left, &right_ty) {
            left_ty = adapted;
        }
        if let Some(adapted) = self.adapt_literal(right, &left_ty) {
            right_ty = adapted;
        }
        (left_ty, right_ty)
    }

    /// Gives an integer literal the type of `target` when the target is an
    /// integer, reporting a literal whose value does not fit.
    fn adapt_literal(&mut self, expr: &Expr, target: &Ty) -> Option<Ty> {
        let value = int_literal_value(expr)?;
        if target.is_integer() {
            Some(self.literal_type(expr.span, value, target))
        } else {
            None
        }
    }

    /// The integer scalar a destination context supplies, if any.
    ///
    /// Only an integer scalar supplies a literal context. A float, `bool`, or
    /// non-scalar destination leaves the literal at its default type so the
    /// ordinary conversion reports the mismatch.
    fn integer_context(expected: &Ty) -> Option<ScalarType> {
        match expected.kind {
            TyKind::Scalar(scalar) if is_integer_scalar(scalar) => Some(scalar),
            _ => None,
        }
    }

    /// Types an integer literal for an integer target, reporting a literal
    /// that does not fit.
    fn literal_type(&mut self, span: Span, value: i128, target: &Ty) -> Ty {
        if let TyKind::Scalar(scalar) = target.kind {
            if !int_fits(value, scalar) {
                self.error(
                    "E10202",
                    format!("literal does not fit '{}'", target.display()),
                    "use a wider type or a hex/binary form",
                    span,
                );
            }
        }
        target.clone()
    }

    /// Computes the result of a binary operator from its operand types.
    fn binary_result(
        &mut self,
        op: BinaryOp,
        left_ty: &Ty,
        left_span: Span,
        right_ty: &Ty,
        right_span: Span,
    ) -> Ty {
        match op {
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                self.require_bool(left_ty, left_span);
                self.require_bool(right_ty, right_span);
                Ty::boolean()
            }
            BinaryOp::Eq | BinaryOp::Ne => {
                // A view is nullable by definition, so comparing it with the
                // `null` literal is its emptiness test: the emitter lowers the
                // pairing to a pointer test and the ownership walk narrows the
                // place. Every other pairing keeps the operand rules, so two
                // slice views still have no defined equality, a view never
                // pairs with a non-null operand, and an untyped operand is
                // not a null literal.
                let null_on_either_side = null_operand(left_ty) || null_operand(right_ty);
                let view_on_either_side = is_view(left_ty) || is_view(right_ty);
                if !(null_on_either_side && view_on_either_side) {
                    if !comparable(left_ty) {
                        self.report_operator(op, left_ty, left_span);
                    } else if !comparable(right_ty) {
                        self.report_operator(op, right_ty, right_span);
                    }
                }
                Ty::boolean()
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                if !orderable(left_ty) {
                    self.report_operator(op, left_ty, left_span);
                } else if !orderable(right_ty) {
                    self.report_operator(op, right_ty, right_span);
                } else if left_ty.is_numeric() && right_ty.is_numeric() {
                    // Two numeric operands must unify; `bool` operands are
                    // orderable but are not numeric, so a comparison of a
                    // boolean result with a number is left alone.
                    if unify_numeric(left_ty, right_ty).is_none() {
                        self.error(
                            "E10200",
                            format!(
                                "cannot convert '{}' to '{}' implicitly",
                                left_ty.display(),
                                right_ty.display()
                            ),
                            "use an explicit cast `T(expr)`",
                            left_span,
                        );
                    }
                }
                Ty::boolean()
            }
            BinaryOp::Add | BinaryOp::Sub => {
                if let Some(result) = pointer_arithmetic(op, left_ty, right_ty) {
                    result
                } else {
                    self.arithmetic(op, left_ty, left_span, right_ty, right_span)
                }
            }
            BinaryOp::Mul | BinaryOp::Div => {
                self.arithmetic(op, left_ty, left_span, right_ty, right_span)
            }
            BinaryOp::Rem => {
                if self.require_integer(left_ty, "%", left_span) {
                    self.require_integer(right_ty, "%", right_span);
                }
                self.integer_result(left_ty, left_span, right_ty, right_span)
            }
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                let spelling = binary_spelling(op);
                if self.require_integer(left_ty, spelling, left_span) {
                    self.require_integer(right_ty, spelling, right_span);
                }
                self.integer_result(left_ty, left_span, right_ty, right_span)
            }
            BinaryOp::Shl | BinaryOp::Shr => {
                let spelling = binary_spelling(op);
                if self.require_integer(left_ty, spelling, left_span) {
                    self.require_integer(right_ty, spelling, right_span);
                }
                left_ty.clone()
            }
        }
    }

    /// Types `+ - * /`, which accept integers and floats.
    fn arithmetic(
        &mut self,
        op: BinaryOp,
        left_ty: &Ty,
        left_span: Span,
        right_ty: &Ty,
        right_span: Span,
    ) -> Ty {
        if left_ty.is_unknown() || left_ty.is_param() {
            return right_ty.clone();
        }
        if right_ty.is_unknown() || right_ty.is_param() {
            return left_ty.clone();
        }
        if left_ty.is_numeric() && right_ty.is_numeric() {
            if let Some(unified) = unify_numeric(left_ty, right_ty) {
                return unified;
            }
            self.error(
                "E10200",
                format!(
                    "cannot convert '{}' to '{}' implicitly",
                    left_ty.display(),
                    right_ty.display()
                ),
                "use an explicit cast `T(expr)`",
                left_span,
            );
            return left_ty.clone();
        }
        if !left_ty.is_numeric() {
            self.report_operator(op, left_ty, left_span);
        } else {
            self.report_operator(op, right_ty, right_span);
        }
        Ty::unknown()
    }

    /// Unifies two operands already known to be integers.
    fn integer_result(&mut self, left_ty: &Ty, left_span: Span, right_ty: &Ty, _right_span: Span) -> Ty {
        if left_ty.is_unknown() || left_ty.is_param() {
            return right_ty.clone();
        }
        if right_ty.is_unknown() || right_ty.is_param() {
            return left_ty.clone();
        }
        if let Some(unified) = unify_numeric(left_ty, right_ty) {
            return unified;
        }
        self.error(
            "E10200",
            format!(
                "cannot convert '{}' to '{}' implicitly",
                left_ty.display(),
                right_ty.display()
            ),
            "use an explicit cast `T(expr)`",
            left_span,
        );
        left_ty.clone()
    }

    /// Reports that the operator is not defined for a type.
    fn report_operator(&mut self, op: BinaryOp, ty: &Ty, span: Span) {
        self.error(
            "E10222",
            format!(
                "operator '{}' is not defined for '{}'",
                binary_spelling(op),
                ty.display()
            ),
            "convert explicitly or use a supported operator",
            span,
        );
    }

    /// Requires an integer operand, reporting `E10222` otherwise.
    ///
    /// Returns false when it reported, so a caller can avoid a second report
    /// for the same operator on a pair of operands that share the cause.
    fn require_integer(&mut self, ty: &Ty, op: &str, span: Span) -> bool {
        if ty.is_integer() || ty.is_unknown() || ty.is_param() {
            return true;
        }
        self.error(
            "E10222",
            format!("operator '{op}' is not defined for '{}'", ty.display()),
            "convert explicitly or use a supported operator",
            span,
        );
        false
    }

    /// Requires an integer or float operand, reporting `E10222` otherwise.
    fn require_numeric(&mut self, ty: &Ty, op: &str, span: Span) -> bool {
        if ty.is_numeric() || ty.is_unknown() || ty.is_param() {
            return true;
        }
        self.error(
            "E10222",
            format!("operator '{op}' is not defined for '{}'", ty.display()),
            "convert explicitly or use a supported operator",
            span,
        );
        false
    }

    /// Types `cond ? a : b`, judging each arm against the destination when
    /// one is known.
    ///
    /// The expected type reaches both arm checks so a literal takes the
    /// destination's type, and a null arm is judged against the destination
    /// before unification would swallow the null shape. An arm type that still
    /// names a type parameter keeps the declaration's acceptance: the
    /// per-instance audit re-applies this rule to the substituted arms.
    pub(super) fn check_conditional(
        &mut self,
        condition: &Expr,
        then_value: &Expr,
        else_value: &Expr,
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        let condition_ty = self.check_value(condition, None);
        self.require_bool(&condition_ty, condition.span);
        let then_ty = self.check_value(then_value, expected);
        let else_ty = self.check_value(else_value, expected);
        let null_reported = if let Some(expected) = expected {
            self.report_null_conditional_arm(&then_ty, expected, then_value.span)
                | self.report_null_conditional_arm(&else_ty, expected, else_value.span)
        } else {
            false
        };
        // The arm types are still open at the declaration, so the instance
        // decides whether they agree and whether a null arm reaches a
        // non-nullable destination. The site records its arms and the
        // destination it has so far; a generic call attaches its resolved
        // parameter later, and the module finish turns every open site into
        // the deferred duty.
        let key = then_value as *const Expr as usize;
        // A null arm whose destination is still open is judged at the
        // instance too: the substituted destination decides whether the null
        // is legal. An arm whose type stays unknown at the declaration can be
        // an opaque member read whose concrete type decides the arm rule, so
        // the instance resolves it before judging.
        let open_destination = expected.is_some_and(contains_param);
        let null_arm = is_null_shaped(&then_ty) || is_null_shaped(&else_ty);
        // A nullable-typed arm at a destination that still names a type
        // parameter is judged by the instance too: the substituted
        // destination decides whether the arm's possible null is legal.
        let nullable_arm = then_ty.nullable || else_ty.nullable;
        let unresolved_arm = (then_ty.is_unknown() && !is_null_shaped(&then_ty))
            || (else_ty.is_unknown() && !is_null_shaped(&else_ty));
        let arms = super::ConditionalArms {
            function: self.function_name.clone(),
            then_ty: then_ty.clone(),
            else_ty: else_ty.clone(),
            then_span: then_value.span,
            else_span: else_value.span,
            then_member: super::expr::member_read_name(then_value),
            else_member: super::expr::member_read_name(else_value),
            then_base: super::expr::plain_binding_root(then_value),
            else_base: super::expr::plain_binding_root(else_value),
            span,
            destination: expected.cloned(),
            opaque: contains_param(&then_ty)
                || contains_param(&else_ty)
                || unresolved_arm
                || ((null_arm || nullable_arm) && open_destination),
            null_reported,
        };
        match self.conditional_index.get(&key).copied() {
            Some(index) => self.conditional_arms[index] = arms,
            None => {
                self.conditional_index.insert(key, self.conditional_arms.len());
                self.conditional_arms.push(arms);
            }
        }
        if let Some(result) = crate::types::conditional_result(&then_ty, &else_ty) {
            return result;
        }
        // Two distinct capturing closures have different environment types, so
        // no single callable value can carry either arm. The conditional is an
        // ephemeral closure, not a conversion between two matching closure
        // types.
        if then_ty.is_closure() && else_ty.is_closure() {
            self.error(
                "E10404",
                "closures are ephemeral and cannot be stored, returned, or captured",
                "pass the lambda directly",
                span,
            );
            return Ty::unknown();
        }
        self.error(
            "E10200",
            format!(
                "cannot convert '{}' to '{}' implicitly",
                else_ty.display(),
                then_ty.display()
            ),
            "use an explicit cast `T(expr)`",
            else_value.span,
        );
        then_ty
    }

    /// Reports the null rule for one conditional arm at a known destination.
    ///
    /// A destination that is nullable or still open accepts the null arm. A
    /// concrete non-nullable destination reports at the arm, exactly where the
    /// null is written.
    pub(super) fn report_null_conditional_arm(
        &mut self,
        arm_ty: &Ty,
        expected: &Ty,
        span: Span,
    ) -> bool {
        if crate::types::null_arm_needs_report(arm_ty, expected) {
            self.error(
                "E10012",
                "'null' is not allowed here",
                "declare the type `T?` or avoid the empty state",
                span,
            );
            return true;
        }
        false
    }

    /// Attaches a conditional's destination and reports whether the site is
    /// deferred to its instance.
    ///
    /// A conditional argument to a generic call is typed before the type
    /// arguments are inferred, so its destination is known only afterwards.
    /// The site keeps the destination for the instance audit and the ordinary
    /// deferred conversion is skipped: the conditional's own duty judges both
    /// the arm agreement and the resolved result against it, so one site
    /// reports once exactly as its concrete twin does.
    pub(super) fn note_conditional_destination(&mut self, expr: &Expr, expected: &Ty) -> bool {
        let ExprKind::Conditional { then_value, .. } = &expr.kind else {
            return false;
        };
        let key = &**then_value as *const Expr as usize;
        let Some(index) = self.conditional_index.get(&key).copied() else {
            return false;
        };
        let arms = &mut self.conditional_arms[index];
        // A null arm at a still-open destination is judged by the instance:
        // the substituted destination decides whether the null is legal.
        let open_destination = contains_param(expected);
        let null_arm = is_null_shaped(&arms.then_ty) || is_null_shaped(&arms.else_ty);
        let nullable_arm = arms.then_ty.nullable || arms.else_ty.nullable;
        if open_destination && (null_arm || nullable_arm) {
            arms.opaque = true;
        }
        arms.destination = Some(expected.clone());
        arms.opaque
    }

    /// Applies a resolved destination to a conditional's null arms.
    ///
    /// A generic call types its arguments before its type arguments are
    /// inferred, so a conditional argument's null arm has no destination then.
    /// The recorded arm types let the call site apply the null rule once the
    /// parameter resolves, without walking the expression a second time. The
    /// same helper serves the `if` condition, a cast, and a binary operand:
    /// each is a non-nullable context that the conditional's result feeds.
    pub(super) fn judge_conditional_argument(&mut self, expr: &Expr, expected: &Ty) {
        let ExprKind::Conditional { then_value, .. } = &expr.kind else {
            return;
        };
        let key = &**then_value as *const Expr as usize;
        let Some(index) = self.conditional_index.get(&key).copied() else {
            return;
        };
        let arms = &self.conditional_arms[index];
        // A site whose arms still name a type parameter is judged by the
        // instance audit, which substitutes the same destination.
        if arms.opaque {
            return;
        }
        let (then_ty, else_ty, then_span, else_span) =
            (arms.then_ty.clone(), arms.else_ty.clone(), arms.then_span, arms.else_span);
        self.report_null_conditional_arm(&then_ty, expected, then_span);
        self.report_null_conditional_arm(&else_ty, expected, else_span);
    }

    /// Types an assignment, which has a `void` result.
    pub(super) fn check_assign(&mut self, op: AssignOp, target: &Expr, value: &Expr) -> Ty {
        if !self.is_place(target) {
            self.error(
                "E10221",
                "left side of an assignment must be a place",
                "assign to a variable, field, or element",
                target.span,
            );
        }
        // Writing a captured binding needs a `[borrow x]` capture; a copy or a
        // read-only borrow cannot be written.
        self.reject_capture_write(target);
        let target_ty = self.check_value(target, None);
        match op {
            AssignOp::Assign => {
                self.check_into_slot(value, &target_ty);
                // A capture moved through the store is recorded with the
                // destination. A borrowed capture is recorded only at a
                // writable lend destination — the out-slot class, where the
                // store writes through the caller's storage; a local, a
                // member, or a plain view destination keeps the frozen
                // lend-store report.
                let lends = match &target.kind {
                    ExprKind::Name(name) => self.binding_lends(name),
                    _ => false,
                };
                self.record_store_capture_move(value, &target_ty, lends);
            }
            _ => {
                let value_ty = self.check_value(value, None);
                // A compound assignment is `x = x op y`, so a literal right
                // operand takes the target's type exactly as it would in the
                // equivalent binary operator.
                let value_ty = self.adapt_literal(value, &target_ty).unwrap_or(value_ty);
                if let Some(binary) = op.binary_op() {
                    // `x op= y` is `x = x op y`, so the operator result must
                    // convert back to the target exactly as the equivalent
                    // assignment would.
                    let result =
                        self.binary_result(binary, &target_ty, target.span, &value_ty, value.span);
                    self.check_conversion(&result, &target_ty, value.span);
                }
            }
        }
        Ty::void()
    }

    /// True when the expression may stand on the left of an assignment.
    pub(super) fn is_place(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Name(_) => true,
            // An element store needs a place base exactly as a member store
            // does. The walk recurses into the index's base, so an indexing
            // step whose base chain carries no member between a conditional
            // and the index reports the place rule rather than silently
            // dropping the store; a member step above the index is judged by
            // the member arm.
            ExprKind::Index { base, .. } => self.is_place(base),
            // The parser spells `value.field` as a dotted name, so a dotted name
            // that starts with a value in scope is a place just like a field.
            ExprKind::Qualified(name) => {
                name.segments.len() >= 2 && self.name_is_value(&name.segments[0])
            }
            ExprKind::Unary {
                op: UnaryOp::Deref, ..
            } => true,
            ExprKind::Member { base, .. } => self.is_place(base),
            // The unsafe wrapper is transparent to the place it wraps: a field
            // or element write through a raw pointer is still a place, while a
            // selection it wraps is not.
            ExprKind::Unsafe(inner) => self.is_place(inner),
            _ => false,
        }
    }
}

/// Types pointer arithmetic when an operand is a raw pointer.
///
/// `raw T + n`, `n + raw T`, and `raw T - n` all yield `raw T`. The operation
/// itself is legal only inside `unsafe`; the type layer accepts it so the
/// unsafe-context check owns the diagnostic instead of reporting an undefined
/// operator. Any other shape, including `raw - raw`, is left to the ordinary
/// arithmetic rules.
fn pointer_arithmetic(op: BinaryOp, left_ty: &Ty, right_ty: &Ty) -> Option<Ty> {
    let left_pointer = matches!(left_ty.kind, TyKind::Raw(_));
    let right_pointer = matches!(right_ty.kind, TyKind::Raw(_));
    match op {
        BinaryOp::Add if left_pointer && offset_operand(right_ty) => Some(left_ty.clone()),
        BinaryOp::Add if right_pointer && offset_operand(left_ty) => Some(right_ty.clone()),
        BinaryOp::Sub if left_pointer && offset_operand(right_ty) => Some(left_ty.clone()),
        _ => None,
    }
}

/// True for an operand that advances a raw pointer: an integer, an opaque
/// type, or a generic parameter.
fn offset_operand(ty: &Ty) -> bool {
    ty.is_integer() || ty.is_unknown() || ty.is_param()
}

/// True for a type that supports ordering with `<`, `<=`, `>`, `>=`.
///
/// Ordering is defined for the numeric scalars only. `bool` is a scalar for
/// storage but is not a number and has no ordering, so it is excluded.
fn orderable(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Scalar(scalar) => *scalar != ScalarType::Bool,
        TyKind::Unknown | TyKind::Param(_) => true,
        _ => false,
    }
}

/// True for a type that supports `==` and `!=`.
fn comparable(ty: &Ty) -> bool {
    matches!(
        ty.kind,
        TyKind::Scalar(_) | TyKind::Enum(_) | TyKind::Raw(_) | TyKind::Handle(_) | TyKind::CString
    ) || ty.is_unknown()
        || ty.is_param()
}

/// True for a nullable unknown type: the `null` literal's shape.
///
/// The null literal types as a nullable unknown, while an untyped local or an
/// unresolved operand is unknown *without* the flag. Only a nullable unknown
/// may pair with a view, so an untyped operand keeps the operand rules and
/// reports on the view side instead of emitting a comparison C cannot
/// represent. A binding initialized from `null` shares the literal's type and
/// therefore pairs too; making that binding's own type and emission exact is
/// separate work.
fn null_operand(ty: &Ty) -> bool {
    ty.is_unknown() && ty.nullable
}

/// The source spelling of a binary operator.
fn binary_spelling(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::LogicalOr => "||",
        BinaryOp::LogicalAnd => "&&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::BitAnd => "&",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
    }
}

/// The source spelling of a unary operator.
fn unary_spelling(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
        UnaryOp::Neg => "-",
        UnaryOp::AddressOf => "&",
        UnaryOp::Deref => "*",
        UnaryOp::PreIncrement | UnaryOp::PostIncrement => "++",
        UnaryOp::PreDecrement | UnaryOp::PostDecrement => "--",
    }
}


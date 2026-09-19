//! Operator, assignment, and conditional checking.
//!
//! Binary and unary operators unify their operands, `bool` conditions are
//! enforced, assignment targets must be places, and the conditional operator
//! types its branches. The split from plain expression checking keeps both
//! files focused: this one owns the operator table, the other owns names,
//! calls, literals, and aggregates.

use crate::ast::{AssignOp, BinaryOp, Expr, ExprKind, ScalarType, UnaryOp};
use crate::span::Span;
use crate::types::{implicit_conversion, int_fits, int_literal_value, unify_numeric, Ty, TyKind};

use super::Checker;

impl Checker<'_, '_> {
    /// Types a unary operator application.
    pub(super) fn check_unary(&mut self, op: UnaryOp, operand: &Expr) -> Ty {
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
                ty
            }
        }
    }

    /// Types a binary operator application.
    pub(super) fn check_binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Ty {
        let left_ty = self.check_value(left, None);
        let right_ty = self.check_value(right, None);
        let (left_ty, right_ty) = self.adapt_operands(left, left_ty, right, right_ty);
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
        if let Some(value) = int_literal_value(left) {
            if right_ty.is_integer() {
                left_ty = self.literal_type(left.span, value, &right_ty);
            }
        }
        if let Some(value) = int_literal_value(right) {
            if left_ty.is_integer() {
                right_ty = self.literal_type(right.span, value, &left_ty);
            }
        }
        (left_ty, right_ty)
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
                if !comparable(left_ty) {
                    self.report_operator(op, left_ty, left_span);
                } else if !comparable(right_ty) {
                    self.report_operator(op, right_ty, right_span);
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
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
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

    /// Types `cond ? a : b`.
    pub(super) fn check_conditional(
        &mut self,
        condition: &Expr,
        then_value: &Expr,
        else_value: &Expr,
    ) -> Ty {
        let condition_ty = self.check_value(condition, None);
        self.require_bool(&condition_ty, condition.span);
        let then_ty = self.check_value(then_value, None);
        let else_ty = self.check_value(else_value, None);
        if then_ty.kind == else_ty.kind {
            return then_ty;
        }
        if let Some(unified) = unify_numeric(&then_ty, &else_ty) {
            return unified;
        }
        if implicit_conversion(&else_ty, &then_ty) {
            return then_ty;
        }
        if implicit_conversion(&then_ty, &else_ty) {
            return else_ty;
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
        let target_ty = self.check_value(target, None);
        match op {
            AssignOp::Assign => {
                self.check_into(value, &target_ty);
            }
            _ => {
                let value_ty = self.check_value(value, None);
                if let Some(binary) = assign_binary(op) {
                    self.binary_result(binary, &target_ty, target.span, &value_ty, value.span);
                }
            }
        }
        Ty::void()
    }

    /// True when the expression may stand on the left of an assignment.
    fn is_place(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Name(_) | ExprKind::Index { .. } => true,
            ExprKind::Unary {
                op: UnaryOp::Deref, ..
            } => true,
            ExprKind::Member { base, .. } => matches!(
                base.kind,
                ExprKind::Name(_)
                    | ExprKind::Member { .. }
                    | ExprKind::Index { .. }
                    | ExprKind::Unary {
                        op: UnaryOp::Deref,
                        ..
                    }
                    | ExprKind::Unsafe(_)
            ),
            _ => false,
        }
    }
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
        TyKind::Scalar(_)
            | TyKind::Enum(_)
            | TyKind::Raw(_)
            | TyKind::Handle(_)
            | TyKind::Str
            | TyKind::CString
            | TyKind::StaticStr
    ) || ty.is_unknown()
        || ty.is_param()
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

/// The binary operator a compound assignment applies.
fn assign_binary(op: AssignOp) -> Option<BinaryOp> {
    match op {
        AssignOp::Assign => None,
        AssignOp::Add => Some(BinaryOp::Add),
        AssignOp::Sub => Some(BinaryOp::Sub),
        AssignOp::Mul => Some(BinaryOp::Mul),
        AssignOp::Div => Some(BinaryOp::Div),
        AssignOp::Rem => Some(BinaryOp::Rem),
        AssignOp::BitAnd => Some(BinaryOp::BitAnd),
        AssignOp::BitOr => Some(BinaryOp::BitOr),
        AssignOp::BitXor => Some(BinaryOp::BitXor),
        AssignOp::Shl => Some(BinaryOp::Shl),
        AssignOp::Shr => Some(BinaryOp::Shr),
    }
}

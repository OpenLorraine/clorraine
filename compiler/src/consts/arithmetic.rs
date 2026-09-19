//! Integer and floating-point folding primitives.
//!
//! These helpers evaluate one operator over already-folded operands. The
//! integer helpers wrap, mask a shift count, and fault exactly where the
//! runtime operator does, using the type the operation is evaluated at. A
//! `FoldValue` carries that type next to the value so a caller can keep
//! folding an expression at its own width.

use crate::ast::{BinaryOp, ScalarType};
use crate::span::Span;
use crate::types::{
    integer_scalar, is_integer_scalar, is_signed_scalar, scalar_bits, unify_numeric, Ty,
};

use super::{ConstError, ConstValue};

/// A folded value together with the scalar type its expression carries.
///
/// The scalar is `None` when the expression has no integer width of its own: a
/// context-free fold, a boolean result, or a value whose type the folder cannot
/// name. Carrying it lets each operation wrap and fault at the expression's own
/// type instead of at the wider type the constant context later requires.
#[derive(Clone, Copy, Debug)]
pub(super) struct FoldValue {
    /// The scalar type of the folded expression, when it has one.
    pub(super) scalar: Option<ScalarType>,
    /// The folded value.
    pub(super) value: ConstValue,
}

/// A `usize`-typed folded value, the result type of a size-like intrinsic.
pub(super) fn usize_value(size: usize) -> FoldValue {
    FoldValue {
        scalar: Some(ScalarType::Usize),
        value: ConstValue::Int(size as i128),
    }
}

/// Wraps a wide value to the width and signedness of an integer context.
///
/// A `None` context leaves the value untouched in the wide accumulator.
pub(super) fn wrap_int(value: i128, context: Option<ScalarType>) -> i128 {
    let Some(scalar) = context.filter(|scalar| is_integer_scalar(*scalar)) else {
        return value;
    };
    let bits = scalar_bits(scalar);
    if bits >= 128 {
        return value;
    }
    let modulus = 1i128 << bits;
    let mask = modulus - 1;
    let truncated = value & mask;
    if is_signed_scalar(scalar) && (truncated & (modulus >> 1)) != 0 {
        truncated - modulus
    } else {
        truncated
    }
}

/// The most negative value an integer context can hold, when it is signed.
fn signed_min(scalar: ScalarType) -> Option<i128> {
    match scalar {
        ScalarType::I8 => Some(i8::MIN as i128),
        ScalarType::I16 => Some(i16::MIN as i128),
        ScalarType::I32 | ScalarType::Isize => Some(i32::MIN as i128),
        _ => None,
    }
}

/// The scalar type an integer binary operation is evaluated at.
///
/// A shift has the left operand's type, because it keeps that type and masks
/// its count to that width. Every other integer operator unifies its operands
/// under the T1a widenings. `None` means the operation runs in the wide
/// accumulator, either because neither operand has a type or because the two
/// operand types do not unify.
pub(super) fn binary_operand_scalar(
    op: BinaryOp,
    left: Option<ScalarType>,
    right: Option<ScalarType>,
) -> Option<ScalarType> {
    if matches!(op, BinaryOp::Shl | BinaryOp::Shr) {
        return left;
    }
    match (left, right) {
        (Some(left), Some(right)) => {
            unify_numeric(&Ty::scalar(left), &Ty::scalar(right)).and_then(|ty| integer_scalar(&ty))
        }
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

/// The scalar type an integer operation results in.
///
/// A comparison yields `bool`, which has no integer width; every other integer
/// operator keeps the type it was evaluated at.
pub(super) fn result_scalar(op: BinaryOp, operand: Option<ScalarType>) -> Option<ScalarType> {
    match op {
        BinaryOp::Eq
        | BinaryOp::Ne
        | BinaryOp::Lt
        | BinaryOp::Le
        | BinaryOp::Gt
        | BinaryOp::Ge => None,
        _ => operand,
    }
}

/// Folds a binary operator over two integers with runtime semantics.
///
/// `context` is the integer type the operation is evaluated at: it fixes the
/// wrap width, the shift-count mask, and the signed minimum over `-1`. Without
/// a context the wide accumulator's own rules apply.
pub(super) fn fold_integer(
    op: BinaryOp,
    a: i128,
    b: i128,
    span: Span,
    context: Option<ScalarType>,
) -> Result<ConstValue, ConstError> {
    let wrap = |value: i128| ConstValue::Int(wrap_int(value, context));
    match op {
        BinaryOp::Add => Ok(wrap(a.wrapping_add(b))),
        BinaryOp::Sub => Ok(wrap(a.wrapping_sub(b))),
        BinaryOp::Mul => Ok(wrap(a.wrapping_mul(b))),
        BinaryOp::Div => {
            if let Some(min) = context.and_then(signed_min) {
                if a == min && b == -1 {
                    return Err(ConstError::DivisionByZero(span));
                }
            }
            a.checked_div(b)
                .map(wrap)
                .ok_or(ConstError::DivisionByZero(span))
        }
        BinaryOp::Rem => {
            if let Some(min) = context.and_then(signed_min) {
                if a == min && b == -1 {
                    return Err(ConstError::DivisionByZero(span));
                }
            }
            a.checked_rem(b)
                .map(wrap)
                .ok_or(ConstError::DivisionByZero(span))
        }
        BinaryOp::BitAnd => Ok(wrap(a & b)),
        BinaryOp::BitOr => Ok(wrap(a | b)),
        BinaryOp::BitXor => Ok(wrap(a ^ b)),
        BinaryOp::Shl => {
            let count = shift_count(b, context);
            Ok(wrap(a.wrapping_shl(count)))
        }
        BinaryOp::Shr => {
            let count = shift_count(b, context);
            Ok(wrap(a.wrapping_shr(count)))
        }
        BinaryOp::Eq => Ok(ConstValue::Bool(a == b)),
        BinaryOp::Ne => Ok(ConstValue::Bool(a != b)),
        BinaryOp::Lt => Ok(ConstValue::Bool(a < b)),
        BinaryOp::Le => Ok(ConstValue::Bool(a <= b)),
        BinaryOp::Gt => Ok(ConstValue::Bool(a > b)),
        BinaryOp::Ge => Ok(ConstValue::Bool(a >= b)),
        BinaryOp::LogicalAnd | BinaryOp::LogicalOr => Err(ConstError::NotConstant(span)),
    }
}

/// Masks a shift count to the width of the operand's type.
fn shift_count(count: i128, context: Option<ScalarType>) -> u32 {
    let width = context
        .filter(|scalar| is_integer_scalar(*scalar))
        .map(scalar_bits)
        .unwrap_or(128);
    (count as u32) & (width - 1)
}

/// Folds a binary operator over two floats. A float division by zero yields an
/// infinity rather than an error, matching the defined float behavior.
pub(super) fn fold_float(op: BinaryOp, a: f64, b: f64, span: Span) -> Result<ConstValue, ConstError> {
    match op {
        BinaryOp::Add => Ok(ConstValue::Float(a + b)),
        BinaryOp::Sub => Ok(ConstValue::Float(a - b)),
        BinaryOp::Mul => Ok(ConstValue::Float(a * b)),
        BinaryOp::Div => Ok(ConstValue::Float(a / b)),
        BinaryOp::Eq => Ok(ConstValue::Bool(a == b)),
        BinaryOp::Ne => Ok(ConstValue::Bool(a != b)),
        BinaryOp::Lt => Ok(ConstValue::Bool(a < b)),
        BinaryOp::Le => Ok(ConstValue::Bool(a <= b)),
        BinaryOp::Gt => Ok(ConstValue::Bool(a > b)),
        BinaryOp::Ge => Ok(ConstValue::Bool(a >= b)),
        _ => Err(ConstError::NotConstant(span)),
    }
}

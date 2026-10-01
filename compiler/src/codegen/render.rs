//! Pure C rendering helpers for the expression emitter.
//!
//! These functions turn small pieces of the typed IR into C text and carry no
//! emitter state: index spellings, bounds conditions, the wrapping of signed
//! arithmetic, operator tokens, float literals, and C string escaping. They are
//! split from the expression emitter so each file stays focused and within the
//! project size limit.

use crate::ast::{BinaryOp, ScalarType};
use crate::types::{Ty, TyKind, TypeTable};

use super::types::{pointer_to_array_type, scalar_c_type};

/// The C condition of an index bounds guard.
///
/// A signed index is first checked against zero so a negative value never
/// reaches the unsigned comparison. Both sides are cast to `usize`, the type
/// of a view's length, so a signed index does not raise a sign-compare
/// warning under `-Werror`.
pub(super) fn bounds_condition(index_c: &str, signed: bool, bound: &str) -> String {
    if signed {
        format!("({index_c} < 0) || ((usize)({index_c}) >= (usize)({bound}))")
    } else {
        format!("(usize)({index_c}) >= (usize)({bound})")
    }
}

/// The C text of an indexed access, spelling the pointer of a view.
pub(super) fn index_text(ty: &Ty, base: &str, index: &str, table: &TypeTable) -> String {
    match &ty.kind {
        TyKind::Str | TyKind::StaticStr => format!("({base}).ptr[{index}]"),
        TyKind::Array { element, extents } if extents.iter().any(Option::is_none) => {
            // A slice of rows keeps a base-element pointer; a row-typed
            // access reinterprets the pointer as a pointer to the row, so
            // the element read yields the whole row rather than one scalar.
            // The row type keeps every extent after the first.
            if extents.len() > 1 {
                let row = Ty::array((**element).clone(), extents[1..].to_vec());
                if let Some(row_type) = pointer_to_array_type(&row, table) {
                    return format!("(({row_type})({base}).ptr)[{index}]");
                }
            }
            format!("({base}).ptr[{index}]")
        }
        _ => format!("{base}[{index}]"),
    }
}

/// The element type of an array, slice, or string view.
pub(super) fn element_type(ty: &Ty) -> Ty {
    match &ty.kind {
        TyKind::Array { element, .. } => (**element).clone(),
        TyKind::Str | TyKind::StaticStr => Ty::scalar(ScalarType::U8),
        _ => Ty::unknown(),
    }
}

/// The unsigned scalar of the same width as a signed scalar.
pub(super) fn unsigned_scalar(scalar: ScalarType) -> ScalarType {
    match scalar {
        ScalarType::I8 => ScalarType::U8,
        ScalarType::I16 => ScalarType::U16,
        ScalarType::I32 => ScalarType::U32,
        ScalarType::Isize => ScalarType::Usize,
        other => other,
    }
}

/// The C typedef of an unsigned scalar.
pub(super) fn unsigned_c(scalar: ScalarType) -> &'static str {
    scalar_c_type(scalar)
}

/// The most negative value of a signed scalar, as a C literal.
pub(super) fn signed_min(scalar: ScalarType) -> &'static str {
    match scalar {
        ScalarType::I8 => "-128",
        ScalarType::I16 => "-32768",
        ScalarType::I32 | ScalarType::Isize => "-2147483648",
        _ => "0",
    }
}

/// The C token of a binary operator.
pub(super) fn binary_token(op: BinaryOp) -> &'static str {
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

/// Formats a float literal with a decimal point.
pub(super) fn float_literal(value: f64) -> String {
    let text = format!("{value}");
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

/// Escapes bytes into a C string literal.
pub(super) fn escape_c_string(bytes: &[u8]) -> String {
    let mut text = String::from("\"");
    for byte in bytes {
        match *byte {
            b'\\' => text.push_str("\\\\"),
            b'"' => text.push_str("\\\""),
            b'\n' => text.push_str("\\n"),
            b'\r' => text.push_str("\\r"),
            b'\t' => text.push_str("\\t"),
            0x20..=0x7e => text.push(*byte as char),
            other => text.push_str(&format!("\\{other:03o}")),
        }
    }
    text.push('"');
    text
}

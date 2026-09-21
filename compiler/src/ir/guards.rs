//! Runtime-guard prediction for lowered values.
//!
//! A structured C loop evaluates its condition in the loop header, so a
//! condition that needs a runtime guard cannot be printed there directly; the
//! emitter moves the guard into the loop body instead. This walk mirrors the
//! emitter's guard placement exactly and errs toward reporting a guard when it
//! is unsure, so a value wrongly called unguarded is the only way to drop one.

use crate::ast::BinaryOp;
use crate::types::TyKind;

use super::{Value, ValueKind, ValuePiece};

/// True when evaluating a value emits a runtime guard before it can run.
///
/// A guard is a division check, a bounds check, or a float-to-integer range
/// check. A structured loop keeps its shape only when its header holds no such
/// guard; the emitter moves the guard into the body otherwise.
pub fn value_needs_guards(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Binary { op, left, right } => {
            let division = matches!(op, BinaryOp::Div | BinaryOp::Rem) && left.ty.is_integer();
            division || value_needs_guards(left) || value_needs_guards(right)
        }
        ValueKind::Index { base, index } => {
            let bounds = matches!(
                base.ty.kind,
                TyKind::Array { .. } | TyKind::Str | TyKind::StaticStr
            );
            bounds || value_needs_guards(base) || value_needs_guards(index)
        }
        ValueKind::Cast { value: inner, .. } => {
            let target = &value.ty;
            let source = &inner.ty;
            let range = target.is_integer() && source.is_numeric() && !source.is_integer();
            range || value_needs_guards(inner)
        }
        ValueKind::Unary { operand, .. } => value_needs_guards(operand),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            value_needs_guards(condition)
                || value_needs_guards(then_value)
                || value_needs_guards(else_value)
        }
        ValueKind::Member { base, .. } => value_needs_guards(base),
        ValueKind::Call {
            args, receiver, ..
        } => {
            receiver
                .as_deref()
                .is_some_and(value_needs_guards)
                || args.iter().any(value_needs_guards)
        }
        ValueKind::Intrinsic { args, .. } => args.iter().any(value_needs_guards),
        ValueKind::StructLit { fields, .. } => {
            fields.iter().any(|(_, field)| value_needs_guards(field))
        }
        ValueKind::ArrayLit { elements, fill } => {
            elements.iter().any(value_needs_guards)
                || fill.as_deref().is_some_and(value_needs_guards)
        }
        ValueKind::Template(pieces) => pieces.iter().any(|piece| match piece {
            ValuePiece::Literal(_) => false,
            ValuePiece::Placeholder(inner) => value_needs_guards(inner),
        }),
        ValueKind::Place(_)
        | ValueKind::Int(_)
        | ValueKind::Float(_)
        | ValueKind::Char(_)
        | ValueKind::Bool(_)
        | ValueKind::Null
        | ValueKind::Str(_)
        | ValueKind::Path(_)
        | ValueKind::Aggregate
        | ValueKind::Temp(_) => false,
    }
}

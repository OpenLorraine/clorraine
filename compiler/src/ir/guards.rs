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

/// True when a value contains a materialized operand.
///
/// A materialization needs a statement position. A loop header cannot carry
/// one, so `while` emission splits a short-circuit condition whose right side
/// contains a materialization into two tests; the first test guards the right
/// side's statement fill and preserves short-circuit evaluation.
pub fn value_has_materialize(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Materialize { .. } => true,
        ValueKind::Binary { left, right, .. } => {
            value_has_materialize(left) || value_has_materialize(right)
        }
        ValueKind::Index { base, index } => {
            value_has_materialize(base) || value_has_materialize(index)
        }
        ValueKind::Cast { value: inner, .. } | ValueKind::Unary { operand: inner, .. } => {
            value_has_materialize(inner)
        }
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            value_has_materialize(condition)
                || value_has_materialize(then_value)
                || value_has_materialize(else_value)
        }
        ValueKind::Member { base, .. } => value_has_materialize(base),
        ValueKind::Call {
            args, receiver, ..
        } => {
            receiver
                .as_deref()
                .is_some_and(value_has_materialize)
                || args.iter().any(value_has_materialize)
        }
        ValueKind::Intrinsic { args, .. } => args.iter().any(value_has_materialize),
        ValueKind::StructLit { fields, .. } => {
            fields.iter().any(|(_, field)| value_has_materialize(field))
        }
        ValueKind::ArrayLit { elements, fill } => {
            elements.iter().any(value_has_materialize)
                || fill.as_deref().is_some_and(value_has_materialize)
        }
        ValueKind::Template(pieces) => pieces.iter().any(|piece| match piece {
            ValuePiece::Literal(_) => false,
            ValuePiece::Placeholder { value: inner, .. } => value_has_materialize(inner),
        }),
        ValueKind::Closure { captures, .. } => captures.iter().any(value_has_materialize),
        ValueKind::ClosureCall { callable, args } => {
            value_has_materialize(callable) || args.iter().any(value_has_materialize)
        }
        ValueKind::DiscardThen { effects, value } => {
            value_has_materialize(effects) || value_has_materialize(value)
        }
        _ => false,
    }
}

/// True when evaluating a value cannot live in a structured loop header.
///
/// A guard is a division check, a bounds check, or a float-to-integer range
/// check. A materialized operand also counts: its store, its drop of a previous
/// value, and its release are statements, so the condition has to move into the
/// loop body. A structured loop keeps its shape only when its header carries
/// neither; the emitter moves the work into the body otherwise.
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
            ValuePiece::Placeholder { value: inner, .. } => value_needs_guards(inner),
        }),
        // A materialized operand is always evaluated where it stands, so it is
        // filled by a statement before the construct; a loop header cannot
        // carry that statement.
        ValueKind::Materialize { .. } => true,
        // The effects must run where the value is evaluated; a loop header
        // moves them into the per-test guard position.
        ValueKind::DiscardThen { .. } => true,
        ValueKind::Closure { captures, .. } => captures.iter().any(value_needs_guards),
        ValueKind::ClosureCall { callable, args } => {
            value_needs_guards(callable) || args.iter().any(value_needs_guards)
        }
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

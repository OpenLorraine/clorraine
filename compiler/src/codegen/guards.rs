//! Runtime-guard emission for lowered values.
//!
//! A guard is a division check, a bounds check, or a float-to-integer range
//! check. Most guards are emitted as statements before the guarded statement,
//! but the right operand of `&&`/`||` and the arms of `?:` are evaluated only
//! sometimes, so their guards embed inside the expression instead. This module
//! owns both spellings and the conditions they share.

use crate::ast::{BinaryOp, ScalarType};
use crate::ir::{Value, ValueKind, ValuePiece};
use crate::types::{is_signed_scalar, Ty, TyKind};

use super::render::{bounds_condition, signed_min};
use super::types::scalar_of;
use super::Emitter;

impl<'a> Emitter<'a> {

    /// Emits every runtime guard a value's evaluation needs.
    ///
    /// The right operand of `&&`/`||` and the arms of `?:` are evaluated only
    /// sometimes, so their guards are embedded by [`Self::value`] instead of
    /// hoisted here; hoisting would check an operand the program never
    /// evaluates and trap a correct program.
    pub(super) fn emit_guards(&mut self, value: &Value) {
        match &value.kind {
            ValueKind::Binary {
                op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr,
                left,
                ..
            } => {
                self.emit_guards(left);
            }
            ValueKind::Binary { op, left, right } => {
                self.emit_guards(left);
                self.emit_guards(right);
                if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                    self.emit_division_guard(left, right);
                }
            }
            ValueKind::Index { base, index } => {
                self.emit_guards(base);
                self.emit_guards(index);
                self.emit_bounds_guard(base, index);
            }
            ValueKind::Cast { value: inner, .. } => {
                self.emit_guards(inner);
                let target = self.substitute(&value.ty);
                let source = self.substitute(&inner.ty);
                if target.is_integer() && source.is_numeric() && !source.is_integer() {
                    self.emit_float_guard(inner, &target);
                }
            }
            ValueKind::Call {
                args, receiver, ..
            } => {
                if let Some(receiver) = receiver {
                    self.emit_guards(receiver);
                }
                for arg in args {
                    self.emit_guards(arg);
                }
            }
            ValueKind::Unary { operand, .. } => self.emit_guards(operand),
            // Only the condition always runs; the arms' guards are embedded.
            ValueKind::Conditional { condition, .. } => {
                self.emit_guards(condition);
            }
            ValueKind::Member { base, .. } => self.emit_guards(base),
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.emit_guards(field);
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.emit_guards(element);
                }
                if let Some(fill) = fill {
                    self.emit_guards(fill);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.emit_guards(inner);
                    }
                }
            }
            ValueKind::Intrinsic { name, args, .. } => {
                for arg in args {
                    self.emit_guards(arg);
                }
                if name == "slice" {
                    self.emit_slice_guard(args);
                }
            }
            // A materialized operand is always evaluated where it stands, so
            // its assignment runs as a statement before the guarded construct.
            ValueKind::Materialize { temp, value: inner } => {
                self.emit_guards(inner);
                let rendered = self.value(inner);
                self.push(&format!("    __cl_tmp{temp} = {rendered};"));
            }
            _ => {}
        }
    }

    /// Renders a value whose evaluation is conditional, with its guards
    /// embedded as expressions.
    ///
    /// A short-circuit operand is evaluated only when its controlling test
    /// holds, so its guard cannot be a statement before the enclosing
    /// construct. The guard becomes a comma operand: it traps when the check
    /// fails and evaluates to a harmless value otherwise. A nested `&&`, `||`,
    /// or `?:` keeps its own conditional guards nested inside the same
    /// expression.
    pub(super) fn guarded_value(&self, value: &Value) -> String {
        match &value.kind {
            ValueKind::Binary {
                op: BinaryOp::LogicalAnd,
                left,
                right,
            } => format!(
                "({} ? {} : 0)",
                self.guarded_value(left),
                self.guarded_value(right)
            ),
            ValueKind::Binary {
                op: BinaryOp::LogicalOr,
                left,
                right,
            } => format!(
                "({} ? 1 : {})",
                self.guarded_value(left),
                self.guarded_value(right)
            ),
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => format!(
                "({} ? {} : {})",
                self.guarded_value(condition),
                self.guarded_value(then_value),
                self.guarded_value(else_value)
            ),
            _ => {
                let mut guards: Vec<String> = Vec::new();
                self.guard_expressions(value, &mut guards);
                let rendered = self.value(value);
                if guards.is_empty() {
                    rendered
                } else {
                    format!("(({}), {})", guards.join(", "), rendered)
                }
            }
        }
    }

    /// Collects the guard expressions of a value's always-evaluated part.
    ///
    /// The right operand of a logical operator and the arms of `?:` are left
    /// out because [`Self::guarded_value`] embeds their guards where the
    /// operand is actually evaluated.
    fn guard_expressions(&self, value: &Value, out: &mut Vec<String>) {
        match &value.kind {
            ValueKind::Binary {
                op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr,
                left,
                ..
            }
            | ValueKind::Conditional {
                condition: left, ..
            } => self.guard_expressions(left, out),
            ValueKind::Binary { op, left, right } => {
                self.guard_expressions(left, out);
                self.guard_expressions(right, out);
                if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                    if let Some(condition) = self.division_guard_condition(left, right) {
                        out.push(trap_expression(&condition, "E11210", "integer division by zero"));
                    }
                }
            }
            ValueKind::Index { base, index } => {
                self.guard_expressions(base, out);
                self.guard_expressions(index, out);
                if let Some(condition) = self.bounds_guard_condition(base, index) {
                    out.push(trap_expression(&condition, "E11211", "index out of range"));
                }
            }
            ValueKind::Cast { value: inner, .. } => {
                self.guard_expressions(inner, out);
                let target = self.substitute(&value.ty);
                let source = self.substitute(&inner.ty);
                if target.is_integer() && source.is_numeric() && !source.is_integer() {
                    if let Some(condition) = self.float_guard_condition(inner, &target) {
                        out.push(trap_expression(
                            &condition,
                            "E11212",
                            "float-to-integer conversion out of range",
                        ));
                    }
                }
            }
            ValueKind::Unary { operand, .. } => self.guard_expressions(operand, out),
            ValueKind::Member { base, .. } => self.guard_expressions(base, out),
            ValueKind::Call {
                args, receiver, ..
            } => {
                if let Some(receiver) = receiver {
                    self.guard_expressions(receiver, out);
                }
                for arg in args {
                    self.guard_expressions(arg, out);
                }
            }
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.guard_expressions(field, out);
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.guard_expressions(element, out);
                }
                if let Some(fill) = fill {
                    self.guard_expressions(fill, out);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.guard_expressions(inner, out);
                    }
                }
            }
            ValueKind::Intrinsic { name, args, .. } => {
                for arg in args {
                    self.guard_expressions(arg, out);
                }
                if name == "slice" {
                    if let Some(condition) = self.slice_guard_condition(args) {
                        out.push(trap_expression(&condition, "E11211", "index out of range"));
                    }
                }
            }
            // A materialized operand inside a conditionally evaluated value
            // (a short-circuit operand or a `?:` arm) carries its assignment
            // as an expression, so the side effect runs only on that path.
            ValueKind::Materialize { temp, value: inner } => {
                self.guard_expressions(inner, out);
                out.push(format!("(__cl_tmp{temp} = {})", self.value(inner)));
            }
            _ => {}
        }
    }

    /// Emits the integer division and modulo guard.
    ///
    /// Float division is IEEE-defined and never faults, so only an integer
    /// operation gets a guard.
    fn emit_division_guard(&mut self, left: &Value, right: &Value) {
        if let Some(condition) = self.division_guard_condition(left, right) {
            self.push(&format!(
                "    if ({condition}) cl_trap(\"E11210\", \"integer division by zero\");"
            ));
        }
    }

    /// The condition under which an integer division or modulo aborts.
    fn division_guard_condition(&self, left: &Value, right: &Value) -> Option<String> {
        let ty = self.substitute(&left.ty);
        if !ty.is_integer() {
            return None;
        }
        let dividend = self.value(left);
        let divisor = self.value(right);
        Some(match scalar_of(&ty) {
            Some(scalar) if is_signed_scalar(scalar) => {
                let min = signed_min(scalar);
                format!("{divisor} == 0 || ({dividend} == {min} && {divisor} == -1)")
            }
            _ => format!("{divisor} == 0"),
        })
    }

    /// Emits the index bounds check for a fixed array, slice, or string view.
    ///
    /// A signed index also has to be non-negative: the upper-bound comparison
    /// alone lets `xs[-1]` through. The comparison casts both sides to the
    /// unsigned `usize` so a signed index does not trigger
    /// `-Werror=sign-compare`; the negative check runs first, so the cast
    /// never sees a value that wrapped.
    fn emit_bounds_guard(&mut self, base: &Value, index: &Value) {
        if let Some(condition) = self.bounds_guard_condition(base, index) {
            self.push(&format!(
                "    if ({condition}) cl_trap(\"E11211\", \"index out of range\");"
            ));
        }
    }

    /// The condition under which an index is out of range.
    fn bounds_guard_condition(&self, base: &Value, index: &Value) -> Option<String> {
        let ty = self.substitute(&base.ty);
        let index_c = self.value(index);
        let signed = scalar_of(&self.substitute(&index.ty)).is_some_and(is_signed_scalar);
        match &ty.kind {
            TyKind::Array { extents, .. } => {
                if extents.iter().all(Option::is_some) {
                    let extent = extents
                        .iter()
                        .filter_map(|size| *size)
                        .fold(1usize, usize::saturating_mul);
                    Some(bounds_condition(&index_c, signed, &extent.to_string()))
                } else {
                    let base_c = self.value(base);
                    Some(bounds_condition(&index_c, signed, &format!("({base_c}).len")))
                }
            }
            TyKind::Str | TyKind::StaticStr => {
                let base_c = self.value(base);
                Some(bounds_condition(&index_c, signed, &format!("({base_c}).len")))
            }
            _ => None,
        }
    }

    /// Emits the float-to-integer range guard.
    fn emit_float_guard(&mut self, value: &Value, target: &Ty) {
        if let Some(condition) = self.float_guard_condition(value, target) {
            self.push(&format!(
                "    if ({condition}) cl_trap(\"E11212\", \"float-to-integer conversion out of range\");"
            ));
        }
    }

    /// The condition under which a float-to-integer conversion is out of range.
    fn float_guard_condition(&self, value: &Value, target: &Ty) -> Option<String> {
        let value_c = self.value(value);
        let (low, high) = match scalar_of(target) {
            Some(ScalarType::I8) => ("-128.0", "127.0"),
            Some(ScalarType::I16) => ("-32768.0", "32767.0"),
            Some(ScalarType::I32) | Some(ScalarType::Isize) => ("-2147483648.0", "2147483647.0"),
            Some(ScalarType::U8) => ("0.0", "255.0"),
            Some(ScalarType::U16) => ("0.0", "65535.0"),
            Some(ScalarType::U32) | Some(ScalarType::Usize) => ("0.0", "4294967295.0"),
            _ => return None,
        };
        Some(format!(
            "cl_isnan({value_c}) || !({value_c} >= {low} && {value_c} <= {high})"
        ))
    }

}

/// The C expression that aborts when `condition` holds.
///
/// A guard that must embed inside an expression cannot use an `if` statement,
/// so it becomes a ternary whose trap branch runs `cl_trap` through the comma
/// operator and yields zero. The expression always has type `int`, so it is a
/// valid comma operand wherever a guard belongs.
fn trap_expression(condition: &str, code: &str, message: &str) -> String {
    format!("(({condition}) ? (cl_trap(\"{code}\", \"{message}\"), 0) : 1)")
}

//! Instantiation-time re-check of the substitution-blind guards.
//!
//! The type stage checks a struct field typed `T` and a fill whose element is
//! `T` against the type as it is written: a parameter is not a view and has no
//! known duplicability, so both pass. This module runs once, after the
//! monomorphization is complete, and re-applies the same two predicates to each
//! recorded concrete instantiation so the substituted kind is judged. It
//! reports through the same diagnostics the declaration-site guards use and
//! changes no other stage.
//!
//! The audit reads only the recorded instance set: it never discovers an
//! instance the collectors missed. A program whose instance is never recorded
//! (a discarded call's result type, a nested struct reached only from the IR, or
//! a fill inside a lifted lambda body) is left to the host C compiler, which
//! rejects the undeclared or unsubstituted type it emits.

use crate::diag::{Diagnostic, Diagnostics};
use crate::ir::{
    Function, Instance, Monomorphization, Statement, Terminator, Value, ValueKind,
};
use crate::load::Loaded;
use crate::sema::SemaOutput;
use crate::span::Span;
use crate::types::{resolve_ty, substitute_instance, Ty, TyKind};

use super::generics::{struct_declarations, type_param_names};
use super::lower::mentions_type_param;
use super::program::{Callee, ProgramInfo};

/// Audits the recorded concrete instantiations for the two guards that cannot
/// see a substituted type.
///
/// It reports `E10205` for a substituted view-typed field and `E10215` for a
/// substituted non-duplicable fill. Only declared types that mention a type
/// parameter are checked, so a program whose illegal field or fill is already
/// concrete keeps its single declaration-site diagnostic.
pub fn check(
    loaded: &Loaded,
    program: &ProgramInfo,
    functions: &[Function],
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    check_view_fields(loaded, sema, mono, diagnostics);
    check_fill_instances(program, functions, sema, mono, diagnostics);
}

/// Reports a substituted view field for every recorded struct instance.
///
/// For each instance, every field whose declared type mentions a type parameter
/// is substituted with the instance arguments and tested with the same
/// predicate as the declaration-site guard: a `str` or a slice is rejected.
/// Nullability does not change the predicate, so a substituted `str?` is
/// rejected too.
fn check_view_fields(
    loaded: &Loaded,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    let declarations = struct_declarations(loaded);
    for instance in mono.struct_instances() {
        let Some(decl) = declarations.get(instance.name.as_str()) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for field in &decl.fields {
            let declared = resolve_ty(&field.ty, &params, &sema.table);
            if !mentions_type_param(&declared) {
                continue;
            }
            let substituted = substitute_instance(&declared, &params, &instance.args);
            if matches!(substituted.kind, TyKind::Str) || substituted.is_slice() {
                diagnostics.push(
                    Diagnostic::error(
                        "E10205",
                        format!("view-typed field '{}' is not allowed", field.name),
                        field.name_span,
                    )
                    .with_help("store an owner, or `static str` for literals"),
                );
            }
        }
    }
}

/// Reports a substituted non-duplicable fill for every recorded function
/// instance.
///
/// The fill lives in the generic body, so each instance's lowered body is
/// walked. A declaration that is not a user function, or an instance with no
/// lowered body, is skipped to keep the walk total.
fn check_fill_instances(
    program: &ProgramInfo,
    functions: &[Function],
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for instance in mono.instances() {
        let Some(Callee::User(decl)) = program.callee(&instance.function) else {
            continue;
        };
        let Some(function) = functions.iter().find(|function| function.name == decl.name)
        else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        check_function_fills(function, &params, &instance.args, sema, diagnostics);
    }
}

/// Walks one lowered function body looking for fill literals to judge.
///
/// The statement and terminator shapes mirror the instance collector's walk, so
/// every value position that can hold an array literal is reached: assignments,
/// stores, call arguments, discards, return values, branch conditions, and
/// switch scrutinees and labels. Each value carries the enclosing instruction
/// span so the diagnostic points at the fill's statement.
fn check_function_fills(
    function: &Function,
    params: &[String],
    args: &[Ty],
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) {
    for block in &function.blocks {
        for instr in &block.statements {
            match &instr.statement {
                Statement::Assign { value, .. } => {
                    check_value_fills(value, params, args, sema, instr.span, diagnostics);
                }
                Statement::StorePointer { pointer, value } => {
                    check_value_fills(pointer, params, args, sema, instr.span, diagnostics);
                    check_value_fills(value, params, args, sema, instr.span, diagnostics);
                }
                Statement::Call { args: call_args, .. } => {
                    for arg in call_args {
                        check_value_fills(arg, params, args, sema, instr.span, diagnostics);
                    }
                }
                Statement::Discard { value } => {
                    check_value_fills(value, params, args, sema, instr.span, diagnostics);
                }
                _ => {}
            }
        }
        match &block.terminator {
            Terminator::Branch { condition, .. } => {
                check_value_fills(
                    condition,
                    params,
                    args,
                    sema,
                    block.terminator_span,
                    diagnostics,
                );
            }
            Terminator::Switch { value, arms, .. } => {
                check_value_fills(value, params, args, sema, block.terminator_span, diagnostics);
                for (label, _) in arms {
                    check_value_fills(
                        label,
                        params,
                        args,
                        sema,
                        block.terminator_span,
                        diagnostics,
                    );
                }
            }
            Terminator::Return(Some(value)) => {
                check_value_fills(value, params, args, sema, block.terminator_span, diagnostics);
            }
            _ => {}
        }
    }
}

/// Walks one value, judging every array literal with a fill and recursing into
/// the values nested inside it.
///
/// The recursion follows the same shapes the instance collector walks, so a
/// fill nested in a call argument, a struct field, or a conditional is reached.
fn check_value_fills(
    value: &Value,
    params: &[String],
    args: &[Ty],
    sema: &SemaOutput,
    span: Span,
    diagnostics: &mut Diagnostics,
) {
    if let ValueKind::ArrayLit {
        fill: Some(_), ..
    } = &value.kind
    {
        check_fill_type(&value.ty, params, args, sema, span, diagnostics);
    }
    match &value.kind {
        ValueKind::Call {
            receiver, args: call_args, ..
        } => {
            if let Some(receiver) = receiver {
                check_value_fills(receiver, params, args, sema, span, diagnostics);
            }
            for arg in call_args {
                check_value_fills(arg, params, args, sema, span, diagnostics);
            }
        }
        ValueKind::Member { base, .. } => {
            check_value_fills(base, params, args, sema, span, diagnostics);
        }
        ValueKind::Index { base, index } => {
            check_value_fills(base, params, args, sema, span, diagnostics);
            check_value_fills(index, params, args, sema, span, diagnostics);
        }
        ValueKind::Unary { operand, .. } => {
            check_value_fills(operand, params, args, sema, span, diagnostics);
        }
        ValueKind::Binary { left, right, .. } => {
            check_value_fills(left, params, args, sema, span, diagnostics);
            check_value_fills(right, params, args, sema, span, diagnostics);
        }
        ValueKind::Cast { value, .. } => {
            check_value_fills(value, params, args, sema, span, diagnostics);
        }
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            check_value_fills(condition, params, args, sema, span, diagnostics);
            check_value_fills(then_value, params, args, sema, span, diagnostics);
            check_value_fills(else_value, params, args, sema, span, diagnostics);
        }
        ValueKind::StructLit { fields, .. } => {
            for (_, field) in fields {
                check_value_fills(field, params, args, sema, span, diagnostics);
            }
        }
        ValueKind::ArrayLit { elements, fill } => {
            for element in elements {
                check_value_fills(element, params, args, sema, span, diagnostics);
            }
            if let Some(fill) = fill {
                check_value_fills(fill, params, args, sema, span, diagnostics);
            }
        }
        ValueKind::Template(pieces) => {
            for piece in pieces {
                if let crate::ir::ValuePiece::Placeholder(inner) = piece {
                    check_value_fills(inner, params, args, sema, span, diagnostics);
                }
            }
        }
        ValueKind::Intrinsic { args: call_args, .. } => {
            for arg in call_args {
                check_value_fills(arg, params, args, sema, span, diagnostics);
            }
        }
        ValueKind::Materialize { value, .. } => {
            check_value_fills(value, params, args, sema, span, diagnostics);
        }
        ValueKind::Closure { captures, .. } => {
            for capture in captures {
                check_value_fills(capture, params, args, sema, span, diagnostics);
            }
        }
        ValueKind::ClosureCall { callable, args: call_args } => {
            check_value_fills(callable, params, args, sema, span, diagnostics);
            for arg in call_args {
                check_value_fills(arg, params, args, sema, span, diagnostics);
            }
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
        | ValueKind::Temp(_) => {}
    }
}

/// Judges one array literal's type against the fill rule.
///
/// The literal is skipped unless its type mentions a type parameter: a concrete
/// fill is the declaration-site guard's case, which already stopped the program
/// before this audit runs. The type is substituted with the instance arguments,
/// and the base element of the resulting array is checked for duplicability.
/// An unresolved element is skipped rather than judged.
fn check_fill_type(
    ty: &Ty,
    params: &[String],
    args: &[Ty],
    sema: &SemaOutput,
    span: Span,
    diagnostics: &mut Diagnostics,
) {
    if !mentions_type_param(ty) {
        return;
    }
    let substituted = substitute_instance(ty, params, args);
    let TyKind::Array { element, .. } = &substituted.kind else {
        return;
    };
    let base = element.as_ref();
    if base.is_unknown() || base.is_param() {
        return;
    }
    if !sema.table.duplicable(base) {
        diagnostics.push(
            Diagnostic::error("E10215", "array fill requires a fixed extent", span).with_help(
                "a fill cannot repeat a move-only element; construct each element",
            ),
        );
    }
}

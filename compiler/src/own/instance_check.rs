//! Instantiation-time re-check of the substitution-blind guards.
//!
//! The type stage checks a struct field typed `T`, a fill whose element is
//! `T`, and a move of a lend whose type is a parameter against the type as it is
//! written: a parameter is not a view, has no known duplicability, and hides
//! whether a lend move is real. This module runs once, after the
//! monomorphization is complete, and re-applies the same rules to each recorded
//! concrete instantiation so the substituted kind is judged. It reports through
//! the same diagnostics the declaration-site rules use and changes no other
//! stage.
//!
//! The audit reads only the recorded instance set: it never discovers an
//! instance the collectors missed. A program whose instance is never recorded
//! (a discarded call's result type, a nested struct reached only from the IR, or
//! a fill inside a lifted lambda body) is left to the host C compiler, which
//! rejects the undeclared or unsubstituted type it emits.

use std::collections::BTreeSet;

use crate::borrow::is_view;
use crate::diag::{Diagnostic, Diagnostics};
use crate::ir::{Function, Monomorphization, Statement, Terminator, Value, ValueKind};
use crate::load::Loaded;
use crate::sema::SemaOutput;
use crate::span::Span;
use crate::types::{resolve_ty, substitute_instance, Ty, TyKind};

use super::generics::{struct_declarations, type_param_names};
use super::lower::mentions_type_param;
use super::program::{Callee, ProgramInfo};

/// Audits the recorded concrete instantiations for the guards that cannot see
/// a substituted type.
///
/// It reports `E10205` for a substituted view-typed field and `E10215` for a
/// substituted non-duplicable fill, `E10020` when a maybe-owning store's later
/// use needs a duplicable instance, `E10034` when a lend moved into an owned
/// position instantiates with an owning type, and the whole-binding
/// self-assign judgments (`E10020` for a nullable substitution, `E10034` for
/// an owning one). Only declared types that mention a type parameter are
/// checked for the field and fill guards, so a program whose illegal field or
/// fill is already concrete keeps its single declaration-site diagnostic.
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
    check_duplicable_requirements(program, sema, mono, diagnostics);
    check_not_lend_requirements(program, sema, mono, diagnostics);
    check_out_slot_requirements(program, mono, diagnostics);
    check_storable_slot_requirements(program, sema, mono, diagnostics);
    check_self_assign_requirements(program, sema, mono, diagnostics);
}

/// Reports `E10200` when an out-slot duty's instance makes the slot nullable
/// and the recorded argument is not an exact-typed place.
///
/// The generic body accepted the call because its slot type was still a
/// parameter; the audit substitutes each recorded instance's type arguments,
/// then applies the same exact-place rule the concrete call paths use. One duty
/// reports once, at its first invalid instance.
fn check_out_slot_requirements(
    program: &ProgramInfo,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    let mut reported: BTreeSet<(String, u32, u32)> = BTreeSet::new();
    for requirement in mono.out_slot_requirements() {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let slot = substitute_instance(&requirement.slot, &params, &instance.args);
            if !slot.nullable {
                continue;
            }
            let argument = substitute_instance(&requirement.argument, &params, &instance.args);
            if requirement.place && argument == slot {
                continue;
            }
            let key = (
                requirement.function.clone(),
                requirement.span.start,
                requirement.span.end,
            );
            if !reported.insert(key) {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10200",
                    format!(
                        "cannot convert '{}' to '{}' implicitly",
                        argument.display(),
                        slot.display()
                    ),
                    requirement.span,
                )
                .with_help("pass a place of the exact slot type; the callee writes through it"),
            );
        }
    }
}

/// Reports `E10034` when a recorded lend move instantiates with an owning type.
///
/// A generic body that moves a lend into an owned position is accepted while
/// the type is a parameter, because the checker cannot yet tell whether the
/// move is real. The audit substitutes each recorded instance's type arguments
/// into the moved type: a duplicable substitution stays a copy, while an
/// owning one reports the same diagnostic the concrete spelling emits. A
/// nullable substitution is exempt only where the move's destination is a
/// slot that may be the `borrow T?` out-slot carve-out; every other
/// destination keeps the strict judgment. One duty reports once — the first
/// owning instance — so the body site is not repeated.
fn check_not_lend_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for requirement in mono.not_lend_requirements() {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let substituted = substitute_instance(&requirement.ty, &params, &instance.args);
            // A nullable substitution is the `borrow T?` out-slot carve-out,
            // so a move stays legal where the concrete spelling is — but only
            // for a writable lend moved into a slot that may be that out-slot.
            // Both conditions are needed: a read-only source may never be
            // emptied, and a move into an owned position would leave the
            // source and the destination aliasing one allocation.
            if sema.table.duplicable(&substituted)
                || (substituted.nullable
                    && requirement.writable_source
                    && requirement.stores_into_slot)
            {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10034",
                    format!(
                        "cannot release or store borrowed parameter '{}'",
                        requirement.parameter
                    ),
                    requirement.span,
                )
                .with_help(
                    "declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt",
                ),
            );
            break;
        }
    }
}

/// Reports `E10034` when a recorded lend-slot store instantiates with an
/// owning type.
///
/// A generic body that stores a whole value into a lend parameter is accepted
/// while the slot type is still a parameter, because the checker cannot yet
/// tell whether the store would release caller storage. The audit substitutes
/// each recorded instance's type arguments into the slot type: a nullable
/// substitution is an out-parameter slot and a duplicable or view-typed one
/// stores no owned storage, so those stay legal, while an owning substitution
/// reports the same diagnostic the concrete spelling emits. One duty reports
/// once — the first owning instance — so the store site is not repeated.
fn check_storable_slot_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for requirement in mono.storable_slot_requirements() {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let substituted = substitute_instance(&requirement.ty, &params, &instance.args);
            if substituted.nullable
                || substituted.is_unknown()
                || is_view(&substituted)
                || sema.table.duplicable(&substituted)
            {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10034",
                    format!(
                        "cannot release or store borrowed parameter '{}'",
                        requirement.parameter
                    ),
                    requirement.span,
                )
                .with_help(
                    "declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt",
                ),
            );
            break;
        }
    }
}

/// Reports the whole-binding self-assign judgment at each recorded instance.
///
/// A generic body that assigns a lend root to itself is accepted while the
/// root's type is still a parameter, because the concrete rule depends on the
/// substitution. The audit substitutes each recorded instance's type arguments
/// and reproduces the concrete spelling's judgment: a nullable substitution
/// reports `E10020` — checked before the kind skips, so a nullable view, raw,
/// or cstring substitution reports exactly as its concrete `borrow T?`
/// spelling does — an owning non-nullable substitution reports `E10034`, and a
/// duplicable, view, or unknown substitution stays accepted. One duty reports
/// once — the first offending instance in instantiation order — so the body
/// site is not repeated.
fn check_self_assign_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for requirement in mono.self_assign_requirements() {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let substituted = substitute_instance(&requirement.ty, &params, &instance.args);
            if substituted.is_unknown() {
                continue;
            }
            if substituted.nullable {
                diagnostics.push(
                    Diagnostic::error(
                        "E10020",
                        format!("'{}' was moved and cannot be used", requirement.parameter),
                        requirement.span,
                    )
                    .with_help("re-initialize the binding or use a view"),
                );
                break;
            }
            if sema.table.duplicable(&substituted) || is_view(&substituted) {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10034",
                    format!(
                        "cannot release or store borrowed parameter '{}'",
                        requirement.parameter
                    ),
                    requirement.span,
                )
                .with_help(
                    "declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt",
                ),
            );
            break;
        }
    }
}

/// Reports the duplicability duty when an instance substitutes an owning type.
///
/// A generic body that stores a maybe-owning value and then uses the source
/// again is accepted only when the instance's type argument is duplicable. The
/// audit substitutes each recorded argument into the binding's declared type
/// and reports `E10020` at the conflicting use for the first owning
/// instantiation.
fn check_duplicable_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for requirement in mono.duplicable_requirements() {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        // The record carries the declared type, so a base-overridden binding
        // without a local entry is still judged.
        let declared = &requirement.ty;
        if !mentions_type_param(declared) {
            continue;
        }
        let params = type_param_names(&decl.type_params);
        let binding_name = requirement
            .binding
            .split('#')
            .next()
            .unwrap_or(&requirement.binding);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let substituted = substitute_instance(declared, &params, &instance.args);
            if sema.table.duplicable(&substituted) {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10020",
                    format!("'{binding_name}' was moved and cannot be used"),
                    requirement.span,
                )
                .with_help("re-initialize the binding or use a view"),
            );
            // One duty per function and binding: the first owning instance
            // reports it.
            break;
        }
    }
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
            Terminator::Return {
                value: Some(value), ..
            } => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Parser;
    use crate::profile::Profile;
    use crate::span::SourceMap;

    /// Runs the front end on one source and returns its error diagnostics as
    /// `(code, message)` pairs.
    fn errors(source: &str) -> Vec<(String, String)> {
        let mut sources = SourceMap::new();
        let mut sink = Diagnostics::new(false, None);
        let id = sources.add_file("test.clo".to_owned(), source.to_owned());
        let program = Parser::new(&sources, id, &mut sink).parse_program();
        let loaded = crate::load::load(vec![(id, program)], &mut sink);
        let sema = crate::sema::check(&loaded, Profile::Host, &mut sink);
        if !sink.failed() {
            let _ = crate::own::check(&loaded, &sema, &mut sink);
        }
        sink.items()
            .iter()
            .filter(|diagnostic| diagnostic.severity == crate::diag::Severity::Error)
            .map(|diagnostic| (diagnostic.code.to_owned(), diagnostic.message.clone()))
            .collect()
    }

    /// The messages of the errors with one code, in report order.
    fn errors_of(source: &str, code: &str) -> Vec<String> {
        errors(source)
            .into_iter()
            .filter(|(candidate, _)| candidate == code)
            .map(|(_, message)| message)
            .collect()
    }

    /// A concrete fill in a generic body is the declaration-site guard's case and
    /// is skipped by the audit, while the parameterized fill in the same body is
    /// judged once.
    #[test]
    fn the_parameter_mention_gate_skips_concrete_fills() {
        let source = r#"module test;
import { String, stringOf } from std.text;
func f<T>(a: T): T[2] {
    let concrete: i32[2] = [; 0];
    return [; a];
}
func demo(): void {
    let s: String = stringOf("x");
    let xs: String[2] = f<String>(s);
}
func main(): i32 { return 0; }
"#;
        assert_eq!(errors_of(source, "E10215").len(), 1);
    }

    /// A nested generic struct named by a literal is recorded, so the inner
    /// field is judged once with the outer instance's argument.
    #[test]
    fn a_declaration_named_nested_instance_reports_the_inner_field() {
        let source = r#"module test;
struct Wrap<T> {
    v: T;
}
struct Outer<T> {
    w: Wrap<T>;
}
func demo(): void {
    let o: Outer<str> = Outer<str> { w: Wrap<str> { v: "hi" } };
}
func main(): i32 { return 0; }
"#;
        let messages = errors_of(source, "E10205");
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("'v'"), "unexpected message: {}", messages[0]);
    }

    /// A multi-extent fill judges the base element, so a move-only substitution
    /// is rejected once.
    #[test]
    fn a_multi_extent_fill_uses_the_base_element() {
        let source = r#"module test;
import { String, stringOf } from std.text;
func g<T>(a: T): T[2][2] {
    return [; a];
}
func demo(): void {
    let s: String = stringOf("x");
    let m: String[2][2] = g<String>(s);
}
func main(): i32 { return 0; }
"#;
        assert_eq!(errors_of(source, "E10215").len(), 1);
    }

    /// The duplicable substitution is accepted: it is the ordinary fill form.
    #[test]
    fn a_duplicable_fill_is_accepted() {
        let source = r#"module test;
func fillIt<T>(a: T): T[2] {
    return [; a];
}
func demo(): void {
    let ns: i32[2] = fillIt<i32>(7);
}
func main(): i32 { return 0; }
"#;
        assert!(errors(source).is_empty(), "unexpected diagnostics: {:?}", errors(source));
    }

    /// A fill that is a local binding's value is reached through the statement
    /// walk, not only through a return.
    #[test]
    fn a_fill_in_a_local_statement_is_audited() {
        let source = r#"module test;
import { String, stringOf } from std.text;
func f<T>(a: T): i32 {
    let xs: T[2] = [; a];
    return 0;
}
func demo(): void {
    let s: String = stringOf("x");
    let n: i32 = f<String>(s);
}
func main(): i32 { return 0; }
"#;
        assert_eq!(errors_of(source, "E10215").len(), 1);
    }

    /// A later use after a maybe-owning move reports once for the owning
    /// instance.
    #[test]
    fn a_later_use_reports_the_duplicability_duty_once() {
        let source = r#"module test;
import { String, stringOf } from std.text;
func sink(borrow v: String): i32 { return i32(len(v)); }
func reuse<T>(take a: T): i32 {
    let b: T = a;
    sink(borrow a);
    return 0;
}
func demo(): void {
    let s: String = stringOf("x");
    let n: i32 = reuse<String>(s);
}
func main(): i32 { return 0; }
"#;
        let messages = errors_of(source, "E10020");
        assert_eq!(messages.len(), 1, "unexpected messages: {messages:?}");
        assert!(messages[0].contains("'a'"), "unexpected message: {}", messages[0]);
    }

    /// The same body instantiated with a duplicable type stays accepted.
    #[test]
    fn a_duplicable_instance_keeps_the_later_use_accepted() {
        let source = r#"module test;
func sink(borrow v: i32): i32 { return v; }
func reuse<T>(take a: T): i32 {
    let b: T = a;
    sink(borrow a);
    return sink(borrow b);
}
func demo(): void {
    let n: i32 = reuse<i32>(3);
}
func main(): i32 { return 0; }
"#;
        assert!(
            errors_of(source, "E10020").is_empty(),
            "a duplicable instance must stay accepted"
        );
    }

    /// A `borrow` parameter never becomes maybe-owning, so its ownsership is
    /// left to the caller.
    #[test]
    fn a_borrow_parameter_is_never_maybe_owning() {
        let source = r#"module test;
import { String, stringOf } from std.text;
func peek<T>(borrow a: T): i32 { return 1; }
func demo(): i32 {
    let s: String = stringOf("x");
    return peek<String>(s) + i32(len(s));
}
func main(): i32 { return 0; }
"#;
        assert!(
            errors_of(source, "E10020").is_empty(),
            "a borrow parameter must keep the caller's value usable"
        );
    }

    /// A fill that is a return value is reached through the terminator walk.
    #[test]
    fn a_fill_in_a_return_terminator_is_audited() {
        let source = r#"module test;
import { String, stringOf } from std.text;
func f<T>(a: T): T[2] {
    return [; a];
}
func demo(): void {
    let s: String = stringOf("x");
    let xs: String[2] = f<String>(s);
}
func main(): i32 { return 0; }
"#;
        assert_eq!(errors_of(source, "E10215").len(), 1);
    }
}

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

use std::collections::{BTreeMap, BTreeSet};

use crate::borrow::{is_view, is_view_like};
use crate::diag::{Diagnostic, Diagnostics};
use crate::ir::{Function, Monomorphization, Statement, Terminator, Value, ValueKind};
use crate::load::Loaded;
use crate::sema::{ConversionLiteral, SemaOutput};
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
/// use needs a duplicable instance, `E10026` when a borrowed element's moved
/// place owns at the instance, `E10034` when a lend moved into an owned
/// position instantiates with an owning type, and the whole-binding
/// self-assign judgments (`E10020` for a nullable substitution, `E10034` for
/// an owning one), and `E10043` when a recorded loop's substituted iterable
/// cannot be bound, and `E10212` when an opaque projection's substituted
/// base has no such member. Only declared types that mention a type parameter are
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
    check_element_move_requirements(program, sema, mono, diagnostics);
    check_not_lend_requirements(program, sema, mono, diagnostics);
    check_iterable_requirements(program, sema, mono, diagnostics);
    check_opaque_projections(program, sema, mono, diagnostics);
    check_opaque_indexes(program, sema, mono, diagnostics);
    check_opaque_member_uses(program, sema, mono, diagnostics);
    check_opaque_conditionals(program, sema, mono, diagnostics);
    check_deferred_conversions(program, sema, mono, diagnostics);
    check_out_slot_requirements(program, mono, diagnostics);
    check_storable_slot_requirements(program, sema, mono, diagnostics);
    check_self_assign_requirements(program, sema, mono, diagnostics);
    check_opaque_store_requirements(program, sema, mono, diagnostics);
    check_capture_move_requirements(program, sema, mono, diagnostics);
}

/// Reports `E10401` when a recorded capture move instantiates with an owning
/// capture type.
///
/// A lambda body that returns, consumes, or stores a capture is accepted
/// while the capture's type is still a parameter, because the declaration
/// cannot tell whether the move aliases an owner. The audit substitutes each
/// recorded instance's type arguments into the capture type: a duplicable or
/// unknown substitution stays a copy and is legal, while an owning one
/// reports the same diagnostic the concrete spelling emits. One duty reports
/// once — the first owning instance — so the body site is not repeated.
fn check_capture_move_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    let mut reported: BTreeSet<(String, String, u32)> = BTreeSet::new();
    for requirement in &sema.capture_moves {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let bound = crate::types::instance_bindings(&params, &instance.args);
            // A record from a store: only a nullable owning slot consumes the
            // value there. A duplicable destination copies the value in (a
            // view widening is the case) and a non-nullable one keeps the
            // pre-existing lend-store report, so neither is judged here.
            if let Some(dest) = &requirement.dest {
                let destination = crate::types::substitute(dest, &bound);
                if destination.is_unknown() || sema.table.duplicable(&destination) {
                    continue;
                }
                // A borrowed capture is only moved out of the caller's
                // storage at a nullable owning slot (the out-slot class); a
                // non-nullable destination keeps its frozen lend-store
                // report, exactly as the concrete spelling does.
                if requirement.borrow && !destination.nullable {
                    continue;
                }
            }
            let substituted = crate::types::substitute(&requirement.ty, &bound);
            // The move is judged at the place it consumes: a duplicable
            // projection copies out, so a field read stays accepted while a
            // whole capture or an owning projection is the unsound move.
            let moved =
                crate::types::project_place_path(&substituted, &requirement.path, &sema.table);
            if moved.is_unknown() || sema.table.duplicable(&moved) {
                continue;
            }
            let key = (
                requirement.function.clone(),
                requirement.capture.clone(),
                requirement.span.start,
            );
            if !reported.insert(key) {
                break;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10401",
                    format!("cannot move captured '{}' in a lambda", requirement.capture),
                    requirement.span,
                )
                .with_help("copy the value at capture time"),
            );
            break;
        }
    }
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
            // The duty names the root parameter; apply the projection after
            // the substitution so the selected member's duplicability is the
            // one judged. A raw-pointer element selects the pointee, which
            // `project_place_path` does not model; the other steps reuse its
            // field/element resolution.
            let path: Vec<crate::types::PlacePathStep> = requirement
                .projections
                .iter()
                .map(|projection| match projection {
                    crate::ir::Projection::Field(name) => {
                        crate::types::PlacePathStep::Field(name.clone())
                    }
                    crate::ir::Projection::Element(_)
                    | crate::ir::Projection::ElementTemp(_) => {
                        crate::types::PlacePathStep::Element
                    }
                })
                .collect();
            let judged = projected_type(sema, &substituted, &path);
            if sema.table.duplicable(&judged)
                && !is_view_member_projection(sema, &substituted, &path)
            {
                continue;
            }
            // A nullable substitution is the `borrow T?` out-slot carve-out,
            // so a move stays legal where the concrete spelling is — but only
            // for a writable lend moved into a slot that may be that out-slot.
            // Both conditions are needed: a read-only source may never be
            // emptied, and a move into an owned position would leave the
            // source and the destination aliasing one allocation.
            if judged.nullable && requirement.writable_source && requirement.stores_into_slot {
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

/// Reports the instance judgment for a store into a type-parameter member.
///
/// The declaration saw only `T`, so it could not tell whether the member holds
/// a view or an owner. Each instance substitutes the root and projects the
/// stored path: an incompatible pair reports the concrete implicit-conversion
/// error at the store, and a view member that receives a frame-dying backing
/// reports the escape at the return when the carrier is returned. A
/// caller-backed or static view stays accepted. One duty reports once — the
/// first offending instance — so the body site is not repeated.
fn check_opaque_store_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    let mut reported_conversion = BTreeSet::new();
    let mut reported_escape = BTreeSet::new();
    let mut reported_use: BTreeSet<(String, u32, &'static str)> = BTreeSet::new();
    for requirement in mono.opaque_store_requirements() {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let bindings = crate::types::instance_bindings(&params, &instance.args);
            let base = crate::types::substitute(&requirement.ty, &bindings);
            let value = crate::types::substitute(&requirement.value, &bindings);
            if base.is_unknown()
                || crate::types::contains_param(&base)
                || (value.is_unknown() && !crate::types::is_null_shaped(&value))
                || crate::types::contains_param(&value)
            {
                continue;
            }
            let path: Vec<crate::types::PlacePathStep> = requirement
                .path
                .iter()
                .map(|projection| match projection {
                    crate::ir::Projection::Field(name) => {
                        crate::types::PlacePathStep::Field(name.clone())
                    }
                    crate::ir::Projection::Element(_)
                    | crate::ir::Projection::ElementTemp(_) => {
                        crate::types::PlacePathStep::Element
                    }
                })
                .collect();
            let destination =
                crate::types::project_place_path(&base, &path, &sema.table);
            if destination.is_unknown() || crate::types::contains_param(&destination) {
                continue;
            }
            if crate::types::is_null_shaped(&requirement.value) {
                if !destination.nullable {
                    if reported_use.insert((
                        requirement.function.clone(),
                        requirement.store_span.start,
                        "E10012",
                    )) {
                        diagnostics.push(
                            Diagnostic::error(
                                "E10012",
                                "'null' is not allowed here",
                                requirement.store_span,
                            )
                            .with_help("declare the type `T?` or avoid the empty state"),
                        );
                    }
                    break;
                }
                continue;
            }
            if !crate::types::implicit_conversion(&value, &destination) {
                let site = (
                    requirement.function.clone(),
                    requirement.store_span.start,
                    requirement.store_span.end,
                );
                if reported_conversion.insert(site) {
                    diagnostics.push(
                        Diagnostic::error(
                            "E10200",
                            format!(
                                "cannot convert '{}' to '{}' implicitly",
                                value.display(),
                                destination.display()
                            ),
                            requirement.store_span,
                        )
                        .with_help("use an explicit cast `T(expr)`"),
                    );
                }
                break;
            }
            if is_view_like(&destination) && !requirement.superseded {
                let subject = requirement.frame_owner.as_deref();
                if requirement.caller_slot {
                    if let Some(subject) = subject {
                        if reported_use.insert((
                            requirement.function.clone(),
                            requirement.store_span.start,
                            "E10030",
                        )) {
                            diagnostics.push(opaque_escape(subject, requirement.store_span));
                        }
                        break;
                    }
                }
                if let (Some(subject), Some(span)) = (subject, requirement.return_span) {
                    let site = (
                        requirement.function.clone(),
                        span.start,
                        span.end,
                    );
                    if reported_escape.insert(site) {
                        diagnostics.push(opaque_escape(subject, span));
                    }
                    break;
                }
                if let Some((span, subject)) = requirement.late_uses.first() {
                    if reported_use.insert((
                        requirement.function.clone(),
                        span.start,
                        "E10030",
                    )) {
                        diagnostics.push(opaque_escape(subject, *span));
                    }
                    break;
                }
                if let Some((span, subject)) = requirement.invalidations.first() {
                    if reported_use.insert((
                        requirement.function.clone(),
                        span.start,
                        "E10031",
                    )) {
                        diagnostics.push(
                            Diagnostic::error(
                                "E10031",
                                format!(
                                    "cannot modify '{}' while it is shared-borrowed",
                                    subject.trim_matches('\'')
                                ),
                                *span,
                            )
                            .with_help("end the shared borrow first"),
                        );
                    }
                    break;
                }
                if let Some((span, subject)) = requirement.moved_owner.first() {
                    if reported_use.insert((
                        requirement.function.clone(),
                        span.start,
                        "E10022",
                    )) {
                        diagnostics.push(
                            Diagnostic::error(
                                "E10022",
                                format!(
                                    "cannot move '{}' while it is borrowed",
                                    subject.trim_matches('\'')
                                ),
                                *span,
                            )
                            .with_help("end the borrow first"),
                        );
                    }
                    break;
                }
                continue;
            }
            if is_view_like(&destination) {
                // A replaced view keeps only its owned-source judgment.
                continue;
            }
            // An owning destination keeps the concrete move judgment: a
            // non-take parameter cannot be moved, and a later read of a moved
            // local is the moved-value error.
            if requirement.source_param {
                diagnostics.push(
                    Diagnostic::error(
                        "E10034",
                        format!(
                            "cannot release or store borrowed parameter '{}'",
                            requirement
                                .source_name
                                .as_deref()
                                .unwrap_or("the source")
                        ),
                        requirement.store_span,
                    )
                    .with_help(
                        "declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt",
                    ),
                );
                break;
            }
            // A conditional's arms are separate sources. An owning
            // destination keeps the concrete arm judgment: every non-take
            // lend arm reports E10034, and a deeper projected arm reports
            // E10026. A view destination returned before this point, so all
            // its arms stay widened copies.
            let mut arm_reported = false;
            for source in &requirement.source_arms {
                if source.param {
                    diagnostics.push(
                        Diagnostic::error(
                            "E10034",
                            format!(
                                "cannot release or store borrowed parameter '{}'",
                                source.name
                            ),
                            requirement.store_span,
                        )
                        .with_help(
                            "declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt",
                        ),
                    );
                    arm_reported = true;
                }
            }
            if arm_reported {
                break;
            }
            for source in &requirement.source_arms {
                let top_level = source.path.len() == 1
                    && matches!(
                        source.path.first(),
                        Some(crate::ir::Projection::Field(_))
                    );
                if !source.path.is_empty() && !top_level {
                    diagnostics.push(
                        Diagnostic::error(
                            "E10026",
                            "partial moves are limited to top-level fields in v1",
                            requirement.store_span,
                        )
                        .with_help("move the whole value instead"),
                    );
                    arm_reported = true;
                }
            }
            if arm_reported {
                break;
            }
            let top_level_field = requirement.source_path.len() == 1
                && matches!(
                    requirement.source_path.first(),
                    Some(crate::ir::Projection::Field(_))
                );
            if !requirement.source_path.is_empty() && !top_level_field {
                diagnostics.push(
                    Diagnostic::error(
                        "E10026",
                        "partial moves are limited to top-level fields in v1",
                        requirement.store_span,
                    )
                    .with_help("move the whole value instead"),
                );
                break;
            }
            if let Some(span) = requirement.later_uses.first() {
                if !reported_use.insert((
                    requirement.function.clone(),
                    span.start,
                    "E10020",
                )) {
                    break;
                }
                let name = requirement
                    .source_name
                    .as_deref()
                    .unwrap_or("the source");
                diagnostics.push(
                    Diagnostic::error(
                        "E10020",
                        format!("'{name}' was moved and cannot be used"),
                        *span,
                    )
                    .with_help("re-initialize the binding or use a view"),
                );
                break;
            }
        }
    }
}

/// The escape report for one stored view whose backing dies with the frame.
fn opaque_escape(subject: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        "E10030",
        format!("borrow of {subject} does not live long enough"),
        span,
    )
    .with_help("finish the use before the owner's scope ends")
}

/// Reports the duplicability duty when an instance substitutes an owning type.
///
/// A generic body that stores a maybe-owning value and then uses the source
/// again is accepted only when the instance's type argument is duplicable. The
/// audit substitutes each recorded argument into the binding's declared type
/// and reports at the conflicting use for the first owning instantiation: the
/// moved-value code (`E10020`) for a whole-binding move, and the partial-move
/// code (`E10025`) when only a member path moved and the use names the whole
/// binding, matching the concrete spelling's two judgments.
/// True when a projection selects a view member that the opaque lowering
/// cannot emit through a type parameter.
///
/// A `.ptr`/`.len` member or an element of a view is not lowered through an
/// opaque type; the conservative duty rejects the store instead of emitting
/// invalid C.
fn is_view_member_projection(
    sema: &SemaOutput,
    root: &Ty,
    path: &[crate::types::PlacePathStep],
) -> bool {
    let mut selected = root.clone();
    for step in path {
        while let TyKind::Raw(inner) = &selected.kind {
            selected = (**inner).clone();
        }
        match step {
            crate::types::PlacePathStep::Field(field)
                if field == "ptr" || field == "len" =>
            {
                // Only a view's intrinsic member is unlowerable; a user field
                // named `len` or `ptr` on an ordinary value is copied. A
                // structural `.len` on an array is a duplicable count.
                if is_view(&selected) {
                    return true;
                }
                // A structural `.len` through an opaque type parameter is not
                // folded by the emitter (the projected base type is unknown),
                // so the conservative duty rejects it. A user field named
                // `len` on an ordinary value is a copy.
                if field == "len" && matches!(selected.kind, TyKind::Array { .. }) {
                    return true;
                }
            }
            crate::types::PlacePathStep::Element => {
                if is_view(&selected) {
                    return true;
                }
                // An unresolved or still-generic base stays conservative.
                if selected.is_unknown() || crate::types::contains_param(&selected) {
                    return true;
                }
            }
            _ => {}
        }
        let next = crate::types::project_place_path(
            &selected,
            std::slice::from_ref(step),
            &sema.table,
        );
        if next.is_unknown() {
            return true;
        }
        selected = next;
    }
    false
}

/// True when a moved or selected projection reaches an owned value.
///
/// The projection resolver stops at a type it does not model. A view's
/// intrinsic `.ptr`/`.len` member and an element of a view reach no owned
/// value, so they stay copies; any other unresolved projection falls back to
/// the owning root, as the concrete spelling would.
fn projected_type(
    sema: &SemaOutput,
    root: &Ty,
    path: &[crate::types::PlacePathStep],
) -> Ty {
    let mut selected = root.clone();
    for step in path {
        while let TyKind::Raw(inner) = &selected.kind {
            selected = (**inner).clone();
        }
        // A structural `.len` on an array is a duplicable count.
        if matches!(
            step,
            crate::types::PlacePathStep::Field(field) if field == "len"
        ) && matches!(selected.kind, TyKind::Array { .. })
        {
            selected = Ty::usize_ty();
            continue;
        }
        // A view member (`.ptr`, `.len`, or an element of a view) is not
        // lowered through an opaque type; the conservative duty keeps the
        // store rejected instead of emitting invalid C.
        if matches!(step, crate::types::PlacePathStep::Element) && is_view(&selected) {
            return root.clone();
        }
        let next = crate::types::project_place_path(
            &selected,
            std::slice::from_ref(step),
            &sema.table,
        );
        if next.is_unknown() {
            return root.clone();
        }
        selected = next;
    }
    selected
}

fn check_duplicable_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    let mut reported: BTreeSet<(String, String, u32, u32)> = BTreeSet::new();
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
            let bound = crate::types::instance_bindings(&params, &instance.args);
            let substituted = crate::types::substitute(declared, &bound);
            // The judgment follows every conflicting moved path. An empty
            // path is a whole-binding move and judges the whole substituted
            // type; a projected path judges the moved member.
            let owns = requirement.paths.iter().any(|path| {
                is_view_member_projection(sema, &substituted, path)
                    || !sema.table.duplicable(&projected_type(sema, &substituted, path))
            });
            if !owns {
                continue;
            }
            if !reported.insert((
                requirement.function.clone(),
                requirement.binding.clone(),
                requirement.span.start,
                requirement.span.end,
            )) {
                continue;
            }
            if requirement.partial {
                diagnostics.push(
                    Diagnostic::error(
                        "E10025",
                        format!("'{binding_name}' is partially moved and cannot be used as a whole"),
                        requirement.span,
                    )
                    .with_help("use the remaining fields or re-initialize the struct"),
                );
            } else {
                diagnostics.push(
                    Diagnostic::error(
                        "E10020",
                        format!("'{binding_name}' was moved and cannot be used"),
                        requirement.span,
                    )
                    .with_help("re-initialize the binding or use a view"),
                );
            }
            // One duty per function and binding: the first owning instance
            // reports it.
            break;
        }
    }
}

/// Reports `E10026` when a recorded borrowed-element move owns at its instance.
///
/// A `for (borrow e in xs)` element alias never owns its element, so a
/// value-consuming move through it is the v1 element-move limit in the
/// concrete spelling. The body records the duty while the moved place's type
/// is still a parameter or unresolved; the audit substitutes each recorded
/// instance's type arguments and projects the recorded path: a duplicable or
/// view-typed substitution copies and stays accepted, while an owning one
/// reports the same limit the concrete spelling reports. One duty reports
/// once — the first owning instance.
fn check_element_move_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for requirement in mono.element_move_requirements() {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let bound = crate::types::instance_bindings(&params, &instance.args);
            let base = crate::types::substitute(&requirement.ty, &bound);
            let moved = projected_type(sema, &base, &requirement.path);
            if moved.is_unknown()
                || crate::types::contains_param(&moved)
                || sema.table.duplicable(&moved)
            {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10026",
                    "partial moves are limited to top-level fields in v1",
                    requirement.span,
                )
                .with_help("move the whole value instead"),
            );
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
        ValueKind::DiscardThen { effects, value } => {
            check_value_fills(effects, params, args, sema, span, diagnostics);
            check_value_fills(value, params, args, sema, span, diagnostics);
        }
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

/// Reports the loop's judgment when a recorded opaque iterable's substitution
/// cannot be bound.
///
/// The declaration accepts an opaque iterable because its instance may be a
/// fixed array or a slice; the audit substitutes each recorded instance's type
/// arguments. A substitution that is not an array has no element sequence and
/// reports `E10043`; a non-borrow copy of a non-duplicable element reports
/// `E10040`, matching the concrete spelling. Both report once — the first
/// offending instance — at the loop's span. A substitution that still mentions
/// a parameter is judged at its own instance, and an unresolved one keeps the
/// declaration's accepted path.
fn check_iterable_requirements(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for requirement in mono.iterable_requirements() {
        let Some(Callee::User(decl)) = program.callee(&requirement.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != requirement.function {
                continue;
            }
            let substituted = substitute_instance(&requirement.ty, &params, &instance.args);
            if crate::types::contains_param(&substituted) || substituted.is_unknown() {
                continue;
            }
            if !matches!(substituted.kind, TyKind::Array { .. }) {
                diagnostics.push(
                    Diagnostic::error(
                        "E10043",
                        format!("type '{}' cannot be iterated", substituted.display()),
                        requirement.span,
                    )
                    .with_help("iterate a fixed array or a slice"),
                );
                break;
            }
            let element = substituted.iterated_element();
            if !requirement.borrow && !sema.table.duplicable(&element) {
                diagnostics.push(
                    Diagnostic::error(
                        "E10040",
                        format!(
                            "element loop requires 'borrow' for non-duplicable '{}'",
                            element.display()
                        ),
                        requirement.span,
                    )
                    .with_help("write `for (borrow e in xs)`"),
                );
                break;
            }
        }
    }
}

/// Reports `E10212` when an opaque projection's substituted base has no such
/// member.
///
/// The declaration accepts a value-form member read on a bare type parameter
/// because the instance decides: the substituted type resolves the member. An
/// instance whose type has no such member would emit an unresolved member, so
/// the audit reports the unknown member once — the first offending instance —
/// at the read's span.
fn check_opaque_projections(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for projection in &sema.opaque_projections {
        let Some(Callee::User(decl)) = program.callee(&projection.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != projection.function {
                continue;
            }
            let bound = crate::types::instance_bindings(&params, &instance.args);
            let substituted = crate::types::substitute(&projection.ty, &bound);
            if crate::types::contains_param(&substituted) || substituted.is_unknown() {
                continue;
            }
            if projection_resolves(sema, &substituted, &projection.name) {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10212",
                    format!(
                        "'{}' is not a member of '{}'",
                        projection.name,
                        substituted.display()
                    ),
                    projection.span,
                )
                .with_help("check the declaration or the spelling"),
            );
            break;
        }
    }
}

/// Reports `E10216` when an opaque index exceeds the substituted row extent.
///
/// The declaration accepts an index on a bare type parameter because the
/// instance supplies the row type: a constant index past the substituted
/// extent reports the frozen range error once — the first offending instance —
/// at the read's span, exactly as the concrete spelling does. A run-time
/// index carries no constant, so its bounds judgment is the emitter's guard on
/// the substituted row.
fn check_opaque_indexes(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for index in &sema.opaque_indexes {
        let Some(Callee::User(decl)) = program.callee(&index.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != index.function {
                continue;
            }
            let bound = crate::types::instance_bindings(&params, &instance.args);
            let substituted = crate::types::substitute(&index.ty, &bound);
            if crate::types::contains_param(&substituted) || substituted.is_unknown() {
                continue;
            }
            // A possibly-null substituted base is not a valid index base: the
            // concrete spelling reports the null rule at the base's own span.
            if substituted.nullable {
                if let Some(name) = &index.base {
                    diagnostics.push(
                        Diagnostic::error(
                            "E10010",
                            format!("value '{name}' may be null here"),
                            index.span,
                        )
                        .with_help("check `!= null` before the use"),
                    );
                    break;
                }
            }
            match &substituted.kind {
                // A fixed first extent folds a constant index exactly as the
                // concrete spelling does; a run-time index is the emitter's
                // guard on the substituted row.
                TyKind::Array { element, extents } => {
                    let Some(Some(extent)) = extents.first() else {
                        continue;
                    };
                    let Some(value) = index.constant else {
                        continue;
                    };
                    if value < 0 || value >= *extent as i128 {
                        diagnostics.push(
                            Diagnostic::error(
                                "E10216",
                                format!(
                                    "index {value} is out of range for '{}[{extent}]'",
                                    element.display()
                                ),
                                index.index_span,
                            )
                            .with_help("fix the index or the extent"),
                        );
                        break;
                    }
                }
                // A string view, a `cstring`, and a `raw` pointer are all
                // indexable, exactly as the concrete spelling judges; none
                // carries a fixed constant extent, so no range rule applies.
                TyKind::Str | TyKind::CString | TyKind::StaticStr | TyKind::Raw(_) => {}
                // Every other substituted kind has no index surface: the
                // concrete spelling reports the frozen non-indexable rule at
                // the base.
                _ => {
                    diagnostics.push(
                        Diagnostic::error(
                            "E10209",
                            format!("type '{}' cannot be indexed", substituted.display()),
                            index.span,
                        )
                        .with_help("arrays, slices, and `str` are indexable"),
                    );
                    break;
                }
            }
        }
    }
}

/// Reports `E10200` when an opaque member read cannot convert at its use.
///
/// The declaration records the read's destination because the member's type
/// is unresolved there; at the instance the member resolves through the
/// matching projection and the concrete conversion rule applies. The view
/// pointer member's sanctioned `cstring` conversion stays accepted exactly as
/// the concrete spelling accepts it. One use reports once — the first
/// offending instance.
fn check_opaque_member_uses(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for usage in &sema.opaque_member_uses {
        let Some(Callee::User(decl)) = program.callee(&usage.function) else {
            continue;
        };
        // The projection records the read's base span while the use records
        // the whole read's span; both begin where the base expression begins,
        // so the start identifies the site.
        let Some(projection) = sema.opaque_projections.iter().find(|projection| {
            projection.function == usage.function
                && projection.name == usage.name
                && projection.span.start == usage.span.start
        }) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != usage.function {
                continue;
            }
            let bound = crate::types::instance_bindings(&params, &instance.args);
            let base = crate::types::substitute(&projection.ty, &bound);
            if crate::types::contains_param(&base) || base.is_unknown() {
                continue;
            }
            let Some(member) = resolved_member_ty(sema, &base, &usage.name) else {
                continue;
            };
            let expected = crate::types::substitute(&usage.expected, &bound);
            if crate::types::contains_param(&expected) || expected.is_unknown() {
                continue;
            }
            if crate::types::implicit_conversion(&member, &expected) {
                // The conversion is legal, but a nullable member at a
                // non-nullable destination keeps the concrete twin's
                // possible-null rule. The report names the root binding and
                // lands where the read is written; a read rooted at a call or
                // a literal has no binding the concrete spelling could name,
                // so it keeps the conversion path only.
                if crate::types::nullable_value_needs_narrowing(&member, &expected) {
                    if let Some(name) = &usage.base {
                        diagnostics.push(
                            Diagnostic::error(
                                "E10010",
                                format!("value '{name}' may be null here"),
                                usage.span,
                            )
                            .with_help("check `!= null` before the use"),
                        );
                        break;
                    }
                }
                continue;
            }
            if usage.sanctioned
                && crate::types::converts_view_pointer_member(&base, &usage.name, &expected)
            {
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    "E10200",
                    format!(
                        "cannot convert '{}' to '{}' implicitly",
                        member.display(),
                        expected.display()
                    ),
                    usage.span,
                )
                .with_help("use an explicit cast `T(expr)`"),
            );
            break;
        }
    }
}

/// The type of a member resolved on a substituted base.
///
/// A struct field, a field reached through a `raw` pointer, and a view's ABI
/// member all resolve through the shared rules; every other kind has no
/// member surface.
fn resolved_member_ty(sema: &SemaOutput, base: &Ty, name: &str) -> Option<Ty> {
    match &base.kind {
        TyKind::Struct(..) => sema.table.instance_field(base, name),
        TyKind::Raw(inner) => resolved_member_ty(sema, inner, name),
        _ => crate::types::view_member_ty(base, name),
    }
}

/// The concrete type of a conditional arm that names an opaque member read.
///
/// The declaration types the arm `Unknown` because its base is an opaque
/// value; the projection recorded at the same read carries the base type, so
/// the instance substitutes it and resolves the member exactly as the
/// member-use audit does. Every other unresolved arm keeps its unknown type,
/// which the arm rule accepts as the sibling's type.
fn resolved_conditional_arm(
    sema: &SemaOutput,
    function: &str,
    arm_ty: &Ty,
    span: Span,
    bindings: &BTreeMap<String, Ty>,
) -> Ty {
    if !arm_ty.is_unknown() {
        return arm_ty.clone();
    }
    let Some(projection) = sema.opaque_projections.iter().find(|projection| {
        projection.function == function && projection.span.start == span.start
    }) else {
        return arm_ty.clone();
    };
    let base = crate::types::substitute(&projection.ty, bindings);
    if crate::types::contains_param(&base) || base.is_unknown() {
        return arm_ty.clone();
    }
    resolved_member_ty(sema, &base, &projection.name).unwrap_or_else(|| arm_ty.clone())
}

/// Reports the conditional rule on an opaque site's substituted arms.
///
/// The declaration accepts a conditional whose arms still name a type
/// parameter because the substitution decides whether they agree. Each
/// concrete instantiation applies the rule the declaration applies to concrete
/// arms: a null arm at a non-nullable sibling or destination reports the null
/// rule at the null, a pair the ordinary widenings cannot join reports the
/// conversion at the else arm, and the settled result faces the destination the
/// concrete twin's store faces. One site reports once — its first offending
/// instance.
fn check_opaque_conditionals(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for conditional in &sema.opaque_conditionals {
        let Some(Callee::User(decl)) = program.callee(&conditional.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != conditional.function {
                continue;
            }
            let bindings = crate::types::instance_bindings(&params, &instance.args);
            let then_ty = crate::types::substitute(&conditional.then_ty, &bindings);
            let else_ty = crate::types::substitute(&conditional.else_ty, &bindings);
            // An arm that names an opaque member read has no type at the
            // declaration; the matching projection resolves it through the
            // instance before the arm rule and the null rule judge it.
            let then_ty = resolved_conditional_arm(
                sema,
                &conditional.function,
                &then_ty,
                conditional.then_span,
                &bindings,
            );
            let else_ty = resolved_conditional_arm(
                sema,
                &conditional.function,
                &else_ty,
                conditional.else_span,
                &bindings,
            );
            let expected = conditional
                .expected
                .as_ref()
                .map(|expected| crate::types::substitute(expected, &bindings));
            // A destination that is still open defers the judgment to a later
            // instance, exactly as an open arm does.
            let open_expected = expected
                .as_ref()
                .is_some_and(|ty| crate::types::contains_param(ty) || ty.is_void());
            if open_expected {
                continue;
            }
            let expected = expected.as_ref();
            // A null arm faces the destination, or the sibling when the
            // conditional has no destination of its own. The instance keeps
            // the concrete twin's order: the null report first, then the
            // settled result's conversion. Both arms are judged, and either
            // report ends the site so a second instance cannot repeat it.
            if !conditional.null_reported {
                let then_reported = null_conditional_report(
                    &then_ty,
                    expected.unwrap_or(&else_ty),
                    conditional.then_span,
                    diagnostics,
                );
                let else_reported = null_conditional_report(
                    &else_ty,
                    expected.unwrap_or(&then_ty),
                    conditional.else_span,
                    diagnostics,
                );
                if then_reported || else_reported {
                    break;
                }
            }
            // The settled result follows the declaration's arm rule, then the
            // destination conversion the concrete twin's store applies.
            if crate::types::contains_param(&then_ty)
                || crate::types::contains_param(&else_ty)
            {
                continue;
            }
            let Some(result) = crate::types::conditional_result(&then_ty, &else_ty) else {
                diagnostics.push(
                    Diagnostic::error(
                        "E10200",
                        format!(
                            "cannot convert '{}' to '{}' implicitly",
                            else_ty.display(),
                            then_ty.display()
                        ),
                        conditional.else_span,
                    )
                    .with_help("use an explicit cast `T(expr)`"),
                );
                break;
            };
            if let Some(expected) = expected {
                if !crate::types::implicit_conversion(&result, expected) {
                    diagnostics.push(
                        Diagnostic::error(
                            "E10200",
                            format!(
                                "cannot convert '{}' to '{}' implicitly",
                                result.display(),
                                expected.display()
                            ),
                            conditional.span,
                        )
                        .with_help("use an explicit cast `T(expr)`"),
                    );
                    break;
                }
            }
        }
    }
}

/// Reports the null rule for one substituted arm, returning whether it did.
///
/// The arm reports only when it is the null shape and the context — the
/// destination or the sibling — is concrete and non-nullable; a nullable or
/// still-open context accepts it, exactly as the concrete spelling does.
fn null_conditional_report(
    arm: &Ty,
    context: &Ty,
    span: Span,
    diagnostics: &mut Diagnostics,
) -> bool {
    if !crate::types::null_arm_needs_report(arm, context) {
        return false;
    }
    diagnostics.push(
        Diagnostic::error("E10012", "'null' is not allowed here", span)
            .with_help("declare the type `T?` or avoid the empty state"),
    );
    true
}

/// Reports the frozen judgment for a value whose conversion was deferred.
///
/// The declaration records a value whose type mentions a type parameter or is
/// the null shape; at each instance the resolved source and destination face
/// the ordinary judgment — the null shape at a concrete destination reports
/// the null rule, and any other pair that the frozen table does not convert
/// implicitly reports the conversion. One site reports once, at its first
/// offending instance; a legal instance reports nothing.
fn check_deferred_conversions(
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
) {
    for conversion in &sema.opaque_conversions {
        let Some(Callee::User(decl)) = program.callee(&conversion.function) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for instance in mono.instances() {
            if instance.function != conversion.function {
                continue;
            }
            // The instance's bindings include the element placeholder
            // (`#element of T`), so a source written as an element of the
            // opaque parameter resolves to the substituted argument's element
            // before the conversion is judged, exactly as the concrete read
            // would report it.
            let bindings = crate::types::instance_bindings(&params, &instance.args);
            let expected = crate::types::substitute(&conversion.expected, &bindings);
            if crate::types::contains_param(&expected) || expected.is_void() {
                continue;
            }
            // A nullable borrow slot's report belongs to the out-slot place
            // duty; every other instance keeps the ordinary conversion report.
            if conversion.borrow_slot && expected.nullable {
                continue;
            }
            let source = crate::types::substitute(&conversion.source, &bindings);
            let Some(report) = deferred_conversion_report(
                &source,
                &expected,
                conversion.span,
                conversion.literal.as_ref(),
            ) else {
                continue;
            };
            diagnostics.push(report);
            break;
        }
    }
}

/// The frozen report for one deferred value at its resolved destination.
///
/// `None` means the pair is legal: the null shape fits a nullable destination,
/// an unresolved source keeps the accepted path, a numeric literal takes a
/// scalar destination's type through the frozen widenings, and every other
/// pair converts implicitly. The null shape reports the null rule because it
/// has no concrete value surface to convert; every other rejected pair reports
/// the frozen conversion.
fn deferred_conversion_report(
    source: &Ty,
    expected: &Ty,
    span: Span,
    literal: Option<&ConversionLiteral>,
) -> Option<Diagnostic> {
    if crate::types::is_null_shaped(source) {
        if expected.nullable || expected.is_unknown() {
            return None;
        }
        return Some(
            Diagnostic::error("E10012", "'null' is not allowed here", span)
                .with_help("declare the type `T?` or avoid the empty state"),
        );
    }
    if source.is_unknown() || crate::types::contains_param(source) {
        return None;
    }
    // A literal takes the destination's type exactly where its concrete twin
    // converts, so the written value judges the resolved scalar.
    if let Some(literal) = literal {
        return literal_destination_report(source, expected, span, literal);
    }
    if crate::types::implicit_conversion(source, expected) {
        return None;
    }
    Some(conversion_rejected(source, expected, span))
}

/// The frozen report for one literal at its resolved destination.
///
/// An integer literal at an integer scalar must fit — a value outside the
/// destination's range reports the literal-fit rule, exactly as the concrete
/// spelling does — a float literal widens to `f32`/`f64`, and a cross-kind
/// pair reports the frozen conversion.
fn literal_destination_report(
    source: &Ty,
    expected: &Ty,
    span: Span,
    literal: &ConversionLiteral,
) -> Option<Diagnostic> {
    let TyKind::Scalar(scalar) = &expected.kind else {
        return Some(conversion_rejected(source, expected, span));
    };
    match literal {
        ConversionLiteral::Int(value) => {
            if !crate::types::is_integer_scalar(*scalar) {
                return Some(conversion_rejected(source, expected, span));
            }
            if crate::types::int_fits(*value, *scalar) {
                return None;
            }
            Some(
                Diagnostic::error(
                    "E10202",
                    format!("literal does not fit '{}'", expected.display()),
                    span,
                )
                .with_help("use a wider type or a hex/binary form"),
            )
        }
        ConversionLiteral::Float(_) => {
            if crate::types::is_float_scalar(*scalar) {
                None
            } else {
                Some(conversion_rejected(source, expected, span))
            }
        }
    }
}

/// The frozen conversion report for one rejected pair.
fn conversion_rejected(source: &Ty, expected: &Ty, span: Span) -> Diagnostic {
    Diagnostic::error(
        "E10200",
        format!(
            "cannot convert '{}' to '{}' implicitly",
            source.display(),
            expected.display()
        ),
        span,
    )
    .with_help("use an explicit cast `T(expr)`")
}

/// True when a value-form member resolves on the substituted type.
///
/// The dispatch mirrors the value-form checker's own rules: a struct field
/// (which wins over everything else, including a field named `len`), an enum
/// member, a member reached through a `raw` pointer to a struct, and the
/// structural `.len` a fixed array, a slice, a `str`, or a `static str` value
/// carries. Every other kind has no member surface.
fn projection_resolves(sema: &SemaOutput, ty: &Ty, name: &str) -> bool {
    match &ty.kind {
        TyKind::Struct(..) => sema.table.instance_field(ty, name).is_some(),
        // A value-form enum member has no member surface at any substitution:
        // the frozen table spells the member `Type.MEMBER`, so the instance
        // reports the unknown member exactly as the concrete read would.
        TyKind::Enum(_) => false,
        TyKind::Raw(inner) => match &inner.kind {
            TyKind::Struct(..) => sema.table.instance_field(inner, name).is_some(),
            TyKind::Raw(_) => projection_resolves(sema, inner, name),
            _ => false,
        },
        // A view's ABI members are its pointer and its element count; only a
        // dynamic extent is a view, so a fixed array keeps its base
        // unknown-member report.
        TyKind::Array { extents, .. } => {
            name == "len" || (name == "ptr" && extents.iter().any(|extent| extent.is_none()))
        }
        TyKind::Str | TyKind::StaticStr => name == "len" || name == "ptr",
        _ => false,
    }
}

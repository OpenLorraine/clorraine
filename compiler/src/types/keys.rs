//! Symbol keys for types.
//!
//! The keys here name types in C symbols and in the monomorphization identity.
//! They live with the type model rather than with the emitter because both the
//! ownership stage and codegen must derive the same key for an instance: the
//! ownership stage stamps a closure's enclosing instance key at recording time,
//! and the emitter builds the matching callable and symbol names from it.

use super::{ClosureTy, Ty, TyKind};
use crate::ast::Mode;

/// The stable key of a closure type.
///
/// The site id alone identifies a shared closure. A closure specialized for an
/// enclosing instance carries that instance's key too, so two specializations
/// of one site never share a C type or a symbol.
pub(crate) fn closure_key(closure: &ClosureTy) -> String {
    match &closure.instance {
        Some(key) => format!("closure_{}__{key}", closure.id),
        None => format!("closure_{}", closure.id),
    }
}

/// A stable key naming a type in a symbol.
///
/// A fixed array renders every extent, so `i32[2]` and `i32[3]` never share a
/// key. An array with an unknown extent is a slice and renders the slice
/// marker over its element: a scalar view keys as `slice_i32`, and a view of
/// rows keys by the whole row, so `i32[][3]` keys as `slice_arr3_i32`. The two
/// forms cannot collide with each other.
///
/// A function type renders its whole signature — every parameter's type key
/// (with its mode, because a `borrow` parameter reaches the callee as a
/// pointer and mode-differing signatures are different types) and the return
/// key, as `fn_<params>__<return>` — so two signatures never share a key. The
/// `_` separators and the `borrow_`/`take_`/`const_` prefixes are fixed
/// spellings, so a crafted user identifier can textually collide with a
/// signature (`struct borrow_i32` against `func(borrow i32)`); like the `Q`
/// marker this is fail-closed — a collision produces a redefinition the strict
/// build rejects, never a runnable wrong program.
///
/// A nullable type carries a `Q` suffix. Nullability changes the C
/// representation of a borrow parameter and the identity of an instance, so
/// `String` and `String?` never share a key; the suffix applies recursively
/// with the arguments, so the instance argument `Wrap<String?>` keys as
/// `Wrap__StringQ`. A user identifier may itself end in `Q`, in which case its
/// spelling collides with the marker — a known limitation shared with the
/// function-instance symbols.
pub(crate) fn type_key(ty: &Ty) -> String {
    let base = match &ty.kind {
        TyKind::Void => "void".to_owned(),
        TyKind::Scalar(scalar) => super::scalar_name(*scalar).to_owned(),
        TyKind::Str => "str".to_owned(),
        TyKind::StaticStr => "static_str".to_owned(),
        TyKind::CString => "cstring".to_owned(),
        TyKind::Raw(inner) => format!("raw_{}", type_key(inner)),
        TyKind::Handle(name) | TyKind::Enum(name) | TyKind::Param(name) => name.clone(),
        TyKind::Struct(name, args) => {
            if args.is_empty() {
                name.clone()
            } else {
                let rendered: Vec<String> = args.iter().map(type_key).collect();
                format!("{name}__{}", rendered.join("_"))
            }
        }
        TyKind::Array { element, extents } => {
            if extents.iter().all(Option::is_some) {
                let dims: Vec<String> = extents
                    .iter()
                    .filter_map(|size| size.map(|value| value.to_string()))
                    .collect();
                format!("arr{}_{}", dims.join("x"), type_key(element))
            } else {
                format!("slice_{}", type_key(&view_element(ty)))
            }
        }
        TyKind::Func(signature) => {
            let params: Vec<String> = signature
                .params
                .iter()
                .map(|param| {
                    let key = type_key(&param.ty);
                    match param.mode {
                        Mode::None => key,
                        Mode::Const => format!("const_{key}"),
                        Mode::Borrow => format!("borrow_{key}"),
                        Mode::Take => format!("take_{key}"),
                    }
                })
                .collect();
            format!("fn_{}__{}", params.join("_"), type_key(&signature.returns))
        }
        TyKind::Closure(closure) => closure_key(closure),
        TyKind::Unknown => "unknown".to_owned(),
    };
    if ty.nullable {
        format!("{base}Q")
    } else {
        base
    }
}

/// The element a slice view exposes: the row for a multi-extent source.
///
/// A view's element is one step of iteration, exactly as an element loop binds
/// it: a slice over a multi-extent source exposes whole rows (`i32[][3]`
/// exposes `i32[3]`), while a single-extent view exposes its scalar. Both
/// spellings of a row view — the written `T[][M]` and a `T[]` whose element is
/// `T[M]` — yield the same element here, so the key, the typedef, the builder,
/// and the access spelling all agree.
pub(crate) fn view_element(ty: &Ty) -> Ty {
    ty.iterated_element()
}

/// The view type of an element, in its written `T[][M]` form.
///
/// A scalar element becomes `E[]`; a fixed row element folds its extents after
/// the omitted outer one, so a view of `i32[3]` rows reads `i32[][3]`, the
/// form the language spells. Building the canonical form at the view-builder
/// result sites keeps a view's recorded type and its diagnostic spelling the
/// same as a declared one.
pub(crate) fn view_type(element: Ty) -> Ty {
    match &element.kind {
        TyKind::Array { element: base, extents } if extents.iter().all(Option::is_some) => {
            let mut view_extents = vec![None];
            view_extents.extend(extents.iter().copied());
            Ty::array((**base).clone(), view_extents)
        }
        _ => Ty::array(element, vec![None]),
    }
}

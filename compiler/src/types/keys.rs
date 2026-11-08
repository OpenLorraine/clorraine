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

/// Reserved type-key spellings that a user component may not reuse.
const RESERVED_COMPONENTS: &[&str] = &[
    "void", "bool", "i8", "i16", "i32", "u8", "u16", "u32", "usize", "isize", "f32",
    "f64", "str", "cstring", "static_str", "unknown", "String",
];

/// Escapes one user-derived type component.
///
/// A common identifier keeps its spelling, so existing ABI keys stay stable.
/// An identifier that contains a fixed separator (`_`), the nullable marker
/// (`Q`), a non-identifier byte, or a reserved type spelling is prefixed with
/// `u<length>_`; an unescaped component can never start with that shape, and
/// the length makes the spelling injective. A type parameter is always prefixed
/// with `p<length>_`, so a parameter named like a user type cannot collide with
/// that type's key.
pub(crate) fn type_component(name: &str) -> String {
    let plain = !name.is_empty()
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric())
        && (!name.contains('Q') || name == "Q")
        && !RESERVED_COMPONENTS.contains(&name);
    if plain {
        name.to_owned()
    } else {
        format!("u{}_{}", name.len(), name)
    }
}

/// Escapes a generic parameter name, keeping it distinct from a user type.
fn param_component(name: &str) -> String {
    format!("p{}_{}", name.len(), name)
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
/// User components are escaped by [`type_component`] when they could collide
/// with a fixed spelling, so a crafted identifier cannot textually collide with
/// a signature (`struct fn_borrow_i32__i32` against `func(borrow i32)`).
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
        TyKind::Box(inner) => format!("Box__{}", type_key(inner)),
        TyKind::Handle(name) => {
            if name == "String" {
                name.clone()
            } else {
                type_component(name)
            }
        }
        TyKind::Enum(name) => type_component(name),
        TyKind::Param(name) => param_component(name),
        TyKind::Struct(name, args) => {
            let base = type_component(name);
            if args.is_empty() {
                base
            } else {
                let rendered: Vec<String> = args.iter().map(type_key).collect();
                format!("{base}__{}", rendered.join("_"))
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

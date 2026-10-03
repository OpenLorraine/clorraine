//! Symbol keys for types.
//!
//! The keys here name types in C symbols and in the monomorphization identity.
//! They live with the type model rather than with the emitter because both the
//! ownership stage and codegen must derive the same key for an instance: the
//! ownership stage stamps a closure's enclosing instance key at recording time,
//! and the emitter builds the matching callable and symbol names from it.

use super::{ClosureTy, Ty, TyKind};

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
/// marker instead; the two forms cannot collide with each other.
pub(crate) fn type_key(ty: &Ty) -> String {
    match &ty.kind {
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
                format!("slice_{}", slice_element_key(element))
            }
        }
        TyKind::Func(_) => "fn".to_owned(),
        TyKind::Closure(closure) => closure_key(closure),
        TyKind::Unknown => "unknown".to_owned(),
    }
}

/// The base element a slice view points at, with nested rows unwrapped.
///
/// A slice of a fixed row (`i32[3][]`) and a row slice written with its
/// extents after the marker (`i32[][3]`) describe the same view. Both flatten
/// to the same base element, so their `ClSlice_` names stay shared and a view
/// built by the `slice` intrinsic matches the declared typedef.
pub(crate) fn slice_base_element(element: &Ty) -> &Ty {
    let mut base = element;
    while let TyKind::Array { element, .. } = &base.kind {
        base = element;
    }
    base
}

/// The element key a slice's C spelling uses.
pub(crate) fn slice_element_key(element: &Ty) -> String {
    type_key(slice_base_element(element))
}

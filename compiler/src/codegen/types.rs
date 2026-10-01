//! C type surface and symbol naming for the emitter.
//!
//! The mapping is the codegen contract table: fixed-width typedefs, view
//! structs, contract-table handle types, user aggregates, and the FFI ABI. The
//! functions here are pure, so a caller can spell a type without touching the
//! output buffer.

use std::collections::BTreeMap;

use crate::abi;
use crate::ast::ScalarType;
use crate::types::{Ty, TyKind};

/// Replaces the dots of a dotted module path with underscores.
pub(super) fn mangle_module(module: &str) -> String {
    module.replace('.', "_")
}

/// The C spelling of a type in a value position.
pub(super) fn c_type(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Void => "void".to_owned(),
        TyKind::Scalar(scalar) => scalar_c_type(*scalar).to_owned(),
        TyKind::Str | TyKind::StaticStr => "ClStr".to_owned(),
        TyKind::CString => "char *".to_owned(),
        // A pointer to a row keeps the row's extents attached to the pointer:
        // `T (*)[M]` points at a whole `T[M]`, while `T *` points at one `T`.
        TyKind::Raw(inner) => pointer_to_array_type(inner)
            .unwrap_or_else(|| format!("{} *", c_type(inner))),
        TyKind::Handle(name) => handle_type(name),
        TyKind::Enum(name) => format!("Cl{name}"),
        TyKind::Struct(name, args) => {
            if args.is_empty() {
                format!("Cl{name}")
            } else {
                format!("Cl{}", type_key(ty))
            }
        }
        TyKind::Array { element, extents } => {
            if extents.iter().all(Option::is_some) {
                c_type(element)
            } else {
                format!("ClSlice_{}", type_key(element))
            }
        }
        TyKind::Func(signature) => func_pointer_type(signature),
        TyKind::Closure(closure) => closure_type(closure.id),
        TyKind::Param(name) => format!("Cl{name}"),
        TyKind::Unknown => "i32".to_owned(),
    }
}

/// The C spelling of a function-pointer type in a value position.
///
/// A `func` value is a C function pointer, so it can be stored in a field and
/// called through the field. An unnamed `ret (*)(params)` is a complete type
/// usable in casts and `sizeof`.
fn func_pointer_type(signature: &crate::types::FuncTy) -> String {
    function_declarator(
        &signature.returns,
        func_pointer_params(signature),
        "(*)",
        None,
    )
}

/// The spelling of a function-pointer declarator that names the pointer.
fn func_pointer_declarator(signature: &crate::types::FuncTy, name: &str) -> String {
    function_declarator(
        &signature.returns,
        func_pointer_params(signature),
        &format!("(*{name})"),
        None,
    )
}

/// The parameter declarations of a function pointer, without names.
///
/// A function-pointer type lists only each parameter's type, so the same
/// spelling fits a field, a cast, or a nested return. A `borrow` parameter
/// reaches the callee as a pointer to the value, exactly as the named-function
/// ABI spells a writable lend.
fn func_pointer_params(signature: &crate::types::FuncTy) -> Vec<String> {
    signature
        .params
        .iter()
        .map(|param| match param.mode {
            crate::ast::Mode::Borrow => declarator(&Ty::raw(param.ty.clone()), "").trim().to_owned(),
            _ => declarator(&param.ty, "").trim().to_owned(),
        })
        .collect()
}

/// Builds a full C function declarator with the shared ABI rules.
///
/// `name` is the declared name in place, so a nested declarator can be built
/// by passing an already-suffixed name: a function returning a function pointer
/// passes the pointer's own `name(params)` text. `params` are the rendered
/// parameter declarations and `env` is an optional leading environment
/// parameter (a capturing lambda's struct pointer). A fixed-array return
/// appends the hidden `__cl_ret` destination parameter and returns `void`,
/// exactly as the named-function ABI spells it; a function-typed return nests
/// the pointer declarator around the name.
pub(super) fn function_declarator(
    returns: &Ty,
    params: Vec<String>,
    name: &str,
    env: Option<String>,
) -> String {
    let mut params = params;
    if let Some(env) = env {
        params.insert(0, env);
    }
    if returns.is_fixed_array() {
        params.push(declarator(returns, "__cl_ret"));
        return format!("void {name}({})", parameter_list(params));
    }
    if let TyKind::Func(signature) = &returns.kind {
        let pointer = format!("(*{name}({}))", parameter_list(params));
        return function_declarator(
            &signature.returns,
            func_pointer_params(signature),
            &pointer,
            None,
        );
    }
    format!("{} {name}({})", c_type(returns), parameter_list(params))
}

/// Joins a rendered parameter list, spelling an empty list as `void`.
fn parameter_list(params: Vec<String>) -> String {
    if params.is_empty() {
        "void".to_owned()
    } else {
        params.join(", ")
    }
}

/// The C type of a handle: its contract ABI type, or an opaque `Cl<Name>`.
pub(super) fn handle_type(name: &str) -> String {
    if name == "String" {
        return "ClString".to_owned();
    }
    abi::contract_handle(name)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Cl{name}"))
}

/// The C extent suffix of a fixed array: `[M]`, `[M][K]`, or so on.
///
/// `None` for a non-array type or a slice, whose extent is a runtime length.
fn fixed_extent_suffix(row: &Ty) -> Option<String> {
    let TyKind::Array { extents, .. } = &row.kind else {
        return None;
    };
    if extents.iter().any(Option::is_none) {
        return None;
    }
    let mut suffix = String::new();
    for extent in extents {
        suffix.push_str(&format!("[{}]", extent.unwrap_or(0)));
    }
    Some(suffix)
}

/// The C spelling of a pointer-to-array type: `T (*)[M]` for a fixed row.
///
/// A row address (`raw T[M]`) needs the extent suffix attached to the pointer,
/// not to the element type; `T (*)[M]` is a pointer to a whole row, while
/// `T *` is a pointer to one element. `None` for a non-array type or a slice,
/// which keeps the ordinary pointer spelling.
pub(super) fn pointer_to_array_type(row: &Ty) -> Option<String> {
    let TyKind::Array { element, .. } = &row.kind else {
        return None;
    };
    Some(format!("{} (*){}", c_type(element), fixed_extent_suffix(row)?))
}

/// The C spelling of a pointer-to-array declarator that names the pointer:
/// `T (*name)[M]` for a fixed row. `None` when the inner type is not a fixed
/// array.
pub(super) fn pointer_to_array_declarator(row: &Ty, name: &str) -> Option<String> {
    let TyKind::Array { element, .. } = &row.kind else {
        return None;
    };
    Some(format!(
        "{} (*{name}){}",
        c_type(element),
        fixed_extent_suffix(row)?
    ))
}

/// The C type of a scalar in generated code.
pub(super) fn scalar_c_type(scalar: ScalarType) -> &'static str {
    match scalar {
        ScalarType::I8 => "i8",
        ScalarType::I16 => "i16",
        ScalarType::I32 => "i32",
        ScalarType::U8 => "u8",
        ScalarType::U16 => "u16",
        ScalarType::U32 => "u32",
        ScalarType::Usize => "usize",
        ScalarType::Isize => "isize",
        ScalarType::Bool => "bool",
        ScalarType::F32 => "float",
        ScalarType::F64 => "double",
    }
}

/// A declaration of `name` with type `ty`, handling array suffixes.
pub(super) fn declarator(ty: &Ty, name: &str) -> String {
    match &ty.kind {
        TyKind::Array { element, extents } if extents.iter().all(Option::is_some) => {
            let mut suffix = String::new();
            for extent in extents {
                suffix.push_str(&format!("[{}]", extent.unwrap_or(0)));
            }
            format!("{} {name}{suffix}", c_type(element))
        }
        TyKind::Raw(inner) => pointer_to_array_declarator(inner, name)
            .unwrap_or_else(|| format!("{} *{name}", c_type(inner))),
        TyKind::CString => format!("char *{name}"),
        TyKind::Func(signature) => func_pointer_declarator(signature, name),
        _ => format!("{} {name}", c_type(ty)),
    }
}

/// The initializer that puts a value in its empty state.
///
/// A droppable type's empty state is the all-zero pattern, so a hoisted
/// declaration starts safe and a later drop is a no-op.
pub(super) fn empty_state(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Handle(_) | TyKind::Raw(_) | TyKind::CString | TyKind::Func(_) => "NULL".to_owned(),
        TyKind::Str | TyKind::StaticStr | TyKind::Struct(..) | TyKind::Enum(_) => "{0}".to_owned(),
        TyKind::Array { .. } => "{0}".to_owned(),
        _ => "0".to_owned(),
    }
}

/// A stable key naming a type in a symbol.
pub(super) fn type_key(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Void => "void".to_owned(),
        TyKind::Scalar(scalar) => crate::types::scalar_name(*scalar).to_owned(),
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
        TyKind::Array { element, .. } => type_key(element),
        TyKind::Func(_) => "fn".to_owned(),
        TyKind::Closure(closure) => format!("closure_{}", closure.id),
        TyKind::Unknown => "unknown".to_owned(),
    }
}

/// The C struct name of a closure's two-word value.
pub(super) fn closure_type(id: usize) -> String {
    format!("ClClosure_{id}")
}

/// The scalar a type denotes, when it is a scalar.
pub(super) fn scalar_of(ty: &Ty) -> Option<ScalarType> {
    match ty.kind {
        TyKind::Scalar(scalar) => Some(scalar),
        _ => None,
    }
}

/// Records every slice element type reachable from `ty`.
pub(super) fn collect_slice_types(ty: &Ty, found: &mut BTreeMap<String, Ty>) {
    match &ty.kind {
        TyKind::Array { element, extents } => {
            // A slice over a generic parameter has no concrete element type to
            // declare; its concrete instances add the typedefs they need.
            if extents.iter().any(Option::is_none) && !element.is_param() {
                found.entry(type_key(element)).or_insert_with(|| (**element).clone());
            }
            collect_slice_types(element, found);
        }
        TyKind::Raw(inner) => collect_slice_types(inner, found),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_paths_mangle_dots_to_underscores() {
        assert_eq!(mangle_module("Demo.Copy"), "Demo_Copy");
        assert_eq!(mangle_module("Main"), "Main");
    }

    #[test]
    fn view_and_handle_types_use_their_emitted_names() {
        assert_eq!(
            c_type(&Ty {
                kind: TyKind::Str,
                nullable: false,
            }),
            "ClStr"
        );
        assert_eq!(c_type(&Ty::handle("String")), "ClString");
        assert_eq!(c_type(&Ty::handle("File")), "BPTR");
        assert_eq!(c_type(&Ty::handle("Buffer")), "ClBuffer");
        assert_eq!(c_type(&Ty::struct_("Point")), "ClPoint");
    }

    #[test]
    fn arrays_and_pointers_get_a_declarator_suffix() {
        let array = Ty::array(Ty::scalar(ScalarType::U32), vec![Some(2)]);
        assert_eq!(declarator(&array, "xs"), "u32 xs[2]");
        let slice = Ty::array(Ty::scalar(ScalarType::U8), vec![None]);
        assert_eq!(c_type(&slice), "ClSlice_u8");
        assert_eq!(declarator(&slice, "bytes"), "ClSlice_u8 bytes");
        let raw = Ty {
            kind: TyKind::Raw(Box::new(Ty::scalar(ScalarType::U32))),
            nullable: false,
        };
        assert_eq!(declarator(&raw, "p"), "u32 *p");
    }

    #[test]
    fn instantiated_struct_keys_compose_over_nested_arguments() {
        let pair_i32 = Ty::struct_with("Pair", vec![Ty::i32_ty()]);
        assert_eq!(type_key(&pair_i32), "Pair__i32");
        assert_eq!(c_type(&pair_i32), "ClPair__i32");

        let nested = Ty::struct_with("Pair", vec![pair_i32]);
        assert_eq!(type_key(&nested), "Pair__Pair__i32");
        assert_eq!(c_type(&nested), "ClPair__Pair__i32");

        assert_eq!(type_key(&Ty::struct_("Point")), "Point");
        assert_eq!(c_type(&Ty::struct_("Point")), "ClPoint");
    }

    #[test]
    fn empty_states_match_the_drop_contract() {
        assert_eq!(empty_state(&Ty::handle("File")), "NULL");
        assert_eq!(empty_state(&Ty::i32_ty()), "0");
        assert_eq!(
            empty_state(&Ty {
                kind: TyKind::Str,
                nullable: false,
            }),
            "{0}"
        );
    }
}

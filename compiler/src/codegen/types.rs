//! C type surface and symbol naming for the emitter.
//!
//! The mapping is the codegen contract table: fixed-width typedefs, view
//! structs, contract-table handle types, user aggregates, and the FFI ABI. The
//! functions here are pure spellings: the type table is passed in so a
//! function type can apply the named-parameter rule to a `borrow` parameter,
//! and no function touches the output buffer.

use std::collections::BTreeMap;

use crate::abi;
use crate::ast::{Mode, ScalarType};
use crate::types::{contains_param, ClosureTy, Ty, TyKind, TypeTable};

pub(super) use crate::types::{type_key, view_element};

/// Replaces the dots of a dotted module path with underscores.
pub(super) fn mangle_module(module: &str) -> String {
    module.replace('.', "_")
}

/// True when a parameter lowers to a C pointer rather than a value.
///
/// A writable lend of a non-duplicable struct must reach the caller's storage,
/// so it becomes a `T *`; a duplicable struct lend binds a copy and stays by
/// value, matching the unnecessary-lend warning. A nullable lend is an
/// ownership out-slot the callee writes and releases through, so it becomes a
/// pointer as well; structs cannot be nullable, so that arm covers handles,
/// `raw`, `cstring`, and views. The named-function ABI and a function-pointer
/// type both spell their parameters this way, so a func value's C type matches
/// every function it can hold.
pub(super) fn is_pointer_param(ty: &Ty, mode: Mode, table: &TypeTable) -> bool {
    mode == Mode::Borrow
        && (ty.nullable || (matches!(ty.kind, TyKind::Struct(..)) && !table.duplicable(ty)))
}

/// True when a type is a view emitted as a value struct rather than a pointer.
///
/// `str`, `static str`, and slice arrays (`T[]`) lower to pointer-and-length
/// structs (`ClStr`, `ClSlice_T`), so their empty state is the zero struct, not
/// a null pointer. Handles, `raw`, and `cstring` already are C pointers and
/// keep the pointer spelling.
pub(super) fn is_struct_view(ty: &Ty) -> bool {
    matches!(ty.kind, TyKind::Str | TyKind::StaticStr)
        || matches!(&ty.kind, TyKind::Array { extents, .. } if extents.iter().any(Option::is_none))
}

/// The C helper name returning the empty value of a struct-typed view.
///
/// The helper is emitted next to the view typedefs, so an empty view in an
/// expression position obeys the C99-subset contract without a compound
/// literal. `None` for every other type, which spells its empty state as a
/// null pointer.
pub(super) fn zero_view_helper(ty: &Ty) -> Option<String> {
    match &ty.kind {
        TyKind::Str | TyKind::StaticStr => Some("cl_zero_ClStr".to_owned()),
        TyKind::Array { extents, .. } if extents.iter().any(Option::is_none) => {
            Some(format!("cl_zero_ClSlice_{}", type_key(&view_element(ty))))
        }
        _ => None,
    }
}

/// True when a type's innermost array element is a struct.
///
/// A row-element type such as `P[2]` names the struct by value in the array
/// form of a slice's pointer declarator, so the struct must be complete when
/// the typedef is written; a plain pointer element only needs the forward
/// declaration.
pub(super) fn innermost_is_struct(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Array { element, .. } => innermost_is_struct(element),
        TyKind::Struct(..) => true,
        _ => false,
    }
}

/// The C spelling of a type in a value position.
pub(super) fn c_type(ty: &Ty, table: &TypeTable) -> String {
    match &ty.kind {
        TyKind::Void => "void".to_owned(),
        TyKind::Scalar(scalar) => scalar_c_type(*scalar).to_owned(),
        TyKind::Str | TyKind::StaticStr => "ClStr".to_owned(),
        TyKind::CString => "char *".to_owned(),
        // A pointer to a row keeps the row's extents attached to the pointer:
        // `T (*)[M]` points at a whole `T[M]`, while `T *` points at one `T`.
        TyKind::Raw(inner) => pointer_to_array_type(inner, table)
            .unwrap_or_else(|| format!("{} *", c_type(inner, table))),
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
                c_type(element, table)
            } else {
                format!("ClSlice_{}", type_key(&view_element(ty)))
            }
        }
        TyKind::Func(signature) => func_pointer_type(signature, table),
        TyKind::Closure(closure) => closure_type(closure),
        TyKind::Param(name) => format!("Cl{name}"),
        TyKind::Unknown => "i32".to_owned(),
    }
}

/// The C spelling of a function-pointer type in a value position.
///
/// A `func` value is a C function pointer, so it can be stored in a field and
/// called through the field. An unnamed `ret (*)(params)` is a complete type
/// usable in casts and `sizeof`.
fn func_pointer_type(signature: &crate::types::FuncTy, table: &TypeTable) -> String {
    function_declarator(
        &signature.returns,
        func_pointer_params(signature, table),
        "(*)",
        None,
        table,
    )
}

/// The spelling of a function-pointer declarator that names the pointer.
fn func_pointer_declarator(signature: &crate::types::FuncTy, name: &str, table: &TypeTable) -> String {
    function_declarator(
        &signature.returns,
        func_pointer_params(signature, table),
        &format!("(*{name})"),
        None,
        table,
    )
}

/// The parameter declarations of a function pointer, without names.
///
/// A function-pointer type lists only each parameter's type, so the same
/// spelling fits a field, a cast, or a nested return. A `borrow` parameter
/// follows [`is_pointer_param`]: a non-duplicable struct lend is the pointer
/// to the value, exactly as the named-function ABI spells it, and every other
/// parameter stays by value.
fn func_pointer_params(signature: &crate::types::FuncTy, table: &TypeTable) -> Vec<String> {
    signature
        .params
        .iter()
        .map(|param| {
            if is_pointer_param(&param.ty, param.mode, table) {
                declarator(&Ty::raw(param.ty.clone()), "", table)
                    .trim()
                    .to_owned()
            } else {
                declarator(&param.ty, "", table).trim().to_owned()
            }
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
    table: &TypeTable,
) -> String {
    let mut params = params;
    if let Some(env) = env {
        params.insert(0, env);
    }
    if returns.is_fixed_array() {
        params.push(declarator(returns, "__cl_ret", table));
        return format!("void {name}({})", parameter_list(params));
    }
    if let TyKind::Func(signature) = &returns.kind {
        let pointer = format!("(*{name}({}))", parameter_list(params));
        return function_declarator(
            &signature.returns,
            func_pointer_params(signature, table),
            &pointer,
            None,
            table,
        );
    }
    format!("{} {name}({})", c_type(returns, table), parameter_list(params))
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
pub(super) fn pointer_to_array_type(row: &Ty, table: &TypeTable) -> Option<String> {
    let TyKind::Array { element, .. } = &row.kind else {
        return None;
    };
    Some(format!(
        "{} (*){}",
        c_type(element, table),
        fixed_extent_suffix(row)?
    ))
}

/// The C spelling of a pointer-to-array declarator that names the pointer:
/// `T (*name)[M]` for a fixed row. `None` when the inner type is not a fixed
/// array.
pub(super) fn pointer_to_array_declarator(
    row: &Ty,
    name: &str,
    table: &TypeTable,
) -> Option<String> {
    let TyKind::Array { element, .. } = &row.kind else {
        return None;
    };
    Some(format!(
        "{} (*{name}){}",
        c_type(element, table),
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
pub(super) fn declarator(ty: &Ty, name: &str, table: &TypeTable) -> String {
    match &ty.kind {
        TyKind::Array { element, extents } if extents.iter().all(Option::is_some) => {
            let mut suffix = String::new();
            for extent in extents {
                suffix.push_str(&format!("[{}]", extent.unwrap_or(0)));
            }
            format!("{} {name}{suffix}", c_type(element, table))
        }
        TyKind::Raw(inner) => pointer_to_array_declarator(inner, name, table)
            .unwrap_or_else(|| format!("{} *{name}", c_type(inner, table))),
        TyKind::CString => format!("char *{name}"),
        TyKind::Func(signature) => func_pointer_declarator(signature, name, table),
        _ => format!("{} {name}", c_type(ty, table)),
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

/// The C struct name of a closure's two-word value.
///
/// A closure specialized for an enclosing instance carries the instance key in
/// its name, so two specializations of one site name different C structs.
pub(super) fn closure_type(closure: &ClosureTy) -> String {
    match &closure.instance {
        Some(key) => format!("ClClosure_{}__{}", closure.id, key),
        None => format!("ClClosure_{}", closure.id),
    }
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
            // declare; its concrete instances add the typedefs they need. The
            // typedef is keyed by the view's element — the whole row for a
            // multi-extent view — so a written row slice and an inferred one
            // share the spelling.
            if extents.iter().any(Option::is_none) {
                let view_element = view_element(ty);
                if !contains_param(&view_element) {
                    found
                        .entry(type_key(&view_element))
                        .or_insert_with(|| view_element.clone());
                }
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
        let table = TypeTable::default();
        assert_eq!(
            c_type(
                &Ty {
                    kind: TyKind::Str,
                    nullable: false,
                },
                &table
            ),
            "ClStr"
        );
        assert_eq!(c_type(&Ty::handle("String"), &table), "ClString");
        assert_eq!(c_type(&Ty::handle("File"), &table), "BPTR");
        assert_eq!(c_type(&Ty::handle("Buffer"), &table), "ClBuffer");
        assert_eq!(c_type(&Ty::struct_("Point"), &table), "ClPoint");
    }

    #[test]
    fn arrays_and_pointers_get_a_declarator_suffix() {
        let table = TypeTable::default();
        let array = Ty::array(Ty::scalar(ScalarType::U32), vec![Some(2)]);
        assert_eq!(declarator(&array, "xs", &table), "u32 xs[2]");
        let slice = Ty::array(Ty::scalar(ScalarType::U8), vec![None]);
        assert_eq!(c_type(&slice, &table), "ClSlice_u8");
        assert_eq!(declarator(&slice, "bytes", &table), "ClSlice_u8 bytes");
        let raw = Ty {
            kind: TyKind::Raw(Box::new(Ty::scalar(ScalarType::U32))),
            nullable: false,
        };
        assert_eq!(declarator(&raw, "p", &table), "u32 *p");
    }

    #[test]
    fn instantiated_struct_keys_compose_over_nested_arguments() {
        let table = TypeTable::default();
        let pair_i32 = Ty::struct_with("Pair", vec![Ty::i32_ty()]);
        assert_eq!(type_key(&pair_i32), "Pair__i32");
        assert_eq!(c_type(&pair_i32, &table), "ClPair__i32");

        let nested = Ty::struct_with("Pair", vec![pair_i32]);
        assert_eq!(type_key(&nested), "Pair__Pair__i32");
        assert_eq!(c_type(&nested, &table), "ClPair__Pair__i32");

        assert_eq!(type_key(&Ty::struct_("Point")), "Point");
        assert_eq!(c_type(&Ty::struct_("Point"), &table), "ClPoint");
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

    #[test]
    fn struct_views_are_distinguished_from_pointer_likes() {
        let string_view = Ty {
            kind: TyKind::Str,
            nullable: true,
        };
        assert!(is_struct_view(&string_view));
        assert!(is_struct_view(&Ty::array(Ty::i32_ty(), vec![None])));
        assert!(!is_struct_view(&Ty::array(Ty::i32_ty(), vec![Some(2)])));
        assert!(!is_struct_view(&Ty::handle("String")));
        assert!(!is_struct_view(&Ty {
            kind: TyKind::CString,
            nullable: true,
        }));
        assert!(!is_struct_view(&Ty {
            kind: TyKind::Raw(Box::new(Ty::i32_ty())),
            nullable: true,
        }));
    }

    #[test]
    fn type_keys_distinguish_extent_lists() {
        assert_eq!(type_key(&Ty::i32_ty()), "i32");
        assert_eq!(type_key(&Ty::array(Ty::i32_ty(), vec![Some(2)])), "arr2_i32");
        assert_eq!(type_key(&Ty::array(Ty::i32_ty(), vec![Some(3)])), "arr3_i32");
        assert_eq!(
            type_key(&Ty::array(Ty::i32_ty(), vec![Some(2), Some(3)])),
            "arr2x3_i32"
        );
        assert_eq!(type_key(&Ty::array(Ty::i32_ty(), vec![None])), "slice_i32");
        assert_eq!(
            type_key(&Ty::array(
                Ty::struct_with("Pair", vec![Ty::i32_ty()]),
                vec![Some(2)]
            )),
            "arr2_Pair__i32"
        );
    }

    #[test]
    fn slice_keys_share_the_row_element() {
        let table = TypeTable::default();
        let flat = Ty::array(Ty::i32_ty(), vec![None, Some(3)]);
        let nested = Ty::array(Ty::array(Ty::i32_ty(), vec![Some(3)]), vec![None]);
        assert_eq!(type_key(&flat), "slice_arr3_i32");
        assert_eq!(type_key(&nested), "slice_arr3_i32");
        assert_eq!(c_type(&flat, &table), "ClSlice_arr3_i32");
        assert_eq!(c_type(&nested, &table), "ClSlice_arr3_i32");
    }
}

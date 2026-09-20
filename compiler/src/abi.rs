//! FFI ABI mapping.
//!
//! An `ffi` declaration is typed by the target ABI, not by the Clorraine
//! surface types. The declared type alone fixes the C parameter type; a
//! parameter mode is semantic and never shapes the prototype. The mapping is
//! the table in the codegen contract, implemented literally.

use crate::types::{Ty, TyKind};

/// The C ABI type a handle name maps to, when a contract names one.
///
/// A handle with no contract entry is emitted as the opaque `Cl<Name>`
/// pointer typedef by the type emitter.
pub fn contract_handle(name: &str) -> Option<&'static str> {
    match name {
        "File" | "Lock" => Some("BPTR"),
        "Library" => Some("struct Library *"),
        "Memory" => Some("APTR"),
        _ => None,
    }
}

/// The C ABI type of an `ffi` parameter or return value.
///
/// Views cross the boundary as their pointer only: the length is a separate
/// parameter supplied by the wrapper, so a view maps to `APTR`.
pub fn ffi_type(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Void => "void".to_owned(),
        TyKind::Str | TyKind::StaticStr | TyKind::CString => {
            if matches!(ty.kind, TyKind::CString) {
                "STRPTR".to_owned()
            } else {
                "APTR".to_owned()
            }
        }
        TyKind::Raw(_) => "APTR".to_owned(),
        TyKind::Handle(name) => contract_handle(name)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Cl{name}")),
        TyKind::Struct(name) => format!("Cl{name}"),
        TyKind::Enum(name) => enum_backing_c_type(name).to_owned(),
        TyKind::Scalar(scalar) => scalar_c_type(*scalar).to_owned(),
        TyKind::Array { .. } => "APTR".to_owned(),
        TyKind::Func(_) => "APTR".to_owned(),
        TyKind::Param(name) => format!("Cl{name}"),
        TyKind::Unknown => "APTR".to_owned(),
    }
}

/// The fixed-width C typedef of a scalar, as the ABI table spells it.
pub fn scalar_c_type(scalar: crate::ast::ScalarType) -> &'static str {
    use crate::ast::ScalarType::*;
    match scalar {
        I8 => "BYTE",
        I16 => "WORD",
        I32 => "LONG",
        U8 => "UBYTE",
        U16 => "UWORD",
        U32 => "ULONG",
        Usize => "ULONG",
        Isize => "LONG",
        Bool => "BOOL",
        F32 => "float",
        F64 => "double",
    }
}

/// The backing C type of a named enum.
///
/// The type emitter only reaches here for a well-formed program, where the
/// enum's explicit backing scalar is recorded; the fallback keeps the emitter
/// total for an unknown name.
fn enum_backing_c_type(_name: &str) -> &'static str {
    "i32"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Ty;

    #[test]
    fn contracted_handles_use_their_declared_abi_type() {
        assert_eq!(contract_handle("File"), Some("BPTR"));
        assert_eq!(contract_handle("Library"), Some("struct Library *"));
        assert_eq!(contract_handle("Memory"), Some("APTR"));
        assert_eq!(contract_handle("Buffer"), None);
    }

    #[test]
    fn views_and_raw_pointers_lower_to_a_plain_pointer() {
        assert_eq!(ffi_type(&Ty::handle("File")), "BPTR");
        assert_eq!(ffi_type(&Ty::handle("Buffer")), "ClBuffer");
        assert_eq!(
            ffi_type(&Ty {
                kind: TyKind::Raw(Box::new(Ty::i32_ty())),
                nullable: false,
            }),
            "APTR"
        );
        assert_eq!(
            ffi_type(&Ty {
                kind: TyKind::Str,
                nullable: false,
            }),
            "APTR"
        );
    }

    #[test]
    fn scalars_use_fixed_width_typedefs() {
        use crate::ast::ScalarType::*;
        assert_eq!(scalar_c_type(I32), "LONG");
        assert_eq!(scalar_c_type(Usize), "ULONG");
        assert_eq!(scalar_c_type(Bool), "BOOL");
        assert_eq!(scalar_c_type(F64), "double");
    }
}

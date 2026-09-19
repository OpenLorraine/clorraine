//! FFI contract checks and the unsafe/raw boundary.
//!
//! An `ffi` block declares the ownership contract of an external function so
//! Clorraine code can be checked against it. This module owns the declaration
//! level rules (a `freedBy` deleter must take `take T` and return `void`, an
//! `ffi` block may not be exported, a native struct may not declare
//! `freedBy`) and the small classification helpers the call-site and
//! unsafe-context checks share.

use crate::ast::{FuncSignature, Mode, StructDecl, Type, TypeKind};
use crate::diag::{Diagnostic, Diagnostics};
use crate::types::{resolve_ty, Ty, TypeTable};

use super::own::program::{type_name_matches, ProgramInfo};

/// Checks the declaration-level FFI rules of a whole program.
///
/// These checks do not depend on any function body: a handle's deleter, an
/// exported block, and a native struct's `freedBy` are all one-shot
/// declaration facts.
pub fn check_items(program: &ProgramInfo, diagnostics: &mut Diagnostics) {
    for block in &program.ffi_blocks {
        if block.exported {
            diagnostics.push(
                Diagnostic::error(
                    "E10103",
                    "'ffi' blocks are module-scoped and cannot be exported",
                    block.name_span,
                )
                .with_help("export a wrapper function instead"),
            );
        }
    }
    for handle in program.handles.values() {
        if !deleter_is_valid(handle.deleter.as_str(), &handle.name, program) {
            diagnostics.push(
                Diagnostic::error(
                    "E10102",
                    format!(
                        "'{}' is not a valid deleter for '{}'",
                        handle.deleter, handle.name
                    ),
                    handle.span,
                )
                .with_help("declare the deleter in an `ffi` block with the right signature"),
            );
        }
    }
    for decl in program.structs.values() {
        if decl.freed_by.is_some() {
            diagnostics.push(
                Diagnostic::error(
                    "E10104",
                    "'freedBy' is only for handle types",
                    decl.name_span,
                )
                .with_help("native structs drop structurally"),
            );
        }
    }
}

/// True when `deleter` is an `ffi` function taking `take T` and returning
/// `void`, the CN-6 signature for a handle's deleter.
fn deleter_is_valid(deleter: &str, handle: &str, program: &ProgramInfo) -> bool {
    let Some(signature) = program.ffi_functions.get(deleter) else {
        return false;
    };
    if signature.params.len() != 1 {
        return false;
    }
    let param = &signature.params[0];
    param.mode == Mode::Take
        && type_name_matches(&param.ty, handle)
        && matches!(signature.returns.kind, TypeKind::Void)
        && !signature.returns.nullable
}

/// True when an `ffi` signature has at least one uncontracted parameter.
///
/// A parameter is uncontracted when its ownership mode is omitted on a handle
/// or aggregate type; the call then requires `unsafe` and the parameter may
/// not receive an owned or borrowed value (CN-10, CN-11).
pub fn is_uncontracted(signature: &FuncSignature, table: &TypeTable) -> bool {
    signature
        .params
        .iter()
        .any(|param| is_uncontracted_param(param.mode, &resolve_ty(&param.ty, &[], table)))
}

/// True when one `ffi` parameter is uncontracted.
pub fn is_uncontracted_param(mode: Mode, ty: &Ty) -> bool {
    mode == Mode::None && matches!(ty.kind, crate::types::TyKind::Handle(_) | crate::types::TyKind::Struct(_))
}

/// Resolves the declared type of an `ffi` parameter.
pub fn signature_param_type(ty: &Type, table: &TypeTable) -> Ty {
    resolve_ty(ty, &[], table)
}

/// True when a struct declaration carries `freedBy`.
pub fn struct_has_freed_by(decl: &StructDecl) -> bool {
    decl.freed_by.is_some()
}

/// A raw or volatile operation that is legal only inside `unsafe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsafeOp {
    /// `*p` or an implied dereference through `p.field`.
    Deref,
    /// `&place`.
    AddressOf,
    /// `p + n`, `p - n`, or `p[i]`.
    PointerArithmetic,
    /// `raw T(q)` where `q` is already a raw pointer.
    PointerCast,
    /// A volatile intrinsic or overlay access.
    Volatile,
}

impl UnsafeOp {
    /// The registry code for this operation.
    pub fn code(self) -> &'static str {
        match self {
            Self::Deref => "E10600",
            Self::AddressOf => "E10601",
            Self::PointerArithmetic => "E10602",
            Self::PointerCast => "E10603",
            Self::Volatile => "E10604",
        }
    }

    /// The registry message for this operation.
    pub fn message(self) -> &'static str {
        match self {
            Self::Deref => "dereference of a raw pointer requires 'unsafe'",
            Self::AddressOf => "address-of requires 'unsafe'",
            Self::PointerArithmetic => "pointer arithmetic requires 'unsafe'",
            Self::PointerCast => "raw pointer cast requires 'unsafe'",
            Self::Volatile => "volatile access requires 'unsafe'",
        }
    }

    /// The registry help text for this operation.
    pub fn help(self) -> &'static str {
        match self {
            Self::Deref => "wrap it in `unsafe` or an `unsafe { ... }` block",
            Self::AddressOf => "take a view parameter instead, or use `unsafe`",
            Self::PointerArithmetic => "wrap the computation in `unsafe`",
            Self::PointerCast => {
                "integer-to-`raw` value casts are allowed; pointer casts are not"
            }
            Self::Volatile => "wrap hardware access in `unsafe` or use the stdlib wrappers",
        }
    }
}

/// True for a reserved volatile intrinsic name.
pub fn is_volatile_intrinsic(name: &str) -> bool {
    name.starts_with("readVolatile") || name.starts_with("writeVolatile")
}

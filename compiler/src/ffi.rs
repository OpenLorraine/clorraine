//! FFI contract checks and the unsafe/raw boundary.
//!
//! An `ffi` block declares the ownership contract of an external function so
//! Clorraine code can be checked against it. This module owns the declaration
//! level rules (a `freedBy` deleter must take `take T` and return `void`, an
//! `ffi` block may not be exported, a native struct may not declare
//! `freedBy`) and the small classification helpers the call-site and
//! unsafe-context checks share.

use crate::ast::{FuncSignature, Item, Mode, StructDecl, Type};
use crate::diag::{Diagnostic, Diagnostics};
use crate::types::{resolve_ty, resolve_ty_folded, Ty, TyKind, TypeTable};

use super::own::program::{HandleInfo, ProgramInfo};

/// Checks the declaration-level FFI rules of a whole program.
///
/// These checks do not depend on any function body: a handle's deleter, an
/// exported block, a native struct's `freedBy`, and the FFI-boundary type rule
/// are all one-shot declaration facts.
pub fn check_items(program: &ProgramInfo, table: &TypeTable, diagnostics: &mut Diagnostics) {
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
    for module in program.loaded.modules.values() {
        for item in &module.items {
            let Item::Ffi(block) = item else {
                continue;
            };
            for signature in &block.decls {
                for param in &signature.params {
                    check_ffi_boundary_type(
                        &param.ty,
                        table,
                        module,
                        program.loaded,
                        diagnostics,
                    );
                }
                check_ffi_boundary_type(
                    &signature.returns,
                    table,
                    module,
                    program.loaded,
                    diagnostics,
                );
            }
        }
    }
    for handle in program.handles.values() {
        if !deleter_is_valid(handle, table, program) {
            diagnostics.push(
                Diagnostic::error(
                    "E10102",
                    format!(
                        "'{}' is not a valid deleter for '{}'",
                        handle.deleter, handle.name
                    ),
                    handle.span,
                )
                .with_help("declare a function taking 'take T' and returning 'void'"),
            );
        } else if let Some(decl) = ordinary_deleter_decl(handle, program) {
            // The deleter's own `take` parameter is not dropped at its scope
            // exit (that would call the function again). The body must release
            // it explicitly; a body that never consumes it would leak.
            let Some(param) = decl.params.first() else {
                continue;
            };
            if !body_consumes_parameter(&decl.body, &param.name, &decl.name, program) {
                diagnostics.push(
                    Diagnostic::error(
                        "E10102",
                        format!(
                            "'{}' is not a valid deleter for '{}'",
                            handle.deleter, handle.name
                        ),
                        decl.name_span,
                    )
                    .with_help("the deleter must release its 'take' parameter"),
                );
            }
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

/// Rejects a type that cannot cross the FFI boundary.
///
/// A `Box<T>` has an opaque C form the callee cannot interpret, and a generic
/// struct instance currently spells `ClWrap` while the emitted typedef is
/// `ClWrap__i32`; both would emit invalid C. The check resolves import aliases
/// through the declaring module and recurses through arrays, raw pointers, and
/// function-pointer signatures, so a nested case is caught too.
fn check_ffi_boundary_type(
    ty: &Type,
    table: &TypeTable,
    module: &crate::load::Module,
    loaded: &crate::load::Loaded,
    diagnostics: &mut Diagnostics,
) {
    let resolved = resolve_ty_folded(ty, &[], table, module, loaded);
    check_ffi_boundary_ty(&resolved, ty.span, diagnostics);
}

/// Recursive worker for [`check_ffi_boundary_type`].
fn check_ffi_boundary_ty(ty: &Ty, span: crate::span::Span, diagnostics: &mut Diagnostics) {
    match &ty.kind {
        TyKind::Box(_) => {
            diagnostics.push(
                Diagnostic::error(
                    "E10105",
                    format!("'{}' cannot cross the FFI boundary", ty.display()),
                    span,
                )
                .with_help("pass the pointee through `boxWith`, or a raw pointer from an unsafe wrapper"),
            );
        }
        TyKind::Struct(_, args) if !args.is_empty() => {
            diagnostics.push(
                Diagnostic::error(
                    "E10105",
                    format!("'{}' cannot cross the FFI boundary", ty.display()),
                    span,
                )
                .with_help("wrap the generic aggregate in a concrete C-facing struct"),
            );
        }
        TyKind::Array { element, .. } | TyKind::Raw(element) => {
            check_ffi_boundary_ty(element, span, diagnostics);
        }
        TyKind::Func(signature) => {
            for param in &signature.params {
                check_ffi_boundary_ty(&param.ty, span, diagnostics);
            }
            check_ffi_boundary_ty(&signature.returns, span, diagnostics);
        }
        _ => {}
    }
}


fn deleter_is_valid(handle: &HandleInfo, table: &TypeTable, program: &ProgramInfo) -> bool {
    let Some(resolved) = &handle.resolved else {
        return false;
    };
    let Some(module) = program.loaded.module(&resolved.module) else {
        return false;
    };
    for item in &module.items {
        match item {
            Item::Func(decl) if !resolved.ffi && decl.name == resolved.name => {
                if !decl.type_params.is_empty() {
                    return false;
                }
                return signature_is_valid(
                    &decl.params,
                    &decl.returns,
                    &handle.name,
                    module,
                    table,
                    program.loaded,
                );
            }
            Item::Ffi(block) if resolved.ffi => {
                for signature in &block.decls {
                    if signature.name == resolved.name {
                        return signature_is_valid(
                            &signature.params,
                            &signature.returns,
                            &handle.name,
                            module,
                            table,
                            program.loaded,
                        );
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// True when a declaration is `take T -> void` for the exact handle type `T`.
fn signature_is_valid(
    params: &[crate::ast::Param],
    returns: &Type,
    handle: &str,
    module: &crate::load::Module,
    table: &TypeTable,
    loaded: &crate::load::Loaded,
) -> bool {
    let [param] = params else {
        return false;
    };
    if param.mode != Mode::Take {
        return false;
    }
    let param_ty = resolve_ty_folded(&param.ty, &[], table, module, loaded);
    let expected = Ty::handle(handle);
    let returns_ty = resolve_ty_folded(returns, &[], table, module, loaded);
    param_ty == expected
        && matches!(returns_ty.kind, crate::types::TyKind::Void)
        && !returns_ty.nullable
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
///
/// A handle, a struct, and a fixed array are owned storage the extern has no
/// contract for, so the caller must either contract the parameter or call
/// inside `unsafe`. A slice is a view over storage owned elsewhere and stays
/// acceptable, like a scalar, `raw`, or `cstring`.
pub fn is_uncontracted_param(mode: Mode, ty: &Ty) -> bool {
    mode == Mode::None
        && (matches!(
            ty.kind,
            crate::types::TyKind::Handle(_)
                | crate::types::TyKind::Box(_)
                | crate::types::TyKind::Struct(..)
        ) || ty.is_fixed_array())
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

#[cfg(test)]
mod tests {
    use crate::own::test_support::{error_codes, reports};

    #[test]
    fn a_well_formed_deleter_is_accepted() {
        let source = "module t;\n\
            handle File freedBy CloseFile;\n\
            ffi lib { func CloseFile(take f: File): void; }\n\
            func main(): i32 { return 0; }\n";
        assert!(!reports(source, "E10102"), "{:?}", error_codes(source));
    }

    #[test]
    fn a_deleter_with_the_wrong_signature_is_rejected() {
        let source = "module t;\n\
            handle File freedBy BadDeleter;\n\
            ffi lib { func BadDeleter(f: File): i32; }\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10102"), "{:?}", error_codes(source));
    }

    #[test]
    fn an_exported_ffi_block_is_rejected() {
        let source = "module t;\n\
            export ffi lib { func CloseFile(take f: File): void; }\n\
            handle File freedBy CloseFile;\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10103"), "{:?}", error_codes(source));
    }

    #[test]
    fn freed_by_on_a_struct_is_rejected() {
        let source = "module t;\n\
            struct Node freedBy CloseFile { value: i32; }\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10104"), "{:?}", error_codes(source));
    }

    #[test]
    fn an_uncontracted_call_outside_unsafe_is_rejected() {
        let source = "module t;\n\
            handle File freedBy CloseFile;\n\
            ffi lib {\n\
                func RawRead(f: File?, n: i32): i32;\n\
                func CloseFile(take f: File): void;\n\
            }\n\
            func demo(): i32 {\n\
                let f: File? = null;\n\
                return RawRead(f, 0);\n\
            }\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10100"), "{:?}", error_codes(source));
    }

    #[test]
    fn an_owned_handle_to_an_uncontracted_parameter_is_rejected() {
        let source = "module t;\n\
            handle File freedBy CloseFile;\n\
            ffi lib {\n\
                func RawStat(f: File): i32;\n\
                func CloseFile(take f: File): void;\n\
            }\n\
            func demo(f: File): void {\n\
                unsafe { let n = RawStat(f); }\n\
            }\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10101"), "{:?}", error_codes(source));
    }

    #[test]
    fn a_narrowed_nullable_handle_is_still_rejected_by_an_uncontracted_parameter() {
        let source = "module t;\n\
            handle File freedBy Close;\n\
            ffi lib {\n\
                func RawStat(f: File?): i32;\n\
                func Close(take f: File): void;\n\
            }\n\
            func demo(f: File): void {\n\
                let g: File? = f;\n\
                if (g != null) { unsafe { let n = RawStat(g); } }\n\
            }\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10101"), "{:?}", error_codes(source));
    }

    #[test]
    fn a_scalar_to_an_uncontracted_parameter_is_accepted() {
        let source = "module t;\n\
            ffi lib { func RawTick(n: i32): i32; }\n\
            func demo(): i32 { unsafe { return RawTick(0); } }\n\
            func main(): i32 { return 0; }\n";
        assert!(!reports(source, "E10101"), "{:?}", error_codes(source));
    }

    #[test]
    fn a_fixed_array_to_an_uncontracted_parameter_is_rejected() {
        let source = "module t;\n\
            ffi lib { func RawSum(xs: u8[4]): i32; }\n\
            func demo(xs: u8[4]): i32 {\n\
                let n: i32 = 0;\n\
                unsafe { n = RawSum(xs); }\n\
                return n;\n\
            }\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10101"), "{:?}", error_codes(source));
    }

    #[test]
    fn a_slice_to_an_uncontracted_parameter_is_accepted() {
        let source = "module t;\n\
            ffi lib { func RawSum(xs: u8[]): i32; }\n\
            func demo(xs: u8[]): i32 {\n\
                let n: i32 = 0;\n\
                unsafe { n = RawSum(xs); }\n\
                return n;\n\
            }\n\
            func main(): i32 { return 0; }\n";
        assert!(!reports(source, "E10101"), "{:?}", error_codes(source));
    }
}

/// The ordinary function declaration a handle's deleter resolves to.
fn ordinary_deleter_decl<'a>(
    handle: &HandleInfo,
    program: &'a ProgramInfo,
) -> Option<&'a crate::ast::FuncDecl> {
    let resolved = handle.resolved.as_ref()?;
    if resolved.ffi {
        return None;
    }
    let module = program.loaded.module(&resolved.module)?;
    module.items.iter().find_map(|item| match item {
        Item::Func(decl) if decl.name == resolved.name => Some(decl),
        _ => None,
    })
}

/// True when a deleter body releases its parameter through a top-level `ffi`
/// `take` call. The body's own end-of-scope drop is suppressed, so a body with
/// no consuming call would leak the handle.
///
/// v1.1 supports the straight-line release shape: the parameter must reach an
/// `ffi` parameter declared `take` in a statement of the body's own sequence.
/// A borrow-only call, a conditional release, a `defer`, or forwarding through
/// another ordinary function is rejected with E10102; the conditional and
/// defer shapes can leak on a path, and forwarding can recurse back into the
/// deleter.
fn body_consumes_parameter(
    body: &crate::ast::Block,
    name: &str,
    self_name: &str,
    program: &super::own::program::ProgramInfo,
) -> bool {
    body.statements.iter().any(|stmt| match stmt {
        crate::ast::Stmt::Expr(expr) => ffi_take_call(expr, name, self_name, program),
        _ => false,
    })
}

/// True when the expression is an `ffi` call passing the parameter to a `take`
/// position, and is not a call back to the deleter itself.
fn ffi_take_call(
    expr: &crate::ast::Expr,
    name: &str,
    self_name: &str,
    program: &super::own::program::ProgramInfo,
) -> bool {
    let crate::ast::ExprKind::Call { callee, args, .. } = &expr.kind else {
        return false;
    };
    let callee_name = match &callee.kind {
        crate::ast::ExprKind::Name(found) => found.as_str(),
        crate::ast::ExprKind::Qualified(qualified) => {
            qualified.segments.last().map(String::as_str).unwrap_or("")
        }
        _ => return false,
    };
    if callee_name == self_name {
        return false;
    }
    let Some(signature) = program.ffi_function(callee_name) else {
        return false;
    };
    args.iter().enumerate().any(|(index, arg)| {
        is_parameter(&arg.value, name)
            && signature
                .params
                .get(index)
                .is_some_and(|param| param.mode == crate::ast::Mode::Take)
    })
}

/// True when an expression names the parameter directly.
fn is_parameter(expr: &crate::ast::Expr, name: &str) -> bool {
    match &expr.kind {
        crate::ast::ExprKind::Name(found) => found == name,
        crate::ast::ExprKind::Qualified(qualified) => {
            qualified.segments.len() == 1 && qualified.segments[0] == name
        }
        _ => false,
    }
}

//! The program facts the ownership checker needs, gathered in one pass.
//!
//! The ownership stage sees the whole loaded program, so it collects the
//! declarations that decide ownership: the functions (user and `ffi`), the
//! handles and their deleters, the structs, the module-level bindings, and the
//! behavior of the standard-library helpers that return views. Gathering them
//! once keeps the per-function walk free of repeated lookups.

use std::collections::BTreeMap;

use crate::ast::{FfiBlock, FuncDecl, FuncSignature, Item, StructDecl};
use crate::borrow::{is_nullable_capable, LoanKind};
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{Loaded, Module};
use crate::sema::SemaOutput;
use crate::span::Span;
use crate::types::{resolve_ty, Ty};

/// A user-declared function with a body.
pub type UserFunc<'a> = &'a FuncDecl;

/// A handle declaration and the deleter it names.
pub struct HandleInfo<'a> {
    /// The handle's name.
    pub name: String,
    /// The named deleter.
    pub deleter: String,
    /// Span of the handle's name, used for the deleter diagnostic.
    pub span: Span,
    /// The declaration, for type checking of the deleter.
    pub decl: &'a crate::ast::HandleDecl,
}

/// A module-level binding.
pub struct GlobalInfo {
    /// The binding's name.
    pub name: String,
    /// Its declared type.
    pub ty: Ty,
    /// True for a `const`, which may not be assigned.
    pub is_const: bool,
    /// Span of the name.
    pub span: Span,
}

/// What a standard-library helper does with ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdlibEffect {
    /// Returns a freshly owned value; the caller owns it.
    OwnsResult,
    /// Returns a view over the storage of the argument at this index.
    ViewOf {
        /// Argument index whose storage the result views.
        arg: usize,
        /// The permission the result carries.
        kind: LoanKind,
    },
    /// Produces an ordinary value or `void` and owns nothing.
    Value,
}

/// Every declaration the ownership stage needs, keyed by simple name.
///
/// Names are program-global in the model the MVP corpus uses, so a simple key
/// is enough; a later revision can qualify by module without changing callers.
#[derive(Default)]
pub struct ProgramInfo<'a> {
    /// User functions with bodies.
    pub functions: BTreeMap<String, UserFunc<'a>>,
    /// `ffi`-declared signatures.
    pub ffi_functions: BTreeMap<String, &'a FuncSignature>,
    /// `ffi` blocks, for the export check.
    pub ffi_blocks: Vec<&'a FfiBlock>,
    /// Handle declarations and their deleters.
    pub handles: BTreeMap<String, HandleInfo<'a>>,
    /// Struct declarations, for the `freedBy` check.
    pub structs: BTreeMap<String, &'a StructDecl>,
    /// Module-level bindings.
    pub globals: BTreeMap<String, GlobalInfo>,
}

impl<'a> ProgramInfo<'a> {
    /// Collects every ownership-relevant declaration of the program.
    pub fn collect(loaded: &'a Loaded, table: &crate::types::TypeTable) -> Self {
        let mut info = ProgramInfo::default();
        for module in loaded.modules.values() {
            for item in &module.items {
                match item {
                    Item::Func(decl) => {
                        info.functions.entry(decl.name.clone()).or_insert(decl);
                    }
                    Item::Ffi(block) => {
                        info.ffi_blocks.push(block);
                        for signature in &block.decls {
                            info.ffi_functions
                                .entry(signature.name.clone())
                                .or_insert(signature);
                        }
                    }
                    Item::Handle(decl) => {
                        info.handles.insert(
                            decl.name.clone(),
                            HandleInfo {
                                name: decl.name.clone(),
                                deleter: decl.freed_by.segments.join("."),
                                span: decl.name_span,
                                decl,
                            },
                        );
                    }
                    Item::Struct(decl) => {
                        info.structs.entry(decl.name.clone()).or_insert(decl);
                    }
                    Item::GlobalLet(decl) => {
                        info.globals.insert(
                            decl.name.clone(),
                            GlobalInfo {
                                name: decl.name.clone(),
                                ty: table
                                    .global(&module.name, &decl.name)
                                    .cloned()
                                    .unwrap_or_else(Ty::unknown),
                                is_const: false,
                                span: decl.name_span,
                            },
                        );
                    }
                    Item::GlobalConst(decl) => {
                        info.globals.insert(
                            decl.name.clone(),
                            GlobalInfo {
                                name: decl.name.clone(),
                                ty: table
                                    .global(&module.name, &decl.name)
                                    .cloned()
                                    .unwrap_or_else(Ty::unknown),
                                is_const: true,
                                span: decl.name_span,
                            },
                        );
                    }
                    Item::Comptime(_) | Item::Enum(_) | Item::Intrinsic(_) => {}
                }
            }
        }
        info
    }

    /// Resolves the callee of a call by simple name: a user function, an
    /// `ffi` function, or a known standard-library helper.
    pub fn callee(&self, name: &str) -> Option<Callee<'a>> {
        if let Some(function) = self.functions.get(name) {
            return Some(Callee::User(function));
        }
        if let Some(signature) = self.ffi_functions.get(name) {
            return Some(Callee::Ffi(signature));
        }
        stdlib_effect(name).map(|effect| Callee::Stdlib {
            name: name.to_owned(),
            effect,
        })
    }
}

/// The declaration a call name resolves to.
pub enum Callee<'a> {
    /// A user function with a body.
    User(UserFunc<'a>),
    /// An `ffi`-declared signature.
    Ffi(&'a FuncSignature),
    /// A standard-library helper with known ownership behavior.
    Stdlib {
        /// The helper's name.
        name: String,
        /// What the helper does with ownership.
        effect: StdlibEffect,
    },
}

/// The ownership behavior of the modeled standard-library helpers.
///
/// A helper that is not listed is treated as an opaque call that produces no
/// view and owns nothing, so it never triggers a false ownership diagnostic.
pub fn stdlib_effect(name: &str) -> Option<StdlibEffect> {
    match name {
        // Allocating constructors return an owned value.
        "stringOf" | "clone" | "text" | "toHex" | "toDecimal" | "padLeft" | "padRight"
        | "bufferNew" | "bufferFrom" | "arrayNew" | "openFile" | "createFile" | "addHandler" => {
            Some(StdlibEffect::OwnsResult)
        }
        // Helpers that return a view or `cstring` over storage the caller
        // still owns, so the call is a loan of their first argument.
        "bytes" | "toCString" | "slice" => Some(StdlibEffect::ViewOf {
            arg: 0,
            kind: LoanKind::Shared,
        }),
        // `bufferBytes` derives a writable view, so a call through a writable
        // borrow is an exclusive reborrow.
        "bufferBytes" => Some(StdlibEffect::ViewOf {
            arg: 0,
            kind: LoanKind::Exclusive,
        }),
        _ => None,
    }
}

/// Resolves a written type name to an AST spelling for the handle/deleter
/// comparison, which only needs the declared name.
pub fn type_name_matches(ty: &crate::ast::Type, name: &str) -> bool {
    match &ty.kind {
        crate::ast::TypeKind::Named { name: written, .. } => {
            written.segments.last().is_some_and(|last| last == name)
        }
        _ => false,
    }
}

/// Rejects a module-level `let` whose declared type needs a drop.
///
/// There are no drops at process exit, so a global must be duplicable; a
/// `const` is exempt because it cannot own a released resource.
pub(super) fn check_globals(module: &Module, sema: &SemaOutput, diagnostics: &mut Diagnostics) {
    for item in &module.items {
        match item {
            Item::GlobalLet(decl) => {
                let ty = resolve_ty(&decl.ty, &[], &sema.table);
                report_illegal_nullable(&ty, decl.name_span, diagnostics);
                if !sema.table.duplicable(&ty) && !ty.is_unknown() {
                    diagnostics.push(
                        Diagnostic::error(
                            "E11106",
                            "module-level 'let' must be duplicable or 'const'",
                            decl.name_span,
                        )
                        .with_help("use `const`, or initialize inside an explicit `init()`"),
                    );
                }
            }
            Item::GlobalConst(decl) => {
                let ty = resolve_ty(&decl.ty, &[], &sema.table);
                report_illegal_nullable(&ty, decl.name_span, diagnostics);
            }
            _ => {}
        }
    }
}

/// Rejects a `?` suffix on a struct field whose type cannot be empty.
pub(super) fn check_struct_declarations(
    module: &Module,
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) {
    for item in &module.items {
        if let Item::Struct(decl) = item {
            let params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
            for field in &decl.fields {
                let ty = resolve_ty(&field.ty, &params, &sema.table);
                report_illegal_nullable(&ty, field.name_span, diagnostics);
            }
        }
    }
}

/// Reports `E10011` when a type carries `?` but cannot be empty.
fn report_illegal_nullable(ty: &Ty, span: Span, diagnostics: &mut Diagnostics) {
    if !ty.nullable || is_nullable_capable(ty) {
        return;
    }
    let base = Ty {
        kind: ty.kind.clone(),
        nullable: false,
    };
    diagnostics.push(
        Diagnostic::error(
            "E10011",
            format!("nullable type is not allowed on '{}'", base.display()),
            span,
        )
        .with_help("`?` applies to handles, `raw`, `cstring`, and views"),
    );
}

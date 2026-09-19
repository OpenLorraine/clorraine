//! Declaration checking for structs, enums, and module-level bindings.
//!
//! These checks are mostly one-shot: a declaration is validated where it is
//! read, not while a body is walked. Struct fields reject the plain `str` view
//! and require at least one field; an enum needs an integer backing type and
//! constant member values; a module-level `const` initializer must fold while a
//! module-level `let` may be computed at run time. The same pass validates the
//! constant-shaped parts of every type, such as array extents and the type
//! arguments of a generic struct.

use crate::ast::{EnumDecl, Expr, StructDecl, Type, TypeKind};
use crate::types::{int_fits, TyKind};

use super::Checker;

impl Checker<'_, '_> {
    /// Checks a struct declaration.
    ///
    /// A struct must declare at least one field, no field may be a plain `str`
    /// view, and an `export` on a generic struct is rejected. A `volatile`
    /// field is accepted here; the overlay ban is enforced where a value is
    /// built.
    pub(super) fn check_struct(&mut self, decl: &StructDecl) {
        if decl.exported && !decl.type_params.is_empty() {
            self.error(
                "E10302",
                format!("generic '{}' cannot be exported", decl.name),
                "write a concrete exported wrapper",
                decl.name_span,
            );
        }
        if decl.fields.is_empty() {
            self.error(
                "E10206",
                format!("struct '{}' declares no fields", decl.name),
                "add a field; C99 has no empty structs",
                decl.span,
            );
        }
        self.type_params = decl.type_params.iter().map(|p| p.name.clone()).collect();
        for field in &decl.fields {
            let ty = self.resolve_type(&field.ty);
            if matches!(ty.kind, TyKind::Str) {
                self.error(
                    "E10205",
                    format!("view-typed field '{}' is not allowed", field.name),
                    "store an owner, or `static str` for literals",
                    field.name_span,
                );
            }
        }
        self.type_params.clear();
    }

    /// Checks an enum declaration.
    ///
    /// The backing type must be an integer, and each explicit member value
    /// must be a constant expression that fits the backing type.
    pub(super) fn check_enum(&mut self, decl: &EnumDecl) {
        let backing = self.resolve_type(&decl.backing);
        if !backing.is_unknown() && !backing.is_param() && !backing.is_integer() {
            self.error(
                "E10211",
                "enum backing type must be an integer type",
                "use `i8`-`u32`, `usize`, or `isize`",
                decl.backing.span,
            );
        }
        for member in &decl.members {
            let Some(value) = &member.value else {
                continue;
            };
            self.require_constant(value);
            if let TyKind::Scalar(scalar) = backing.kind {
                if let Some(folded) = self.folder.integer(value) {
                    if !int_fits(folded, scalar) {
                        self.error(
                            "E10202",
                            format!("literal does not fit '{}'", backing.display()),
                            "use a wider type or a hex/binary form",
                            value.span,
                        );
                    }
                }
            }
        }
    }

    /// Checks a module-level `let` or `const`.
    ///
    /// `constant` is true for a `const`, whose initializer is a constant
    /// context; a `let` initializer is computed at run time and is not.
    pub(super) fn check_global(&mut self, declared: &Type, value: &Expr, constant: bool) {
        let ty = self.resolve_type(declared);
        if constant {
            self.require_constant(value);
        }
        self.check_into(value, &ty);
    }

    /// Reports `E10224` or `E10223` when an expression is not foldable.
    pub(super) fn require_constant(&mut self, expr: &Expr) {
        self.folder.require(expr, self.diagnostics);
    }

    /// Validates the constant-shaped parts of a type.
    ///
    /// An array extent must be greater than zero, only the outermost extent of
    /// a multi-dimensional type may be omitted, and a generic struct's type
    /// argument count must match its declaration.
    pub(super) fn validate_type(&mut self, ty: &Type) {
        match &ty.kind {
            TypeKind::Array { element, extents } => {
                self.validate_type(element);
                for (index, extent) in extents.iter().enumerate() {
                    match &extent.size {
                        Some(size) => {
                            self.require_constant(size);
                            if let Some(value) = self.folder.integer(size) {
                                if value <= 0 {
                                    self.error(
                                        "E10226",
                                        "array extent must be greater than zero",
                                        "use a positive extent",
                                        extent.span,
                                    );
                                }
                            }
                        }
                        None if index > 0 => {
                            self.error(
                                "E10220",
                                "only the outermost extent may be omitted",
                                "write every inner extent",
                                extent.span,
                            );
                        }
                        None => {}
                    }
                }
            }
            TypeKind::Raw(inner) => self.validate_type(inner),
            TypeKind::Function { params, returns } => {
                for param in params {
                    self.validate_type(&param.ty);
                }
                self.validate_type(returns);
            }
            TypeKind::Named { name, args } => {
                for arg in args {
                    self.validate_type(arg);
                }
                if let Some(last) = name.segments.last() {
                    if let Some(params) = self.table.struct_type_params(last) {
                        if args.len() != params.len() {
                            self.error(
                                "E10300",
                                format!(
                                    "wrong number of type arguments for '{last}': expected {}, found {}",
                                    params.len(),
                                    args.len()
                                ),
                                "list all type arguments",
                                ty.span,
                            );
                        }
                    }
                }
            }
            TypeKind::Void
            | TypeKind::Scalar(_)
            | TypeKind::Str
            | TypeKind::CString
            | TypeKind::StaticStr
            | TypeKind::Error => {}
        }
    }
}

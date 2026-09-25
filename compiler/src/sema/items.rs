//! Declaration checking for structs, enums, and module-level bindings.
//!
//! These checks are mostly one-shot: a declaration is validated where it is
//! read, not while a body is walked. Struct fields reject the plain `str` view
//! and require at least one field; an enum needs an integer backing type and
//! constant member values; a module-level `const` initializer must fold while a
//! module-level `let` may be computed at run time. The same pass validates the
//! constant-shaped parts of every type, such as array extents and the type
//! arguments of a generic struct.

use crate::ast::{EnumDecl, Expr, ScalarType, StructDecl, Type, TypeKind};
use crate::types::{int_fits, integer_scalar, substitute_instance, Ty, TyKind, TypeTable};

use super::Checker;

impl Checker<'_, '_> {
    /// Checks a struct declaration.
    ///
    /// A struct must declare at least one field and no field may be a plain
    /// `str` view. A generic struct may be exported so another module can
    /// instantiate it; only a generic function is rejected under `E10302`. A
    /// `volatile` field is accepted here; the overlay ban is enforced where a
    /// value is built.
    pub(super) fn check_struct(&mut self, decl: &StructDecl) {
        if decl.fields.is_empty() {
            self.error(
                "E10206",
                format!("struct '{}' declares no fields", decl.name),
                "add a field; C99 has no empty structs",
                decl.span,
            );
        }
        self.type_params = decl.type_params.iter().map(|p| p.name.clone()).collect();
        let mut reported_recursion = false;
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
            // A field that embeds the struct by value makes it infinitely
            // sized; a generic field that instantiates the struct with a new
            // argument recurses without bound. Only a `raw` pointer, a handle,
            // a function pointer, or a slice breaks the chain.
            if !reported_recursion
                && embeds_by_value(self.table, &ty, &decl.name, 0)
            {
                reported_recursion = true;
                self.error(
                    "E10305",
                    format!(
                        "instantiation of '{}' is recursive beyond the limit",
                        decl.name
                    ),
                    "break the recursion with a `raw` pointer",
                    decl.name_span,
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
            self.require_constant(value, integer_scalar(&backing));
            if let TyKind::Scalar(scalar) = backing.kind {
                if let Some(folded) = self.folder.integer_in(value, &backing) {
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
        self.check_overlay_value(&ty, declared.span);
        if constant {
            self.require_constant(value, integer_scalar(&ty));
        }
        self.check_into(value, &ty);
    }

    /// Reports `E10224` or `E10223` when an expression is not foldable at the
    /// width of its constant context.
    pub(super) fn require_constant(&mut self, expr: &Expr, context: Option<ScalarType>) {
        self.folder.require_at(expr, context, self.diagnostics);
    }

    /// Validates the constant-shaped parts of a type.
    ///
    /// An array extent must fold at `usize` width and be greater than zero,
    /// only the outermost extent of a multi-dimensional type may be omitted,
    /// and a generic struct's type argument count must match its declaration.
    ///
    /// Returns false when an extent is present but not constant; the caller
    /// then treats the whole type as opaque so the reported `E10224` does not
    /// cascade into array-literal or bounds errors.
    pub(super) fn validate_type(&mut self, ty: &Type) -> bool {
        let mut valid = true;
        match &ty.kind {
            TypeKind::Array { element, extents } => {
                if !self.validate_type(element) {
                    valid = false;
                }
                for (index, extent) in extents.iter().enumerate() {
                    match &extent.size {
                        Some(size) => {
                            let folded = self.folder.integer_at(size, Some(ScalarType::Usize));
                            match folded {
                                Some(value) => {
                                    if value <= 0 {
                                        self.error(
                                            "E10226",
                                            "array extent must be greater than zero",
                                            "use a positive extent",
                                            extent.span,
                                        );
                                    }
                                }
                                None => {
                                    self.require_constant(size, Some(ScalarType::Usize));
                                    valid = false;
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
            TypeKind::Raw(inner) => {
                if !self.validate_type(inner) {
                    valid = false;
                }
            }
            TypeKind::Function { params, returns } => {
                for param in params {
                    if !self.validate_type(&param.ty) {
                        valid = false;
                    }
                }
                if !self.validate_type(returns) {
                    valid = false;
                }
            }
            TypeKind::Named { name, args } => {
                for arg in args {
                    if !self.validate_type(arg) {
                        valid = false;
                    }
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
        valid
    }
}

/// True when a value of `ty` embeds `target` by value, directly or through
/// other struct fields.
///
/// A `raw` pointer, a handle, a function pointer, and a slice are pointer-sized,
/// so they end the by-value chain; a fixed array and a nested struct continue
/// it. A generic struct's arguments are substituted into its fields before the
/// walk descends, so `Box<S>` with `Box<T> { inner: T }` is seen to embed `S`.
/// The walk is depth-limited so a cycle that does not name `target` cannot
/// loop.
fn embeds_by_value(table: &TypeTable, ty: &Ty, target: &str, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    match &ty.kind {
        TyKind::Struct(name, args) => {
            if name == target {
                return true;
            }
            let Some(fields) = table.struct_fields(name) else {
                return false;
            };
            let params = table.struct_type_params(name).unwrap_or(&[]);
            fields.iter().any(|(_, field)| {
                embeds_by_value(
                    table,
                    &substitute_instance(field, params, args),
                    target,
                    depth + 1,
                )
            })
        }
        // A slice carries a pointer and a length, so only a fixed array embeds
        // its element.
        TyKind::Array { element, extents } => {
            extents.iter().all(Option::is_some)
                && embeds_by_value(table, element, target, depth + 1)
        }
        _ => false,
    }
}

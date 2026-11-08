//! Declaration checking for structs, enums, and module-level bindings.
//!
//! These checks are mostly one-shot: a declaration is validated where it is
//! read, not while a body is walked. Struct fields reject a view — a `str` or a
//! slice — and require at least one field; an enum needs an integer backing type
//! and constant member values; a module-level `const` initializer must fold while
//! a module-level `let` may be computed at run time. The same pass validates the
//! constant-shaped parts of every type, such as array extents and the type
//! arguments of a generic struct.

use crate::ast::{EnumDecl, Expr, ScalarType, StructDecl, Type, TypeKind};
use crate::borrow::is_nullable_capable;
use crate::span::Span;
use crate::types::{
    int_fits, integer_scalar, resolve_ty_with, substitute_instance, Ty, TyKind, TypeTable,
};

use super::Checker;

impl Checker<'_, '_> {
    /// Checks a struct declaration.
    ///
    /// A struct must declare at least one field and no field may be a view: a
    /// `str` or a slice borrows storage owned elsewhere, so a struct that stored
    /// one could outlive it. `static str` is a literal, `cstring` is an FFI
    /// pointer, and a fixed array is owned, so all three stay allowed. A generic
    /// struct may be exported so another module can instantiate it; only a
    /// generic function is rejected under `E10302`. A `volatile` field is
    /// accepted here; the overlay ban is enforced where a value is built.
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
            self.check_nullable_type(&ty, field.name_span);
            if matches!(ty.kind, TyKind::Str) || ty.is_slice() {
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
        self.check_nullable_type(&backing, decl.backing.span);
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
            // An explicit value is checked here independently of later
            // members: a later non-integer member must not hide an earlier
            // out-of-range value. The enum member policy is the target ABI,
            // so the extent folder supplies the same `sizeof` layout the
            // emitted macro and the walk use.
            if let TyKind::Scalar(scalar) = backing.kind {
                if let Some(folded) = self.extent_folder.integer_in(value, &backing) {
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
        // Every auto-incremented member must also fit the backing type. The
        // shared walk computes the sequence once; explicit members were
        // checked above, so only the implicit values are checked here.
        if let Some(values) = self.folder.enum_member_values(&decl.name) {
            if let TyKind::Scalar(scalar) = backing.kind {
                for (member, (_, value)) in decl.members.iter().zip(values.iter()) {
                    if member.value.is_some() {
                        continue;
                    }
                    if !int_fits(*value, scalar) {
                        self.error(
                            "E10202",
                            format!("literal does not fit '{}'", backing.display()),
                            "use a wider type or a hex/binary form",
                            member.name_span,
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
    /// `name_span` is where a nullable declared type reports, matching the
    /// ownership stage's declaration check.
    pub(super) fn check_global(
        &mut self,
        declared: &Type,
        value: &Expr,
        constant: bool,
        name_span: Span,
    ) {
        let ty = self.resolve_type(declared);
        self.check_nullable_type(&ty, name_span);
        self.check_overlay_value(&ty, declared.span);
        // A module `const` initializer is the one constant context; a
        // module-level `let` is a runtime position.
        let saved_const_context = self.in_const_context;
        self.in_const_context = constant;
        if constant {
            self.require_constant(value, integer_scalar(&ty));
        }
        self.check_into_slot(value, &ty);
        self.in_const_context = saved_const_context;
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
        self.validate_type_at(ty, false)
    }

    /// Reports `E10011` when a declared type carries `?` but cannot be empty.
    ///
    /// The declaration sites report here as well as in the ownership stage:
    /// the ownership stage runs only on a program whose type pass is clean, so
    /// a nested type-argument report would otherwise hide the declaration
    /// reports of the same program. The span is the declaration's own name
    /// span, so the site matches the ownership check exactly.
    pub(super) fn check_nullable_type(&mut self, ty: &Ty, span: Span) {
        self.check_nullable_type_shallow(ty, span);
        // A declared type can carry a function signature, a fixed-array
        // element, a raw pointee, or a struct argument. Every nested node
        // faces the same rule, so `func(i32?)` and `Box<i32?>` report too.
        match &ty.kind {
            TyKind::Array { element, .. } | TyKind::Raw(element) => {
                self.check_nullable_type(element, span);
            }
            TyKind::Func(signature) => {
                for param in &signature.params {
                    self.check_nullable_type(&param.ty, span);
                }
                self.check_nullable_type(&signature.returns, span);
            }
            _ => {}
        }
    }

    /// Reports one nullable node without walking its children.
    ///
    /// The type-argument walk visits every node itself, so it uses this form to
    /// keep one report per offending node.
    fn check_nullable_type_shallow(&mut self, ty: &Ty, span: Span) {
        if !ty.nullable || is_nullable_capable(ty) {
            return;
        }
        let base = Ty {
            kind: ty.kind.clone(),
            nullable: false,
        };
        self.error(
            "E10011",
            format!("nullable type is not allowed on '{}'", base.display()),
            "`?` applies to handles, `raw`, `cstring`, and views",
            span,
        );
    }

    /// Validates a type written in a type-argument position.
    ///
    /// A type argument may name a nullable scalar or any other non-capable
    /// type, and no declaration check sees it there: the declaration checks
    /// own a parameter, return, binding, global, or field as a whole. Every
    /// node inside the argument subtree is therefore checked against the
    /// nullability rule, and each offending node reports once at its own span.
    /// A nullable handle, `raw`, `cstring`, or view stays legal.
    pub(super) fn validate_type_argument(&mut self, ty: &Type) -> bool {
        self.validate_type_at(ty, true)
    }

    /// The shared type validation, marked with the position it sits at.
    ///
    /// `argument` marks a node inside a type argument; only there the
    /// nullability rule is applied, so a declared binding or field keeps the
    /// single report its own check produces.
    fn validate_type_at(&mut self, ty: &Type, argument: bool) -> bool {
        let mut valid = true;
        if argument {
            // Classifying a nullable node needs its resolved shape: a handle,
            // `raw`, `cstring`, or view can be empty, a scalar, `bool`, enum,
            // or fixed array cannot.
            let resolved = {
                let table = self.table;
                let type_params = &self.type_params;
                let folder = &mut self.extent_folder;
                resolve_ty_with(ty, type_params, table, &mut |expr| {
                    folder.integer_at(expr, Some(ScalarType::Usize))
                })
            };
            self.check_nullable_type_shallow(&resolved, ty.span);
        }
        match &ty.kind {
            TypeKind::Array { element, extents } => {
                if !self.validate_type_at(element, argument) {
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
                if !self.validate_type_at(inner, argument) {
                    valid = false;
                }
            }
            TypeKind::Function { params, returns } => {
                for param in params {
                    if !self.validate_type_at(&param.ty, argument) {
                        valid = false;
                    }
                }
                if !self.validate_type_at(returns, argument) {
                    valid = false;
                }
            }
            TypeKind::Named { name, args } => {
                for arg in args {
                    if !self.validate_type_at(arg, true) {
                        valid = false;
                    }
                }
                if let Some(last) = name.segments.last() {
                    if self.is_box_name(last) && self.builtin_box_active() && args.len() != 1 {
                        self.error(
                            "E10300",
                            format!(
                                "wrong number of type arguments for 'Box': expected 1, found {}",
                                args.len()
                            ),
                            "list one type argument",
                            ty.span,
                        );
                    }
                    if self.is_box_name(last) && self.builtin_box_active() && args.len() == 1 {
                        self.check_box_element(&args[0], ty.span);
                    }
                    if let Some(params) = self.table.struct_type_params(last) {
                        if self.is_box_name(last) && self.builtin_box_active() {
                            // The built-in meaning wins inside the library;
                            // a same-named user struct's parameter count does
                            // not apply to the library's own `Box<T>`.
                        } else if args.len() != params.len() {
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

    /// Checks one `Box<T>` element for the shapes a box cannot own.
    ///
    /// A box owns its storage for an unbounded time, so a non-static view
    /// (`str`, `cstring`, a slice, a container of such views) could outlive its
    /// origin; an overlay describes memory the program does not own; a closure
    /// is ephemeral. `static str` stays allowed, matching the struct-field rule.
    /// The shared checker also follows struct fields after instance
    /// substitution, so a concrete `Box<str>` born from `Holder<str>` is caught.
    fn check_box_element(&mut self, arg: &Type, span: Span) {
        let resolved = {
            let table = self.table;
            let type_params = &self.type_params;
            let folder = &mut self.extent_folder;
            let module = self.module;
            crate::types::resolve_ty_with_alias(
                arg,
                type_params,
                table,
                &mut |expr| folder.integer_at(expr, Some(ScalarType::Usize)),
                &mut |name| module.import(name).map(|binding| binding.original.clone()),
                module.embedded && module.name == "std.mem",
            )
        };
        if let Some((code, message)) =
            crate::types::box_rule_violation(self.table, &Ty::boxed(resolved))
        {
            let help = match code {
                "E10227" => "box an owner, or use `static str` for literals",
                "E10207" => "access it through a `raw` pointer inside `unsafe`",
                _ => "use a storable type",
            };
            self.error(code, message, help, span);
        }
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

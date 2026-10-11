//! Program-level emission: typedefs, aggregates, prototypes, globals,
//! module initializers, function bodies, and the entry stub.
//!
//! These phases walk the whole program rather than one function body, so they
//! are split from the per-function statement emitter. Each method is a method
//! on [`super::Emitter`] and reads the same collected tables.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{BinaryOp, EnumDecl, Expr, ExprKind, IntrinsicArg, Item, StructDecl, TemplatePiece};
use crate::ir::{GlobalId, Place, PlaceBase, Projection, Value, ValueKind, ValuePiece};
use crate::load::Module;
use crate::profile::Profile;
use crate::types::{resolve_ty, resolve_ty_folded, substitute_instance, view_type, CaptureMode, Ty, TyKind, TypeTable};

use super::function::ArrayTarget;
use super::types::{
    c_type, collect_slice_types, declarator, function_declarator, innermost_is_struct,
    mangle_module, pointer_to_array_declarator, scalar_of, type_key,
};
use super::{Emitter, FuncInfo};

impl<'a> Emitter<'a> {
    // -- top-level sections --------------------------------------------------

    /// Reports a same-named generic function declared in more than one module.
    ///
    /// A generic instance is named `cl_<name>__<typekey>` without a module
    /// qualifier, so two modules that declare a generic of the same name would
    /// bind one emitted symbol and silently miscompile. The collision is not
    /// guessed away: it is reported as a duplicate declaration, which stops the
    /// build before any C is written. A name declared once is unaffected.
    pub(super) fn report_generic_collisions(&mut self) {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for instance in self.ownership.monomorphization.instances() {
            names.insert(instance.function.clone());
        }
        for name in names {
            let mut matches: Vec<&FuncInfo<'a>> = self
                .functions
                .values()
                .filter(|info| info.decl.name == name)
                .collect();
            if matches.len() < 2 {
                continue;
            }
            // The function table is ordered by module name, so the second entry
            // is the later module; reporting on it is deterministic.
            let colliding = matches.remove(1);
            self.diagnostics.push(
                crate::diag::Diagnostic::error(
                    "E11101",
                    format!(
                        "duplicate declaration of '{name}' in module '{}'",
                        colliding.module
                    ),
                    colliding.decl.name_span,
                )
                .with_help("rename one declaration; generic instances share one symbol"),
            );
        }
    }

    /// Writes the comment header. No timestamp or host path appears.
    pub(super) fn emit_header(&mut self) {
        self.push("/* clorraine generated C - do not edit */");
        self.push("");
    }

    /// Writes the fixed-width typedefs and the built-in view structs.
    ///
    /// The Clorraine fixed-width scalars are always declared because no target
    /// header provides them. The AmigaOS ABI typedefs (`BPTR`, `LONG`, and the
    /// rest) are declared for the host and freestanding profiles: the
    /// hosted-amiga shim includes `exec/types.h`, which already declares them,
    /// and re-declaring a typedef with a different underlying type is an error
    /// under `-std=c99`. The freestanding shim includes no header, so the
    /// emitted prototypes need these typedefs.
    pub(super) fn emit_typedefs(&mut self) {
        self.push("typedef signed char i8;");
        self.push("typedef short i16;");
        self.push("typedef int i32;");
        self.push("typedef unsigned char u8;");
        self.push("typedef unsigned short u16;");
        self.push("typedef unsigned int u32;");
        self.push("typedef unsigned int usize;");
        self.push("typedef int isize;");
        self.push("");
        if matches!(self.profile, Profile::Freestanding) {
            // The no-libc stance includes no header, so the boolean type the
            // emitted signatures spell is defined here: one byte, matching
            // the compiler's `bool`.
            self.push("typedef unsigned char bool;");
            self.push("");
        }
        if matches!(self.profile, Profile::Host | Profile::Freestanding) {
            self.push("typedef void *BPTR;");
            self.push("typedef void *APTR;");
            self.push("typedef char *STRPTR;");
            self.push("typedef signed char BYTE;");
            self.push("typedef short WORD;");
            self.push("typedef int LONG;");
            self.push("typedef unsigned char UBYTE;");
            self.push("typedef unsigned short UWORD;");
            self.push("typedef unsigned int ULONG;");
            self.push("typedef int BOOL;");
            self.push("");
        }
        self.push("typedef void *ClString;");
        self.push("");
        self.push("typedef struct { const u8 *ptr; usize len; } ClStr;");
        self.push("static inline ClStr cl_str(const char *text)");
        self.push("{");
        self.push("    ClStr value;");
        self.push("    value.ptr = (const u8 *)text;");
        self.push("    value.len = (usize)strlen(text);");
        self.push("    return value;");
        self.push("}");
        self.push("");
        // The implicit `String`-to-`str` widening views an owned string's
        // bytes through the shim accessors. The emitter declares both here so
        // the helper is self-contained: a program can format a `String` with
        // the compiler-known `text` without importing the text module, and a
        // duplicate compatible prototype is legal C when it does import it.
        self.push("APTR cl_string_ptr(ClString);");
        self.push("ULONG cl_string_len(ClString);");
        self.push("static inline ClStr cl_string_view(ClString string)");
        self.push("{");
        self.push("    ClStr view;");
        self.push("    view.ptr = (const u8 *)cl_string_ptr(string);");
        self.push("    view.len = (usize)cl_string_len(string);");
        self.push("    return view;");
        self.push("}");
        self.push("");
        // A view's empty state is returned by a helper rather than a compound
        // literal, which the C99-subset contract forbids.
        self.push("static inline ClStr cl_zero_ClStr(void)");
        self.push("{");
        self.push("    ClStr value = {0};");
        self.push("    return value;");
        self.push("}");
        self.push("");
        self.emit_handle_typedefs();
    }

    /// Writes one `ClSlice_<key>` typedef per slice element type used.
    ///
    /// The element pointer is mutable so a store through a writable view
    /// compiles. A read-only view is protected by the ownership checks, not by
    /// C `const`, because the same typedef is shared by both lend kinds. The
    /// pointer carries the row extents for a multi-extent element
    /// (`T (*ptr)[M]`), so an indexed access yields a whole row.
    pub(super) fn emit_slice_typedefs(&mut self) {
        let keys = self.slice_element_types();
        let early: Vec<(String, Ty)> = keys
            .into_iter()
            .filter(|(_, element)| !self.slice_row_needs_definition(element))
            .collect();
        self.emit_slice_typedef_group(&early);
    }

    /// Writes the slice typedefs whose row element needs a complete struct.
    ///
    /// A row-element slice spells `ClP (*ptr)[2]`, an array of the element, so
    /// the element type must be complete when the typedef is written. These
    /// typedefs are emitted after the struct definitions; the pointer-element
    /// ones only point at their element and keep their place before them.
    pub(super) fn emit_row_slice_typedefs(&mut self) {
        let keys = self.slice_element_types();
        let late: Vec<(String, Ty)> = keys
            .into_iter()
            .filter(|(_, element)| self.slice_row_needs_definition(element))
            .collect();
        self.emit_slice_typedef_group(&late);
    }

    /// Writes one slice-typedef group: the typedefs, then their zero helpers.
    fn emit_slice_typedef_group(&mut self, keys: &[(String, Ty)]) {
        if keys.is_empty() {
            return;
        }
        for (key, element) in keys {
            let pointer = pointer_to_array_declarator(element, "ptr", &self.sema.table)
                .unwrap_or_else(|| format!("{} *ptr", c_type(element, &self.sema.table)));
            self.push(&format!(
                "typedef struct {{ {pointer}; usize len; }} ClSlice_{key};"
            ));
        }
        for (key, _) in keys {
            self.push(&format!(
                "static inline ClSlice_{key} cl_zero_ClSlice_{key}(void)"
            ));
            self.push("{");
            self.push(&format!("    ClSlice_{key} value = {{0}};"));
            self.push("    return value;");
            self.push("}");
        }
        self.push("");
    }

    /// True when a slice's row-element pointer needs the struct definition.
    ///
    /// The array form of the pointer declarator names the element by value, so
    /// the element must be complete; a plain pointer element is fine with the
    /// forward declaration.
    fn slice_row_needs_definition(&self, element: &Ty) -> bool {
        pointer_to_array_declarator(element, "ptr", &self.sema.table).is_some()
            && innermost_is_struct(element)
    }

    /// The distinct slice element types the program uses, ordered by key.
    pub(super) fn slice_element_types(&self) -> Vec<(String, Ty)> {
        let mut found: BTreeMap<String, Ty> = BTreeMap::new();
        for module in self.loaded.modules.values() {
            for item in &module.items {
                match item {
                    Item::Func(decl) => {
                        let params: Vec<String> =
                            decl.type_params.iter().map(|p| p.name.clone()).collect();
                        for param in &decl.params {
                            let ty = resolve_ty_folded(
                                &param.ty,
                                &params,
                                &self.sema.table,
                                module,
                                self.loaded,
                            );
                            collect_slice_types(&ty, &mut found);
                        }
                        let ty = resolve_ty_folded(
                            &decl.returns,
                            &params,
                            &self.sema.table,
                            module,
                            self.loaded,
                        );
                        collect_slice_types(&ty, &mut found);
                    }
                    Item::GlobalLet(decl) => {
                        let ty = resolve_ty_folded(
                            &decl.ty,
                            &[],
                            &self.sema.table,
                            module,
                            self.loaded,
                        );
                        collect_slice_types(&ty, &mut found);
                        self.collect_init_slice_types(&decl.value, &mut found);
                    }
                    Item::GlobalConst(decl) => {
                        let ty = resolve_ty_folded(
                            &decl.ty,
                            &[],
                            &self.sema.table,
                            module,
                            self.loaded,
                        );
                        collect_slice_types(&ty, &mut found);
                        self.collect_init_slice_types(&decl.value, &mut found);
                    }
                    _ => {}
                }
            }
        }
        // A body can hold a slice local whose element type appears in no
        // signature, so collect from every lowered body too. A generic body's
        // element type is still a parameter here and is skipped; it becomes
        // concrete only where the body is instantiated, below.
        for function in &self.ownership.functions {
            for local in &function.locals {
                collect_slice_types(&local.ty, &mut found);
            }
            for temp in &function.temps {
                collect_slice_types(temp, &mut found);
            }
        }
        // A generic body's slice types become concrete only where it is
        // instantiated, so collect the substituted parameter and return types
        // of every emitted instance, plus the substituted locals and
        // temporaries of its lowered body.
        for instance in self.ownership.monomorphization.instances() {
            let Some(info) = self.first_function_by_name(&instance.function) else {
                continue;
            };
            let subst = self.instance_subst(info.decl, &instance.args);
            let params: Vec<String> = info
                .decl
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect();
            for param in &info.decl.params {
                let ty = self.resolve_in_module(&info.module, &param.ty, &params);
                collect_slice_types(&super::substitute_in(&ty, &subst), &mut found);
            }
            let ret = self.resolve_in_module(&info.module, &info.decl.returns, &params);
            collect_slice_types(&super::substitute_in(&ret, &subst), &mut found);
            let module = info.module.clone();
            let Some(body) = self
                .ownership
                .functions
                .iter()
                .find(|function| function.name == instance.function && function.module == module)
            else {
                continue;
            };
            for local in &body.locals {
                collect_slice_types(&super::substitute_in(&local.ty, &subst), &mut found);
            }
            for temp in &body.temps {
                collect_slice_types(&super::substitute_in(temp, &subst), &mut found);
            }
        }
        found.into_iter().collect()
    }

    /// Records the slice element types a module initializer spells.
    ///
    /// An initializer's own expression types appear in no signature, so an
    /// initializer that builds a view is the only evidence of its element
    /// type; the typedef must exist before the init function that uses it.
    /// A lambda body is lowered as its own function, so its slice types come
    /// from the body collection instead.
    fn collect_init_slice_types(&self, expr: &Expr, found: &mut BTreeMap<String, Ty>) {
        if let Some(ty) = self.expr_type(expr) {
            collect_slice_types(&ty, found);
        }
        match &expr.kind {
            ExprKind::Call { callee, args, .. } => {
                self.collect_init_slice_types(callee, found);
                for arg in args {
                    self.collect_init_slice_types(&arg.value, found);
                }
            }
            ExprKind::Index { base, index } => {
                self.collect_init_slice_types(base, found);
                self.collect_init_slice_types(index, found);
            }
            ExprKind::Member { base, .. } => self.collect_init_slice_types(base, found),
            ExprKind::Unary { operand, .. } => self.collect_init_slice_types(operand, found),
            ExprKind::Binary { left, right, .. } => {
                self.collect_init_slice_types(left, found);
                self.collect_init_slice_types(right, found);
            }
            ExprKind::Assign { target, value, .. } => {
                self.collect_init_slice_types(target, found);
                self.collect_init_slice_types(value, found);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.collect_init_slice_types(condition, found);
                self.collect_init_slice_types(then_value, found);
                self.collect_init_slice_types(else_value, found);
            }
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
                self.collect_init_slice_types(value, found);
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.collect_init_slice_types(&field.value, found);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.collect_init_slice_types(element, found);
                }
                if let Some(fill) = fill {
                    self.collect_init_slice_types(fill, found);
                }
            }
            ExprKind::Unsafe(inner) => self.collect_init_slice_types(inner, found),
            ExprKind::IntrinsicCall { args, .. } => {
                for arg in args {
                    match arg {
                        IntrinsicArg::Expr(inner) => self.collect_init_slice_types(inner, found),
                        IntrinsicArg::Type(written) => {
                            // A written type argument such as `i32[]` needs its
                            // slice typedef exactly like a value position does.
                            let ty = self.resolve_written(written);
                            collect_slice_types(&ty, found);
                        }
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let TemplatePiece::Placeholder(inner) = piece {
                        self.collect_init_slice_types(inner, found);
                    }
                }
            }
            _ => {}
        }
    }

    /// Writes the environment and two-word callable structs of every capturing
    /// lambda.
    ///
    /// The environment holds one field per recorded capture: the copied value
    /// for a `Copy` capture, or a pointer to the captured place for a borrow.
    /// The callable pairs the lifted function pointer with an environment
    /// pointer. A lambda in a generic body gets one pair per enclosing
    /// instance, named with the instance key, so the environment a creation
    /// site builds matches the specialized function it passes to. Both are
    /// emitted before the prototypes, because a closure-typed parameter
    /// appears there.
    pub(super) fn emit_closure_declarations(&mut self) {
        // One environment and callable struct per capturing site, or per
        // specialization when the site's enclosing instance changes the
        // capture or parameter types.
        let mut units: Vec<(usize, Option<(BTreeMap<String, Ty>, String)>)> = Vec::new();
        for emission in self.lifted_emissions() {
            if !emission.capturing {
                continue;
            }
            units.push((emission.site, emission.key.map(|key| (emission.subst, key))));
        }
        for (site, instance) in units {
            let Some(info) = self.sema.closures.get(&site) else {
                continue;
            };
            let subst = instance
                .as_ref()
                .map(|(subst, _)| subst.clone())
                .unwrap_or_default();
            let env_name = match &instance {
                Some((_, key)) => format!("ClEnv_{site}__{key}"),
                None => format!("ClEnv_{site}"),
            };
            let closure_name = match &instance {
                Some((_, key)) => format!("ClClosure_{site}__{key}"),
                None => format!("ClClosure_{site}"),
            };
            self.push(&format!("struct {env_name} {{"));
            for capture in &info.captures {
                let field = &capture.name;
                let captured = super::substitute_in(&capture.ty, &subst);
                let ty = match capture.mode {
                    CaptureMode::Copy => captured,
                    CaptureMode::ReadOnlyBorrow | CaptureMode::WriteBorrow => Ty::raw(captured),
                };
                self.push(&format!("    {};", declarator(&ty, field, &self.sema.table)));
            }
            self.push("};");
            // The field's declarator shares the lifted function's signature
            // builder, so the environment prefix, a by-value fixed-array
            // return's hidden destination, and a nested func-typed return all
            // spell the same way on both sides.
            let params: Vec<String> = info
                .params
                .iter()
                .map(|ty| {
                    declarator(&super::substitute_in(ty, &subst), "", &self.sema.table)
                        .trim()
                        .to_owned()
                })
                .collect();
            let returns = super::substitute_in(&info.returns, &subst);
            let field = function_declarator(
                &returns,
                params,
                "(*fn)",
                Some(format!("struct {env_name} *__env")),
                &self.sema.table,
            );
            self.push(&format!("typedef struct {closure_name} {{"));
            self.push(&format!("    {field};"));
            self.push(&format!("    struct {env_name} *env;"));
            self.push(&format!("}} {closure_name};"));
            self.push("");
        }
    }

    /// Writes an opaque typedef for each user handle with no contract type.
    pub(super) fn emit_handle_typedefs(&mut self) {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for module in self.loaded.modules.values() {
            for item in &module.items {
                if let Item::Handle(decl) = item {
                    if crate::abi::contract_handle(&decl.name).is_none() {
                        names.insert(decl.name.clone());
                    }
                }
            }
        }
        if names.is_empty() {
            return;
        }
        for name in names {
            self.push(&format!("typedef void *Cl{name};"));
        }
        self.push("");
    }

    /// Writes the enum typedefs and member macros.
    ///
    /// An enum typedef precedes every struct definition (a field may name an
    /// enum), every slice typedef (an element may be an enum), and every
    /// function body that reads an enum value.
    pub(super) fn emit_enum_typedefs(&mut self) {
        let enums: Vec<&'a EnumDecl> = self.enums.values().map(|info| info.decl).collect();
        for decl in enums {
            self.emit_enum(decl);
        }
    }

    /// Writes the forward typedef of every emitted struct.
    ///
    /// A non-generic struct is emitted once under its bare name; a generic
    /// struct is emitted once per instantiation the program uses. The forward
    /// typedef declares the tag alone, so a pointer field can name a struct
    /// whose definition follows, and the slice typedefs can name struct
    /// elements whose definitions follow.
    pub(super) fn emit_struct_forward_typedefs(&mut self) {
        let units = self.struct_units();
        if units.is_empty() {
            return;
        }
        for unit in &units {
            self.push(&format!("typedef struct {s} {s};", s = unit.symbol));
        }
        self.push("");
    }

    /// Writes one definition per emitted struct, dependencies first.
    ///
    /// A definition is ordered after every struct it embeds by value, and
    /// after the enum typedefs and slice typedefs it names.
    pub(super) fn emit_struct_definitions(&mut self) {
        let units = self.struct_units();
        for unit in units {
            self.push(&format!("struct {} {{", unit.symbol));
            for (name, ty, volatile) in &unit.fields {
                let qualifier = if *volatile { "volatile " } else { "" };
                self.push(&format!("    {qualifier}{};", declarator(ty, name, &self.sema.table)));
            }
            self.push("};");
            // The empty value of a struct is a zeroed temporary returned
            // through a helper, exactly like the view helpers: the
            // C99-subset contract forbids the compound literal an empty
            // return would otherwise spell.
            self.push(&format!(
                "static inline {} cl_zero_{}(void)",
                unit.symbol, unit.symbol
            ));
            self.push("{");
            self.push(&format!("    {} value = {{0}};", unit.symbol));
            self.push("    return value;");
            self.push("}");
            self.push("");
        }
    }

    /// Builds one [`StructUnit`] per emitted struct definition, in an order
    /// where a struct embedded by value is defined before its user.
    fn struct_units(&self) -> Vec<StructUnit> {
        let mut units: Vec<StructUnit> = Vec::new();
        for &decl in self.structs.values() {
            if decl.type_params.is_empty() {
                units.push(self.struct_unit(decl, &[]));
                continue;
            }
            for instance in self
                .ownership
                .monomorphization
                .struct_instances()
                .iter()
                .filter(|instance| instance.name == decl.name)
            {
                units.push(self.struct_unit(decl, &instance.args));
            }
        }
        order_struct_units(units)
    }

    /// Builds the definition of one struct: a non-generic declaration or one
    /// generic instance with its parameters substituted.
    fn struct_unit(&self, decl: &'a StructDecl, args: &[Ty]) -> StructUnit {
        let params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        let mut fields = Vec::with_capacity(decl.fields.len());
        let symbol = if decl.type_params.is_empty() {
            format!("Cl{}", decl.name)
        } else {
            format!(
                "Cl{}",
                type_key(&Ty::struct_with(decl.name.clone(), args.to_vec()))
            )
        };
        let mut deps: Vec<String> = Vec::new();
        for field in &decl.fields {
            let ty = resolve_ty(&field.ty, &params, &self.sema.table);
            let ty = substitute_instance(&ty, &params, args);
            collect_value_deps(&ty, &mut deps, &self.sema.table);
            fields.push((field.name.clone(), ty, field.volatile));
        }
        StructUnit {
            symbol,
            fields,
            deps,
        }
    }

    /// Writes the enum constants as macros over the backing integer type.
    pub(super) fn emit_enum(&mut self, decl: &EnumDecl) {
        let backing = resolve_ty(&decl.backing, &[], &self.sema.table);
        self.push(&format!("typedef {} Cl{};", c_type(&backing, &self.sema.table), decl.name));
        let mut next: i128 = 0;
        let members: Vec<(String, i128)> = decl
            .members
            .iter()
            .map(|member| {
                let value = member
                    .value
                    .as_ref()
                    .and_then(crate::types::int_literal_value)
                    .unwrap_or(next);
                next = value + 1;
                (member.name.clone(), value)
            })
            .collect();
        for (member, value) in members {
            self.push(&format!("#define {}_{member} {value}", decl.name));
        }
        self.push("");
    }

    /// Writes a prototype for every emitted function and `ffi` declaration.
    pub(super) fn emit_prototypes(&mut self) {
        let mut lines: Vec<String> = Vec::new();
        for info in self.functions.values() {
            if !info.decl.type_params.is_empty() {
                continue;
            }
            lines.push(format!("{};", self.function_signature(&info.module, info.decl)));
        }
        for instance in self.ownership.monomorphization.instances() {
            let Some(info) = self.first_function_by_name(&instance.function) else {
                continue;
            };
            let subst = self.instance_subst(info.decl, &instance.args);
            let symbol = format!(
                "cl_{}__{}",
                instance.function,
                self.instance_typekey(&instance.args)
            );
            lines.push(format!(
                "{};",
                self.signature_with_symbol(&info.module, info.decl, &subst, &symbol)
            ));
        }
        // A lifted lambda is used from the body that writes it, which is
        // printed before the lifted function's own definition, so it needs a
        // prototype too. A specialized prototype installs the instance's
        // substitution so its types match the creation site's value.
        for emission in self.lifted_emissions() {
            let symbol = emission.symbol();
            let saved_subst = std::mem::take(&mut self.subst);
            let saved_key = self.instance_key.take();
            self.subst = emission.subst;
            self.instance_key = emission.key;
            lines.push(format!(
                "{};",
                self.lifted_signature(
                    emission.function,
                    emission.site,
                    emission.capturing,
                    &symbol
                )
            ));
            self.subst = saved_subst;
            self.instance_key = saved_key;
        }
        for module in self.loaded.modules.values() {
            for item in &module.items {
                if let Item::Ffi(block) = item {
                    for signature in &block.decls {
                        lines.push(format!("{};", self.ffi_signature(signature)));
                    }
                }
            }
        }
        if lines.is_empty() {
            return;
        }
        for line in lines {
            self.push(&line);
        }
        self.push("");
    }

    /// Builds the C prototype of one `ffi` declaration.
    ///
    /// An `ffi` signature lowers through the ABI mapping, so a parameter mode
    /// never shapes it: the declared type alone fixes the C type. The
    /// parameters are unnamed because the prototype only declares the ABI.
    fn ffi_signature(&self, signature: &crate::ast::FuncSignature) -> String {
        let ret = resolve_ty(&signature.returns, &[], &self.sema.table);
        let ret_c = self.ffi_c_type(&ret);
        let mut params: Vec<String> = Vec::new();
        for param in &signature.params {
            let ty = resolve_ty(&param.ty, &[], &self.sema.table);
            params.push(self.ffi_c_type(&ty));
        }
        if params.is_empty() {
            params.push("void".to_owned());
        }
        format!("{ret_c} {}({})", signature.name, params.join(", "))
    }

    /// The C ABI type of an `ffi` type, resolving an enum's backing scalar.
    fn ffi_c_type(&self, ty: &Ty) -> String {
        let ty = self.substitute(ty);
        let backing = match &ty.kind {
            TyKind::Enum(name) => self.sema.table.enum_backing(name).and_then(scalar_of),
            _ => None,
        };
        crate::abi::ffi_type_with(&ty, backing)
    }

    /// Writes the module-level variables with empty-state initializers.
    ///
    /// A `volatile let` keeps its C `volatile` qualifier so the compiler does
    /// not fold, cache, or reorder accesses to memory another agent can change.
    pub(super) fn emit_globals(&mut self) {
        if self.globals.is_empty() {
            return;
        }
        let globals: Vec<(String, String, Ty, bool)> = self
            .globals
            .values()
            .map(|global| {
                (
                    global.name.clone(),
                    global.module.clone(),
                    global.ty.clone(),
                    global.volatile,
                )
            })
            .collect();
        for (name, module, ty, volatile) in globals {
            let symbol = self.global_symbol(&module, &name);
            let empty = self.empty_state(&ty);
            let qualifier = if volatile { "volatile " } else { "" };
            self.push(&format!(
                "static {qualifier}{} = {};",
                declarator(&ty, &symbol, &self.sema.table),
                empty
            ));
        }
        self.push("");
    }

    /// Writes the `cl_init_<module>` functions in topological order.
    pub(super) fn emit_init_functions(&mut self) {
        for module_name in self.init_order() {
            self.push(&format!(
                "static void cl_init_{}(void)",
                mangle_module(&module_name)
            ));
            self.push("{");
            self.init_locals = 0;
            for (name, value) in self.global_init_order(&module_name) {
                self.emit_global_init(&module_name, &name, value);
            }
            self.push("}");
            self.push("");
        }
    }

    /// Writes one global's initializer assignment.
    ///
    /// A fixed array has no C assignment form, so its covered initializer
    /// forms fill the target element-wise; every other value takes one
    /// assignment after its guards. A fixed-array call result has no
    /// expression form either, so the initializer binds each one into an init
    /// local before either path uses it.
    pub(super) fn emit_global_init(&mut self, module_name: &str, name: &str, value: &'a Expr) {
        self.emit_line_directive(value.span);
        let target = self.global_symbol(module_name, name);
        let declared_ty = self
            .globals
            .values()
            .find(|global| global.module == module_name && global.name == name)
            .map(|global| global.ty.clone());
        let mut lowered = match self.loaded.module(module_name) {
            Some(module) => self.lower_global(module, value),
            None => Value {
                ty: self.expr_type(value).unwrap_or_else(Ty::unknown),
                kind: ValueKind::Aggregate,
            },
        };
        // A global is a file-scope object, so the place keeps its declared type.
        if lowered.ty.is_unknown() {
            if let Some(ty) = self.expr_type(value) {
                lowered.ty = ty;
            }
        }
        // A null initializer carries no type of its own, so the declared global
        // type supplies the empty state; a view needs its zero view.
        if matches!(lowered.kind, ValueKind::Null) {
            if let Some(declared) = &declared_ty {
                lowered.ty = declared.clone();
            }
        }
        // A call result and the other un-covered array forms keep the single
        // assignment; a value outside the fill forms still cannot spell a
        // fixed-array call result, so each one is bound into an init local
        // before the render.
        if let Some(declared) = &declared_ty {
            if declared.is_fixed_array() && fills_element_wise(&lowered) {
                self.emit_global_array_fill(&target, declared, &lowered);
                return;
            }
        }
        // One left-to-right walk binds every fixed-array call, stores every
        // impure operand at its position, and emits each guard where the
        // operand it checks is evaluated, so evaluation order and per-branch
        // laziness match the function path.
        let lowered = self.materialize_init_value(&lowered, None);
        // A view destination takes an owned `String` value through the view
        // constructor, exactly as the function path's store does; without the
        // wrapper the handle cannot assign to the view.
        let rendered = match (&declared_ty, &self.substitute(&lowered.ty).kind) {
            (Some(declared), TyKind::Handle(handle))
                if handle == "String"
                    && matches!(declared.kind, TyKind::Str | TyKind::StaticStr) =>
            {
                format!("cl_string_view({})", self.value(&lowered))
            }
            _ => self.value(&lowered),
        };
        self.push(&format!("    {target} = {rendered};"));
    }

    /// Fills a fixed-array global element-wise.
    ///
    /// An array value has no C assignment form, so the init function writes
    /// one element per statement, in declaration order. A nested array
    /// recurses one extent per level; a literal's fill covers the elements its
    /// list omits, and an omitted trailing element keeps the zero state the
    /// global declaration starts from. A place value is copied element-wise
    /// from its rendered base, and a conditional value reads its chosen arm
    /// per element from a condition the preparation stored exactly once.
    fn emit_global_array_fill(&mut self, target: &str, ty: &Ty, value: &Value) {
        // The value is prepared once, before any element store, so every
        // condition and run-time index is evaluated exactly once even though
        // the stores read the value once per element.
        let prepared = self.prepare_fill_value(value, None);
        self.emit_filled_array(target, ty, &prepared);
    }

    /// Writes the prepared element stores of a filled array.
    fn emit_filled_array(&mut self, target: &str, ty: &Ty, value: &Value) {
        let TyKind::Array { element, extents } = &ty.kind else {
            return;
        };
        let Some(Some(extent)) = extents.first() else {
            return;
        };
        let row_ty = if extents.len() > 1 {
            Ty::array((**element).clone(), extents[1..].to_vec())
        } else {
            (**element).clone()
        };
        for index in 0..*extent {
            let sub_target = format!("{target}[{index}]");
            let Some(element_value) = fill_element_value(value, index, &row_ty) else {
                continue;
            };
            if row_ty.is_fixed_array() {
                self.emit_filled_array(&sub_target, &row_ty, &element_value);
                continue;
            }
            self.emit_guards(&element_value);
            let text = self.store_text(&element_value, &row_ty);
            self.push(&format!("    {sub_target} = {text};"));
        }
    }

    /// Prepares a fill value so no sub-expression runs more than once.
    ///
    /// The materializing walk runs first, in source order: a call that needs a
    /// place is bound, an impure operand stores, and a lazily evaluated
    /// position keeps its stores under its own guard. Then a conditional's
    /// condition runs exactly once, before the element stores, and a nested
    /// condition or run-time index runs only when its enclosing arm is chosen,
    /// so the prepared value reads an init local per stored operand and each
    /// local's assignment carries its enclosing guard. Every other form is
    /// safe to render per element.
    fn prepare_fill_value(&mut self, value: &Value, guard: Option<&str>) -> Value {
        let value = self.materialize_init_value(value, guard);
        self.prepare_walked_fill_value(&value, guard)
    }

    /// Applies the fill discipline to a value the walk already prepared.
    ///
    /// The walk owns the binds and the single-evaluation stores, so this pass
    /// only adds what the element stores need from an already-walked value:
    /// a conditional's condition stored once for all element reads, and a
    /// call bound into a place so no element re-evaluates it. A traversal
    /// that would re-run the walk would emit its guards a second time.
    fn prepare_walked_fill_value(&mut self, value: &Value, guard: Option<&str>) -> Value {
        match &value.kind {
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                // The walk left the condition alone unless an arm needed it;
                // when it did store the condition, that read is the branch
                // guard, so the element stores and the arms share one
                // evaluation.
                let (local_name, local) = match stored_init_read_name(condition) {
                    Some(name) => (name, (**condition).clone()),
                    None => self.materialize_init_local(condition, guard, "false"),
                };
                let then_guard = match guard {
                    Some(enclosing) => format!("{enclosing} && {local_name}"),
                    None => local_name.clone(),
                };
                let else_guard = match guard {
                    Some(enclosing) => format!("{enclosing} && !{local_name}"),
                    None => format!("!{local_name}"),
                };
                let then_value = self.prepare_walked_fill_value(then_value, Some(&then_guard));
                let else_value = self.prepare_walked_fill_value(else_value, Some(&else_guard));
                Value::new(
                    value.ty.clone(),
                    ValueKind::Conditional {
                        condition: Box::new(local),
                        then_value: Box::new(then_value),
                        else_value: Box::new(else_value),
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.prepare_walked_fill_value(base, guard);
                // The walk stored an impure index; a form it left in place is
                // safe to read per element, so it keeps its spelling.
                let index = if init_operand_is_pure(index) {
                    (**index).clone()
                } else {
                    let (_, local) = self.materialize_init_local(index, guard, "0");
                    local
                };
                Value::new(
                    value.ty.clone(),
                    ValueKind::Index {
                        base: Box::new(base),
                        index: Box::new(index),
                    },
                )
            }
            ValueKind::Member { base, name } => Value::new(
                value.ty.clone(),
                ValueKind::Member {
                    base: Box::new(self.prepare_walked_fill_value(base, guard)),
                    name: name.clone(),
                },
            ),
            // A call has no place form, so the fill binds it into an init
            // local: a fixed-array result writes through the local as its
            // hidden destination, and any other result assigns into it.
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. } => {
                let value = self.prepare_call_arguments(value, guard);
                let zero = self.empty_state(&value.ty);
                let (_, local) = self.materialize_init_local(&value, guard, &zero);
                local
            }
            // An element of a literal is read once per element store, so a
            // condition or run-time index inside it is prepared under the same
            // guard the stores carry; a nested row recurses.
            ValueKind::ArrayLit { elements, fill } => Value::new(
                value.ty.clone(),
                ValueKind::ArrayLit {
                    elements: elements
                        .iter()
                        .map(|element| self.prepare_walked_fill_value(element, guard))
                        .collect(),
                    fill: fill
                        .as_deref()
                        .map(|fill| Box::new(self.prepare_walked_fill_value(fill, guard))),
                },
            ),
            _ => value.clone(),
        }
    }

    /// Stores a walked value into a fresh init local and returns the local's
    /// name and its read value.
    ///
    /// The caller walks the value first, so every bind and operand store is
    /// already in source order. With a guard, the store runs only when the
    /// guard holds — a nested condition or a run-time index is evaluated only
    /// on the path that reaches it — and the local starts at `zero` so reads
    /// on the untaken path stay defined. A fixed-array call result has no C
    /// expression form, so the local is the call's hidden destination: it
    /// starts at the empty state and the call writes into it, with the call's
    /// own argument guards inside the same guard.
    fn materialize_init_local(
        &mut self,
        value: &Value,
        guard: Option<&str>,
        zero: &str,
    ) -> (String, Value) {
        let name = self.next_init_local();
        if self.substitute(&value.ty).is_fixed_array()
            && matches!(
                value.kind,
                ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
            )
        {
            let empty = self.empty_state(&value.ty);
            self.push(&format!(
                "    {} = {empty};",
                declarator(&value.ty, &name, &self.sema.table)
            ));
            let call = self.destination_call(value, &name);
            match guard {
                Some(enclosing) => {
                    // The walk left the guarded argument guards to this store,
                    // so they run only on the path that reaches the call.
                    self.push(&format!("    if ({enclosing}) {{"));
                    self.emit_guards(value);
                    self.push(&format!("        {call};"));
                    self.push("    }");
                }
                None => {
                    self.push(&format!("    {call};"));
                }
            }
        } else {
            match guard {
                Some(enclosing) => {
                    self.push(&format!(
                        "    {} = {zero};",
                        declarator(&value.ty, &name, &self.sema.table)
                    ));
                    // The operand's own guards ride the guarded assignment, so a
                    // bounds or division check runs only on the path that reaches
                    // the operand.
                    let rendered = self.guarded_value(value);
                    self.push(&format!("    if ({enclosing}) {{ {name} = {rendered}; }}"));
                }
                None => {
                    // The walk emitted the value's guards where its operands
                    // are evaluated, so the store only reads them.
                    let rendered = self.value(value);
                    self.push(&format!(
                        "    {} = {rendered};",
                        declarator(&value.ty, &name, &self.sema.table)
                    ));
                }
            }
        }
        let read = Value::new(
            value.ty.clone(),
            ValueKind::Place(Place {
                base: PlaceBase::Local(name.clone()),
                projections: Vec::new(),
                ty: value.ty.clone(),
            }),
        );
        (name, read)
    }

    /// Binds a struct literal whose fields need statements into an init local.
    ///
    /// A compound literal cannot carry an array member initialized from an
    /// expression, so the literal declares a local in the empty state and
    /// stores its fields in source order; the local reads where the literal
    /// stood. The walk already emitted every field operand's guards where the
    /// operand is evaluated, so this store only reads them — the body's field
    /// emitter would place the guards a second time. Under a branch guard the
    /// stores run only on the path that reaches the literal.
    fn materialize_init_literal(&mut self, value: &Value, guard: Option<&str>) -> Value {
        let ty = value.ty.clone();
        let fields = match &value.kind {
            ValueKind::StructLit { fields, .. } => fields.clone(),
            _ => return value.clone(),
        };
        let name = self.next_init_local();
        let empty = self.empty_state(&ty);
        self.push(&format!(
            "    {} = {empty};",
            declarator(&ty, &name, &self.sema.table)
        ));
        let target = Place {
            base: PlaceBase::Local(name.clone()),
            projections: Vec::new(),
            ty: ty.clone(),
        };
        let target_c = self.place(&target);
        match guard {
            Some(enclosing) => {
                self.push(&format!("    if ({enclosing}) {{"));
                self.emit_init_literal_fields(&target_c, &fields);
                self.push("    }");
            }
            None => self.emit_init_literal_fields(&target_c, &fields),
        }
        Value::new(ty, ValueKind::Place(target))
    }

    /// Stores a walked literal's fields into a rendered C lvalue.
    ///
    /// The field rules mirror the body's literal store: a fixed-array field
    /// goes through the fixed-array store core because C cannot assign an
    /// array field, a nested literal that needs stores recurses field by
    /// field, an owned conditional branches through the conditional move, and
    /// every other field stores as one assignment with the view wrap its
    /// destination drives. The guards are not emitted here: the walk already
    /// placed each operand's guard where the operand was evaluated.
    fn emit_init_literal_fields(&mut self, target_c: &str, fields: &[(String, Value)]) {
        for (field_name, field) in fields {
            let field_ty = self.substitute(&field.ty);
            let field_c = format!("{target_c}.{field_name}");
            if field_ty.is_fixed_array() {
                let field_target = ArrayTarget::object(&field_c);
                self.emit_fixed_array_assign_to(&field_target, &field_ty, field, false);
                continue;
            }
            if let ValueKind::StructLit { fields: inner, .. } = &field.kind {
                if self.struct_literal_needs_stores(inner) {
                    self.emit_init_literal_fields(&field_c, inner);
                    continue;
                }
            }
            if self.needs_drop(&field_ty) {
                if let ValueKind::Conditional {
                    condition,
                    then_value,
                    else_value,
                } = &field.kind
                {
                    self.emit_conditional_move_assign(
                        &field_c,
                        &field_ty,
                        condition,
                        then_value,
                        else_value,
                    );
                    continue;
                }
            }
            let lowered = self.store_text(field, &field_ty);
            self.push(&format!("    {field_c} = {lowered};"));
            if self.needs_drop(&field_ty) {
                self.emit_arm_source_moves(field, &field_ty);
            }
        }
    }

    /// Renders a call whose result lands in `destination`.
    ///
    /// A fixed-array result passes the destination as the callee's trailing
    /// hidden argument; the lifted-closure spelling passes it after the
    /// environment and the lambda's own arguments. Any other value renders as
    /// the destination itself, so the renderer stays total.
    fn destination_call(&self, value: &Value, destination: &str) -> String {
        match &value.kind {
            ValueKind::Call {
                callee,
                type_args,
                args,
                receiver,
            } => self.call_with_destination(
                callee,
                type_args,
                args,
                receiver.as_deref(),
                destination,
            ),
            ValueKind::ClosureCall { callable, args } => {
                self.closure_call_with_destination(callable, args, Some(destination))
            }
            _ => destination.to_owned(),
        }
    }

    /// Materializes one module-initializer value for emission.
    ///
    /// The walk visits each sub-expression in source evaluation order and
    /// prepares it where it stands:
    ///
    /// - a fixed-array-returning call has no C expression form — the callee
    ///   writes through a hidden destination — so it binds into a fresh init
    ///   local and the local reads where the call stood;
    /// - an impure index, divisor, or cast operand stores once into an init
    ///   local, and the guard that checks it is emitted right there, so the
    ///   guard and the read share one evaluation and a later operand's effects
    ///   cannot run before an earlier operand's trap;
    /// - a conditional whose part binds or stores keeps the lazily evaluated
    ///   position lazy: the condition stores once and each part runs under its
    ///   branch guard;
    /// - a short-circuit operand whose right side binds or stores restructures
    ///   so the right side runs under the left operand's decision, with the
    ///   operator's left-decided result (`false` for `&&`, `true` for `||`)
    ///   when the left side already decides;
    /// - array-literal elements, struct-literal fields, template placeholders,
    ///   call arguments, and intrinsic arguments are walked too, because a
    ///   destination-less call or a double-evaluated operand can hide in any
    ///   of them.
    ///
    /// A part that needs nothing keeps its spelling, so a program the batch
    /// does not affect re-emits byte-identically.
    fn materialize_init_value(&mut self, value: &Value, guard: Option<&str>) -> Value {
        match &value.kind {
            ValueKind::Call {
                callee,
                type_args,
                args,
                receiver,
            } => {
                let receiver = receiver
                    .as_deref()
                    .map(|inner| Box::new(self.materialize_init_value(inner, guard)));
                let args = args
                    .iter()
                    .map(|arg| self.materialize_init_value(arg, guard))
                    .collect();
                let rewritten = Value::new(
                    value.ty.clone(),
                    ValueKind::Call {
                        callee: callee.clone(),
                        type_args: type_args.clone(),
                        args,
                        receiver,
                    },
                );
                if self.substitute(&value.ty).is_fixed_array() {
                    let zero = self.empty_state(&value.ty);
                    let (_, local) = self.materialize_init_local(&rewritten, guard, &zero);
                    return local;
                }
                rewritten
            }
            ValueKind::ClosureCall { callable, args } => {
                let callable = Box::new(self.materialize_init_value(callable, guard));
                let args = args
                    .iter()
                    .map(|arg| self.materialize_init_value(arg, guard))
                    .collect();
                let rewritten = Value::new(
                    value.ty.clone(),
                    ValueKind::ClosureCall { callable, args },
                );
                if self.substitute(&value.ty).is_fixed_array() {
                    let zero = self.empty_state(&value.ty);
                    let (_, local) = self.materialize_init_local(&rewritten, guard, &zero);
                    return local;
                }
                rewritten
            }
            ValueKind::Intrinsic {
                name,
                args,
                type_arg,
            } => {
                let mut args: Vec<Value> = args
                    .iter()
                    .map(|arg| self.materialize_init_value(arg, guard))
                    .collect();
                if name == "slice" {
                    // The rendered view builder reads its offset operands
                    // twice, so an operand with effects stores once, on the
                    // path that reaches the construction; a fixed-array
                    // operand keeps its form (the renderer uses its decay).
                    for arg in args.iter_mut() {
                        *arg = self.store_impure_intrinsic_arg(arg, guard);
                    }
                    // The view builder's slice guard checks the offsets
                    // against the source extent; it runs where the
                    // construction is evaluated, exactly as the function path
                    // emits it, and a lazily evaluated construction embeds it
                    // through the guarded renderer instead.
                    if guard.is_none() {
                        self.emit_slice_guard(&args);
                    }
                }
                Value::new(
                    value.ty.clone(),
                    ValueKind::Intrinsic {
                        name: name.clone(),
                        args,
                        type_arg: type_arg.clone(),
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.materialize_init_value(base, guard);
                let index = self.materialize_init_value(index, guard);
                let index = self.store_impure_init_operand(&index, guard);
                if guard.is_none() {
                    self.emit_bounds_guard(&base, &index);
                }
                Value::new(
                    value.ty.clone(),
                    ValueKind::Index {
                        base: Box::new(base),
                        index: Box::new(index),
                    },
                )
            }
            ValueKind::Member { base, name } => Value::new(
                value.ty.clone(),
                ValueKind::Member {
                    base: Box::new(self.materialize_init_value(base, guard)),
                    name: name.clone(),
                },
            ),
            ValueKind::Unary { op, operand } => Value::new(
                value.ty.clone(),
                ValueKind::Unary {
                    op: *op,
                    operand: Box::new(self.materialize_init_value(operand, guard)),
                },
            ),
            ValueKind::Binary { op, left, right } => {
                if matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr)
                    && self.init_value_needs_materialization(right)
                {
                    // The right operand runs only when the left side decides,
                    // so its binds and stores live inside that decision and the
                    // operator's result reads a local. The left side always
                    // evaluates — under the enclosing guard when there is one
                    // — and the left-decided value stands when the right side
                    // is skipped.
                    let left = self.materialize_init_value(left, guard);
                    let left_c = self.guarded_value(&left);
                    // The enclosing guard stays a real conjunct, so a skipped
                    // enclosing choice never reaches the right side: `&&`
                    // decides on the left operand, `||` on its negation.
                    let (zero, decision) = match op {
                        BinaryOp::LogicalAnd => {
                            let decision = match guard {
                                Some(enclosing) => format!("{enclosing} && {left_c}"),
                                None => left_c,
                            };
                            ("0", decision)
                        }
                        _ => {
                            let decision = match guard {
                                Some(enclosing) => format!("{enclosing} && !({left_c})"),
                                None => format!("!({left_c})"),
                            };
                            ("1", decision)
                        }
                    };
                    let name = self.next_init_local();
                    self.push(&format!(
                        "    {} = {zero};",
                        declarator(&value.ty, &name, &self.sema.table)
                    ));
                    self.push(&format!("    if ({decision}) {{"));
                    let right = self.materialize_init_value(right, None);
                    let rendered = self.value(&right);
                    self.push(&format!("        {name} = {rendered};"));
                    self.push("    }");
                    return Value::new(
                        value.ty.clone(),
                        ValueKind::Place(Place {
                            base: PlaceBase::Local(name),
                            projections: Vec::new(),
                            ty: value.ty.clone(),
                        }),
                    );
                }
                let left = self.materialize_init_value(left, guard);
                let right = if matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                    // Nothing in the right side needs preparing, so the lazy
                    // form keeps its spelling and its guards stay embedded.
                    (**right).clone()
                } else {
                    let right = self.materialize_init_value(right, guard);
                    let right = if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                        self.store_impure_init_operand(&right, guard)
                    } else {
                        right
                    };
                    if guard.is_none() && matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                        self.emit_division_guard(&left, &right);
                    }
                    right
                };
                Value::new(
                    value.ty.clone(),
                    ValueKind::Binary {
                        op: *op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                )
            }
            ValueKind::Cast { to, value: inner } => {
                let inner = self.materialize_init_value(inner, guard);
                let inner = self.store_impure_init_operand(&inner, guard);
                if guard.is_none() {
                    let target = self.substitute(&value.ty);
                    let source = self.substitute(&inner.ty);
                    if target.is_integer() && source.is_numeric() && !source.is_integer() {
                        self.emit_float_guard(&inner, &target);
                    }
                }
                Value::new(
                    value.ty.clone(),
                    ValueKind::Cast {
                        to: to.clone(),
                        value: Box::new(inner),
                    },
                )
            }
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let condition = self.materialize_init_value(condition, guard);
                let arms_need = self.init_value_needs_materialization(then_value)
                    || self.init_value_needs_materialization(else_value);
                if !arms_need {
                    return Value::new(
                        value.ty.clone(),
                        ValueKind::Conditional {
                            condition: Box::new(condition),
                            then_value: then_value.clone(),
                            else_value: else_value.clone(),
                        },
                    );
                }
                // A part binds or stores, so the condition stores once and
                // each part runs under its branch guard: the binds and stores
                // reach the parts only on the chosen path.
                let (local_name, local) = self.materialize_init_local(&condition, guard, "false");
                let (then_guard, else_guard) = match guard {
                    Some(enclosing) => (
                        format!("{enclosing} && {local_name}"),
                        format!("{enclosing} && !{local_name}"),
                    ),
                    None => (local_name.clone(), format!("!{local_name}")),
                };
                let then_value = self.materialize_init_value(then_value, Some(&then_guard));
                let else_value = self.materialize_init_value(else_value, Some(&else_guard));
                Value::new(
                    value.ty.clone(),
                    ValueKind::Conditional {
                        condition: Box::new(local),
                        then_value: Box::new(then_value),
                        else_value: Box::new(else_value),
                    },
                )
            }
            ValueKind::ArrayLit { elements, fill } => Value::new(
                value.ty.clone(),
                ValueKind::ArrayLit {
                    elements: elements
                        .iter()
                        .map(|element| self.materialize_init_value(element, guard))
                        .collect(),
                    fill: fill
                        .as_deref()
                        .map(|fill| Box::new(self.materialize_init_value(fill, guard))),
                },
            ),
            ValueKind::StructLit { name, fields } => {
                let walked = Value::new(
                    value.ty.clone(),
                    ValueKind::StructLit {
                        name: name.clone(),
                        fields: fields
                            .iter()
                            .map(|(field_name, field_value)| {
                                (
                                    field_name.clone(),
                                    self.materialize_init_value(field_value, guard),
                                )
                            })
                            .collect(),
                    },
                );
                // A literal whose fields need statements (a fixed-array field,
                // a deferred fill, a nested literal) has no C expression form:
                // a compound literal cannot carry an array member initialized
                // from an expression. Bind it into an init local and store the
                // fields per statement, as the body's assignment path does.
                match &walked.kind {
                    ValueKind::StructLit { fields, .. } if self.struct_literal_needs_stores(fields) => {
                        self.materialize_init_literal(&walked, guard)
                    }
                    _ => walked,
                }
            }
            ValueKind::Template(pieces) => Value::new(
                value.ty.clone(),
                ValueKind::Template(
                    pieces
                        .iter()
                        .map(|piece| match piece {
                            ValuePiece::Literal(text) => ValuePiece::Literal(text.clone()),
                            ValuePiece::Placeholder(inner) => {
                                ValuePiece::Placeholder(self.materialize_init_value(inner, guard))
                            }
                        })
                        .collect(),
                ),
            ),
            _ => value.clone(),
        }
    }

    /// Stores an impure operand once into an init local.
    fn store_impure_init_operand(&mut self, value: &Value, guard: Option<&str>) -> Value {
        if init_operand_is_pure(value) {
            return value.clone();
        }
        let zero = self.empty_state(&value.ty);
        let (_, local) = self.materialize_init_local(value, guard, &zero);
        local
    }

    /// Stores one impure intrinsic operand into an init local.
    ///
    /// The view builder reads an offset operand twice, so an operand with
    /// effects must run once before the construction; a pure operand and a
    /// fixed-array operand keep their form, because C cannot assign an array
    /// to a temporary and the renderer uses the array's pointer decay.
    fn store_impure_intrinsic_arg(&mut self, value: &Value, guard: Option<&str>) -> Value {
        if init_operand_is_pure(value) || value.ty.is_fixed_array() {
            return value.clone();
        }
        let zero = self.empty_state(&value.ty);
        let (_, local) = self.materialize_init_local(value, guard, &zero);
        local
    }

    /// Rebuilds a call whose parts the walk already prepared.
    ///
    /// The walk visited the receiver and every argument in source order, so
    /// this pass only re-applies the fill discipline the arguments need; it
    /// never re-runs the walk, which would emit their guards a second time.
    fn prepare_call_arguments(&mut self, value: &Value, guard: Option<&str>) -> Value {
        match &value.kind {
            ValueKind::Call {
                callee,
                type_args,
                args,
                receiver,
            } => {
                let receiver = receiver
                    .as_deref()
                    .map(|inner| Box::new(self.prepare_walked_fill_value(inner, guard)));
                let args = args
                    .iter()
                    .map(|arg| self.prepare_walked_fill_value(arg, guard))
                    .collect();
                Value::new(
                    value.ty.clone(),
                    ValueKind::Call {
                        callee: callee.clone(),
                        type_args: type_args.clone(),
                        args,
                        receiver,
                    },
                )
            }
            ValueKind::ClosureCall { callable, args } => {
                let callable = Box::new(self.prepare_walked_fill_value(callable, guard));
                let args = args
                    .iter()
                    .map(|arg| self.prepare_walked_fill_value(arg, guard))
                    .collect();
                Value::new(value.ty.clone(), ValueKind::ClosureCall { callable, args })
            }
            _ => value.clone(),
        }
    }

    /// True when the materializing walk would bind or store inside a value.
    ///
    /// The walk uses the answer to decide whether a lazily evaluated position
    /// — a conditional's arms or a short-circuit right operand — needs its
    /// guard structured, without emitting anything. The predicate mirrors the
    /// walk's own triggers: a fixed-array call result, an impure index,
    /// divisor, or cast operand, and the same recursively. A part the walk
    /// would leave alone answers `false`, so its lazy form keeps its spelling.
    fn init_value_needs_materialization(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Call { args, receiver, .. } => {
                self.substitute(&value.ty).is_fixed_array()
                    || receiver
                        .as_deref()
                        .is_some_and(|receiver| self.init_value_needs_materialization(receiver))
                    || args.iter().any(|arg| self.init_value_needs_materialization(arg))
            }
            ValueKind::ClosureCall { callable, args } => {
                self.substitute(&value.ty).is_fixed_array()
                    || self.init_value_needs_materialization(callable)
                    || args.iter().any(|arg| self.init_value_needs_materialization(arg))
            }
            ValueKind::Intrinsic { name, args, .. } => {
                // The walk stores an impure slice operand at the construction
                // site, so a lazily evaluated view whose offset is impure
                // needs the same structured guard as any other store.
                args.iter().any(|arg| self.init_value_needs_materialization(arg))
                    || (name == "slice"
                        && args
                            .iter()
                            .any(|arg| !init_operand_is_pure(arg) && !arg.ty.is_fixed_array()))
            }
            ValueKind::Index { base, index } => {
                self.init_value_needs_materialization(base)
                    || self.init_value_needs_materialization(index)
                    || !init_operand_is_pure(index)
            }
            ValueKind::Member { base, .. } => self.init_value_needs_materialization(base),
            ValueKind::Unary { operand, .. } => self.init_value_needs_materialization(operand),
            ValueKind::Binary { op, left, right } => {
                self.init_value_needs_materialization(left)
                    || self.init_value_needs_materialization(right)
                    || (matches!(op, BinaryOp::Div | BinaryOp::Rem)
                        && !init_operand_is_pure(right))
            }
            ValueKind::Cast { value: inner, .. } => {
                self.init_value_needs_materialization(inner) || !init_operand_is_pure(inner)
            }
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.init_value_needs_materialization(condition)
                    || self.init_value_needs_materialization(then_value)
                    || self.init_value_needs_materialization(else_value)
            }
            ValueKind::StructLit { fields, .. } => {
                // A literal whose fields need statements is bound into an init
                // local wherever it stands, so a lazily evaluated position
                // prepares it like a call that writes through a destination.
                self.struct_literal_needs_stores(fields)
                    || fields
                        .iter()
                        .any(|(_, field)| self.init_value_needs_materialization(field))
            }
            ValueKind::ArrayLit { elements, fill } => {
                elements
                    .iter()
                    .any(|element| self.init_value_needs_materialization(element))
                    || fill
                        .as_deref()
                        .is_some_and(|fill| self.init_value_needs_materialization(fill))
            }
            ValueKind::Template(pieces) => pieces.iter().any(|piece| match piece {
                ValuePiece::Literal(_) => false,
                ValuePiece::Placeholder(inner) => self.init_value_needs_materialization(inner),
            }),
            _ => false,
        }
    }

    /// The C name of the next init-function local.
    fn next_init_local(&mut self) -> String {
        let name = format!("__cl_itmp{}", self.init_locals);
        self.init_locals += 1;
        name
    }

    /// Lowers a module-level initializer expression to a typed IR value.
    ///
    /// Module initialization is a program-level phase, so it reuses the IR
    /// value renderer instead of the per-function body emitter. Only the forms
    /// an initializer can spell are built here; a global name, whether bare,
    /// field-projected, or written as a full module path, becomes a global
    /// place so the renderer spells its mangled symbol, and an enum member
    /// keeps its dotted path.
    fn lower_global(&mut self, module: &Module, expr: &Expr) -> Value {
        let ty = self.expr_type(expr).unwrap_or_else(Ty::unknown);
        let kind = match &expr.kind {
            ExprKind::Int(value) => ValueKind::Int(*value as i128),
            ExprKind::Float(value) => ValueKind::Float(*value),
            ExprKind::Char(byte) => ValueKind::Char(*byte),
            ExprKind::Bool(value) => ValueKind::Bool(*value),
            ExprKind::Null => ValueKind::Null,
            ExprKind::Str(bytes) => ValueKind::Str(String::from_utf8_lossy(bytes).into_owned()),
            ExprKind::Name(name) => {
                match crate::init_graph::resolve_name_global(self.loaded, module, name) {
                    Some(global) => ValueKind::Place(
                        Place::global(GlobalId::new(global.module, global.name))
                            .with_type(ty.clone()),
                    ),
                    None => ValueKind::Path(vec![name.clone()]),
                }
            }
            ExprKind::Qualified(name) => {
                match crate::init_graph::resolve_global(self.loaded, module, name) {
                    Some(global) => {
                        let identity = GlobalId::new(global.module, global.name);
                        let projections =
                            global.fields.into_iter().map(Projection::Field).collect();
                        ValueKind::Place(Place {
                            base: PlaceBase::Global(identity),
                            projections,
                            ty: ty.clone(),
                        })
                    }
                    None => ValueKind::Path(name.segments.clone()),
                }
            }
            ExprKind::Member { base, name } => ValueKind::Member {
                base: Box::new(self.lower_global(module, base)),
                name: name.clone(),
            },
            ExprKind::Index { base, index } => ValueKind::Index {
                base: Box::new(self.lower_global(module, base)),
                index: Box::new(self.lower_global(module, index)),
            },
            ExprKind::Unary { op, operand } => ValueKind::Unary {
                op: *op,
                operand: Box::new(self.lower_global(module, operand)),
            },
            ExprKind::Binary { op, left, right } => ValueKind::Binary {
                op: *op,
                left: Box::new(self.lower_global(module, left)),
                right: Box::new(self.lower_global(module, right)),
            },
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => ValueKind::Cast {
                to: ty.display(),
                value: Box::new(self.lower_global(module, value)),
            },
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => ValueKind::Conditional {
                condition: Box::new(self.lower_global(module, condition)),
                then_value: Box::new(self.lower_global(module, then_value)),
                else_value: Box::new(self.lower_global(module, else_value)),
            },
            ExprKind::StructLiteral { name, fields, .. } => ValueKind::StructLit {
                name: name.segments.last().cloned().unwrap_or_default(),
                fields: fields
                    .iter()
                    .map(|field| (field.name.clone(), self.lower_global(module, &field.value)))
                    .collect(),
            },
            ExprKind::ArrayLiteral { elements, fill } => ValueKind::ArrayLit {
                elements: elements
                    .iter()
                    .map(|element| self.lower_global(module, element))
                    .collect(),
                fill: fill
                    .as_ref()
                    .map(|fill| Box::new(self.lower_global(module, fill))),
            },
            ExprKind::Unsafe(inner) => return self.lower_global(module, inner),
            ExprKind::IntrinsicCall { name, args } => {
                let mut values = Vec::new();
                let mut type_arg = None;
                for arg in args {
                    match arg {
                        IntrinsicArg::Expr(inner) => values.push(self.lower_global(module, inner)),
                        IntrinsicArg::Type(written) => type_arg = Some(self.resolve_written(written)),
                    }
                }
                ValueKind::Intrinsic {
                    name: name.clone(),
                    args: values,
                    type_arg,
                }
            }
            ExprKind::Template(pieces) => ValueKind::Template(
                pieces
                    .iter()
                    .map(|piece| match piece {
                        TemplatePiece::Literal(bytes) => {
                            ValuePiece::Literal(String::from_utf8_lossy(bytes).into_owned())
                        }
                        TemplatePiece::Placeholder(inner) => {
                            ValuePiece::Placeholder(self.lower_global(module, inner))
                        }
                    })
                    .collect(),
            ),
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                if let Some(intrinsic) =
                    self.lower_global_intrinsic(module, callee, type_args, args)
                {
                    return intrinsic;
                }
                // A lambda-literal callee resolves to the lifted function the
                // ownership stage recorded for the node (a module initializer
                // has no enclosing body to lift it).
                if let ExprKind::Lambda { .. } = &callee.kind {
                    if let Some(&site) = self
                        .ownership
                        .init_lambdas
                        .get(&(callee.as_ref() as *const Expr as usize))
                    {
                        let lambda_ty = self.expr_type(callee).unwrap_or_else(Ty::unknown);
                        return Value::new(
                            ty,
                            ValueKind::ClosureCall {
                                callable: Box::new(Value::new(
                                    lambda_ty,
                                    ValueKind::Closure {
                                        site,
                                        captures: Vec::new(),
                                    },
                                )),
                                args: args
                                    .iter()
                                    .map(|arg| self.lower_global(module, &arg.value))
                                    .collect(),
                            },
                        );
                    }
                }
                // A func-typed global or field names the callable itself; the
                // callable carries its own signature, so the argument
                // conversions run exactly as a body call's.
                if let Some(callable) = self.init_callable_value(module, callee) {
                    return Value::new(
                        ty,
                        ValueKind::Call {
                            callee: callee_name(callee).unwrap_or_default(),
                            type_args: type_args
                                .iter()
                                .map(|ty| self.resolve_written(ty))
                                .collect(),
                            args: args
                                .iter()
                                .map(|arg| self.lower_global(module, &arg.value))
                                .collect(),
                            receiver: Some(Box::new(callable)),
                        },
                    );
                }
                // Any other dot call passes its receiver as the first
                // argument, like the body's dot-call path; a path that names a
                // module or a declared function has no receiver.
                let mut written_args: Vec<Value> = Vec::new();
                if let Some(receiver) = self.global_dot_receiver(module, callee) {
                    written_args.push(receiver);
                }
                for arg in args {
                    written_args.push(self.lower_global(module, &arg.value));
                }
                ValueKind::Call {
                    callee: callee_name(callee).unwrap_or_default(),
                    type_args: type_args.iter().map(|ty| self.resolve_written(ty)).collect(),
                    args: written_args,
                    receiver: None,
                }
            }
            ExprKind::Lambda { .. } => {
                // A lifted initializer lambda renders its function identity,
                // exactly as a body lambda's capture-less value does.
                match self
                    .ownership
                    .init_lambdas
                    .get(&(expr as *const Expr as usize))
                {
                    Some(site) => ValueKind::Closure {
                        site: *site,
                        captures: Vec::new(),
                    },
                    None => ValueKind::Aggregate,
                }
            }
            _ => ValueKind::Aggregate,
        };
        Value { ty, kind }
    }

    /// The callable value a func-typed callee names, carrying its signature.
    ///
    /// Mirrors the body's call resolution: a bare or qualified name resolving
    /// to a func-typed module global, or a value projection resolving to a
    /// func-typed field, is the callable; the value's type is the resolved
    /// func signature, so the call renders through the callable and the
    /// argument conversions apply. A declared function path and a module path
    /// are not callables — they keep the named-call lowering.
    fn init_callable_value(&mut self, module: &Module, callee: &Expr) -> Option<Value> {
        let global = match &callee.kind {
            ExprKind::Name(name) => {
                crate::init_graph::resolve_name_global(self.loaded, module, name)
            }
            ExprKind::Qualified(name) => {
                crate::init_graph::resolve_global(self.loaded, module, name)
            }
            _ => None,
        };
        if let Some(global) = global {
            let identity = GlobalId::new(global.module, global.name);
            if let Some(info) = self.globals.get(&identity) {
                let func_ty = self.substitute(&info.ty);
                if matches!(func_ty.kind, TyKind::Func(_)) {
                    return Some(Value::new(
                        func_ty.clone(),
                        ValueKind::Place(Place::global(identity).with_type(func_ty)),
                    ));
                }
            }
        }
        // A projection (a member or element read) resolving to a func-typed
        // field is the callable; a path or literal is not.
        let read = self.lower_global(module, callee);
        if !matches!(
            read.kind,
            ValueKind::Place(_) | ValueKind::Member { .. } | ValueKind::Index { .. }
        ) {
            return None;
        }
        let resolved = self.resolved_value_ty(&read);
        if matches!(resolved.kind, TyKind::Func(_)) {
            let mut callable = read;
            callable.ty = resolved;
            Some(callable)
        } else {
            None
        }
    }

    /// The receiver value of a module-level dot callee.
    ///
    /// The receiver is the callee's base for a member spelling and the
    /// segments before the called name for a qualified spelling. Both the
    /// dot-call receiver argument and the structural `len` fold read this
    /// value, and the read carries the projected field's type: the fold
    /// reports the field's extent and the call's argument conversions see the
    /// receiver's own signature.
    fn global_dot_receiver(&mut self, module: &Module, callee: &Expr) -> Option<Value> {
        match &callee.kind {
            ExprKind::Member { base, .. } => Some(self.lower_global(module, base)),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                self.global_projected_read(module, &name.segments[..name.segments.len() - 1])
            }
            _ => None,
        }
    }

    /// The projected read a module-level qualified receiver path names.
    ///
    /// Resolves only the head segment as a value and walks the rest as field
    /// projections, so the read carries the projected field's type and a
    /// module path that spells a segment like an exported global is never a
    /// value. A head that names no global (a module qualifier or a declared
    /// function) has no read.
    fn global_projected_read(&mut self, module: &Module, segments: &[String]) -> Option<Value> {
        let (head, rest) = segments.split_first()?;
        let global = crate::init_graph::resolve_name_global(self.loaded, module, head)?;
        let identity = GlobalId::new(global.module, global.name);
        let mut ty = self
            .globals
            .get(&identity)
            .map(|info| info.ty.clone())
            .unwrap_or_else(Ty::unknown);
        let mut projections: Vec<Projection> = Vec::new();
        for segment in rest {
            projections.push(Projection::Field(segment.clone()));
            ty = self.field_type(&ty, segment);
        }
        Some(Value::new(
            ty.clone(),
            ValueKind::Place(Place {
                base: PlaceBase::Global(identity),
                projections,
                ty,
            }),
        ))
    }

    /// Lowers a module-level call that names a compiler intrinsic.
    ///
    /// Mirrors the function path's rule: a user or `ffi` declaration of the
    /// name keeps its own symbol, and only the receiver-taking `len` spelling
    /// and the three view builders lower as intrinsics. The emission walk
    /// stores an impure operand once where the construction is evaluated, so
    /// no operand binds here, where the enclosing choice is not yet known.
    fn lower_global_intrinsic(
        &mut self,
        module: &Module,
        callee: &Expr,
        type_args: &[crate::ast::Type],
        args: &[crate::ast::Arg],
    ) -> Option<Value> {
        let name = self.global_intrinsic_call(module, callee)?;
        if name == "len" {
            let receiver = self.global_dot_receiver(module, callee)?;
            return Some(Value::new(
                Ty::usize_ty(),
                ValueKind::Intrinsic {
                    name: "len".to_owned(),
                    args: vec![receiver],
                    type_arg: None,
                },
            ));
        }
        let explicit: Vec<Ty> = type_args.iter().map(|ty| self.resolve_written(ty)).collect();
        let lowered: Vec<Value> = args
            .iter()
            .map(|arg| self.lower_global(module, &arg.value))
            .collect();
        // The view's element is one step of iteration over the source: whole
        // rows for a multi-extent source, the scalar for a single-extent one.
        let source_element = lowered
            .first()
            .map(|value| self.substitute(&value.ty).iterated_element());
        // An explicit type argument fixes the view's element, so it must name
        // the source's element exactly, exactly as the function path requires.
        if name == "slice" {
            if let (Some(explicit_ty), Some(source_element)) =
                (explicit.first(), source_element.as_ref())
            {
                if !source_element.is_unknown() && explicit_ty != source_element {
                    if let Some(source) = lowered.first() {
                        self.diagnostics.push(
                            crate::diag::Diagnostic::error(
                                "E10200",
                                format!(
                                    "cannot convert '{}' to '{}' implicitly",
                                    self.substitute(&source.ty).display(),
                                    view_type(explicit_ty.clone()).display()
                                ),
                                type_args[0].span,
                            )
                            .with_help("use an explicit cast `T(expr)`"),
                        );
                    }
                }
            }
        }
        let type_arg = explicit
            .first()
            .cloned()
            .or_else(|| source_element.filter(|element| !element.is_unknown()));
        let is_raw_str = name == "rawStr";
        let result = if is_raw_str {
            Ty {
                kind: TyKind::Str,
                nullable: false,
            }
        } else {
            view_type(type_arg.clone().unwrap_or_else(Ty::unknown))
        };
        Some(Value::new(
            result,
            ValueKind::Intrinsic {
                name,
                args: lowered,
                type_arg: if is_raw_str { None } else { type_arg },
            },
        ))
    }

    /// The compiler intrinsic a module-level call names, when it names one.
    ///
    /// A user or `ffi` declaration of the name keeps its own symbol, exactly
    /// as the function path resolves the call; the view builders and the
    /// receiver-taking `len` are the intrinsics a module initializer can
    /// spell.
    fn global_intrinsic_call(&mut self, module: &Module, callee: &Expr) -> Option<String> {
        let name = callee_name(callee)?;
        if !matches!(name.as_str(), "len" | "rawSlice" | "rawStr" | "slice") {
            return None;
        }
        if self.ffi_functions.contains_key(&name)
            || self.functions.keys().any(|(_, simple)| *simple == name)
        {
            return None;
        }
        // A callable field or func-typed global shadows the built-in name too:
        // `b.len()` calls the field, not the length intrinsic.
        if self.init_callable_value(module, callee).is_some() {
            return None;
        }
        Some(name)
    }

    /// Writes every function body, user functions first in source order, then
    /// one body per generic instantiation.
    pub(super) fn emit_bodies(&mut self) {
        let mut tasks: Vec<(FuncInfo<'a>, Option<(BTreeMap<String, Ty>, String)>)> = Vec::new();
        for module in self.loaded.modules.values() {
            for item in &module.items {
                if let Item::Func(decl) = item {
                    if !decl.type_params.is_empty() {
                        continue;
                    }
                    tasks.push((
                        FuncInfo {
                            module: module.name.clone(),
                            decl,
                        },
                        None,
                    ));
                }
            }
        }
        for instance in self.ownership.monomorphization.instances() {
            let Some(info) = self.first_function_by_name(&instance.function) else {
                continue;
            };
            let subst = self.instance_subst(info.decl, &instance.args);
            let typekey = self.instance_typekey(&instance.args);
            tasks.push((
                FuncInfo {
                    module: info.module.clone(),
                    decl: info.decl,
                },
                Some((subst, typekey)),
            ));
        }
        for (info, instance) in tasks {
            self.emit_function(&info, instance);
        }
        // A lifted lambda function is not a declared item, so it is emitted
        // from the ownership stage's function list directly: once when shared,
        // once per enclosing instance when the body mentions a parameter.
        for emission in self.lifted_emissions() {
            self.emit_lifted_function(
                emission.function,
                emission
                    .key
                    .as_deref()
                    .map(|key| (&emission.subst, key)),
            );
        }
    }

    /// Writes the `int main(void)` stub that runs inits then the user `main`.
    pub(super) fn emit_entry_stub(&mut self) {
        let inits: Vec<String> = self
            .init_order()
            .into_iter()
            .map(|module| format!("    cl_init_{}();", mangle_module(&module)))
            .collect();
        let main = self
            .resolve_function("main")
            .map(|info| self.function_symbol(&info.module, "main"));
        // A module-private function is emitted `static`, so a helper no caller
        // reaches would fail `-Wunused-function`. Referencing each private
        // function once from the entry stub keeps the definition used without
        // a compiler extension. The reference is an address read inside a
        // `sizeof`, which is portable: both GCC and vbcc accept it, whereas a
        // bare `(void)fn;` cast is flagged by vbcc as a no-effect statement.
        let references = self.private_function_references();
        self.push("int main(void)");
        self.push("{");
        for line in inits {
            self.push(&line);
        }
        for symbol in references {
            self.push(&format!("    if (sizeof(&{symbol})) {{ }}"));
        }
        match main {
            Some(symbol) => self.push(&format!("    return {symbol}();")),
            None => self.push("    return 0;"),
        }
        self.push("}");
        self.push("");
    }

    /// The C symbols of every module-private function and generic instance.
    ///
    /// A generic template is not defined on its own, so only its concrete
    /// instances are referenced. An exported function already has external
    /// linkage and never triggers the unused-function warning.
    fn private_function_references(&self) -> Vec<String> {
        let mut symbols: Vec<String> = Vec::new();
        for info in self.functions.values() {
            if info.decl.exported || !info.decl.type_params.is_empty() {
                continue;
            }
            symbols.push(self.function_symbol(&info.module, &info.decl.name));
        }
        for instance in self.ownership.monomorphization.instances() {
            symbols.push(format!(
                "cl_{}__{}",
                instance.function,
                self.instance_typekey(&instance.args)
            ));
        }
        for emission in self.lifted_emissions() {
            symbols.push(emission.symbol());
        }
        symbols
    }

    /// The lifted lambda emissions this program needs, in deterministic order.
    ///
    /// A lifted function in a non-generic declaration, or one whose types never
    /// mention an enclosing parameter, keeps the single shared emission. A
    /// lifted function that mentions a parameter is emitted once per recorded
    /// enclosing specialization; with no recorded instance it is not emitted at
    /// all, because its body cannot be spelled with an unresolved parameter.
    /// The list is keyed by (site, instance): a loop's speculative pass lowers
    /// its body twice, so the ownership stage can hold two records for one
    /// site, and a second emission would redefine the environment struct, the
    /// closure typedef, and the lifted function.
    fn lifted_emissions(&self) -> Vec<LiftedEmission<'a>> {
        let mut emissions = Vec::new();
        let mut seen: BTreeSet<(usize, Option<String>)> = BTreeSet::new();
        for function in &self.ownership.functions {
            let Some(site) = function.lambda_site else {
                continue;
            };
            let capturing = self
                .sema
                .closures
                .get(&site)
                .is_some_and(|closure| !closure.captures.is_empty());
            let enclosing = function
                .enclosing
                .as_deref()
                .and_then(|name| self.first_function_by_name(name))
                .map(|info| info.decl);
            let specialized = enclosing.is_some_and(|decl| {
                crate::own::lifted_needs_specialization(function, decl, &self.sema.closures)
            });
            if !specialized {
                if !seen.insert((site, None)) {
                    continue;
                }
                emissions.push(LiftedEmission {
                    function,
                    site,
                    capturing,
                    subst: BTreeMap::new(),
                    key: None,
                });
                continue;
            }
            let Some(decl) = enclosing else {
                continue;
            };
            for specialization in self.ownership.monomorphization.lifted_specializations() {
                if specialization.site != site {
                    continue;
                }
                let key = self.instance_typekey(&specialization.args);
                if !seen.insert((site, Some(key.clone()))) {
                    continue;
                }
                emissions.push(LiftedEmission {
                    function,
                    site,
                    capturing,
                    subst: self.instance_subst(decl, &specialization.args),
                    key: Some(key),
                });
            }
        }
        emissions
    }

}

/// One lifted lambda emission: the lifted function, its symbol, and, for a
/// specialization, the enclosing instance's substitution and key.
struct LiftedEmission<'a> {
    /// The lifted function to write.
    function: &'a crate::ir::Function,
    /// The lambda site the lifted function came from.
    site: usize,
    /// True when the lambda captures at least one binding.
    capturing: bool,
    /// The enclosing instance's substitution, empty for a shared emission.
    subst: BTreeMap<String, Ty>,
    /// The enclosing instance key, absent for a shared emission.
    key: Option<String>,
}

impl LiftedEmission<'_> {
    /// The C symbol of the lifted function.
    fn symbol(&self) -> String {
        match &self.key {
            Some(key) => format!("cl_lambda_{}__{key}", self.site),
            None => format!("cl_lambda_{}", self.site),
        }
    }
}

/// One struct definition to emit: a non-generic struct or a generic instance.
struct StructUnit {
    /// The C symbol of the struct.
    symbol: String,
    /// The fields in declaration order, with any instance substitution applied.
    fields: Vec<(String, Ty, bool)>,
    /// Symbols of the structs this definition embeds by value, which must be
    /// defined first.
    deps: Vec<String>,
}

/// Adds the symbol of every aggregate type `ty` embeds by value.
///
/// A pointer, handle, function pointer, or slice does not embed its referent,
/// so only a struct and a fixed array contribute.
fn collect_value_deps(ty: &Ty, deps: &mut Vec<String>, table: &TypeTable) {
    match &ty.kind {
        TyKind::Struct(..) => deps.push(c_type(ty, table)),
        TyKind::Array { element, extents } if extents.iter().all(Option::is_some) => {
            collect_value_deps(element, deps, table);
        }
        _ => {}
    }
}

/// Orders struct definitions so a by-value dependency precedes its user.
///
/// A back edge is a cycle the checker rejects, so it cannot occur in a valid
/// program; if one is present the remaining units keep their input order rather
/// than looping.
fn order_struct_units(units: Vec<StructUnit>) -> Vec<StructUnit> {
    let known: BTreeSet<String> = units.iter().map(|unit| unit.symbol.clone()).collect();
    let mut remaining: Vec<Option<StructUnit>> = units.into_iter().map(Some).collect();
    let mut defined: BTreeSet<String> = BTreeSet::new();
    let mut ordered: Vec<StructUnit> = Vec::new();
    loop {
        let mut progressed = false;
        for slot in remaining.iter_mut() {
            let ready = slot.as_ref().is_some_and(|unit| {
                unit.deps.iter().all(|dep| {
                    dep == &unit.symbol || !known.contains(dep) || defined.contains(dep)
                })
            });
            if ready {
                if let Some(unit) = slot.take() {
                    defined.insert(unit.symbol.clone());
                    ordered.push(unit);
                    progressed = true;
                }
            }
        }
        if !progressed {
            break;
        }
    }
    ordered.extend(remaining.into_iter().flatten());
    ordered
}

/// The simple name a callee expression carries, for a program-level call.
fn callee_name(callee: &Expr) -> Option<String> {
    match &callee.kind {
        ExprKind::Name(name) => Some(name.clone()),
        ExprKind::Qualified(name) => name.segments.last().cloned(),
        ExprKind::Member { name, .. } => Some(name.clone()),
        _ => None,
    }
}

/// True when a module array value has an element-wise fill form.
///
/// A literal, a place (or a member or index projection over one), and a
/// conditional whose arms are fillable all have an element-wise form. A call
/// result has one too, because the fill binds it into an init local first:
/// a fixed-array-returning call writes through a hidden destination, and a
/// call that a projection reads through is bound for the same reason, so no
/// element store re-evaluates it.
fn fills_element_wise(value: &Value) -> bool {
    match &value.kind {
        ValueKind::ArrayLit { .. } | ValueKind::Place(_) => true,
        ValueKind::Call { .. } | ValueKind::ClosureCall { .. } => true,
        ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => fills_element_wise(base),
        ValueKind::Conditional {
            then_value,
            else_value,
            ..
        } => fills_element_wise(then_value) && fills_element_wise(else_value),
        _ => false,
    }
}

/// One element value of a filled array.
///
/// A literal contributes its listed element, else its fill; an omitted
/// element keeps the zero state the global declaration starts from (`None`).
/// A conditional selects each element from its chosen arm, and any other form
/// is place-rooted, so the element reads the base at the constant index.
fn fill_element_value(value: &Value, index: usize, element_ty: &Ty) -> Option<Value> {
    match &value.kind {
        ValueKind::ArrayLit { elements, fill } => {
            elements.get(index).cloned().or_else(|| fill.as_deref().cloned())
        }
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => Some(Value::new(
            element_ty.clone(),
            ValueKind::Conditional {
                condition: condition.clone(),
                then_value: Box::new(fill_element_value(then_value, index, element_ty)?),
                else_value: Box::new(fill_element_value(else_value, index, element_ty)?),
            },
        )),
        _ => Some(Value::new(
            element_ty.clone(),
            ValueKind::Index {
                base: Box::new(value.clone()),
                index: Box::new(Value::new(Ty::usize_ty(), ValueKind::Int(index as i128))),
            },
        )),
    }
}

/// The local name a stored init read names, when the value is one.
///
/// The materializing walk spells the read of an init local it stored as a
/// bare place. Only the walk and the fill preparation create those locals — a
/// module initializer has no source locals — so a place over an `__cl_itmp`
/// name is exactly a value some step already stored, and a caller can reuse
/// the stored read instead of storing a second copy.
fn stored_init_read_name(value: &Value) -> Option<String> {
    let ValueKind::Place(place) = &value.kind else {
        return None;
    };
    if !place.projections.is_empty() {
        return None;
    }
    match &place.base {
        PlaceBase::Local(name) if name.starts_with("__cl_itmp") => Some(name.clone()),
        _ => None,
    }
}

/// True when an initializer operand can be rendered where it is used.
///
/// Mirrors the checker's purity predicate: a call, template, closure call, or
/// stored temporary runs effects or reads a name this phase cannot spell, so
/// the init lowering stores it into an init local first; every other form is
/// safe to render in place.
fn init_operand_is_pure(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Call { .. }
        | ValueKind::Intrinsic { .. }
        | ValueKind::Template(_)
        | ValueKind::ClosureCall { .. }
        | ValueKind::Materialize { .. } => false,
        ValueKind::Closure { captures, .. } => captures.iter().all(init_operand_is_pure),
        ValueKind::Unary { op, operand } => !op.is_update() && init_operand_is_pure(operand),
        ValueKind::Binary { left, right, .. } => {
            init_operand_is_pure(left) && init_operand_is_pure(right)
        }
        ValueKind::Cast { value, .. } => init_operand_is_pure(value),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            init_operand_is_pure(condition)
                && init_operand_is_pure(then_value)
                && init_operand_is_pure(else_value)
        }
        ValueKind::Member { base, .. } => init_operand_is_pure(base),
        ValueKind::Index { base, index } => {
            init_operand_is_pure(base) && init_operand_is_pure(index)
        }
        ValueKind::StructLit { fields, .. } => {
            fields.iter().all(|(_, field)| init_operand_is_pure(field))
        }
        ValueKind::ArrayLit { elements, fill } => {
            elements.iter().all(init_operand_is_pure)
                && fill.as_deref().map_or(true, init_operand_is_pure)
        }
        _ => true,
    }
}

//! Program-level emission: typedefs, aggregates, prototypes, globals,
//! module initializers, function bodies, and the entry stub.
//!
//! These phases walk the whole program rather than one function body, so they
//! are split from the per-function statement emitter. Each method is a method
//! on [`super::Emitter`] and reads the same collected tables.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{EnumDecl, Expr, ExprKind, IntrinsicArg, Item, StructDecl, TemplatePiece};
use crate::ir::{
    GlobalId, Place, PlaceBase, Projection, StructInstance, Value, ValueKind, ValuePiece,
};
use crate::load::Module;
use crate::profile::Profile;
use crate::types::{resolve_ty, Ty, TyKind};

use super::types::{c_type, collect_slice_types, declarator, mangle_module, scalar_of, type_key};
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
        self.emit_slice_typedefs();
        self.emit_handle_typedefs();
    }

    /// Writes one `ClSlice_<key>` typedef per slice element type used.
    ///
    /// The element pointer is mutable so a store through a writable view
    /// compiles. A read-only view is protected by the ownership checks, not by
    /// C `const`, because the same typedef is shared by both lend kinds.
    pub(super) fn emit_slice_typedefs(&mut self) {
        let keys = self.slice_element_types();
        if keys.is_empty() {
            return;
        }
        for (key, element) in &keys {
            let element_c = c_type(element);
            self.push(&format!(
                "typedef struct {{ {element_c} *ptr; usize len; }} ClSlice_{key};"
            ));
        }
        self.push("");
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
                            let ty = resolve_ty(&param.ty, &params, &self.sema.table);
                            collect_slice_types(&ty, &mut found);
                        }
                        let ty = resolve_ty(&decl.returns, &params, &self.sema.table);
                        collect_slice_types(&ty, &mut found);
                    }
                    Item::GlobalLet(decl) => {
                        let ty = resolve_ty(&decl.ty, &[], &self.sema.table);
                        collect_slice_types(&ty, &mut found);
                    }
                    Item::GlobalConst(decl) => {
                        let ty = resolve_ty(&decl.ty, &[], &self.sema.table);
                        collect_slice_types(&ty, &mut found);
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
                let ty = resolve_ty(&param.ty, &params, &self.sema.table);
                collect_slice_types(&super::substitute_in(&ty, &subst), &mut found);
            }
            let ret = resolve_ty(&info.decl.returns, &params, &self.sema.table);
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

    /// Writes the user struct and enum declarations.
    ///
    /// A non-generic struct is written once under its bare name. A generic
    /// struct has no single C layout, so its bare template is skipped and one
    /// concrete struct is written for each instantiation the program uses.
    pub(super) fn emit_aggregate_declarations(&mut self) {
        let structs: Vec<&'a StructDecl> = self.structs.values().copied().collect();
        for decl in structs {
            if decl.type_params.is_empty() {
                self.emit_struct(decl);
            } else {
                self.emit_struct_instances(decl);
            }
        }
        let enums: Vec<&'a EnumDecl> = self.enums.values().map(|info| info.decl).collect();
        for decl in enums {
            self.emit_enum(decl);
        }
    }

    /// Writes one C struct with its fields in declaration order.
    pub(super) fn emit_struct(&mut self, decl: &StructDecl) {
        self.push(&format!("struct Cl{name} {{", name = decl.name));
        let params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        for field in &decl.fields {
            let ty = resolve_ty(&field.ty, &params, &self.sema.table);
            let qualifier = if field.volatile { "volatile " } else { "" };
            self.push(&format!("    {qualifier}{};", declarator(&ty, &field.name)));
        }
        self.push("};");
        self.push(&format!("typedef struct Cl{name} Cl{name};", name = decl.name));
        self.push("");
    }

    /// Writes one C struct per recorded instantiation of a generic struct.
    ///
    /// The bare template is never emitted: its fields still name the type
    /// parameters, so it is not a complete C type. Each recorded instance
    /// replaces the declaration's parameters with its concrete arguments, which
    /// fixes the field types, and takes the instantiated symbol
    /// `Cl<Name>__<typekey>`. Instances are recorded with a nested argument's
    /// inner instantiation first, so a field type is always declared before the
    /// struct that uses it.
    fn emit_struct_instances(&mut self, decl: &StructDecl) {
        let instances: Vec<StructInstance> = self
            .ownership
            .monomorphization
            .struct_instances()
            .iter()
            .filter(|instance| instance.name == decl.name)
            .cloned()
            .collect();
        let params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        for instance in instances {
            let symbol = format!(
                "Cl{}",
                type_key(&Ty::struct_with(decl.name.clone(), instance.args.clone()))
            );
            let subst: BTreeMap<String, Ty> = decl
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .zip(instance.args.iter().cloned())
                .collect();
            self.push(&format!("struct {symbol} {{"));
            for field in &decl.fields {
                let ty = resolve_ty(&field.ty, &params, &self.sema.table);
                let ty = super::substitute_in(&ty, &subst);
                let qualifier = if field.volatile { "volatile " } else { "" };
                self.push(&format!("    {qualifier}{};", declarator(&ty, &field.name)));
            }
            self.push("};");
            self.push(&format!("typedef struct {symbol} {symbol};"));
            self.push("");
        }
    }

    /// Writes the enum constants as macros over the backing integer type.
    pub(super) fn emit_enum(&mut self, decl: &EnumDecl) {
        let backing = resolve_ty(&decl.backing, &[], &self.sema.table);
        self.push(&format!("typedef {} Cl{};", c_type(&backing), decl.name));
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
                self.signature_with_symbol(info.decl, &subst, &symbol)
            ));
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
    pub(super) fn emit_globals(&mut self) {
        if self.globals.is_empty() {
            return;
        }
        let globals: Vec<(String, String, Ty)> = self
            .globals
            .values()
            .map(|global| (global.name.clone(), global.module.clone(), global.ty.clone()))
            .collect();
        for (name, module, ty) in globals {
            let symbol = self.global_symbol(&module, &name);
            let empty = self.empty_state(&ty);
            self.push(&format!(
                "static {} = {};",
                declarator(&ty, &symbol),
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
            for (name, value) in self.global_init_order(&module_name) {
                self.emit_global_init(&module_name, &name, value);
            }
            self.push("}");
            self.push("");
        }
    }

    /// Writes one global's initializer assignment.
    pub(super) fn emit_global_init(&mut self, module_name: &str, name: &str, value: &'a Expr) {
        self.emit_line_directive(value.span);
        let target = self.global_symbol(module_name, name);
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
        let rendered = self.value(&lowered);
        // A module initializer runs at startup, so it carries the same
        // arithmetic guards a function statement would: a division by zero,
        // an out-of-range index, or a bad float conversion must fault with its
        // diagnostic instead of trapping on undefined behavior.
        self.emit_guards(&lowered);
        self.push(&format!("    {target} = {rendered};"));
    }

    /// Lowers a module-level initializer expression to a typed IR value.
    ///
    /// Module initialization is a program-level phase, so it reuses the IR
    /// value renderer instead of the per-function body emitter. Only the forms
    /// an initializer can spell are built here; a global name, whether bare,
    /// field-projected, or written as a full module path, becomes a global
    /// place so the renderer spells its mangled symbol, and an enum member
    /// keeps its dotted path.
    fn lower_global(&self, module: &Module, expr: &Expr) -> Value {
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
            } => ValueKind::Call {
                callee: callee_name(callee).unwrap_or_default(),
                type_args: type_args.iter().map(|ty| self.resolve_written(ty)).collect(),
                args: args
                    .iter()
                    .map(|arg| self.lower_global(module, &arg.value))
                    .collect(),
                receiver: None,
            },
            _ => ValueKind::Aggregate,
        };
        Value { ty, kind }
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
        symbols
    }

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

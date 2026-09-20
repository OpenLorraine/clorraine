//! Program-level emission: typedefs, aggregates, prototypes, globals,
//! module initializers, function bodies, and the entry stub.
//!
//! These phases walk the whole program rather than one function body, so they
//! are split from the per-function statement emitter. Each method is a method
//! on [`super::Emitter`] and reads the same collected tables.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{EnumDecl, Expr, ExprKind, IntrinsicArg, Item, StructDecl, TemplatePiece};
use crate::ir::{Place, PlaceBase, Projection, Value, ValueKind, ValuePiece};
use crate::load::Module;
use crate::types::{resolve_ty, Ty};

use super::types::{c_type, collect_slice_types, declarator, empty_state, mangle_module};
use super::{Emitter, FuncInfo};

impl<'a> Emitter<'a> {
    // -- top-level sections --------------------------------------------------

    /// Writes the comment header. No timestamp or host path appears.
    pub(super) fn emit_header(&mut self) {
        self.push("/* clorraine generated C - do not edit */");
        self.push("");
    }

    /// Writes the fixed-width typedefs and the built-in view structs.
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
        self.emit_slice_typedefs();
        self.emit_handle_typedefs();
    }

    /// Writes one `ClSlice_<key>` typedef per slice element type used.
    pub(super) fn emit_slice_typedefs(&mut self) {
        let keys = self.slice_element_types();
        if keys.is_empty() {
            return;
        }
        for (key, element) in &keys {
            let element_c = c_type(element);
            self.push(&format!(
                "typedef struct {{ const {element_c} *ptr; usize len; }} ClSlice_{key};"
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
    pub(super) fn emit_aggregate_declarations(&mut self) {
        let structs: Vec<&'a StructDecl> = self.structs.values().copied().collect();
        for decl in structs {
            self.emit_struct(decl);
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

    /// Writes a prototype for every emitted function.
    pub(super) fn emit_prototypes(&mut self) {
        let mut lines: Vec<String> = Vec::new();
        for info in self.functions.values() {
            if !info.decl.type_params.is_empty() {
                continue;
            }
            lines.push(format!("{};", self.function_signature(info.decl)));
        }
        for instance in self.ownership.monomorphization.instances() {
            let Some(info) = self.functions.get(&instance.function) else {
                continue;
            };
            let subst = self.instance_subst(info.decl, &instance.args);
            let symbol = format!("cl_{}__{}", instance.function, instance.args.join("_"));
            lines.push(format!(
                "{};",
                self.signature_with_symbol(info.decl, &subst, &symbol)
            ));
        }
        if lines.is_empty() {
            return;
        }
        for line in lines {
            self.push(&line);
        }
        self.push("");
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
            self.push(&format!(
                "static {} = {};",
                declarator(&ty, &self.global_symbol(&module, &name)),
                empty_state(&ty)
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
            let inits: Vec<(String, &'a Expr)> = match self.loaded.module(&module_name) {
                Some(module) => module
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        Item::GlobalLet(decl) => Some((decl.name.clone(), &decl.value)),
                        Item::GlobalConst(decl) => Some((decl.name.clone(), &decl.value)),
                        _ => None,
                    })
                    .collect(),
                None => Vec::new(),
            };
            for (name, value) in inits {
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
        let mut lowered = self.lower_global(value);
        // A global is a file-scope object, so the place keeps its declared type.
        if lowered.ty.is_unknown() {
            if let Some(ty) = self.expr_type(value) {
                lowered.ty = ty;
            }
        }
        let rendered = self.value(&lowered);
        self.push(&format!("    {target} = {rendered};"));
    }

    /// Lowers a module-level initializer expression to a typed IR value.
    ///
    /// Module initialization is a program-level phase, so it reuses the IR
    /// value renderer instead of the per-function body emitter. Only the forms
    /// an initializer can spell are built here; a global name becomes a global
    /// place and an enum member keeps its dotted path so the renderer can spell
    /// it.
    fn lower_global(&self, expr: &Expr) -> Value {
        let ty = self.expr_type(expr).unwrap_or_else(Ty::unknown);
        let kind = match &expr.kind {
            ExprKind::Int(value) => ValueKind::Int(*value as i128),
            ExprKind::Float(value) => ValueKind::Float(*value),
            ExprKind::Char(byte) => ValueKind::Char(*byte),
            ExprKind::Bool(value) => ValueKind::Bool(*value),
            ExprKind::Null => ValueKind::Null,
            ExprKind::Str(bytes) => ValueKind::Str(String::from_utf8_lossy(bytes).into_owned()),
            ExprKind::Name(name) => {
                if self.globals.contains_key(name) {
                    ValueKind::Place(Place::global(name).with_type(ty.clone()))
                } else {
                    ValueKind::Path(vec![name.clone()])
                }
            }
            ExprKind::Qualified(name) => match name.segments.split_first() {
                Some((first, rest)) if self.globals.contains_key(first) => {
                    let projections = rest
                        .iter()
                        .map(|segment| Projection::Field(segment.clone()))
                        .collect();
                    ValueKind::Place(Place {
                        base: PlaceBase::Global(first.clone()),
                        projections,
                        ty: ty.clone(),
                    })
                }
                _ => ValueKind::Path(name.segments.clone()),
            },
            ExprKind::Member { base, name } => ValueKind::Member {
                base: Box::new(self.lower_global(base)),
                name: name.clone(),
            },
            ExprKind::Index { base, index } => ValueKind::Index {
                base: Box::new(self.lower_global(base)),
                index: Box::new(self.lower_global(index)),
            },
            ExprKind::Unary { op, operand } => ValueKind::Unary {
                op: *op,
                operand: Box::new(self.lower_global(operand)),
            },
            ExprKind::Binary { op, left, right } => ValueKind::Binary {
                op: *op,
                left: Box::new(self.lower_global(left)),
                right: Box::new(self.lower_global(right)),
            },
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => ValueKind::Cast {
                to: ty.display(),
                value: Box::new(self.lower_global(value)),
            },
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => ValueKind::Conditional {
                condition: Box::new(self.lower_global(condition)),
                then_value: Box::new(self.lower_global(then_value)),
                else_value: Box::new(self.lower_global(else_value)),
            },
            ExprKind::StructLiteral { name, fields, .. } => ValueKind::StructLit {
                name: name.segments.last().cloned().unwrap_or_default(),
                fields: fields
                    .iter()
                    .map(|field| (field.name.clone(), self.lower_global(&field.value)))
                    .collect(),
            },
            ExprKind::ArrayLiteral { elements, fill } => ValueKind::ArrayLit {
                elements: elements.iter().map(|element| self.lower_global(element)).collect(),
                fill: fill.as_ref().map(|fill| Box::new(self.lower_global(fill))),
            },
            ExprKind::Unsafe(inner) => return self.lower_global(inner),
            ExprKind::IntrinsicCall { name, args } => {
                let mut values = Vec::new();
                let mut type_arg = None;
                for arg in args {
                    match arg {
                        IntrinsicArg::Expr(inner) => values.push(self.lower_global(inner)),
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
                            ValuePiece::Placeholder(self.lower_global(inner))
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
                args: args.iter().map(|arg| self.lower_global(&arg.value)).collect(),
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
            let Some(info) = self.functions.get(&instance.function) else {
                continue;
            };
            let subst = self.instance_subst(info.decl, &instance.args);
            let typekey = instance.args.join("_");
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
            .functions
            .get("main")
            .map(|info| self.function_symbol(&info.module, "main"));
        self.push("int main(void)");
        self.push("{");
        for line in inits {
            self.push(&line);
        }
        match main {
            Some(symbol) => self.push(&format!("    return {symbol}();")),
            None => self.push("    return 0;"),
        }
        self.push("}");
        self.push("");
    }

    // -- module initialization order ----------------------------------------

    /// The user modules in initialization order.
    ///
    /// A module is initialized after every module it imports or names in a
    /// global initializer, transitively; independent modules break ties on the
    /// dotted name compared byte-wise, so the order is deterministic.
    pub(super) fn init_order(&self) -> Vec<String> {
        let modules: Vec<String> = self.loaded.modules.keys().cloned().collect();
        let known: BTreeSet<String> = modules.iter().cloned().collect();
        let mut prereqs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for name in &modules {
            let mut deps: BTreeSet<String> = BTreeSet::new();
            if let Some(module) = self.loaded.module(name) {
                for import in &module.imports {
                    if known.contains(&import.module) && &import.module != name {
                        deps.insert(import.module.clone());
                    }
                }
                for item in &module.items {
                    let init = match item {
                        Item::GlobalLet(decl) => Some(&decl.value),
                        Item::GlobalConst(decl) => Some(&decl.value),
                        _ => None,
                    };
                    if let Some(init) = init {
                        self.collect_name_modules(module, init, &known, &mut deps);
                    }
                }
            }
            prereqs.insert(name.clone(), deps);
        }

        let mut order: Vec<String> = Vec::new();
        let mut done: BTreeSet<String> = BTreeSet::new();
        while order.len() < modules.len() {
            let ready = modules
                .iter()
                .filter(|name| !done.contains(*name))
                .filter(|name| {
                    prereqs
                        .get(*name)
                        .is_none_or(|deps| deps.iter().all(|dep| done.contains(dep)))
                })
                .min()
                .cloned();
            match ready {
                Some(name) => {
                    done.insert(name.clone());
                    order.push(name);
                }
                None => {
                    // A dependency cycle is E11100 before codegen; keep the
                    // emitter total by appending the rest in name order.
                    for name in &modules {
                        if !done.contains(name) {
                            done.insert(name.clone());
                            order.push(name.clone());
                        }
                    }
                }
            }
        }
        order
    }

    /// Adds the user modules an initializer expression names through imports.
    pub(super) fn collect_name_modules(
        &self,
        module: &Module,
        expr: &Expr,
        known: &BTreeSet<String>,
        deps: &mut BTreeSet<String>,
    ) {
        use crate::ast::ExprKind;
        match &expr.kind {
            ExprKind::Name(name) => {
                if let Some(binding) = module.import(name) {
                    if known.contains(&binding.module) && binding.module != module.name {
                        deps.insert(binding.module.clone());
                    }
                }
            }
            ExprKind::Qualified(name) => {
                if let Some(first) = name.segments.first() {
                    if let Some(binding) = module.import(first) {
                        if known.contains(&binding.module) && binding.module != module.name {
                            deps.insert(binding.module.clone());
                        }
                    }
                }
            }
            ExprKind::Call { callee, args, .. } => {
                self.collect_name_modules(module, callee, known, deps);
                for arg in args {
                    self.collect_name_modules(module, &arg.value, known, deps);
                }
            }
            ExprKind::Index { base, index } => {
                self.collect_name_modules(module, base, known, deps);
                self.collect_name_modules(module, index, known, deps);
            }
            ExprKind::Member { base, .. } => self.collect_name_modules(module, base, known, deps),
            ExprKind::Unary { operand, .. } => {
                self.collect_name_modules(module, operand, known, deps)
            }
            ExprKind::Binary { left, right, .. } => {
                self.collect_name_modules(module, left, known, deps);
                self.collect_name_modules(module, right, known, deps);
            }
            ExprKind::Assign { target, value, .. } => {
                self.collect_name_modules(module, target, known, deps);
                self.collect_name_modules(module, value, known, deps);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.collect_name_modules(module, condition, known, deps);
                self.collect_name_modules(module, then_value, known, deps);
                self.collect_name_modules(module, else_value, known, deps);
            }
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
                self.collect_name_modules(module, value, known, deps)
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.collect_name_modules(module, &field.value, known, deps);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.collect_name_modules(module, element, known, deps);
                }
                if let Some(fill) = fill {
                    self.collect_name_modules(module, fill, known, deps);
                }
            }
            ExprKind::Unsafe(inner) => self.collect_name_modules(module, inner, known, deps),
            ExprKind::IntrinsicCall { args, .. } => {
                for arg in args {
                    if let crate::ast::IntrinsicArg::Expr(inner) = arg {
                        self.collect_name_modules(module, inner, known, deps);
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let crate::ast::TemplatePiece::Placeholder(inner) = piece {
                        self.collect_name_modules(module, inner, known, deps);
                    }
                }
            }
            _ => {}
        }
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

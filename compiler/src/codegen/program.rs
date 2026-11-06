//! Program-level emission: typedefs, aggregates, prototypes, globals,
//! module initializers, function bodies, and the entry stub.
//!
//! These phases walk the whole program rather than one function body, so they
//! are split from the per-function statement emitter. Each method is a method
//! on [`super::Emitter`] and reads the same collected tables.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Arg, BinaryOp, EnumDecl, Expr, ExprKind, IntrinsicArg, Item, StructDecl, TemplatePiece, Type,
};
use crate::ir::{GlobalId, Place, PlaceBase, Projection, Statement, Terminator, Value, ValueKind, ValuePiece};
use crate::load::Module;
use crate::profile::Profile;
use crate::types::{resolve_ty, resolve_ty_folded, substitute_instance, view_type, CaptureMode, Ty, TyKind, TypeTable};

use super::function::ArrayTarget;
use super::types::{
    c_type, collect_slice_types, declarator, function_declarator, innermost_is_struct,
    is_struct_view, mangle_module, pointer_to_array_declarator, scalar_of, type_key,
    zero_view_helper,
};
use super::{Emitter, FuncInfo};

/// One constant step from a global initializer's literal root to the stored
/// view. A run-time index or a non-literal root produces no path at all.
#[derive(Clone, Debug)]
pub(super) enum InitStep {
    Field(String),
    Index(usize),
}

/// How much of a literal's view backing the current initializer stores.
///
/// `Whole` preserves the previous retention behavior: every backing stays
/// inline and is reachable through the destination. `Path` names the one
/// constant position the destination stores, so sibling positions can release
/// their backing. `Unselected` means nothing in the subtree is stored, so every
/// fresh fill root may be registered for release.
#[derive(Clone, Debug)]
pub(super) enum ViewSelect {
    Whole,
    Path(Vec<InitStep>),
    Unselected,
}

impl ViewSelect {
    /// The selection for a named field below the current position.
    fn field(&self, name: &str) -> Self {
        match self {
            Self::Whole => Self::Whole,
            Self::Unselected => Self::Unselected,
            Self::Path(steps) => match steps.split_first() {
                Some((InitStep::Field(field), rest)) if field.as_str() == name => {
                    Self::from_rest(rest)
                }
                _ => Self::Unselected,
            },
        }
    }

    /// The selection for one constant index below the current position.
    fn index(&self, index: usize) -> Self {
        match self {
            Self::Whole => Self::Whole,
            Self::Unselected => Self::Unselected,
            Self::Path(steps) => match steps.split_first() {
                Some((InitStep::Index(position), rest)) if *position == index => {
                    Self::from_rest(rest)
                }
                _ => Self::Unselected,
            },
        }
    }

    /// The selection for a fill value that covers every index at or above
    /// `prefix`.
    ///
    /// A fill repeats one value across the omitted tail, so a path that points
    /// into that tail stores the fill's backing; any other path does not.
    fn fill(&self, prefix: usize) -> Self {
        match self {
            Self::Whole => Self::Whole,
            Self::Unselected => Self::Unselected,
            Self::Path(steps) => match steps.first() {
                None => Self::Whole,
                Some(InitStep::Index(position)) if *position >= prefix => Self::Whole,
                _ => Self::Unselected,
            },
        }
    }

    /// The selection represented by the remaining path.
    fn from_rest(steps: &[InitStep]) -> Self {
        if steps.is_empty() {
            Self::Whole
        } else {
            Self::Path(steps.to_vec())
        }
    }
}

/// One step in a fresh projection chain.
///
/// A member read keeps its field name. An index keeps the exact IR value the
/// source evaluated, so a run-time index is shared by the read and the
/// post-store move-out instead of being evaluated twice.
#[derive(Clone, Debug)]
enum ProjectionAccessor {
    Member(String),
    Index(Value),
}

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
            let pointer = declarator(&Ty::raw(element.clone()), "ptr", &self.sema.table);
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
            // A written type argument such as `sizeof(i32[])` appears in no
            // local or signature, so the body's own values are the only
            // evidence of its element type. A generic body's argument is still
            // a parameter here and is skipped; the instance walk below
            // substitutes it.
            self.collect_body_slice_types(function, &BTreeMap::new(), &mut found);
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
            self.collect_body_slice_types(body, &subst, &mut found);
        }
        // A lifted lambda whose types mention an enclosing parameter is
        // emitted once per recorded specialization, so the specialization's
        // substitution decides the slice types its body spells.
        for function in &self.ownership.functions {
            let Some(site) = function.lambda_site else {
                continue;
            };
            let Some(info) = function
                .enclosing
                .as_deref()
                .and_then(|name| self.first_function_by_name(name))
            else {
                continue;
            };
            if !crate::own::lifted_needs_specialization(function, info.decl, &self.sema.closures) {
                continue;
            }
            for specialization in self.ownership.monomorphization.lifted_specializations() {
                if specialization.site != site {
                    continue;
                }
                let subst = self.instance_subst(info.decl, &specialization.args);
                for local in &function.locals {
                    collect_slice_types(&super::substitute_in(&local.ty, &subst), &mut found);
                }
                for temp in &function.temps {
                    collect_slice_types(&super::substitute_in(temp, &subst), &mut found);
                }
                self.collect_body_slice_types(function, &subst, &mut found);
            }
        }
        found.into_iter().collect()
    }

    /// Records the slice element types a body's written type arguments need.
    ///
    /// The walk visits every value position a block can hold — statement
    /// operands, guard checks, call arguments and receivers, and the values a
    /// terminator reads — so an argument's typedef exists wherever the value
    /// lands. `subst` carries an instance's substitutions for a generic body
    /// and is empty for a plain one.
    fn collect_body_slice_types(
        &self,
        function: &crate::ir::Function,
        subst: &BTreeMap<String, Ty>,
        found: &mut BTreeMap<String, Ty>,
    ) {
        for block in &function.blocks {
            for instr in &block.statements {
                match &instr.statement {
                    Statement::Assign { value, .. } | Statement::Discard { value } => {
                        self.collect_value_slice_types(value, subst, found);
                    }
                    // A run-time store index (and its guards) lives in the
                    // pointer operand, so a written type argument there needs
                    // its typedef exactly like the stored value's would.
                    Statement::StorePointer { pointer, value } => {
                        self.collect_value_slice_types(pointer, subst, found);
                        self.collect_value_slice_types(value, subst, found);
                    }
                    Statement::Guards { checks } => {
                        for check in checks {
                            self.collect_value_slice_types(check, subst, found);
                        }
                    }
                    Statement::Call { args, receiver, .. } => {
                        for arg in args {
                            self.collect_value_slice_types(arg, subst, found);
                        }
                        if let Some(receiver) = receiver {
                            self.collect_value_slice_types(receiver, subst, found);
                        }
                    }
                    // The remaining statements hold places, scopes, or
                    // registrations and read no value; a value move-out reads
                    // the source's type, which contributes its own typedef.
                    Statement::MoveOutValue { value } => {
                        self.collect_value_slice_types(value, subst, found);
                    }
                    Statement::Move { .. }
                    | Statement::MoveOut { .. }
                    | Statement::Borrow { .. }
                    | Statement::Drop { .. }
                    | Statement::ReleaseTemp(_)
                    | Statement::Defer { .. }
                    | Statement::ScopeEnd { .. }
                    | Statement::ScopeExit { .. }
                    | Statement::Nop => {}
                }
            }
            match &block.terminator {
                Terminator::Branch { condition, .. } => {
                    self.collect_value_slice_types(condition, subst, found);
                }
                Terminator::Switch { value, arms, .. } => {
                    self.collect_value_slice_types(value, subst, found);
                    for (label, _) in arms {
                        self.collect_value_slice_types(label, subst, found);
                    }
                }
                Terminator::Return { value, .. } => {
                    if let Some(value) = value {
                        self.collect_value_slice_types(value, subst, found);
                    }
                }
                // No other terminator reads a value.
                Terminator::Goto(_) | Terminator::Unreachable => {}
            }
        }
    }

    /// Walks one IR value for written slice type arguments.
    ///
    /// A `sizeof`'s `type_arg` is the one position a written type reaches the
    /// body IR, so the walk descends every sub-value to find it; the
    /// substitution applies exactly as it does to locals and temporaries.
    fn collect_value_slice_types(
        &self,
        value: &Value,
        subst: &BTreeMap<String, Ty>,
        found: &mut BTreeMap<String, Ty>,
    ) {
        // A value's own type is evidence too: an inline `slice(...)` result is
        // built as a compound literal of its view type, so the typedef must
        // exist even though no written type argument names it.
        collect_slice_types(&super::substitute_in(&value.ty, subst), found);
        match &value.kind {
            ValueKind::Intrinsic { args, type_arg, .. } => {
                if let Some(ty) = type_arg {
                    collect_slice_types(&super::substitute_in(ty, subst), found);
                }
                for arg in args {
                    self.collect_value_slice_types(arg, subst, found);
                }
            }
            ValueKind::Call { args, receiver, .. } => {
                for arg in args {
                    self.collect_value_slice_types(arg, subst, found);
                }
                if let Some(receiver) = receiver {
                    self.collect_value_slice_types(receiver, subst, found);
                }
            }
            ValueKind::ClosureCall { callable, args } => {
                self.collect_value_slice_types(callable, subst, found);
                for arg in args {
                    self.collect_value_slice_types(arg, subst, found);
                }
            }
            ValueKind::Member { base, .. } => self.collect_value_slice_types(base, subst, found),
            ValueKind::Index { base, index } => {
                self.collect_value_slice_types(base, subst, found);
                self.collect_value_slice_types(index, subst, found);
            }
            ValueKind::Unary { operand, .. } => {
                self.collect_value_slice_types(operand, subst, found);
            }
            ValueKind::Binary { left, right, .. } => {
                self.collect_value_slice_types(left, subst, found);
                self.collect_value_slice_types(right, subst, found);
            }
            ValueKind::Cast { value, .. } => self.collect_value_slice_types(value, subst, found),
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.collect_value_slice_types(condition, subst, found);
                self.collect_value_slice_types(then_value, subst, found);
                self.collect_value_slice_types(else_value, subst, found);
            }
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.collect_value_slice_types(field, subst, found);
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.collect_value_slice_types(element, subst, found);
                }
                if let Some(fill) = fill {
                    self.collect_value_slice_types(fill, subst, found);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.collect_value_slice_types(inner, subst, found);
                    }
                }
            }
            ValueKind::Closure { captures, .. } => {
                for capture in captures {
                    self.collect_value_slice_types(capture, subst, found);
                }
            }
            ValueKind::Materialize { value, .. } => {
                self.collect_value_slice_types(value, subst, found);
            }
            _ => {}
        }
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
            self.push(&format!("typedef void *Cl{};", crate::types::type_component(&name)));
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
            format!("Cl{}", crate::types::type_component(&decl.name))
        } else {
            format!(
                "Cl{}",
                type_key(&Ty::struct_with(decl.name.clone(), args.to_vec()))
            )
        };
        let mut deps: Vec<String> = Vec::new();
        let declared_fields = self.sema.table.struct_fields(&decl.name);
        for (index, field) in decl.fields.iter().enumerate() {
            // The type table resolves imported type aliases in the declaring
            // module; the AST alone cannot recover the alias binding here.
            let ty = declared_fields
                .and_then(|fields| fields.get(index))
                .map(|(_, ty)| ty.clone())
                .unwrap_or_else(|| resolve_ty(&field.ty, &params, &self.sema.table));
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
        self.push(&format!(
            "typedef {} Cl{};",
            c_type(&backing, &self.sema.table),
            crate::types::type_component(&decl.name)
        ));
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
    pub(super) fn ffi_signature(&self, signature: &crate::ast::FuncSignature) -> String {
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
        // A module initializer has no IR function body, so no temporary type
        // resolution applies; the map must not leak from the last emitted body.
        self.resolved_temp_types.clear();
        for module_name in self.init_order() {
            self.push(&format!(
                "static void cl_init_{}(void)",
                mangle_module(&module_name)
            ));
            self.push("{");
            self.init_locals = 0;
            self.init_owners.clear();
            self.init_literal_locals.clear();
            // An initializer resolves its names in its own module: a
            // same-named function elsewhere must not supply the callee's
            // symbol, signature, or parameter modes.
            self.current_module = module_name.clone();
            for (name, value) in self.global_init_order(&module_name) {
                self.emit_global_init(&module_name, &name, value);
            }
            // Release every owning init temporary in reverse creation order.
            // A local an untaken conditional path left zero drops nothing.
            let owners = std::mem::take(&mut self.init_owners);
            for (name, ty) in owners.into_iter().rev() {
                self.emit_drop_expr(&name, &ty);
            }
            self.push("}");
            self.push("");
        }
    }

    /// The constant projection steps from a literal root to a stored value.
    ///
    /// A run-time index or a root that is already a call or a bound local has
    /// no constant path; the caller then keeps the previous whole-retention
    /// behavior. The steps are outermost-first, matching the source spelling.
    fn init_projection_steps(value: &Value) -> Option<Vec<InitStep>> {
        let mut steps = Vec::new();
        let mut cursor = value;
        loop {
            match &cursor.kind {
                ValueKind::Member { base, name } => {
                    steps.push(InitStep::Field(name.clone()));
                    cursor = base;
                }
                ValueKind::Index { base, index } => {
                    let ValueKind::Int(position) = index.kind else {
                        return None;
                    };
                    steps.push(InitStep::Index(usize::try_from(position).ok()?));
                    cursor = base;
                }
                _ => break,
            }
        }
        if !matches!(cursor.kind, ValueKind::StructLit { .. }) {
            return None;
        }
        steps.reverse();
        Some(steps)
    }

    /// The selection for a global view initializer.
    ///
    /// Only a literal root with at least one constant step can identify which
    /// backing is stored; anything else retains every backing.
    fn view_select(value: &Value) -> ViewSelect {
        match Self::init_projection_steps(value) {
            Some(steps) if !steps.is_empty() => ViewSelect::Path(steps),
            _ => ViewSelect::Whole,
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
        // A view destination keeps the storage it points into for the
        // program's lifetime, so this initializer's owned values are retained
        // rather than released; every other destination copies its bytes.
        self.init_retain_owners = declared_ty
            .as_ref()
            .is_some_and(|declared| self.stores_view_storage(declared));
        self.init_global_literals.clear();
        self.init_call_locals.clear();
        self.init_select = ViewSelect::Whole;
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
        self.init_select = Self::view_select(&lowered);
        let lowered = self.materialize_init_value(&lowered, None);
        self.init_select = ViewSelect::Whole;
        // A view destination retains the storage its view points into, but a
        // member projection of a fresh owned root reads only one position: the
        // root binds into an init local the release pass empties, so its other
        // fields free at the initializer's end, and the read member is emptied
        // right after the store — the view keeps the member's storage for the
        // program's lifetime while nothing frees it.
        let (lowered, member_move_out) = self.retain_view_member_root(&lowered, true, None);
        // A view destination takes an owned `String` value through the view
        // constructor, exactly as the function path's store does; without the
        // wrapper the handle cannot assign to the view.
        let declared_view = declared_ty
            .as_ref()
            .filter(|declared| matches!(declared.kind, TyKind::Str | TyKind::StaticStr));
        // A nullable owned source at a view destination needs the null guard:
        // the view constructor would dereference a null handle. The guard
        // reads its source twice, so a source whose read is not a place — a
        // call result or a conditional over one — binds into an init local
        // first and the guard reads the local; the view destination retains
        // the bound storage exactly as it retains one written directly.
        let lowered = match declared_view {
            Some(declared) => self.bind_guarded_view_source(&lowered, declared),
            None => lowered,
        };
        let rendered = match declared_view {
            // A conditional at a view destination keeps each arm's own
            // nullness: the owned arm widens to the view and the null arm
            // carries the destination's zero view. Wrapping the whole
            // conditional would construct a view over whichever arm ran —
            // including the null one, which dereferences the null handle.
            Some(declared) if matches!(lowered.kind, ValueKind::Conditional { .. }) => {
                self.conditional_arm(&lowered, declared)
            }
            Some(declared)
                if matches!(
                    &self.substitute(&lowered.ty).kind,
                    TyKind::Handle(handle) if handle == "String"
                ) =>
            {
                // A possibly-null handle keeps its nullness through the
                // widening: the guard selects the destination's zero view for
                // a null handle instead of dereferencing it.
                self.guarded_view_widening(&lowered, &self.resolved_value_ty(&lowered), declared)
                    .unwrap_or_else(|| format!("cl_string_view({})", self.value(&lowered)))
            }
            // A non-view destination still spells the member conversions a
            // function-path store does: a text view's pointer member at a
            // raw destination takes the sanctioned cast, and a byte pointer
            // at a `cstring` destination takes its own. Every other render
            // is unchanged.
            _ => match declared_ty.as_ref() {
                Some(declared)
                    if self.raw_text_view_pointer(&lowered, declared)
                        || (matches!(declared.kind, TyKind::CString)
                            && matches!(
                                self.resolved_value_ty(&lowered).kind,
                                TyKind::Raw(_)
                            )) =>
                {
                    self.store_text(&lowered, declared)
                }
                _ => self.value(&lowered),
            },
        };
        self.push(&format!("    {target} = {rendered};"));
        for move_out in &member_move_out {
            self.push(&format!("    {move_out};"));
        }
    }

    /// Binds the fresh owned roots a retained view projection reads.
    ///
    /// A view destination keeps the storage its view points into, so an
    /// initializer's owned results are otherwise left inline and retained
    /// whole. A member projection reads one position of a fresh root, though:
    /// binding the root registers it with the release pass, so its remaining
    /// fields free at the initializer's end, and the read member empties right
    /// after the store so the release leaves the view's storage alone. A
    /// conditional reads one arm, so each arm's root binds under its own
    /// branch guard. Returns the rebound value and the move-out statements to
    /// emit after the store.
    fn retain_view_member_root(
        &mut self,
        value: &Value,
        move_out: bool,
        guard: Option<&str>,
    ) -> (Value, Vec<String>) {
        if !self.init_retain_owners || self.init_skip_owner_binding {
            return (value.clone(), Vec::new());
        }
        if matches!(value.kind, ValueKind::Conditional { .. }) {
            return self.retain_conditional_projection_roots(value, guard, move_out);
        }
        // A containing literal whose field is a store-needing inner literal
        // must keep its other field initializers. Materialize the literal with
        // the store form first, so every field evaluates in source order, then
        // resolve the read member through the resulting init local.
        let prepared = self.materialize_containing_literal(value, guard);
        let value = prepared.as_ref().unwrap_or(value);
        let (value, move_out) = self.retain_fresh_projection_root(value, move_out, guard);
        (value, move_out.into_iter().collect())
    }

    /// Materializes the containing struct literal of a projection chain when
    /// one of its fields is a store-needing inner literal local.
    ///
    /// Resolving the projection straight to the inner local would drop the
    /// containing literal's other field initializers, including their effects;
    /// the store form evaluates every field at its own position and leaves the
    /// read member reachable through the resulting local.
    fn materialize_containing_literal(
        &mut self,
        value: &Value,
        guard: Option<&str>,
    ) -> Option<Value> {
        let mut cursor = value;
        loop {
            match &cursor.kind {
                ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => cursor = base,
                _ => break,
            }
        }
        let ValueKind::StructLit { fields, .. } = &cursor.kind else {
            return None;
        };
        let has_inner_literal = fields.iter().any(|(_, field)| {
            matches!(
                &field.kind,
                ValueKind::Place(Place {
                    base: PlaceBase::Local(name),
                    ..
                }) if self.init_literal_locals.contains(name)
            )
        });
        if !has_inner_literal {
            return None;
        }
        let stored = self.materialize_init_literal(cursor, guard, Self::view_select(value));
        Some(Self::rebind_projection_root(value, stored))
    }

    /// The fresh owning root of a projection chain, with the chain's accessors.
    ///
    /// The chain walks down through members and constant-index elements to a
    /// call or closure result; the accessors read outermost-first, so they
    /// spell the read position over the root's place. `None` when the chain
    /// has no step, names no fresh call root, owns nothing to release, or ends
    /// in a fixed array the C cannot assign through.
    fn fresh_projection_chain<'v>(
        &self,
        value: &'v Value,
    ) -> Option<(&'v Value, Vec<ProjectionAccessor>)> {
        let mut accessors: Vec<ProjectionAccessor> = Vec::new();
        let mut cursor = value;
        loop {
            match &cursor.kind {
                ValueKind::Member { base, name } => {
                    accessors.push(ProjectionAccessor::Member(name.clone()));
                    cursor = base;
                }
                ValueKind::Index { base, index } => {
                    // A constant index keeps the source spelling `[0]`; a
                    // `usize` literal would render its `u` suffix. A run-time
                    // index keeps the exact IR value the source evaluated, so
                    // the read and the move-out share one evaluation.
                    let index_value = match &index.kind {
                        ValueKind::Int(position) => {
                            Value::new(Ty::i32_ty(), ValueKind::Int(*position))
                        }
                        _ => (**index).clone(),
                    };
                    accessors.push(ProjectionAccessor::Index(index_value));
                    cursor = base;
                }
                _ => break,
            }
        }

        // A literal whose field is a store-needing inner literal keeps that
        // field as an init-local place. Resolve the first accessor through the
        // literal's fields so a nested projection roots at the inner local.
        if let ValueKind::StructLit { fields, .. } = &cursor.kind {
            if let Some(ProjectionAccessor::Member(field)) = accessors.last() {
                if let Some((_, field_value)) = fields.iter().find(|(name, _)| name == field) {
                    if let ValueKind::Place(Place {
                        base: PlaceBase::Local(name),
                        ..
                    }) = &field_value.kind
                    {
                        if self.init_literal_locals.contains(name) {
                            cursor = field_value;
                            accessors.pop();
                        }
                    }
                }
            }
        }
        let literal_root = matches!(
            &cursor.kind,
            ValueKind::Place(Place {
                base: PlaceBase::Local(name),
                ..
            }) if self.init_literal_locals.contains(name)
        );
        let call_root = matches!(
            &cursor.kind,
            ValueKind::Place(Place {
                base: PlaceBase::Local(name),
                ..
            }) if self.init_call_locals.contains(name)
        );
        if accessors.is_empty()
            || !(matches!(
                cursor.kind,
                ValueKind::Call { .. }
                    | ValueKind::ClosureCall { .. }
                    | ValueKind::StructLit { .. }
            ) || literal_root
                || call_root)
            || !self.needs_drop(&cursor.ty)
            // The read position itself need not own storage: a view position
            // reads storage the root never owned, so the root still binds for
            // its remaining fields and the view position empties like the
            // owning ones.
            || value.ty.is_fixed_array()
        {
            return None;
        }
        accessors.reverse();
        Some((cursor, accessors))
    }

    /// Renders a projection chain over a bound root.
    ///
    /// A run-time index renders the same IR value the read uses, so the
    /// move-out cannot evaluate the index a second time.
    fn accessors_text(&self, accessors: &[ProjectionAccessor]) -> String {
        accessors
            .iter()
            .map(|accessor| match accessor {
                ProjectionAccessor::Member(name) => format!(".{name}"),
                ProjectionAccessor::Index(index) => format!("[{}]", self.value(index)),
            })
            .collect()
    }

    /// Binds a fresh owning root under a projection chain and empties the read
    /// position after the store.
    ///
    /// A value with no projection step and a chain step that names no constant
    /// position keep their inline form: the first is the destination's own
    /// value, and the second has no position the move-out could empty ahead of
    /// the release.
    fn retain_fresh_projection_root(
        &mut self,
        value: &Value,
        move_out: bool,
        guard: Option<&str>,
    ) -> (Value, Option<String>) {
        let Some((root, accessors)) = self.fresh_projection_chain(value) else {
            return (value.clone(), None);
        };
        let local_root = match &root.kind {
            ValueKind::Place(Place {
                base: PlaceBase::Local(name),
                ..
            }) if self.init_literal_locals.contains(name) || self.init_call_locals.contains(name) => {
                Some(name.clone())
            }
            _ => None,
        };
        let (local, root_read) = if let Some(name) = local_root {
            // The literal already fills an init local of its own. Register
            // only the resolved root: a containing literal materialized by the
            // walk has already moved its inner fields into this root, while a
            // sibling literal whose member widened into a view stored elsewhere
            // must stay retained or the view backing is freed.
            if !self.init_owners.iter().any(|(owner, _)| owner == &name) {
                self.init_owners.push((name.clone(), root.ty.clone()));
            }
            (name, root.clone())
        } else {
            let zero = self.empty_state(&root.ty);
            // The root's fields free at the initializer's end: the release
            // registration must see the local even under a retaining
            // destination.
            let retaining = std::mem::replace(&mut self.init_retain_owners, false);
            let (local, root_read) = self.materialize_init_local(root, guard, &zero);
            self.init_retain_owners = retaining;
            (local, root_read)
        };
        let read = self.rebuild_read_path(root_read, &accessors, &value.ty);
        let empty = match zero_view_helper(&value.ty) {
            // A struct-typed view position spells its empty state with the
            // zero view helper, exactly as a store into it would.
            Some(helper) => format!("{helper}()"),
            None => self.empty_state(&value.ty),
        };
        (
            read,
            move_out.then(|| format!("{local}{} = {empty}", self.accessors_text(&accessors))),
        )
    }

    /// Binds each fresh projection root a conditional's arms read.
    ///
    /// Each arm is evaluated only when it is selected, so its root binds under
    /// the arm's own guard: the arm's call runs exactly once on its path, the
    /// root registers with the release pass, and the read position empties
    /// after the store so the retained view keeps the storage while the root's
    /// other fields free. The condition stores once, so both guards read the
    /// same decision, and an arm with no fresh projection keeps its form.
    fn retain_conditional_projection_roots(
        &mut self,
        value: &Value,
        outer_guard: Option<&str>,
        move_out: bool,
    ) -> (Value, Vec<String>) {
        let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &value.kind
        else {
            return (value.clone(), Vec::new());
        };
        if self.fresh_projection_chain(then_value).is_none()
            && self.fresh_projection_chain(else_value).is_none()
        {
            return (value.clone(), Vec::new());
        }
        // The arm roots bind before the store and the store re-reads the
        // condition. A condition that is a place or a fresh expression can be
        // changed by an arm's call, so store it once and let the guards and the
        // store all read the same local. A condition the walk already stored is
        // reused as is.
        let (condition, then_guard, else_guard) = match stored_init_read_name(condition) {
            Some(name) => ((**condition).clone(), name.clone(), format!("!{name}")),
            None => {
                let (name, read) = self.materialize_init_local(condition, outer_guard, "false");
                (read, name.clone(), format!("!{name}"))
            }
        };
        let (then_guard, else_guard) = match outer_guard {
            Some(enclosing) => (
                format!("{enclosing} && {then_guard}"),
                format!("{enclosing} && {else_guard}"),
            ),
            None => (then_guard, else_guard),
        };
        let mut move_outs = Vec::new();
        let then_value = self.retain_guarded_projection_root(
            then_value,
            &then_guard,
            &mut move_outs,
            move_out,
        );
        let else_value = self.retain_guarded_projection_root(
            else_value,
            &else_guard,
            &mut move_outs,
            move_out,
        );
        (
            Value::new(
                value.ty.clone(),
                ValueKind::Conditional {
                    condition: Box::new(condition),
                    then_value: Box::new(then_value),
                    else_value: Box::new(else_value),
                },
            ),
            move_outs,
        )
    }

    /// Binds one conditional arm's fresh projection root under the arm's guard.
    ///
    /// The arm's root call runs only when the guard holds; the read then
    /// follows the root's bound place. An arm with no fresh projection chain
    /// keeps its value.
    fn retain_guarded_projection_root(
        &mut self,
        arm: &Value,
        guard: &str,
        move_outs: &mut Vec<String>,
        move_out: bool,
    ) -> Value {
        // A nested conditional has no single projection chain of its own. Its
        // arms still need retention under the composed outer guard.
        if matches!(arm.kind, ValueKind::Conditional { .. }) {
            let (value, mut nested) =
                self.retain_conditional_projection_roots(arm, Some(guard), move_out);
            move_outs.append(&mut nested);
            return value;
        }
        let Some((root, accessors)) = self.fresh_projection_chain(arm) else {
            return arm.clone();
        };
        let zero = self.empty_state(&root.ty);
        // The root's fields free at the initializer's end: the release
        // registration must see the local even under a retaining destination.
        let retaining = std::mem::replace(&mut self.init_retain_owners, false);
        let (local, root_read) = self.materialize_init_local(root, Some(guard), &zero);
        self.init_retain_owners = retaining;
        let read = self.rebuild_read_path(root_read, &accessors, &arm.ty);
        let empty = match zero_view_helper(&arm.ty) {
            Some(helper) => format!("{helper}()"),
            None => self.empty_state(&arm.ty),
        };
        if move_out {
            // The arm's root call and its read ran only under this guard, so
            // the move-out must run there too: an untaken arm still holds the
            // zero state, and indexing that zero array with the source's
            // run-time index would write out of bounds. An unselected element
            // keeps every root whole so the release pass frees it.
            move_outs.push(format!(
                "if ({guard}) {{ {local}{} = {empty}; }}",
                self.accessors_text(&accessors)
            ));
        }
        read
    }

    /// Binds every fresh member root a retained view array reads.
    ///
    /// A view array stores one view per element, so each element that projects
    /// a member out of a fresh owning call binds its root and empties the read
    /// position after the fills; the roots' remaining fields then free at the
    /// initializer's end. Returns the rewritten array and the move-out
    /// statements to emit after the element stores.
    fn retain_view_array_roots(&mut self, value: &Value) -> (Value, Vec<String>) {
        if !self.init_retain_owners || self.init_skip_owner_binding {
            return (value.clone(), Vec::new());
        }
        let ValueKind::ArrayLit { elements, fill } = &value.kind else {
            return (value.clone(), Vec::new());
        };
        let mut move_outs = Vec::new();
        let elements = elements
            .iter()
            .map(|element| {
                let (element, move_out) = self.retain_fresh_projection_root(element, true, None);
                move_outs.extend(move_out);
                element
            })
            .collect();
        let fill = fill.as_ref().map(|fill| {
            let (fill, move_out) = self.retain_fresh_projection_root(fill, true, None);
            move_outs.extend(move_out);
            Box::new(fill)
        });
        (
            Value::new(value.ty.clone(), ValueKind::ArrayLit { elements, fill }),
            move_outs,
        )
    }

    /// Rebuilds a projection chain over a rebound root.
    ///
    /// The chain's own members and indexes stay, so the read reaches the same
    /// position the source expression named, but it now reads through the
    /// bound owner.
    fn rebind_projection_root(value: &Value, root: Value) -> Value {
        match &value.kind {
            ValueKind::Member { base, name } => Value::new(
                value.ty.clone(),
                ValueKind::Member {
                    base: Box::new(Self::rebind_projection_root(base, root)),
                    name: name.clone(),
                },
            ),
            ValueKind::Index { base, index } => Value::new(
                value.ty.clone(),
                ValueKind::Index {
                    base: Box::new(Self::rebind_projection_root(base, root)),
                    index: index.clone(),
                },
            ),
            _ => root,
        }
    }

    /// Rebuilds a projection read from a root and its accessor steps.
    ///
    /// The resolved chain may have dropped a step after a struct literal was
    /// replaced by one of its init-local fields, so the original spelling
    /// cannot simply be rebound at the root. Each step keeps the declared type
    /// for an intermediate projection and the final read type for the last.
    fn rebuild_read_path(
        &self,
        mut read: Value,
        accessors: &[ProjectionAccessor],
        final_ty: &Ty,
    ) -> Value {
        for (index, accessor) in accessors.iter().enumerate() {
            let last = index + 1 == accessors.len();
            match accessor {
                ProjectionAccessor::Member(field) => {
                    let ty = if last {
                        final_ty.clone()
                    } else {
                        self.sema
                            .table
                            .instance_field(&read.ty, field)
                            .unwrap_or_else(Ty::unknown)
                    };
                    read = Value::new(
                        ty,
                        ValueKind::Member {
                            base: Box::new(read),
                            name: field.clone(),
                        },
                    );
                }
                ProjectionAccessor::Index(index_value) => {
                    let ty = if last {
                        final_ty.clone()
                    } else {
                        read.ty.iterated_element()
                    };
                    read = Value::new(
                        ty,
                        ValueKind::Index {
                            base: Box::new(read),
                            // The index value is shared with the move-out, so
                            // a run-time index is evaluated exactly once.
                            index: Box::new(index_value.clone()),
                        },
                    );
                }
            }
        }
        read
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
        // A view array of fresh members retains each member while its root's
        // remaining fields release: a projection over a fresh owning root
        // binds that root first, and every element's root binds into an init
        // local, with the read positions emptied after the fills.
        // Prepare the value first: every condition and run-time index becomes
        // an init local before retention records a move-out. Retaining first
        // would copy the raw index expression into the move-out and evaluate it
        // a second time there.
        let prepared = self.prepare_fill_value(value, None);
        let prepared = self.retain_view_array_fill_root(&prepared);
        let (prepared, move_outs) = self.retain_view_array_roots(&prepared);
        self.emit_filled_array(target, ty, &prepared);
        for move_out in move_outs {
            self.push(&format!("    {move_out};"));
        }
    }

    /// Binds the fresh root under a retained fixed-array view fill.
    ///
    /// A fixed-array view destination keeps every element's storage, but the
    /// elements of a member projection read the root's own field: the root
    /// binds with its release registered, so its remaining owning fields free
    /// at the initializer's end while the element views keep reading what they
    /// name. Only a position whose elements own nothing binds this way — an
    /// owning element would be freed while the destination still reads it —
    /// and a value that is not a projection over a fresh owning call keeps its
    /// form.
    fn retain_view_array_fill_root(&mut self, value: &Value) -> Value {
        if !self.init_retain_owners || self.needs_drop(&value.ty.iterated_element()) {
            return value.clone();
        }
        let mut cursor = value;
        loop {
            match &cursor.kind {
                ValueKind::Member { base, .. } => cursor = base,
                ValueKind::Index { base, index } => {
                    if !matches!(index.kind, ValueKind::Int(_)) {
                        return value.clone();
                    }
                    cursor = base;
                }
                _ => break,
            }
        }
        // The preparation pass may already have bound the root call into an
        // init local. The retention must register that local; reading it as a
        // place and binding it again is impossible (C cannot assign an array),
        // and skipping it would free no residual owning fields.
        let prepared_root = match &cursor.kind {
            ValueKind::Place(Place {
                base: PlaceBase::Local(name),
                ..
            }) if self.init_call_locals.contains(name) => Some(name.clone()),
            _ => None,
        };
        if prepared_root.is_none()
            && !matches!(cursor.kind, ValueKind::Call { .. } | ValueKind::ClosureCall { .. })
        {
            return value.clone();
        }
        if !self.needs_drop(&cursor.ty) {
            return value.clone();
        }
        if let Some(name) = prepared_root {
            if !self.init_owners.iter().any(|(owner, _)| owner == &name) {
                self.init_owners.push((name, cursor.ty.clone()));
            }
            return value.clone();
        }
        let zero = self.empty_state(&cursor.ty);
        let retaining = std::mem::replace(&mut self.init_retain_owners, false);
        let (_, root) = self.materialize_init_local(cursor, None, &zero);
        self.init_retain_owners = retaining;
        Self::rebind_projection_root(value, root)
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

    /// Binds a fresh nullable owned source whose view store needs the guard.
    ///
    /// The guard reads its source twice, so a source that is not a place-rooted
    /// read — a call result or a conditional over one — binds into an init
    /// local first and the guard reads the local. The view destination retains
    /// the bound storage exactly as it retains one written directly. A
    /// place-rooted, non-nullable, or non-`String` source keeps its form.
    fn bind_guarded_view_source(&mut self, value: &Value, view: &Ty) -> Value {
        let fresh = match &value.kind {
            ValueKind::Conditional { .. } => self.conditional_arms_need_guard_binding(value),
            _ => {
                self.needs_view_guard(&self.resolved_value_ty(value), view)
                    && !self.reads_stable_storage(value)
            }
        };
        if !fresh {
            return value.clone();
        }
        let zero = self.empty_state(&value.ty);
        let (_, mut local) = self.materialize_init_local(value, None, &zero);
        // A conditional whose arm carries the nullness can hide it behind the
        // sibling arm's matching type; the bound local stands for a value that
        // can be null, so the store's guard has to fire.
        if value.selects_null() {
            local.ty.nullable = true;
        }
        local
    }

    /// The declared parameter types of a call's callee, when the emitter can
    /// name them.
    ///
    /// A user or `ffi` function keeps its declared signature; a call through a
    /// func value takes the parameter types from the value's own signature.
    /// Every other callee yields `None`, and its arguments keep their form.
    fn call_expected_types(&self, callee: &str, receiver: Option<&Value>) -> Option<Vec<Ty>> {
        if let Some(types) = self.callee_param_types(callee) {
            return Some(types);
        }
        let ty = self.callee_func_type(callee, receiver)?;
        let TyKind::Func(signature) = &ty.kind else {
            return None;
        };
        Some(signature.params.iter().map(|param| param.ty.clone()).collect())
    }

    /// True when an argument's widening at `expected` needs a bound source.
    ///
    /// The widening tests its source and selects the parameter's empty view
    /// for a null one, so an argument that reads a fresh value and can select
    /// null binds to a local that the guard reads twice. A stable source, a
    /// non-view parameter, and a source that cannot select null keep their
    /// form.
    fn nullable_view_argument_needs_binding(&self, arg: &Value, expected: &Ty) -> bool {
        is_struct_view(&self.substitute(expected))
            && arg.selects_null()
            && !self.reads_stable_storage(arg)
    }

    /// Binds an argument whose view widening needs a guard.
    ///
    /// The bound local is the argument's source, and the view destination
    /// retains it exactly as it retains one written directly: the storage the
    /// view points into lives as long as the program.
    fn bind_nullable_view_argument(&mut self, arg: &Value, guard: Option<&str>) -> Value {
        let zero = self.empty_state(&arg.ty);
        let (_, mut local) = self.materialize_init_local(arg, guard, &zero);
        // A conditional whose arm carries the nullness can hide it behind the
        // sibling arm's matching type; the bound local stands for a value that
        // can be null.
        if arg.selects_null() {
            local.ty.nullable = true;
        }
        local
    }

    /// True when a conditional's view store needs the guard on an arm that
    /// cannot spell it.
    ///
    /// The guard reads its source twice, so an arm that reads a fresh value
    /// and can select null — a call result, a field of one, or a nested
    /// conditional over either — binds with the whole conditional into an init
    /// local first. A null, stable, or non-null arm renders its own guard or
    /// its plain widening.
    fn conditional_arms_need_guard_binding(&self, value: &Value) -> bool {
        let ValueKind::Conditional {
            then_value,
            else_value,
            ..
        } = &value.kind
        else {
            return false;
        };
        self.conditional_arm_needs_guard_binding(then_value)
            || self.conditional_arm_needs_guard_binding(else_value)
    }

    /// True when one conditional arm needs the view guard but reads a fresh
    /// value.
    fn conditional_arm_needs_guard_binding(&self, value: &Value) -> bool {
        if matches!(value.kind, ValueKind::Null) || self.carries_null_shape(value) {
            return false;
        }
        if matches!(value.kind, ValueKind::Conditional { .. }) {
            return self.conditional_arms_need_guard_binding(value);
        }
        // The arm's own type is the owned handle the widening wraps, and the
        // arm can select null: the guard has to read a bound place.
        matches!(
            &self.substitute(&value.ty).kind,
            TyKind::Handle(name) if name == "String"
        ) && value.selects_null()
            && !self.reads_stable_storage(value)
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
        if matches!(
            &value.kind,
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
        ) {
            self.init_call_locals.insert(name.clone());
        }
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
        } else if self.substitute(&value.ty).is_fixed_array() {
            // A fixed-array value has no C assignment form: a brace list or
            // an arm-selected array cannot be assigned to the local. Declare
            // it zeroed and fill it element-wise at the value's own position,
            // so a later field's fill cannot hoist above its effects.
            let empty = self.empty_state(&value.ty);
            self.push(&format!(
                "    {} = {empty};",
                declarator(&value.ty, &name, &self.sema.table)
            ));
            let target = ArrayTarget::object(&name);
            match guard {
                Some(enclosing) => {
                    self.push(&format!("    if ({enclosing}) {{"));
                    self.emit_fixed_array_assign_to(&target, &value.ty, value, false, false);
                    self.push("    }");
                }
                None => {
                    self.emit_fixed_array_assign_to(&target, &value.ty, value, false, false);
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
        if !self.init_retain_owners && !self.init_skip_owner_binding && self.needs_drop(&value.ty) {
            // The local holds an owned value, so the release pass frees it at
            // the init function's end; a view destination retains it, and a
            // consumed argument's subtree records nothing.
            self.init_owners.push((name.clone(), value.ty.clone()));
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
    fn materialize_init_literal(
        &mut self,
        value: &Value,
        guard: Option<&str>,
        select: ViewSelect,
    ) -> Value {
        let ty = value.ty.clone();
        let fields = match &value.kind {
            ValueKind::StructLit { fields, .. } => fields.clone(),
            _ => return value.clone(),
        };
        let name = self.next_init_local();
        if !self.init_skip_owner_binding {
            self.init_literal_locals.insert(name.clone());
            self.init_global_literals.push((name.clone(), ty.clone()));
        }
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
        let saved_walked_guards = self.init_walked_guards;
        // Under a branch guard the walk leaves operand guards to the store
        // core, which runs them inside the arm. Only the unguarded path has
        // already emitted them.
        self.init_walked_guards = guard.is_none();
        let mut stored_fields = fields;
        let mut guarded_move_outs = Vec::new();
        if let Some(enclosing) = guard {
            // Retain the field roots before the store block opens. The local
            // declarations then live in the enclosing scope while the call and
            // the move-out stay under the arm guard; declaring them inside the
            // block would make the function-scope release name an undeclared
            // local.
            let (retained, mut move_outs) = self.retain_guarded_literal_fields(
                &stored_fields,
                select.clone(),
                enclosing,
            );
            stored_fields = retained;
            guarded_move_outs.append(&mut move_outs);
        }
        match guard {
            Some(enclosing) => {
                self.push(&format!("    if ({enclosing}) {{"));
                self.emit_init_literal_fields(
                    &target_c,
                    &stored_fields,
                    select,
                    guard,
                    guard.is_some(),
                );
                for move_out in guarded_move_outs {
                    self.push(&format!("    {move_out};"));
                }
                self.push("    }");
            }
            None => {
                self.emit_init_literal_fields(&target_c, &stored_fields, select, guard, false)
            }
        }
        self.init_walked_guards = saved_walked_guards;
        if !self.init_retain_owners && !self.init_skip_owner_binding && self.needs_drop(&ty) {
            // The literal owns fields, so the release pass frees them at the
            // init function's end; a view destination registers the local
            // only when it moves the read member out.
            self.init_owners.push((name, ty.clone()));
        }
        Value::new(ty, ValueKind::Place(target))
    }

    /// Retains the fresh roots of a literal's fields before a guarded store
    /// block opens.
    ///
    /// The guarded store path declares its root locals before the `if` and
    /// assigns them inside it. A nested literal recurses with the same field
    /// selection; a non-literal field keeps its value and is stored normally.
    fn retain_guarded_literal_fields(
        &mut self,
        fields: &[(String, Value)],
        select: ViewSelect,
        guard: &str,
    ) -> (Vec<(String, Value)>, Vec<String>) {
        let mut stored_fields = Vec::with_capacity(fields.len());
        let mut move_outs = Vec::new();
        for (field_name, field) in fields {
            let field_select = select.field(field_name);
            let field_ty = self.substitute(&field.ty);
            if field_ty.is_fixed_array() {
                let (stored, mut moves) = self.retain_view_array_field_elements(
                    field,
                    &field_ty,
                    field_select,
                    Some(guard),
                );
                stored_fields.push((field_name.clone(), stored));
                move_outs.append(&mut moves);
                continue;
            }
            if matches!(
                &field.kind,
                ValueKind::Call { .. }
                    | ValueKind::ClosureCall { .. }
                    | ValueKind::Conditional { .. }
            ) {
                // A field call or conditional before a fixed-array field must
                // run at its own position. Bind it here, before the array
                // retention below, so a later fill cannot hoist above it.
                let zero = self.empty_state(&field.ty);
                let (_, read) = self.materialize_init_local(field, Some(guard), &zero);
                stored_fields.push((field_name.clone(), read));
                continue;
            }
            if let ValueKind::StructLit {
                name: inner_name,
                fields: inner,
            } = &field.kind
            {
                if self.struct_literal_needs_stores(inner_name, inner) {
                    let (stored_inner, mut moves) =
                        self.retain_guarded_literal_fields(inner, field_select, guard);
                    stored_fields.push((
                        field_name.clone(),
                        Value::new(
                            field.ty.clone(),
                            ValueKind::StructLit {
                                name: inner_name.clone(),
                                fields: stored_inner,
                            },
                        ),
                    ));
                    move_outs.append(&mut moves);
                    continue;
                }
            }
            stored_fields.push((field_name.clone(), field.clone()));
        }
        (stored_fields, move_outs)
    }

    /// Retains the fresh roots a view-array field's elements read.
    ///
    /// A module initializer's view destination keeps the storage its views
    /// point into. An element such as `mk().v` reads one member of a fresh
    /// owning root, so the root binds into an init local the release pass
    /// empties and the read member is emptied after the array store: the
    /// view keeps that member's bytes while the root's other fields free.
    fn retain_view_array_field_elements(
        &mut self,
        field: &Value,
        field_ty: &Ty,
        select: ViewSelect,
        guard: Option<&str>,
    ) -> (Value, Vec<String>) {
        if !self.init_retain_owners || self.init_skip_owner_binding {
            return (field.clone(), Vec::new());
        }
        let TyKind::Array { element, extents } = &field_ty.kind else {
            return (field.clone(), Vec::new());
        };
        let nested_ty = (extents.len() > 1)
            .then(|| Ty::array((**element).clone(), extents[1..].to_vec()));
        if nested_ty.is_none() && !crate::borrow::is_view_like(element) {
            return (field.clone(), Vec::new());
        }
        let ValueKind::ArrayLit { elements, fill } = &field.kind else {
            return (field.clone(), Vec::new());
        };
        let bind_fills = !matches!(&select, ViewSelect::Whole);
        let mut stored_elements = Vec::with_capacity(elements.len());
        let mut move_outs = Vec::new();
        for (index, element_value) in elements.iter().enumerate() {
            let element_select = select.index(index);
            let (stored, mut moves) =
                if let (Some(row_ty), ValueKind::ArrayLit { .. }) = (&nested_ty, &element_value.kind)
                {
                    self.retain_view_array_field_elements(
                        element_value,
                        row_ty,
                        element_select,
                        guard,
                    )
                } else if bind_fills
                    && matches!(
                        &element_value.kind,
                        ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
                    )
                {
                    // A bare call element has no projection to bind later. It
                    // must still run before a fill that follows it, so bind it
                    // here in the element loop instead of leaving the call for
                    // the store that runs after the fill.
                    let stored = self.bind_fill_root(
                        element_value,
                        matches!(element_select, ViewSelect::Unselected),
                        guard,
                    );
                    (stored, Vec::new())
                } else if bind_fills
                    && !self.needs_drop(&element_value.ty)
                    && !self.has_fresh_projection_root(element_value)
                    && !matches!(
                        &element_value.kind,
                        ValueKind::Int(_)
                            | ValueKind::Float(_)
                            | ValueKind::Char(_)
                            | ValueKind::Str(_)
                            | ValueKind::Bool(_)
                            | ValueKind::Null
                            | ValueKind::Aggregate
                    )
                {
                    // A place, member, or conditional view element with no
                    // owner still has to be read at its source position: a
                    // later fill can mutate the place or the condition, and
                    // the store runs after the fill. Bind the view copy here.
                    let zero = self.empty_state(&element_value.ty);
                    let (_, read) = self.materialize_init_local(element_value, guard, &zero);
                    (read, Vec::new())
                } else {
                    self.retain_view_member_root(
                        element_value,
                        !matches!(element_select, ViewSelect::Unselected),
                        guard,
                    )
                };
            stored_elements.push(stored);
            move_outs.append(&mut moves);
        }
        let fill = fill.as_deref().map(|fill| {
            let fill_select = select.fill(elements.len());
            if let (Some(row_ty), ValueKind::ArrayLit { .. }) = (&nested_ty, &fill.kind) {
                let (stored, mut moves) =
                    self.retain_view_array_field_elements(fill, row_ty, fill_select, guard);
                move_outs.append(&mut moves);
                Box::new(stored)
            } else if bind_fills {
                if matches!(fill.kind, ValueKind::Member { .. } | ValueKind::Index { .. }) {
                    // A fill that projects one member out of a fresh root has
                    // to bind that root; a bare call binder would leave the
                    // projection inline and leak the root's siblings.
                    let (stored, mut moves) = self.retain_view_member_root(
                        fill,
                        !matches!(fill_select, ViewSelect::Unselected),
                        guard,
                    );
                    move_outs.append(&mut moves);
                    Box::new(stored)
                } else {
                    Box::new(self.bind_fill_root(
                        fill,
                        matches!(fill_select, ViewSelect::Unselected),
                        guard,
                    ))
                }
            } else {
                Box::new(fill.clone())
            }
        });
        (
            Value::new(
                field.ty.clone(),
                ValueKind::ArrayLit {
                    elements: stored_elements,
                    fill,
                },
            ),
            move_outs,
        )
    }

    /// True when a view element roots at a fresh projection, either directly
    /// or through a conditional arm.
    ///
    /// Such an element must go through `retain_view_member_root` so the fresh
    /// root binds and its residual owning fields release; a plain view copy
    /// would strand the root.
    fn has_fresh_projection_root(&self, value: &Value) -> bool {
        if self.fresh_projection_chain(value).is_some() {
            return true;
        }
        if let ValueKind::Conditional {
            then_value,
            else_value,
            ..
        } = &value.kind
        {
            return self.has_fresh_projection_root(then_value)
                || self.has_fresh_projection_root(else_value);
        }
        false
    }

    /// Binds a fill row's fresh owning root at its source position.
    ///
    /// A fill row under a constant selection binds both the selected and
    /// unselected roots so the calls keep source order. The selected root is
    /// not registered: the stored global view still reads its backing. The
    /// unselected root is registered in `init_owners`, so the init release pass
    /// frees it. A place or a conditional keeps its current form.
    fn bind_fill_root(&mut self, fill: &Value, register: bool, guard: Option<&str>) -> Value {
        // Bind every call, not only an owning one: a view-returning call has
        // no release of its own, but its effects still have to run at this
        // source position instead of being hoisted below a later fill.
        let fresh = matches!(
            &fill.kind,
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
        ) && !self.reads_stable_storage(fill);
        if !fresh {
            return fill.clone();
        }
        let zero = self.empty_state(&fill.ty);
        // A selected root stays retained; an unselected root is not stored
        // anywhere, so let `materialize_init_local` record it for release.
        let retaining = std::mem::replace(&mut self.init_retain_owners, !register);
        let (_, read) = self.materialize_init_local(fill, guard, &zero);
        self.init_retain_owners = retaining;
        read
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
    fn emit_init_literal_fields(
        &mut self,
        target_c: &str,
        fields: &[(String, Value)],
        select: ViewSelect,
        guard: Option<&str>,
        already_retained: bool,
    ) {
        for (field_name, field) in fields {
            let field_ty = self.substitute(&field.ty);
            let field_c = format!("{target_c}.{field_name}");
            let field_select = select.field(field_name);
            if field_ty.is_fixed_array() {
                // A view-array field in a module initializer reads fresh
                // members (`names: [mk().v]`). Retain each element's root and
                // empty the read member after the store, so the root's other
                // owning fields release while the view keeps its backing. A
                // guarded literal already ran this retention before its store
                // block; the field is then a rewritten value.
                let (stored_field, move_outs) = if already_retained {
                    (field.clone(), Vec::new())
                } else {
                    self.retain_view_array_field_elements(
                        field,
                        &field_ty,
                        field_select.clone(),
                        guard,
                    )
                };
                let field_target = ArrayTarget::object(&field_c);
                self.emit_fixed_array_assign_to(
                    &field_target,
                    &field_ty,
                    &stored_field,
                    false,
                    false,
                );
                for move_out in move_outs {
                    self.push(&format!("    {move_out};"));
                }
                continue;
            }
            if let ValueKind::StructLit {
                name: inner_name,
                fields: inner,
            } = &field.kind
            {
                if self.struct_literal_needs_stores(inner_name, inner) {
                    self.emit_init_literal_fields(
                        &field_c,
                        inner,
                        field_select,
                        guard,
                        already_retained,
                    );
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

    /// The parameter modes a call applies, from the callee's own signature.
    ///
    /// A value callee (a func-typed global, field, or closure value) carries
    /// its own signature, which is authoritative for its modes; a declared or
    /// `ffi` name resolves by name as the fallback. `None` when neither
    /// resolves: the arguments then keep their owned values here, which is
    /// the lend convention.
    fn call_param_modes(
        &self,
        callee: &str,
        receiver: Option<&Value>,
    ) -> Option<Vec<crate::ast::Mode>> {
        if let Some(ty) = self.callee_func_type(callee, receiver) {
            if let TyKind::Func(signature) = &ty.kind {
                return Some(signature.params.iter().map(|param| param.mode).collect());
            }
        }
        self.callee_modes(callee)
    }

    /// True when a declared type reaches view storage anywhere it stores.
    ///
    /// A view destination keeps the storage it points into for the program's
    /// lifetime, so its initializer's owned values are retained rather than
    /// released. Views hide behind arrays and struct fields too, so the check
    /// descends both; a generic field that names no resolved view stays out,
    /// because the retention question is only ever asked of a concrete
    /// destination type.
    fn stores_view_storage(&self, ty: &Ty) -> bool {
        match &ty.kind {
            TyKind::Str | TyKind::StaticStr | TyKind::CString => true,
            // A slice is a view of its owner's storage, so its initializer's
            // backing must outlive the program exactly like a `str` view.
            TyKind::Array { .. } if ty.is_slice() => true,
            TyKind::Array { element, .. } => self.stores_view_storage(element),
            TyKind::Struct(name, _) => self
                .sema
                .table
                .struct_fields(name)
                .is_some_and(|fields| {
                    fields
                        .iter()
                        .any(|(_, field)| self.stores_view_storage(field))
                }),
            _ => false,
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
    /// - an impure index, index base, dividend, divisor, or cast operand
    ///   stores once into an init local, and the guard that checks it is
    ///   emitted right there, so the guard and the read share one evaluation
    ///   and a later operand's effects cannot run before an earlier operand's
    ///   trap;
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
                // The parameter modes decide which arguments move into the
                // callee before the arguments are prepared: nothing in a
                // consumed argument's subtree records an owner, because the
                // callee frees the value. The parameter types let an owned
                // argument at a view parameter bind first: its widening tests
                // the value, so the guard has to read a place.
                let modes = self.call_param_modes(callee, receiver.as_deref());
                let expected = self.call_expected_types(callee, receiver.as_deref());
                let receiver = receiver
                    .as_deref()
                    .map(|inner| Box::new(self.materialize_init_value(inner, guard)));
                let mut written: Vec<Value> = Vec::new();
                for (index, arg) in args.iter().enumerate() {
                    let consumed = modes.as_ref().and_then(|modes| modes.get(index))
                        == Some(&crate::ast::Mode::Take);
                    let restore = self.init_skip_owner_binding;
                    if consumed {
                        self.init_skip_owner_binding = true;
                    }
                    let walked = self.materialize_init_value(arg, guard);
                    let prepared = match expected.as_ref().and_then(|types| types.get(index)) {
                        Some(expected)
                            if self.nullable_view_argument_needs_binding(&walked, expected) =>
                        {
                            self.bind_nullable_view_argument(&walked, guard)
                        }
                        _ => walked,
                    };
                    written.push(prepared);
                    self.init_skip_owner_binding = restore;
                }
                let args = written;
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
                // An owned result used by a larger expression binds into an
                // init local too, so the release pass frees it; a view
                // destination retains it instead.
                if self.init_result_needs_binding(&value.ty) {
                    let zero = self.empty_state(&value.ty);
                    let (_, local) = self.materialize_init_local(&rewritten, guard, &zero);
                    return local;
                }
                rewritten
            }
            ValueKind::ClosureCall { callable, args } => {
                // A closure value carries its own parameter modes, exactly as
                // a func-typed global or field does.
                let modes = self.call_param_modes("", Some(callable));
                let callable = Box::new(self.materialize_init_value(callable, guard));
                let mut written: Vec<Value> = Vec::new();
                for (index, arg) in args.iter().enumerate() {
                    let consumed = modes.as_ref().and_then(|modes| modes.get(index))
                        == Some(&crate::ast::Mode::Take);
                    let restore = self.init_skip_owner_binding;
                    if consumed {
                        self.init_skip_owner_binding = true;
                    }
                    written.push(self.materialize_init_value(arg, guard));
                    self.init_skip_owner_binding = restore;
                }
                let rewritten = Value::new(
                    value.ty.clone(),
                    ValueKind::ClosureCall {
                        callable,
                        args: written,
                    },
                );
                if self.substitute(&value.ty).is_fixed_array() {
                    let zero = self.empty_state(&value.ty);
                    let (_, local) = self.materialize_init_local(&rewritten, guard, &zero);
                    return local;
                }
                if self.init_result_needs_binding(&value.ty) {
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
                // The bounds guard and the access both render the base, so an
                // impure base (a call returning a view) stores once here and
                // both read the stored value. A fixed-array base keeps its
                // spelling: C cannot assign an array to a temporary, and the
                // element access reads it through the array's decay.
                let base = if base.ty.is_fixed_array() {
                    base
                } else {
                    self.store_impure_init_operand(&base, guard)
                };
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
                    // An owned bind the right side declares lives in this
                    // decision block, so its release runs at the block's end:
                    // the end-of-init release pass stands outside the block
                    // and could not name it. The binds keep their reverse
                    // creation order, and the untaken path releases nothing.
                    let declared_here = self.init_owners.len();
                    let right = self.materialize_init_value(right, None);
                    let rendered = self.value(&right);
                    self.push(&format!("        {name} = {rendered};"));
                    let block_owners: Vec<(String, Ty)> = self.init_owners.split_off(declared_here);
                    for (owner, ty) in block_owners.into_iter().rev() {
                        self.emit_drop_expr(&owner, &ty);
                    }
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
                // The division guard renders the dividend too, so an impure
                // dividend stores once before the guard and the operation
                // read the same value.
                let left = if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                    self.store_impure_init_operand(&left, guard)
                } else {
                    left
                };
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
                // A conditional has no single constant path, but each arm is
                // read independently and may name its own stored position.
                // Carry the arm's selection into the arm walk; otherwise a
                // selected arm's unselected sibling fields are treated as if
                // they were stored too, and their move-outs leak the backing.
                let outer_select = std::mem::replace(&mut self.init_select, ViewSelect::Whole);
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
                let then_select = match &outer_select {
                    ViewSelect::Whole => Self::view_select(then_value),
                    _ => outer_select.clone(),
                };
                let else_select = match &outer_select {
                    ViewSelect::Whole => Self::view_select(else_value),
                    _ => outer_select.clone(),
                };
                self.init_select = then_select;
                let then_value = self.materialize_init_value(then_value, Some(&then_guard));
                self.init_select = else_select;
                let else_value = self.materialize_init_value(else_value, Some(&else_guard));
                self.init_select = ViewSelect::Whole;
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
                // The first literal on the constant projection chain consumes
                // the global initializer's selection. A nested field literal
                // receives its own field selection just below; every other
                // literal sees Whole and keeps the previous retention rule.
                let select = std::mem::replace(&mut self.init_select, ViewSelect::Whole);
                let mut walked_fields: Vec<(String, Value)> = Vec::with_capacity(fields.len());
                for (index, (field_name, field_value)) in fields.iter().enumerate() {
                    let field_select = select.field(field_name);
                    // A conditional field's arms are the field's value, so the
                    // field path applies to each arm exactly as a nested
                    // literal's path does. Without it the arms walk as Whole
                    // and an unselected element is moved out and leaked.
                    self.init_select = if matches!(
                        &field_value.kind,
                        ValueKind::StructLit { .. } | ValueKind::Conditional { .. }
                    ) {
                        field_select.clone()
                    } else {
                        ViewSelect::Whole
                    };
                    let walked = self.materialize_init_value(field_value, guard);
                    self.init_select = ViewSelect::Whole;
                    // An impure field evaluates at its own source position. A
                    // later field's statement fill must not hoist above that
                    // effect, so an earlier effectful field stores once into
                    // an init local here. Only a value whose copy carries no
                    // ownership is stored; an owned result was already bound
                    // by its own walk.
                    let later_effect = fields[index + 1..].iter().any(|(_, later)| {
                        !init_operand_is_pure(later)
                            || self.init_value_needs_materialization(later)
                    });
                    let already_stored = stored_init_read_name(&walked).is_some();
                    let constant = matches!(
                        walked.kind,
                        ValueKind::Int(_)
                            | ValueKind::Float(_)
                            | ValueKind::Char(_)
                            | ValueKind::Str(_)
                            | ValueKind::Bool(_)
                            | ValueKind::Null
                            | ValueKind::Aggregate
                    );
                    // A later field's fill can mutate a place the earlier
                    // field reads, so every non-constant earlier value stores
                    // at its source position, not only an impure one. An owned
                    // value was already bound by its own walk.
                    let walked = if later_effect
                        && !already_stored
                        && !constant
                        && !self.needs_drop(&walked.ty)
                    {
                        // A view-array field stored before a later effect can
                        // read fresh members (`names: [mk().v]`). Retain each
                        // element's root and empty the read member after the
                        // array local is filled, so the root's other owning
                        // fields release while the view keeps its backing.
                        let (walked, move_outs) = if self.init_retain_owners {
                            self.retain_view_array_field_elements(
                                &walked,
                                &walked.ty,
                                field_select.clone(),
                                guard,
                            )
                        } else {
                            (walked, Vec::new())
                        };
                        let zero = self.empty_state(&walked.ty);
                        let (_, local) = self.materialize_init_local(&walked, guard, &zero);
                        for move_out in move_outs {
                            self.push(&format!("    {move_out};"));
                        }
                        local
                    } else {
                        walked
                    };
                    walked_fields.push((field_name.clone(), walked));
                }
                let walked = Value::new(
                    value.ty.clone(),
                    ValueKind::StructLit {
                        name: name.clone(),
                        fields: walked_fields,
                    },
                );
                // A literal whose fields need statements (a fixed-array field,
                // a deferred fill, a nested literal) has no C expression form:
                // a compound literal cannot carry an array member initialized
                // from an expression. Bind it into an init local and store the
                // fields per statement, as the body's assignment path does.
                match &walked.kind {
                    ValueKind::StructLit { name, fields }
                        if self.struct_literal_needs_stores(name, fields) =>
                    {
                        self.materialize_init_literal(&walked, guard, select)
                    }
                    _ => walked,
                }
            }
            ValueKind::Template(pieces) => {
                let walked = Value::new(
                    value.ty.clone(),
                    ValueKind::Template(
                        pieces
                            .iter()
                            .map(|piece| match piece {
                                ValuePiece::Literal(text) => ValuePiece::Literal(text.clone()),
                                ValuePiece::Placeholder(inner) => ValuePiece::Placeholder(
                                    self.materialize_init_value(inner, guard),
                                ),
                            })
                            .collect(),
                    ),
                );
                // A formatted value owns its buffer; bind it so the release
                // pass frees it, unless a view destination retains it.
                if !self.init_retain_owners && self.needs_drop(&value.ty) {
                    let zero = self.empty_state(&value.ty);
                    let (_, local) = self.materialize_init_local(&walked, guard, &zero);
                    return local;
                }
                walked
            }
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
    /// walk's own triggers: a fixed-array call result, an impure index, index
    /// base, dividend, divisor, or cast operand, and the same recursively. A
    /// part the walk would leave alone answers `false`, so its lazy form keeps
    /// its spelling.
    fn init_value_needs_materialization(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::Call { args, receiver, .. } => {
                self.substitute(&value.ty).is_fixed_array()
                    || self.init_result_needs_binding(&value.ty)
                    || receiver
                        .as_deref()
                        .is_some_and(|receiver| self.init_value_needs_materialization(receiver))
                    || args.iter().any(|arg| self.init_value_needs_materialization(arg))
            }
            ValueKind::ClosureCall { callable, args } => {
                self.substitute(&value.ty).is_fixed_array()
                    || self.init_result_needs_binding(&value.ty)
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
                    || !init_operand_is_pure(base) && !base.ty.is_fixed_array()
                    || !init_operand_is_pure(index)
            }
            ValueKind::Member { base, .. } => self.init_value_needs_materialization(base),
            ValueKind::Unary { operand, .. } => self.init_value_needs_materialization(operand),
            ValueKind::Binary { op, left, right } => {
                self.init_value_needs_materialization(left)
                    || self.init_value_needs_materialization(right)
                    || (matches!(op, BinaryOp::Div | BinaryOp::Rem)
                        && (!init_operand_is_pure(left) || !init_operand_is_pure(right)))
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
            ValueKind::StructLit { name, fields } => {
                // A literal whose fields need statements is bound into an init
                // local wherever it stands, so a lazily evaluated position
                // prepares it like a call that writes through a destination.
                self.struct_literal_needs_stores(name, fields)
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
            ValueKind::Template(pieces) => {
                self.init_result_needs_binding(&value.ty)
                    || pieces.iter().any(|piece| match piece {
                        ValuePiece::Literal(_) => false,
                        ValuePiece::Placeholder(inner) => {
                            self.init_value_needs_materialization(inner)
                        }
                    })
            }
            _ => false,
        }
    }

    /// True when an owned call result must bind into an init local.
    ///
    /// The walk binds an owned result so the release pass frees it, except
    /// when the destination retains the storage (a view destination) or the
    /// value moves into a consuming callee; a lazily evaluated position
    /// prepares the same way, so this predicate gates the structured
    /// conditional and logical paths too.
    fn init_result_needs_binding(&self, ty: &Ty) -> bool {
        !self.init_retain_owners && !self.init_skip_owner_binding && self.needs_drop(ty)
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
                // A module function value names the function symbol, even
                // when an imported value shares the head's spelling. The
                // module path wins in a module initializer exactly as it does
                // in a function body.
                if let Some(info) = self.resolve_function(&name.segments.join(".")) {
                    if info.decl.type_params.is_empty() {
                        return Value::new(ty, ValueKind::Path(name.segments.clone()));
                    }
                }
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
                        IntrinsicArg::Expr(inner) => {
                            // `offsetof`'s field spelling is not a value read.
                            // A module binding of the same name must not turn
                            // it into a place whose name the C macro cannot
                            // spell.
                            if name == "offsetof" {
                                if let ExprKind::Name(field) = &inner.kind {
                                    values.push(Value::new(
                                        Ty::unknown(),
                                        ValueKind::Path(vec![field.clone()]),
                                    ));
                                    continue;
                                }
                            }
                            values.push(self.lower_global(module, inner));
                        }
                        IntrinsicArg::Type(written) => type_arg = Some(self.resolve_written(written)),
                    }
                }
                // The free `len` spelling folds a fixed-array operand exactly
                // as the dot spelling does: the extent is a structural
                // property, so the operand never materializes and its own
                // operands are never evaluated.
                if name == "len" && type_arg.is_none() {
                    if let [receiver] = values.as_slice() {
                        if let Some(extent) = self.fixed_array_len_extent(receiver) {
                            return Value::new(Ty::usize_ty(), ValueKind::Int(extent as i128));
                        }
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
                // A func-typed global or field names the callable itself; the
                // callable carries its own signature, so the argument
                // conversions run exactly as a body call's. Every other dot
                // call passes its receiver as the first argument, exactly as
                // the body's call lowering builds its list, and the inferred
                // instance key below must align with that list.
                let callable = self.init_callable_value(module, callee);
                let receiver = match callable {
                    Some(_) => None,
                    None => self.global_dot_receiver(module, callee),
                };
                // A callee carrying its own callable, or a dot call's receiver,
                // keeps the simple member name; a module-qualified free
                // function keeps its whole path so the exact declaration is
                // reached.
                let name = if callable.is_some() || receiver.is_some() {
                    callee_name(callee).unwrap_or_default()
                } else {
                    qualified_callee_name(callee)
                };
                let resolved_type_args: Vec<Ty> = if callable.is_some() {
                    type_args
                        .iter()
                        .map(|ty| self.resolve_written(ty))
                        .collect()
                } else {
                    self.init_call_type_args(
                        module,
                        &name,
                        type_args,
                        args,
                        receiver.as_ref().map(|value| value.ty.clone()),
                    )
                };
                if let Some(callable) = callable {
                    return Value::new(
                        ty,
                        ValueKind::Call {
                            callee: callee_name(callee).unwrap_or_default(),
                            type_args: resolved_type_args,
                            args: args
                                .iter()
                                .map(|arg| self.lower_global(module, &arg.value))
                                .collect(),
                            receiver: Some(Box::new(callable)),
                        },
                    );
                }
                // An expression callee — a call result or a conditional over
                // func values — is the callable itself. Lower it as a value
                // and attach it as the call's receiver, so the emitted C calls
                // the value the callee produced instead of collapsing the
                // call into its arguments.
                if callee_name(callee).is_none() {
                    let callable = self.lower_global(module, callee);
                    return Value::new(
                        ty,
                        ValueKind::Call {
                            callee: String::new(),
                            type_args: resolved_type_args,
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
                if let Some(receiver) = receiver {
                    written_args.push(receiver);
                }
                for arg in args {
                    written_args.push(self.lower_global(module, &arg.value));
                }
                ValueKind::Call {
                    callee: name,
                    type_args: resolved_type_args,
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

    /// The resolved type arguments of a program-level call.
    ///
    /// A written list resolves to its types directly. With none written, a
    /// user generic callee derives its arguments from the checked argument
    /// types with the same matcher the body path's inference uses, so an
    /// inferred call spells the keyed instance the monomorphization declares;
    /// a callee that is not generic, or whose parameters the arguments leave
    /// unbound, resolves to no arguments. `receiver_ty` is the dot receiver
    /// the call carries as its argument position zero, so it takes part in
    /// the derivation first, exactly as the body's inference sees it.
    fn init_call_type_args(
        &self,
        module: &Module,
        name: &str,
        type_args: &[Type],
        args: &[Arg],
        receiver_ty: Option<Ty>,
    ) -> Vec<Ty> {
        if !type_args.is_empty() {
            return type_args
                .iter()
                .map(|ty| self.resolve_written(ty))
                .collect();
        }
        let Some(info) = self.resolve_function(name) else {
            return Vec::new();
        };
        let decl = info.decl;
        if decl.type_params.is_empty() {
            return Vec::new();
        }
        let declared: Vec<String> = decl
            .type_params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        let param_types: Vec<Ty> = decl
            .params
            .iter()
            .map(|param| self.resolve_in_module(&module.name, &param.ty, &declared))
            .collect();
        let mut arg_types: Vec<Ty> = receiver_ty.into_iter().collect();
        for arg in args {
            let Some(ty) = self.expr_type(&arg.value) else {
                return Vec::new();
            };
            arg_types.push(ty);
        }
        crate::sema::call::infer_type_args(&declared, &param_types, &arg_types).unwrap_or_default()
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
    /// The path resolves like a read: a leading module path first, then a
    /// same-module or imported head, and the segments after the binding are
    /// field projections. The read carries the projected field's type, so a
    /// receiver that is a module-path binding is not mistaken for a callee. A
    /// path that names no binding has no read.
    fn global_projected_read(&mut self, module: &Module, segments: &[String]) -> Option<Value> {
        // A receiver path that names a loaded module is a module qualifier,
        // not a binding: a parent module's same-spelled export must not
        // become the receiver of the qualified call.
        if segments.len() >= 2 && self.loaded.has_module(&segments.join(".")) {
            return None;
        }
        let path = crate::ast::QualifiedName {
            segments: segments.to_vec(),
            span: crate::span::Span::default(),
        };
        let global = crate::init_graph::resolve_global(self.loaded, module, &path)?;
        let identity = GlobalId::new(global.module, global.name);
        let mut ty = self
            .globals
            .get(&identity)
            .map(|info| info.ty.clone())
            .unwrap_or_else(Ty::unknown);
        let mut projections: Vec<Projection> = Vec::new();
        for segment in &global.fields {
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

    /// The extent a fixed-array `len` operand folds to, when it folds.
    ///
    /// A fixed-array length is a structural property: the fold reads no
    /// runtime value, so the operand's stores never emit and its operands
    /// are never evaluated, exactly as the function path's structural fold
    /// drops them. A slice or an owned `Array` yields `None`, because their
    /// length is a runtime field. Both the dot spelling and the free `len`
    /// spelling fold through this rule.
    fn fixed_array_len_extent(&self, receiver: &Value) -> Option<usize> {
        let ty = self.resolved_value_ty(receiver);
        match &ty.kind {
            TyKind::Array { extents, .. } => extents.first().and_then(|size| *size),
            _ => None,
        }
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
            // The dot spelling takes the receiver; the module-qualified free
            // spelling takes its first argument.
            let dot = self.global_dot_callee(module, callee);
            let operand = if dot {
                self.global_dot_receiver(module, callee)?
            } else {
                let argument = args.first()?;
                self.lower_global(module, &argument.value)
            };
            // A fixed-array place folds to its extent before any
            // materialization: the fold reads no runtime value, so the
            // receiver's stores never emit and its operands are never
            // evaluated, exactly as the function path's structural fold drops
            // them. A slice or an owned `Array` keeps its operand, because
            // their length is a runtime field. A call or literal keeps its
            // evaluation because it may have effects.
            let foldable = dot
                || matches!(
                    args.first().map(|argument| &argument.value.kind),
                    Some(ExprKind::Name(_) | ExprKind::Qualified(_))
                );
            if foldable {
                if let Some(extent) = self.fixed_array_len_extent(&operand) {
                    return Some(Value::new(Ty::usize_ty(), ValueKind::Int(extent as i128)));
                }
            }
            return Some(Value::new(
                Ty::usize_ty(),
                ValueKind::Intrinsic {
                    name: "len".to_owned(),
                    args: vec![operand],
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
        let written = qualified_callee_name(callee);
        let name = written.rsplit('.').next().unwrap_or(&written);
        if !matches!(name, "len" | "rawSlice" | "rawStr" | "slice") {
            return None;
        }
        // A module path that ends at the final segment names that module's
        // declaration. Only the compiler intrinsic there lowers as the
        // built-in helper; a user or `ffi` declaration of the same simple
        // name keeps its own symbol. A path that ends earlier is a dot call on
        // a module binding, so the simple-name shadow checks below apply.
        let module_path = match &callee.kind {
            ExprKind::Qualified(target) if target.segments.len() >= 2 => {
                self.global_module_prefix(&target.segments)
                    .filter(|split| split + 1 == target.segments.len())
                    .map(|split| target.segments[..split].join("."))
            }
            _ => None,
        };
        if let Some(module_path) = module_path {
            let intrinsic = self
                .loaded
                .module(&module_path)
                .and_then(|target| target.decl(name))
                .is_some_and(|decl| matches!(decl.kind, crate::load::DeclKind::Intrinsic));
            if !intrinsic {
                return None;
            }
        } else if self.ffi_callee_signature(name).is_some()
            || self
                .functions
                .keys()
                .any(|(_, simple)| simple.as_str() == name)
        {
            return None;
        }
        if self.init_callable_value(module, callee).is_some() {
            return None;
        }
        Some(name.to_owned())
    }

    /// The length of the longest leading module path in a qualified name.
    ///
    /// The search runs from the longest candidate down so a nested module wins
    /// over its parent's prefix. A single-segment module counts as a path.
    fn global_module_prefix(&self, segments: &[String]) -> Option<usize> {
        (1..segments.len())
            .rev()
            .find(|split| self.loaded.has_module(&segments[..*split].join(".")))
    }

    /// True when a module-initializer callee is a dot call.
    ///
    /// The leading segments must name a module and the final segment a
    /// declaration in it for the callee to be a module-qualified free call.
    /// When the module prefix ends earlier, the next segment is a module
    /// binding and the rest are its members, so the callee is a dot call on
    /// that binding. A `Member` callee always has a receiver.
    fn global_dot_callee(&self, module: &Module, callee: &Expr) -> bool {
        match &callee.kind {
            ExprKind::Member { .. } => true,
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                if let Some(split) = self.global_module_prefix(&name.segments) {
                    return split + 1 < name.segments.len();
                }
                // No module path: a module-level binding head followed by its
                // members is a dot call on that binding.
                let receiver = crate::ast::QualifiedName {
                    segments: name.segments[..name.segments.len() - 1].to_vec(),
                    span: name.span,
                };
                crate::init_graph::resolve_global(self.loaded, module, &receiver).is_some()
            }
            _ => false,
        }
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

/// The callee name of a program-level call, keeping a module path whole.
///
/// A module-qualified free-function call names the loaded module's declaration,
/// so every segment is kept and a same-named function of the caller's module
/// never displaces it. A member callee carries its receiver separately, so it
/// keeps the simple member name.
fn qualified_callee_name(callee: &Expr) -> String {
    match &callee.kind {
        ExprKind::Qualified(name) if name.segments.len() >= 2 => name.segments.join("."),
        _ => callee_name(callee).unwrap_or_default(),
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
pub(super) fn init_operand_is_pure(value: &Value) -> bool {
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

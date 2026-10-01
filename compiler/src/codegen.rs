//! C code generation: the emitter for the single C99 translation unit.
//!
//! The emitter consumes the loaded program, the types the type stage computed,
//! and the generic instantiations the ownership stage collected, and writes one
//! C translation unit: a profile shim, the declaration surface, the generated
//! module initializers, the function bodies, and the `int main(void)` entry
//! stub. Every emission decision is deterministic: the same source, profile,
//! and flags produce byte-identical C.
//!
//! The module is a thin driver over focused submodules: [`types`] owns the C
//! type surface and naming, [`expr`] owns expressions and runtime guards, and
//! [`function`] owns statements, the cleanup ladder, and function bodies.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{EnumDecl, Expr, FuncDecl, Item, Mode, StructDecl, Type};
use crate::ir;
use crate::load::{Loaded, Module};
use crate::own::OwnershipOutput;
use crate::profile::Profile;
use crate::sema::SemaOutput;
use crate::span::{SourceMap, Span};
use crate::types::{resolve_ty, Ty, TyKind, TypeTable};

mod calls;
mod cleanup;
mod control;
mod drop;
mod expr;
mod function;
mod guards;
mod init;
mod program;
mod render;
mod types;

use cleanup::Action;
use types::{declarator, function_declarator, mangle_module};

/// Everything the emitter needs about one function with a body.
pub(super) struct FuncInfo<'a> {
    /// Dotted module the function is declared in.
    pub(super) module: String,
    /// The declaration, including the body.
    pub(super) decl: &'a FuncDecl,
}

/// Everything the emitter needs about one module-level binding.
pub(super) struct GlobalInfo {
    /// Dotted module the binding is declared in.
    pub(super) module: String,
    /// Binding name.
    pub(super) name: String,
    /// The checked type.
    pub(super) ty: Ty,
    /// True for a `volatile let`, which becomes a C `volatile` global.
    pub(super) volatile: bool,
}

/// A user enum with its declaration.
pub(super) struct EnumInfo<'a> {
    /// The declaration.
    pub(super) decl: &'a EnumDecl,
}

/// The translation-unit emitter.
pub(super) struct Emitter<'a> {
    /// The loaded program.
    loaded: &'a Loaded,
    /// The types the type stage computed.
    sema: &'a SemaOutput,
    /// The generic instantiations and lowered bodies the ownership stage built.
    ownership: &'a OwnershipOutput,
    /// The source map, for `#line` paths and line numbers.
    sources: &'a SourceMap,
    /// The selected profile.
    profile: Profile,
    /// The output buffer.
    out: String,
    /// User functions keyed by `(module, name)`.
    functions: BTreeMap<(String, String), FuncInfo<'a>>,
    /// `ffi` signatures keyed by simple name, for call-site ABI handling.
    ffi_functions: BTreeMap<String, &'a crate::ast::FuncSignature>,
    /// The module of the body currently being emitted, for call resolution.
    pub(super) current_module: String,
    /// Module-level bindings keyed by their module and simple name.
    globals: BTreeMap<ir::GlobalId, GlobalInfo>,
    /// User structs keyed by name.
    structs: BTreeMap<String, &'a StructDecl>,
    /// User enums keyed by name.
    enums: BTreeMap<String, EnumInfo<'a>>,
    /// Generic type-parameter substitutions active while a body is emitted.
    pub(super) subst: BTreeMap<String, Ty>,
    /// Unique C names already handed out in the function being emitted.
    pub(super) used_names: BTreeSet<String>,
    /// The IR body currently being emitted, when a function is open.
    pub(super) function: Option<&'a ir::Function>,
    /// C name assigned to each source local or parameter of the open function.
    pub(super) local_names: BTreeMap<String, String>,
    /// Parameters rendered as a pointer because they are a writable lend of a
    /// non-duplicable struct; a member access on one dereferences first.
    pub(super) pointer_params: BTreeSet<String>,
    /// Cleanup actions of every scope of the open function, keyed by scope id.
    ///
    /// Each entry pairs an action with its registration order so a scope's
    /// drops and `defer` entries can be run in one reverse registration order.
    pub(super) scope_actions: BTreeMap<usize, Vec<(u32, Action)>>,
    /// The owning scopes that take a function-end ladder rung, innermost first.
    pub(super) rungs: Vec<usize>,
    /// The cleanup label number assigned to each rung scope.
    pub(super) labels: BTreeMap<usize, usize>,
    /// Name of the result variable, set when the function has a ladder.
    pub(super) result_var: Option<String>,
    /// The open function's return type.
    pub(super) return_ty: Ty,
    /// Blocks reachable from the open function's entry.
    pub(super) reachable: BTreeSet<ir::BlockId>,
    /// Blocks already emitted during the current structured pass.
    pub(super) emitted: BTreeSet<ir::BlockId>,
    /// Blocks that need a `__cl_b<N>` label because a `goto` reaches them.
    ///
    /// The structured emitter plans these in a first pass that writes no output
    /// and then prints them in the real pass, so a forward `goto` finds a label
    /// already in place.
    pub(super) pending_labels: BTreeSet<ir::BlockId>,
    /// Entry block of each structured loop, mapped to the branch that describes
    /// it. The entry is the branch block for a `while`/`for` and the body block
    /// for a `do ... while`, whose test lives after the body.
    pub(super) loop_entry: BTreeMap<ir::BlockId, ir::BlockId>,
    /// True while a planning pass runs; it suppresses output but records the
    /// labels the real pass needs.
    pub(super) suppress_output: bool,
    /// The diagnostic sink, for the large-stack-frame warning.
    diagnostics: &'a mut crate::diag::Diagnostics,
}

/// Emits the whole translation unit for a successful compilation.
///
/// The emitter reports the large-stack-frame warning as it emits each function
/// body, so the driver renders it together with the front-end diagnostics.
pub fn emit(
    loaded: &Loaded,
    sema: &SemaOutput,
    ownership: &OwnershipOutput,
    profile: Profile,
    sources: &SourceMap,
    diagnostics: &mut crate::diag::Diagnostics,
) -> String {
    Emitter::new(loaded, sema, ownership, profile, sources, diagnostics).run()
}

impl<'a> Emitter<'a> {
    /// Collects the program facts the emitter reads repeatedly.
    fn new(
        loaded: &'a Loaded,
        sema: &'a SemaOutput,
        ownership: &'a OwnershipOutput,
        profile: Profile,
        sources: &'a SourceMap,
        diagnostics: &'a mut crate::diag::Diagnostics,
    ) -> Self {
        let mut emitter = Self {
            loaded,
            sema,
            ownership,
            sources,
            profile,
            out: String::new(),
            functions: BTreeMap::new(),
            ffi_functions: BTreeMap::new(),
            current_module: String::new(),
            globals: BTreeMap::new(),
            structs: BTreeMap::new(),
            enums: BTreeMap::new(),
            subst: BTreeMap::new(),
            used_names: BTreeSet::new(),
            function: None,
            local_names: BTreeMap::new(),
            pointer_params: BTreeSet::new(),
            scope_actions: BTreeMap::new(),
            rungs: Vec::new(),
            labels: BTreeMap::new(),
            result_var: None,
            return_ty: Ty::void(),
            reachable: BTreeSet::new(),
            emitted: BTreeSet::new(),
            pending_labels: BTreeSet::new(),
            loop_entry: BTreeMap::new(),
            suppress_output: false,
            diagnostics,
        };
        emitter.collect();
        emitter
    }

    /// Reports one warning through the shared diagnostic sink.
    pub(super) fn warn(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(crate::diag::Diagnostic::warning(code, message, span));
    }

    /// Builds the per-program lookup tables.
    fn collect(&mut self) {
        let modules: Vec<&'a Module> = self.loaded.modules.values().collect();
        for module in modules {
            for item in &module.items {
                match item {
                    Item::Func(decl) => {
                        self.functions
                            .entry((module.name.clone(), decl.name.clone()))
                            .or_insert(FuncInfo {
                                module: module.name.clone(),
                                decl,
                            });
                    }
                    Item::Struct(decl) => {
                        self.structs.entry(decl.name.clone()).or_insert(decl);
                    }
                    Item::Enum(decl) => {
                        self.enums.entry(decl.name.clone()).or_insert(EnumInfo { decl });
                    }
                    Item::GlobalLet(decl) => {
                        self.record_global(
                            module,
                            &decl.name,
                            Some(&decl.ty),
                            &decl.value,
                            decl.volatile,
                        );
                    }
                    Item::GlobalConst(decl) => {
                        self.record_global(module, &decl.name, Some(&decl.ty), &decl.value, false);
                    }
                    Item::Ffi(block) => {
                        for signature in &block.decls {
                            self.ffi_functions
                                .entry(signature.name.clone())
                                .or_insert(signature);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Records a module-level binding with its resolved type.
    fn record_global(
        &mut self,
        module: &Module,
        name: &str,
        declared: Option<&Type>,
        init: &'a Expr,
        volatile: bool,
    ) {
        let ty = declared
            .map(|ty| resolve_ty(ty, &[], &self.sema.table))
            .or_else(|| self.expr_type(init))
            .unwrap_or_else(Ty::unknown);
        self.globals
            .entry(ir::GlobalId::new(&module.name, name))
            .or_insert(GlobalInfo {
                module: module.name.clone(),
                name: name.to_owned(),
                ty,
                volatile,
            });
    }

    /// Runs the emission phases in order.
    fn run(mut self) -> String {
        self.report_generic_collisions();
        self.emit_header();
        self.out.push_str(self.profile.shim());
        self.emit_typedefs();
        self.emit_aggregate_declarations();
        self.emit_closure_declarations();
        self.emit_prototypes();
        self.emit_globals();
        self.emit_init_functions();
        self.emit_bodies();
        self.emit_entry_stub();
        self.out
    }

    // -- shared helpers ------------------------------------------------------

    /// Appends one line to the output.
    ///
    /// A planning pass suppresses output so the structured emitter can record
    /// the label set without writing a first copy of the body.
    pub(super) fn push(&mut self, line: &str) {
        if self.suppress_output {
            return;
        }
        self.out.push_str(line);
        self.out.push('\n');
    }

    /// The resolved type of an expression, with any active generic
    /// substitution applied.
    pub(super) fn expr_type(&self, expr: &Expr) -> Option<Ty> {
        self.sema
            .expr_types
            .get(&(expr as *const Expr as usize))
            .cloned()
            .map(|ty| self.substitute(&ty))
    }

    /// Applies the active generic substitution to a type.
    pub(super) fn substitute(&self, ty: &Ty) -> Ty {
        substitute_in(ty, &self.subst)
    }

    /// The empty-state initializer of a type under the selected profile.
    ///
    /// The hosted-amiga ABI declares `BPTR`, the file/lock handle type, as an
    /// integer, so a pointer constant would not convert to it. The integer
    /// constant `0` is the null value for both an integer and a pointer handle,
    /// so the hosted profile replaces the `NULL` empty state with it. The
    /// freestanding profile keeps the same spelling for consistency with the
    /// provisional ABI; every other type keeps its fixed empty state.
    pub(super) fn empty_state(&self, ty: &Ty) -> String {
        let base = types::empty_state(ty);
        if matches!(self.profile, Profile::HostedAmiga | Profile::Freestanding)
            && base == "NULL"
        {
            "0".to_owned()
        } else {
            base
        }
    }

    /// The null literal under the selected profile.
    ///
    /// See [`Self::empty_state`]: the hosted-amiga and freestanding ABIs use the
    /// integer constant `0` as the null spelling. The host profile keeps
    /// `NULL`, which the emitted C and its tests pin.
    pub(super) fn null_literal(&self) -> &'static str {
        if matches!(self.profile, Profile::HostedAmiga | Profile::Freestanding) {
            "0"
        } else {
            "NULL"
        }
    }

    /// The source line a span maps to.
    pub(super) fn line_of(&self, span: Span) -> u32 {
        self.sources.location(span.file, span.start as usize).line
    }

    /// The logical path a span belongs to.
    pub(super) fn path_of(&self, span: Span) -> &str {
        self.sources.path(span.file)
    }

    /// Writes a `#line` directive for a construct whose source is known.
    pub(super) fn emit_line_directive(&mut self, span: Span) {
        let line = self.line_of(span);
        let path = self.path_of(span).to_owned();
        self.push(&format!("#line {line} \"{path}\""));
    }

    /// The mangled symbol of a module-level binding.
    pub(super) fn global_symbol(&self, module: &str, name: &str) -> String {
        format!("cl_{}_{}", mangle_module(module), name)
    }

    /// The mangled symbol of a user function.
    pub(super) fn function_symbol(&self, module: &str, name: &str) -> String {
        format!("cl_{}_{}", mangle_module(module), name)
    }

    /// Picks a unique C name for a local declared with `name`.
    pub(super) fn unique_local(&mut self, name: &str) -> String {
        let base = sanitize_identifier(name);
        let mut candidate = base.clone();
        let mut counter = 1;
        while self.used_names.contains(&candidate) {
            candidate = format!("{base}_{counter}");
            counter += 1;
        }
        self.used_names.insert(candidate.clone());
        candidate
    }

    /// The signature of a non-generic function.
    fn function_signature(&self, module: &str, decl: &FuncDecl) -> String {
        let symbol = self.function_symbol(module, &decl.name);
        self.signature_with_symbol(decl, &BTreeMap::new(), &symbol)
    }

    /// Builds a full C function signature.
    fn signature_with_symbol(
        &self,
        decl: &FuncDecl,
        subst: &BTreeMap<String, Ty>,
        symbol: &str,
    ) -> String {
        let storage = if decl.exported { "" } else { "static " };
        let ret = resolve_ty(&decl.returns, &type_param_names(decl), &self.sema.table);
        let ret = substitute_in(&ret, subst);
        let mut params: Vec<String> = Vec::new();
        for param in &decl.params {
            let ty = resolve_ty(&param.ty, &type_param_names(decl), &self.sema.table);
            let ty = substitute_in(&ty, subst);
            let c_name = sanitize_identifier(&param.name);
            if is_pointer_param(&ty, param.mode, &self.sema.table) {
                params.push(declarator(&Ty::raw(ty), &c_name));
            } else {
                params.push(declarator(&ty, &c_name));
            }
        }
        // The shared declarator adds the hidden destination for a by-value
        // fixed-array return and nests a func-typed return in place.
        let signature = function_declarator(&ret, params, symbol, None);
        format!("{storage}{signature}")
    }

    /// Resolves a called function name against the emitting module.
    ///
    /// The caller's own module wins, so two modules with a same-named function
    /// each call their own. When the caller declares no such function, a name
    /// bound by one of the caller's imports resolves to that module's function,
    /// so an imported same-named function is not mistaken for ambiguous. Only
    /// then is a unique function of that name elsewhere used; an ambiguous name
    /// resolves to `None` rather than guessing.
    pub(super) fn resolve_function(&self, name: &str) -> Option<&FuncInfo<'a>> {
        if let Some(info) = self
            .functions
            .get(&(self.current_module.clone(), name.to_owned()))
        {
            return Some(info);
        }
        // The caller's import table disambiguates a name declared in more than
        // one module. The binding's local name may be an alias, so the lookup
        // uses the original imported name in the target module.
        if let Some(module) = self.loaded.module(&self.current_module) {
            if let Some(binding) = module.import(name) {
                if let Some(info) = self
                    .functions
                    .get(&(binding.module.clone(), binding.original.clone()))
                {
                    return Some(info);
                }
            }
        }
        let mut found: Option<&FuncInfo<'a>> = None;
        for ((_, simple), info) in &self.functions {
            if simple != name {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(info);
        }
        found
    }

    /// The first declared function with a simple name.
    ///
    /// A generic instance records only the simple name, so this lookup keeps
    /// the instance emission working; generic instances dedupe by name alone.
    pub(super) fn first_function_by_name(&self, name: &str) -> Option<&FuncInfo<'a>> {
        self.functions
            .iter()
            .find(|((_, simple), _)| simple == name)
            .map(|(_, info)| info)
    }

    /// The substitution for a generic instantiation.
    ///
    /// The instance carries the checked type arguments, so the substitution
    /// binds each declared parameter to its concrete type directly and a
    /// function-typed argument keeps its exact signature.
    fn instance_subst(&self, decl: &FuncDecl, args: &[Ty]) -> BTreeMap<String, Ty> {
        let mut subst = BTreeMap::new();
        for (param, arg) in decl.type_params.iter().zip(args) {
            subst.insert(param.name.clone(), arg.clone());
        }
        subst
    }

    /// The symbol key of a function instance: its type arguments under the
    /// canonical key scheme, joined with `_`.
    ///
    /// A call site and the instance prototype both derive the key here, so the
    /// call and its emitted copy always name the same symbol.
    fn instance_typekey(&self, args: &[Ty]) -> String {
        let keys: Vec<String> = args.iter().map(|ty| types::type_key(ty)).collect();
        keys.join("_")
    }

    // -- local type helpers --------------------------------------------------

    /// Resolves a written type in the current generic scope.
    pub(super) fn resolve_written(&self, ty: &Type) -> Ty {
        let params: Vec<String> = self.subst.keys().cloned().collect();
        let resolved = resolve_ty(ty, &params, &self.sema.table);
        self.substitute(&resolved)
    }
}

/// Applies a substitution to a type.
fn substitute_in(ty: &Ty, subst: &BTreeMap<String, Ty>) -> Ty {
    crate::types::substitute(ty, subst)
}

/// True when a parameter lowers to a C pointer rather than a value.
///
/// A writable lend of a non-duplicable struct must reach the caller's storage,
/// so it becomes a `T *`; a duplicable struct lend binds a copy and stays by
/// value, matching the unnecessary-lend warning. A handle already is a pointer
/// and a view is passed by value, so neither is affected.
fn is_pointer_param(ty: &Ty, mode: Mode, table: &TypeTable) -> bool {
    mode == Mode::Borrow && matches!(ty.kind, TyKind::Struct(..)) && !table.duplicable(ty)
}

/// The type-parameter names of a declaration.
fn type_param_names(decl: &FuncDecl) -> Vec<String> {
    decl.type_params.iter().map(|param| param.name.clone()).collect()
}

/// Turns a source name into a safe C identifier.
pub(super) fn sanitize_identifier(name: &str) -> String {
    let mut result = String::new();
    for (index, ch) in name.chars().enumerate() {
        let valid = ch == '_' || ch.is_ascii_alphanumeric();
        if !valid || (index == 0 && ch.is_ascii_digit()) {
            result.push('_');
        } else {
            result.push(ch);
        }
    }
    if result.is_empty() || is_c_keyword(&result) {
        return format!("cl_{result}");
    }
    result
}

/// True for a C keyword that may not be an identifier.
fn is_c_keyword(name: &str) -> bool {
    matches!(
        name,
        "auto" | "break" | "case" | "char" | "const" | "continue" | "default" | "do" | "double"
            | "else" | "enum" | "extern" | "float" | "for" | "goto" | "if" | "inline" | "int"
            | "long" | "register" | "return" | "short" | "signed" | "sizeof" | "static"
            | "struct" | "switch" | "typedef" | "union" | "unsigned" | "void" | "volatile"
            | "while" | "bool"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_sanitized_for_c() {
        assert_eq!(sanitize_identifier("value"), "value");
        assert_eq!(sanitize_identifier("_tmp"), "_tmp");
        assert_eq!(sanitize_identifier("2fast"), "_fast");
        assert_eq!(sanitize_identifier("a-b"), "a_b");
    }

    #[test]
    fn c_keywords_receive_a_stable_prefix() {
        assert_eq!(sanitize_identifier("int"), "cl_int");
        assert_eq!(sanitize_identifier("return"), "cl_return");
        assert_eq!(sanitize_identifier("while"), "cl_while");
    }

    #[test]
    fn an_empty_name_still_yields_a_valid_identifier() {
        assert_eq!(sanitize_identifier(""), "cl_");
    }
}

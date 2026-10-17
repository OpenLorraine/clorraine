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

use crate::ast::{EnumDecl, Expr, FuncDecl, Item, StructDecl, Type};
use crate::ir;
use crate::load::{Loaded, Module};
use crate::own::OwnershipOutput;
use crate::profile::Profile;
use crate::sema::SemaOutput;
use crate::span::{SourceMap, Span};
use crate::types::{resolve_ty, resolve_ty_folded, Ty, TyKind};

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
use types::{declarator, function_declarator, is_pointer_param, mangle_module, type_key};

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

/// One cleanup activation that is currently emitting.
///
/// The frame records which scope's cleanup is running and which action of the
/// scope's list is emitting, so a re-entry into the same scope from inside a
/// deferred copy runs only the actions older than the running one.
#[derive(Clone, Copy)]
pub(super) struct CleanupFrame {
    /// The scope whose cleanup is running.
    pub(super) scope: usize,
    /// Index of the action currently emitting in the scope's action list.
    pub(super) position: usize,
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
    /// `ffi` signatures keyed by their declaring module and name.
    ///
    /// The module is part of the key so a module-qualified call reaches that
    /// module's declaration; a simple name still resolves through the caller's
    /// module, its imports, or a unique declaration elsewhere.
    ffi_functions: BTreeMap<(String, String), &'a crate::ast::FuncSignature>,
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
    /// The enclosing instance key active while a body is emitted.
    ///
    /// A lifted lambda in a generic body is specialized per enclosing instance;
    /// while that instance's body is emitted this names the specialization so
    /// closure values, calls, and types spell the per-instance C names.
    pub(super) instance_key: Option<String>,
    /// Unique C names already handed out in the function being emitted.
    pub(super) used_names: BTreeSet<String>,
    /// The IR body currently being emitted, when a function is open.
    pub(super) function: Option<&'a ir::Function>,
    /// The per-instance type of every compiler temporary whose recorded type
    /// is still opaque, keyed by temporary id.
    ///
    /// A temporary created in a generic body for a value the declaration
    /// cannot type — the selected member of a conditional take, for example —
    /// carries the opaque type. The instance's substitution resolves it
    /// through the value that fills the temporary, so its declaration, its
    /// drop, and its move-outs use the instance's concrete shape.
    pub(super) resolved_temp_types: BTreeMap<ir::TempId, Ty>,
    /// C name assigned to each source local or parameter of the open function.
    pub(super) local_names: BTreeMap<String, String>,
    /// Init locals handed out in the module initializer being emitted.
    ///
    /// A module initializer stores operands it must not re-evaluate into
    /// `__cl_itmp<N>` locals; the counter resets per init function, so the
    /// numbering is stable per function.
    pub(super) init_locals: usize,
    /// The owning init locals of the current init function, in creation order.
    ///
    /// The release pass frees them in reverse order at the init function's
    /// end; a local an untaken path left zero drops nothing.
    pub(super) init_owners: Vec<(String, Ty)>,
    /// True while the current global's initializer feeds a view destination.
    ///
    /// A view keeps the storage it points into for the program's lifetime, so
    /// the owned values of such an initializer are retained rather than
    /// released; every other destination copies its bytes and releases its
    /// temporaries.
    pub(super) init_retain_owners: bool,
    /// True while an argument a `take` parameter consumes is prepared.
    ///
    /// The callee frees a consumed value, so nothing in the argument's own
    /// subtree records an owner; the values flow into the callee unreleased
    /// here, exactly as the base behaved before the release pass existed.
    pub(super) init_skip_owner_binding: bool,
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
    /// Rung labels an early edge (or a chain from one) actually reaches.
    ///
    /// A rung is emitted only when something jumps to it. An edge that ran its
    /// scopes inline leaves later rungs unreached, and a defined-but-unused
    /// label fails the strict build.
    pub(super) used_rung_labels: BTreeSet<usize>,
    /// True once an owned result store has been emitted inside a deferred copy.
    ///
    /// A deferred return leaves its value in the result variable; a later
    /// ordinary store that supersedes it (a deferred `break` or `continue`
    /// preempts the pending return and control reaches another `return`) must
    /// release the pending owner first. The flag limits that release to
    /// functions where such a store exists, so ordinary returns keep their
    /// unchanged shape.
    pub(super) deferred_result_store: bool,
    /// True when the open function has a deferred body that leaves its own
    /// block range.
    ///
    /// Such a body runs inline at each exit edge, so its `break` or
    /// `continue` can preempt a return anywhere in the function and abandon
    /// the stored result. Every ordinary store in the function then releases
    /// a pending value first; the release is null-safe, so a path that
    /// preempted nothing stays a no-op. A function with no such defer keeps
    /// the release to the deferred-store path alone.
    pub(super) out_edge_defer: bool,
    /// Name of the result variable, set when the function has a ladder.
    pub(super) result_var: Option<String>,
    /// The open function's return type.
    pub(super) return_ty: Ty,
    /// Blocks reachable from the open function's entry.
    pub(super) reachable: BTreeSet<ir::BlockId>,
    /// Blocks already emitted during the current structured pass.
    pub(super) emitted: BTreeSet<ir::BlockId>,
    /// Blocks that need a label because a `goto` reaches them, keyed by the
    /// label namespace that needs it.
    ///
    /// The structured emitter plans these in a first pass that writes no output
    /// and then prints them in the real pass, so a forward `goto` finds a label
    /// already in place. The namespace is the active deferred copy (zero for
    /// the main-body walk): a label needed inside a copy must not make the main
    /// walk print its own unused label, and vice versa.
    pub(super) pending_labels: BTreeSet<(ir::BlockId, u32)>,
    /// How many copies of each deferred block this function has emitted.
    ///
    /// A scope's cleanup activation emits its own copy of the scope's deferred
    /// blocks, so one deferred block can be copied several times into the
    /// function. The count gives each copy its own C label namespace: the
    /// first copy keeps the plain `__cl_b<N>` spelling and later copies suffix
    /// their labels, because C labels are function-scoped.
    pub(super) defer_copies: BTreeMap<ir::BlockId, u32>,
    /// The deferred roots whose copies are currently being emitted.
    ///
    /// A copy's own return cleanup can re-enter the running scope, so a nested
    /// activation of a root already emitting would recurse forever; it is a
    /// no-op. Activations of other roots emit normally.
    pub(super) active_defers: Vec<ir::BlockId>,
    /// The cleanup activations currently emitting, innermost last.
    ///
    /// A return or edge inside a deferred copy re-enters the running scope's
    /// cleanup; the frame records which action of the scope's list is emitting,
    /// so the re-entry runs only the actions older than it — the newer ones
    /// already ran.
    pub(super) cleanup_frames: Vec<CleanupFrame>,
    /// The label namespace of the innermost deferred copy being emitted.
    ///
    /// Zero means no copy is active (plain `__cl_b<N>` labels); a copy's
    /// ordinal makes labels unique per copy.
    pub(super) label_copy: u32,
    /// The block range of the innermost deferred copy being emitted.
    ///
    /// A copy's own labels are namespaced, but a `goto` that leaves the range
    /// targets a block the main walk owns and prints, so that target keeps the
    /// main walk's plain label spelling.
    pub(super) copy_region: Option<(ir::BlockId, ir::BlockId)>,
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
            instance_key: None,
            used_names: BTreeSet::new(),
            function: None,
            resolved_temp_types: BTreeMap::new(),
            local_names: BTreeMap::new(),
            init_locals: 0,
            init_owners: Vec::new(),
            init_retain_owners: false,
            init_skip_owner_binding: false,
            pointer_params: BTreeSet::new(),
            scope_actions: BTreeMap::new(),
            rungs: Vec::new(),
            labels: BTreeMap::new(),
            used_rung_labels: BTreeSet::new(),
            deferred_result_store: false,
            out_edge_defer: false,
            result_var: None,
            return_ty: Ty::void(),
            reachable: BTreeSet::new(),
            emitted: BTreeSet::new(),
            pending_labels: BTreeSet::new(),
            defer_copies: BTreeMap::new(),
            active_defers: Vec::new(),
            cleanup_frames: Vec::new(),
            label_copy: 0,
            copy_region: None,
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
                                .entry((module.name.clone(), signature.name.clone()))
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
            .map(|ty| resolve_ty_folded(ty, &[], &self.sema.table, module, self.loaded))
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
    ///
    /// Every emitted typedef precedes its uses: the fixed-width and handle
    /// typedefs come first, the enum typedefs and struct forward typedefs
    /// follow, the slice typedefs name element typedefs declared above, and
    /// the struct definitions, closure environments, prototypes, and bodies
    /// come after every type they name.
    fn run(mut self) -> String {
        self.report_generic_collisions();
        self.emit_header();
        self.out.push_str(self.profile.shim());
        self.emit_typedefs();
        self.emit_enum_typedefs();
        self.emit_struct_forward_typedefs();
        self.emit_slice_typedefs();
        self.emit_struct_definitions();
        self.emit_row_slice_typedefs();
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

    /// Applies the active generic substitution to a type and marks closure
    /// types with the active instance specialization.
    pub(super) fn substitute(&self, ty: &Ty) -> Ty {
        let substituted = substitute_in(ty, &self.subst);
        self.tag_closure_instance(substituted)
    }

    /// Marks every closure type in `ty` with the active instance spelling.
    ///
    /// A closure type created in a generic instance body names its specialized
    /// callable struct, so a local or temporary that holds it matches the
    /// environment and function pointer the creation site builds. A site with
    /// no specialization for the active instance is left alone, so shared
    /// lifted functions and non-generic closures keep their names.
    fn tag_closure_instance(&self, ty: Ty) -> Ty {
        let Some(key) = self.instance_key.as_deref() else {
            return ty;
        };
        self.tag_closure_instance_with(ty, key)
    }

    /// The recursive worker of [`Self::tag_closure_instance`].
    fn tag_closure_instance_with(&self, mut ty: Ty, key: &str) -> Ty {
        match &mut ty.kind {
            TyKind::Closure(closure) => {
                if closure.instance.is_none() && self.has_lifted_specialization(closure.id, key) {
                    closure.instance = Some(key.to_owned());
                }
            }
            TyKind::Raw(inner) => {
                **inner = self.tag_closure_instance_with((**inner).clone(), key);
            }
            TyKind::Array { element, .. } => {
                **element = self.tag_closure_instance_with((**element).clone(), key);
            }
            TyKind::Struct(_, args) => {
                for arg in args {
                    *arg = self.tag_closure_instance_with(arg.clone(), key);
                }
            }
            TyKind::Func(signature) => {
                for param in &mut signature.params {
                    param.ty = self.tag_closure_instance_with(param.ty.clone(), key);
                }
                signature.returns =
                    Box::new(self.tag_closure_instance_with((*signature.returns).clone(), key));
            }
            _ => {}
        }
        ty
    }

    /// True when the lifted site is emitted for the instance named `key`.
    pub(super) fn has_lifted_specialization(&self, site: usize, key: &str) -> bool {
        self.ownership
            .monomorphization
            .lifted_specializations()
            .iter()
            .any(|specialization| {
                specialization.site == site && self.instance_typekey(&specialization.args) == key
            })
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

    /// Reserves the C names the emitter generates for a function body.
    ///
    /// The generated spellings are fixed strings in the emitted C — the
    /// closure environment, the hoisted result, and the temporaries — so a
    /// source binding that shares one de-duplicates instead of colliding with
    /// it.
    pub(super) fn seed_generated_names(&mut self, temps: usize) {
        self.used_names.insert("__env".to_owned());
        self.used_names.insert("__cl_ret".to_owned());
        for index in 0..temps {
            self.used_names.insert(format!("__cl_tmp{index}"));
        }
    }

    /// The signature of a non-generic function.
    fn function_signature(&self, module: &str, decl: &FuncDecl) -> String {
        let symbol = self.function_symbol(module, &decl.name);
        self.signature_with_symbol(module, decl, &BTreeMap::new(), &symbol)
    }

    /// Builds a full C function signature.
    fn signature_with_symbol(
        &self,
        module: &str,
        decl: &FuncDecl,
        subst: &BTreeMap<String, Ty>,
        symbol: &str,
    ) -> String {
        let storage = if decl.exported { "" } else { "static " };
        let ret = self.resolve_in_module(module, &decl.returns, &type_param_names(decl));
        let ret = substitute_in(&ret, subst);
        let mut params: Vec<String> = Vec::new();
        let c_names = self.param_c_names(module, decl);
        for (index, param) in decl.params.iter().enumerate() {
            let ty = self.resolve_in_module(module, &param.ty, &type_param_names(decl));
            let ty = substitute_in(&ty, subst);
            let c_name = &c_names[index];
            if is_pointer_param(&ty, param.mode, &self.sema.table) {
                params.push(declarator(&Ty::raw(ty), c_name, &self.sema.table));
            } else {
                params.push(declarator(&ty, c_name, &self.sema.table));
            }
        }
        // The shared declarator adds the hidden destination for a by-value
        // fixed-array return and nests a func-typed return in place.
        let signature = function_declarator(&ret, params, symbol, None, &self.sema.table);
        format!("{storage}{signature}")
    }

    /// The C names of a declaration's parameters, in order.
    ///
    /// A parameter whose written name collides with a module callee carries a
    /// masked IR spelling, so the emitted signature must use the same spelling
    /// as the local-name table and the lowered callee names. The IR function
    /// is found by its declaration: its parameter list is the declaration's in
    /// order, and masking never changes the count. A declaration without an
    /// emitted body keeps its written names.
    fn param_c_names(&self, module: &str, decl: &FuncDecl) -> Vec<String> {
        let ir_function = self
            .ownership
            .functions
            .iter()
            .find(|function| function.module == module && function.name == decl.name);
        match ir_function {
            Some(function) => {
                let names = function.params.iter().map(|param| param.name.as_str());
                self.unique_param_names(function.temps.len(), names)
            }
            None => {
                self.unique_param_names(0, decl.params.iter().map(|param| param.name.as_str()))
            }
        }
    }

    /// The unique C names of a parameter list, in declaration order.
    ///
    /// Each name is sanitized and then de-duplicated against the names
    /// assigned before it, so a masked spelling (`give#0` becoming `give_0`)
    /// can never collide with a literally-named sibling parameter. The emitted
    /// signature, the local-name table, and the pointer-param keys resolve
    /// through this one pass, so they agree by construction.
    pub(super) fn unique_param_names<'n>(
        &self,
        temps: usize,
        names: impl Iterator<Item = &'n str>,
    ) -> Vec<String> {
        let mut used: BTreeSet<String> = BTreeSet::new();
        // The generated spellings are fixed strings in the emitted C — the
        // closure environment, the hoisted result, and the temporaries — so
        // they are reserved first: a parameter that shares one de-duplicates
        // instead of colliding with it. The reservation is part of this one
        // pure pass, so the signature and the body agree by construction.
        used.insert("__env".to_owned());
        used.insert("__cl_ret".to_owned());
        for index in 0..temps {
            used.insert(format!("__cl_tmp{index}"));
        }
        let mut resolved: Vec<String> = Vec::new();
        for name in names {
            let base = sanitize_identifier(name);
            let mut candidate = base.clone();
            let mut counter = 1;
            while used.contains(&candidate) {
                candidate = format!("{base}_{counter}");
                counter += 1;
            }
            used.insert(candidate.clone());
            resolved.push(candidate);
        }
        resolved
    }

    /// Resolves a called function name against the emitting module.
    ///
    /// A module-qualified name names its declaration exactly, with or without
    /// an import, and wins over a same-named function of the caller's module;
    /// a qualified name that reaches no function is a func value, an intrinsic,
    /// or a helper, so no simple-name fallback applies. Otherwise the caller's
    /// own module wins, so two modules with a same-named function each call
    /// their own. When the caller declares no such function, a name bound by
    /// one of the caller's imports resolves to that module's function, so an
    /// imported same-named function is not mistaken for ambiguous. Only then is
    /// a unique function of that name elsewhere used; an ambiguous name resolves
    /// to `None` rather than guessing.
    pub(super) fn resolve_function(&self, name: &str) -> Option<&FuncInfo<'a>> {
        if let Some((module, simple)) = name.rsplit_once('.') {
            return self
                .functions
                .get(&(module.to_owned(), simple.to_owned()));
        }
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

    /// Resolves an `ffi` declaration by its call name.
    ///
    /// A module-qualified name resolves in that module only. A simple name
    /// resolves the way a call does: the caller's module first, then an import
    /// binding, and only then a unique declaration elsewhere; an ambiguous
    /// name resolves to `None` rather than guessing another module's function.
    pub(super) fn ffi_callee_signature(&self, name: &str) -> Option<&'a crate::ast::FuncSignature> {
        if let Some((module, simple)) = name.rsplit_once('.') {
            return self
                .ffi_functions
                .get(&(module.to_owned(), simple.to_owned()))
                .copied();
        }
        if let Some(signature) = self
            .ffi_functions
            .get(&(self.current_module.clone(), name.to_owned()))
        {
            return Some(*signature);
        }
        if let Some(module) = self.loaded.module(&self.current_module) {
            if let Some(binding) = module.import(name) {
                if let Some(signature) = self
                    .ffi_functions
                    .get(&(binding.module.clone(), binding.original.clone()))
                {
                    return Some(*signature);
                }
            }
        }
        let mut found: Option<&'a crate::ast::FuncSignature> = None;
        for ((_, simple), signature) in &self.ffi_functions {
            if simple != name {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(*signature);
        }
        found
    }

    /// True when the caller's module declares the function, or one of its
    /// imports binds the name to a function.
    ///
    /// The scope rule the call resolution follows: a function bound in the
    /// caller's module or by one of its imports is the callee, while a
    /// function of the same simple name elsewhere is not in scope. A guard
    /// that decides between a function and a module-level value of the same
    /// name must not treat `resolve_function`'s unique-function-elsewhere arm
    /// as a binding. An `ffi` signature is in scope the same way.
    pub(super) fn function_in_scope(&self, name: &str) -> bool {
        if self
            .functions
            .contains_key(&(self.current_module.clone(), name.to_owned()))
        {
            return true;
        }
        if self.ffi_callee_signature(name).is_some() {
            return true;
        }
        self.loaded
            .module(&self.current_module)
            .and_then(|module| module.import(name))
            .is_some_and(|binding| {
                self.functions
                    .contains_key(&(binding.module.clone(), binding.original.clone()))
            })
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
        let params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        crate::types::instance_bindings(&params, args)
    }

    /// The symbol key of a function instance: its type arguments under the
    /// canonical key scheme, joined with `_`.
    ///
    /// A call site and the instance prototype both derive the key here, so the
    /// call and its emitted copy always name the same symbol.
    fn instance_typekey(&self, args: &[Ty]) -> String {
        let keys: Vec<String> = args.iter().map(type_key).collect();
        keys.join("_")
    }

    // -- local type helpers --------------------------------------------------

    /// Resolves a written type in the current generic scope.
    pub(super) fn resolve_written(&self, ty: &Type) -> Ty {
        let params: Vec<String> = self.subst.keys().cloned().collect();
        let resolved = self.resolve_in_module(&self.current_module, ty, &params);
        self.substitute(&resolved)
    }

    /// Resolves a written type in a named module, folding constant extents.
    ///
    /// The module supplies the constant environment a `const`-sized extent
    /// needs, so this matches the type stage's resolution; a name with no
    /// loaded module (a program that was already rejected) falls back to the
    /// literal-only resolver.
    pub(super) fn resolve_in_module(
        &self,
        module_name: &str,
        ty: &Type,
        type_params: &[String],
    ) -> Ty {
        match self.loaded.module(module_name) {
            Some(module) => {
                resolve_ty_folded(ty, type_params, &self.sema.table, module, self.loaded)
            }
            None => resolve_ty(ty, type_params, &self.sema.table),
        }
    }
}

/// Applies a substitution to a type.
fn substitute_in(ty: &Ty, subst: &BTreeMap<String, Ty>) -> Ty {
    crate::types::substitute(ty, subst)
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

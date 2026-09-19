//! Ownership, moves, initialization, and drop planning.
//!
//! This stage runs after the type stage on a program that already passed name
//! resolution and typing. It lowers each function body into the typed
//! control-flow IR of [`crate::ir`] while checking the ownership rules: a
//! local must be assigned on every path before it is read, a move-only value
//! transfers by move and invalidates its source, partial moves are limited to
//! top-level fields, a live loan blocks a move or a write of its owner, and a
//! view may not escape its origin. The checks use the types the type stage
//! recorded, so the two stages cannot disagree about the type of an
//! expression.
//!
//! The version 1 model is the conservative lexical approximation: a loan is
//! live from its creation until the end of the scope that owns the derived
//! view. It never accepts a program a more precise analysis would reject.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Arg, AssignOp, BinaryOp, Block, ConstStmt, ElseBranch, Expr, ExprKind, ForCStmt, ForInStmt,
    ForInit, IfStmt, Item, LetStmt, Mode, Param, Stmt, SwitchLabel, SwitchStmt, UnaryOp,
};
use crate::borrow::{
    is_elided_view, is_handle_or_aggregate, is_lend_candidate, is_nullable_capable, is_view,
    LoanKind, Provenance,
};
use crate::diag::{Diagnostic, Diagnostics};
use crate::ffi::{self, UnsafeOp};
use crate::ir::{self, BlockId, Builder, Place, Statement, Terminator, Value};
use crate::load::{Loaded, Module};
use crate::sema::SemaOutput;
use crate::span::Span;
use crate::types::{int_literal_value, resolve_ty, Ty, TyKind};

pub mod program;

mod expr;
mod flow;
mod generics;
mod place;
mod statements;

use flow::{Binding, BindingKind, DeferEntry, ExitFrame, FlowState, PlaceState, PlaceTarget, Scope};

#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;

use program::{Callee, ProgramInfo, StdlibEffect};

/// What the ownership stage hands to the emitter.
pub struct OwnershipOutput {
    /// The lowered control-flow graph of every function body.
    pub functions: Vec<ir::Function>,
    /// The generic instantiation set and copy obligations.
    pub monomorphization: ir::Monomorphization,
}

/// Checks the ownership, borrow, and FFI rules of every function body and
/// returns the lowered IR and the monomorphization inputs codegen consumes.
///
/// The stage is run only after a clean name and type pass, so it never
/// cascades a diagnostic from a program that already failed earlier.
pub fn check(
    loaded: &Loaded,
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) -> OwnershipOutput {
    let program = ProgramInfo::collect(loaded, &sema.table);
    ffi::check_items(&program, diagnostics);
    for module in loaded.modules.values() {
        check_globals(module, sema, diagnostics);
        check_struct_declarations(module, sema, diagnostics);
    }
    let mut functions = Vec::new();
    for module in loaded.modules.values() {
        for item in &module.items {
            if let Item::Func(decl) = item {
                let checker = FunctionChecker::new(&program, sema, decl, diagnostics);
                functions.push(checker.check());
            }
        }
    }
    OwnershipOutput {
        functions,
        monomorphization: generics::collect(loaded, &program, sema),
    }
}

/// Rejects a module-level `let` whose declared type needs a drop.
///
/// There are no drops at process exit, so a global must be duplicable; a
/// `const` is exempt because it cannot own a released resource.
fn check_globals(module: &Module, sema: &SemaOutput, diagnostics: &mut Diagnostics) {
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
fn check_struct_declarations(module: &Module, sema: &SemaOutput, diagnostics: &mut Diagnostics) {
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

/// Checks one function body and lowers it to the typed control-flow IR.
struct FunctionChecker<'a, 'd> {
    /// Program-wide ownership facts.
    program: &'a ProgramInfo<'a>,
    /// The types the type stage computed.
    sema: &'a SemaOutput,
    /// The function being checked.
    decl: &'a crate::ast::FuncDecl,
    /// Generic parameter names in scope.
    type_params: Vec<String>,
    /// The declared return type.
    returns: Ty,
    /// Lexical scopes from outermost to innermost.
    scopes: Vec<Scope>,
    /// Every binding declared so far.
    bindings: Vec<Binding>,
    /// The current flow state.
    state: FlowState,
    /// Number of enclosing `unsafe` constructs.
    unsafe_depth: u32,
    /// View-like lend parameters of the function, an elision candidate count.
    elision_candidates: usize,
    /// True when a returned view derives from static storage or a local, so
    /// the elision count should stay silent.
    suppress_elision: bool,
    /// The control-flow graph being built.
    builder: Builder,
    /// Next compiler-created temporary index. A cell lets the lowering hand
    /// out fresh indices through a shared reference.
    next_temp: std::cell::Cell<u32>,
    /// Open loops and switches, innermost last.
    exits: Vec<ExitFrame>,
    /// Deferred blocks registered by the open scopes, in registration order.
    defer_entries: Vec<DeferEntry<'a>>,
    /// False during a speculative pass, which computes state without reporting.
    report_diagnostics: bool,
    /// Diagnostic sink.
    diagnostics: &'d mut Diagnostics,
}

impl<'a, 'd> FunctionChecker<'a, 'd> {
    /// Creates a checker for one function.
    fn new(
        program: &'a ProgramInfo<'a>,
        sema: &'a SemaOutput,
        decl: &'a crate::ast::FuncDecl,
        diagnostics: &'d mut Diagnostics,
    ) -> Self {
        let type_params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        let returns = resolve_ty(&decl.returns, &type_params, &sema.table);
        let params: Vec<Place> = decl
            .params
            .iter()
            .map(|param| Place::param(&param.name))
            .collect();
        Self {
            program,
            sema,
            decl,
            type_params,
            returns,
            scopes: Vec::new(),
            bindings: Vec::new(),
            state: FlowState::default(),
            unsafe_depth: 0,
            elision_candidates: 0,
            suppress_elision: false,
            builder: Builder::new(&decl.name, params),
            next_temp: std::cell::Cell::new(0),
            exits: Vec::new(),
            defer_entries: Vec::new(),
            report_diagnostics: true,
            diagnostics,
        }
    }

    /// Runs the whole function check and finishes the IR.
    fn check(mut self) -> ir::Function {
        let returns = self.returns.clone();
        self.check_nullable_decl(&returns, self.decl.name_span);
        self.elision_prelude();
        self.push_scope();
        for param in &self.decl.params {
            self.declare_param(param);
        }
        self.check_block(&self.decl.body);
        self.pop_scope();
        self.elision_postlude();
        if !self.is_terminated() {
            self.builder.terminate(Terminator::Return(None));
        }
        self.builder.finish()
    }

    // -- scopes and bindings -------------------------------------------------

    /// Enters a new lexical scope.
    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    /// Leaves the innermost scope, running its defers and emitting its drops.
    ///
    /// The defers run before the scope is popped so the names the deferred
    /// block closes over are still visible to the ownership walk.
    fn pop_scope(&mut self) {
        let depth = self.scopes.len();
        if depth == 0 {
            return;
        }
        self.run_defers_from(depth);
        let Some(scope) = self.scopes.pop() else {
            return;
        };
        for id in scope.order.iter().rev() {
            if self.bindings[*id].needs_drop {
                let place = self.binding_place(*id);
                self.emit(Statement::Drop(place));
            }
        }
        for id in scope.order {
            self.state.places.remove(&id);
        }
        self.state.loans.end_scope(depth);
    }

    /// Runs every deferred block registered at or below `floor`, newest first.
    ///
    /// Each deferred block is checked against the state on the exit edge and
    /// then discarded, so its writes and moves belong to the edge and do not
    /// leak into the flow that follows. The check emits no IR: the deferred
    /// block already has its own lowerd block from registration.
    fn run_defers_from(&mut self, floor: usize) {
        while self
            .defer_entries
            .last()
            .is_some_and(|entry| entry.depth >= floor)
        {
            let entry = self.defer_entries.pop().expect("defer entry");
            self.check_defer_exit(entry.block);
        }
    }

    /// Checks one deferred block on an exit edge without emitting IR.
    fn check_defer_exit(&mut self, block: &'a Block) {
        let saved_state = self.state.clone();
        let saved_builder = self.builder.clone();
        let saved_bindings = self.bindings.len();
        let saved_scopes = self.scopes.len();
        let saved_defers = self.defer_entries.len();
        self.push_scope();
        for statement in &block.statements {
            self.check_stmt(statement);
        }
        self.pop_scope();
        self.state = saved_state;
        self.builder = saved_builder;
        self.bindings.truncate(saved_bindings);
        self.scopes.truncate(saved_scopes);
        self.defer_entries.truncate(saved_defers);
    }

    /// Repeats `run_body` in a speculative pass until the loop state stops
    /// changing, starting from `pre` joined with the body effect.
    ///
    /// The speculative pass emits into a copy of the graph and reports no
    /// diagnostics; the graph and the binding table are restored before the
    /// real pass that lowers the body. It returns the stable entry state, the
    /// state at the head of every iteration, and the stable state after the
    /// body, which is the exit state of a loop whose body always runs.
    fn loop_entry_state<F>(&mut self, pre: FlowState, run_body: F) -> (FlowState, FlowState)
    where
        F: Fn(&mut Self),
    {
        let saved_state = self.state.clone();
        let saved_builder = self.builder.clone();
        let saved_bindings = self.bindings.len();
        let saved_scopes = self.scopes.len();
        let saved_defers = self.defer_entries.len();
        let saved_exits = self.exits.len();
        let saved_report = self.report_diagnostics;
        self.report_diagnostics = false;
        let (entry, stable_post) = 'fixpoint: {
            let mut entry = pre.clone();
            loop {
                self.state = entry.clone();
                self.scopes.truncate(saved_scopes);
                self.bindings.truncate(saved_bindings);
                self.defer_entries.truncate(saved_defers);
                self.exits.truncate(saved_exits);
                self.builder = saved_builder.clone();
                run_body(self);
                let post = self.state.clone();
                let next = Self::merge(pre.clone(), post.clone());
                if next == entry {
                    break 'fixpoint (entry, post);
                }
                entry = next;
            }
        };
        self.state = saved_state;
        self.builder = saved_builder;
        self.bindings.truncate(saved_bindings);
        self.scopes.truncate(saved_scopes);
        self.defer_entries.truncate(saved_defers);
        self.exits.truncate(saved_exits);
        self.report_diagnostics = saved_report;
        (entry, stable_post)
    }

    /// Declares a local binding and returns its index.
    fn declare_local(&mut self, name: &str, ty: Ty, is_const: bool) -> usize {
        let needs_drop = self.owns_drop(&ty);
        self.declare(name, ty, BindingKind::Local, Mode::None, is_const, needs_drop)
    }

    /// Declares a function parameter.
    fn declare_param(&mut self, param: &Param) {
        let ty = self.resolve(&param.ty);
        self.check_nullable_decl(&ty, param.name_span);
        let out_slot = param.mode == Mode::Borrow && ty.nullable;
        let kind = if out_slot {
            BindingKind::OutSlot
        } else {
            BindingKind::Param
        };
        let needs_drop = param.mode == Mode::Take && self.owns_drop(&ty);
        let id = self.declare(
            &param.name,
            ty,
            kind,
            param.mode,
            false,
            needs_drop,
        );
        self.set_place(id, PlaceState {
            initialized: true,
            ..PlaceState::default()
        });
    }

    /// Inserts a binding into the innermost scope.
    fn declare(
        &mut self,
        name: &str,
        ty: Ty,
        kind: BindingKind,
        mode: Mode,
        is_const: bool,
        needs_drop: bool,
    ) -> usize {
        let id = self.bindings.len();
        let scope = self.scopes.len();
        let provenance = if kind == BindingKind::Param && is_lend_candidate(&ty) {
            Provenance::Param(id)
        } else {
            Provenance::None
        };
        self.bindings.push(Binding {
            name: name.to_owned(),
            ty,
            scope,
            kind,
            mode,
            is_const,
            needs_drop,
            provenance,
        });
        if let Some(scope) = self.scopes.last_mut() {
            scope.names.insert(name.to_owned(), id);
            scope.order.push(id);
        }
        id
    }

    /// Looks a name up from innermost scope outwards.
    fn lookup(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.names.get(name) {
                return Some(*id);
            }
        }
        None
    }

    /// True when a value of this type needs a drop.
    fn owns_drop(&self, ty: &Ty) -> bool {
        !self.sema.table.duplicable(ty)
            && !is_view(ty)
            && !ty.is_unknown()
            && !ty.is_param()
            && !ty.is_void()
    }

    /// The IR place for a binding.
    fn binding_place(&self, id: usize) -> Place {
        let binding = &self.bindings[id];
        match binding.kind {
            BindingKind::Param | BindingKind::OutSlot => Place::param(&binding.name),
            BindingKind::Local => Place::local(&binding.name),
        }
    }

    /// The IR place for a resolved target, keeping the binding's own name.
    fn target_place(&self, target: &PlaceTarget) -> Place {
        match target {
            PlaceTarget::Local(id, projections) => Place {
                base: self.binding_place(*id).base,
                projections: projections.clone(),
            },
            PlaceTarget::Global(name, projections) => Place {
                base: ir::PlaceBase::Global(name.clone()),
                projections: projections.clone(),
            },
        }
    }

    // -- flow state ----------------------------------------------------------

    /// Replaces the state of a place.
    fn set_place(&mut self, id: usize, state: PlaceState) {
        self.state.places.insert(id, state);
    }

    /// Mutates the state of a place in place.
    fn with_place(&mut self, id: usize, update: impl FnOnce(&mut PlaceState)) {
        let entry = self.state.places.entry(id).or_default();
        update(entry);
    }

    /// Merges two branch states conservatively.
    ///
    /// A move or partial move on either path survives; a value is initialized
    /// only when it is on both; narrowing survives only when both paths agree;
    /// a loan live on either path stays live.
    fn merge(first: FlowState, second: FlowState) -> FlowState {
        let mut places: BTreeMap<usize, PlaceState> = BTreeMap::new();
        let mut ids: BTreeSet<usize> = BTreeSet::new();
        ids.extend(first.places.keys().copied());
        ids.extend(second.places.keys().copied());
        for id in ids {
            let a = first.place(id);
            let b = second.place(id);
            let mut initialized_fields = BTreeSet::new();
            for field in a.initialized_fields.iter().chain(b.initialized_fields.iter()) {
                if a.field_initialized(field) && b.field_initialized(field) {
                    initialized_fields.insert(field.clone());
                }
            }
            places.insert(
                id,
                PlaceState {
                    moved: a.moved || b.moved,
                    partial: a.partial.union(&b.partial).cloned().collect(),
                    initialized: a.initialized && b.initialized,
                    initialized_fields,
                    narrowed: a.narrowed.intersection(&b.narrowed).cloned().collect(),
                },
            );
        }
        let mut loans = first.loans;
        loans.merge(&second.loans);
        FlowState { places, loans }
    }

    // -- IR emission ---------------------------------------------------------

    /// Emits a statement into the current block.
    fn emit(&mut self, statement: Statement) {
        self.builder.emit(statement);
    }

    /// Starts a fresh IR block.
    fn new_block(&mut self) -> BlockId {
        self.builder.new_block()
    }

    /// Makes a block the current one.
    fn switch_to(&mut self, block: BlockId) {
        self.builder.switch_to(block);
    }

    /// Ends the current block with a terminator.
    fn terminate(&mut self, terminator: Terminator) {
        self.builder.terminate(terminator);
    }

    /// True when the current block already has a real exit.
    fn is_terminated(&self) -> bool {
        !matches!(
            self.builder.function().blocks[self.builder.current()].terminator,
            Terminator::Unreachable
        )
    }

    /// Emits the drops of every open scope, innermost first, then returns.
    ///
    /// Deferred blocks run first, in reverse registration order, followed by
    /// the drops. The deferred effects are checked but do not change the
    /// return value, which was already evaluated.
    fn emit_return_cleanup(&mut self) {
        self.run_defers_from(1);
        let depths: Vec<usize> = (1..=self.scopes.len()).rev().collect();
        for depth in depths {
            let ids: Vec<usize> = self
                .scopes
                .get(depth - 1)
                .map(|scope| scope.order.iter().rev().copied().collect())
                .unwrap_or_default();
            for id in ids {
                if self.bindings[id].needs_drop {
                    let place = self.binding_place(id);
                    self.emit(Statement::Drop(place));
                }
            }
        }
    }
}

impl PlaceTarget {
    /// Replaces the projections of a target.
    fn with_projections(self, projections: Vec<ir::Projection>) -> Self {
        match self {
            Self::Local(id, _) => Self::Local(id, projections),
            Self::Global(name, _) => Self::Global(name, projections),
        }
    }
}

/// The exclusive/shared class a call-site or declaration marker denotes.
///
/// An unmarked parameter and an explicit `const` both grant read-only access,
/// so they are treated as the same assertion.
fn marker_kind(mode: Mode) -> Mode {
    match mode {
        Mode::None | Mode::Const => Mode::Const,
        other => other,
    }
}

/// The constant index of an element expression, when it is a literal.
fn element_index(expr: &Expr) -> Option<usize> {
    match &expr.kind {
        ExprKind::Int(value) => usize::try_from(*value).ok(),
        _ => None,
    }
}

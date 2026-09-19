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
    LoanKind, Loans, Provenance,
};
use crate::diag::{Diagnostic, Diagnostics};
use crate::ffi::{self, UnsafeOp};
use crate::ir::{self, BlockId, Builder, Place, Statement, Terminator, Value};
use crate::load::{Loaded, Module};
use crate::sema::SemaOutput;
use crate::span::Span;
use crate::types::{resolve_ty, Ty, TyKind};

pub mod program;

mod expr;
mod generics;
mod place;
mod statements;

use program::{Callee, ProgramInfo, StdlibEffect};

/// Checks the ownership, borrow, and FFI rules of every function body and
/// returns the monomorphization inputs codegen consumes.
///
/// The stage is run only after a clean name and type pass, so it never
/// cascades a diagnostic from a program that already failed earlier.
pub fn check(
    loaded: &Loaded,
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) -> ir::Monomorphization {
    let program = ProgramInfo::collect(loaded, &sema.table);
    ffi::check_items(&program, diagnostics);
    for module in loaded.modules.values() {
        check_globals(module, sema, diagnostics);
        check_struct_declarations(module, sema, diagnostics);
    }
    for module in loaded.modules.values() {
        for item in &module.items {
            if let Item::Func(decl) = item {
                let mut checker = FunctionChecker::new(&program, sema, decl, diagnostics);
                checker.check();
            }
        }
    }
    generics::collect(loaded, &program, sema)
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

/// What a binding is: a local, a parameter, or a `borrow T?` out-parameter.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BindingKind {
    /// A local `let` or `const`.
    Local,
    /// A parameter that lends a value.
    Param,
    /// A `borrow T?` out-parameter slot, which owns its incoming value.
    OutSlot,
}

/// One declared binding of the function currently being checked.
struct Binding {
    /// The written name.
    name: String,
    /// The resolved type.
    ty: Ty,
    /// The scope depth the binding was declared in.
    scope: usize,
    /// What kind of binding it is.
    kind: BindingKind,
    /// The written parameter mode, or no mode for a local.
    mode: Mode,
    /// True for a `const` local.
    is_const: bool,
    /// True when the binding owns a value that needs a drop.
    needs_drop: bool,
    /// For a view binding, where its storage comes from.
    provenance: Provenance,
}

/// One lexical scope and the bindings declared in it.
#[derive(Default)]
struct Scope {
    /// Names bound here, innermost wins.
    names: BTreeMap<String, usize>,
    /// Binding indices in declaration order, for reverse drop emission.
    order: Vec<usize>,
}

/// The flow-sensitive state of one place.
#[derive(Clone, Default)]
struct PlaceState {
    /// The value was moved out and not re-initialized.
    moved: bool,
    /// Top-level fields moved out, which makes whole use invalid.
    partial: BTreeSet<String>,
    /// The binding is assigned on all paths reaching this point.
    initialized: bool,
    /// A dominating null test proved the place is not null.
    narrowed: bool,
}

/// The flow state of one function at one program point.
#[derive(Clone, Default)]
struct FlowState {
    /// Per-binding place state.
    places: BTreeMap<usize, PlaceState>,
    /// Loans live at this point.
    loans: Loans,
}

impl FlowState {
    /// Reads the state of a place, defaulting to "uninitialized and unmoved".
    fn place(&self, id: usize) -> PlaceState {
        self.places.get(&id).cloned().unwrap_or_default()
    }
}

/// The target of an assignment or a move.
enum PlaceTarget {
    /// A local or parameter with its projections.
    Local(usize, Vec<ir::Projection>),
    /// A module-level binding with its projections.
    Global(String, Vec<ir::Projection>),
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
            diagnostics,
        }
    }

    /// Runs the whole function check and finishes the IR.
    fn check(&mut self) {
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
    }

    // -- scopes and bindings -------------------------------------------------

    /// Enters a new lexical scope.
    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    /// Leaves the innermost scope, emitting the drops it owns.
    fn pop_scope(&mut self) {
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
        self.state.loans.end_scope(self.scopes.len() + 1);
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
    fn merge(then_state: FlowState, else_state: FlowState) -> FlowState {
        let mut places: BTreeMap<usize, PlaceState> = BTreeMap::new();
        let mut ids: BTreeSet<usize> = BTreeSet::new();
        ids.extend(then_state.places.keys().copied());
        ids.extend(else_state.places.keys().copied());
        for id in ids {
            let a = then_state.place(id);
            let b = else_state.place(id);
            places.insert(
                id,
                PlaceState {
                    moved: a.moved || b.moved,
                    partial: a.partial.union(&b.partial).cloned().collect(),
                    initialized: a.initialized && b.initialized,
                    narrowed: a.narrowed && b.narrowed,
                },
            );
        }
        let mut loans = then_state.loans;
        loans.merge(&else_state.loans);
        FlowState { places, loans }
    }

    /// Merges a loop body state back into the pre-loop state, keeping the
    /// assignments that hold on every path.
    fn merge_loop(pre: FlowState, body: FlowState) -> FlowState {
        let mut places: BTreeMap<usize, PlaceState> = BTreeMap::new();
        let mut ids: BTreeSet<usize> = BTreeSet::new();
        ids.extend(pre.places.keys().copied());
        ids.extend(body.places.keys().copied());
        for id in ids {
            let a = pre.place(id);
            let b = body.place(id);
            places.insert(
                id,
                PlaceState {
                    moved: a.moved || b.moved,
                    partial: a.partial.union(&b.partial).cloned().collect(),
                    initialized: a.initialized && b.initialized,
                    narrowed: a.narrowed,
                },
            );
        }
        let mut loans = pre.loans;
        loans.merge(&body.loans);
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
    fn emit_return_cleanup(&mut self) {
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

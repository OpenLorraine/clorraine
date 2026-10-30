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
//! A loan is live from its creation until its last use on any path: the walk
//! records the loan events and the liveness pass computes each loan's region
//! over the emitted graph, so a view that is never used again blocks nothing.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Arg, AssignOp, BinaryOp, Block, ConstStmt, ElseBranch, Expr, ExprKind, ForCStmt, ForInStmt,
    ForInit, FuncDecl, IfStmt, IntrinsicArg, Item, LambdaBody, LambdaParam, LetStmt, Mode, Param,
    ScalarType, Stmt, SwitchLabel, SwitchStmt, TemplatePiece, Type, TypeKind, UnaryOp,
};
use crate::borrow::{
    is_elided_view, is_handle_or_aggregate, is_lend_candidate, is_nullable_capable, is_view,
    is_view_like, is_whole_slot_view, LoanKind, Provenance,
};
use crate::diag::{Diagnostic, Diagnostics};
use crate::ffi::{self, UnsafeOp};
use crate::ir::{
    self, BlockId, BranchShape, Builder, LoopKind, Place, Release, Statement, Terminator, Value,
    ValueKind, ValuePiece,
};
use crate::load::Loaded;
use crate::sema::SemaOutput;
use crate::span::Span;
use crate::types::{resolve_ty_folded, view_type, CaptureInfo, CaptureMode, Ty, TyKind};

pub mod program;

mod call;
mod call_check;
mod expr;
mod flow;
mod generics;
pub(crate) use generics::lifted_needs_specialization;
mod instance_check;
mod liveness;
mod lower;
mod loops;
mod place;
mod statements;

use flow::{
    Binding, BindingKind, DeferEntry, ExitFrame, FlowState, LiteralViews, PlaceState, PlaceTarget,
    Scope,
};
use liveness::{AccessKind, LoanEvent, LoanEventKind, LoanId, LoanOrigin, LoanOwner, Position};

#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod scope_tests;
#[cfg(test)]
mod tests;

use program::{Callee, ProgramInfo, StdlibEffect};

/// Remedy shared by every rejected lend store.
///
/// The `borrow T?` exception covers only the slot's own incoming value. A
/// different lend's value or view is still rejected, while a static view and
/// a newly owned value remain legal.
pub(crate) const LEND_STORE_HELP: &str = "declare the parameter `take` or by value; a `borrow T?` slot may hold its own incoming value, a static view, or a newly owned value";

/// What the ownership stage hands to the emitter.
pub struct OwnershipOutput {
    /// The lowered control-flow graph of every function body.
    pub functions: Vec<ir::Function>,
    /// The generic instantiation set and copy obligations.
    pub monomorphization: ir::Monomorphization,
    /// The lifted site of each module-initializer lambda, keyed by the lambda
    /// node's address.
    ///
    /// A module initializer has no enclosing body, so its capture-less lambdas
    /// are lifted by the synthetic context in [`check`]; the emitter resolves
    /// the initializer's callable through this map.
    pub init_lambdas: BTreeMap<usize, usize>,
}

/// The declaration identity a take-view requirement and its call records share.
fn function_identity(loaded: &Loaded, decl: &FuncDecl) -> String {
    for module in loaded.modules.values() {
        for item in &module.items {
            if let Item::Func(candidate) = item {
                if std::ptr::eq(candidate, decl) {
                    return format!("{}.{}", module.name, candidate.name);
                }
            }
        }
    }
    decl.name.clone()
}

/// A take parameter's view store that needs a call-site judgment.
#[derive(Clone, Debug)]
struct TakeViewRequirement {
    function: String,
    parameter: String,
    parameter_index: usize,
    path: Vec<ir::Projection>,
    span: Span,
}

/// One user-call argument seen while checking a caller body.
#[derive(Clone, Debug)]
struct TakeCallArgument {
    function: String,
    parameter_index: usize,
    arg_ty: Ty,
    safe: bool,
}

/// Marks user functions used as values in module initializers.
///
/// The initializer walkers do not evaluate a function name as a call, so a
/// global `func` value would otherwise be invisible to the fail-closed check.
/// A call's own callee is skipped: only a function used as a value makes the
/// call form unknown.
fn collect_global_value_uses(
    loaded: &Loaded,
    program: &ProgramInfo<'_>,
    out: &mut BTreeSet<String>,
) {
    for module in loaded.modules.values() {
        for item in &module.items {
            match item {
                Item::GlobalLet(decl) => {
                    collect_expr_value_uses(&decl.value, module, loaded, program, out)
                }
                Item::GlobalConst(decl) => {
                    collect_expr_value_uses(&decl.value, module, loaded, program, out)
                }
                _ => {}
            }
        }
    }
}

fn collect_expr_value_uses(
    expr: &Expr,
    module: &crate::load::Module,
    loaded: &Loaded,
    program: &ProgramInfo<'_>,
    out: &mut BTreeSet<String>,
) {
    match &expr.kind {
        ExprKind::Name(name) => {
            if let Some(decl) = resolve_initializer_function(module, name, loaded, program) {
                out.insert(function_identity(loaded, decl));
            }
        }
        ExprKind::Qualified(name) => {
            let written = name.segments.join(".");
            if let Some(decl) = program.user_function(&written) {
                out.insert(function_identity(loaded, decl));
            }
        }
        ExprKind::Call { callee, args, .. } => {
            if !matches!(callee.kind, ExprKind::Name(_) | ExprKind::Qualified(_)) {
                collect_expr_value_uses(callee, module, loaded, program, out);
            }
            for arg in args {
                collect_expr_value_uses(&arg.value, module, loaded, program, out);
            }
        }
        ExprKind::Unary { operand, .. }
        | ExprKind::Cast { value: operand, .. }
        | ExprKind::RawCast { value: operand, .. }
        | ExprKind::Unsafe(operand) => {
            collect_expr_value_uses(operand, module, loaded, program, out)
        }
        ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
            collect_expr_value_uses(base, module, loaded, program, out);
            if let ExprKind::Index { index, .. } = &expr.kind {
                collect_expr_value_uses(index, module, loaded, program, out);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_value_uses(left, module, loaded, program, out);
            collect_expr_value_uses(right, module, loaded, program, out);
        }
        ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } => {
            collect_expr_value_uses(then_value, module, loaded, program, out);
            collect_expr_value_uses(else_value, module, loaded, program, out);
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_expr_value_uses(&field.value, module, loaded, program, out);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                collect_expr_value_uses(element, module, loaded, program, out);
            }
            if let Some(fill) = fill {
                collect_expr_value_uses(fill, module, loaded, program, out);
            }
        }
        _ => {}
    }
}

/// Resolves a function name in a module initializer's own scope.
///
/// A module-local declaration wins, then an import binding, then the
/// program-wide fallback. This matches the call checker's resolution order and
/// keeps a same-named function in another module from stealing the value-use
/// record.
fn resolve_initializer_function<'a>(
    module: &'a crate::load::Module,
    name: &str,
    loaded: &'a Loaded,
    program: &ProgramInfo<'a>,
) -> Option<&'a FuncDecl> {
    if let Some(decl) = module.decl(name) {
        if decl.kind == crate::load::DeclKind::Func {
            return module.items.iter().find_map(|item| match item {
                Item::Func(function) if function.name == name => Some(function),
                _ => None,
            });
        }
        return None;
    }
    if let Some(binding) = module.import(name) {
        if binding.poisoned {
            return None;
        }
        return loaded
            .module(&binding.module)
            .and_then(|target| target.items.iter().find_map(|item| match item {
                Item::Func(function) if function.name == binding.original => Some(function),
                _ => None,
            }));
    }
    program.user_function(name)
}

/// True when the named path on an argument is a view leaf that can outlive a
/// call. The path resolves fields and elements from the argument type; a
/// `String` leaf is owned storage, so it does not qualify.
fn take_path_is_view(
    table: &crate::types::TypeTable,
    arg_ty: &Ty,
    path: &[ir::Projection],
) -> bool {
    let mut current = arg_ty.clone();
    for step in path {
        current = match step {
            ir::Projection::Field(name) => match table.instance_field(&current, name) {
                Some(field) => field,
                None => return false,
            },
            ir::Projection::Element(_) | ir::Projection::ElementTemp(_) => {
                current.iterated_element()
            }
        };
    }
    is_view_like(&current)
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
        program::check_globals(module, loaded, &program, sema, diagnostics);
        program::check_struct_declarations(module, loaded, sema, diagnostics);
    }
    let mut functions = Vec::new();
    let mut duplicable_requirements = Vec::new();
    let mut element_move_requirements = Vec::new();
    let mut not_lend_requirements = Vec::new();
    let mut whole_out_slot_duties = Vec::new();
    let mut iterable_duties = Vec::new();
    let mut out_slot_duties = Vec::new();
    let mut storable_duties = Vec::new();
    let mut self_assign_duties = Vec::new();
    let mut opaque_store_duties = Vec::new();
    let mut view_return_duties = Vec::new();
    let mut conditional_narrowing_duties = Vec::new();
    let mut len_operand_duties = Vec::new();
    let mut take_view_requirements: Vec<TakeViewRequirement> = Vec::new();
    let mut take_call_arguments: Vec<TakeCallArgument> = Vec::new();
    let mut function_value_uses: BTreeSet<String> = BTreeSet::new();
    // Capture-less lambda sites have no recorded semantic site, so the stage
    // hands out ids above every recorded capturing site. The prefix keeps the
    // lifted function names of the two kinds disjoint.
    let mut next_lifted = sema
        .closures
        .keys()
        .max()
        .map_or(0usize, |highest| highest + 1);
    for module in loaded.modules.values() {
        for item in &module.items {
            if let Item::Func(decl) = item {
                let checker = FunctionChecker::new(
                    &program,
                    loaded,
                    module,
                    sema,
                    decl,
                    diagnostics,
                    &mut next_lifted,
                );
                let (
                    mut function,
                    lifted,
                    requirements,
                    element_moves,
                    lend_duties,
                    whole_out_slot_reqs,
                    iterable_reqs,
                    slot_duties,
                    store_duties,
                    self_assign_reqs,
                    opaque_store_reqs,
                    view_returns,
                    conditional_narrowing,
                    len_operands,
                    take_requirements,
                    take_calls,
                    value_uses,
                ) = checker.check();
                function.module = module.name.clone();
                functions.push(function);
                functions.extend(lifted);
                duplicable_requirements.extend(requirements);
                element_move_requirements.extend(element_moves);
                not_lend_requirements.extend(lend_duties);
                whole_out_slot_duties.extend(whole_out_slot_reqs);
                iterable_duties.extend(iterable_reqs);
                out_slot_duties.extend(slot_duties);
                storable_duties.extend(store_duties);
                self_assign_duties.extend(self_assign_reqs);
                opaque_store_duties.extend(opaque_store_reqs);
                view_return_duties.extend(view_returns);
                conditional_narrowing_duties.extend(conditional_narrowing);
                len_operand_duties.extend(len_operands);
                take_view_requirements.extend(take_requirements);
                take_call_arguments.extend(take_calls);
                function_value_uses.extend(value_uses);
            }
        }
    }
    collect_global_value_uses(loaded, &program, &mut function_value_uses);
    // A take parameter's view store is safe only when every call to the
    // declaration passes a proven-static carrier for the stored path. A
    // function used as a value has call forms this pass cannot see, so it
    // fails closed. No call site at all keeps the callee rejection.
    for requirement in take_view_requirements {
        let calls: Vec<&TakeCallArgument> = take_call_arguments
            .iter()
            .filter(|call| {
                call.function == requirement.function
                    && call.parameter_index == requirement.parameter_index
            })
            .collect();
        let safe = !requirement.path.is_empty()
            && !calls.is_empty()
            && !function_value_uses.contains(&requirement.function)
            && calls.iter().all(|call| call.safe)
            && calls.iter().all(|call| {
                take_path_is_view(&sema.table, &call.arg_ty, &requirement.path)
            });
        if !safe {
            diagnostics.push(
                Diagnostic::error(
                    "E10030",
                    format!(
                        "borrow of '{}' does not live long enough",
                        requirement.parameter
                    ),
                    requirement.span,
                )
                .with_help("finish the use before the owner's scope ends"),
            );
        }
    }

    // A module initializer has no enclosing body, so the capture-less lambdas
    // it spells are lifted here through a synthetic function context; the
    // emitter resolves each one by the lambda node's address.
    let mut init_lambdas: BTreeMap<usize, usize> = BTreeMap::new();
    for module in loaded.modules.values() {
        let values: Vec<&Expr> = module
            .items
            .iter()
            .filter_map(|item| match item {
                Item::GlobalLet(decl) => Some(&decl.value),
                Item::GlobalConst(decl) => Some(&decl.value),
                _ => None,
            })
            .collect();
        if values.is_empty() {
            continue;
        }
        let decl = init_decl(module);
        let mut checker = FunctionChecker::new(
            &program,
            loaded,
            module,
            sema,
            &decl,
            diagnostics,
            &mut next_lifted,
        );
        checker.check_initializer_writes(&values);
        for item in &module.items {
            match item {
                Item::GlobalLet(decl) => {
                    let ty = sema
                        .table
                        .global(&module.name, &decl.name)
                        .cloned()
                        .unwrap_or_else(Ty::unknown);
                    checker.check_initializer_reads(&decl.value);
                    checker.check_initializer_nulls(&ty, &decl.value);
                }
                Item::GlobalConst(decl) => {
                    let ty = sema
                        .table
                        .global(&module.name, &decl.name)
                        .cloned()
                        .unwrap_or_else(Ty::unknown);
                    checker.check_initializer_reads(&decl.value);
                    checker.check_initializer_nulls(&ty, &decl.value);
                }
                _ => {}
            }
        }
        let (sites, lifted) = checker.lift_initializer_lambdas(&values);
        init_lambdas.extend(sites);
        functions.extend(lifted);
    }
    let mut monomorphization = generics::collect(loaded, &program, sema);
    for requirement in duplicable_requirements {
        monomorphization.require_duplicable(requirement);
    }
    for requirement in element_move_requirements {
        monomorphization.require_element_move(requirement);
    }
    for requirement in not_lend_requirements {
        monomorphization.require_not_lend(requirement);
    }
    for duty in whole_out_slot_duties {
        monomorphization.require_whole_out_slot_store(duty);
    }
    for duty in iterable_duties {
        monomorphization.require_iterable(duty);
    }
    for duty in out_slot_duties {
        monomorphization.require_out_slot(duty);
    }
    for duty in storable_duties {
        monomorphization.require_storable_slot(duty);
    }
    for duty in self_assign_duties {
        monomorphization.require_self_assign(duty);
    }
    for duty in opaque_store_duties {
        monomorphization.require_opaque_store(duty);
    }
    for duty in view_return_duties {
        monomorphization.require_view_return(duty);
    }
    for duty in conditional_narrowing_duties {
        monomorphization.require_conditional_narrowing(duty);
    }
    for duty in len_operand_duties {
        monomorphization.require_len_operand(duty);
    }
    generics::collect_ir_instances(&functions, &program, sema, &mut monomorphization);
    // The IR walk records struct instances a declaration-only walk cannot
    // see, such as a `sizeof(T[])` type argument at a concrete function
    // instance. Expand their fields now so a nested `Pair<T>` inside
    // `Box<i32>` is recorded before the emitter writes the unit.
    generics::collect_struct_instances(loaded, sema, &mut monomorphization);
    instance_check::check(loaded, &program, &functions, sema, &monomorphization, diagnostics);
    OwnershipOutput {
        functions,
        monomorphization,
        init_lambdas,
    }
}

/// A synthetic declaration naming one module's initializer context.
///
/// The checker machinery names its generated artifacts after the enclosing
/// declaration; a module initializer has none, so the initializer-lambda lift
/// runs under this placeholder. Its body is never lowered.
fn init_decl(module: &crate::load::Module) -> FuncDecl {
    FuncDecl {
        exported: false,
        name: format!("__cl_init_{}", module.name),
        name_span: Span::default(),
        type_params: Vec::new(),
        params: Vec::new(),
        returns: Type {
            kind: TypeKind::Void,
            nullable: false,
            span: Span::default(),
        },
        body: Block {
            statements: Vec::new(),
            span: Span::default(),
        },
        span: Span::default(),
    }
}

/// True when a type may own once a type argument is substituted.
///
/// A type that mentions a type parameter is maybe-owning unless it is a view,
/// unknown, or void. The predicate is free-standing so the checker and its
/// tests share one definition.
fn maybe_owns_ty(ty: &Ty) -> bool {
    lower::mentions_type_param(ty) && !is_view(ty) && !ty.is_unknown() && !ty.is_void()
}

/// Checks one function body and lowers it to the typed control-flow IR.
struct FunctionChecker<'a, 'd> {
    /// Program-wide ownership facts.
    program: &'a ProgramInfo<'a>,
    /// The loaded program, for module-aware global resolution.
    loaded: &'a Loaded,
    /// The module the function is declared in.
    module: &'a crate::load::Module,
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
    /// The place-state reports already emitted at each site.
    ///
    /// The site is the binding, the span's start, and the code. The update
    /// paths read a place twice and write it again — the operand walk, the
    /// read check, and the write check — so one site would otherwise repeat
    /// the same state error up to three times; the first report at a site
    /// emits and later reports for the same site are suppressed. Reads of one
    /// site share the span's start even when they select different projections
    /// (an element read also reads its base), so the projections are not part
    /// of the identity; they rank the reports instead, and a later report with
    /// more projections replaces the whole-place one at the same site. The
    /// value is the emitted diagnostic's index and the report's rank.
    state_reports: BTreeMap<(usize, u32, &'static str), (usize, usize)>,
    /// Number of enclosing `unsafe` constructs.
    unsafe_depth: u32,
    /// View-like lend parameters of the function, an elision candidate count.
    elision_candidates: usize,
    /// True when a returned view derives from static storage or a local, so
    /// the elision count should stay silent.
    suppress_elision: bool,
    /// The single view-like lend candidate of the function, when there is
    /// exactly one; the elision postlude compares the return's actual
    /// derivation against it.
    elision_candidate: Option<String>,
    /// True when a view-return candidate's declared type mentions a parameter.
    elision_type_param_candidate: bool,
    /// The view-return candidates' declared types and names.
    elision_candidate_types: Vec<(String, crate::types::Ty)>,
    /// The parameter the returned view derives from, when it is a parameter.
    view_return_source: Option<String>,
    /// The frame-local binding a type-parameter return reads.
    ///
    /// A return typed by a parameter becomes a view only at its view
    /// instances; a local owner returned there is the concrete `E10035`
    /// escape, which the instance audit reports.
    view_return_local_owner: Option<String>,
    /// True when a type-parameter return reads a fresh owned temporary.
    ///
    /// The view has no local name and no parameter source; the instance audit
    /// reports the concrete one-candidate rule when the return is a view.
    view_return_fresh: bool,
    /// True when different returns derive from different parameters.
    view_return_source_mixed: bool,
    /// The parameter and member path of every direct returned view.
    view_return_paths: Vec<(String, Vec<crate::types::PlacePathStep>)>,
    /// True when an observed view return derives from something other than
    /// the single candidate, so the postlude reports `E10036` even though the
    /// candidate count is one.
    return_derivation_mismatch: bool,
    /// Number of enclosing array-literal lowerings whose target is a
    /// view-element array: those elements keep their sources in place, so the
    /// view binding loans or adopts each element's backing instead of moving
    /// it out.
    view_array_literal_depth: u32,
    /// True when the current store destination is a local binding of this
    /// function frame, so a view-element array may adopt an element backing in
    /// this scope. A destination that outlives the frame — a global, a
    /// borrowed parameter slot, or a projected target — keeps the derived
    /// element in place instead.
    view_destination_adopts: bool,
    /// Where the struct literal currently being evaluated lands, which
    /// decides whether a caller-backed view field may remain: a frame
    /// destination and a return keep it, every other destination keeps
    /// module storage alone.
    literal_views: LiteralViews,
    /// True while the value of a discarded conditional statement is being
    /// checked.
    ///
    /// The statement drops whichever arm it selects, so a struct literal's
    /// field there is a read: nothing stores the value, and the source binding
    /// stays usable afterwards.
    discarded_conditional: bool,
    /// True while a discarded literal's field value is being checked.
    ///
    /// The field is read rather than stored, so an array literal inside it
    /// reads its elements the same way; an array literal that *is* a discarded
    /// arm keeps the ordinary element rules, exactly as the direct statement
    /// spelling does.
    discarded_literal_field: bool,
    /// Number of enclosing discarded conditionals: their arms materialize a
    /// fresh owning result into a statement temporary the guard fills on the
    /// arm's own path, so the statement's release frees exactly the arm that
    /// ran.
    discard_conditional_depth: u32,
    /// The control-flow graph being built.
    builder: Builder,
    /// Type of each compiler-created temporary, by creation order.
    temps: Vec<Ty>,
    /// Source span of the construct currently being walked; attached to the IR
    /// statements and terminators it emits.
    current_span: Span,
    /// Open loops and switches, innermost last.
    exits: Vec<ExitFrame>,
    /// Deferred blocks registered by the open scopes, in registration order.
    defer_entries: Vec<DeferEntry<'a>>,
    /// Registrations whose deferred block is currently being checked.
    ///
    /// A deferred block's own return path re-enters the enclosing scope's
    /// cleanup, so without this guard the same block would be checked again
    /// while its first check is still running. Each registration is checked
    /// once per cleanup run and once per exit edge.
    active_defer_checks: BTreeSet<u32>,
    /// The ordered loan event log for the function being checked.
    events: Vec<LoanEvent>,
    /// The next ordinal to hand out in each block's event sequence.
    next_ordinal: BTreeMap<BlockId, usize>,
    /// Stable identity of the persistent loan a view derivation creates.
    ///
    /// A loan is keyed by where it is created, not by allocation order, so the
    /// same derivation gets the same id in a speculative pass and on the
    /// reporting pass. That stability is what lets a loan created in a loop body
    /// or a deferred block be seen at the loop head and after the scope, where
    /// the speculative pass must carry it before the reporting pass records it.
    loan_ids: BTreeMap<LoanOrigin, LoanId>,
    /// The written name of every binding ever declared, by binding index.
    ///
    /// The name table outlives the binding's truncation at a deferred block's
    /// exit, so a diagnostic about a loan whose owner was declared there still
    /// names the owner instead of falling back to `?`.
    binding_names: BTreeMap<usize, String>,
    /// The joined state of every `continue` edge, by its target block.
    ///
    /// A `continue` returns to the loop head (or the `for` step), so its state
    /// must join the loop entry: a loan the edge creates is live at the body
    /// top, in the condition, and after the loop. The target block identifies
    /// the loop, so nested loops do not mix their continue edges.
    continue_states: BTreeMap<BlockId, FlowState>,
    /// The block a currently checked deferred block is anchored to.
    ///
    /// While a deferred block is checked on an exit edge its IR is lowered
    /// against a discarded clone, so the clone's block ids must not leak into
    /// the log. Every event recorded while this is set is placed at `anchor`
    /// block and in the order it is recorded, so the block's own statements run
    /// in order between the edge and the owner releases that follow it.
    defer_anchor: Option<BlockId>,
    /// Next cleanup registration number, shared by bindings and `defer`
    /// entries so a scope's cleanup can interleave them in one reverse order.
    next_registration: u32,
    /// Next name suffix for a compiler-owned temporary local that takes an
    /// unnamed owned call result so the scope cleanup can release it.
    next_owned_temp: u32,
    /// Generated-owner ids whose bindings a rolled-back deferred pass removed.
    ///
    /// A deferred block is checked once per exit edge against a discarded
    /// graph and its binding table range is rolled back afterwards, while the
    /// loan events it recorded stay for the liveness report. The generated
    /// owners it declared are therefore remembered here so the report can
    /// still render their phrase; an id a later binding has since taken is
    /// dropped from the report's set by comparing it against the live table.
    rolled_back_generated: BTreeSet<usize>,
    /// Destination temporaries of the statement currently being checked, in
    /// creation order.
    ///
    /// A value stored into a compiler temporary to be read later in the same
    /// statement owns its storage only until the statement ends. Every
    /// destination materialization whose type needs a drop joins this list, and
    /// the statement's end drains it in reverse creation order. Transfer
    /// temporaries (a take-call result, an aggregate-literal adoption) never
    /// join: their destination already owns the value.
    pending_releases: Vec<Release>,
    /// Nesting depth of the operands that are evaluated only conditionally.
    ///
    /// Inside a `&&`/`||` right operand or a `?:` arm, a call's `take`
    /// move-out must not run eagerly: it is carried on the materialized value
    /// and emitted on the evaluated path instead.
    defer_move_outs: u32,
    /// True while a loop condition is being lowered.
    ///
    /// A loop condition has its own owned-call materialization and release
    /// plumbing, so the member-receiver bind must not also fire there: the
    /// condition machinery already binds the calls it contains.
    in_loop_condition: bool,
    /// True while the ownership pass evaluates a loop condition.
    ///
    /// The lowering flag marks the deferred-emission context after the
    /// expression is evaluated; the take-argument check needs the same
    /// context during evaluation.
    in_loop_condition_check: bool,
    /// True while an arm of a conditional that was lowered in an eager context
    /// is being lowered.
    ///
    /// The arm's value travels into a store the enclosing construct emits, so
    /// the store fills the arm's fields itself: a literal inside the arm must
    /// not materialize into its own destination the way a guard operand does.
    /// The flag stays set through nested conditionals, because their arms are
    /// stored by the same construct.
    eager_conditional_arm: bool,
    /// Which temporaries are destinations the compiler releases at the
    /// statement's end, parallel to `temps`.
    ///
    /// A projection of such a temporary reads storage the release will free,
    /// so no second owner may be bound for it. A transfer temporary is not a
    /// destination: its value travels to a binding that owns it.
    destination_temps: Vec<bool>,
    /// False while lowering a deferred block, whose bindings are not kept in
    /// the function's local list, so an owned temporary must not be bound
    /// there.
    materialize_owned_temporaries: bool,
    /// False during a speculative pass, which computes state without reporting.
    /// True while lowering the speculative loop pass, whose IR — and whose
    /// lifted-function records — is discarded and re-lowered by the real pass.
    /// A `defer` registration lowering also reports no diagnostics, but its IR
    /// is emitted, so its records must survive.
    speculative_lowering: bool,
    report_diagnostics: bool,
    /// True while lowering a `defer` body.
    ///
    /// The body's owners stay in the block's own scope instead of moving to
    /// the function body: the deferred copy runs from every cleanup edge, so a
    /// body-scope release would run once for the whole program rather than
    /// once per execution.
    defer_body: bool,
    /// The later-use duplicability duties this body owes its instances.
    ///
    /// The checker records each maybe-owning store whose source is used again;
    /// the per-instance audit discharges every duty against each recorded
    /// concrete instantiation.
    duplicable_requirements: Vec<ir::RequiresDuplicable>,
    /// Later-use duties already recorded, keyed by the binding and its moved
    /// paths. A later use updates the destinations, the strict flag, and the
    /// reported span instead of adding a second duty.
    recorded_duplicable:
        BTreeMap<(usize, Vec<Vec<crate::types::PlacePathStep>>, u32, u32), usize>,
    /// The source field paths a view loan names.
    ///
    /// A carrier loan is keyed by its owner for liveness, but a write to a
    /// different field of that owner does not invalidate the view. The paths
    /// let the write check skip a conflict only when every loan on the owner
    /// reaches a disjoint member path. An empty entry keeps the whole-owner
    /// judgment.
    loan_source_paths:
        BTreeMap<LoanId, Vec<Vec<crate::types::PlacePathStep>>>,
    /// The borrowed-element move duties this body owes its instances.
    ///
    /// A value-consuming move through a `for (borrow e in xs)` element alias
    /// is the v1 element-move limit once the moved place owns. While the
    /// moved type is still a parameter or unresolved the checker records the
    /// duty; the per-instance audit reports the same limit for the first
    /// owning substitution.
    element_move_requirements: Vec<ir::RequiresElementMove>,
    /// The lend-move duties this body owes its instances.
    ///
    /// A lend moved into an owned position under a still-parameter type is
    /// judged after substitution: the per-instance audit reports `E10034` when
    /// the type argument owns.
    not_lend_requirements: Vec<ir::RequiresNotLend>,
    /// The whole out-slot store duties this body owes its instances.
    ///
    /// A whole store into a `borrow` slot whose declared type still mentions a
    /// type parameter is judged after substitution: a nullable instance
    /// reports `E10034` when the source is a different lend, while a `take`,
    /// static, own-value, or non-nullable substitution keeps the ordinary
    /// rule.
    whole_out_slot_store_requirements: Vec<ir::RequiresWholeOutSlotStore>,
    /// The element-loop iterable duties this body owes its instances.
    ///
    /// A loop over an opaque `T` accepts the declaration because its instance
    /// may be an array; the per-instance audit reports `E10043` when the
    /// substituted sequence or its element cannot be bound.
    iterable_requirements: Vec<ir::RequiresIterable>,
    /// The out-slot place duties this body owes its instances.
    ///
    /// A nullable borrow slot whose declared type still mentions a type
    /// parameter is judged after substitution: the per-instance audit applies
    /// the exact-place rule when the type argument makes the slot nullable.
    out_slot_duties: Vec<ir::RequiresOutSlotPlace>,
    /// The lend-slot store duties this body owes its instances.
    ///
    /// A whole-value store into a lend slot whose declared type still mentions
    /// a type parameter is judged after substitution: the per-instance audit
    /// reports `E10034` when the type argument owns the slot and stays exempt
    /// when the slot is nullable.
    storable_slot_requirements: Vec<ir::RequiresStorableSlot>,
    /// The whole-binding self-assign duties this body owes its instances.
    ///
    /// A self-assign through a lend root whose declared type still mentions a
    /// type parameter is judged after substitution: the per-instance audit
    /// reports the owning or nullable diagnostic, while a duplicable or view
    /// substitution stays accepted.
    self_assign_requirements: Vec<ir::RequiresSelfAssign>,
    /// The opaque-member store duties this body owes its instances.
    ///
    /// A store into a projection the declaration sees through a type parameter
    /// is judged after substitution: a view member accepts the widened value
    /// (and reports the escape when the carrier is returned), while an owned
    /// member keeps the concrete implicit-conversion judgment.
    opaque_store_requirements: Vec<ir::RequiresOpaqueStore>,
    /// The view-return candidate duties this body owes its instances.
    ///
    /// A view return whose candidates include a `borrow T` cannot decide the
    /// elision count until the instance supplies T; the per-instance audit
    /// reports the frozen view-return rule when the substituted candidates are
    /// not exactly the returned parameter.
    view_return_requirements: Vec<ir::RequiresViewReturnCandidates>,
    /// The narrowed-arm duties this body owes its instances.
    ///
    /// A nullable conditional arm at a type-parameter destination is judged
    /// per instance. The declaration records which arms its own flow already
    /// narrowed, so the instance reports only the arms that can still be null.
    conditional_narrowing_requirements: Vec<ir::RequiresConditionalArmNarrowing>,
    /// The `len` operands this body owes its instances.
    ///
    /// A generic `len(x)` can be a scalar at one instance and an array at
    /// another; the declaration records the site and the audit applies the
    /// frozen length surface after substitution.
    len_operand_requirements: Vec<ir::RequiresLenOperand>,
    /// Take view stores deferred to the call-site summary.
    take_view_requirements: Vec<TakeViewRequirement>,
    /// Take-view sites already recorded, so one store site reports once.
    recorded_take_escapes: BTreeSet<(String, u32, u32)>,
    /// Every normal/dot user-call argument recorded for the summary.
    take_call_arguments: Vec<TakeCallArgument>,
    /// Function declarations used as values, so their call forms are unknown.
    function_value_uses: BTreeSet<String>,
    /// Opaque-store sites already recorded, so one site owes one duty.
    recorded_opaque_stores: BTreeSet<(String, u32, u32)>,
    /// `len` operand sites already recorded, so one site owes one duty.
    recorded_len_operands: BTreeSet<(u32, u32)>,
    /// Move sites already recorded, so one site owes one lend duty.
    recorded_not_lend: BTreeSet<(String, u32, u32)>,
    /// Store spans that already emitted the concrete lend-move report.
    reported_store_spans: BTreeSet<(u32, u32)>,
    /// Whole out-slot store sites already recorded, so one site owes one duty.
    recorded_whole_out_slot_store: BTreeSet<(String, u32, u32)>,
    /// Store sites already recorded, so one site owes one slot duty.
    recorded_storable_slot: BTreeSet<(String, u32, u32)>,
    /// Self-assign sites already recorded, so one site owes one duty.
    recorded_self_assign: BTreeSet<(String, u32, u32)>,
    /// Later-use duties already recorded, keyed by the binding and the
    /// conflicting moved paths, so one moved-place set owes one duty.
    /// Move sites already recorded, so one site owes one element-move duty.
    ///
    /// The speculative loop pass lowers the same body twice, so the site —
    /// the binding and the span — identifies one duty.
    recorded_element_moves: BTreeSet<(usize, u32, u32)>,
    /// The lifted functions of every lambda this function lowers.
    ///
    /// A lambda's body becomes a file-scope `static` C function, so it is
    /// collected here and appended to the program's function list once the
    /// enclosing body is checked.
    lifted: Vec<ir::Function>,
    /// Next lambda-site id for a capture-less lambda, shared with every other
    /// function so one program never names two lifted functions the same.
    next_lifted: &'a mut usize,
    /// Diagnostic sink.
    diagnostics: &'d mut Diagnostics,
}

impl<'a, 'd> FunctionChecker<'a, 'd> {
    /// Creates a checker for one function.
    fn new(
        program: &'a ProgramInfo<'a>,
        loaded: &'a Loaded,
        module: &'a crate::load::Module,
        sema: &'a SemaOutput,
        decl: &'a crate::ast::FuncDecl,
        diagnostics: &'d mut Diagnostics,
        next_lifted: &'a mut usize,
    ) -> Self {
        let type_params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        let returns = resolve_ty_folded(&decl.returns, &type_params, &sema.table, module, loaded);
        let params: Vec<ir::Param> = decl
            .params
            .iter()
            .map(|param| ir::Param {
                name: param.name.clone(),
                ty: resolve_ty_folded(&param.ty, &type_params, &sema.table, module, loaded),
                mode: param.mode,
            })
            .collect();
        Self {
            program,
            loaded,
            module,
            sema,
            decl,
            type_params,
            returns: returns.clone(),
            scopes: Vec::new(),
            bindings: Vec::new(),
            state: FlowState::default(),
            unsafe_depth: 0,
            elision_candidates: 0,
            suppress_elision: false,
            elision_candidate: None,
            elision_type_param_candidate: false,
            elision_candidate_types: Vec::new(),
            view_return_source: None,
            view_return_local_owner: None,
            view_return_fresh: false,
            view_return_source_mixed: false,
            view_return_paths: Vec::new(),
            view_return_requirements: Vec::new(),
            conditional_narrowing_requirements: Vec::new(),
            len_operand_requirements: Vec::new(),
            take_view_requirements: Vec::new(),
            recorded_take_escapes: BTreeSet::new(),
            take_call_arguments: Vec::new(),
            function_value_uses: BTreeSet::new(),
            return_derivation_mismatch: false,
            view_array_literal_depth: 0,
            view_destination_adopts: true,
            literal_views: LiteralViews::Escaping,
            discarded_conditional: false,
            discarded_literal_field: false,
            discard_conditional_depth: 0,
            builder: Builder::new(&decl.name, params, returns),
            temps: Vec::new(),
            current_span: Span::default(),
            exits: Vec::new(),
            defer_entries: Vec::new(),
            active_defer_checks: BTreeSet::new(),
            events: Vec::new(),
            next_ordinal: BTreeMap::new(),
            loan_ids: BTreeMap::new(),
            binding_names: BTreeMap::new(),
            state_reports: BTreeMap::new(),
            continue_states: BTreeMap::new(),
            defer_anchor: None,
            next_registration: 0,
            next_owned_temp: 0,
            rolled_back_generated: BTreeSet::new(),
            pending_releases: Vec::new(),
            defer_move_outs: 0,
            in_loop_condition: false,
            in_loop_condition_check: false,
            eager_conditional_arm: false,
            destination_temps: Vec::new(),
            materialize_owned_temporaries: true,
            speculative_lowering: false,
            report_diagnostics: true,
            defer_body: false,
            duplicable_requirements: Vec::new(),
            recorded_duplicable: BTreeMap::new(),
            loan_source_paths: BTreeMap::new(),
            element_move_requirements: Vec::new(),
            not_lend_requirements: Vec::new(),
            whole_out_slot_store_requirements: Vec::new(),
            iterable_requirements: Vec::new(),
            out_slot_duties: Vec::new(),
            storable_slot_requirements: Vec::new(),
            self_assign_requirements: Vec::new(),
            opaque_store_requirements: Vec::new(),
            recorded_opaque_stores: BTreeSet::new(),
            recorded_len_operands: BTreeSet::new(),
            recorded_not_lend: BTreeSet::new(),
            reported_store_spans: BTreeSet::new(),
            recorded_whole_out_slot_store: BTreeSet::new(),
            recorded_storable_slot: BTreeSet::new(),
            recorded_self_assign: BTreeSet::new(),
            recorded_element_moves: BTreeSet::new(),
            lifted: Vec::new(),
            next_lifted,
            diagnostics,
        }
    }

    /// Runs the whole function check and finishes the IR.
    ///
    /// Returns the declared body, every lambda lifted out of it, the
    /// duplicability duties the body owes its instances, the lend-move duties
    /// it owes them, the out-slot place duties it owes them, the lend-slot
    /// store duties it owes them, and the whole-binding self-assign duties it
    /// owes them.
    fn check(
        mut self,
    ) -> (
        ir::Function,
        Vec<ir::Function>,
        Vec<ir::RequiresDuplicable>,
        Vec<ir::RequiresElementMove>,
        Vec<ir::RequiresNotLend>,
        Vec<ir::RequiresWholeOutSlotStore>,
        Vec<ir::RequiresIterable>,
        Vec<ir::RequiresOutSlotPlace>,
        Vec<ir::RequiresStorableSlot>,
        Vec<ir::RequiresSelfAssign>,
        Vec<ir::RequiresOpaqueStore>,
        Vec<ir::RequiresViewReturnCandidates>,
        Vec<ir::RequiresConditionalArmNarrowing>,
        Vec<ir::RequiresLenOperand>,
        Vec<TakeViewRequirement>,
        Vec<TakeCallArgument>,
        BTreeSet<String>,
    ) {
        let returns = self.returns.clone();
        self.check_nullable_decl(&returns, self.decl.name_span);
        self.elision_prelude();
        self.push_scope(self.decl.body.span);
        for param in &self.decl.params {
            self.declare_param(param);
        }
        self.check_block(&self.decl.body);
        self.pop_scope();
        self.elision_postlude();
        self.report_unused_locals();
        if !self.is_terminated() {
            self.builder
                .terminate(Terminator::Return {
                    value: None,
                    releases: Vec::new(),
                });
        }
        let generated = self.generated_owner_ids();
        let mut function = self.builder.finish();
        Self::sync_param_names(&mut function, &self.bindings);
        function.locals = self
            .bindings
            .iter()
            // A base-overridden binding (a lambda capture or a borrow-element
            // alias) never declares a C local: its places root at the override.
            .filter(|binding| binding.kind == BindingKind::Local && binding.base.is_none())
            .map(|binding| ir::Local {
                name: binding.ir_name.clone(),
                ty: binding.ty.clone(),
            })
            .collect();
        function.temps = self.temps.clone();
        liveness::check(
            &function,
            &self.events,
            &self.binding_names,
            &generated,
            self.diagnostics,
        );
        (
            function,
            self.lifted,
            self.duplicable_requirements,
            self.element_move_requirements,
            self.not_lend_requirements,
            self.whole_out_slot_store_requirements,
            self.iterable_requirements,
            self.out_slot_duties,
            self.storable_slot_requirements,
            self.self_assign_requirements,
            self.opaque_store_requirements,
            self.view_return_requirements,
            self.conditional_narrowing_requirements,
            self.len_operand_requirements,
            self.take_view_requirements,
            self.take_call_arguments,
            self.function_value_uses,
        )
    }

    /// Records a `len` operand whose declared type still mentions a parameter.
    ///
    /// The free and dot spellings share the duty; one site records once even
    /// when the ownership walk visits the expression twice.
    pub(super) fn record_len_operand(
        &mut self,
        ty: &Ty,
        span: Span,
        narrowed: bool,
        base: Option<String>,
    ) {
        if !crate::types::contains_param(ty) {
            return;
        }
        let key = (span.start, span.end);
        if !self.recorded_len_operands.insert(key) {
            return;
        }
        self.len_operand_requirements.push(ir::RequiresLenOperand {
            function: self.decl.name.clone(),
            ty: ty.clone(),
            span,
            narrowed,
            base,
        });
    }

    // -- scopes and bindings -------------------------------------------------

    /// Enters a new lexical scope and returns its id.
    ///
    /// The parent is the scope currently on top of the stack, or `None` for the
    /// function body scope. A scope that opens while a loop body is being
    /// checked is marked `in_loop`, so its cleanup runs on each iteration
    /// rather than once at the end of the function. `close_span` is the source
    /// range of the point that closes the scope, so the emitter can attribute
    /// the scope's generated cleanup to that line.
    fn push_scope(&mut self, close_span: Span) -> ir::ScopeId {
        let parent = self.scopes.last().map(|scope| scope.id);
        let in_loop = self
            .exits
            .iter()
            .any(|frame| frame.continue_block.is_some());
        let id = self.builder.add_scope(parent, in_loop, close_span);
        self.scopes.push(Scope {
            id,
            close_span,
            ..Scope::default()
        });
        id
    }

    /// The id of the innermost open scope.
    ///
    /// Every statement is emitted inside at least the function body scope, so
    /// the stack is never empty when this is called; the fallback keeps the
    /// caller total rather than panicking.
    fn current_scope(&self) -> ir::ScopeId {
        self.scopes.last().map(|scope| scope.id).unwrap_or(0)
    }

    /// Leaves the innermost scope when its exit edge already ran the cleanup.
    ///
    /// A block that ended in `return`, `break`, or `continue` ran the scope's
    /// cleanup as part of that edge, so the scope's deferred entries are
    /// dropped here without checking them again: a second check would read
    /// state the edge already released, and the fall-through cleanup this
    /// scope would otherwise run is unreachable.
    fn close_scope_after_edge(&mut self) {
        let depth = self.scopes.len();
        if depth == 0 {
            return;
        }
        while self
            .defer_entries
            .last()
            .is_some_and(|entry| entry.depth == depth)
        {
            self.defer_entries.pop();
        }
        self.scopes.pop();
    }

    /// Leaves the innermost scope, running its cleanup in registration order.
    ///
    /// A scope's drops and `defer` blocks share one registration sequence and
    /// the emitter runs them in reverse, so the checker interleaves them the
    /// same way. The scope stays on the stack while its cleanup runs, so the
    /// names the deferred blocks close over are still visible to the walk.
    fn pop_scope(&mut self) {
        let depth = self.scopes.len();
        if depth == 0 {
            return;
        }
        self.run_scope_cleanup(depth - 1);
        // The scope's deferred entries end with the scope: a cleanup for an
        // enclosing scope must not see them again. Entries kept across earlier
        // exit edges are dropped here, at the scope's final edge.
        while self
            .defer_entries
            .last()
            .is_some_and(|entry| entry.depth == depth)
        {
            self.defer_entries.pop();
        }
        self.scopes.pop();
    }

    /// Runs one scope's cleanup actions in reverse registration order.
    ///
    /// `scope_index` is the scope's position on the stack; the caller leaves it
    /// in place, and every deeper scope has already run its own cleanup, so the
    /// scope's deferred blocks are the newest entries in `defer_entries`. A
    /// deferred block registered before a binding therefore runs after that
    /// binding's release, exactly as the emitter orders them. Each drop is
    /// tagged with the scope's id so the emitter can place it on the matching
    /// rung of the cleanup ladder.
    fn run_scope_cleanup(&mut self, scope_index: usize) {
        let scope_depth = scope_index + 1;
        let scope_id = self.scopes[scope_index].id;
        let mut actions: Vec<(u32, CleanupAction<'a>)> = Vec::new();
        for id in &self.scopes[scope_index].order {
            actions.push((
                self.bindings[*id].registration,
                CleanupAction::Release(*id),
            ));
        }
        for entry in self.defer_entries.iter().rev() {
            // Deeper scopes' entries can still sit above this scope's on the
            // stack (their cleanup kept them), so the scan collects every
            // matching entry instead of stopping at the first depth change.
            if entry.depth == scope_depth {
                actions.push((
                    entry.registration,
                    CleanupAction::Defer(entry.block, entry.registration),
                ));
            }
        }
        actions.sort_by(|left, right| right.0.cmp(&left.0));
        for (_, action) in actions {
            match action {
                CleanupAction::Release(id) => {
                    let span = self.current_span;
                    self.record_event(
                        LoanEventKind::Release {
                            owner: LoanOwner::Binding(id),
                        },
                        span,
                    );
                    if self.bindings[id].needs_drop {
                        let place = self.binding_place(id);
                        let order = self.bindings[id].registration;
                        self.emit(Statement::Drop {
                            place,
                            scope: scope_id,
                            order,
                        });
                    }
                    self.state.places.remove(&id);
                    self.state.loans.remove(&id);
                }
                CleanupAction::Defer(block, registration) => {
                    // A deferred block's own return path re-enters the
                    // enclosing scope's cleanup; the block already running must
                    // not be checked a second time on that path.
                    if !self.active_defer_checks.insert(registration) {
                        continue;
                    }
                    let anchor = self.builder.current();
                    let saved = self.state.clone();
                    // Every edge checks the block for its flow effects, so a
                    // diagnostic it re-reports describes the same source
                    // occurrence; drop the repeats while keeping distinct ones.
                    let reported = self.diagnostics.items().len();
                    self.check_defer_exit(block, anchor);
                    self.diagnostics.dedupe_from(reported);
                    self.restore_enclosing_state(&saved, scope_index);
                    self.active_defer_checks.remove(&registration);
                }
            }
        }
    }

    /// Restores the scopes of the running cleanup after a deferred block was
    /// checked.
    ///
    /// A `return`, `break`, or `continue` inside a deferred block cleans the
    /// scopes it leaves as part of its own path, and that cleanup removes their
    /// bindings from the flow state. The running scope and its enclosers still
    /// owe their own release actions in this same cleanup run, so a binding the
    /// deferred check dropped is put back and dropped again when its release
    /// action's turn comes, in the emitted order. Deeper scopes already ran
    /// their cleanup and stay dropped, and a binding the deferred block itself
    /// rewrote, moved, or cleared keeps its new state. `saved` is the state
    /// before the block was checked, and `scope_index` is the scope running the
    /// cleanup.
    fn restore_enclosing_state(&mut self, saved: &FlowState, scope_index: usize) {
        let mut dropped: Vec<usize> = Vec::new();
        for scope in self.scopes.iter().take(scope_index + 1) {
            for id in &scope.order {
                if saved.places.contains_key(id) && !self.state.places.contains_key(id) {
                    dropped.push(*id);
                }
            }
        }
        for id in dropped {
            self.state.places.insert(id, saved.place(id));
            match saved.loans.get(&id) {
                Some(loans) => {
                    self.state.loans.insert(id, loans.clone());
                }
                None => {
                    self.state.loans.remove(&id);
                }
            }
        }
    }

    /// Checks one deferred block on an exit edge without emitting IR.
    ///
    /// The block is checked against a discarded clone of the graph, so every
    /// event it records is anchored to the real block the edge leaves rather
    /// than to a clone block that a later real block may reuse. A nested defer
    /// keeps the outer anchor, so its events stay in the outer block's order.
    /// The flow state is kept, except for what the enclosing scopes lose to
    /// control flow inside the block: only the graph, the binding table, and
    /// the scopes are rolled back, so a write the defer makes to an outer
    /// binding stays visible on the flow that leaves the scope.
    fn check_defer_exit(&mut self, block: &'a Block, anchor: BlockId) {
        let saved_builder = self.builder.clone();
        let saved_bindings = self.bindings.len();
        let saved_scopes = self.scopes.clone();
        let saved_defers = self.defer_entries.len();
        let saved_temps = self.temps.len();
        let saved_releases = self.pending_releases.len();
        let saved_anchor = self.defer_anchor;
        if self.defer_anchor.is_none() {
            self.defer_anchor = Some(anchor);
        }
        self.push_scope(block.span);
        for statement in &block.statements {
            self.check_stmt(statement);
        }
        if self.is_terminated() {
            // The block left through `return`, `break`, or `continue`, and that
            // edge already ran the enclosing cleanup (or the block diverged).
            // Leaving the block scope must not check its deferred blocks again.
            self.close_scope_after_edge();
        } else {
            self.pop_scope();
        }
        self.defer_anchor = saved_anchor;
        self.builder = saved_builder;
        // A generated owner the deferred body declared is a copy this rollback
        // removes; the events recorded for it are judged later against the
        // shortened table, so its id is remembered for the liveness report.
        for id in saved_bindings..self.bindings.len() {
            if self.bindings[id].generated {
                self.rolled_back_generated.insert(id);
            }
        }
        // The deferred body's bindings are removed from the live scopes, but
        // their ids stay reserved. A later binding that reuses a parked id
        // would otherwise give the escaped owner report the later binding's
        // name; keeping the binding table slots makes the generated owner
        // recognizable at every report site.
        let parked = self.bindings[saved_bindings..].to_vec();
        self.bindings.truncate(saved_bindings);
        self.scopes = saved_scopes;
        self.defer_entries.truncate(saved_defers);
        self.temps.truncate(saved_temps);
        self.destination_temps.truncate(saved_temps);
        self.pending_releases.truncate(saved_releases);
        // A temporary the deferred block materialized is no longer a binding,
        // so its state must not be inherited by a later local that reuses the
        // id.
        self.state.drop_bindings_from(saved_bindings);
        // Reserve the parked ids with inert binding slots. A `Param` kind with
        // an explicit base is never declared as a C local, and the restored
        // scopes cannot reach it, so only the binding table's identity and the
        // recorded display name remain.
        for mut binding in parked {
            binding.kind = BindingKind::Param;
            binding.base = Some(ir::PlaceBase::Local(binding.ir_name.clone()));
            binding.used = true;
            self.bindings.push(binding);
        }
    }

    /// Repeats `run_body` in a speculative pass until the loop state stops
    /// changing, starting from `pre` joined with the body effect.
    ///
    /// The speculative pass emits into a copy of the graph and reports no
    /// diagnostics; the graph and the binding table are restored before the
    /// real pass that lowers the body. The head joins the pre-state with the
    /// fall-through state and with every `continue` edge back to this loop;
    /// a `break` or `return` edge leaves the loop, so its effect reaches only
    /// the exit and never the head. It returns the stable entry state and the
    /// stable state after the body, which is the exit state of a loop whose
    /// body always runs.
    fn loop_entry_state<F>(
        &mut self,
        pre: FlowState,
        continue_block: Option<BlockId>,
        run_body: F,
    ) -> (FlowState, FlowState)
    where
        F: Fn(&mut Self),
    {
        let saved_state = self.state.clone();
        let saved_builder = self.builder.clone();
        let saved_bindings = self.bindings.len();
        let saved_scopes = self.scopes.clone();
        let saved_defers = self.defer_entries.len();
        let saved_exits = self.exits.len();
        let saved_temps = self.temps.len();
        let saved_releases = self.pending_releases.clone();
        let saved_owned_temp = self.next_owned_temp;
        let saved_report = self.report_diagnostics;
        let saved_speculative = self.speculative_lowering;
        self.report_diagnostics = false;
        self.speculative_lowering = true;
        let (entry, stable_post) = 'fixpoint: {
            let mut entry = pre.clone();
            loop {
                self.state = entry.clone();
                self.scopes = saved_scopes.clone();
                self.bindings.truncate(saved_bindings);
                self.defer_entries.truncate(saved_defers);
                self.exits.truncate(saved_exits);
                self.builder = saved_builder.clone();
                run_body(self);
                let post = self.state.clone();
                // A body whose current block is already terminated does not
                // fall through to a later point in the body, so its effect
                // belongs to that exit edge rather than to the next iteration's
                // entry. A `continue` edge, however, does return to the head,
                // so the state recorded on every continue edge to this loop
                // joins the entry even when the body ended in a terminator.
                let mut next = pre.clone();
                if !self.is_terminated() {
                    next = Self::merge(next, post.clone());
                }
                if let Some(state) = continue_block
                    .and_then(|target| self.continue_states.get(&target))
                    .cloned()
                {
                    next = Self::merge(next, state);
                }
                if next == entry {
                    break 'fixpoint (entry, post);
                }
                entry = next;
            }
        };
        self.state = saved_state;
        self.builder = saved_builder;
        self.bindings.truncate(saved_bindings);
        self.scopes = saved_scopes;
        self.defer_entries.truncate(saved_defers);
        self.exits.truncate(saved_exits);
        self.temps.truncate(saved_temps);
        self.destination_temps.truncate(saved_temps);
        self.pending_releases = saved_releases;
        // A speculative body may bind view owners; the real pass declares them
        // again, so the numbering restarts and the emitted names stay stable.
        self.next_owned_temp = saved_owned_temp;
        self.report_diagnostics = saved_report;
        self.speculative_lowering = saved_speculative;
        let mut entry = entry;
        entry.drop_bindings_from(saved_bindings);
        (entry, stable_post)
    }

    /// Declares a local binding and returns its index.
    fn declare_local(&mut self, name: &str, ty: Ty, is_const: bool, span: Span) -> usize {
        let needs_drop = self.owns_drop(&ty) || self.maybe_owns(&ty);
        self.declare(
            name,
            span,
            ty,
            BindingKind::Local,
            Mode::None,
            is_const,
            needs_drop,
            true,
        )
    }

    /// Declares a compiler-generated local and returns its index.
    ///
    /// The binding joins its scope's cleanup order like any local, but it
    /// never enters the scope's name map: a source binding can never resolve
    /// to generated storage, so a source local spelled like a generated owner
    /// (`__cl_owned<N>`) keeps its own identity. Every use of the generated
    /// binding goes through its binding id.
    fn declare_generated_local(&mut self, name: &str, ty: Ty, span: Span) -> usize {
        let needs_drop = self.owns_drop(&ty) || self.maybe_owns(&ty);
        let id = self.declare(
            name,
            span,
            ty,
            BindingKind::Local,
            Mode::None,
            false,
            needs_drop,
            false,
        );
        self.bindings[id].generated = true;
        id
    }

    /// The binding indices of the compiler-generated owners.
    ///
    /// A generated owner has no source name a reader could recognize, so the
    /// liveness report renders it as a display phrase and needs the
    /// generated bindings' identities to do so. A deferred body's copy is
    /// declared, recorded in an event, and then rolled back with its binding
    /// table range, so its id is reported from the remembered set; an id a
    /// later binding has since taken belongs to that binding instead.
    fn generated_owner_ids(&self) -> BTreeSet<usize> {
        let mut ids: BTreeSet<usize> = self
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, binding)| binding.generated)
            .map(|(id, _)| id)
            .collect();
        ids.extend(
            self.rolled_back_generated
                .iter()
                .filter(|id| **id >= self.bindings.len()),
        );
        ids
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
        let needs_drop =
            param.mode == Mode::Take && (self.owns_drop(&ty) || self.maybe_owns(&ty));
        let id = self.declare(
            &param.name,
            param.name_span,
            ty,
            kind,
            param.mode,
            false,
            needs_drop,
            true,
        );
        self.set_place(id, PlaceState {
            initialized: true,
            ..PlaceState::default()
        });
    }

    /// Inserts a binding into the innermost scope.
    ///
    /// `register_name` controls the scope's name map: source bindings register
    /// so `lookup` can resolve them, while a compiler-generated binding skips
    /// registration so no source name can ever resolve to generated storage.
    /// The binding joins the scope's cleanup order either way.
    fn declare(
        &mut self,
        name: &str,
        span: Span,
        ty: Ty,
        kind: BindingKind,
        mode: Mode,
        is_const: bool,
        needs_drop: bool,
        register_name: bool,
    ) -> usize {
        let id = self.bindings.len();
        self.binding_names.insert(id, name.to_owned());
        let scope = self.scopes.len();
        let registration = self.register_cleanup();
        let provenance = if kind == BindingKind::Param && is_lend_candidate(&ty) {
            Provenance::Param(id)
        } else {
            Provenance::None
        };
        let ir_name = self.unique_ir_name(name);
        self.bindings.push(Binding {
            name: name.to_owned(),
            ir_name,
            ty,
            span,
            used: false,
            scope,
            scope_end: self.scopes.last().map(|s| s.close_span).unwrap_or_default(),
            kind,
            mode,
            is_const,
            needs_drop,
            registration,
            provenance,
            copy_source: None,
            rebound: false,
            base: None,
            alias_root: None,
            generated: false,
            capture_dies_with_frame: false,
            capture_caller_slot: false,
            capture_read_only: false,
            static_view_carrier: false,
            capture_whole_out_slot: false,
            capture_written: false,
        });
        if let Some(scope) = self.scopes.last_mut() {
            if register_name {
                scope.names.insert(name.to_owned(), id);
            }
            scope.order.push(id);
        }
        id
    }

    /// The unique IR spelling for a new binding named `name`.
    ///
    /// The first binding with a source name keeps it; a later binding that
    /// shadows it appends `#<binding-index>`. The binding table is not
    /// truncated when a scope closes, so every binding declared so far in the
    /// function counts and sibling scopes get distinct spellings even though
    /// neither shadows the other. A binding whose name matches a module-level
    /// callee is suffixed for every binding kind: the emitter would otherwise
    /// resolve a call through that binding to the same-named function, and the
    /// binding must win because it shadows the module name. The suffix makes
    /// the name unresolvable by the module lookup, so every name-based
    /// consumer treats the call as a call through the binding.
    fn unique_ir_name(&self, name: &str) -> String {
        let shadowed = self.bindings.iter().any(|binding| binding.name == name);
        let masks_callee = self.program.callee(name).is_some();
        if shadowed || masks_callee {
            format!("{name}#{}", self.bindings.len())
        } else {
            name.to_owned()
        }
    }

    /// Copies the declared parameter spellings onto a finished function.
    ///
    /// A parameter that shares its written name with a module callee keeps a
    /// masked IR name, so the emitted signature, the local-name table, and the
    /// lowered callee names all agree. Parameters are the first bindings
    /// declared, in declaration order, so the function's parameter list lines
    /// up with the binding table.
    fn sync_param_names(function: &mut ir::Function, bindings: &[Binding]) {
        for (index, param) in function.params.iter_mut().enumerate() {
            if let Some(binding) = bindings.get(index) {
                param.name = binding.ir_name.clone();
            }
        }
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

    /// True when a value of this type may own storage once a type argument is
    /// substituted.
    ///
    /// A type that mentions a generic parameter is treated as maybe-owning when
    /// it is not a view, unknown, or void. At every recorded instantiation the
    /// substituted type either owns (the emitted drop and release run) or does
    /// not (they are no-ops), so the checker can register the duty without
    /// knowing the argument. The parameter mode is not consulted here;
    /// `declare_param` applies its own `take` gate, so a `borrow` parameter or
    /// an out-slot never becomes maybe-owning.
    fn maybe_owns(&self, ty: &Ty) -> bool {
        maybe_owns_ty(ty)
    }

    /// The IR place for a binding.
    fn binding_place(&self, id: usize) -> Place {
        let binding = &self.bindings[id];
        // A lifted lambda's capture is rooted at its environment field rather
        // than at a C local, so it never takes a name in the enclosing frame.
        if let Some(base) = &binding.base {
            return Place {
                base: base.clone(),
                projections: Vec::new(),
                ty: binding.ty.clone(),
            };
        }
        let place = match binding.kind {
            BindingKind::Param | BindingKind::OutSlot => Place::param(&binding.ir_name),
            BindingKind::Local => Place::local(&binding.ir_name),
        };
        place.with_type(binding.ty.clone())
    }

    /// The IR place for a resolved target, keeping the binding's own name.
    fn target_place(&self, target: &PlaceTarget) -> Place {
        match target {
            PlaceTarget::Local(id, projections) => Place {
                base: self.binding_place(*id).base,
                projections: projections.clone(),
                ty: self.projected_type(*id, projections),
            },
            PlaceTarget::Global(id, projections) => Place {
                base: ir::PlaceBase::Global(id.clone()),
                projections: projections.clone(),
                ty: self
                    .program
                    .globals
                    .get(id)
                    .map(|global| self.projected_ty(global.ty.clone(), projections))
                    .unwrap_or_else(Ty::unknown),
            },
        }
    }

    /// The type a projection chain reaches from a base type.
    ///
    /// An element projection removes the outermost extent but keeps the
    /// remaining ones, so indexing a `T[M][N]` reaches `T[N]`, not the base
    /// scalar. Keeping the array type matters for a store: an assignment to a
    /// row is a whole-array copy, not a scalar assignment.
    fn projected_ty(&self, ty: Ty, projections: &[ir::Projection]) -> Ty {
        let mut ty = ty;
        for projection in projections {
            ty = match projection {
                ir::Projection::Field(field) => self.member_type(&ty, field),
                // A raw element read sees through the raw pointer to its
                // pointee, so the moved type is the pointee itself rather than
                // unknown or the pointee's element.
                ir::Projection::Element(_) | ir::Projection::ElementTemp(_) => {
                    match &ty.kind {
                        crate::types::TyKind::Raw(inner) => (**inner).clone(),
                        _ => ty.iterated_element(),
                    }
                }
            };
        }
        ty
    }

    /// The type a projection chain reaches from a local binding.
    fn projected_type(&self, id: usize, projections: &[ir::Projection]) -> Ty {
        self.projected_ty(self.bindings[id].ty.clone(), projections)
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

    /// The state that leaves the scopes at or below `floor` on an exit edge.
    ///
    /// The scopes being left are removed from a clone of the current state, so
    /// a `break` or `continue` edge does not carry a closed binding's loan into
    /// the loop head or the code after the loop.
    fn edge_state(&self, floor: usize) -> FlowState {
        let mut state = self.state.clone();
        for scope in self.scopes.iter().skip(floor) {
            for id in &scope.order {
                state.places.remove(id);
                state.loans.remove(id);
                state.element_loans.remove(id);
            }
        }
        state
    }

    /// Records a `continue` edge's state against the block it jumps to.
    ///
    /// Two `continue`s to the same loop join by union, exactly like branches.
    fn record_continue(&mut self, target: BlockId, state: FlowState) {
        match self.continue_states.get(&target) {
            Some(existing) => {
                let joined = Self::merge(existing.clone(), state);
                self.continue_states.insert(target, joined);
            }
            None => {
                self.continue_states.insert(target, state);
            }
        }
    }

    /// Merges two branch states conservatively.
    ///
    /// A move or partial move on either path survives; a value is initialized
    /// only when it is on both; narrowing survives only when both paths agree;
    /// a view binding carries every loan either path gave it.
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
            let mut merged = PlaceState {
                moved: a.moved || b.moved,
                // A maybe-owning consume on either path survives the join,
                // so a later use still deposits the duty.
                moved_if_owning: a.moved_if_owning.union(&b.moved_if_owning).cloned().collect(),
                // The destinations and the strict markers join by union, so a
                // write on one path cannot hide the other path's move.
                maybe_move_destinations: {
                    let mut destinations = a.maybe_move_destinations.clone();
                    for (path, found) in &b.maybe_move_destinations {
                        let entry = destinations.entry(path.clone()).or_default();
                        for destination in found {
                            if !entry.contains(destination) {
                                entry.push(destination.clone());
                            }
                        }
                    }
                    destinations
                },
                maybe_move_strict: a
                    .maybe_move_strict
                    .union(&b.maybe_move_strict)
                    .cloned()
                    .collect(),
                partial: a.partial.union(&b.partial).cloned().collect(),
                initialized: a.initialized && b.initialized,
                initialized_fields,
                narrowed: a.narrowed.intersection(&b.narrowed).cloned().collect(),
                sources: a.sources.clone(),
            };
            // The sources a local may hold join by union: either path's value
            // can reach the store, so every source must stay visible.
            merged.join_sources(&b.sources);
            places.insert(id, merged);
        }
        let mut loans: BTreeMap<usize, BTreeSet<LoanId>> = BTreeMap::new();
        for (id, set) in first.loans.iter().chain(second.loans.iter()) {
            loans.entry(*id).or_default().extend(set.iter().copied());
        }
        loans.retain(|_, set| !set.is_empty());
        // A path either branch still carries keeps its loans after the join,
        // because the join's value may be the one that path stored.
        let mut element_loans: BTreeMap<usize, BTreeMap<Vec<ir::Projection>, BTreeSet<LoanId>>> =
            BTreeMap::new();
        for (id, paths) in first.element_loans.iter().chain(second.element_loans.iter()) {
            let entry = element_loans.entry(*id).or_default();
            for (path, set) in paths {
                entry.entry(path.clone()).or_default().extend(set.iter().copied());
            }
        }
        element_loans.retain(|_, paths| {
            paths.retain(|_, set| !set.is_empty());
            !paths.is_empty()
        });
        let mut carried_owners: BTreeMap<usize, BTreeSet<liveness::LoanOwner>> = BTreeMap::new();
        for (id, set) in first
            .carried_owners
            .iter()
            .chain(second.carried_owners.iter())
        {
            carried_owners.entry(*id).or_default().extend(set.iter().cloned());
        }
        carried_owners.retain(|_, set| !set.is_empty());
        let reinitialized_sources = first
            .reinitialized_sources
            .intersection(&second.reinitialized_sources)
            .cloned()
            .collect();
        let global_narrowed = first
            .global_narrowed
            .intersection(&second.global_narrowed)
            .cloned()
            .collect();
        FlowState {
            places,
            loans,
            element_loans,
            carried_owners,
            reinitialized_sources,
            global_narrowed,
        }
    }

    // -- IR emission ---------------------------------------------------------

    /// Emits a statement into the current block.
    ///
    /// The current span is attached so the emitter can place a `#line`
    /// directive before the statement.
    fn emit(&mut self, statement: Statement) {
        self.builder.set_span(self.current_span);
        self.builder.emit(statement);
    }

    /// Starts a fresh IR block.
    fn new_block(&mut self) -> BlockId {
        self.builder.new_block()
    }

    /// Allocates the next cleanup registration number.
    ///
    /// Bindings and `defer` entries take one number in the order the walk
    /// reaches them, so the emitter can interleave a scope's drops and defers
    /// in a single reverse registration order.
    fn register_cleanup(&mut self) -> u32 {
        let registration = self.next_registration;
        self.next_registration += 1;
        registration
    }

    /// Appends one loan event to the function's log.
    ///
    /// Nothing is recorded during a speculative pass, which computes flow
    /// state without reporting. An event takes the next ordinal in its block;
    /// an event recorded while a deferred block is checked is placed at that
    /// block's exit-edge anchor in recorded order, so the block's own accesses
    /// keep their order relative to each other and to the releases on the edge.
    fn record_event(&mut self, kind: LoanEventKind, span: Span) {
        if !self.report_diagnostics {
            return;
        }
        let block = self.defer_anchor.unwrap_or_else(|| self.builder.current());
        let index = self.next_ordinal.entry(block).or_insert(0);
        let position = Position {
            block,
            index: *index,
        };
        *index += 1;
        self.events.push(LoanEvent {
            position,
            span,
            kind,
        });
    }

    /// The stable id of a loan created by a derivation with this origin.
    ///
    /// Interning makes the id independent of how many times a pass walks the
    /// derivation, so a speculative loop pass and the reporting pass agree, and
    /// a loan created in a loop body does not multiply once per iteration.
    fn intern_loan(&mut self, origin: LoanOrigin) -> LoanId {
        let next = self.loan_ids.len();
        *self
            .loan_ids
            .entry(origin)
            .or_insert_with(|| LoanId(next))
    }

    /// Records a compiler-created temporary and returns its id.
    ///
    /// A temporary is declared with the locals and named `__cl_tmp<N>` by the
    /// emitter. It has no binding, so it takes no part in the ownership walk
    /// except through the release list below.
    fn new_temp(&mut self, ty: Ty) -> ir::TempId {
        let id = self.temps.len() as ir::TempId;
        self.temps.push(ty);
        self.destination_temps.push(false);
        id
    }

    /// Marks a temporary as a destination the statement end releases.
    fn mark_destination_temp(&mut self, temp: ir::TempId) {
        if let Some(entry) = self.destination_temps.get_mut(temp as usize) {
            *entry = true;
        }
    }

    /// True when a temporary is a destination the statement end releases.
    fn is_destination_temp(&self, temp: ir::TempId) -> bool {
        self.destination_temps
            .get(temp as usize)
            .copied()
            .unwrap_or(false)
    }

    /// True when the temporary already carries a registered release.
    ///
    /// Such a temporary's storage is released by the statement cleanup or the
    /// enclosing construct, so a second owning binding for the same value would
    /// free it twice.
    fn has_pending_release(&self, temp: ir::TempId, ty: &Ty) -> bool {
        self.pending_releases
            .iter()
            .any(|release| release.temp == temp && &release.ty == ty)
    }

    /// Registers a destination temporary for release at the statement's end.
    ///
    /// Only a temporary whose type owns storage joins the pending list; a
    /// scalar or view temporary has no storage to free. The statement's end
    /// drains the list in reverse creation order through a dedicated inline
    /// release, so the storage dies with the statement that created it.
    fn register_pending_release(&mut self, temp: ir::TempId, ty: &Ty) {
        if self.owns_drop(ty) || self.maybe_owns(ty) {
            self.pending_releases.push(Release {
                temp,
                ty: ty.clone(),
            });
        }
    }

    /// Drains destination temporaries created since `floor`, newest first.
    ///
    /// A nested statement records its own starting length, so it drains only
    /// the temporaries it created; an enclosing statement's temporary stays
    /// pending until that statement ends. A block that already left through a
    /// terminator cannot run the drain: the releases were copied onto that
    /// edge, and the pending list stays for the statement's other paths.
    fn drain_pending_releases(&mut self, floor: usize) {
        if self.is_terminated() {
            return;
        }
        while self.pending_releases.len() > floor {
            let release = self.pending_releases.pop().expect("pending release");
            self.release_temp(release);
        }
    }

    /// The pending releases, newest first, as a copy.
    ///
    /// An edge that leaves the statement early carries a copy of the pending
    /// releases; the pending list itself stays for the statement's other
    /// paths, and the null-safe drops make the overlap harmless.
    fn pending_release_copies(&self) -> Vec<Release> {
        self.pending_releases.iter().rev().cloned().collect()
    }

    /// Emits the pending releases on an edge that leaves early.
    fn emit_pending_release_copies(&mut self) {
        for release in self.pending_release_copies() {
            self.release_temp(release);
        }
    }

    /// Emits one statement-temporary release and records it for the liveness
    /// pass.
    ///
    /// The release event names the temporary as an owner, so a view derived
    /// from it is rejected when a later use has no release-free path.
    fn release_temp(&mut self, release: Release) {
        let span = self.current_span;
        self.record_event(
            LoanEventKind::Release {
                owner: LoanOwner::Temp(release.temp),
            },
            span,
        );
        self.emit(Statement::ReleaseTemp(release));
    }

    /// Records the release events a return's copied release list states.
    ///
    /// The return terminator carries the releases to the emitter; the liveness
    /// pass needs the same facts at the return's position, before the view use
    /// a returned temporary derivation would record.
    fn record_release_events(&mut self, releases: &[Release]) {
        let span = self.current_span;
        for release in releases {
            self.record_event(
                LoanEventKind::Release {
                    owner: LoanOwner::Temp(release.temp),
                },
                span,
            );
        }
    }

    /// Makes a block the current one.
    fn switch_to(&mut self, block: BlockId) {
        self.builder.switch_to(block);
    }

    /// Ends the current block with a terminator, carrying the current span.
    fn terminate(&mut self, terminator: Terminator) {
        self.builder.set_span(self.current_span);
        self.builder.terminate(terminator);
    }

    /// True when the current block already has a real exit.
    fn is_terminated(&self) -> bool {
        !matches!(
            self.builder.function().blocks[self.builder.current()].terminator,
            Terminator::Unreachable
        )
    }

    /// Emits the cleanup of every open scope, innermost first, then returns.
    ///
    /// Each scope the edge leaves gets a `ScopeExit` marker, so the emitter
    /// runs that scope's cleanup even when the scope owns only a `defer`, and
    /// then the scope's drops and defers run in reverse registration order.
    /// The deferred effects are checked but do not change the return value,
    /// which was already evaluated.
    fn emit_return_cleanup(&mut self) {
        let depths: Vec<usize> = (1..=self.scopes.len()).rev().collect();
        for depth in depths {
            let Some(scope) = self.scopes.get(depth - 1) else {
                continue;
            };
            let scope_id = scope.id;
            self.emit(Statement::ScopeExit { scope: scope_id });
            self.run_scope_cleanup(depth - 1);
        }
    }
}

/// One cleanup action of a scope being left.
///
/// A scope's bindings and its deferred blocks share one registration sequence;
/// the actions are sorted newest first so the checker records their effects in
/// the same order the emitter runs them.
enum CleanupAction<'a> {
    /// Release a binding and emit its drop.
    Release(usize),
    /// Check a deferred block on the exit edge, identified by its registration
    /// so a running check can be recognized.
    Defer(&'a Block, u32),
}

impl PlaceTarget {
    /// Replaces the projections of a target.
    fn with_projections(self, projections: Vec<ir::Projection>) -> Self {
        match self {
            Self::Local(id, _) => Self::Local(id, projections),
            Self::Global(id, _) => Self::Global(id, projections),
        }
    }
}

/// True when an expression names a place without evaluating a call or literal.
///
/// The structural `len` fold reads no storage, so it can drop a fixed-array
/// place without evaluating anything. A call, index, or literal may have
/// effects or own a temporary, so the fold leaves it to the ordinary lowering.
fn is_pure_place(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Name(_) | ExprKind::Qualified(_) => true,
        ExprKind::Member { base, .. } => is_pure_place(base),
        // A constant-index read evaluates no call of its own; a run-time
        // index may, so it stays out of the structural fold.
        ExprKind::Index { base, index } => {
            is_pure_place(base) && matches!(index.kind, ExprKind::Int(_) | ExprKind::Char(_))
        }
        _ => false,
    }
}

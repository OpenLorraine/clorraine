//! Module-global initializer dependency cycles.
//!
//! A module-level `let` is initialized at run time, in an order that respects
//! the reads between globals. Its initializer may read another global, either
//! in the same module or through an import; a closed chain of such reads has no
//! valid initialization order, so it is the `E11100` cycle. Only the direct
//! reads of the initializer expression are edges: a call to a function that
//! happens to read a global is not, because the function body runs later.
//!
//! This pass runs before the per-module type checks so the cycle is reported
//! once, with a source location, before any later stage would have to order the
//! initializers.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, ExprKind, Item, QualifiedName};
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{DeclKind, Loaded, Module};
use crate::span::Span;

/// A runtime-initialized module global: its dotted module and its name.
type Key = (String, String);

/// One global's initializer: where it is written and which globals it reads.
struct Node {
    /// Span of the initializer expression, used for the diagnostic location.
    span: Span,
    /// The globals this initializer reads directly.
    reads: Vec<Key>,
}

/// The depth-first state of one global while cycles are sought.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Not visited yet.
    New,
    /// On the current search path.
    Active,
    /// Fully explored.
    Done,
}

/// Reports `E11100` for every cycle among runtime-initialized module globals.
pub fn check(loaded: &Loaded, diagnostics: &mut Diagnostics) {
    let graph = build(loaded);
    let mut state: BTreeMap<Key, State> = BTreeMap::new();
    let mut reported: BTreeSet<Key> = BTreeSet::new();
    let keys: Vec<Key> = graph.keys().cloned().collect();
    for key in keys {
        if state.get(&key).copied().unwrap_or(State::New) == State::New {
            let mut stack: Vec<Key> = Vec::new();
            visit(&key, &graph, &mut state, &mut stack, &mut reported, diagnostics);
        }
    }
}

/// Walks one global and its reads, reporting a cycle when a read closes one.
fn visit(
    key: &Key,
    graph: &BTreeMap<Key, Node>,
    state: &mut BTreeMap<Key, State>,
    stack: &mut Vec<Key>,
    reported: &mut BTreeSet<Key>,
    diagnostics: &mut Diagnostics,
) {
    state.insert(key.clone(), State::Active);
    stack.push(key.clone());
    if let Some(node) = graph.get(key) {
        for next in &node.reads {
            if !graph.contains_key(next) {
                continue;
            }
            match state.get(next).copied().unwrap_or(State::New) {
                State::New => visit(next, graph, state, stack, reported, diagnostics),
                State::Active => {
                    // A read back to a global still on the search path closes a
                    // cycle: the part of the path from that global to here.
                    if let Some(start) = stack.iter().position(|item| item == next) {
                        let cycle: Vec<Key> = stack[start..].to_vec();
                        report(&cycle, node.span, reported, diagnostics);
                    }
                }
                State::Done => {}
            }
        }
    }
    stack.pop();
    state.insert(key.clone(), State::Done);
}

/// Reports one cycle, naming the modules it spans, unless it was already part
/// of a reported cycle.
fn report(cycle: &[Key], span: Span, reported: &mut BTreeSet<Key>, diagnostics: &mut Diagnostics) {
    if cycle.iter().any(|key| reported.contains(key)) {
        return;
    }
    let modules: BTreeSet<&str> = cycle.iter().map(|(module, _)| module.as_str()).collect();
    let names = modules.into_iter().collect::<Vec<_>>().join(", ");
    diagnostics.push(Diagnostic::error(
        "E11100",
        format!("initializer cycle between modules: {names}"),
        span,
    ));
    for key in cycle {
        reported.insert(key.clone());
    }
}

/// Builds the read graph over every runtime-initialized module global.
fn build(loaded: &Loaded) -> BTreeMap<Key, Node> {
    let mut graph = BTreeMap::new();
    for module in loaded.modules.values() {
        for item in &module.items {
            let Item::GlobalLet(decl) = item else {
                continue;
            };
            let mut reads = Vec::new();
            collect(loaded, module, &decl.value, &mut reads);
            graph.insert(
                (module.name.clone(), decl.name.clone()),
                Node {
                    span: decl.value.span,
                    reads,
                },
            );
        }
    }
    graph
}

/// Collects the globals one initializer expression reads directly.
fn collect(loaded: &Loaded, module: &Module, expr: &Expr, reads: &mut Vec<Key>) {
    match &expr.kind {
        ExprKind::Name(name) => {
            if let Some(key) = global_named(loaded, module, name) {
                reads.push(key);
            }
        }
        ExprKind::Qualified(name) => {
            if let Some(key) = qualified_global(loaded, module, name) {
                reads.push(key);
            }
        }
        ExprKind::Call { callee, args, .. } => {
            collect(loaded, module, callee, reads);
            for arg in args {
                collect(loaded, module, &arg.value, reads);
            }
        }
        ExprKind::Index { base, index } => {
            collect(loaded, module, base, reads);
            collect(loaded, module, index, reads);
        }
        ExprKind::Member { base, .. } => collect(loaded, module, base, reads),
        ExprKind::Unary { operand, .. } => collect(loaded, module, operand, reads),
        ExprKind::Binary { left, right, .. } => {
            collect(loaded, module, left, reads);
            collect(loaded, module, right, reads);
        }
        ExprKind::Assign { target, value, .. } => {
            collect(loaded, module, target, reads);
            collect(loaded, module, value, reads);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            collect(loaded, module, condition, reads);
            collect(loaded, module, then_value, reads);
            collect(loaded, module, else_value, reads);
        }
        ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
            collect(loaded, module, value, reads)
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect(loaded, module, &field.value, reads);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                collect(loaded, module, element, reads);
            }
            if let Some(fill) = fill {
                collect(loaded, module, fill, reads);
            }
        }
        ExprKind::Unsafe(inner) => collect(loaded, module, inner, reads),
        ExprKind::IntrinsicCall { args, .. } => {
            for arg in args {
                if let crate::ast::IntrinsicArg::Expr(inner) = arg {
                    collect(loaded, module, inner, reads);
                }
            }
        }
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let crate::ast::TemplatePiece::Placeholder(inner) = piece {
                    collect(loaded, module, inner, reads);
                }
            }
        }
        _ => {}
    }
}

/// Resolves a bare name in an initializer to a module global, if it is one.
///
/// A name declared in the module itself wins; otherwise an imported binding
/// names the global in the module it was imported from.
fn global_named(loaded: &Loaded, module: &Module, name: &str) -> Option<Key> {
    if let Some(decl) = module.decl(name) {
        return (decl.kind == DeclKind::Global).then(|| (module.name.clone(), name.to_owned()));
    }
    let binding = module.import(name)?;
    target_global(loaded, &binding.module, &binding.original)
}

/// Resolves a qualified access `Module.name` in an initializer to a global.
fn qualified_global(loaded: &Loaded, module: &Module, name: &QualifiedName) -> Option<Key> {
    let first = name.segments.first()?;
    let last = name.segments.last()?;
    let binding = module.import(first)?;
    target_global(loaded, &binding.module, last)
}

/// The key of a global `name` declared in module `module`, when it is one.
fn target_global(loaded: &Loaded, module: &str, name: &str) -> Option<Key> {
    let node = loaded.module(module)?;
    match node.decl(name) {
        Some(decl) if decl.kind == DeclKind::Global => Some((module.to_owned(), name.to_owned())),
        _ => None,
    }
}

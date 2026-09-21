//! Module-global initializer dependency cycles.
//!
//! A module-level `let` is initialized at run time, in an order that respects
//! the reads between globals. Its initializer may read another global, either
//! in the same module or through an import; a closed chain of such reads has no
//! valid initialization order, so it is the `E11100` cycle. Only the direct
//! reads of the initializer expression are edges: a call to a function that
//! happens to read a global is not, because the function body runs later.
//!
//! A module dependency cycle is the second form: a module initializes after
//! every module it imports and every module its initializer expressions name,
//! so a closed chain of modules has no valid order. That cycle is an error only
//! when it involves at least one runtime `let`; a cycle of `const`-only modules
//! is allowed because it adds no runtime step.
//!
//! Both graphs come from [`crate::init_graph`], so this pass and the emitter
//! that orders the initializers cannot disagree about the dependencies.

use std::collections::{BTreeMap, BTreeSet};

use crate::diag::{Diagnostic, Diagnostics};
use crate::init_graph::{self, GlobalKey};
use crate::load::Loaded;
use crate::span::Span;

/// One global's initializer: where it is written and which globals it reads.
struct Node {
    /// Span of the initializer expression, used for the diagnostic location.
    span: Span,
    /// The other globals in the same module this initializer reads.
    reads: Vec<GlobalKey>,
}

/// The depth-first state of one node while cycles are sought.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Not visited yet.
    New,
    /// On the current search path.
    Active,
    /// Fully explored.
    Done,
}

/// Reports `E11100` for every initializer dependency cycle.
pub fn check(loaded: &Loaded, diagnostics: &mut Diagnostics) {
    check_global_cycles(loaded, diagnostics);
    check_module_cycles(loaded, diagnostics);
}

/// Reports a cycle among the runtime globals of a single module.
fn check_global_cycles(loaded: &Loaded, diagnostics: &mut Diagnostics) {
    let graph = build_global_graph(loaded);
    let mut state: BTreeMap<GlobalKey, State> = BTreeMap::new();
    let mut reported: BTreeSet<GlobalKey> = BTreeSet::new();
    let keys: Vec<GlobalKey> = graph.keys().cloned().collect();
    for key in keys {
        if state.get(&key).copied().unwrap_or(State::New) == State::New {
            let mut stack: Vec<GlobalKey> = Vec::new();
            visit_global(&key, &graph, &mut state, &mut stack, &mut reported, diagnostics);
        }
    }
}

/// Walks one global and its reads, reporting a cycle when a read closes one.
fn visit_global(
    key: &GlobalKey,
    graph: &BTreeMap<GlobalKey, Node>,
    state: &mut BTreeMap<GlobalKey, State>,
    stack: &mut Vec<GlobalKey>,
    reported: &mut BTreeSet<GlobalKey>,
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
                State::New => visit_global(next, graph, state, stack, reported, diagnostics),
                State::Active => {
                    // A read back to a global still on the search path closes a
                    // cycle: the part of the path from that global to here.
                    if let Some(start) = stack.iter().position(|item| item == next) {
                        let cycle: Vec<GlobalKey> = stack[start..].to_vec();
                        report_global_cycle(&cycle, node.span, reported, diagnostics);
                    }
                }
                State::Done => {}
            }
        }
    }
    stack.pop();
    state.insert(key.clone(), State::Done);
}

/// Reports a same-module cycle unless one of its globals was already reported.
fn report_global_cycle(
    cycle: &[GlobalKey],
    span: Span,
    reported: &mut BTreeSet<GlobalKey>,
    diagnostics: &mut Diagnostics,
) {
    if cycle.iter().any(|key| reported.contains(key)) {
        return;
    }
    let modules: BTreeSet<String> = cycle.iter().map(|(module, _)| module.clone()).collect();
    report(&modules, span, diagnostics);
    for key in cycle {
        reported.insert(key.clone());
    }
}

/// Builds the same-module read graph over every runtime-initialized global.
fn build_global_graph(loaded: &Loaded) -> BTreeMap<GlobalKey, Node> {
    let mut graph = BTreeMap::new();
    for module in loaded.modules.values() {
        for item in &module.items {
            let crate::ast::Item::GlobalLet(decl) = item else {
                continue;
            };
            // A cross-module read is part of the module dependency relation,
            // which is checked separately, so only the same-module reads create
            // a global edge here. Reporting both would duplicate one cycle.
            let reads = init_graph::global_reads(loaded, module, &decl.value)
                .into_iter()
                .filter(|(read_module, _)| read_module == &module.name)
                .collect();
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

/// Reports a cycle in the module dependency relation that involves runtime work.
fn check_module_cycles(loaded: &Loaded, diagnostics: &mut Diagnostics) {
    let deps = init_graph::module_dependencies(loaded);
    let mut state: BTreeMap<String, State> = BTreeMap::new();
    let mut reported: BTreeSet<String> = BTreeSet::new();
    let names: Vec<String> = deps.keys().cloned().collect();
    for name in names {
        if state.get(&name).copied().unwrap_or(State::New) == State::New {
            let mut stack: Vec<String> = Vec::new();
            visit_module(
                &name,
                &deps,
                &mut state,
                &mut stack,
                &mut reported,
                loaded,
                diagnostics,
            );
        }
    }
}

/// Walks one module and its dependencies, reporting a cycle when one closes.
fn visit_module(
    name: &str,
    deps: &BTreeMap<String, BTreeSet<String>>,
    state: &mut BTreeMap<String, State>,
    stack: &mut Vec<String>,
    reported: &mut BTreeSet<String>,
    loaded: &Loaded,
    diagnostics: &mut Diagnostics,
) {
    state.insert(name.to_owned(), State::Active);
    stack.push(name.to_owned());
    if let Some(nexts) = deps.get(name) {
        for next in nexts {
            match state.get(next).copied().unwrap_or(State::New) {
                State::New => visit_module(
                    next, deps, state, stack, reported, loaded, diagnostics,
                ),
                State::Active => {
                    if let Some(start) = stack.iter().position(|item| item == next) {
                        let cycle: Vec<String> = stack[start..].to_vec();
                        report_module_cycle(&cycle, loaded, reported, diagnostics);
                    }
                }
                State::Done => {}
            }
        }
    }
    stack.pop();
    state.insert(name.to_owned(), State::Done);
}

/// Reports a module cycle, unless it is `const`-only or already reported.
///
/// A cycle of `const`-only modules is allowed: it adds no runtime step, so
/// there is no order to satisfy. A cycle that includes at least one runtime
/// `let` has no valid initialization order, so it is the `E11100` error.
fn report_module_cycle(
    cycle: &[String],
    loaded: &Loaded,
    reported: &mut BTreeSet<String>,
    diagnostics: &mut Diagnostics,
) {
    if cycle.iter().any(|name| reported.contains(name)) {
        return;
    }
    let runtime: Option<&String> = cycle
        .iter()
        .filter(|name| init_graph::has_runtime_global(loaded, name))
        .min();
    let Some(runtime) = runtime else {
        return;
    };
    let Some(span) = init_graph::runtime_span(loaded, runtime) else {
        return;
    };
    let modules: BTreeSet<String> = cycle.iter().cloned().collect();
    report(&modules, span, diagnostics);
    for name in cycle {
        reported.insert(name.clone());
    }
}

/// Reports one cycle, naming the modules it spans.
fn report(modules: &BTreeSet<String>, span: Span, diagnostics: &mut Diagnostics) {
    let names = modules.iter().cloned().collect::<Vec<_>>().join(", ");
    diagnostics.push(Diagnostic::error(
        "E11100",
        format!("initializer cycle between modules: {names}"),
        span,
    ));
}

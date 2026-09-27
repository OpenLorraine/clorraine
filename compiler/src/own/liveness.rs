//! Non-lexical lifetime analysis for the ownership checker.
//!
//! The ownership walk records an ordered log of the facts that decide the view
//! rules: loan creations, view uses, owner accesses, loan requests, owner
//! releases, and the accesses a deferred block makes on its exit edges. After
//! the walk this module turns that log into last-use regions over the emitted
//! control-flow graph. A loan is live at a point when its creation reaches the
//! point and the point can still reach one of the loan's uses, so a view that
//! is bound and never used blocks nothing.
//!
//! The four borrow diagnostics are decided against the regions instead of a
//! lexical scope: a move through a live loan is `E10022`, a write through a
//! live shared loan is `E10031`, a loan request that conflicts with a live loan
//! is `E10032`, and a use that no release-free path can reach is `E10030`.

use std::collections::{BTreeMap, BTreeSet};

use crate::borrow::LoanKind;
use crate::diag::{Diagnostic, Diagnostics};
use crate::ir::{self, BlockId, Terminator};
use crate::span::Span;

/// Identity of one loan inside one function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LoanId(pub(super) usize);

/// What makes a loan's identity stable across every pass over a function.
///
/// A loan is created by one source derivation, so its owner, permission, and
/// source range are the same in a speculative pass and on the reporting pass.
/// Keying by that origin means a loan created inside a loop body or a deferred
/// block keeps one id everywhere it is seen, which is what lets a use recorded
/// at the loop head, in the loop condition, or after the owner's scope resolve
/// to the loan the reporting pass created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LoanOrigin {
    /// Index of the binding that owns the storage.
    pub(super) owner: usize,
    /// The permission the loan grants.
    pub(super) kind: LoanKind,
    /// Start byte offset of the derivation that created the loan.
    pub(super) start: u32,
    /// End byte offset of the derivation that created the loan.
    pub(super) end: u32,
}

impl LoanOrigin {
    /// The origin of a loan created by a derivation at `span`.
    pub(super) fn new(owner: usize, kind: LoanKind, span: Span) -> Self {
        Self {
            owner,
            kind,
            start: span.start,
            end: span.end,
        }
    }
}

/// A point in the finished function's event sequence.
///
/// A position is a block and an ordinal inside that block's recorded events.
/// Two events never share a position, so a position addresses one access point
/// on the control-flow graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Position {
    /// The block the event was recorded in.
    pub(super) block: BlockId,
    /// The event's ordinal inside that block's recorded events.
    pub(super) index: usize,
}

/// What one recorded event states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LoanEventKind {
    /// A view derivation created a persistent loan of `owner`.
    Created {
        /// The new loan.
        loan: LoanId,
        /// Index of the binding that owns the storage.
        owner: usize,
        /// The permission the loan grants.
        kind: LoanKind,
    },
    /// A view value that carries `loan` was read, written through, derived
    /// from, passed, or returned.
    Used {
        /// The loan the value carries.
        loan: LoanId,
    },
    /// A binding that owns storage was written or moved.
    Access {
        /// Index of the owner binding.
        owner: usize,
        /// How the owner was accessed.
        access: AccessKind,
    },
    /// A persistent loan was created, or a `borrow` argument lent an owner.
    Request {
        /// Index of the owner binding.
        owner: usize,
        /// The permission the new loan would grant.
        kind: LoanKind,
    },
    /// An owner's storage stopped existing on this edge.
    Release {
        /// Index of the owner binding.
        owner: usize,
    },
}

/// How an owner binding was accessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AccessKind {
    /// The value was written in place.
    Write,
    /// The value was moved out.
    Move,
}

/// One recorded fact at one position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LoanEvent {
    /// Where the event runs on the control-flow graph.
    pub(super) position: Position,
    /// Source span of the access that produced the event.
    pub(super) span: Span,
    /// What the event states.
    pub(super) kind: LoanEventKind,
}

/// The successor blocks of one block, from its terminator.
///
/// A `switch` lists every arm and its default but never the `join` block,
/// because other terminators already jump to the join; `return` and an
/// unreachable terminator have no successor.
pub(super) fn successors(function: &ir::Function) -> Vec<Vec<BlockId>> {
    function
        .blocks
        .iter()
        .map(|block| terminator_successors(&block.terminator))
        .collect()
}

/// The blocks a terminator can transfer control to.
fn terminator_successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Goto(block) => vec![*block],
        Terminator::Branch {
            on_true, on_false, ..
        } => vec![*on_true, *on_false],
        Terminator::Switch { arms, default, .. } => {
            let mut out: Vec<BlockId> = arms.iter().map(|(_, block)| *block).collect();
            out.push(*default);
            out
        }
        Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
    }
}

/// Runs the liveness pass over one finished function and reports its findings.
///
/// `events` is the walk's log for this function, `names` maps a binding index
/// to its written name for diagnostics, and every reported error keeps the
/// version 1 code, message, help, and span.
pub(super) fn check(
    function: &ir::Function,
    events: &[LoanEvent],
    names: &BTreeMap<usize, String>,
    diagnostics: &mut Diagnostics,
) {
    if events.is_empty() {
        return;
    }
    let analysis = Analysis::build(&successors(function), events);
    let live = analysis.regions();
    analysis.report(events, &live, names, diagnostics);
}

/// The reachability facts the rules are decided against.
///
/// Events are grouped into positions and the position graph follows the
/// function's control flow. The live region of a loan is computed from this
/// graph by [`Analysis::regions`].
struct Analysis {
    /// Every position in the log, ordered by block and ordinal.
    positions: Vec<Position>,
    /// The position index of each position.
    position_index: BTreeMap<Position, usize>,
    /// The owner binding of each loan.
    owners: BTreeMap<LoanId, usize>,
    /// The permission each loan grants.
    kinds: BTreeMap<LoanId, LoanKind>,
    /// The creation span of each loan.
    spans: BTreeMap<LoanId, Span>,
    /// The creation positions of each loan.
    creations: BTreeMap<LoanId, BTreeSet<Position>>,
    /// The use positions of each loan.
    uses: BTreeMap<LoanId, BTreeSet<Position>>,
    /// The release positions of each owner.
    releases: BTreeMap<usize, BTreeSet<Position>>,
    /// Forward edges of the position graph.
    adjacency: Vec<Vec<usize>>,
    /// Backward edges of the position graph.
    reverse: Vec<Vec<usize>>,
}

impl Analysis {
    /// Builds the position graph and the loan facts from a recorded log.
    fn build(successors: &[Vec<BlockId>], events: &[LoanEvent]) -> Self {
        let positions = collect_positions(events);
        let mut by_block: BTreeMap<BlockId, Vec<usize>> = BTreeMap::new();
        for (rank, position) in positions.iter().enumerate() {
            by_block.entry(position.block).or_default().push(rank);
        }
        let position_index: BTreeMap<Position, usize> = positions
            .iter()
            .enumerate()
            .map(|(rank, position)| (*position, rank))
            .collect();
        let (adjacency, reverse) = build_edges(&positions, successors, &by_block);
        let facts = collect_facts(events, &position_index);
        Self {
            positions,
            position_index,
            owners: facts.owners,
            kinds: facts.kinds,
            spans: facts.spans,
            creations: facts.creations,
            uses: facts.uses,
            releases: facts.releases,
            adjacency,
            reverse,
        }
    }

    /// The live region of every loan, as the positions where it is live.
    ///
    /// Exposed so the region computation can be driven directly on a
    /// hand-built log and control-flow graph.
    fn regions(&self) -> BTreeMap<LoanId, BTreeSet<Position>> {
        let mut live = BTreeMap::new();
        for (loan, creations) in &self.creations {
            let region = match self.uses.get(loan) {
                Some(uses) if !uses.is_empty() => {
                    let forward = closure(
                        &self.adjacency,
                        &to_indices(creations, &self.position_index),
                    );
                    let backward = closure(
                        &self.reverse,
                        &to_indices(uses, &self.position_index),
                    );
                    forward
                        .intersection(&backward)
                        .copied()
                        .map(|rank| self.positions[rank])
                        .collect()
                }
                _ => BTreeSet::new(),
            };
            live.insert(*loan, region);
        }
        live
    }

    /// Reports every conflict and every loan that outlives its owner.
    fn report(
        &self,
        events: &[LoanEvent],
        live: &BTreeMap<LoanId, BTreeSet<Position>>,
        names: &BTreeMap<usize, String>,
        diagnostics: &mut Diagnostics,
    ) {
        for event in events {
            match event.kind {
                LoanEventKind::Access { owner, access } => {
                    if self.access_conflicts(owner, access, live, event.position) {
                        let (code, message, help) = match access {
                            AccessKind::Move => (
                                "E10022",
                                format!("cannot move '{}' while it is borrowed", display(owner, names)),
                                "end the borrow first",
                            ),
                            AccessKind::Write => (
                                "E10031",
                                format!(
                                    "cannot modify '{}' while it is shared-borrowed",
                                    display(owner, names)
                                ),
                                "end the shared borrow first",
                            ),
                        };
                        push(diagnostics, code, message, help, event.span);
                    }
                }
                LoanEventKind::Request { owner, kind } => {
                    if self.request_conflicts(owner, kind, live, event.position) {
                        push(
                            diagnostics,
                            "E10032",
                            format!("'{}' is already borrowed", display(owner, names)),
                            "end the first borrow first",
                            event.span,
                        );
                    }
                }
                _ => {}
            }
        }
        for (loan, owner) in &self.owners {
            if !self.escapes_owner(loan, *owner) {
                continue;
            }
            let span = self.spans.get(loan).copied().unwrap_or_default();
            push(
                diagnostics,
                "E10030",
                format!("borrow of '{}' does not live long enough", display(*owner, names)),
                "finish the use before the owner's scope ends",
                span,
            );
        }
    }

    /// True when a use of the loan has no path from its creation that avoids a
    /// release of the owner.
    ///
    /// Every path from the creation to such a use crosses the point where the
    /// owner's storage dies, so the use reads freed storage. Requiring the
    /// release-free path — not merely the existence of one release before a use
    /// — keeps a loop-local owner safe: the next iteration's use is reached
    /// from a fresh creation before the release, so it stays valid.
    fn escapes_owner(&self, loan: &LoanId, owner: usize) -> bool {
        let Some(uses) = self.uses.get(loan) else {
            return false;
        };
        if uses.is_empty() {
            return false;
        }
        let Some(creations) = self.creations.get(loan) else {
            return false;
        };
        let blocked: BTreeSet<usize> = self
            .releases
            .get(&owner)
            .map(|releases| to_indices(releases, &self.position_index))
            .unwrap_or_default();
        let safe = closure_avoiding(
            &self.adjacency,
            &to_indices(creations, &self.position_index),
            &blocked,
        );
        uses.iter()
            .filter_map(|position| self.position_index.get(position))
            .any(|rank| !safe.contains(rank))
    }

    /// True when an owner access hits a live loan at one position.
    fn access_conflicts(
        &self,
        owner: usize,
        access: AccessKind,
        live: &BTreeMap<LoanId, BTreeSet<Position>>,
        position: Position,
    ) -> bool {
        live.iter().any(|(loan, region)| {
            self.owners.get(loan) == Some(&owner)
                && region.contains(&position)
                && match access {
                    AccessKind::Move => true,
                    AccessKind::Write => self.kinds.get(loan) == Some(&LoanKind::Shared),
                }
        })
    }

    /// True when a loan request hits a live loan at one position.
    fn request_conflicts(
        &self,
        owner: usize,
        kind: LoanKind,
        live: &BTreeMap<LoanId, BTreeSet<Position>>,
        position: Position,
    ) -> bool {
        live.iter().any(|(loan, region)| {
            self.owners.get(loan) == Some(&owner)
                && region.contains(&position)
                && (self.kinds.get(loan) == Some(&LoanKind::Exclusive)
                    || kind == LoanKind::Exclusive)
        })
    }
}

/// The positions of the recorded events, ordered by block and ordinal.
fn collect_positions(events: &[LoanEvent]) -> Vec<Position> {
    let mut all: Vec<Position> = events.iter().map(|event| event.position).collect();
    all.sort_by_key(|position| (position.block, position.index));
    all
}

/// Builds the forward and backward edges of the position graph.
fn build_edges(
    positions: &[Position],
    successors: &[Vec<BlockId>],
    by_block: &BTreeMap<BlockId, Vec<usize>>,
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let mut adjacency = vec![Vec::new(); positions.len()];
    for (rank, position) in positions.iter().enumerate() {
        let block_events = by_block.get(&position.block).expect("recorded block");
        let next = block_events
            .iter()
            .copied()
            .find(|other| positions[*other].index == position.index + 1);
        match next {
            Some(other) => adjacency[rank].push(other),
            None => {
                let mut visited = vec![false; successors.len()];
                let mut targets = Vec::new();
                for successor in &successors[position.block] {
                    append_entry(*successor, successors, by_block, &mut visited, &mut targets);
                }
                adjacency[rank] = targets;
            }
        }
    }
    let mut reverse = vec![Vec::new(); positions.len()];
    for (rank, targets) in adjacency.iter().enumerate() {
        for &target in targets {
            reverse[target].push(rank);
        }
    }
    (adjacency, reverse)
}

/// Adds the first event of `block`, walking through empty blocks on the way.
fn append_entry(
    block: BlockId,
    successors: &[Vec<BlockId>],
    by_block: &BTreeMap<BlockId, Vec<usize>>,
    visited: &mut [bool],
    out: &mut Vec<usize>,
) {
    if visited[block] {
        return;
    }
    visited[block] = true;
    if let Some(events) = by_block.get(&block) {
        if let Some(&first) = events.first() {
            out.push(first);
        }
        return;
    }
    for successor in &successors[block] {
        append_entry(*successor, successors, by_block, visited, out);
    }
}

/// The loan facts the region computation and the reports need.
struct Facts {
    /// The owner binding of each loan.
    owners: BTreeMap<LoanId, usize>,
    /// The permission each loan grants.
    kinds: BTreeMap<LoanId, LoanKind>,
    /// The creation span of each loan.
    spans: BTreeMap<LoanId, Span>,
    /// The creation positions of each loan.
    creations: BTreeMap<LoanId, BTreeSet<Position>>,
    /// The use positions of each loan.
    uses: BTreeMap<LoanId, BTreeSet<Position>>,
    /// The release positions of each owner.
    releases: BTreeMap<usize, BTreeSet<Position>>,
}

/// Collects the loan identity and access facts from the recorded log.
fn collect_facts(events: &[LoanEvent], position_index: &BTreeMap<Position, usize>) -> Facts {
    let mut facts = Facts {
        owners: BTreeMap::new(),
        kinds: BTreeMap::new(),
        spans: BTreeMap::new(),
        creations: BTreeMap::new(),
        uses: BTreeMap::new(),
        releases: BTreeMap::new(),
    };
    for event in events {
        if !position_index.contains_key(&event.position) {
            continue;
        }
        let position = event.position;
        match event.kind {
            LoanEventKind::Created { loan, owner, kind } => {
                facts.owners.insert(loan, owner);
                facts.kinds.insert(loan, kind);
                facts.spans.insert(loan, event.span);
                facts.creations.entry(loan).or_default().insert(position);
            }
            LoanEventKind::Used { loan } => {
                facts.uses.entry(loan).or_default().insert(position);
            }
            LoanEventKind::Release { owner } => {
                facts.releases.entry(owner).or_default().insert(position);
            }
            _ => {}
        }
    }
    facts
}

/// Converts a set of positions into their position indices.
fn to_indices(
    positions: &BTreeSet<Position>,
    position_index: &BTreeMap<Position, usize>,
) -> BTreeSet<usize> {
    positions
        .iter()
        .filter_map(|position| position_index.get(position).copied())
        .collect()
}

/// Every position reachable from `seeds`, including the seeds themselves.
fn closure(edges: &[Vec<usize>], seeds: &BTreeSet<usize>) -> BTreeSet<usize> {
    let mut seen = seeds.clone();
    let mut stack: Vec<usize> = seeds.iter().copied().collect();
    while let Some(position) = stack.pop() {
        for &next in &edges[position] {
            if seen.insert(next) {
                stack.push(next);
            }
        }
    }
    seen
}

/// Every position reachable from `seeds` without leaving a `blocked` position.
///
/// A blocked position is reachable itself, but control does not continue out of
/// it, which models storage that has died on the way through it.
fn closure_avoiding(
    edges: &[Vec<usize>],
    seeds: &BTreeSet<usize>,
    blocked: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let mut seen = seeds.clone();
    let mut stack: Vec<usize> = seeds.iter().copied().collect();
    while let Some(position) = stack.pop() {
        if blocked.contains(&position) {
            continue;
        }
        for &next in &edges[position] {
            if seen.insert(next) {
                stack.push(next);
            }
        }
    }
    seen
}

/// The written name of a binding, for a diagnostic message.
fn display(owner: usize, names: &BTreeMap<usize, String>) -> String {
    names
        .get(&owner)
        .cloned()
        .unwrap_or_else(|| String::from("?"))
}

/// Pushes one error diagnostic at a source span.
fn push(diagnostics: &mut Diagnostics, code: &'static str, message: String, help: &str, span: Span) {
    diagnostics.push(Diagnostic::error(code, message, span).with_help(help));
}

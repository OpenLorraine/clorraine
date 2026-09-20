//! Structured-control metadata over the flat block graph.
//!
//! The ownership fixpoint needs a flat block graph, but the C emitter must
//! print structured statements, because the target language forbids `goto`
//! outside the cleanup ladder. These types record the C shape a branch came
//! from so the emitter can follow it without recovering structure from the
//! graph.

use super::BlockId;

/// The structured shape a two-way branch came from, when it has one.
///
/// The checker records this while it lowers a statement so the emitter can
/// print C `if` and loop forms instead of a label and a `goto`. The block graph
/// is unchanged: the shape is metadata over the same successors, so the
/// ownership fixpoint still sees the flat graph. A branch the checker cannot
/// describe as a C control-flow form is [`BranchShape::Plain`], and the emitter
/// falls back to a label and a `goto` for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchShape {
    /// An `if`/`else` whose branches reconverge at `join`.
    If {
        /// The block both branches continue at after the `if`.
        join: BlockId,
    },
    /// A loop whose body is `body` and whose exit is `exit`.
    Loop {
        /// Which loop form to print.
        kind: LoopKind,
        /// The first block of the loop body.
        body: BlockId,
        /// The block reached when the loop ends.
        exit: BlockId,
    },
    /// A branch with no structured form; emit a label plus `goto`.
    Plain,
}

/// The C loop form a structured loop branch lowers to.
///
/// The kind tells the emitter where the loop test runs: a `while` tests before
/// the body, a `do ... while` tests after it, and a three-clause `for` tests in
/// its header before running its step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopKind {
    /// A `while` loop, whose test runs before the body.
    While,
    /// A `do ... while` loop, whose test runs after the body.
    DoWhile,
    /// A three-clause `for` loop.
    For,
}

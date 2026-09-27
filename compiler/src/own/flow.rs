//! Flow-state values for the ownership walk.
//!
//! The checker tracks, for every binding, whether its value was moved or is
//! partially moved, whether it is initialized (as a whole or field by field),
//! and which projection paths were narrowed against `null`. Loops and
//! `switch`es open an exit frame so a `break` or `continue` records the state
//! on its edge. Keeping these plain values separate from the walk keeps the
//! module that drives the walk focused on the rules.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Block, Mode};
use crate::borrow::Provenance;
use crate::ir::{self, BlockId};
use crate::span::Span;
use crate::types::Ty;

use super::liveness::LoanId;

/// What a binding is: a local, a parameter, or a `borrow T?` out-parameter.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BindingKind {
    /// A local `let` or `const`.
    Local,
    /// A parameter that lends a value.
    Param,
    /// A `borrow T?` out-parameter slot, which owns its incoming value.
    OutSlot,
}

/// One declared binding of the function currently being checked.
pub(super) struct Binding {
    /// The written name.
    pub(super) name: String,
    /// The resolved type.
    pub(super) ty: Ty,
    /// Source span of the declared name, used to report an unused local.
    pub(super) span: Span,
    /// True once the binding is read, written, or moved. A local that stays
    /// false through the whole function is reported as unused.
    pub(super) used: bool,
    /// The scope depth the binding was declared in.
    pub(super) scope: usize,
    /// What kind of binding it is.
    pub(super) kind: BindingKind,
    /// The written parameter mode, or no mode for a local.
    pub(super) mode: Mode,
    /// True for a `const` local.
    pub(super) is_const: bool,
    /// True when the binding owns a value that needs a drop.
    pub(super) needs_drop: bool,
    /// Registration order of the binding, so its drop interleaves with the
    /// scope's `defer` entries in one reverse cleanup order.
    pub(super) registration: u32,
    /// For a view binding, where its storage comes from.
    pub(super) provenance: Provenance,
    /// An explicit place base that overrides the binding's usual local or
    /// parameter spelling.
    ///
    /// A lifted lambda binds its captures here, rooted at an environment field
    /// instead of a C local. Every other binding leaves this `None`.
    pub(super) base: Option<ir::PlaceBase>,
}

/// One lexical scope and the bindings declared in it.
#[derive(Clone, Default)]
pub(super) struct Scope {
    /// The scope's id in the lowered function.
    pub(super) id: ir::ScopeId,
    /// Names bound here, innermost wins.
    pub(super) names: BTreeMap<String, usize>,
    /// Binding indices in declaration order, for reverse drop emission.
    pub(super) order: Vec<usize>,
}

/// The flow-sensitive state of one place.
///
/// Initialization is tracked both for the whole binding and per top-level
/// field, so a struct that is written field by field is usable for its
/// initialized fields but never as a whole. Narrowing is tracked per projection
/// path, so a nullable field can be narrowed without narrowing the owner.
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct PlaceState {
    /// The value was moved out and not re-initialized.
    pub(super) moved: bool,
    /// Top-level fields moved out, which makes whole use invalid.
    pub(super) partial: BTreeSet<String>,
    /// The binding is assigned on all paths reaching this point.
    pub(super) initialized: bool,
    /// Top-level fields written on all paths reaching this point.
    pub(super) initialized_fields: BTreeSet<String>,
    /// Projection paths proved not null on all paths reaching this point. The
    /// empty path means the whole binding.
    pub(super) narrowed: BTreeSet<Vec<ir::Projection>>,
}

impl PlaceState {
    /// True when the whole binding is initialized or the top-level field is.
    pub(super) fn field_initialized(&self, field: &str) -> bool {
        self.initialized || self.initialized_fields.contains(field)
    }

    /// True when the projection path is known to be not null.
    pub(super) fn is_narrowed(&self, projections: &[ir::Projection]) -> bool {
        self.narrowed.contains(projections)
    }
}

/// The flow state of one function at one program point.
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct FlowState {
    /// Per-binding place state.
    pub(super) places: BTreeMap<usize, PlaceState>,
    /// The loans each view binding may carry at this point.
    ///
    /// A loan lives here rather than on the binding so it is flow-sensitive: a
    /// view assigned one owner on one path and another owner on another path
    /// carries both loans after the join, and a use reaches both regions. The
    /// set is empty or absent for a binding that carries no loan.
    pub(super) loans: BTreeMap<usize, BTreeSet<LoanId>>,
}

impl FlowState {
    /// Reads the state of a place, defaulting to "uninitialized and unmoved".
    pub(super) fn place(&self, id: usize) -> PlaceState {
        self.places.get(&id).cloned().unwrap_or_default()
    }
}

/// The target of an assignment or a move.
pub(super) enum PlaceTarget {
    /// A local or parameter with its projections.
    Local(usize, Vec<ir::Projection>),
    /// A module-level binding with its identity and projections.
    Global(ir::GlobalId, Vec<ir::Projection>),
}

/// The write-back of one `for (borrow e in xs)` element loop.
///
/// A `borrow` element is an exclusive view of an iterable element, so its final
/// value is copied back into the iterable at the end of the iteration. Recording
/// the write-back on the loop's exit frame lets a `break` or `continue` run it
/// too, instead of silently losing the element's last write.
#[derive(Clone)]
pub(super) struct ForInWrite {
    /// The element type.
    pub(super) element: Ty,
    /// The iterable being walked.
    pub(super) iterable: ir::Value,
    /// The current index into the iterable.
    pub(super) index: ir::Value,
    /// The element binding whose value is copied back.
    pub(super) id: usize,
}

/// One open `break`/`continue` target: a loop or a `switch`.
///
/// The frame joins the state of every edge seen so far, so the loop exit or
/// the code after a `switch` sees the conservative union of the paths that
/// actually leave it.
pub(super) struct ExitFrame {
    /// The block a `break` jumps to.
    pub(super) break_block: BlockId,
    /// The block a `continue` jumps to, or `None` inside a `switch`.
    pub(super) continue_block: Option<BlockId>,
    /// Scope depth a `break` must not clean: scopes at or below it stay.
    pub(super) scope_floor: usize,
    /// Scope depth a `continue` must not clean.
    pub(super) continue_floor: usize,
    /// Joined state of every `break` edge.
    pub(super) break_state: FlowState,
    /// True once a `break` edge was seen.
    pub(super) saw_break: bool,
    /// The element write-back of an element loop, when this frame is one.
    pub(super) for_in: Option<ForInWrite>,
}

impl ExitFrame {
    /// A frame that no edge has reached yet.
    pub(super) fn new(
        break_block: BlockId,
        continue_block: Option<BlockId>,
        scope_floor: usize,
        continue_floor: usize,
    ) -> Self {
        Self {
            break_block,
            continue_block,
            scope_floor,
            continue_floor,
            break_state: FlowState::default(),
            saw_break: false,
            for_in: None,
        }
    }

    /// Records a `break` edge leaving the frame with `state`.
    pub(super) fn record_break(&mut self, state: FlowState) {
        if self.saw_break {
            self.break_state = super::FunctionChecker::merge(self.break_state.clone(), state);
        } else {
            self.break_state = state;
            self.saw_break = true;
        }
    }
}

/// A deferred block waiting to be checked on the scope's exit edges.
pub(super) struct DeferEntry<'a> {
    /// Depth of the scope that registered the defer.
    pub(super) depth: usize,
    /// The deferred statements.
    pub(super) block: &'a Block,
    /// Registration order shared with the scope's bindings, so the checker can
    /// interleave the scope's drops and defers exactly as the emitter runs
    /// them: one reverse registration sequence.
    pub(super) registration: u32,
}

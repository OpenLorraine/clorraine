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

use super::liveness::{LoanId, LoanOwner};

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
    /// The unique IR spelling of the binding.
    ///
    /// The first binding with a given source name keeps that name; a later
    /// binding that shadows it appends `#<index>`. The spelling is
    /// deterministic and reaches the C emitter through `PlaceBase::Local` and
    /// `Local::name`; diagnostics keep the source name. A base-overridden
    /// binding (a capture) keeps the source name and is exempt: its places
    /// root at the override and it never reaches `Function.locals`.
    pub(super) ir_name: String,
    /// The resolved type.
    pub(super) ty: Ty,
    /// Source span of the declared name, used to report an unused local.
    pub(super) span: Span,
    /// True once the binding is read, written, or moved. A local that stays
    /// false through the whole function is reported as unused.
    pub(super) used: bool,
    /// The scope depth the binding was declared in.
    pub(super) scope: usize,
    /// The source range where the binding's scope closes.
    ///
    /// A stored view backed by this binding dies when its scope closes; the
    /// opaque-store hooks compare a later read's span with it.
    pub(super) scope_end: Span,
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
    /// instead of a C local. A `for (borrow e in xs)` element binds here too,
    /// rooted at the temporary that holds the element's address. Every other
    /// binding leaves this `None`.
    pub(super) base: Option<ir::PlaceBase>,
    /// The storage a `for (borrow e in xs)` element alias writes.
    ///
    /// The alias has no place of its own: every read and write reaches the
    /// iterable element, so a store through the binding is judged at this
    /// container and its loan belongs to the container. The chain is resolved
    /// when the alias is bound, so a nested alias names the outermost
    /// container. A non-alias binding leaves this `None`.
    pub(super) alias_root: Option<PlaceTarget>,
    /// True for a compiler-generated owner binding (`__cl_ownedN`).
    ///
    /// A generated owner belongs to the compiler, not to a source name: a view
    /// returned through one dies with the function and reports like a local
    /// view, so the escape checks recognize it without matching on the
    /// generated spelling.
    pub(super) generated: bool,
    /// True when a lifted lambda's capture points at storage that dies with
    /// the enclosing frame.
    ///
    /// A capture has no declaration of its own: it aliases the enclosing
    /// binding named in the capture list. The lifted body judges an escape to
    /// the enclosing caller, so the captured binding's lifespan decides
    /// whether the escape is frame-backed. Every non-capture binding leaves
    /// this false.
    pub(super) capture_dies_with_frame: bool,
    /// True when a lifted lambda's capture names a caller slot.
    ///
    /// A write through a captured out-slot or writable parameter leaves the
    /// enclosing frame; a write through a captured local stays in the same
    /// frame. The lifted body cannot tell the origin from the capture mode
    /// alone, so the enclosing checker records the destination's frame here.
    pub(super) capture_caller_slot: bool,
    /// True when a lambda wrote through a capture of this binding.
    ///
    /// The closure body is checked separately, so the enclosing frame cannot
    /// see which view the write stored. The binding is treated as possibly
    /// holding frame-backed storage afterwards: a later escape rejects, while a
    /// read inside the frame stays accepted.
    pub(super) capture_written: bool,
}

/// One lexical scope and the bindings declared in it.
#[derive(Clone, Default)]
pub(super) struct Scope {
    /// The scope's id in the lowered function.
    pub(super) id: ir::ScopeId,
    /// The source range of the point that closes the scope.
    pub(super) close_span: Span,
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
    /// Projection paths consumed by a store whose type may own at an
    /// instantiation: an owning instance moves the path, a duplicable one
    /// copies. The empty path means the whole binding. A later use records the
    /// duplicability duty the per-instance audit discharges.
    pub(super) moved_if_owning: BTreeSet<Vec<ir::Projection>>,
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
    /// The element loans a container carries, keyed by the target path.
    ///
    /// A view element store gives the container one loan per backing, exactly
    /// as a whole store does. Overwriting the element replaces that element's
    /// loans, because the old view value is gone; keeping them would report a
    /// stale conflict on the old backing. A path with a run-time index keeps
    /// the union, because the overwritten element cannot be named.
    pub(super) element_loans: BTreeMap<usize, BTreeMap<Vec<ir::Projection>, BTreeSet<LoanId>>>,
    /// The adopted view backings each binding's value carries.
    ///
    /// A carrier binding — a struct or array that holds views in its view-like
    /// positions — keeps the generated-owner and temporary backings those
    /// positions read, so a later return of the binding records their loans at
    /// the return and the escape rule reports them. The map is flow-sensitive
    /// the same way the view loans are: a branch that stores a carrier and one
    /// that does not join to the union.
    pub(super) carried_owners: BTreeMap<usize, BTreeSet<LoanOwner>>,
    /// The source paths a definite write re-initialized on every path here.
    ///
    /// An opaque store may move a local source at an owning instance. A later
    /// write that covers the source makes a subsequent read legal, but only
    /// when the write is definite: a branch that re-initializes on one path
    /// must not hide the other path's use. A set of covered write paths joins
    /// by intersection, so only an all-path write clears the state.
    pub(super) reinitialized_sources: BTreeSet<(usize, Vec<ir::Projection>)>,
}

impl FlowState {
    /// Reads the state of a place, defaulting to "uninitialized and unmoved".
    pub(super) fn place(&self, id: usize) -> PlaceState {
        self.places.get(&id).cloned().unwrap_or_default()
    }

    /// Removes the place and loan state of every binding at or above `floor`.
    ///
    /// A speculative pass or a deferred check may materialize a compiler-owned
    /// temporary with an id above the bindings the caller saved. Its binding is
    /// truncated from the binding table, so its state must not linger and be
    /// inherited by a later local that reuses the id.
    pub(super) fn drop_bindings_from(&mut self, floor: usize) {
        self.places.retain(|id, _| *id < floor);
        self.loans.retain(|id, _| *id < floor);
        self.element_loans.retain(|id, _| *id < floor);
        self.carried_owners.retain(|id, _| *id < floor);
    }
}

/// Where a struct literal's view fields land.
///
/// The literal's own storage is materialized in the destination's frame, so
/// the destination decides which backings a view field may keep: a frame
/// destination keeps this frame's and the caller's storage, a returned value
/// keeps the caller's parameters while this frame dies, and every other
/// destination keeps module storage alone.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LiteralViews {
    /// A destination this frame keeps: a local binding or element.
    Frame,
    /// The value returned to the caller.
    Return,
    /// Any other destination: a global, a field, or an argument.
    Escaping,
}

/// The target of an assignment or a move.
#[derive(Clone, Debug)]
pub(super) enum PlaceTarget {
    /// A local or parameter with its projections.
    Local(usize, Vec<ir::Projection>),
    /// A module-level binding with its identity and projections.
    Global(ir::GlobalId, Vec<ir::Projection>),
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

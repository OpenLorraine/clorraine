//! Places: the memory locations the program reads, writes, moves, and drops.
//!
//! A place is a base plus zero or more projections, plus the type of the value
//! it addresses. `x` is a local base with no projection, `p.left` is a local
//! base with a field projection, and `xs[0]` is a local base with an element
//! projection.

use crate::types::Ty;

use super::{GlobalId, TempId};

/// A memory location the program can read, write, move, or drop.
///
/// A place is a base plus zero or more projections, plus the type of the value
/// it addresses. `x` is a local base with no projection, `p.left` is a local
/// base with a field projection, and `xs[0]` is a local base with an element
/// projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// The root of the place: a local, a parameter, a global, or a temporary.
    pub base: PlaceBase,
    /// Field and element selections applied from the base outwards.
    pub projections: Vec<Projection>,
    /// The checked type of the value the place addresses.
    pub ty: Ty,
}

impl Place {
    /// A place that names a plain local, with an as-yet unknown type.
    pub fn local(name: impl Into<String>) -> Self {
        Self::with_base(PlaceBase::Local(name.into()))
    }

    /// A place that names a parameter, with an as-yet unknown type.
    pub fn param(name: impl Into<String>) -> Self {
        Self::with_base(PlaceBase::Param(name.into()))
    }

    /// A place that names a module-level binding, with an unknown type.
    pub fn global(id: GlobalId) -> Self {
        Self::with_base(PlaceBase::Global(id))
    }

    /// A place with no projections and an unknown type.
    fn with_base(base: PlaceBase) -> Self {
        Self {
            base,
            projections: Vec::new(),
            ty: Ty::unknown(),
        }
    }

    /// Attaches the checked type to the place.
    pub fn with_type(mut self, ty: Ty) -> Self {
        self.ty = ty;
        self
    }

    /// True when the place names a top-level local or parameter.
    pub fn is_plain(&self) -> bool {
        self.projections.is_empty()
    }
}

/// What a place is rooted in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceBase {
    /// A local binding.
    Local(String),
    /// A function parameter.
    Param(String),
    /// A module-level `let` or `const`, with its declaring module.
    Global(GlobalId),
    /// A compiler-created temporary.
    Temp(TempId),
}

/// One selection applied to a place.
///
/// The ordering is total so a set of projection paths can be kept sorted; the
/// order itself has no meaning beyond giving narrowing facts a stable key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Projection {
    /// A named struct field.
    Field(String),
    /// An array or slice element; `None` is a run-time index.
    Element(Option<usize>),
}

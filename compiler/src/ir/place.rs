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

    /// A place rooted at a lifted lambda's environment field.
    ///
    /// The type is carried on both the place and the base so the emitter can
    /// spell a projected field without a separate environment lookup.
    pub fn env(field: impl Into<String>, by_pointer: bool, ty: Ty) -> Self {
        Self {
            base: PlaceBase::EnvField {
                field: field.into(),
                by_pointer,
                ty: ty.clone(),
            },
            projections: Vec::new(),
            ty,
        }
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
    /// A binding captured by a lifted lambda, reached through the environment.
    ///
    /// The lifted function receives the environment as a `struct ClEnv_<id> *`
    /// parameter named `__env`. A by-value field is read as `(env->field)`; a
    /// pointer field, used for a borrowed capture, is dereferenced as
    /// `(*env->field)` so reads and writes reach the capturing frame.
    EnvField {
        /// The environment struct field name.
        field: String,
        /// True when the field holds a pointer to the captured place.
        by_pointer: bool,
        /// The captured value's checked type.
        ty: Ty,
    },
    /// A place reached by dereferencing a compiler temporary that holds its
    /// address.
    ///
    /// A `for (borrow e in xs)` element is an alias of the iterable element: a
    /// per-iteration temporary holds the element's address and every use of
    /// the binding goes through it, so a write reaches the iterable
    /// immediately and the assignment rules apply to the element itself.
    DerefTemp {
        /// The temporary holding the element address.
        temp: TempId,
        /// The type of the value the address points at.
        ty: Ty,
    },
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

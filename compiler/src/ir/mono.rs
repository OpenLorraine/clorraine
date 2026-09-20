//! Generic instantiation and copy obligations.
//!
//! A generic body is checked once and monomorphized: codegen emits one copy per
//! distinct instantiation. This module names those copies and the copy
//! obligations a generic body owes, so codegen can consume them without reading
//! the source tree again.

/// One concrete instantiation of a generic function.
///
/// A generic body is checked once; every call that instantiates it produces an
/// instance, and codegen emits one copy per distinct instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    /// The generic function's name.
    pub function: String,
    /// The concrete type arguments, in declaration order.
    pub args: Vec<String>,
}

impl Instance {
    /// The key that identifies this instance uniquely.
    ///
    /// Two calls with the same function and the same type arguments share one
    /// emitted copy, so the key is the deduplication identity codegen uses.
    pub fn dedupe_key(&self) -> String {
        format!("{}<{}>", self.function, self.args.join(", "))
    }
}

/// A copy a generic body must perform at each instantiation.
///
/// A writable lend parameter instantiated with a duplicable type binds a copy
/// instead of a lend, so the body records the obligation here and the checker
/// discharges it against each concrete instance. The instance key names the
/// instantiation site so a diagnostic can point at it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyObligation {
    /// The generic function that carries the obligation.
    pub function: String,
    /// The instantiation the obligation was discharged for.
    pub instance: String,
    /// The parameter whose value must be copied.
    pub parameter: String,
    /// The concrete duplicable type bound at the instantiation.
    pub ty: String,
}

/// The instantiation set and copy obligations of a generic program.
///
/// This is the input codegen consumes: the set of concrete function copies to
/// emit, each with a stable deduplication key, plus the copies a generic body
/// owes. The type arguments are stored as their rendered spellings because the
/// emitter names symbols from them.
#[derive(Clone, Debug, Default)]
pub struct Monomorphization {
    instances: Vec<Instance>,
    obligations: Vec<CopyObligation>,
}

impl Monomorphization {
    /// Records an instance, returning true when it was not already present.
    pub fn record(&mut self, instance: Instance) -> bool {
        if self
            .instances
            .iter()
            .any(|known| known.dedupe_key() == instance.dedupe_key())
        {
            return false;
        }
        self.instances.push(instance);
        true
    }

    /// Every distinct instance recorded so far.
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    /// Records a copy obligation the generic body owes.
    pub fn require_copy(&mut self, obligation: CopyObligation) {
        self.obligations.push(obligation);
    }

    /// Every copy obligation recorded so far.
    pub fn obligations(&self) -> &[CopyObligation] {
        &self.obligations
    }
}

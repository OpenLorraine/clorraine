//! Generic instantiation and copy obligations.
//!
//! A generic body is checked once and monomorphized: codegen emits one copy per
//! distinct instantiation. This module names those copies and the copy
//! obligations a generic body owes, so codegen can consume them without reading
//! the source tree again.

use crate::span::Span;
use crate::types::{Ty, TyKind};

/// One concrete instantiation of a generic function.
///
/// A generic body is checked once; every call that instantiates it produces an
/// instance, and codegen emits one copy per distinct instance.
///
/// The type arguments are checked types rather than rendered spellings so
/// codegen can substitute them into the body's parameter and local types. A
/// function-typed argument such as `func(i32): i32` is a signature that a
/// string spelling alone cannot rebuild.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    /// The generic function's name.
    pub function: String,
    /// The concrete type arguments, in declaration order.
    pub args: Vec<Ty>,
}

impl Instance {
    /// The key that identifies this instance uniquely.
    ///
    /// Two calls with the same function and the same type arguments share one
    /// emitted copy, so the key is the deduplication identity codegen uses.
    pub fn dedupe_key(&self) -> String {
        let rendered: Vec<String> = self.args.iter().map(dedupe_arg).collect();
        format!("{}<{}>", self.function, rendered.join(", "))
    }
}

/// Renders a type argument so a closure site is part of the key.
///
/// A closure type's rendered signature alone does not identify its site, so two
/// lambdas with the same signature would share a key. The site id is included
/// here so distinct closures stay distinct.
fn dedupe_arg(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Closure(closure) => format!("closure_{}", closure.id),
        _ => ty.display(),
    }
}

/// One concrete instantiation of a generic struct.
///
/// A generic struct's fields are laid out per distinct argument list, so each
/// instantiation has its own emitted C struct. The instance records the struct
/// name and its concrete type arguments so codegen can emit and deduplicate it.
///
/// The arguments are kept as checked types rather than rendered spellings
/// because the emitter substitutes them into the declaration's field types; a
/// nested argument such as `Pair<i32>` is a structured type that a string
/// spelling alone cannot reproduce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructInstance {
    /// The struct's name as declared.
    pub name: String,
    /// The concrete type arguments, in declaration order.
    pub args: Vec<Ty>,
}

impl StructInstance {
    /// The key that identifies this instance uniquely.
    ///
    /// Two uses with the same struct and the same type arguments share one
    /// emitted type, so the key is the deduplication identity codegen uses.
    pub fn dedupe_key(&self) -> String {
        let rendered: Vec<String> = self.args.iter().map(Ty::display).collect();
        format!("{}<{}>", self.name, rendered.join(", "))
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

/// A later use that forces a maybe-owning store to stay a copy.
///
/// A generic body that stores a type parameter's value into a destination and
/// then uses the source again is honest only when the instance's type argument
/// is duplicable: an owning argument moves the source, so the later use would
/// read a moved value. The checker records the duty at the conflicting use and
/// the per-instance audit reports `E10020` when the substituted type is not
/// duplicable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresDuplicable {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The unique IR spelling of the binding whose later use conflicts.
    pub binding: String,
    /// The conflicting use, for the body-site diagnostic.
    pub span: Span,
}

/// The instantiation set and copy obligations of a generic program.
///
/// This is the input codegen consumes: the set of concrete function copies to
/// emit, each with a stable deduplication key, the concrete generic structs to
/// emit, and the copies a generic body owes. A function instance's type
/// arguments are stored as their rendered spellings because the emitter names
/// symbols from them; a struct instance keeps the checked types so the emitter
/// can substitute them into the struct's fields.
#[derive(Clone, Debug, Default)]
pub struct Monomorphization {
    instances: Vec<Instance>,
    structs: Vec<StructInstance>,
    obligations: Vec<CopyObligation>,
    duplicable_requirements: Vec<RequiresDuplicable>,
}

impl Monomorphization {
    /// Records an instance, returning true when it was not already present.
    pub fn record(&mut self, instance: Instance) -> bool {
        if self.instances.iter().any(|known| known == &instance) {
            return false;
        }
        self.instances.push(instance);
        true
    }

    /// Every distinct instance recorded so far.
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    /// Records a struct instance, returning true when it was not already
    /// present.
    pub fn record_struct(&mut self, instance: StructInstance) -> bool {
        if self.structs.iter().any(|known| known == &instance) {
            return false;
        }
        self.structs.push(instance);
        true
    }

    /// Every distinct struct instance recorded so far.
    pub fn struct_instances(&self) -> &[StructInstance] {
        &self.structs
    }

    /// Records a copy obligation the generic body owes.
    pub fn require_copy(&mut self, obligation: CopyObligation) {
        self.obligations.push(obligation);
    }

    /// Every copy obligation recorded so far.
    pub fn obligations(&self) -> &[CopyObligation] {
        &self.obligations
    }

    /// Records a later-use duplicability duty the generic body owes.
    pub fn require_duplicable(&mut self, requirement: RequiresDuplicable) {
        self.duplicable_requirements.push(requirement);
    }

    /// Every duplicability duty recorded so far.
    pub fn duplicable_requirements(&self) -> &[RequiresDuplicable] {
        &self.duplicable_requirements
    }
}

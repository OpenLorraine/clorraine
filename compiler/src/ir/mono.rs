//! Generic instantiation and copy obligations.
//!
//! A generic body is checked once and monomorphized: codegen emits one copy per
//! distinct instantiation. This module names those copies and the copy
//! obligations a generic body owes, so codegen can consume them without reading
//! the source tree again.

use crate::span::Span;
use crate::types::{PlacePathStep, Ty, TyKind};

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
/// here so distinct closures stay distinct, and a closure specialized for an
/// enclosing instance carries that instance's key too, so two enclosing
/// specializations of one site stay distinct as well.
fn dedupe_arg(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Closure(closure) => crate::types::closure_key(closure),
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
/// the per-instance audit reports the concrete spelling's diagnostic when the
/// substituted type is not duplicable: `E10020` for a whole-binding move, and
/// `E10025` when only a member path moved and the use names the whole binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresDuplicable {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The unique IR spelling of the binding whose later use conflicts.
    pub binding: String,
    /// The binding's declared type, substituted per instance by the audit.
    ///
    /// A base-overridden binding (a for-in borrow element or a capture) has no
    /// local entry to look up, so the record carries the type itself.
    pub ty: Ty,
    /// True when every conflicting maybe-owning move is a member path and the
    /// conflicting use names the whole binding.
    ///
    /// The concrete spelling reports that whole-binding use of a partially
    /// moved value as `E10025`; a whole-binding move keeps `E10020`.
    pub partial: bool,
    /// The conflicting moved paths at this later use, one per maybe-moved
    /// place the use can observe.
    ///
    /// The audit substitutes the root and projects each moved path so a
    /// duplicable member of an owning generic root stays a copy. An empty path
    /// is a whole-binding move and judges the whole substituted type.
    pub paths: Vec<Vec<crate::types::PlacePathStep>>,
    /// The conflicting use, for the body-site diagnostic.
    pub span: Span,
}

/// A value-consuming move through a borrowed element view at the body site.
///
/// A `for (borrow e in xs)` element aliases the iterable's element, so moving
/// its value or a member out of it is the v1 element-move limit in the
/// concrete spelling. While the element's type is still a parameter the
/// checker cannot see whether the moved place owns, so it records this duty;
/// the per-instance audit substitutes the binding's type, projects the
/// recorded path, and reports the same limit once the moved place owns. A
/// duplicable or view-typed substitution copies and stays accepted, exactly
/// as the concrete spelling does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresElementMove {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The borrowed element binding's declared type, substituted per instance.
    ///
    /// The binding is a loop element alias, which has no local entry to look
    /// up, so the record carries the type itself.
    pub ty: Ty,
    /// The projection path from the binding to the moved place.
    pub path: Vec<PlacePathStep>,
    /// The move site, for the diagnostic span.
    pub span: Span,
}

/// A body's obligation that a lend may only be moved into an owned position
/// when the instance's type is duplicable.
///
/// A concrete lend parameter that is returned, stored, or passed to a `take`
/// position reports `E10034` directly. While the type is still a parameter the
/// checker takes the maybe-owning branch instead, so the body records this duty
/// and the per-instance audit reports the same diagnostic once the type
/// argument is known to own. A duplicable substitution is accepted, matching
/// the concrete copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresNotLend {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The lend parameter's source name, for the diagnostic text.
    pub parameter: String,
    /// The moved value's declared root type, substituted per instance by the
    /// audit.
    pub ty: Ty,
    /// The projection from the root to the moved value, applied after the
    /// substitution so a duplicable member of an owning lend stays a copy.
    pub projections: Vec<super::Projection>,
    /// True when the moved source is a writable lend (`borrow`) parameter.
    ///
    /// Only a writable lend can be the `borrow T?` out-parameter slot that
    /// the concrete rules let a nullable value move out of; a read-only
    /// (by-value or `const`) lend may never be emptied, so its duty stays
    /// strict at every substitution.
    pub writable_source: bool,
    /// True when the move's destination is a writable parameter slot.
    ///
    /// Such a store is the `borrow T?` out-slot carve-out when the instance's
    /// type argument makes the slot nullable, so the audit exempts a nullable
    /// substitution only for these duties. Every other destination — a local,
    /// a return, a `take` position — keeps the strict judgment, because a
    /// nullable lend moved into an owned position would leave the source and
    /// the destination aliasing one allocation.
    pub stores_into_slot: bool,
    /// The move site, for the diagnostic span.
    pub span: Span,
}

/// One element loop whose iterable type still mentions a type parameter.
///
/// The declaration-site check accepts an opaque iterable because its instance
/// may be a fixed array or a slice. The per-instance audit substitutes each
/// recorded instance's type arguments: a substitution that is not an array
/// reports `E10043`, and a non-borrow copy of a non-duplicable substituted
/// element reports `E10040`, exactly as the concrete spellings report. The
/// binding itself carries the substituted element through the element
/// placeholder, so an accepted instance emits the concrete element type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresIterable {
    /// The generic function whose body spells the loop.
    pub function: String,
    /// The iterable's declared type, still mentioning the parameter.
    pub ty: Ty,
    /// True when the loop binds a `borrow` element.
    pub borrow: bool,
    /// The loop's span, for the diagnostic.
    pub span: Span,
}

/// A body's obligation that a parameter-typed nullable borrow slot receives an
/// exact-typed place.
///
/// While the slot's type is still a parameter the call check cannot tell
/// whether the instance makes the slot nullable, so the body records this duty
/// and the per-instance audit applies the exact-place rule once the type
/// argument is known. A substitution that leaves the slot non-nullable is
/// accepted, matching the concrete by-value path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresOutSlotPlace {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The written argument's declared type, substituted per instance.
    pub argument: Ty,
    /// The slot parameter's declared type, substituted per instance.
    pub slot: Ty,
    /// True when the written argument is a place.
    pub place: bool,
    /// The call site, for the diagnostic span.
    pub span: Span,
}

/// A body's obligation that a parameter-typed lend slot accepts a store.
///
/// A concrete lend parameter that owns storage rejects a whole-value store,
/// because the store would release the caller's value. While the slot type is
/// still a parameter the checker records this duty instead, and the
/// per-instance audit reports the same diagnostic once a substitution owns the
/// slot. A nullable substitution is an out-parameter slot and stays exempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresStorableSlot {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The lend parameter's source name, for the diagnostic text.
    pub parameter: String,
    /// The slot's declared type, substituted per instance by the audit.
    pub ty: Ty,
    /// The store site, for the diagnostic span.
    pub span: Span,
}

/// A body's obligation that a whole-binding self-assign through a lend root is
/// judged per instance.
///
/// A concrete lend parameter that owns storage rejects `slot = slot` outright
/// — the store would release the caller's value and write the emptied source
/// back — and a concrete nullable out-slot spelling rejects it the same way.
/// While the slot's type is still a parameter the checker cannot tell which
/// rule applies, so the body records this duty and the per-instance audit
/// reports the concrete diagnostic: `E10020` for a nullable substitution and
/// `E10034` for an owning one, while a duplicable or view substitution stays
/// accepted, exactly as its concrete spelling does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresSelfAssign {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The lend parameter's source name, for the diagnostic text.
    pub parameter: String,
    /// The lend root's declared type, substituted per instance by the audit.
    pub ty: Ty,
    /// The store site, for the diagnostic span.
    pub span: Span,
}

/// A store into a type-parameter member whose instance decides its kind.
///
/// The declaration sees only `T`, so it cannot tell whether the selected member
/// holds a view or an owner. The body records the store and the audit
/// substitutes the root: a view destination accepts the value with the ordinary
/// instance conversion and reports the frame escape when the carrier is
/// returned, while an owned destination keeps the concrete implicit-conversion
/// judgment. The record carries the root binding only while the body is being
/// checked; the audit needs the declared type, the path, and the source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresOpaqueStore {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The unique IR spelling of the destination root binding.
    ///
    /// The return hook compares it with the returned place's binding. The
    /// per-instance audit does not read it.
    pub root: String,
    /// The destination root's declared type, substituted per instance.
    pub ty: Ty,
    /// The projection from the root to the stored member.
    pub path: Vec<super::Projection>,
    /// The stored value's declared type, substituted per instance.
    pub value: Ty,
    /// The frame-dying backing the stored view reads.
    ///
    /// `None` when the source is caller- or module-backed and the view
    /// survives the frame. `Some(subject)` carries the diagnostic subject,
    /// already quoted for a binding, and `Some("a statement temporary")` for a
    /// value the walk cannot name.
    pub frame_owner: Option<String>,
    /// Every frame-dying backing the stored value may read.
    ///
    /// A conditional contributes one frame per arm, so a later read after any
    /// of them dies is judged. A frame with no binding names a temporary.
    pub frames: Vec<OpaqueFrame>,
    /// The owned source binding whose move waits for the instance.
    ///
    /// A store of an owned place into an opaque member moves the source at an
    /// owned destination and widens it at a view destination. The body records
    /// the source and its later reads; the audit reports the concrete
    /// moved-value error only when the instance's destination owns.
    pub source_ir: Option<String>,
    /// The owned source binding's source name, for the moved-value report.
    pub source_name: Option<String>,
    /// The projection from the owned source binding to the stored value.
    pub source_path: Vec<super::Projection>,
    /// True when the owned source is a non-take parameter.
    ///
    /// Such a source cannot be moved by the concrete rule; the audit reports
    /// `E10034` at the store when the destination owns.
    pub source_param: bool,
    /// Reads of the owned source after the store, for the owned instance.
    pub later_uses: Vec<Span>,
    /// Reads of the destination after a backing's scope ends.
    pub late_uses: Vec<(Span, String)>,
    /// Writes or moves of a backing after the store.
    pub invalidations: Vec<(Span, String)>,
    /// Moves of a backing after the store, for the borrow-conflict report.
    pub moved_owner: Vec<(Span, String)>,
    /// True when a later store replaced this one's view before any read.
    ///
    /// A superseded duty keeps its owned-source judgment and any use already
    /// recorded before the overwrite, but its future view watches end.
    pub superseded: bool,
    /// True when the destination root is a slot the caller keeps.
    ///
    /// A stored frame-backed view in a caller slot escapes at the store; the
    /// audit reports it there rather than waiting for a return.
    pub caller_slot: bool,
    /// The store site, for the conversion diagnostic.
    pub store_span: Span,
    /// The return site once the destination root is returned.
    pub return_span: Option<Span>,
}

/// One frame-dying backing a stored view reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpaqueFrame {
    /// The backing binding's unique IR spelling, when it names one.
    pub binding: Option<String>,
    /// The backing binding's source name, for the conflict report.
    pub name: Option<String>,
    /// The diagnostic subject, already quoted for a binding.
    pub subject: String,
    /// The source range where the binding's scope closes.
    pub scope_end: Span,
}

/// One emitted specialization of a lifted lambda.
///
/// A lifted function inside a generic declaration is emitted once per enclosing
/// instance. The site names the lifted function and the type arguments are the
/// enclosing instance's arguments, so the emitter can install the same
/// substitution it installs for the declared instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiftedSpecialization {
    /// The lambda site the lifted function was created from.
    pub site: usize,
    /// The enclosing instance's concrete type arguments.
    pub args: Vec<Ty>,
}

/// The instantiation set and copy obligations of a generic program.
///
/// This is the input codegen consumes: the set of concrete function copies to
/// emit, each with a stable deduplication key, the concrete generic structs to
/// emit, the lifted-lambda specializations the generic bodies need, and the
/// copies a generic body owes. A function instance's type arguments are stored
/// as their rendered spellings because the emitter names symbols from them; a
/// struct instance keeps the checked types so the emitter can substitute them
/// into the struct's fields.
#[derive(Clone, Debug, Default)]
pub struct Monomorphization {
    instances: Vec<Instance>,
    structs: Vec<StructInstance>,
    lifted: Vec<LiftedSpecialization>,
    obligations: Vec<CopyObligation>,
    duplicable_requirements: Vec<RequiresDuplicable>,
    element_move_requirements: Vec<RequiresElementMove>,
    not_lend_requirements: Vec<RequiresNotLend>,
    out_slot_requirements: Vec<RequiresOutSlotPlace>,
    storable_slot_requirements: Vec<RequiresStorableSlot>,
    self_assign_requirements: Vec<RequiresSelfAssign>,
    opaque_store_requirements: Vec<RequiresOpaqueStore>,
    iterable_requirements: Vec<RequiresIterable>,
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

    /// Records a lifted-lambda specialization, returning true when it was not
    /// already present.
    pub fn record_lifted(&mut self, specialization: LiftedSpecialization) -> bool {
        if self.lifted.iter().any(|known| known == &specialization) {
            return false;
        }
        self.lifted.push(specialization);
        true
    }

    /// Every distinct lifted-lambda specialization recorded so far.
    pub fn lifted_specializations(&self) -> &[LiftedSpecialization] {
        &self.lifted
    }

    /// Records a loop over a still-opaque iterable.
    pub fn require_iterable(&mut self, requirement: RequiresIterable) {
        self.iterable_requirements.push(requirement);
    }

    /// Every recorded opaque-iterable duty.
    pub fn iterable_requirements(&self) -> &[RequiresIterable] {
        &self.iterable_requirements
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

    /// Records a borrowed-element move the body owes an instance judgment for.
    pub fn require_element_move(&mut self, requirement: RequiresElementMove) {
        self.element_move_requirements.push(requirement);
    }

    /// Every borrowed-element move duty recorded so far.
    pub fn element_move_requirements(&self) -> &[RequiresElementMove] {
        &self.element_move_requirements
    }

    /// Records a lend move the body owes an instance judgment for.
    pub fn require_not_lend(&mut self, requirement: RequiresNotLend) {
        self.not_lend_requirements.push(requirement);
    }

    /// Every lend-move duty recorded so far.
    pub fn not_lend_requirements(&self) -> &[RequiresNotLend] {
        &self.not_lend_requirements
    }

    /// Records an out-slot place duty the generic body owes.
    pub fn require_out_slot(&mut self, requirement: RequiresOutSlotPlace) {
        self.out_slot_requirements.push(requirement);
    }

    /// Every out-slot place duty recorded so far.
    pub fn out_slot_requirements(&self) -> &[RequiresOutSlotPlace] {
        &self.out_slot_requirements
    }

    /// Records a lend-slot store duty the generic body owes.
    pub fn require_storable_slot(&mut self, requirement: RequiresStorableSlot) {
        self.storable_slot_requirements.push(requirement);
    }

    /// Every lend-slot store duty recorded so far.
    pub fn storable_slot_requirements(&self) -> &[RequiresStorableSlot] {
        &self.storable_slot_requirements
    }

    /// Records a whole-binding self-assign duty the generic body owes.
    pub fn require_self_assign(&mut self, requirement: RequiresSelfAssign) {
        self.self_assign_requirements.push(requirement);
    }

    /// Every whole-binding self-assign duty recorded so far.
    pub fn self_assign_requirements(&self) -> &[RequiresSelfAssign] {
        &self.self_assign_requirements
    }

    /// Records a store into a type-parameter member.
    pub fn require_opaque_store(&mut self, requirement: RequiresOpaqueStore) {
        self.opaque_store_requirements.push(requirement);
    }

    /// Every opaque-member store duty recorded so far.
    pub fn opaque_store_requirements(&self) -> &[RequiresOpaqueStore] {
        &self.opaque_store_requirements
    }
}

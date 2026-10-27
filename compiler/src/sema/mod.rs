//! Expression and statement checking.
//!
//! The name stage already proved that every reference resolves; this stage
//! gives each expression and declaration a type and reports the type errors
//! the specification assigns to this layer. It is deliberately a separate
//! scoped walk rather than a symbol graph, because the name stage does not
//! build one.
//!
//! A type the checker cannot model — a standard-library type reached through
//! an import, for example — becomes the opaque `Unknown` type, which accepts
//! every operation. That keeps the partly-implemented standard library and the
//! staged features from cascading into false diagnostics while the modeled
//! subset is checked exactly.
//!
//! Only errors that have no earlier cause are reported: the driver runs this
//! stage after a clean name pass, so a file that already failed resolution
//! never reaches here.

use std::collections::{ BTreeMap, HashMap, BTreeSet };

use crate::ast::{Expr, ExprKind, FuncDecl, Item, ScalarType, Type};
use crate::consts::ConstFolder;
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{DeclKind, Loaded, Module};
use crate::profile::Profile;
use crate::span::Span;
use crate::types::{ resolve_ty_with, substitute_instance, AbiLayout, CaptureInfo, CaptureMode, ClosureSite, Ty, TyKind, TypeTable, PlacePathStep };

pub(crate) mod call;
mod expr;
mod init_cycle;
mod items;
mod ops;
mod stmt;

#[cfg(test)]
mod regressions;
#[cfg(test)]
mod init_tests;
#[cfg(test)]
mod tests;

/// What the type stage hands to the stage that follows it.
///
/// The program-wide type table is kept so a later stage does not rebuild it,
/// and `expr_types` maps every checked expression node to the type it was
/// given. The key is the address of the node in the loaded tree, which is
/// stable because the tree is borrowed immutably for the whole compilation;
/// this lets a later stage reuse the exact types this stage computed instead
/// of inferring them a second time.
pub struct SemaOutput {
    /// Every declared type of the program.
    pub table: TypeTable,
    /// The type of each expression node, keyed by the node's address.
    pub expr_types: HashMap<usize, Ty>,
    /// The captured bindings of each capturing lambda, keyed by its site id.
    ///
    /// The ownership and emission stages read this instead of re-deriving the
    /// capture list and its modes from the source.
    pub closures: BTreeMap<usize, ClosureSite>,
    /// Lambda bodies that return or consume a by-value capture whose type
    /// still mentions a type parameter.
    ///
    /// The declaration cannot decide such a move: the capture copies while
    /// its type is duplicable, but an owning substitution makes the copy
    /// alias the enclosing binding, so the per-instance audit applies the
    /// concrete judgment after substitution.
    pub capture_moves: Vec<CaptureMoveRequirement>,
    /// Value-form member reads whose base is a bare type parameter.
    ///
    /// A member of an opaque base is resolved after substitution: the instance
    /// decides whether the substituted type has the member, so the read stays
    /// accepted at the declaration and the per-instance audit reports the
    /// unknown member exactly as a concrete read would.
    pub opaque_projections: Vec<OpaqueProjection>,
    /// Index reads whose base is a bare type parameter, in check order.
    ///
    /// An index of an opaque base is judged after substitution: the instance
    /// decides the row's extent for a constant index and the run-time guard,
    /// so the read stays accepted at the declaration and the per-instance
    /// audit reports the out-of-range constant exactly as a concrete read
    /// would.
    pub opaque_indexes: Vec<OpaqueIndex>,
    /// Member reads on a bare type parameter used at a concrete destination.
    ///
    /// The declaration cannot judge the read's conversion because the member's
    /// type is unresolved there; the per-instance audit resolves the member
    /// through the matching [`OpaqueProjection`] and applies the concrete
    /// conversion rule.
    pub opaque_member_uses: Vec<OpaqueMemberUse>,
    /// Values whose conversion is deferred to their instance.
    ///
    /// The written source mentions a type parameter or is the null shape, so
    /// the declaration cannot judge the conversion; the per-instance audit
    /// resolves both sides and applies the ordinary conversion (or null)
    /// judgment.
    pub opaque_conversions: Vec<OpaqueConversion>,
    /// Conditionals whose arm agreement is deferred to their instance.
    ///
    /// An arm that still names a type parameter keeps the declaration's
    /// acceptance: the substitution decides whether the two arms agree and
    /// whether a null arm reaches a non-nullable context, so the per-instance
    /// audit re-applies the conditional rule to the resolved arms.
    pub opaque_conditionals: Vec<OpaqueConditional>,
}

/// A lambda body that returns or consumes a by-value capture whose
/// duplicability is still open.
pub struct CaptureMoveRequirement {
    /// The enclosing function's name.
    pub function: String,
    /// The captured binding's name.
    pub capture: String,
    /// The capture's declared type in terms of the function's type parameters.
    pub ty: Ty,
    /// The projection path from the capture to the moved place.
    ///
    /// An empty path is a whole-capture move; a path names the field or
    /// element the move selects, so the per-instance audit judges the moved
    /// place's substituted type and a duplicable projection stays a copy.
    pub path: Vec<PlacePathStep>,
    /// The store destination when the record comes from an assignment.
    ///
    /// A store only consumes its value when the destination owns it: a
    /// duplicable destination copies the value in — a view widening is the
    /// case — so the audit leaves those stores accepted. A return or a `take`
    /// argument always consumes, so its record carries no destination.
    pub dest: Option<Ty>,
    /// True when the moved capture is a borrowed one.
    ///
    /// Only a borrowed capture is moved out of the caller's storage, so only
    /// that mode is unsound at a non-nullable destination; a copy capture
    /// owns its value and follows the ordinary duplicability rule.
    pub borrow: bool,
    /// The returning or consuming site.
    pub span: Span,
}

/// One value-form member read on a bare type parameter.
///
/// The declaration cannot resolve the member: the substitution may be any
/// type, so the read records this duty and the per-instance audit resolves it
/// through the concrete type argument.
pub struct OpaqueProjection {
    /// The enclosing function's name.
    pub function: String,
    /// The base's declared type, still mentioning the parameter.
    pub ty: Ty,
    /// The member's spelling.
    pub name: String,
    /// The read's span, for the diagnostic.
    pub span: Span,
}

/// One value-form member read on a bare type parameter used at a destination.
///
/// The read's span identifies its recorded [`OpaqueProjection`], which carries
/// the base's declared type: the audit substitutes both and reports the frozen
/// conversion when the resolved member cannot reach the destination.
pub struct OpaqueMemberUse {
    /// The enclosing function's name.
    pub function: String,
    /// The member's spelling.
    pub name: String,
    /// The read's span, which also identifies its recorded projection.
    pub span: Span,
    /// The destination the read must convert to.
    pub expected: Ty,
    /// True when the read sits at a destination slot or a return, the only
    /// positions the frozen rules let the sanctioned view pointer conversion
    /// reach. Every other position reports the frozen conversion rule.
    pub sanctioned: bool,
    /// The written root binding name, when the read is rooted at one.
    ///
    /// The possible-null rule names the root binding as the concrete spelling
    /// does; a read rooted at a call or a literal has no name, so its null
    /// judgment stays with the conversion rule.
    pub base: Option<String>,
}

/// One value whose conversion is deferred to its instance.
///
/// The written source mentions a type parameter or is the null shape, so the
/// declaration cannot judge the conversion: the instance audit substitutes the
/// instance's arguments and applies the ordinary conversion (or null)
/// judgment to the resolved source and destination.
pub struct OpaqueConversion {
    /// The enclosing function's name.
    pub function: String,
    /// The written source's type.
    pub source: Ty,
    /// The destination the value must convert to.
    pub expected: Ty,
    /// The value's span.
    pub span: Span,
    /// The literal the source wrote, when it wrote one.
    ///
    /// A literal takes the destination's type exactly where its concrete twin
    /// converts, so the audit applies that judgment to the resolved
    /// destination.
    pub literal: Option<ConversionLiteral>,
    /// True when the destination is a `borrow` slot whose declared type still
    /// mentions a parameter.
    ///
    /// Its instances split: a nullable substitution belongs to the out-slot
    /// place duty, while a non-nullable one keeps the ordinary conversion
    /// report.
    pub borrow_slot: bool,
}

/// The literal a deferred conversion wrote.
///
/// The audit judges the resolved destination against the written value, so it
/// carries the value itself, not just its kind.
pub enum ConversionLiteral {
    /// An integer literal's value.
    Int(i128),
    /// A float literal's value.
    Float(f64),
}

/// One conditional whose arm agreement is deferred to its instance.
///
/// The declaration cannot judge a conditional whose arms still mention a type
/// parameter: the substitution decides whether the resolved arms agree. The
/// audit applies the same rule the declaration applies to concrete arms, and
/// reports the null rule when a resolved arm is the null shape in a
/// non-nullable sibling context.
pub struct OpaqueConditional {
    /// The enclosing function's name.
    pub function: String,
    /// The then arm's declared type.
    pub then_ty: Ty,
    /// The else arm's declared type.
    pub else_ty: Ty,
    /// The then arm's span, for the null-arm report.
    pub then_span: Span,
    /// The else arm's span, for the conversion and null-arm reports.
    pub else_span: Span,
    /// The then arm's final member spelling, when it is a member read.
    ///
    /// A dotted or postfix chain records one projection per segment at the
    /// same written span, so the instance resolves only the projection naming
    /// this member, never an inner prefix of a longer chain.
    pub then_member: Option<String>,
    /// The else arm's final member spelling, when it is a member read.
    pub else_member: Option<String>,
    /// The then arm's written root binding, for the possible-null report.
    pub then_base: Option<String>,
    /// The else arm's written root binding, for the possible-null report.
    pub else_base: Option<String>,
    /// The whole conditional's span, for the destination conversion.
    pub span: Span,
    /// The destination the conditional must convert to, when one is known.
    ///
    /// A let, return, field, or call-argument destination is recorded here so
    /// the instance judges the resolved result exactly as the concrete twin's
    /// destination does. `None` means the conditional sits in a position with
    /// no destination of its own.
    pub expected: Option<Ty>,
    /// True when the declaration already reported a null arm at a concrete
    /// destination, so the instance must not report it a second time.
    pub null_reported: bool,
}

/// The arm types of one checked conditional.
///
/// A generic call types its arguments before its type arguments are inferred,
/// so a conditional argument's null arm cannot face the destination then. The
/// recorded arms let the call site attach the destination once the parameter
/// is resolved, without walking the expression a second time.
pub struct ConditionalArms {
    /// The enclosing function's name, when the conditional sits in one.
    pub function: Option<String>,
    /// The then arm's checked type.
    pub then_ty: Ty,
    /// The else arm's checked type.
    pub else_ty: Ty,
    /// The then arm's span.
    pub then_span: Span,
    /// The else arm's span.
    pub else_span: Span,
    /// The then arm's final member spelling, when it is a member read.
    ///
    /// A dotted or postfix member chain on a type parameter records one
    /// projection per segment, all at the same written span. The instance must
    /// resolve only the projection that names the arm's final member, not an
    /// inner prefix of a longer chain.
    pub then_member: Option<String>,
    /// The else arm's final member spelling, when it is a member read.
    pub else_member: Option<String>,
    /// The then arm's written root binding, for the possible-null report.
    pub then_base: Option<String>,
    /// The else arm's written root binding, for the possible-null report.
    pub else_base: Option<String>,
    /// The whole conditional's span.
    pub span: Span,
    /// The destination the conditional must convert to, when one is known.
    pub destination: Option<Ty>,
    /// True when an arm still names a type parameter, so the instance must
    /// decide the agreement.
    pub opaque: bool,
    /// True when the declaration already reported a null arm.
    pub null_reported: bool,
}

/// One index read on a bare type parameter.
///
/// The declaration cannot judge the bounds: the substitution may be any type,
/// so the read records this duty and the per-instance audit resolves it
/// through the concrete row type.
pub struct OpaqueIndex {
    /// The enclosing function's name.
    pub function: String,
    /// The base's declared type, still mentioning the parameter.
    pub ty: Ty,
    /// The constant index the source wrote, when it wrote one.
    pub constant: Option<i128>,
    /// The base's span: the non-indexable and null rules report there,
    /// exactly as the concrete spelling does.
    pub span: Span,
    /// The index's span: the constant range rule reports there.
    pub index_span: Span,
    /// The base's written root name when it is a plain binding, so the null
    /// rule names the value as the concrete spelling does.
    pub base: Option<String>,
}

/// Checks every module of the loaded program and records the result.
pub fn check(loaded: &Loaded, profile: Profile, diagnostics: &mut Diagnostics) -> SemaOutput {
    let table = TypeTable::build(loaded, AbiLayout::for_profile(profile));
    // The table keeps the simple-name key for the emitter; the collision is
    // reported here, once, at the second module's declaration. The same-module
    // duplicate is the loader's declaration error and is not repeated.
    for (name, module, span) in table.cross_module_struct_collisions() {
        diagnostics.push(
            Diagnostic::error(
                "E11101",
                format!("duplicate declaration of '{name}' in module '{module}'"),
                span,
            )
            .with_help("remove one declaration; declaration merging forbids duplicates"),
        );
    }
    // A global initializer cycle has no valid initialization order, so it is
    // reported before the per-module type checks that would consume the order.
    init_cycle::check(loaded, diagnostics);
    let mut expr_types: HashMap<usize, Ty> = HashMap::new();
    let mut closures: BTreeMap<usize, ClosureSite> = BTreeMap::new();
    let mut capture_moves: Vec<CaptureMoveRequirement> = Vec::new();
    let mut opaque_projections: Vec<OpaqueProjection> = Vec::new();
    let mut opaque_indexes: Vec<OpaqueIndex> = Vec::new();
    let mut opaque_member_uses: Vec<OpaqueMemberUse> = Vec::new();
    let mut opaque_conversions: Vec<OpaqueConversion> = Vec::new();
    let mut opaque_conditionals: Vec<OpaqueConditional> = Vec::new();
    // Lambda site ids are program-global and deterministic: they count the
    // lambda expressions in source order across every module, so the same
    // program always names the same lifted function and environment.
    let mut next_closure_id = 0usize;
    for module in loaded.modules.values() {
        let mut checker = Checker::new(module, &table, loaded, profile, diagnostics, next_closure_id);
        checker.check_module();
        next_closure_id = checker.next_closure_id;
        closures.extend(checker.closures.drain(..).map(|site| (site.id, site)));
        capture_moves.extend(checker.capture_moves.drain(..));
        opaque_projections.extend(checker.opaque_projections.drain(..));
        opaque_indexes.extend(checker.opaque_indexes.drain(..));
        opaque_member_uses.extend(checker.opaque_member_uses.drain(..));
        opaque_conversions.extend(checker.opaque_conversions.drain(..));
        opaque_conditionals.extend(checker.opaque_conditionals.drain(..));
        expr_types.extend(checker.recorded.drain(..));
    }
    SemaOutput {
        table,
        expr_types,
        closures,
        capture_moves,
        opaque_projections,
        opaque_indexes,
        opaque_member_uses,
        opaque_conversions,
        opaque_conditionals,
    }
}

/// A local or parameter binding: its checked type and lend-ness.
struct Binding {
    /// The type the name has.
    ty: Ty,
    /// True when the binding is a writable lend: a `borrow` parameter or a
    /// borrowed lambda capture. A store into a lend destination writes
    /// through the caller's storage, which is what makes an owning move out
    /// of a capture unsound there.
    lend: bool,
}

/// Checks one module against the program-wide type table.
struct Checker<'a, 'd> {
    /// The module being checked.
    module: &'a Module,
    /// Every loaded module, for imported-declaration lookups.
    loaded: &'a Loaded,
    /// Every declared type of the program.
    table: &'a TypeTable,
    /// Folds the constant expressions of this module.
    folder: ConstFolder<'a>,
    /// Lexical scopes from outermost to innermost.
    scopes: Vec<BTreeMap<String, Binding>>,
    /// Type parameters of the function currently being checked.
    type_params: Vec<String>,
    /// Names of the current function's type parameters that carry a callable
    /// bound. Their values are callable-only: call or pass, never store.
    callable_bound_params: Vec<String>,
    /// Name of the function currently being checked, for recursion checks.
    function_name: Option<String>,
    /// True while a module `const` initializer is being checked.
    ///
    /// `embed` is legal only in a constant context; a module-level `let`, a
    /// local `let`, and a local `const` are runtime positions.
    in_const_context: bool,
    /// Return type of the function currently being checked.
    returns: Ty,
    /// `(node address, type)` for every checked expression, handed to the
    /// ownership stage.
    recorded: Vec<(usize, Ty)>,
    /// The next free lambda-site id, shared across the program's modules.
    next_closure_id: usize,
    /// The captured bindings of each capturing lambda, in site order.
    closures: Vec<ClosureSite>,
    /// Lambda bodies that return or consume a by-value capture whose type
    /// still mentions a type parameter, in check order.
    capture_moves: Vec<CaptureMoveRequirement>,
    /// Value-form member reads whose base is a bare type parameter, in check
    /// order.
    opaque_projections: Vec<OpaqueProjection>,
    /// Index reads whose base is a bare type parameter, in check order.
    opaque_indexes: Vec<OpaqueIndex>,
    /// Member reads on a bare type parameter used at a destination.
    opaque_member_uses: Vec<OpaqueMemberUse>,
    /// Values whose conversion is deferred to their instance, in check order.
    opaque_conversions: Vec<OpaqueConversion>,
    /// Conditionals whose arm agreement is deferred to their instance, in
    /// check order.
    opaque_conditionals: Vec<OpaqueConditional>,
    /// The arm types of every checked conditional, in check order.
    conditional_arms: Vec<ConditionalArms>,
    /// Maps a conditional's then arm address to its entry in
    /// [`Checker::conditional_arms`], so a call site can attach its
    /// destination after inference.
    conditional_index: HashMap<usize, usize>,
    /// The member-read expressions whose base is a view and whose spelling is
    /// `ptr`: they are the one form the frozen conversion table lets reach a
    /// `cstring` destination without a cast in the source.
    view_pointer_members: BTreeSet<usize>,
    /// The resolved parameter types of each `ffi` declaration this module
    /// calls, so its declared types are validated once per declaration.
    ffi_params: BTreeMap<String, Vec<Ty>>,
    /// The resolved result type of each `ffi` declaration this module calls,
    /// resolved once with the parameters so the ordinary conversion judgments
    /// see the result's declared type.
    ffi_results: BTreeMap<String, Ty>,
    /// The capture lists of the lambdas currently being checked, innermost
    /// last. A body write or move of a name found here is a capture error.
    lambda_captures: Vec<Vec<CaptureInfo>>,
    /// The scope depth at each lambda's entry, innermost last. A body read
    /// that resolves below the innermost floor names an enclosing lexical
    /// binding, so it is legal only when the capture list names it.
    lambda_floors: Vec<usize>,
    /// The capture names listed by each lambda currently being checked,
    /// innermost last, in every mode. A listed name stays out of the
    /// unlisted-read report even when its capture mode is itself rejected,
    /// so a `take` capture read does not add a second error.
    lambda_listed: Vec<Vec<String>>,
    /// The return type inferred for each block-bodied lambda currently being
    /// checked, innermost last. Each `return` in the body records the first
    /// non-`void` value type here, so a block body gets the same return type an
    /// expression body would.
    lambda_returns: Vec<Option<Ty>>,
    /// The capture names whose by-value move each currently checked lambda has
    /// already recorded, innermost last.
    ///
    /// A composite move site nests: the field value's walk records first and
    /// the enclosing site's walk would find the same capture again, so one
    /// record per capture keeps the site's report single.
    captured_move_records: Vec<BTreeSet<(String, Vec<PlacePathStep>)>>,
    /// Diagnostic sink.
    diagnostics: &'d mut Diagnostics,
    /// The selected profile, so profile-surface rules can be reported here.
    profile: Profile,
    /// The folder array extents resolve through.
    ///
    /// An extent must equal the C type the emitter spells, so an extent
    /// `sizeof` measures the target ABI; the value positions keep the declared
    /// widths and fold through `folder`.
    extent_folder: ConstFolder<'a>,
}

impl<'a, 'd> Checker<'a, 'd> {
    /// Creates a checker for one module.
    fn new(
        module: &'a Module,
        table: &'a TypeTable,
        loaded: &'a Loaded,
        profile: Profile,
        diagnostics: &'d mut Diagnostics,
        next_closure_id: usize,
    ) -> Self {
        Self {
            module,
            loaded,
            table,
            folder: ConstFolder::new(table, module, loaded),
            extent_folder: ConstFolder::for_extents(table, module, loaded),
            scopes: Vec::new(),
            type_params: Vec::new(),
            callable_bound_params: Vec::new(),
            function_name: None,
            in_const_context: false,
            returns: Ty::void(),
            recorded: Vec::new(),
            next_closure_id,
            closures: Vec::new(),
            capture_moves: Vec::new(),
            opaque_projections: Vec::new(),
            opaque_indexes: Vec::new(),
            opaque_member_uses: Vec::new(),
            opaque_conversions: Vec::new(),
            opaque_conditionals: Vec::new(),
            conditional_arms: Vec::new(),
            conditional_index: HashMap::new(),
            view_pointer_members: BTreeSet::new(),
            ffi_params: BTreeMap::new(),
            ffi_results: BTreeMap::new(),
            lambda_captures: Vec::new(),
            lambda_floors: Vec::new(),
            lambda_listed: Vec::new(),
            lambda_returns: Vec::new(),
            captured_move_records: Vec::new(),
            diagnostics,
            profile,
        }
    }

    /// Checks every declaration of the module.
    fn check_module(&mut self) {
        for item in &self.module.items {
            self.check_item(item);
        }
        // The conditional sites are complete once every body has been checked,
        // including the destinations a call site attached after inference, so
        // the deferred sites are recorded here in source order.
        for arms in self.conditional_arms.drain(..) {
            if !arms.opaque {
                continue;
            }
            let Some(function) = arms.function else {
                continue;
            };
            self.opaque_conditionals.push(OpaqueConditional {
                function,
                then_ty: arms.then_ty,
                else_ty: arms.else_ty,
                then_span: arms.then_span,
                else_span: arms.else_span,
                then_member: arms.then_member,
                else_member: arms.else_member,
                then_base: arms.then_base,
                else_base: arms.else_base,
                span: arms.span,
                expected: arms.destination,
                null_reported: arms.null_reported,
            });
        }
    }

    /// Checks one top-level declaration.
    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Func(decl) => self.check_function(decl),
            Item::Struct(decl) => self.check_struct(decl),
            Item::Enum(decl) => self.check_enum(decl),
            Item::GlobalLet(decl) => {
                self.check_global(&decl.ty, &decl.value, false, decl.name_span)
            }
            Item::GlobalConst(decl) => {
                self.check_global(&decl.ty, &decl.value, true, decl.name_span)
            }
            // A `comptime` body is frozen syntax whose semantics are staged
            // out, so it reports its staged-feature code. Handles and FFI
            // signatures are checked by later work; their signatures are
            // already in the type table.
            Item::Comptime(decl) => self.error(
                "E90001",
                "'comptime' semantics are not yet implemented",
                "remove the comptime declaration or wait for the staged feature",
                decl.name_span,
            ),
            Item::Handle(_) | Item::Ffi(_) | Item::Intrinsic(_) => {}
        }
    }

    /// Checks a function declaration, its type parameters, and its body.
    fn check_function(&mut self, decl: &FuncDecl) {
        if decl.exported && !decl.type_params.is_empty() {
            self.error(
                "E10302",
                format!("generic '{}' cannot be exported", decl.name),
                "write a concrete exported wrapper",
                decl.name_span,
            );
        }
        self.push_scope();
        self.type_params = decl.type_params.iter().map(|p| p.name.clone()).collect();
        self.callable_bound_params = decl
            .type_params
            .iter()
            .filter(|param| param.bound.is_some())
            .map(|param| param.name.clone())
            .collect();
        self.function_name = Some(decl.name.clone());
        for param in &decl.params {
            let ty = self.resolve_type(&param.ty);
            self.check_nullable_type(&ty, param.name_span);
            self.check_overlay_value(&ty, param.name_span);
            let lend = param.mode == crate::ast::Mode::Borrow;
            self.declare_with_lend(&param.name, ty, lend);
        }
        self.returns = self.resolve_type(&decl.returns);
        let returns = self.returns.clone();
        self.check_nullable_type(&returns, decl.name_span);
        self.check_overlay_value(&returns, decl.name_span);
        self.check_block(&decl.body);
        self.function_name = None;
        self.type_params.clear();
        self.callable_bound_params.clear();
        self.pop_scope();
    }

    /// Resolves an AST type in the current type-parameter scope, folding its
    /// array extents and checking the rest of its constant-shaped parts.
    ///
    /// A type whose extent is present but not constant is already reported as
    /// `E10224`, so it becomes the opaque type instead of cascading into an
    /// array-extent error.
    fn resolve_type(&mut self, ty: &Type) -> Ty {
        self.resolve_type_at(ty, false)
    }

    /// Resolves a type written in a type-argument position.
    ///
    /// An explicit call type argument is such a position, so it is checked
    /// against the nullability rule at its own node and every node beneath it,
    /// exactly as a nested type argument in a declaration is.
    fn resolve_type_argument(&mut self, ty: &Type) -> Ty {
        self.resolve_type_at(ty, true)
    }

    /// The shared resolver, marked with the position the type sits at.
    fn resolve_type_at(&mut self, ty: &Type, argument: bool) -> Ty {
        let valid = if argument {
            self.validate_type_argument(ty)
        } else {
            self.validate_type(ty)
        };
        if !valid {
            return Ty::unknown();
        }
        let table = self.table;
        let type_params = &self.type_params;
        let folder = &mut self.extent_folder;
        let resolved = resolve_ty_with(ty, type_params, table, &mut |expr| {
            folder.integer_at(expr, Some(ScalarType::Usize))
        });
        self.reject_fixed_array_sequence(&resolved, ty.span);
        // The built-in `String` handle is heap-backed: its drop and view use
        // `FreeString`/`cl_string_ptr`/`cl_string_len`, which the freestanding
        // runtime does not provide. Reject the type there rather than emit C
        // that cannot link.
        if self.profile == Profile::Freestanding && mentions_heap_string(&resolved) {
            self.error(
                "E11200",
                "'String' is not available on profile 'freestanding'",
                "the freestanding profile has no heap; use a plain string",
                ty.span,
            );
        }
        resolved
    }

    /// Reports `E90000` for a growable sequence whose element is a fixed array.
    ///
    /// The check runs on the resolved `Array<...>` type, so a type built by a
    /// generic call's substitution is caught as well as a written annotation.
    /// The version 1 container stores its elements by value through a pointer,
    /// so a fixed-array element would need a pointer-to-array declaration and
    /// per-dimension indexing that the container API does not emit yet. A
    /// scalar, handle, struct, slice, or nested growable element is accepted.
    fn reject_fixed_array_sequence(&mut self, ty: &Ty, span: Span) {
        let TyKind::Struct(name, args) = &ty.kind else {
            return;
        };
        if name != "Array" {
            return;
        }
        for arg in args {
            if arg.is_fixed_array() {
                self.error(
                    "E90000",
                    format!("'{}' is not yet supported", arg.display()),
                    "wrap the elements in a struct; a fixed-array element is staged",
                    span,
                );
            } else {
                self.reject_fixed_array_sequence(arg, span);
            }
        }
    }

    /// Reports `E10207` when a value of `ty` reaches an overlay type.
    ///
    /// An overlay type is reached only through a `raw` pointer, so this follows
    /// arrays and struct fields but stops at `raw`. The array case matters
    /// because an array of overlay values is itself a by-value local.
    fn check_overlay_value(&mut self, ty: &Ty, span: Span) {
        if let Some(name) = self.overlay_reached(ty, 0) {
            self.error(
                "E10207",
                format!("overlay type '{name}' cannot be used by value"),
                "access it through a `raw` pointer inside `unsafe`",
                span,
            );
        }
    }

    /// The name of the first overlay type a value of `ty` contains, if any.
    ///
    /// The recursion is depth-limited so a malformed self-referential
    /// declaration cannot loop.
    fn overlay_reached(&self, ty: &Ty, depth: usize) -> Option<String> {
        if depth > 32 {
            return None;
        }
        match &ty.kind {
            TyKind::Raw(_) => None,
            TyKind::Array { element, .. } => self.overlay_reached(element, depth + 1),
            TyKind::Struct(name, args) => {
                if self.table.is_overlay(name) {
                    return Some(name.clone());
                }
                let params = self.table.struct_type_params(name).unwrap_or(&[]);
                self.table.struct_fields(name).and_then(|fields| {
                    fields.iter().find_map(|(_, field)| {
                        self.overlay_reached(&substitute_instance(field, params, args), depth + 1)
                    })
                })
            }
            _ => None,
        }
    }

    /// Declares a name in the innermost scope.
    fn declare(&mut self, name: &str, ty: Ty) {
        self.declare_with_lend(name, ty, false);
    }

    /// Declares a name in the innermost scope, marking a writable lend.
    fn declare_with_lend(&mut self, name: &str, ty: Ty, lend: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), Binding { ty, lend });
        }
    }

    /// True when a name resolves to a writable lend binding.
    fn binding_lends(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return binding.lend;
            }
        }
        false
    }

    /// Enters a new lexical scope.
    fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    /// Leaves the innermost lexical scope.
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Reports an error diagnostic.
    fn error(&mut self, code: &'static str, message: impl Into<String>, help: &str, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span).with_help(help));
    }

    /// The depth of the innermost scope that binds `name`, if any.
    ///
    /// Scope depth grows inward from the function scope. The capture check
    /// compares a read's binding depth with the innermost lambda's floor: a
    /// binding below the floor belongs to an enclosing function or lambda and
    /// must appear in the reading lambda's capture list.
    fn name_scope_depth(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(depth, scope)| scope.contains_key(name).then_some(depth))
    }

    /// The capture mode of `name` in the innermost enclosing lambda that
    /// captures it, if any.
    ///
    /// The innermost capture wins so a nested lambda's own capture shadows an
    /// enclosing lambda's capture of the same spelling.
    fn capture_mode(&self, name: &str) -> Option<CaptureMode> {
        for captures in self.lambda_captures.iter().rev() {
            if let Some(info) = captures.iter().find(|info| info.name == name) {
                return Some(info.mode);
            }
        }
        None
    }

    /// True when a capture entry names a module-level binding.
    ///
    /// A module-level name — a `let`, a `const`, a function, a type, an
    /// imported name, or any other module declaration — is visible inside a
    /// lambda without capture, so listing it is meaningless. A lexical
    /// binding of the same spelling wins, exactly as it does for a name read.
    fn name_is_module_binding(&self, name: &str) -> bool {
        if self.name_scope_depth(name).is_some() {
            return false;
        }
        self.module.decl(name).is_some() || self.module.import(name).is_some()
    }

    /// Reports a capture entry the enclosing scope cannot carry.
    ///
    /// A module-level binding needs no capture, so the entry reports
    /// `E10406` and the fix is to use the name directly. Inside a nested
    /// lambda, an entry for a name below the enclosing lambda's floor travels
    /// through that lambda's environment, so the enclosing capture list must
    /// carry the name; the report reuses `E10405` and the fix is to add it
    /// there. Returns `true` when the entry was reported, so the caller skips
    /// it.
    fn check_capture_entry(&mut self, name: &str, span: Span) -> bool {
        if self.name_is_module_binding(name) {
            self.error(
                "E10406",
                format!("module-level '{name}' needs no capture"),
                "use the name directly",
                span,
            );
            return true;
        }
        if let Some(&floor) = self.lambda_floors.last() {
            if self.name_scope_depth(name).is_some_and(|depth| depth < floor) {
                self.error(
                    "E10405",
                    format!("lambda reads '{name}' without listing it"),
                    &format!("add '{name}' to the capture list"),
                    span,
                );
                return true;
            }
        }
        false
    }

    /// Reports `E10402` when a lambda body writes a capture that is not a
    /// `[borrow x]` capture.
    fn reject_capture_write(&mut self, target: &Expr) {
        let Some(root) = place_root_name(target) else {
            return;
        };
        if matches!(
            self.capture_mode(root),
            Some(CaptureMode::Copy | CaptureMode::ReadOnlyBorrow)
        ) {
            self.error(
                "E10402",
                format!("cannot write captured '{root}' without '[borrow {root}]'"),
                "add the borrow capture",
                target.span,
            );
        }
    }

    /// Records the returned type of the block-bodied lambda being checked.
    ///
    /// The first non-`void` `return` value fixes the lambda's return type; a
    /// block with no value `return` stays `void`. A `return` from an enclosing
    /// function, not inside a block-bodied lambda, is ignored.
    fn record_lambda_return(&mut self, ty: &Ty) {
        if let Some(slot) = self.lambda_returns.last_mut() {
            if slot.is_none() && !ty.is_void() {
                *slot = Some(ty.clone());
            }
        }
    }

    /// Reports `E10404` when a capturing closure reaches a storage position.
    ///
    /// A closure owns a stack environment valid only for its synchronous call,
    /// so binding, storing, returning, or capturing one has no meaning.
    fn reject_ephemeral_storage(&mut self, ty: &Ty, span: Span) {
        if ty.is_closure() {
            self.error(
                "E10404",
                "closures are ephemeral and cannot be stored, returned, or captured",
                "pass the lambda directly",
                span,
            );
        }
    }

    /// The type a name reference has.
    ///
    /// A lexical binding wins, then a module declaration, then an import. An
    /// imported name is opaque unless the import targets a known program
    /// function, whose signature is available through the type table.
    fn name_type(&self, name: &str) -> Ty {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return binding.ty.clone();
            }
        }
        if let Some(decl) = self.module.decl(name) {
            match decl.kind {
                DeclKind::Func => {
                    return self
                        .table
                        .function(&self.module.name, name)
                        .map(|signature| Ty::func(signature.ty()))
                        .unwrap_or_else(Ty::unknown);
                }
                DeclKind::Global | DeclKind::Const => {
                    return self
                        .table
                        .global(&self.module.name, name)
                        .cloned()
                        .unwrap_or_else(Ty::unknown);
                }
                DeclKind::Enum => return Ty::enum_(name),
                DeclKind::Handle => return Ty::handle(name),
                DeclKind::Struct | DeclKind::Intrinsic | DeclKind::Ffi => {}
            }
        }
        if let Some(binding) = self.module.import(name) {
            if binding.poisoned {
                return Ty::unknown();
            }
            // The import's original name is the binding: an imported function
            // is typed by its signature and an imported module-level value by
            // its own declared type, so a same-named binding elsewhere never
            // stands in for the imported one.
            if let Some(signature) = self.table.function(&binding.module, &binding.original) {
                return Ty::func(signature.ty());
            }
            return self
                .table
                .global(&binding.module, &binding.original)
                .cloned()
                .unwrap_or_else(Ty::unknown);
        }
        Ty::unknown()
    }

    /// True when a name reads as a value rather than as a type name.
    ///
    /// A lexical binding, function, global, or import shadows a type of the
    /// same spelling; struct, enum, and handle names are types only.
    fn name_is_value(&self, name: &str) -> bool {
        if self.scopes.iter().rev().any(|scope| scope.contains_key(name)) {
            return true;
        }
        match self.module.decl(name) {
            Some(decl) => matches!(
                decl.kind,
                DeclKind::Func | DeclKind::Global | DeclKind::Const
            ),
            None => self.module.import(name).is_some(),
        }
    }

    /// True when the name is bound in a lexical scope.
    ///
    /// A local binding shadows a module-level function of the same spelling,
    /// so a call must resolve through the binding rather than the function.
    fn is_local(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.contains_key(name))
    }

    /// The type a call-shaped name denotes when it is a type name, if any.
    ///
    /// Used to turn `Point(0)`, `E(x)`, and `File(0)` into casts.
    fn callee_type(&self, callee: &crate::ast::Expr) -> Option<Ty> {
        let name = match &callee.kind {
            crate::ast::ExprKind::Name(name) => name,
            crate::ast::ExprKind::Qualified(qualified) if qualified.segments.len() == 1 => {
                &qualified.segments[0]
            }
            _ => return None,
        };
        if self.name_is_value(name) {
            return None;
        }
        if self.table.is_struct(name) {
            return Some(Ty::struct_(name));
        }
        if self.table.is_enum(name) {
            return Some(Ty::enum_(name));
        }
        if self.table.is_handle(name) {
            return Some(Ty::handle(name));
        }
        None
    }

    /// The signature of a free function visible in this module.
    fn free_function(&self, name: &str) -> Option<crate::types::FuncSig> {
        if let Some(signature) = self.table.function(&self.module.name, name) {
            return Some(signature.clone());
        }
        let binding = self.module.import(name)?;
        if binding.poisoned {
            return None;
        }
        self.table
            .function(&binding.module, &binding.original)
            .cloned()
    }

    /// The written signature of an `ffi` function declared in this module.
    ///
    /// An `ffi` declaration is not registered in the type table: its C
    /// prototype crosses by ABI types, so a check that needs the declared
    /// parameter list reads the written signature here. A name reached
    /// through an import has no `ffi` declaration: the declarations resolve
    /// within their own module.
    fn ffi_signature(&self, name: &str) -> Option<&'a crate::ast::FuncSignature> {
        self.module.items.iter().find_map(|item| match item {
            crate::ast::Item::Ffi(block) => block
                .decls
                .iter()
                .find(|signature| signature.name == name),
            _ => None,
        })
    }

    /// The kind of the declaration a name resolves to in this module.
    ///
    /// A module declaration wins, then an import; a name with no visible
    /// declaration has no kind. The kind tells a compiler-known intrinsic
    /// declaration from a user or `ffi` function that shadows its spelling.
    fn visible_decl_kind(&self, name: &str) -> Option<DeclKind> {
        if let Some(decl) = self.module.decl(name) {
            return Some(decl.kind);
        }
        let binding = self.module.import(name)?;
        if binding.poisoned {
            return None;
        }
        self.loaded
            .modules
            .get(&binding.module)?
            .decl(&binding.original)
            .map(|decl| decl.kind)
    }

    /// The kind of a declaration reached through a module path.
    ///
    /// The own module answers from its declaration table. A standard module's
    /// compiler intrinsic is known from the intrinsic table because the
    /// embedded source may not be loaded as a user module. A loaded module
    /// answers from its own declarations, which covers a cross-module `ffi`
    /// function.
    fn qualified_decl_kind(&self, module: &str, name: &str) -> Option<DeclKind> {
        if module == self.module.name {
            return self.module.decl(name).map(|decl| decl.kind);
        }
        if let Some(target) = self.loaded.module(module) {
            if let Some(decl) = target.decl(name) {
                return Some(decl.kind);
            }
        }
        if crate::stdlib::is_standard_module(module)
            && crate::stdlib::is_compiler_intrinsic(name)
        {
            return Some(DeclKind::Intrinsic);
        }
        None
    }

    /// The `ffi` signature a module path names.
    fn qualified_ffi_signature(
        &self,
        module: &str,
        name: &str,
    ) -> Option<&'a crate::ast::FuncSignature> {
        let loaded: &'a Loaded = self.loaded;
        let target = loaded.module(module)?;
        target.items.iter().find_map(|item| match item {
            crate::ast::Item::Ffi(block) => {
                block.decls.iter().find(|signature| signature.name == name)
            }
            _ => None,
        })
    }
}

/// True when a type mentions the heap-backed `String` handle anywhere.
///
/// Used to make the freestanding profile reject `String`, whose runtime
/// helpers live only in the host and hosted-amiga shims.
fn mentions_heap_string(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Handle(name) => name == "String",
        TyKind::Array { element, .. } => mentions_heap_string(element),
        TyKind::Raw(inner) => mentions_heap_string(inner),
        TyKind::Struct(_, args) => args.iter().any(mentions_heap_string),
        TyKind::Func(signature) => {
            signature
                .params
                .iter()
                .any(|param| mentions_heap_string(&param.ty))
                || mentions_heap_string(&signature.returns)
        }
        TyKind::Closure(closure) => {
            closure.params.iter().any(mentions_heap_string)
                || mentions_heap_string(&closure.returns)
        }
        _ => false,
    }
}

/// The root name of a place expression, if it is a plain name.
///
/// `counter`, `obj.field`, and `xs[0]` all root at a name, so a capture check
/// can tell that a write or move targets a captured binding. A dotted place
/// such as `p.x` parses as a multi-segment qualified name, so its root is the
/// first segment; a module path roots at its module segment too, and no
/// capture can carry that spelling (an entry naming a module-level binding is
/// rejected), so the comparison stays safe.
pub(super) fn place_root_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Name(name) => Some(name),
        ExprKind::Qualified(name) => name.segments.first().map(String::as_str),
        ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => place_root_name(base),
        // The wrapper is transparent to the place it wraps, so a wrapped
        // capture write is judged by the same root as the bare spelling.
        ExprKind::Unsafe(inner) => place_root_name(inner),
        _ => None,
    }
}

/// Collects every capture root a move site may consume, with the projection
/// path from each root.
///
/// `path` is the projection chain built so far, outermost step first, and a
/// root records its steps innermost first. A plain place roots at one entry; a
/// conditional selects one of its arms at run time, so both arms are walked
/// with the same chain. A struct or array literal consumes each field or
/// element value whole, so the parts start a fresh chain and a projection of
/// the literal never extends the parts' paths. A projection of a place extends
/// that place's chain, `unsafe` passes through so a wrapped place is found,
/// and a dotted qualified place contributes its later segments as field steps.
/// This mirrors the ownership stage's move targets, so the capture rules and
/// the per-instance duty judge the same places.
pub(super) fn place_move_paths<'e>(
    expr: &'e Expr,
    path: &[PlacePathStep],
    roots: &mut Vec<(&'e str, Vec<PlacePathStep>)>,
) {
    match &expr.kind {
        ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } => {
            place_move_paths(then_value, path, roots);
            place_move_paths(else_value, path, roots);
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                place_move_paths(&field.value, &[], roots);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                place_move_paths(element, &[], roots);
            }
            if let Some(fill) = fill {
                place_move_paths(fill, &[], roots);
            }
        }
        ExprKind::Member { base, name } => {
            let mut nested = path.to_vec();
            nested.push(PlacePathStep::Field(name.clone()));
            place_move_paths(base, &nested, roots);
        }
        ExprKind::Index { base, .. } => {
            let mut nested = path.to_vec();
            nested.push(PlacePathStep::Element);
            place_move_paths(base, &nested, roots);
        }
        ExprKind::Unsafe(inner) => place_move_paths(inner, path, roots),
        _ => {
            if let Some(root) = place_root_name(expr) {
                let mut steps: Vec<PlacePathStep> = Vec::new();
                if let ExprKind::Qualified(name) = &expr.kind {
                    steps.extend(
                        name.segments
                            .iter()
                            .skip(1)
                            .cloned()
                            .map(PlacePathStep::Field),
                    );
                }
                steps.extend(path.iter().rev().cloned());
                roots.push((root, steps));
            }
        }
    }
}

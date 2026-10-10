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

use std::collections::{BTreeMap, HashMap};

use crate::ast::{Expr, ExprKind, FuncDecl, Item, ScalarType, Type};
use crate::consts::ConstFolder;
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{DeclKind, Loaded, Module};
use crate::profile::Profile;
use crate::span::Span;
use crate::types::{
    resolve_ty_with, substitute_instance, CaptureInfo, CaptureMode, ClosureSite, Ty, TyKind,
    TypeTable,
};

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
    /// The returning or consuming site.
    pub span: Span,
}

/// Checks every module of the loaded program and records the result.
pub fn check(loaded: &Loaded, profile: Profile, diagnostics: &mut Diagnostics) -> SemaOutput {
    let table = TypeTable::build(loaded);
    // A global initializer cycle has no valid initialization order, so it is
    // reported before the per-module type checks that would consume the order.
    init_cycle::check(loaded, diagnostics);
    let mut expr_types: HashMap<usize, Ty> = HashMap::new();
    let mut closures: BTreeMap<usize, ClosureSite> = BTreeMap::new();
    let mut capture_moves: Vec<CaptureMoveRequirement> = Vec::new();
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
        expr_types.extend(checker.recorded.drain(..));
    }
    SemaOutput {
        table,
        expr_types,
        closures,
        capture_moves,
    }
}

/// A local or parameter binding: its checked type.
struct Binding {
    /// The type the name has.
    ty: Ty,
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
    /// Diagnostic sink.
    diagnostics: &'d mut Diagnostics,
    /// The selected profile, so profile-surface rules can be reported here.
    profile: Profile,
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
            scopes: Vec::new(),
            type_params: Vec::new(),
            callable_bound_params: Vec::new(),
            function_name: None,
            returns: Ty::void(),
            recorded: Vec::new(),
            next_closure_id,
            closures: Vec::new(),
            capture_moves: Vec::new(),
            lambda_captures: Vec::new(),
            lambda_floors: Vec::new(),
            lambda_listed: Vec::new(),
            lambda_returns: Vec::new(),
            diagnostics,
            profile,
        }
    }

    /// Checks every declaration of the module.
    fn check_module(&mut self) {
        for item in &self.module.items {
            self.check_item(item);
        }
    }

    /// Checks one top-level declaration.
    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Func(decl) => self.check_function(decl),
            Item::Struct(decl) => self.check_struct(decl),
            Item::Enum(decl) => self.check_enum(decl),
            Item::GlobalLet(decl) => self.check_global(&decl.ty, &decl.value, false),
            Item::GlobalConst(decl) => self.check_global(&decl.ty, &decl.value, true),
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
            self.check_overlay_value(&ty, param.name_span);
            self.declare(&param.name, ty);
        }
        self.returns = self.resolve_type(&decl.returns);
        let returns = self.returns.clone();
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
        if !self.validate_type(ty) {
            return Ty::unknown();
        }
        let table = self.table;
        let type_params = &self.type_params;
        let resolved = resolve_ty_with(ty, type_params, table, &mut |expr| {
            self.folder.integer_at(expr, Some(ScalarType::Usize))
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
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), Binding { ty });
        }
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
            return self
                .table
                .function(&binding.module, &binding.original)
                .map(|signature| Ty::func(signature.ty()))
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
        _ => None,
    }
}

/// Collects every root name a capture move site may consume.
///
/// A plain place roots at one name, but a conditional selects one of its arms
/// at run time, so a move out of `c ? x : y` may consume either arm's root; a
/// nested conditional recurses. A struct or array literal consumes each field
/// or element value, so a capture carried inside one is a move of that
/// capture. This mirrors the ownership stage's conditional move sources and
/// its aggregate consumption, so the capture rules and the per-instance duty
/// judge the same shapes.
pub(super) fn place_root_names<'e>(expr: &'e Expr, roots: &mut Vec<&'e str>) {
    match &expr.kind {
        ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } => {
            place_root_names(then_value, roots);
            place_root_names(else_value, roots);
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                place_root_names(&field.value, roots);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                place_root_names(element, roots);
            }
            if let Some(fill) = fill {
                place_root_names(fill, roots);
            }
        }
        _ => {
            if let Some(root) = place_root_name(expr) {
                roots.push(root);
            }
        }
    }
}

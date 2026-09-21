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

use crate::ast::{FuncDecl, Item, ScalarType, Type};
use crate::consts::ConstFolder;
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{DeclKind, Loaded, Module};
use crate::span::Span;
use crate::types::{resolve_ty_with, Ty, TyKind, TypeTable};

pub(crate) mod call;
mod expr;
mod init_cycle;
mod items;
mod ops;
mod stmt;

#[cfg(test)]
mod regressions;
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
}

/// Checks every module of the loaded program and records the result.
pub fn check(loaded: &Loaded, diagnostics: &mut Diagnostics) -> SemaOutput {
    let table = TypeTable::build(loaded);
    // A global initializer cycle has no valid initialization order, so it is
    // reported before the per-module type checks that would consume the order.
    init_cycle::check(loaded, diagnostics);
    let mut expr_types: HashMap<usize, Ty> = HashMap::new();
    for module in loaded.modules.values() {
        let mut checker = Checker::new(module, &table, loaded, diagnostics);
        checker.check_module();
        expr_types.extend(checker.recorded.drain(..));
    }
    SemaOutput { table, expr_types }
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
    /// Every declared type of the program.
    table: &'a TypeTable,
    /// Folds the constant expressions of this module.
    folder: ConstFolder<'a>,
    /// Lexical scopes from outermost to innermost.
    scopes: Vec<BTreeMap<String, Binding>>,
    /// Type parameters of the function currently being checked.
    type_params: Vec<String>,
    /// Name of the function currently being checked, for recursion checks.
    function_name: Option<String>,
    /// Return type of the function currently being checked.
    returns: Ty,
    /// `(node address, type)` for every checked expression, handed to the
    /// ownership stage.
    recorded: Vec<(usize, Ty)>,
    /// Diagnostic sink.
    diagnostics: &'d mut Diagnostics,
}

impl<'a, 'd> Checker<'a, 'd> {
    /// Creates a checker for one module.
    fn new(
        module: &'a Module,
        table: &'a TypeTable,
        loaded: &'a Loaded,
        diagnostics: &'d mut Diagnostics,
    ) -> Self {
        Self {
            module,
            table,
            folder: ConstFolder::new(table, module, loaded),
            scopes: Vec::new(),
            type_params: Vec::new(),
            function_name: None,
            returns: Ty::void(),
            recorded: Vec::new(),
            diagnostics,
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
        self.check_sequence_element(ty);
        let table = self.table;
        let type_params = &self.type_params;
        resolve_ty_with(ty, type_params, table, &mut |expr| {
            self.folder.integer_at(expr, Some(ScalarType::Usize))
        })
    }

    /// Rejects a growable sequence whose element type owns a resource.
    ///
    /// The version 1 container has no generated drop glue, so its elements must
    /// be duplicable; a move-only element such as a handle is a staged-feature
    /// error rather than a type error.
    fn check_sequence_element(&mut self, ty: &Type) {
        let crate::ast::TypeKind::Named { name, args } = &ty.kind else {
            return;
        };
        if !name.segments.last().is_some_and(|last| last == "Array") {
            return;
        }
        let Some(element_ast) = args.first() else {
            return;
        };
        let element = self.resolve_type(element_ast);
        if element.is_unknown() || element.is_param() || self.table.duplicable(&element) {
            return;
        }
        self.error(
            "E90000",
            format!(
                "'{}' is not a duplicable element for 'Array'",
                element.display()
            ),
            "use a duplicable element type; owning-element drop glue is staged",
            ty.span,
        );
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
            TyKind::Struct(name, _) => {
                if self.table.is_overlay(name) {
                    return Some(name.clone());
                }
                self.table.struct_fields(name).and_then(|fields| {
                    fields
                        .iter()
                        .find_map(|(_, field)| self.overlay_reached(field, depth + 1))
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
}

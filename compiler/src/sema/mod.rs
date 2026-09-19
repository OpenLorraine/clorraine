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

use std::collections::BTreeMap;

use crate::ast::{FuncDecl, Item, Type};
use crate::consts::ConstFolder;
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{DeclKind, Loaded, Module};
use crate::span::Span;
use crate::types::{resolve_ty, Ty, TypeTable};

mod call;
mod expr;
mod items;
mod ops;
mod stmt;

/// Checks every module of the loaded program.
pub fn check(loaded: &Loaded, diagnostics: &mut Diagnostics) {
    let table = TypeTable::build(loaded);
    for module in loaded.modules.values() {
        let mut checker = Checker::new(module, &table, diagnostics);
        checker.check_module();
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
    /// Diagnostic sink.
    diagnostics: &'d mut Diagnostics,
}

impl<'a, 'd> Checker<'a, 'd> {
    /// Creates a checker for one module.
    fn new(module: &'a Module, table: &'a TypeTable, diagnostics: &'d mut Diagnostics) -> Self {
        Self {
            module,
            table,
            folder: ConstFolder::new(table, module),
            scopes: Vec::new(),
            type_params: Vec::new(),
            function_name: None,
            returns: Ty::void(),
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
            // `comptime` bodies, handles, and FFI signatures are checked by
            // later work; their signatures are already in the type table.
            Item::Comptime(_)
            | Item::Handle(_)
            | Item::Ffi(_)
            | Item::Intrinsic(_) => {}
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
            self.declare(&param.name, ty);
        }
        self.returns = self.resolve_type(&decl.returns);
        self.check_block(&decl.body);
        self.function_name = None;
        self.type_params.clear();
        self.pop_scope();
    }

    /// Resolves an AST type in the current type-parameter scope, checking the
    /// constant-shaped parts of the type as it goes.
    fn resolve_type(&mut self, ty: &Type) -> Ty {
        self.validate_type(ty);
        resolve_ty(ty, &self.type_params, self.table)
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

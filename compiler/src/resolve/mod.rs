//! Name resolution.
//!
//! Every name reference is checked against the lexical scopes that enclose it.
//! A scope is a block, plus the function parameters or type parameters that
//! belong to it; the innermost binding wins, and a name must be declared before
//! the point of use in its block. Module-level declarations and import bindings
//! are visible throughout their module, so module-level order does not matter.
//!
//! A reference that resolves nowhere is `E10219`. Qualified access such as
//! `std.mem.Buffer` or `Direction.UP` is always available, with or without an
//! import: the longest leading segments are matched against the module table
//! and the remaining name against that module's exports, or the first segment
//! is matched in scope and the rest treated as members.
//!
//! Two things are deliberately left to the type layer: member names after `.`
//! (a field and a free function can share a spelling, and the field wins) and
//! the dot-call candidate choice that depends on the receiver's type. This
//! stage checks the receiver and leaves the member alone.
//!
//! Redeclaring a reserved built-in name is already `E20006` from the parser, so
//! it is not reported a second time here.

use std::collections::BTreeSet;

use crate::ast::*;
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{Loaded, Module};

/// Resolves every name in the loaded program.
pub fn resolve(loaded: &Loaded, diagnostics: &mut Diagnostics) {
    for module in loaded.modules.values() {
        let mut resolver = Resolver {
            loaded,
            module,
            scopes: Vec::new(),
            diagnostics,
        };
        resolver.resolve_module();
    }
}

/// Resolves the names of one module against the loaded module table.
struct Resolver<'a, 'd> {
    /// The module table, for qualified access.
    loaded: &'a Loaded,
    /// The module being resolved.
    module: &'a Module,
    /// Lexical scopes from outermost to innermost.
    scopes: Vec<BTreeSet<String>>,
    /// Diagnostic sink.
    diagnostics: &'d mut Diagnostics,
}

impl Resolver<'_, '_> {
    /// Resolves every declaration of the module.
    fn resolve_module(&mut self) {
        for item in &self.module.items {
            self.resolve_item(item);
        }
    }

    /// Resolves the name references of one declaration.
    fn resolve_item(&mut self, item: &Item) {
        match item {
            Item::Func(decl) | Item::Comptime(decl) => self.resolve_function(decl),
            Item::Struct(decl) => {
                self.push_scope();
                self.declare_type_params(&decl.type_params);
                for field in &decl.fields {
                    self.resolve_type(&field.ty);
                }
                self.pop_scope();
            }
            Item::Enum(decl) => {
                self.resolve_type(&decl.backing);
                for member in &decl.members {
                    if let Some(value) = &member.value {
                        self.resolve_expr(value);
                    }
                }
            }
            Item::Handle(decl) => self.resolve_dotted_name(&decl.freed_by),
            Item::Ffi(block) => {
                for signature in &block.decls {
                    self.resolve_signature(signature);
                }
            }
            Item::Intrinsic(signature) => self.resolve_signature(signature),
            Item::GlobalLet(decl) => {
                self.resolve_type(&decl.ty);
                self.resolve_expr(&decl.value);
            }
            Item::GlobalConst(decl) => {
                self.resolve_type(&decl.ty);
                self.resolve_expr(&decl.value);
            }
        }
    }

    /// Resolves a function body with its type parameters and parameters in
    /// scope.
    fn resolve_function(&mut self, decl: &FuncDecl) {
        self.push_scope();
        self.declare_type_params(&decl.type_params);
        for param in &decl.params {
            self.resolve_type(&param.ty);
            self.declare(&param.name);
        }
        self.resolve_type(&decl.returns);
        for statement in &decl.body.statements {
            self.resolve_stmt(statement);
        }
        self.pop_scope();
    }

    /// Resolves a bodyless signature.
    fn resolve_signature(&mut self, signature: &FuncSignature) {
        self.push_scope();
        for param in &signature.params {
            self.resolve_type(&param.ty);
            self.declare(&param.name);
        }
        self.resolve_type(&signature.returns);
        self.pop_scope();
    }

    /// Declares generic parameters in the current scope.
    fn declare_type_params(&mut self, params: &[TypeParam]) {
        for param in params {
            self.declare(&param.name);
        }
    }

    /// Resolves a block in a fresh scope.
    fn resolve_block(&mut self, block: &Block) {
        self.push_scope();
        for statement in &block.statements {
            self.resolve_stmt(statement);
        }
        self.pop_scope();
    }

    /// Resolves one statement.
    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(decl) => self.resolve_let(decl),
            Stmt::Const(decl) => self.resolve_const(decl),
            Stmt::Expr(expr) => self.resolve_expr(expr),
            Stmt::If(decl) => self.resolve_if(decl),
            Stmt::While(decl) => {
                self.resolve_expr(&decl.condition);
                self.resolve_block(&decl.body);
            }
            Stmt::DoWhile(decl) => {
                self.resolve_block(&decl.body);
                self.resolve_expr(&decl.condition);
            }
            Stmt::ForC(decl) => {
                self.push_scope();
                match &decl.init {
                    Some(ForInit::Let(binding)) => self.resolve_let(binding),
                    Some(ForInit::Const(binding)) => self.resolve_const(binding),
                    Some(ForInit::Expr(exprs)) => {
                        for expr in exprs {
                            self.resolve_expr(expr);
                        }
                    }
                    None => {}
                }
                if let Some(condition) = &decl.condition {
                    self.resolve_expr(condition);
                }
                for step in &decl.step {
                    self.resolve_expr(step);
                }
                self.resolve_block(&decl.body);
                self.pop_scope();
            }
            Stmt::ForIn(decl) => {
                self.resolve_expr(&decl.iterable);
                self.push_scope();
                self.declare(&decl.name);
                self.resolve_block(&decl.body);
                self.pop_scope();
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.resolve_expr(value);
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Switch(decl) => {
                self.resolve_expr(&decl.scrutinee);
                for clause in &decl.clauses {
                    self.push_scope();
                    if let SwitchLabel::Case(value) = &clause.label {
                        self.resolve_expr(value);
                    }
                    for statement in &clause.statements {
                        self.resolve_stmt(statement);
                    }
                    self.pop_scope();
                }
            }
            Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
                self.resolve_block(block)
            }
        }
    }

    /// Resolves an `if` statement, including a chained `else if`.
    fn resolve_if(&mut self, decl: &IfStmt) {
        self.resolve_expr(&decl.condition);
        self.resolve_block(&decl.then_block);
        match &decl.else_branch {
            Some(ElseBranch::If(nested)) => self.resolve_if(nested),
            Some(ElseBranch::Block(block)) => self.resolve_block(block),
            None => {}
        }
    }

    /// Resolves a `let` binding: the initializer sees only outer bindings,
    /// because the name is declared after its own initializer.
    fn resolve_let(&mut self, decl: &LetStmt) {
        if let Some(ty) = &decl.ty {
            self.resolve_type(ty);
        }
        if let Some(value) = &decl.value {
            self.resolve_expr(value);
        }
        self.declare(&decl.name);
    }

    /// Resolves a `const` binding.
    fn resolve_const(&mut self, decl: &ConstStmt) {
        if let Some(ty) = &decl.ty {
            self.resolve_type(ty);
        }
        self.resolve_expr(&decl.value);
        self.declare(&decl.name);
    }

    /// Resolves every name inside an expression.
    fn resolve_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Name(name) => self.resolve_name_use(name, expr.span),
            ExprKind::Qualified(name) => {
                if !self.resolve_qualified(name) {
                    self.report_undeclared(&name.to_dotted(), name.span);
                }
            }
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                self.resolve_expr(callee);
                for ty in type_args {
                    self.resolve_type(ty);
                }
                for arg in args {
                    self.resolve_expr(&arg.value);
                }
            }
            ExprKind::Index { base, index } => {
                self.resolve_expr(base);
                self.resolve_expr(index);
            }
            ExprKind::Member { base, .. } => self.resolve_expr(base),
            ExprKind::Unary { operand, .. } => self.resolve_expr(operand),
            ExprKind::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            ExprKind::Assign { target, value, .. } => {
                self.resolve_expr(target);
                self.resolve_expr(value);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.resolve_expr(condition);
                self.resolve_expr(then_value);
                self.resolve_expr(else_value);
            }
            ExprKind::Cast { ty, value } | ExprKind::RawCast { ty, value } => {
                self.resolve_type(ty);
                self.resolve_expr(value);
            }
            ExprKind::StructLiteral {
                name,
                type_args,
                fields,
            } => {
                self.resolve_dotted_name(name);
                for ty in type_args {
                    self.resolve_type(ty);
                }
                for field in fields {
                    self.resolve_expr(&field.value);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.resolve_expr(element);
                }
                if let Some(fill) = fill {
                    self.resolve_expr(fill);
                }
            }
            ExprKind::Lambda {
                captures,
                params,
                body,
            } => {
                for capture in captures {
                    self.resolve_name_use(&capture.name, capture.span);
                }
                self.push_scope();
                for capture in captures {
                    self.declare(&capture.name);
                }
                for param in params {
                    self.resolve_type(&param.ty);
                    self.declare(&param.name);
                }
                match body {
                    LambdaBody::Expr(inner) => self.resolve_expr(inner),
                    LambdaBody::Block(block) => self.resolve_block(block),
                }
                self.pop_scope();
            }
            ExprKind::Unsafe(inner) => self.resolve_expr(inner),
            ExprKind::IntrinsicCall { args, .. } => {
                for arg in args {
                    match arg {
                        IntrinsicArg::Expr(inner) => self.resolve_expr(inner),
                        IntrinsicArg::Type(ty) => self.resolve_type(ty),
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let TemplatePiece::Placeholder(inner) = piece {
                        self.resolve_expr(inner);
                    }
                }
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Char(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Error => {}
        }
    }

    /// Resolves every name inside a type.
    fn resolve_type(&mut self, ty: &Type) {
        match &ty.kind {
            TypeKind::Named { name, args } => {
                self.resolve_dotted_name(name);
                for arg in args {
                    self.resolve_type(arg);
                }
            }
            TypeKind::Raw(inner) => self.resolve_type(inner),
            TypeKind::Array { element, extents } => {
                self.resolve_type(element);
                for extent in extents {
                    if let Some(size) = &extent.size {
                        self.resolve_expr(size);
                    }
                }
            }
            TypeKind::Function { params, returns } => {
                for param in params {
                    self.resolve_type(&param.ty);
                }
                self.resolve_type(returns);
            }
            TypeKind::Void
            | TypeKind::Scalar(_)
            | TypeKind::Str
            | TypeKind::CString
            | TypeKind::StaticStr
            | TypeKind::Error => {}
        }
    }

    /// Resolves a dotted name in a type or deleter position, reporting an
    /// undeclared name.
    ///
    /// A single segment is a lexical or module-scope name; a longer name is
    /// either a module path or a member of something in scope.
    fn resolve_dotted_name(&mut self, name: &QualifiedName) {
        if name.segments.len() == 1 {
            let only = &name.segments[0];
            if !self.resolve_name(only) {
                self.report_undeclared(only, name.span);
            }
            return;
        }
        if !self.resolve_qualified(name) {
            self.report_undeclared(&name.to_dotted(), name.span);
        }
    }

    /// Resolves a name reference, reporting an undeclared name.
    fn resolve_name_use(&mut self, name: &str, span: crate::span::Span) {
        if !self.resolve_name(name) {
            self.report_undeclared(name, span);
        }
    }

    /// True when the name is bound in a lexical scope, by a module-level
    /// declaration, or by an import.
    fn resolve_name(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
            || self.module.decls.contains_key(name)
            || self.module.import(name).is_some()
    }

    /// True when a qualified name resolves: either as a module path to an
    /// exported name, or as a member of something in scope.
    ///
    /// A module may reach its own private declarations by its full name, so
    /// the visibility check applies only to other modules.
    fn resolve_qualified(&self, name: &QualifiedName) -> bool {
        if let Some((last, head)) = name.segments.split_last() {
            let module = head.join(".");
            if !module.is_empty() {
                if module == self.module.name {
                    if self.module.decls.contains_key(last) {
                        return true;
                    }
                } else if self.loaded.has_module(&module)
                    && self.loaded.module_exports(&module, last)
                {
                    return true;
                }
            }
        }
        name.segments
            .first()
            .is_some_and(|first| self.resolve_name(first))
    }

    /// Declares a name in the innermost scope.
    fn declare(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned());
        }
    }

    /// Enters a new lexical scope.
    fn push_scope(&mut self) {
        self.scopes.push(BTreeSet::new());
    }

    /// Leaves the innermost lexical scope.
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Reports an undeclared name.
    fn report_undeclared(&mut self, name: &str, span: crate::span::Span) {
        self.diagnostics.push(
            Diagnostic::error("E10219", format!("'{name}' is not declared"), span)
                .with_help("check the spelling, the import, or the module"),
        );
    }
}

#[cfg(test)]
mod tests;

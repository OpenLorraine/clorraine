//! User-module references in the compiler's parsed syntax tree.
//!
//! The live symbol table supplies lexical binding visibility. A value's fields
//! must not accidentally select an unrelated module with the same head name.
//! Qualified type names use the inventory directly; type checking stays in the
//! compiler. Embedded module discovery continues to use the loader's collector.

use std::collections::BTreeSet;

use crate::ast::*;
use crate::lsp::source_members::SourceMembers;
use crate::lsp::symbols::SymbolTable;
use crate::span::FileId;

/// Collects references against the complete saved module inventory.
pub(super) fn modules(
    program: &Program,
    file: FileId,
    table: &SymbolTable,
    members: &SourceMembers<'_>,
    inventory: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut collector = Collector {
        file, module: program.module.name.to_dotted(), table, members, inventory,
        found: BTreeSet::new(),
    };
    for item in &program.items {
        collector.item(item);
    }
    collector.found
}

/// Walks parsed nodes using lexical visibility and declared member metadata.
struct Collector<'a> {
    /// File whose references are being visited; local scopes are file-specific.
    file: FileId,
    /// Merged module identity, for order-independent declarations and imports.
    module: String,
    /// Existing LSP binding and lexical-scope metadata for the saved syntax.
    table: &'a SymbolTable,
    /// Request-local full-chain metadata; never performs compiler type checking.
    members: &'a SourceMembers<'a>,
    /// All indexed user modules, including files not yet in the closure.
    inventory: &'a BTreeSet<String>,
    /// Unique module names requested by this file's syntax.
    found: BTreeSet<String>,
}

impl Collector<'_> {
    /// Distinguish value-field projections from inventory-known module prefixes.
    fn name(&mut self, name: &QualifiedName, value: bool) {
        let Some(head) = name.segments.first() else { return };
        if value && (
            self.table.local_at(self.file, head, name.span.start).is_some()
            || self.members.contains_chain(&self.module, name)
        ) {
            return;
        }
        for end in (1..name.segments.len()).rev() {
            let module = name.segments[..end].join(".");
            if self.inventory.contains(&module) {
                self.found.insert(module);
                break;
            }
        }
    }

    /// Visit generic callable bounds as well as ordinary signature types.
    fn bounds(&mut self, params: &[TypeParam]) {
        for param in params {
            if let Some(bound) = &param.bound {
                for ty in &bound.params { self.ty(ty); }
                self.ty(&bound.returns);
            }
        }
    }

    /// Visit every declaration form, including types without executable bodies.
    fn item(&mut self, item: &Item) {
        match item {
            Item::Func(f) | Item::Comptime(f) => {
                self.bounds(&f.type_params);
                for p in &f.params { self.ty(&p.ty); }
                self.ty(&f.returns);
                self.block(&f.body);
            }
            Item::Struct(s) => {
                self.bounds(&s.type_params);
                if let Some(name) = &s.freed_by { self.name(name, true); }
                for field in &s.fields { self.ty(&field.ty); }
            }
            Item::Enum(e) => {
                self.ty(&e.backing);
                for member in &e.members {
                    if let Some(value) = &member.value { self.expr(value); }
                }
            }
            Item::Handle(h) => self.name(&h.freed_by, true),
            Item::Ffi(block) => {
                for signature in &block.decls { self.signature(signature); }
            }
            Item::Intrinsic(signature) => self.signature(signature),
            Item::GlobalLet(d) => { self.ty(&d.ty); self.expr(&d.value); }
            Item::GlobalConst(d) => { self.ty(&d.ty); self.expr(&d.value); }
        }
    }

    /// FFI/intrinsic signatures may contain qualified parameter and return types.
    fn signature(&mut self, signature: &FuncSignature) {
        for p in &signature.params { self.ty(&p.ty); }
        self.ty(&signature.returns);
    }

    /// Type arguments, extents and function types can all name user modules.
    fn ty(&mut self, ty: &Type) {
        match &ty.kind {
            TypeKind::Named { name, args } => {
                self.name(name, false);
                for arg in args { self.ty(arg); }
            }
            TypeKind::Raw(inner) => self.ty(inner),
            TypeKind::Array { element, extents } => {
                self.ty(element);
                for extent in extents {
                    if let Some(size) = &extent.size { self.expr(size); }
                }
            }
            TypeKind::Function { params, returns } => {
                for param in params { self.ty(&param.ty); }
                self.ty(returns);
            }
            TypeKind::Void | TypeKind::Scalar(_) | TypeKind::Str
            | TypeKind::CString | TypeKind::StaticStr | TypeKind::Error => {}
        }
    }

    /// Recurse through each expression variant using the existing AST shapes.
    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Qualified(name) => self.name(name, true),
            ExprKind::Call { callee, type_args, args } => {
                self.expr(callee);
                for ty in type_args { self.ty(ty); }
                for arg in args { self.expr(&arg.value); }
            }
            ExprKind::Index { base, index } => { self.expr(base); self.expr(index); }
            ExprKind::Member { base, .. } => self.expr(base),
            ExprKind::Unary { operand, .. } => self.expr(operand),
            ExprKind::Binary { left, right, .. } => { self.expr(left); self.expr(right); }
            ExprKind::Assign { target, value, .. } => { self.expr(target); self.expr(value); }
            ExprKind::Conditional { condition, then_value, else_value } => {
                self.expr(condition); self.expr(then_value); self.expr(else_value);
            }
            ExprKind::Cast { ty, value } | ExprKind::RawCast { ty, value } => {
                self.ty(ty); self.expr(value);
            }
            ExprKind::StructLiteral { name, type_args, fields } => {
                self.name(name, false);
                for ty in type_args { self.ty(ty); }
                for field in fields { self.expr(&field.value); }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements { self.expr(element); }
                if let Some(fill) = fill { self.expr(fill); }
            }
            ExprKind::Lambda { params, body, .. } => {
                for param in params { self.ty(&param.ty); }
                match body {
                    LambdaBody::Expr(expr) => self.expr(expr),
                    LambdaBody::Block(block) => self.block(block),
                }
            }
            ExprKind::Unsafe(expr) => self.expr(expr),
            ExprKind::IntrinsicCall { args, .. } => {
                for arg in args {
                    match arg {
                        IntrinsicArg::Expr(expr) => self.expr(expr),
                        IntrinsicArg::Type(ty) => self.ty(ty),
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let TemplatePiece::Placeholder(expr) = piece { self.expr(expr); }
                }
            }
            ExprKind::Name(_) | ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Char(_)
            | ExprKind::Str(_) | ExprKind::Bool(_) | ExprKind::Null | ExprKind::Error => {}
        }
    }

    /// Local visibility is obtained from the shared symbol table, not reimplemented.
    fn block(&mut self, block: &Block) {
        for statement in &block.statements { self.statement(statement); }
    }

    /// Handle chained else-if syntax without losing references in any branch.
    fn branch(&mut self, branch: &IfStmt) {
        self.expr(&branch.condition);
        self.block(&branch.then_block);
        match &branch.else_branch {
            Some(ElseBranch::If(next)) => self.branch(next),
            Some(ElseBranch::Block(block)) => self.block(block),
            None => {}
        }
    }

    /// All statement variants participate, including loop headers and switch labels.
    fn statement(&mut self, statement: &Stmt) {
        match statement {
            Stmt::Let(d) => {
                if let Some(ty) = &d.ty { self.ty(ty); }
                if let Some(value) = &d.value { self.expr(value); }
            }
            Stmt::Const(d) => {
                if let Some(ty) = &d.ty { self.ty(ty); }
                self.expr(&d.value);
            }
            Stmt::Expr(expr) => self.expr(expr),
            Stmt::If(branch) => self.branch(branch),
            Stmt::While(d) => { self.expr(&d.condition); self.block(&d.body); }
            Stmt::DoWhile(d) => { self.block(&d.body); self.expr(&d.condition); }
            Stmt::ForC(d) => {
                match &d.init {
                    Some(ForInit::Let(binding)) => {
                        if let Some(ty) = &binding.ty { self.ty(ty); }
                        if let Some(value) = &binding.value { self.expr(value); }
                    }
                    Some(ForInit::Const(binding)) => {
                        if let Some(ty) = &binding.ty { self.ty(ty); }
                        self.expr(&binding.value);
                    }
                    Some(ForInit::Expr(exprs)) => { for expr in exprs { self.expr(expr); } }
                    None => {}
                }
                if let Some(condition) = &d.condition { self.expr(condition); }
                for expr in &d.step { self.expr(expr); }
                self.block(&d.body);
            }
            Stmt::ForIn(d) => { self.expr(&d.iterable); self.block(&d.body); }
            Stmt::Return { value, .. } => {
                if let Some(value) = value { self.expr(value); }
            }
            Stmt::Switch(d) => {
                self.expr(&d.scrutinee);
                for clause in &d.clauses {
                    if let SwitchLabel::Case(value) = &clause.label { self.expr(value); }
                    for statement in &clause.statements { self.statement(statement); }
                }
            }
            Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => self.block(block),
            Stmt::Break(_) | Stmt::Continue(_) => {}
        }
    }
}

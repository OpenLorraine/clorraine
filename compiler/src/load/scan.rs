//! Simple-identifier scan over a module's declarations.
//!
//! The unused-import warning needs one fact: whether a plain identifier is
//! mentioned anywhere in the module. The scan walks every declaration, type,
//! expression, and statement for that answer. It deliberately ignores dotted
//! names (qualified access works without an import) and member names after `.`
//! (those belong to the type layer), so neither marks an import as used.
//!
//! The scan is conservative in one direction: it does not track scopes, so a
//! mention anywhere marks the import used even when that mention resolves to a
//! module declaration instead. That can miss an unused-import warning, but it
//! never reports one for an import the code really uses.

use std::collections::BTreeSet;

use crate::ast::*;

/// Collects every simple identifier a module's declarations mention.
///
/// Only a plain identifier counts. A dotted name such as `std.mem.Buffer` is
/// qualified access, which is available without an import, and a member name
/// after `.` belongs to the type layer, so neither marks an import as used.
pub(super) fn collect_name_mentions(items: &[Item]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for item in items {
        match item {
            Item::Func(decl) | Item::Comptime(decl) => {
                for param in &decl.params {
                    mention_type(&param.ty, &mut names);
                }
                mention_type(&decl.returns, &mut names);
                mention_block(&decl.body, &mut names);
            }
            Item::Struct(decl) => {
                for field in &decl.fields {
                    mention_type(&field.ty, &mut names);
                }
            }
            Item::Enum(decl) => {
                mention_type(&decl.backing, &mut names);
                for member in &decl.members {
                    if let Some(value) = &member.value {
                        mention_expr(value, &mut names);
                    }
                }
            }
            Item::Handle(decl) => {
                if let [only] = decl.freed_by.segments.as_slice() {
                    names.insert(only.clone());
                }
            }
            Item::Ffi(block) => {
                for signature in &block.decls {
                    for param in &signature.params {
                        mention_type(&param.ty, &mut names);
                    }
                    mention_type(&signature.returns, &mut names);
                }
            }
            Item::Intrinsic(signature) => {
                for param in &signature.params {
                    mention_type(&param.ty, &mut names);
                }
                mention_type(&signature.returns, &mut names);
            }
            Item::GlobalLet(decl) => {
                mention_type(&decl.ty, &mut names);
                mention_expr(&decl.value, &mut names);
            }
            Item::GlobalConst(decl) => {
                mention_type(&decl.ty, &mut names);
                mention_expr(&decl.value, &mut names);
            }
        }
    }
    names
}

/// Records the simple identifier of a named type and recurses into its parts.
fn mention_type(ty: &Type, names: &mut BTreeSet<String>) {
    match &ty.kind {
        TypeKind::Named { name, args } => {
            if let [only] = name.segments.as_slice() {
                names.insert(only.clone());
            }
            for arg in args {
                mention_type(arg, names);
            }
        }
        TypeKind::Raw(inner) => mention_type(inner, names),
        TypeKind::Array { element, extents } => {
            mention_type(element, names);
            for extent in extents {
                if let Some(size) = &extent.size {
                    mention_expr(size, names);
                }
            }
        }
        TypeKind::Function { params, returns } => {
            for param in params {
                mention_type(&param.ty, names);
            }
            mention_type(returns, names);
        }
        TypeKind::Void
        | TypeKind::Scalar(_)
        | TypeKind::Str
        | TypeKind::CString
        | TypeKind::StaticStr
        | TypeKind::Error => {}
    }
}

/// Records every simple identifier inside an expression.
fn mention_expr(expr: &Expr, names: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name(name) => {
            names.insert(name.clone());
        }
        ExprKind::Call {
            callee,
            type_args,
            args,
        } => {
            mention_expr(callee, names);
            for ty in type_args {
                mention_type(ty, names);
            }
            for arg in args {
                mention_expr(&arg.value, names);
            }
        }
        ExprKind::Index { base, index } => {
            mention_expr(base, names);
            mention_expr(index, names);
        }
        ExprKind::Member { base, .. } => mention_expr(base, names),
        ExprKind::Unary { operand, .. } => mention_expr(operand, names),
        ExprKind::Binary { left, right, .. } => {
            mention_expr(left, names);
            mention_expr(right, names);
        }
        ExprKind::Assign { target, value, .. } => {
            mention_expr(target, names);
            mention_expr(value, names);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            mention_expr(condition, names);
            mention_expr(then_value, names);
            mention_expr(else_value, names);
        }
        ExprKind::Cast { ty, value } | ExprKind::RawCast { ty, value } => {
            mention_type(ty, names);
            mention_expr(value, names);
        }
        ExprKind::StructLiteral {
            name,
            type_args,
            fields,
        } => {
            if let [only] = name.segments.as_slice() {
                names.insert(only.clone());
            }
            for ty in type_args {
                mention_type(ty, names);
            }
            for field in fields {
                mention_expr(&field.value, names);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                mention_expr(element, names);
            }
            if let Some(fill) = fill {
                mention_expr(fill, names);
            }
        }
        ExprKind::Lambda {
            captures,
            params,
            body,
        } => {
            for capture in captures {
                names.insert(capture.name.clone());
            }
            for param in params {
                mention_type(&param.ty, names);
            }
            match body {
                LambdaBody::Expr(inner) => mention_expr(inner, names),
                LambdaBody::Block(block) => mention_block(block, names),
            }
        }
        ExprKind::Unsafe(inner) => mention_expr(inner, names),
        ExprKind::IntrinsicCall { args, .. } => {
            for arg in args {
                match arg {
                    IntrinsicArg::Expr(inner) => mention_expr(inner, names),
                    IntrinsicArg::Type(ty) => mention_type(ty, names),
                }
            }
        }
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let TemplatePiece::Placeholder(inner) = piece {
                    mention_expr(inner, names);
                }
            }
        }
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::Null
        | ExprKind::Qualified(_)
        | ExprKind::Error => {}
    }
}

/// Records every simple identifier inside a block.
fn mention_block(block: &Block, names: &mut BTreeSet<String>) {
    for statement in &block.statements {
        mention_stmt(statement, names);
    }
}

/// Records every simple identifier inside an `if` statement, including a
/// chained `else if`.
fn mention_if(decl: &IfStmt, names: &mut BTreeSet<String>) {
    mention_expr(&decl.condition, names);
    mention_block(&decl.then_block, names);
    match &decl.else_branch {
        Some(ElseBranch::If(nested)) => mention_if(nested, names),
        Some(ElseBranch::Block(block)) => mention_block(block, names),
        None => {}
    }
}

/// Records every simple identifier inside a statement.
fn mention_stmt(stmt: &Stmt, names: &mut BTreeSet<String>) {
    match stmt {
        Stmt::Let(decl) => {
            if let Some(ty) = &decl.ty {
                mention_type(ty, names);
            }
            if let Some(value) = &decl.value {
                mention_expr(value, names);
            }
        }
        Stmt::Const(decl) => {
            if let Some(ty) = &decl.ty {
                mention_type(ty, names);
            }
            mention_expr(&decl.value, names);
        }
        Stmt::Expr(expr) => mention_expr(expr, names),
        Stmt::If(decl) => mention_if(decl, names),
        Stmt::While(decl) => {
            mention_expr(&decl.condition, names);
            mention_block(&decl.body, names);
        }
        Stmt::DoWhile(decl) => {
            mention_block(&decl.body, names);
            mention_expr(&decl.condition, names);
        }
        Stmt::ForC(decl) => {
            match &decl.init {
                Some(ForInit::Let(binding)) => {
                    if let Some(ty) = &binding.ty {
                        mention_type(ty, names);
                    }
                    if let Some(value) = &binding.value {
                        mention_expr(value, names);
                    }
                }
                Some(ForInit::Const(binding)) => {
                    if let Some(ty) = &binding.ty {
                        mention_type(ty, names);
                    }
                    mention_expr(&binding.value, names);
                }
                Some(ForInit::Expr(exprs)) => {
                    for expr in exprs {
                        mention_expr(expr, names);
                    }
                }
                None => {}
            }
            if let Some(condition) = &decl.condition {
                mention_expr(condition, names);
            }
            for step in &decl.step {
                mention_expr(step, names);
            }
            mention_block(&decl.body, names);
        }
        Stmt::ForIn(decl) => {
            mention_expr(&decl.iterable, names);
            mention_block(&decl.body, names);
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                mention_expr(value, names);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Switch(decl) => {
            mention_expr(&decl.scrutinee, names);
            for clause in &decl.clauses {
                if let SwitchLabel::Case(value) = &clause.label {
                    mention_expr(value, names);
                }
                for statement in &clause.statements {
                    mention_stmt(statement, names);
                }
            }
        }
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
            mention_block(block, names)
        }
    }
}

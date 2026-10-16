//! Simple-identifier and qualified-module scan over a module's declarations.
//!
//! The unused-import warning needs one fact: whether a plain identifier is
//! mentioned anywhere in the module. The scan walks every declaration, type,
//! expression, and statement for that answer. It deliberately ignores dotted
//! names that qualify a module (qualified access works without an import) and
//! member names after `.` (those belong to the type layer), so neither marks
//! an import as used.
//!
//! The same walk also records the standard-library modules a dotted name
//! names, so the driver can inject the module a qualified reference such as
//! `std.mem.bufferNew(4)` needs even when the file writes no `import`.
//!
//! The scan is conservative in one direction: it does not track scopes, so a
//! mention anywhere marks the import used even when a same-named local
//! shadows it at that spot. That can miss an unused-import warning, but it
//! never reports one for an import the code really uses.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::*;
use crate::span::Span;

/// The identifiers and standard-library modules a module's declarations
/// mention.
#[derive(Default)]
pub(super) struct Mentions {
    /// Every simple identifier mentioned.
    pub names: BTreeSet<String>,
    /// Every standard-library module named by a dotted reference, with the
    /// span of the first reference to it.
    pub modules: BTreeMap<String, Span>,
    /// The loaded module paths, read while walking: a dotted name whose
    /// leading segments spell one of them names that module and not a value
    /// binding.
    loaded_modules: BTreeSet<String>,
}

/// Collects every simple identifier and qualified standard-library module a
/// module's declarations mention.
///
/// A plain identifier counts as a name. A dotted name that names a
/// standard-library module such as `std.mem.Buffer` is qualified access, which
/// is available without an import, so it does not mark an import used; its
/// longest standard-library prefix is recorded as a module reference instead.
/// A dotted name whose leading segments spell one of `loaded_modules` names
/// that module rather than a value binding, so it marks no import used either.
/// Any other dotted name counts its head as a name exactly as a plain
/// identifier does. `include_staged` controls whether a staged `comptime` body
/// is walked: the profile-surface check leaves it out so the semantic stage's
/// staged-feature diagnostic is the one reported.
pub(super) fn collect_name_mentions(
    items: &[Item],
    include_staged: bool,
    loaded_modules: &BTreeSet<String>,
) -> Mentions {
    let mut mentions = Mentions {
        loaded_modules: loaded_modules.clone(),
        ..Mentions::default()
    };
    for item in items {
        match item {
            Item::Func(decl) => {
                mention_signature(decl, &mut mentions);
            }
            Item::Comptime(decl) if include_staged => {
                mention_signature(decl, &mut mentions);
            }
            Item::Comptime(_) => {}
            Item::Struct(decl) => {
                for field in &decl.fields {
                    mention_type(&field.ty, &mut mentions);
                }
            }
            Item::Enum(decl) => {
                mention_type(&decl.backing, &mut mentions);
                for member in &decl.members {
                    if let Some(value) = &member.value {
                        mention_expr(value, &mut mentions);
                    }
                }
            }
            Item::Handle(decl) => {
                if let [only] = decl.freed_by.segments.as_slice() {
                    mentions.names.insert(only.clone());
                }
            }
            Item::Ffi(block) => {
                for signature in &block.decls {
                    for param in &signature.params {
                        mention_type(&param.ty, &mut mentions);
                    }
                    mention_type(&signature.returns, &mut mentions);
                }
            }
            Item::Intrinsic(signature) => {
                for param in &signature.params {
                    mention_type(&param.ty, &mut mentions);
                }
                mention_type(&signature.returns, &mut mentions);
            }
            Item::GlobalLet(decl) => {
                mention_type(&decl.ty, &mut mentions);
                mention_expr(&decl.value, &mut mentions);
            }
            Item::GlobalConst(decl) => {
                mention_type(&decl.ty, &mut mentions);
                mention_expr(&decl.value, &mut mentions);
            }
        }
    }
    mentions
}

/// Records the names and module references of a function declaration.
///
/// A `func` body and a staged `comptime` body share this walk, so the two
/// callers stay in step about which parts of a body are scanned.
fn mention_signature(decl: &FuncDecl, mentions: &mut Mentions) {
    for param in &decl.params {
        mention_type(&param.ty, mentions);
    }
    mention_type(&decl.returns, mentions);
    mention_block(&decl.body, mentions);
}

/// Records the longest standard-library module prefix of a dotted path.
///
/// A path such as `std.mem.bufferNew` names the module `std.mem`; a path such
/// as `point.x` names no standard module and is ignored, so an ordinary field
/// access never asks for a module injection. The first span seen for a module
/// is kept, so a later profile-surface diagnostic can point at the reference.
fn mention_module(segments: &[String], span: Span, mentions: &mut Mentions) -> bool {
    if segments.len() < 2 {
        return false;
    }
    for end in (1..segments.len()).rev() {
        let candidate = segments[..end].join(".");
        if crate::stdlib::lookup(&candidate).is_some() {
            mentions.modules.entry(candidate).or_insert(span);
            return true;
        }
    }
    false
}

/// True when the leading segments of a dotted path spell a loaded module.
///
/// A reference such as `probe.dep.member` names the module `probe.dep` before
/// its member, so its head is a module segment rather than a value binding and
/// marks no import as used. The whole path is checked too, so a reference that
/// names only a module stays out.
fn begins_loaded_module(segments: &[String], loaded_modules: &BTreeSet<String>) -> bool {
    (1..=segments.len())
        .rev()
        .any(|end| loaded_modules.contains(&segments[..end].join(".")))
}

/// Records the simple identifier of a named type and recurses into its parts.
fn mention_type(ty: &Type, mentions: &mut Mentions) {
    match &ty.kind {
        TypeKind::Named { name, args } => {
            if let [only] = name.segments.as_slice() {
                mentions.names.insert(only.clone());
            } else {
                let _ = mention_module(&name.segments, ty.span, mentions);
            }
            for arg in args {
                mention_type(arg, mentions);
            }
        }
        TypeKind::Raw(inner) => mention_type(inner, mentions),
        TypeKind::Array { element, extents } => {
            mention_type(element, mentions);
            for extent in extents {
                if let Some(size) = &extent.size {
                    mention_expr(size, mentions);
                }
            }
        }
        TypeKind::Function { params, returns } => {
            for param in params {
                mention_type(&param.ty, mentions);
            }
            mention_type(returns, mentions);
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
fn mention_expr(expr: &Expr, mentions: &mut Mentions) {
    match &expr.kind {
        ExprKind::Name(name) => {
            mentions.names.insert(name.clone());
        }
        ExprKind::Call {
            callee,
            type_args,
            args,
        } => {
            mention_expr(callee, mentions);
            for ty in type_args {
                mention_type(ty, mentions);
            }
            for arg in args {
                mention_expr(&arg.value, mentions);
            }
        }
        ExprKind::Index { base, index } => {
            mention_expr(base, mentions);
            mention_expr(index, mentions);
        }
        ExprKind::Member { base, .. } => mention_expr(base, mentions),
        ExprKind::Unary { operand, .. } => mention_expr(operand, mentions),
        ExprKind::Binary { left, right, .. } => {
            mention_expr(left, mentions);
            mention_expr(right, mentions);
        }
        ExprKind::Assign { target, value, .. } => {
            mention_expr(target, mentions);
            mention_expr(value, mentions);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            mention_expr(condition, mentions);
            mention_expr(then_value, mentions);
            mention_expr(else_value, mentions);
        }
        ExprKind::Cast { ty, value } | ExprKind::RawCast { ty, value } => {
            mention_type(ty, mentions);
            mention_expr(value, mentions);
        }
        ExprKind::StructLiteral {
            name,
            type_args,
            fields,
        } => {
            if let [only] = name.segments.as_slice() {
                mentions.names.insert(only.clone());
            } else {
                mention_module(&name.segments, name.span, mentions);
            }
            for ty in type_args {
                mention_type(ty, mentions);
            }
            for field in fields {
                mention_expr(&field.value, mentions);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                mention_expr(element, mentions);
            }
            if let Some(fill) = fill {
                mention_expr(fill, mentions);
            }
        }
        ExprKind::Lambda {
            captures,
            params,
            body,
        } => {
            for capture in captures {
                mentions.names.insert(capture.name.clone());
            }
            for param in params {
                mention_type(&param.ty, mentions);
            }
            match body {
                LambdaBody::Expr(inner) => mention_expr(inner, mentions),
                LambdaBody::Block(block) => mention_block(block, mentions),
            }
        }
        ExprKind::Unsafe(inner) => mention_expr(inner, mentions),
        ExprKind::IntrinsicCall { args, .. } => {
            for arg in args {
                match arg {
                    IntrinsicArg::Expr(inner) => mention_expr(inner, mentions),
                    IntrinsicArg::Type(ty) => mention_type(ty, mentions),
                }
            }
        }
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let TemplatePiece::Placeholder(inner) = piece {
                    mention_expr(inner, mentions);
                }
            }
        }
        ExprKind::Qualified(name) => {
            // A dotted path that names a standard-library module is qualified
            // access, which needs no import; a path whose leading segments
            // spell a loaded module names that module, so its head is a
            // module segment rather than a value binding. Every other dotted
            // name reads a value binding at its head (`h.call` reads the
            // imported global `h`), so the head counts as a name mention
            // exactly as a plain identifier does.
            if !mention_module(&name.segments, name.span, mentions)
                && !begins_loaded_module(&name.segments, &mentions.loaded_modules)
            {
                if let Some(first) = name.segments.first() {
                    mentions.names.insert(first.clone());
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

/// Records every simple identifier inside a block.
fn mention_block(block: &Block, mentions: &mut Mentions) {
    for statement in &block.statements {
        mention_stmt(statement, mentions);
    }
}

/// Records every simple identifier inside an `if` statement, including a
/// chained `else if`.
fn mention_if(decl: &IfStmt, mentions: &mut Mentions) {
    mention_expr(&decl.condition, mentions);
    mention_block(&decl.then_block, mentions);
    match &decl.else_branch {
        Some(ElseBranch::If(nested)) => mention_if(nested, mentions),
        Some(ElseBranch::Block(block)) => mention_block(block, mentions),
        None => {}
    }
}

/// Records every simple identifier inside a statement.
fn mention_stmt(stmt: &Stmt, mentions: &mut Mentions) {
    match stmt {
        Stmt::Let(decl) => {
            if let Some(ty) = &decl.ty {
                mention_type(ty, mentions);
            }
            if let Some(value) = &decl.value {
                mention_expr(value, mentions);
            }
        }
        Stmt::Const(decl) => {
            if let Some(ty) = &decl.ty {
                mention_type(ty, mentions);
            }
            mention_expr(&decl.value, mentions);
        }
        Stmt::Expr(expr) => mention_expr(expr, mentions),
        Stmt::If(decl) => mention_if(decl, mentions),
        Stmt::While(decl) => {
            mention_expr(&decl.condition, mentions);
            mention_block(&decl.body, mentions);
        }
        Stmt::DoWhile(decl) => {
            mention_block(&decl.body, mentions);
            mention_expr(&decl.condition, mentions);
        }
        Stmt::ForC(decl) => {
            match &decl.init {
                Some(ForInit::Let(binding)) => {
                    if let Some(ty) = &binding.ty {
                        mention_type(ty, mentions);
                    }
                    if let Some(value) = &binding.value {
                        mention_expr(value, mentions);
                    }
                }
                Some(ForInit::Const(binding)) => {
                    if let Some(ty) = &binding.ty {
                        mention_type(ty, mentions);
                    }
                    mention_expr(&binding.value, mentions);
                }
                Some(ForInit::Expr(exprs)) => {
                    for expr in exprs {
                        mention_expr(expr, mentions);
                    }
                }
                None => {}
            }
            if let Some(condition) = &decl.condition {
                mention_expr(condition, mentions);
            }
            for step in &decl.step {
                mention_expr(step, mentions);
            }
            mention_block(&decl.body, mentions);
        }
        Stmt::ForIn(decl) => {
            mention_expr(&decl.iterable, mentions);
            mention_block(&decl.body, mentions);
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                mention_expr(value, mentions);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Switch(decl) => {
            mention_expr(&decl.scrutinee, mentions);
            for clause in &decl.clauses {
                if let SwitchLabel::Case(value) = &clause.label {
                    mention_expr(value, mentions);
                }
                for statement in &clause.statements {
                    mention_stmt(statement, mentions);
                }
            }
        }
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
            mention_block(block, mentions)
        }
    }
}

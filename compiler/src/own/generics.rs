//! Generic instantiation and copy-obligation collection.
//!
//! A generic body is checked once, but codegen needs one concrete copy per
//! distinct instantiation. This pass walks every function body, records each
//! call that instantiates a generic function, and deduplicates the instances
//! by a stable key. It also turns the copy obligation a generic writable lend
//! implies into a concrete obligation per instance: when a `borrow x: T`
//! parameter is instantiated with a duplicable type, the lend binds a copy, so
//! the instance carries that copy duty. The recursion cap itself is checked by
//! the type stage, which owns the `E10305` diagnostic.

use std::collections::BTreeMap;

use crate::ast::{Block, Expr, ExprKind, Item, Mode, Stmt, Type, TypeParam};
use crate::ir::{
    CopyObligation, Function, Instance, Monomorphization, Statement, StructInstance, Terminator,
    Value, ValueKind,
};
use crate::load::Loaded;
use crate::sema::SemaOutput;
use crate::types::{resolve_ty, Ty, TyKind};

use super::program::{Callee, ProgramInfo};

/// Collects the instantiation set and copy obligations of the program.
///
/// Only explicit type arguments are recorded. An inferred instantiation has no
/// written spelling to key on, so it is left to the type stage's inference,
/// which already validates it; this pass never guesses a key.
pub fn collect(
    loaded: &Loaded,
    program: &ProgramInfo,
    sema: &SemaOutput,
) -> Monomorphization {
    let mut mono = Monomorphization::default();
    for module in loaded.modules.values() {
        for item in &module.items {
            if let Item::Func(decl) = item {
                walk_block(&decl.body, program, sema, &mut mono);
            }
        }
    }
    collect_struct_instances(loaded, sema, &mut mono);
    mono
}

/// Records the concrete generic-struct instantiations the declarations name.
///
/// A struct field, function parameter, or return type that spells a concrete
/// instantiation (for example `Pair<i32>`) pins a C struct the emitter must
/// write. A type that still mentions a type parameter belongs to an
/// uninstantiated template and is skipped; its concrete forms are recorded
/// where it is used.
pub fn collect_struct_instances(loaded: &Loaded, sema: &SemaOutput, mono: &mut Monomorphization) {
    for module in loaded.modules.values() {
        for item in &module.items {
            match item {
                Item::Struct(decl) => {
                    let params = type_param_names(&decl.type_params);
                    for field in &decl.fields {
                        record_struct_type(&resolve_ty(&field.ty, &params, &sema.table), sema, mono);
                    }
                }
                Item::Func(decl) | Item::Comptime(decl) => {
                    let params = type_param_names(&decl.type_params);
                    for param in &decl.params {
                        record_struct_type(&resolve_ty(&param.ty, &params, &sema.table), sema, mono);
                    }
                    record_struct_type(&resolve_ty(&decl.returns, &params, &sema.table), sema, mono);
                }
                Item::GlobalLet(decl) => {
                    record_struct_type(&resolve_ty(&decl.ty, &[], &sema.table), sema, mono);
                }
                Item::GlobalConst(decl) => {
                    record_struct_type(&resolve_ty(&decl.ty, &[], &sema.table), sema, mono);
                }
                _ => {}
            }
        }
    }
}

/// Collects a declaration's written type-parameter names.
fn type_param_names(params: &[TypeParam]) -> Vec<String> {
    params.iter().map(|param| param.name.clone()).collect()
}

/// Records every concrete generic-struct instantiation a type names.
///
/// The walk recurses through the type so a struct nested in an array, pointer,
/// function signature, or another struct's argument list is recorded too. A
/// struct whose arguments are not all concrete is skipped.
fn record_struct_type(ty: &Ty, sema: &SemaOutput, mono: &mut Monomorphization) {
    match &ty.kind {
        TyKind::Struct(name, args) => {
            for arg in args {
                record_struct_type(arg, sema, mono);
            }
            let is_instance = sema.table.struct_type_params(name).is_some_and(|params| {
                !params.is_empty() && params.len() == args.len() && args.iter().all(is_concrete)
            });
            if is_instance {
                mono.record_struct(StructInstance {
                    name: name.clone(),
                    args: args.clone(),
                });
            }
        }
        TyKind::Array { element, .. } => record_struct_type(element, sema, mono),
        TyKind::Raw(inner) => record_struct_type(inner, sema, mono),
        TyKind::Func(signature) => {
            for param in &signature.params {
                record_struct_type(param, sema, mono);
            }
            record_struct_type(&signature.returns, sema, mono);
        }
        _ => {}
    }
}

/// True when a type is fully concrete, with no generic parameter or unresolved
/// placeholder anywhere inside it.
fn is_concrete(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Param(_) | TyKind::Unknown => false,
        TyKind::Array { element, .. } => is_concrete(element),
        TyKind::Raw(inner) => is_concrete(inner),
        TyKind::Struct(_, args) => args.iter().all(is_concrete),
        TyKind::Func(signature) => {
            signature.params.iter().all(is_concrete) && is_concrete(&signature.returns)
        }
        _ => true,
    }
}

/// Walks a block looking for generic call sites.
fn walk_block(
    block: &Block,
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
) {
    for statement in &block.statements {
        walk_stmt(statement, program, sema, mono);
    }
}

/// Walks one statement.
fn walk_stmt(
    stmt: &Stmt,
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
) {
    match stmt {
        Stmt::Let(decl) => {
            if let Some(value) = &decl.value {
                walk_expr(value, program, sema, mono);
            }
        }
        Stmt::Const(decl) => walk_expr(&decl.value, program, sema, mono),
        Stmt::Expr(expr) => walk_expr(expr, program, sema, mono),
        Stmt::If(decl) => walk_if(decl, program, sema, mono),
        Stmt::While(decl) => {
            walk_expr(&decl.condition, program, sema, mono);
            walk_block(&decl.body, program, sema, mono);
        }
        Stmt::DoWhile(decl) => {
            walk_block(&decl.body, program, sema, mono);
            walk_expr(&decl.condition, program, sema, mono);
        }
        Stmt::ForC(decl) => {
            match &decl.init {
                Some(crate::ast::ForInit::Expr(expressions)) => {
                    for expression in expressions {
                        walk_expr(expression, program, sema, mono);
                    }
                }
                Some(crate::ast::ForInit::Let(binding)) => {
                    if let Some(value) = &binding.value {
                        walk_expr(value, program, sema, mono);
                    }
                }
                Some(crate::ast::ForInit::Const(binding)) => {
                    walk_expr(&binding.value, program, sema, mono);
                }
                None => {}
            }
            if let Some(condition) = &decl.condition {
                walk_expr(condition, program, sema, mono);
            }
            for step in &decl.step {
                walk_expr(step, program, sema, mono);
            }
            walk_block(&decl.body, program, sema, mono);
        }
        Stmt::ForIn(decl) => {
            walk_expr(&decl.iterable, program, sema, mono);
            walk_block(&decl.body, program, sema, mono);
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                walk_expr(value, program, sema, mono);
            }
        }
        Stmt::Switch(decl) => {
            walk_expr(&decl.scrutinee, program, sema, mono);
            for clause in &decl.clauses {
                for statement in &clause.statements {
                    walk_stmt(statement, program, sema, mono);
                }
            }
        }
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
            walk_block(block, program, sema, mono)
        }
        Stmt::Break(_) | Stmt::Continue(_) => {}
    }
}

/// Walks an `if`, including any chained `else if`.
fn walk_if(
    decl: &crate::ast::IfStmt,
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
) {
    walk_expr(&decl.condition, program, sema, mono);
    walk_block(&decl.then_block, program, sema, mono);
    match &decl.else_branch {
        Some(crate::ast::ElseBranch::If(nested)) => walk_if(nested, program, sema, mono),
        Some(crate::ast::ElseBranch::Block(block)) => walk_block(block, program, sema, mono),
        None => {}
    }
}

/// Walks an expression, recording any generic instantiation it names.
fn walk_expr(
    expr: &Expr,
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
) {
    match &expr.kind {
        ExprKind::Call {
            callee,
            type_args,
            args,
        } => {
            record_call(callee, type_args, program, sema, mono);
            walk_expr(callee, program, sema, mono);
            for arg in args {
                walk_expr(&arg.value, program, sema, mono);
            }
        }
        ExprKind::Index { base, index } => {
            walk_expr(base, program, sema, mono);
            walk_expr(index, program, sema, mono);
        }
        ExprKind::Member { base, .. } => walk_expr(base, program, sema, mono),
        ExprKind::Unary { operand, .. } => walk_expr(operand, program, sema, mono),
        ExprKind::Binary { left, right, .. } => {
            walk_expr(left, program, sema, mono);
            walk_expr(right, program, sema, mono);
        }
        ExprKind::Assign { target, value, .. } => {
            walk_expr(target, program, sema, mono);
            walk_expr(value, program, sema, mono);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            walk_expr(condition, program, sema, mono);
            walk_expr(then_value, program, sema, mono);
            walk_expr(else_value, program, sema, mono);
        }
        ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
            walk_expr(value, program, sema, mono)
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                walk_expr(&field.value, program, sema, mono);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                walk_expr(element, program, sema, mono);
            }
            if let Some(fill) = fill {
                walk_expr(fill, program, sema, mono);
            }
        }
        ExprKind::Unsafe(inner) => walk_expr(inner, program, sema, mono),
        ExprKind::IntrinsicCall { args, .. } => {
            for arg in args {
                if let crate::ast::IntrinsicArg::Expr(inner) = arg {
                    walk_expr(inner, program, sema, mono);
                }
            }
        }
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let crate::ast::TemplatePiece::Placeholder(inner) = piece {
                    walk_expr(inner, program, sema, mono);
                }
            }
        }
        ExprKind::Lambda { body, .. } => match body {
            crate::ast::LambdaBody::Expr(inner) => walk_expr(inner, program, sema, mono),
            crate::ast::LambdaBody::Block(block) => walk_block(block, program, sema, mono),
        },
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::Null
        | ExprKind::Name(_)
        | ExprKind::Qualified(_)
        | ExprKind::Error => {}
    }
}

/// Records one explicit generic call and its copy obligations.
fn record_call(
    callee: &Expr,
    type_args: &[Type],
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
) {
    if type_args.is_empty() {
        return;
    }
    let Some(name) = callee_simple_name(callee) else {
        return;
    };
    let Some(Callee::User(decl)) = program.callee(&name) else {
        return;
    };
    if decl.type_params.len() != type_args.len() {
        return;
    }
    let bindings = type_bindings(decl, type_args, sema);
    let instance = Instance {
        function: decl.name.clone(),
        args: type_args
            .iter()
            .map(|ty| resolve_ty(ty, &[], &sema.table).display())
            .collect(),
    };
    let key = instance.dedupe_key();
    if !mono.record(instance) {
        return;
    }
    let parameter_names: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
    for param in &decl.params {
        if param.mode != Mode::Borrow {
            continue;
        }
        let declared = resolve_ty(&param.ty, &parameter_names, &sema.table);
        let TyKind::Param(parameter) = &declared.kind else {
            continue;
        };
        let Some(concrete) = bindings.get(parameter) else {
            continue;
        };
        if sema.table.duplicable(concrete) && !crate::borrow::is_view(concrete) {
            mono.require_copy(CopyObligation {
                function: decl.name.clone(),
                instance: key.clone(),
                parameter: param.name.clone(),
                ty: concrete.display(),
            });
        }
    }
}

/// Binds each generic parameter to the concrete type its type argument names.
fn type_bindings(
    decl: &crate::ast::FuncDecl,
    type_args: &[Type],
    sema: &SemaOutput,
) -> BTreeMap<String, Ty> {
    let mut bindings = BTreeMap::new();
    for (param, written) in decl.type_params.iter().zip(type_args) {
        bindings.insert(param.name.clone(), resolve_ty(written, &[], &sema.table));
    }
    bindings
}

/// Records the concrete generic instances named by the lowered IR.
///
/// Lowering fills a generic call's concrete type arguments, including an
/// inferred instantiation of a dot-call, so walking the IR catches the
/// instances that the AST-only pass cannot key. A call whose arguments still
/// name a type parameter belongs to an uninstantiated generic template and is
/// skipped.
pub fn collect_ir_instances(
    functions: &[Function],
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
) {
    for function in functions {
        record_struct_type(&function.ret, sema, mono);
        for param in &function.params {
            record_struct_type(&param.ty, sema, mono);
        }
        for local in &function.locals {
            record_struct_type(&local.ty, sema, mono);
        }
        for temp in &function.temps {
            record_struct_type(temp, sema, mono);
        }
        for block in &function.blocks {
            for instr in &block.statements {
                match &instr.statement {
                    Statement::Assign { value, .. } => walk_value(value, program, sema, mono),
                    Statement::StorePointer { pointer, value } => {
                        walk_value(pointer, program, sema, mono);
                        walk_value(value, program, sema, mono);
                    }
                    Statement::Call { callee, type_args, args, .. } => {
                        record_instance(callee, type_args, program, mono);
                        for arg in args {
                            walk_value(arg, program, sema, mono);
                        }
                    }
                    Statement::Discard { value } => walk_value(value, program, sema, mono),
                    Statement::Move { .. }
                    | Statement::MoveOut { .. }
                    | Statement::Borrow { .. }
                    | Statement::Drop { .. }
                    | Statement::Defer { .. }
                    | Statement::ScopeEnd { .. }
                    | Statement::ScopeExit { .. }
                    | Statement::Nop => {}
                }
            }
            match &block.terminator {
                Terminator::Branch { condition, .. } => walk_value(condition, program, sema, mono),
                Terminator::Switch { value, arms, .. } => {
                    walk_value(value, program, sema, mono);
                    for (label, _) in arms {
                        walk_value(label, program, sema, mono);
                    }
                }
                Terminator::Return(Some(value)) => walk_value(value, program, sema, mono),
                _ => {}
            }
        }
    }
}

/// Records an instance for one IR call with concrete type arguments.
fn record_instance(
    callee: &str,
    type_args: &[Ty],
    program: &ProgramInfo,
    mono: &mut Monomorphization,
) {
    if type_args.is_empty()
        || type_args
            .iter()
            .any(|ty| ty.is_param() || ty.is_unknown())
    {
        return;
    }
    let Some(Callee::User(decl)) = program.callee(callee) else {
        return;
    };
    if decl.type_params.len() != type_args.len() {
        return;
    }
    mono.record(Instance {
        function: decl.name.clone(),
        args: type_args.iter().map(Ty::display).collect(),
    });
}

/// Walks a value, recording every generic call it contains.
fn walk_value(value: &Value, program: &ProgramInfo, sema: &SemaOutput, mono: &mut Monomorphization) {
    record_struct_type(&value.ty, sema, mono);
    match &value.kind {
        ValueKind::Call {
            callee,
            type_args,
            args,
            receiver,
        } => {
            record_instance(callee, type_args, program, mono);
            if let Some(receiver) = receiver {
                walk_value(receiver, program, sema, mono);
            }
            for arg in args {
                walk_value(arg, program, sema, mono);
            }
        }
        ValueKind::Member { base, .. } => walk_value(base, program, sema, mono),
        ValueKind::Index { base, index } => {
            walk_value(base, program, sema, mono);
            walk_value(index, program, sema, mono);
        }
        ValueKind::Unary { operand, .. } => walk_value(operand, program, sema, mono),
        ValueKind::Binary { left, right, .. } => {
            walk_value(left, program, sema, mono);
            walk_value(right, program, sema, mono);
        }
        ValueKind::Cast { value, .. } => walk_value(value, program, sema, mono),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            walk_value(condition, program, sema, mono);
            walk_value(then_value, program, sema, mono);
            walk_value(else_value, program, sema, mono);
        }
        ValueKind::StructLit { fields, .. } => {
            for (_, field) in fields {
                walk_value(field, program, sema, mono);
            }
        }
        ValueKind::ArrayLit { elements, fill } => {
            for element in elements {
                walk_value(element, program, sema, mono);
            }
            if let Some(fill) = fill {
                walk_value(fill, program, sema, mono);
            }
        }
        ValueKind::Template(pieces) => {
            for piece in pieces {
                if let crate::ir::ValuePiece::Placeholder(inner) = piece {
                    walk_value(inner, program, sema, mono);
                }
            }
        }
        ValueKind::Intrinsic { args, .. } => {
            for arg in args {
                walk_value(arg, program, sema, mono);
            }
        }
        ValueKind::Materialize { value, .. } => walk_value(value, program, sema, mono),
        ValueKind::Place(_)
        | ValueKind::Int(_)
        | ValueKind::Float(_)
        | ValueKind::Char(_)
        | ValueKind::Bool(_)
        | ValueKind::Null
        | ValueKind::Str(_)
        | ValueKind::Path(_)
        | ValueKind::Aggregate
        | ValueKind::Temp(_) => {}
    }
}

/// The simple name a callee expression carries.
fn callee_simple_name(callee: &Expr) -> Option<String> {
    match &callee.kind {
        ExprKind::Name(name) => Some(name.clone()),
        ExprKind::Qualified(name) => name.segments.last().cloned(),
        ExprKind::Member { name, .. } => Some(name.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Diagnostics;
    use crate::parse::Parser;
    use crate::span::SourceMap;

    /// Parses, loads, and types one source, returning its generic inputs.
    fn inputs(source: &str) -> Monomorphization {
        let mut sources = SourceMap::new();
        let mut diagnostics = Diagnostics::new(false, None);
        let id = sources.add_file("test.clo".to_owned(), source.to_owned());
        let program = Parser::new(&sources, id, &mut diagnostics).parse_program();
        let loaded = crate::load::load(vec![(id, program)], &mut diagnostics);
        let sema = crate::sema::check(&loaded, crate::profile::Profile::Host, &mut diagnostics);
        let info = ProgramInfo::collect(&loaded, &sema.table);
        collect(&loaded, &info, &sema)
    }

    #[test]
    fn explicit_generic_calls_become_deduplicated_instances() {
        let mono = inputs(
            r#"module test;
func identity<T>(x: T): T { return x; }
func demo(): i32 {
    let a = identity<i32>(1);
    let b = identity<i32>(2);
    let c = identity<u8>(3);
    return a;
}
func main(): i32 { return 0; }
"#,
        );
        assert_eq!(mono.instances().len(), 2);
    }

    #[test]
    fn a_writable_lend_of_a_duplicable_type_records_a_copy_obligation() {
        let mono = inputs(
            r#"module test;
func set<T>(borrow slot: T, value: T): void { slot = value; }
func demo(): void {
    let n: i32 = 0;
    set<i32>(borrow n, 1);
}
func main(): i32 { return 0; }
"#,
        );
        assert_eq!(mono.instances().len(), 1);
        assert_eq!(mono.obligations().len(), 1);
        assert_eq!(mono.obligations()[0].instance, "set<i32>");
    }
}



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

use crate::ast::{Arg, Block, Expr, ExprKind, Item, Mode, Stmt, StructDecl, Type, TypeParam};
use crate::ir::{
    CopyObligation, Function, Instance, LiftedSpecialization, Monomorphization, Projection,
    Statement, StructInstance, Terminator, Value, ValueKind, ValuePiece,
};
use crate::load::Loaded;
use crate::sema::SemaOutput;
use crate::types::{resolve_ty, substitute, substitute_instance, ClosureSite, Ty, TyKind};

use super::program::{Callee, ProgramInfo};

/// Collects the instantiation set and copy obligations of the program.
///
/// A written type-argument list keys the instance directly; a call that
/// writes none derives its arguments from the checked argument types with the
/// body path's inference. A parameter the arguments leave unbound records
/// nothing: this pass never guesses a key.
pub fn collect(
    loaded: &Loaded,
    program: &ProgramInfo,
    sema: &SemaOutput,
) -> Monomorphization {
    let mut mono = Monomorphization::default();
    for module in loaded.modules.values() {
        for item in &module.items {
            match item {
                Item::Func(decl) => walk_block(&decl.body, program, sema, &mut mono),
                // A module initializer's calls instantiate like a body's: the
                // initializer runs before `main`, so its instances must exist
                // in the emitted program.
                Item::GlobalLet(decl) => walk_expr(&decl.value, program, sema, &mut mono),
                Item::GlobalConst(decl) => walk_expr(&decl.value, program, sema, &mut mono),
                _ => {}
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

    // A generic struct can hold a field that is itself an instantiation, as in
    // `struct Box<T> { p: Pair<T> }`. Each recorded instance fixes its
    // parameters, so its fields name concrete instantiations that the emitter
    // must also write; expand every instance and record what its fields name,
    // repeating until no new instance appears. A struct that embeds itself by
    // value is rejected by the checker, so this terminates.
    let decls = struct_declarations(loaded);
    let mut index = 0;
    while index < mono.struct_instances().len() {
        let instance = mono.struct_instances()[index].clone();
        index += 1;
        let Some(decl) = decls.get(instance.name.as_str()) else {
            continue;
        };
        let params = type_param_names(&decl.type_params);
        for field in &decl.fields {
            let ty = resolve_ty(&field.ty, &params, &sema.table);
            record_struct_type(&substitute_instance(&ty, &params, &instance.args), sema, mono);
        }
    }
}

/// Every struct declaration, keyed by name.
///
/// The instance collectors and the instantiation audit both need the declared
/// fields of a recorded instance; one map keeps that lookup in one place. A
/// struct name is program-global in the model the corpus uses, so the simple
/// name is the key, matching [`ProgramInfo`].
pub(super) fn struct_declarations(loaded: &Loaded) -> BTreeMap<&str, &StructDecl> {
    loaded
        .modules
        .values()
        .flat_map(|module| module.items.iter())
        .filter_map(|item| match item {
            Item::Struct(decl) => Some((decl.name.as_str(), decl)),
            _ => None,
        })
        .collect()
}

/// Collects a declaration's written type-parameter names.
pub(super) fn type_param_names(params: &[TypeParam]) -> Vec<String> {
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
                record_struct_type(&param.ty, sema, mono);
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
            signature
                .params
                .iter()
                .all(|param| is_concrete(&param.ty))
                && is_concrete(&signature.returns)
        }
        _ => true,
    }
}

/// True when a type can key a recorded instance on its own.
///
/// A type parameter or an unresolved placeholder names no concrete key, and an
/// unstamped closure type belongs to a generic template whose enclosing
/// instance the body's per-instance walk stamps later. Every other type keys
/// its parts.
fn instance_keyable(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Param(_) | TyKind::Unknown => false,
        TyKind::Closure(closure) => closure.instance.is_some(),
        TyKind::Array { element, .. } => instance_keyable(element),
        TyKind::Raw(inner) => instance_keyable(inner),
        TyKind::Struct(_, args) => args.iter().all(instance_keyable),
        TyKind::Func(signature) => {
            signature
                .params
                .iter()
                .all(|param| instance_keyable(&param.ty))
                && instance_keyable(&signature.returns)
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
            record_call(callee, type_args, args, program, sema, mono);
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

/// Records one generic call and its copy obligations.
///
/// A written type-argument list keys the instance directly. A call that
/// writes none derives its arguments from the checked argument types, exactly
/// as the body path's inference does, so an initializer's inferred call
/// declares the same instance its body twin would; a parameter the arguments
/// leave unbound records nothing, and a non-generic callee records nothing.
fn record_call(
    callee: &Expr,
    type_args: &[Type],
    args: &[Arg],
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
) {
    let Some(name) = callee_simple_name(callee) else {
        return;
    };
    let Some(Callee::User(decl)) = program.callee(&name) else {
        return;
    };
    let instance_args: Vec<Ty> = if type_args.is_empty() {
        match infer_call_type_args(decl, callee, args, program, sema) {
            Some(inferred) => inferred,
            None => return,
        }
    } else {
        if decl.type_params.len() != type_args.len() {
            return;
        }
        type_args
            .iter()
            .map(|ty| resolve_ty(ty, &[], &sema.table))
            .collect()
    };
    if instance_args.is_empty() {
        return;
    }
    let instance = Instance {
        function: decl.name.clone(),
        args: instance_args,
    };
    // Only a concrete instantiation has an emitted copy and a stable symbol: a
    // call written inside a generic body may name a type parameter, and an
    // unstamped closure argument (one this template's enclosing instance does
    // not key yet) belongs to the body's own per-instance walk.
    if instance.args.iter().any(|ty| !instance_keyable(ty)) {
        return;
    }
    let key = instance.dedupe_key();
    let bindings: BTreeMap<String, Ty> = decl
        .type_params
        .iter()
        .map(|param| param.name.clone())
        .zip(instance.args.iter().cloned())
        .collect();
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

/// The receiver a dot callee contributes to its call's argument list.
///
/// The body's call lowering puts the dot receiver at argument position zero,
/// next to the written arguments, so an inferred instance key must align with
/// that list. A member that names a declared field is the callable value
/// itself: the call passes through the field and no declared-function
/// inference applies.
enum DotReceiver {
    /// The receiver is the call's argument position zero, with this type.
    Argument(Ty),
    /// A receiver exists but this walk cannot type it: a local-headed path or
    /// a base whose checked type was not recorded. No instance is recorded
    /// here — the body's per-instance IR walk records the stamped one — so a
    /// shifted key can never be guessed.
    Unknown,
    /// The callee names a callable field.
    Field,
    /// No dot receiver: a plain name or a module-qualified path.
    None,
}

/// The receiver type a dot callee contributes to its call's argument list.
fn dot_receiver(callee: &Expr, program: &ProgramInfo, sema: &SemaOutput) -> DotReceiver {
    let (receiver, member) = match &callee.kind {
        ExprKind::Member { base, name } => {
            let Some(ty) = sema
                .expr_types
                .get(&(base.as_ref() as *const Expr as usize))
                .cloned()
            else {
                return DotReceiver::Unknown;
            };
            (ty, name.clone())
        }
        ExprKind::Qualified(name) if name.segments.len() >= 2 => {
            let (head, rest) = name.segments.split_first().expect("checked segment count");
            let Some((_, info)) = program.globals.iter().find(|(id, _)| id.name == *head) else {
                // A local-headed path in a body is typed by the caller's scope,
                // which this walk does not carry, and a module-qualified path
                // names no value receiver at all. Either way the key cannot be
                // derived here.
                return DotReceiver::Unknown;
            };
            let mut ty = info.ty.clone();
            for segment in &rest[..rest.len() - 1] {
                let Some(next) = sema.table.instance_field(&ty, segment) else {
                    return DotReceiver::Unknown;
                };
                ty = next;
            }
            (ty, rest.last().expect("checked segment count").clone())
        }
        _ => return DotReceiver::None,
    };
    if sema.table.instance_field(&receiver, &member).is_some() {
        return DotReceiver::Field;
    }
    DotReceiver::Argument(receiver)
}

/// Derives a call's type arguments from its checked argument types.
///
/// The type stage recorded every argument's type beside its node, so the walk
/// binds each declared parameter against the recorded type of its argument —
/// the same matcher the body path's inference uses. A dot call's receiver is
/// its argument position zero, so it takes part in the derivation first; a
/// callable-field callee records nothing, because the call passes through the
/// field rather than the declared function its name may share. Returns `None`
/// when a parameter stays unbound or an argument's type was never recorded.
fn infer_call_type_args(
    decl: &crate::ast::FuncDecl,
    callee: &Expr,
    args: &[Arg],
    program: &ProgramInfo,
    sema: &SemaOutput,
) -> Option<Vec<Ty>> {
    let declared: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
    let param_types: Vec<Ty> = decl
        .params
        .iter()
        .map(|param| resolve_ty(&param.ty, &declared, &sema.table))
        .collect();
    let mut arg_types: Vec<Ty> = Vec::new();
    match dot_receiver(callee, program, sema) {
        DotReceiver::Argument(ty) => arg_types.push(ty),
        DotReceiver::Unknown | DotReceiver::Field => return None,
        DotReceiver::None => {}
    }
    for arg in args {
        let ty = sema
            .expr_types
            .get(&(&arg.value as *const Expr as usize))
            .cloned()?;
        arg_types.push(ty);
    }
    crate::sema::call::infer_type_args(&declared, &param_types, &arg_types)
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
    let no_bindings = BTreeMap::new();
    for function in functions {
        // A generic declaration and a lifted lambda inside one are walked per
        // concrete instance below. Their bodies must not record an instance
        // keyed on a closure whose enclosing instance is not known yet: a
        // closure-only argument has no type parameter to mark it unresolved,
        // so walking the template would record a plain copy alongside the
        // stamped one. A lifted lambda is only as generic as the declaration
        // that encloses it, so a lambda in a non-generic function keeps its
        // first-pass walk.
        let template = match function.enclosing.as_deref() {
            Some(enclosing) => matches!(
                program.callee(enclosing),
                Some(Callee::User(decl)) if !decl.type_params.is_empty()
            ),
            None => matches!(
                program.callee(&function.name),
                Some(Callee::User(decl)) if !decl.type_params.is_empty()
            ),
        };
        if template {
            continue;
        }
        walk_function(function, &no_bindings, program, sema, mono, None);
    }
    // A generic body may call another generic with its own type parameters, so
    // the first walk cannot name those instances. Each recorded instance is
    // re-walked with its arguments substituted; new instances are appended and
    // processed in turn until the set is closed.
    let mut index = 0;
    while index < mono.instances().len() {
        let instance = mono.instances()[index].clone();
        index += 1;
        let Some(Callee::User(decl)) = program.callee(&instance.function) else {
            continue;
        };
        if decl.type_params.len() != instance.args.len() {
            continue;
        }
        let Some(function) = functions
            .iter()
            .find(|function| function.name == decl.name)
        else {
            continue;
        };
        let params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        // The bindings carry the element placeholder too, so a call argument
        // written as an element of the opaque parameter resolves to the
        // instance's element before its key derives.
        let bindings = crate::types::instance_bindings(&params, &instance.args);
        // The enclosing specializations are recorded before the body is
        // walked: the walk stamps a closure argument it records for this
        // instance, and the stamp is applied only when the site is specialized
        // for that instance.
        for lifted in functions {
            if lifted.enclosing.as_deref() != Some(decl.name.as_str()) {
                continue;
            }
            let Some(site) = lifted.lambda_site else {
                continue;
            };
            if lifted_needs_specialization(lifted, decl, &sema.closures) {
                mono.record_lifted(LiftedSpecialization {
                    site,
                    args: instance.args.clone(),
                });
            }
        }
        // The key mirrors the emitter's instance key, so a stamped closure
        // names the same callable struct the emitter builds for this instance.
        let enclosing = instance_key(&instance.args);
        walk_function(function, &bindings, program, sema, mono, Some(&enclosing));
        // A lifted lambda in this instance's body carries the same bindings.
        // Walking it records the nested generic calls its body makes (the
        // behavior this pass exists for).
        for lifted in functions {
            if lifted.enclosing.as_deref() != Some(decl.name.as_str()) {
                continue;
            }
            if lifted.lambda_site.is_none() {
                continue;
            }
            walk_function(lifted, &bindings, program, sema, mono, Some(&enclosing));
        }
    }
}

/// True when a lifted lambda must be emitted once per enclosing instance.
///
/// A lifted function whose declared types, body value types, lambda captures,
/// or generic call arguments mention a type parameter of its enclosing
/// declaration changes meaning per concrete instance, so it cannot be emitted
/// once with an unresolved parameter. Every other lifted function keeps the
/// single shared emission.
pub(crate) fn lifted_needs_specialization(
    function: &Function,
    decl: &crate::ast::FuncDecl,
    closures: &BTreeMap<usize, ClosureSite>,
) -> bool {
    if decl.type_params.is_empty() {
        return false;
    }
    if super::lower::mentions_type_param(&function.ret)
        || function
            .params
            .iter()
            .any(|param| super::lower::mentions_type_param(&param.ty))
        || function
            .locals
            .iter()
            .any(|local| super::lower::mentions_type_param(&local.ty))
        || function.temps.iter().any(super::lower::mentions_type_param)
    {
        return true;
    }
    // An environment field can carry a parameter even when the body never
    // reads the capture, so the recorded capture types are part of the check.
    if let Some(site) = function.lambda_site {
        if closures.get(&site).is_some_and(|closure| {
            closure
                .captures
                .iter()
                .any(|capture| super::lower::mentions_type_param(&capture.ty))
        }) {
            return true;
        }
    }
    function.blocks.iter().any(|block| {
        block.statements.iter().any(|statement| match &statement.statement {
            Statement::Assign { value, .. } | Statement::Discard { value } => {
                value_mentions_type_param(value)
            }
            Statement::StorePointer { pointer, value } => {
                value_mentions_type_param(pointer) || value_mentions_type_param(value)
            }
            Statement::Call {
                type_args,
                args,
                receiver,
                ..
            } => {
                type_args.iter().any(super::lower::mentions_type_param)
                    || args.iter().any(value_mentions_type_param)
                    || receiver.as_deref().is_some_and(value_mentions_type_param)
            }
            _ => false,
        }) || match &block.terminator {
            Terminator::Branch { condition, .. } => value_mentions_type_param(condition),
            Terminator::Switch { value, arms, .. } => {
                value_mentions_type_param(value)
                    || arms
                        .iter()
                        .any(|(label, _)| value_mentions_type_param(label))
            }
            Terminator::Return {
                value: Some(value), ..
            } => value_mentions_type_param(value),
            _ => false,
        }
    })
}

/// True when a value's type or any nested value mentions a type parameter.
///
/// A parameter can sit below the top of a value tree — in a call's type
/// arguments, a template placeholder, a literal element, or a nested closure's
/// captures — so the check descends every operand instead of reading only the
/// value's own result type.
fn value_mentions_type_param(value: &Value) -> bool {
    if super::lower::mentions_type_param(&value.ty) {
        return true;
    }
    match &value.kind {
        ValueKind::Call {
            type_args,
            args,
            receiver,
            ..
        } => {
            type_args.iter().any(super::lower::mentions_type_param)
                || args.iter().any(value_mentions_type_param)
                || receiver
                    .as_deref()
                    .is_some_and(value_mentions_type_param)
        }
        ValueKind::Member { base, .. }
        | ValueKind::Unary { operand: base, .. }
        | ValueKind::Cast { value: base, .. } => value_mentions_type_param(base),
        ValueKind::Index { base, index } => {
            value_mentions_type_param(base) || value_mentions_type_param(index)
        }
        ValueKind::Binary { left, right, .. } => {
            value_mentions_type_param(left) || value_mentions_type_param(right)
        }
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            value_mentions_type_param(condition)
                || value_mentions_type_param(then_value)
                || value_mentions_type_param(else_value)
        }
        ValueKind::StructLit { fields, .. } => fields
            .iter()
            .any(|(_, field)| value_mentions_type_param(field)),
        ValueKind::ArrayLit { elements, fill } => {
            elements.iter().any(value_mentions_type_param)
                || fill
                    .as_deref()
                    .is_some_and(value_mentions_type_param)
        }
        ValueKind::Template(pieces) => pieces.iter().any(|piece| match piece {
            ValuePiece::Literal(_) => false,
            ValuePiece::Placeholder(value) => value_mentions_type_param(value),
        }),
        ValueKind::Intrinsic { args, type_arg, .. } => {
            type_arg.iter().any(super::lower::mentions_type_param)
                || args.iter().any(value_mentions_type_param)
        }
        ValueKind::Closure { captures, .. } => captures.iter().any(value_mentions_type_param),
        ValueKind::ClosureCall { callable, args } => {
            value_mentions_type_param(callable) || args.iter().any(value_mentions_type_param)
        }
        ValueKind::Materialize { value, .. } => value_mentions_type_param(value),
        _ => false,
    }
}

/// Walks one function's types and values, substituting every generic call and
/// recorded type with the instance bindings.
fn walk_function(
    function: &Function,
    bindings: &BTreeMap<String, Ty>,
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
    enclosing: Option<&str>,
) {
    record_struct_type(&substitute(&function.ret, bindings), sema, mono);
    for param in &function.params {
        record_struct_type(&substitute(&param.ty, bindings), sema, mono);
    }
    for local in &function.locals {
        record_struct_type(&substitute(&local.ty, bindings), sema, mono);
    }
    for temp in &function.temps {
        record_struct_type(&substitute(temp, bindings), sema, mono);
    }
    // The declared local types resolve a projected place read under the
    // instance substitution, exactly as the emitter's place renderer walks
    // them.
    let locals: BTreeMap<String, Ty> = function
        .locals
        .iter()
        .map(|local| (local.name.clone(), local.ty.clone()))
        .collect();
    for block in &function.blocks {
        for instr in &block.statements {
            match &instr.statement {
                Statement::Assign { value, .. } => {
                    walk_value(value, bindings, &locals, program, sema, mono, enclosing)
                }
                Statement::StorePointer { pointer, value } => {
                    walk_value(pointer, bindings, &locals, program, sema, mono, enclosing);
                    walk_value(value, bindings, &locals, program, sema, mono, enclosing);
                }
                Statement::Call {
                    callee,
                    type_args,
                    args,
                    receiver,
                    ..
                } => {
                    record_call_instance(
                        callee, type_args, args, bindings, &locals, program, sema, mono, enclosing,
                    );
                    if let Some(receiver) = receiver {
                        walk_value(receiver, bindings, &locals, program, sema, mono, enclosing);
                    }
                    for arg in args {
                        walk_value(arg, bindings, &locals, program, sema, mono, enclosing);
                    }
                }
                Statement::Discard { value } => {
                    walk_value(value, bindings, &locals, program, sema, mono, enclosing)
                }
                Statement::Guards { checks } => {
                    for check in checks {
                        walk_value(check, bindings, &locals, program, sema, mono, enclosing);
                    }
                }
                Statement::Move { .. }
                | Statement::MoveOut { .. }
                | Statement::Borrow { .. }
                | Statement::Drop { .. }
                | Statement::ReleaseTemp(_)
                | Statement::Defer { .. }
                | Statement::ScopeEnd { .. }
                | Statement::ScopeExit { .. }
                | Statement::Nop => {}
            }
        }
        match &block.terminator {
            Terminator::Branch { condition, .. } => {
                walk_value(condition, bindings, &locals, program, sema, mono, enclosing)
            }
            Terminator::Switch { value, arms, .. } => {
                walk_value(value, bindings, &locals, program, sema, mono, enclosing);
                for (label, _) in arms {
                    walk_value(label, bindings, &locals, program, sema, mono, enclosing);
                }
            }
            Terminator::Return {
                value: Some(value), ..
            } => walk_value(value, bindings, &locals, program, sema, mono, enclosing),
            _ => {}
        }
    }
}

/// The joined symbol key of an instance's type arguments.
///
/// It mirrors the emitter's instance key, so a closure stamped with it names
/// the same callable struct the emitter builds for the enclosing instance.
fn instance_key(args: &[Ty]) -> String {
    let keys: Vec<String> = args.iter().map(crate::types::type_key).collect();
    keys.join("_")
}

/// True when the lifted lambda at `site` is emitted for the instance key.
fn has_lifted_specialization(mono: &Monomorphization, site: usize, key: &str) -> bool {
    mono.lifted_specializations().iter().any(|specialization| {
        specialization.site == site && instance_key(&specialization.args) == key
    })
}

/// Stamps an enclosing instance key on closure types recorded for that instance.
///
/// A closure created in a generic body belongs to the enclosing instance, so a
/// call argument that names the closure must spell the same callable struct the
/// emitter builds there. Only a site with a lifted specialization for this
/// instance is stamped; a shared or non-generic closure keeps its plain key.
/// The walk mirrors the emitter's tagging, so a closure nested inside another
/// recorded type is stamped too.
fn stamp_closure_instances(ty: &mut Ty, key: &str, mono: &Monomorphization) {
    match &mut ty.kind {
        TyKind::Closure(closure) => {
            if closure.instance.is_none() && has_lifted_specialization(mono, closure.id, key) {
                closure.instance = Some(key.to_owned());
            }
        }
        TyKind::Raw(inner) => stamp_closure_instances(inner, key, mono),
        TyKind::Array { element, .. } => stamp_closure_instances(element, key, mono),
        TyKind::Struct(_, args) => {
            for arg in args {
                stamp_closure_instances(arg, key, mono);
            }
        }
        TyKind::Func(signature) => {
            for param in &mut signature.params {
                stamp_closure_instances(&mut param.ty, key, mono);
            }
            stamp_closure_instances(&mut signature.returns, key, mono);
        }
        _ => {}
    }
}

/// The concrete type arguments of one call, stamped for the enclosing instance.
///
/// The arguments are substituted first, so a call in a generic body carries
/// concrete types, and then every closure argument with a lifted
/// specialization for `enclosing` names its specialized callable. Both the
/// value and the discarded-statement call paths use this, so they cannot
/// record different instances for the same call.
fn stamped_type_args(
    type_args: &[Ty],
    bindings: &BTreeMap<String, Ty>,
    enclosing: Option<&str>,
    mono: &Monomorphization,
) -> Vec<Ty> {
    let mut resolved: Vec<Ty> = type_args
        .iter()
        .map(|ty| substitute(ty, bindings))
        .collect();
    if let Some(key) = enclosing {
        for ty in &mut resolved {
            stamp_closure_instances(ty, key, mono);
        }
    }
    resolved
}

/// Records one call's instance with its type arguments resolved.
///
/// A recorded argument can still be opaque in a generic body (a member read on
/// the element placeholder): the bound argument values re-derive the key with
/// the same matcher the call's own inference uses, so the recorded instance
/// matches the symbol the emitter spells.
#[allow(clippy::too_many_arguments)]
fn record_call_instance(
    callee: &str,
    type_args: &[Ty],
    args: &[Value],
    bindings: &BTreeMap<String, Ty>,
    locals: &BTreeMap<String, Ty>,
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
    enclosing: Option<&str>,
) {
    let mut resolved = stamped_type_args(type_args, bindings, enclosing, mono);
    if resolved.iter().any(|ty| ty.is_param() || ty.is_unknown()) {
        if let Some(inferred) = resolve_ir_call_args(callee, args, bindings, locals, program, sema) {
            resolved = stamped_type_args(&inferred, bindings, enclosing, mono);
        }
    }
    record_instance(callee, &resolved, program, mono);
}

/// Re-derives a call's type arguments from its resolved argument types.
///
/// A recorded argument can still be opaque in a generic body: a member read
/// on the element placeholder has no type until the instance is known. The
/// bound argument values resolve it through the same field lookup the
/// emitter's value-type resolver walks, and the matcher derives the arguments
/// the way the call's own inference does — so the recorded instance matches
/// the symbol the emitter spells. `None` when a parameter stays unbound or the
/// callee is not a user generic.
fn resolve_ir_call_args(
    callee: &str,
    args: &[Value],
    bindings: &BTreeMap<String, Ty>,
    locals: &BTreeMap<String, Ty>,
    program: &ProgramInfo,
    sema: &SemaOutput,
) -> Option<Vec<Ty>> {
    let Some(Callee::User(decl)) = program.callee(callee) else {
        return None;
    };
    if decl.type_params.is_empty() {
        return None;
    }
    let declared: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
    let param_types: Vec<Ty> = decl
        .params
        .iter()
        .map(|param| resolve_ty(&param.ty, &declared, &sema.table))
        .collect();
    let arg_types: Vec<Ty> = args
        .iter()
        .map(|arg| resolved_ir_value_ty(arg, bindings, locals, sema))
        .collect();
    crate::sema::call::infer_type_args(&declared, &param_types, &arg_types)
}

/// The type of an IR value under an instance substitution.
///
/// A projection recorded in a generic body carries the opaque type; the
/// substituted base resolves it through the same field lookup the emitter's
/// value-type resolver walks.
fn resolved_ir_value_ty(
    value: &Value,
    bindings: &BTreeMap<String, Ty>,
    locals: &BTreeMap<String, Ty>,
    sema: &SemaOutput,
) -> Ty {
    let ty = substitute(&value.ty, bindings);
    if !ty.is_unknown() {
        return ty;
    }
    match &value.kind {
        ValueKind::Member { base, name } => {
            let base_ty = resolved_ir_value_ty(base, bindings, locals, sema);
            sema.table.instance_field(&base_ty, name).unwrap_or(ty)
        }
        ValueKind::Index { base, .. } => {
            let base_ty = resolved_ir_value_ty(base, bindings, locals, sema);
            let element = base_ty.iterated_element();
            if element.is_unknown() {
                ty
            } else {
                element
            }
        }
        // A projected place reads through its base's declared type: the local
        // table carries the instance's substituted binding, and each named
        // field resolves through the same lookup the emitter's place renderer
        // walks.
        ValueKind::Place(place) => {
            let crate::ir::PlaceBase::Local(name) = &place.base else {
                return ty;
            };
            let Some(local_ty) = locals.get(name) else {
                return ty;
            };
            let mut current = substitute(local_ty, bindings);
            for projection in &place.projections {
                match projection {
                    Projection::Field(field) => {
                        let Some(next) = sema.table.instance_field(&current, field) else {
                            return ty;
                        };
                        current = next;
                    }
                    Projection::Element(_) => current = current.iterated_element(),
                    Projection::ElementTemp(_) => return ty,
                }
            }
            if current.is_unknown() {
                ty
            } else {
                current
            }
        }
        _ => ty,
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
        args: type_args.to_vec(),
    });
}

/// Walks a value, recording every generic call it contains.
fn walk_value(
    value: &Value,
    bindings: &BTreeMap<String, Ty>,
    locals: &BTreeMap<String, Ty>,
    program: &ProgramInfo,
    sema: &SemaOutput,
    mono: &mut Monomorphization,
    enclosing: Option<&str>,
) {
    record_struct_type(&substitute(&value.ty, bindings), sema, mono);
    // A written type argument names concrete types of its own: `sizeof(T[])`
    // at a `Wrap<i32>` instance names the struct inside the view, so the
    // instance is recorded even when no local or parameter spells it.
    if let ValueKind::Intrinsic {
        type_arg: Some(ty), ..
    } = &value.kind
    {
        record_struct_type(&substitute(ty, bindings), sema, mono);
    }
    match &value.kind {
        ValueKind::Call {
            callee,
            type_args,
            args,
            receiver,
        } => {
            record_call_instance(
                callee, type_args, args, bindings, locals, program, sema, mono, enclosing,
            );
            if let Some(receiver) = receiver {
                walk_value(receiver, bindings, locals, program, sema, mono, enclosing);
            }
            for arg in args {
                walk_value(arg, bindings, locals, program, sema, mono, enclosing);
            }
        }
        ValueKind::Member { base, .. } => walk_value(base, bindings, locals, program, sema, mono, enclosing),
        ValueKind::Index { base, index } => {
            walk_value(base, bindings, locals, program, sema, mono, enclosing);
            walk_value(index, bindings, locals, program, sema, mono, enclosing);
        }
        ValueKind::Unary { operand, .. } => walk_value(operand, bindings, locals, program, sema, mono, enclosing),
        ValueKind::Binary { left, right, .. } => {
            walk_value(left, bindings, locals, program, sema, mono, enclosing);
            walk_value(right, bindings, locals, program, sema, mono, enclosing);
        }
        ValueKind::Cast { value, .. } => walk_value(value, bindings, locals, program, sema, mono, enclosing),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            walk_value(condition, bindings, locals, program, sema, mono, enclosing);
            walk_value(then_value, bindings, locals, program, sema, mono, enclosing);
            walk_value(else_value, bindings, locals, program, sema, mono, enclosing);
        }
        ValueKind::StructLit { fields, .. } => {
            for (_, field) in fields {
                walk_value(field, bindings, locals, program, sema, mono, enclosing);
            }
        }
        ValueKind::ArrayLit { elements, fill } => {
            for element in elements {
                walk_value(element, bindings, locals, program, sema, mono, enclosing);
            }
            if let Some(fill) = fill {
                walk_value(fill, bindings, locals, program, sema, mono, enclosing);
            }
        }
        ValueKind::Template(pieces) => {
            for piece in pieces {
                if let crate::ir::ValuePiece::Placeholder(inner) = piece {
                    walk_value(inner, bindings, locals, program, sema, mono, enclosing);
                }
            }
        }
        ValueKind::Intrinsic { args, .. } => {
            for arg in args {
                walk_value(arg, bindings, locals, program, sema, mono, enclosing);
            }
        }
        ValueKind::Materialize { value, .. } => walk_value(value, bindings, locals, program, sema, mono, enclosing),
        ValueKind::Closure { captures, .. } => {
            for capture in captures {
                walk_value(capture, bindings, locals, program, sema, mono, enclosing);
            }
        }
        ValueKind::ClosureCall { callable, args } => {
            walk_value(callable, bindings, locals, program, sema, mono, enclosing);
            for arg in args {
                walk_value(arg, bindings, locals, program, sema, mono, enclosing);
            }
        }
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



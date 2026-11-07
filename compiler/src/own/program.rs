//! The program facts the ownership checker needs, gathered in one pass.
//!
//! The ownership stage sees the whole loaded program, so it collects the
//! declarations that decide ownership: the functions (user and `ffi`), the
//! handles and their deleters, the structs, the module-level bindings, and the
//! behavior of the standard-library helpers that return views. Gathering them
//! once keeps the per-function walk free of repeated lookups.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Arg, Block, Expr, ExprKind, FfiBlock, ForInit, FuncDecl, FuncSignature, IntrinsicArg,
    Item, LambdaBody, Mode, Param, QualifiedName, Stmt, StructDecl, SwitchLabel,
    TemplatePiece, UnaryOp,
};
use crate::borrow::{is_nullable_capable, LoanKind};
use crate::diag::{Diagnostic, Diagnostics};
use crate::ir::GlobalId;
use crate::load::{Loaded, Module};
use crate::sema::SemaOutput;
use crate::span::Span;
use crate::types::{resolve_ty_folded, Ty, TyKind};

/// A user-declared function with a body.
pub type UserFunc<'a> = &'a FuncDecl;

/// A handle declaration and the deleter it names.
pub struct HandleInfo<'a> {
    /// The handle's name.
    pub name: String,
    /// The named deleter.
    pub deleter: String,
    /// Span of the handle's name, used for the deleter diagnostic.
    pub span: Span,
    /// The declaration, for type checking of the deleter.
    pub decl: &'a crate::ast::HandleDecl,
}

/// A module-level binding.
pub struct GlobalInfo<'a> {
    /// The binding's name.
    pub name: String,
    /// Its declared type.
    pub ty: Ty,
    /// True for a `const`, which may not be assigned.
    pub is_const: bool,
    /// The initializer of a `const`, for the typed read fold.
    pub const_init: Option<&'a Expr>,
    /// Span of the name.
    pub span: Span,
}

/// What a standard-library helper does with ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdlibEffect {
    /// Returns a freshly owned value; the caller owns it.
    OwnsResult,
    /// Returns a view over the storage of the argument at this index.
    ViewOf {
        /// Argument index whose storage the result views.
        arg: usize,
        /// The permission the result carries.
        kind: LoanKind,
    },
    /// Produces an ordinary value or `void` and owns nothing.
    Value,
}

/// Every declaration the ownership stage needs.
///
/// User functions keep a simple-name map for the existing scope resolution.
/// `ffi` signatures carry their declaring module as well, so a module-qualified
/// call reaches that module's declaration instead of a same-named one
/// elsewhere.
pub struct ProgramInfo<'a> {
    /// The loaded program, for resolving module-qualified call identities.
    pub loaded: &'a Loaded,
    /// User functions with bodies.
    pub functions: BTreeMap<String, UserFunc<'a>>,
    /// `ffi`-declared signatures, keyed by their declaring module and name.
    pub ffi_functions: BTreeMap<(String, String), &'a FuncSignature>,
    /// `ffi` blocks, for the export check.
    pub ffi_blocks: Vec<&'a FfiBlock>,
    /// Handle declarations and their deleters.
    pub handles: BTreeMap<String, HandleInfo<'a>>,
    /// Struct declarations, for the `freedBy` check.
    pub structs: BTreeMap<String, &'a StructDecl>,
    /// Module-level bindings, keyed by their module and simple name.
    pub globals: BTreeMap<GlobalId, GlobalInfo<'a>>,
    /// Module consts whose address is taken anywhere in the program.
    ///
    /// A recorded const is never folded: an accepted `unsafe &N` can mutate
    /// the storage between two reads, so a literal would change the program.
    pub address_taken_consts: BTreeSet<GlobalId>,
}

impl<'a> ProgramInfo<'a> {
    /// Collects every ownership-relevant declaration of the program.
    pub fn collect(loaded: &'a Loaded, table: &crate::types::TypeTable) -> Self {
        let mut info = ProgramInfo {
            loaded,
            functions: BTreeMap::new(),
            ffi_functions: BTreeMap::new(),
            ffi_blocks: Vec::new(),
            handles: BTreeMap::new(),
            structs: BTreeMap::new(),
            globals: BTreeMap::new(),
            address_taken_consts: BTreeSet::new(),
        };
        for module in loaded.modules.values() {
            for item in &module.items {
                match item {
                    Item::Func(decl) => {
                        info.functions.entry(decl.name.clone()).or_insert(decl);
                    }
                    Item::Ffi(block) => {
                        info.ffi_blocks.push(block);
                        for signature in &block.decls {
                            info.ffi_functions
                                .entry((module.name.clone(), signature.name.clone()))
                                .or_insert(signature);
                        }
                    }
                    Item::Handle(decl) => {
                        info.handles.insert(
                            decl.name.clone(),
                            HandleInfo {
                                name: decl.name.clone(),
                                deleter: decl.freed_by.segments.join("."),
                                span: decl.name_span,
                                decl,
                            },
                        );
                    }
                    Item::Struct(decl) => {
                        info.structs.entry(decl.name.clone()).or_insert(decl);
                    }
                    Item::GlobalLet(decl) => {
                        info.globals.insert(
                            GlobalId::new(&module.name, &decl.name),
                            GlobalInfo {
                                name: decl.name.clone(),
                                ty: table
                                    .global(&module.name, &decl.name)
                                    .cloned()
                                    .unwrap_or_else(Ty::unknown),
                                is_const: false,
                                const_init: None,
                                span: decl.name_span,
                            },
                        );
                    }
                    Item::GlobalConst(decl) => {
                        info.globals.insert(
                            GlobalId::new(&module.name, &decl.name),
                            GlobalInfo {
                                name: decl.name.clone(),
                                ty: table
                                    .global(&module.name, &decl.name)
                                    .cloned()
                                    .unwrap_or_else(Ty::unknown),
                                is_const: true,
                                const_init: Some(&decl.value),
                                span: decl.name_span,
                            },
                        );
                    }
                    Item::Comptime(_) | Item::Enum(_) | Item::Intrinsic(_) => {}
                }
            }
        }
        info.address_taken_consts = collect_address_taken(loaded, &info);
        info
    }

    /// Resolves the callee of a call by simple name: a user function, an
    /// `ffi` function, or a known standard-library helper.
    pub fn callee(&self, name: &str) -> Option<Callee<'a>> {
        if let Some(function) = self.functions.get(name) {
            return Some(Callee::User(function));
        }
        if let Some(signature) = self.ffi_function(name) {
            return Some(Callee::Ffi(signature));
        }
        stdlib_effect(name).map(|effect| Callee::Stdlib {
            name: name.to_owned(),
            effect,
        })
    }

    /// The user function a call name names, resolving a module-qualified path.
    ///
    /// A module-qualified name names the declaration in its own module. A
    /// simple name keeps the existing scope resolution, so an imported or
    /// same-module function still wins over one elsewhere.
    pub fn user_function(&self, name: &str) -> Option<UserFunc<'a>> {
        if let Some((module, simple)) = name.rsplit_once('.') {
            let target = self.loaded.module(module)?;
            return target.items.iter().find_map(|item| match item {
                Item::Func(decl) if decl.name == simple => Some(decl),
                _ => None,
            });
        }
        match self.callee(name) {
            Some(Callee::User(decl)) => Some(decl),
            _ => None,
        }
    }

    /// The `ffi` signature a call name names.
    ///
    /// A module-qualified name resolves in that module only. A simple name
    /// keeps the program-wide lookup the existing callers use: the first
    /// declaration in module order, which is the same order the collected map
    /// was built in.
    pub(crate) fn ffi_function(&self, name: &str) -> Option<&'a FuncSignature> {
        if let Some((module, simple)) = name.rsplit_once('.') {
            return self
                .ffi_functions
                .get(&(module.to_owned(), simple.to_owned()))
                .copied();
        }
        self.ffi_functions
            .iter()
            .find(|((_, simple), _)| simple == name)
            .map(|(_, signature)| *signature)
    }
}

/// The declaration a call name resolves to.
pub enum Callee<'a> {
    /// A user function with a body.
    User(UserFunc<'a>),
    /// An `ffi`-declared signature.
    Ffi(&'a FuncSignature),
    /// A standard-library helper with known ownership behavior.
    Stdlib {
        /// The helper's name.
        name: String,
        /// What the helper does with ownership.
        effect: StdlibEffect,
    },
}

/// The ownership behavior of the modeled standard-library helpers.
///
/// A helper that is not listed is treated as an opaque call that produces no
/// view and owns nothing, so it never triggers a false ownership diagnostic.
pub fn stdlib_effect(name: &str) -> Option<StdlibEffect> {
    match name {
        // Allocating constructors of modules that still have no embedded
        // source return an owned value. A module with a source resolves to its
        // compiled function instead.
        "stringOf" | "clone" | "text" | "toHex" | "toDecimal" | "padLeft" | "padRight"
        | "openFile" | "createFile" | "addHandler" => Some(StdlibEffect::OwnsResult),
        // Helpers that return a view or `cstring` over storage the caller
        // still owns, so the call is a loan of their first argument. The
        // unchecked view builders derive their storage from the same place as
        // their first argument.
        "bytes" | "toCString" | "slice" | "rawSlice" | "rawStr" => {
            Some(StdlibEffect::ViewOf {
                arg: 0,
                kind: LoanKind::Shared,
            })
        }
        // `bufferBytes` derives a writable view. Its body is an embedded
        // source now, so a real compile resolves the function itself; this
        // entry keeps the ownership model for a program checked without the
        // standard-library module loaded.
        "bufferBytes" => Some(StdlibEffect::ViewOf {
            arg: 0,
            kind: LoanKind::Exclusive,
        }),
        _ => None,
    }
}

/// Resolves a written type name to an AST spelling for the handle/deleter
/// comparison, which only needs the declared name.
pub fn type_name_matches(ty: &crate::ast::Type, name: &str) -> bool {
    match &ty.kind {
        crate::ast::TypeKind::Named { name: written, .. } => {
            written.segments.last().is_some_and(|last| last == name)
        }
        _ => false,
    }
}

/// Rejects a module-level `let` whose declared type needs a drop.
///
/// There are no drops at process exit, so a global must be duplicable; a
/// `const` is exempt because it cannot own a released resource.
pub(super) fn check_globals<'a>(
    module: &'a Module,
    loaded: &'a Loaded,
    program: &ProgramInfo<'a>,
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) {
    for item in &module.items {
        match item {
            Item::GlobalLet(decl) => {
                let ty = resolve_ty_folded(&decl.ty, &[], &sema.table, module, loaded);
                report_illegal_nullable(&ty, decl.name_span, diagnostics);
                if !sema.table.duplicable(&ty) && !ty.is_unknown() {
                    diagnostics.push(
                        Diagnostic::error(
                            "E11106",
                            "module-level 'let' must be duplicable or 'const'",
                            decl.name_span,
                        )
                        .with_help("use `const`, or initialize inside an explicit `init()`"),
                    );
                }
                check_initializer_calls(
                    &decl.value,
                    module,
                    loaded,
                    program,
                    sema,
                    diagnostics,
                );
            }
            Item::GlobalConst(decl) => {
                let ty = resolve_ty_folded(&decl.ty, &[], &sema.table, module, loaded);
                report_illegal_nullable(&ty, decl.name_span, diagnostics);
            }
            _ => {}
        }
    }
}

/// Applies the global-borrow call rule to module initializer expressions.
///
/// The own function checker is body-scoped, so an initializer call never
/// reaches it. This walk resolves the callee the way the call checker does and
/// judges each written argument and a dot receiver against the declared
/// parameter mode: a non-`const` module global passed to a `borrow`, `const`,
/// or unmarked parameter reports `E10038`. A `const` global, a non-place
/// argument, a `take` parameter, and a local or call-result source stay
/// accepted.
fn check_initializer_calls<'a>(
    expr: &Expr,
    module: &'a Module,
    loaded: &'a Loaded,
    program: &ProgramInfo<'a>,
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) {
    match &expr.kind {
        ExprKind::Call { callee, args, .. } => {
            check_initializer_call(callee, args, module, loaded, program, sema, diagnostics);
            check_initializer_calls(callee, module, loaded, program, sema, diagnostics);
            for arg in args {
                check_initializer_calls(&arg.value, module, loaded, program, sema, diagnostics);
            }
        }
        ExprKind::Index { base, index } => {
            check_initializer_calls(base, module, loaded, program, sema, diagnostics);
            check_initializer_calls(index, module, loaded, program, sema, diagnostics);
        }
        ExprKind::Member { base, .. } => {
            check_initializer_calls(base, module, loaded, program, sema, diagnostics);
        }
        ExprKind::Unary { operand, .. } => {
            check_initializer_calls(operand, module, loaded, program, sema, diagnostics);
        }
        ExprKind::Binary { left, right, .. } => {
            check_initializer_calls(left, module, loaded, program, sema, diagnostics);
            check_initializer_calls(right, module, loaded, program, sema, diagnostics);
        }
        ExprKind::Assign { target, value, .. } => {
            check_initializer_calls(target, module, loaded, program, sema, diagnostics);
            check_initializer_calls(value, module, loaded, program, sema, diagnostics);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            check_initializer_calls(condition, module, loaded, program, sema, diagnostics);
            check_initializer_calls(then_value, module, loaded, program, sema, diagnostics);
            check_initializer_calls(else_value, module, loaded, program, sema, diagnostics);
        }
        ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
            check_initializer_calls(value, module, loaded, program, sema, diagnostics);
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                check_initializer_calls(&field.value, module, loaded, program, sema, diagnostics);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                check_initializer_calls(element, module, loaded, program, sema, diagnostics);
            }
            if let Some(fill) = fill {
                check_initializer_calls(fill, module, loaded, program, sema, diagnostics);
            }
        }
        ExprKind::IntrinsicCall { name, args, .. } => {
            // An import alias names its original intrinsic. Resolve it before
            // the view-builder lookup, so `import { slice as sl }` applies the
            // same source rule as the direct spelling.
            let resolved = if name.contains('.') {
                name.as_str()
            } else {
                module
                    .import(name)
                    .filter(|binding| !binding.poisoned)
                    .map(|binding| binding.original.as_str())
                    .unwrap_or(name)
            };
            let effect_name = resolved.rsplit_once('.').map(|(_, simple)| simple).unwrap_or(resolved);
            // A view builder borrows its source. The body checker reports a
            // non-const global there; the initializer walk must apply the same
            // rule instead of leaving the intrinsic source unchecked.
            if let Some(StdlibEffect::ViewOf { arg, .. }) = stdlib_effect(effect_name) {
                if let Some(IntrinsicArg::Expr(source)) = args.get(arg) {
                    check_initializer_global_source(
                        source,
                        source.span,
                        module,
                        loaded,
                        program,
                        diagnostics,
                    );
                }
            }
            for arg in args {
                if let IntrinsicArg::Expr(inner) = arg {
                    check_initializer_calls(inner, module, loaded, program, sema, diagnostics);
                }
            }
        }
        ExprKind::Unsafe(inner) => {
            check_initializer_calls(inner, module, loaded, program, sema, diagnostics);
        }
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let TemplatePiece::Placeholder(inner) = piece {
                    check_initializer_calls(inner, module, loaded, program, sema, diagnostics);
                }
            }
        }
        // A lambda body is lifted into its own function, which the ordinary
        // function walk checks; walking it here would judge the same call
        // twice.
        ExprKind::Lambda { .. } => {}
        _ => {}
    }
}

/// One callee shape in an initializer call.
enum InitializerReceiver<'a> {
    /// A direct or module-qualified free call.
    None,
    /// A dotted callee split into its receiver segments and callee name.
    Path {
        /// The receiver's written segments.
        segments: Vec<String>,
        /// Span of the whole callee.
        span: Span,
    },
    /// A member callee over an arbitrary base expression.
    Expr(&'a Expr),
}

/// A resolved initializer callee spelling.
struct InitializerCall<'a> {
    /// Full spelling for a module path, simple name otherwise.
    written: String,
    /// Last segment.
    simple: String,
    /// The declaring module for a module-qualified free call.
    module_path: Option<String>,
    /// The receiver shape.
    receiver: InitializerReceiver<'a>,
}

/// Classifies an initializer callee the way the call checker does.
fn initializer_call<'a>(callee: &'a Expr, loaded: &Loaded) -> Option<InitializerCall<'a>> {
    match &callee.kind {
        ExprKind::Name(name) => Some(InitializerCall {
            written: name.clone(),
            simple: name.clone(),
            module_path: None,
            receiver: InitializerReceiver::None,
        }),
        ExprKind::Qualified(name) if name.segments.len() >= 2 => {
            let simple = name.segments.last()?.clone();
            if let Some(split) = longest_module_prefix(loaded, &name.segments) {
                if split + 1 == name.segments.len() {
                    return Some(InitializerCall {
                        written: name.segments.join("."),
                        simple,
                        module_path: Some(name.segments[..split].join(".")),
                        receiver: InitializerReceiver::None,
                    });
                }
            }
            Some(InitializerCall {
                written: simple.clone(),
                simple,
                module_path: None,
                receiver: InitializerReceiver::Path {
                    segments: name.segments[..name.segments.len() - 1].to_vec(),
                    span: name.span,
                },
            })
        }
        ExprKind::Member { base, name } => Some(InitializerCall {
            written: name.clone(),
            simple: name.clone(),
            module_path: None,
            receiver: InitializerReceiver::Expr(base),
        }),
        _ => None,
    }
}

/// One initializer callee's declaration.
enum InitializerDecl<'a> {
    /// A user function with a body.
    User(&'a FuncDecl),
    /// An `ffi` signature.
    Ffi(&'a FuncSignature),
}

impl<'a> InitializerDecl<'a> {
    /// The declared parameters.
    fn params(&self) -> &'a [Param] {
        match self {
            Self::User(decl) => &decl.params,
            Self::Ffi(signature) => &signature.params,
        }
    }
}

/// Resolves an initializer callee by the call checker's scope order.
///
/// A module-qualified name resolves in that module only. A simple name keeps
/// the caller's own module first, then an import binding under its original
/// name, and only then the program-wide map. This is the same order the
/// function-body call checker uses; the simple-name map alone would pick the
/// first module in load order.
fn initializer_decl<'a>(
    name: &str,
    module: &'a Module,
    loaded: &'a Loaded,
    program: &ProgramInfo<'a>,
) -> Option<InitializerDecl<'a>> {
    if let Some((target, simple)) = name.rsplit_once('.') {
        let target = loaded.module(target)?;
        if let Some(decl) = declared_function(target, simple) {
            return Some(InitializerDecl::User(decl));
        }
        if let Some(signature) = declared_ffi(target, simple) {
            return Some(InitializerDecl::Ffi(signature));
        }
        return None;
    }
    if let Some(decl) = declared_function(module, name) {
        return Some(InitializerDecl::User(decl));
    }
    if let Some(signature) = declared_ffi(module, name) {
        return Some(InitializerDecl::Ffi(signature));
    }
    if let Some(binding) = module.import(name) {
        if !binding.poisoned {
            if let Some(target) = loaded.module(&binding.module) {
                if let Some(decl) = declared_function(target, &binding.original) {
                    return Some(InitializerDecl::User(decl));
                }
                if let Some(signature) = declared_ffi(target, &binding.original) {
                    return Some(InitializerDecl::Ffi(signature));
                }
            }
        }
    }
    match program.callee(name) {
        Some(Callee::User(decl)) => Some(InitializerDecl::User(decl)),
        Some(Callee::Ffi(signature)) => Some(InitializerDecl::Ffi(signature)),
        _ => None,
    }
}

/// Finds a function declaration in one module.
fn declared_function<'a>(module: &'a Module, name: &str) -> Option<&'a FuncDecl> {
    module.items.iter().find_map(|item| match item {
        Item::Func(decl) if decl.name == name => Some(decl),
        _ => None,
    })
}

/// Finds an `ffi` signature in one module.
fn declared_ffi<'a>(module: &'a Module, name: &str) -> Option<&'a FuncSignature> {
    module.items.iter().find_map(|item| match item {
        Item::Ffi(block) => block.decls.iter().find(|signature| signature.name == name),
        _ => None,
    })
}

/// Resolves the declared parameters of an initializer callee.
fn initializer_params<'a>(
    call: &InitializerCall<'_>,
    module: &'a Module,
    loaded: &'a Loaded,
    program: &ProgramInfo<'a>,
) -> Option<&'a [Param]> {
    let name = if call.module_path.is_some() {
        call.written.as_str()
    } else {
        call.simple.as_str()
    };
    let decl = initializer_decl(name, module, loaded, program)?;
    Some(decl.params())
}

/// Checks one initializer call's written arguments and dot receiver.
fn check_initializer_call<'a>(
    callee: &Expr,
    args: &[Arg],
    module: &'a Module,
    loaded: &'a Loaded,
    program: &ProgramInfo<'a>,
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) {
    let Some(call) = initializer_call(callee, loaded) else {
        return;
    };
    // A func-typed global is the callee value, not a free function. Judge its
    // written arguments against the type the global was declared with.
    if matches!(call.receiver, InitializerReceiver::None) {
        if let Some((global, _)) = initializer_global(callee, module, loaded) {
            let id = GlobalId::new(global.module, global.name);
            if let Some(info) = program.globals.get(&id) {
                if let crate::types::TyKind::Func(signature) = &info.ty.kind {
                    check_initializer_func_args(
                        signature,
                        args,
                        module,
                        loaded,
                        program,
                        diagnostics,
                    );
                    return;
                }
            }
        }
    }
    // A standard-library view builder borrows its source. The body checker
    // applies the global rule there; the initializer walk must match.
    let stdlib_name = if call.module_path.is_none() {
        module
            .import(&call.simple)
            .filter(|binding| !binding.poisoned)
            .map(|binding| binding.original.as_str())
            .unwrap_or(&call.simple)
    } else {
        call.simple.as_str()
    };
    if let Some(Callee::Stdlib {
        effect: StdlibEffect::ViewOf { arg, .. },
        ..
    }) = program.callee(stdlib_name)
    {
        if let Some(written) = args.get(arg) {
            check_initializer_global_source(
                &written.value,
                written.span,
                module,
                loaded,
                program,
                diagnostics,
            );
        }
    }
    let receiver_ty =
        initializer_receiver_type(&call.receiver, module, loaded, program, sema);
    // A func-typed field call resolves the field, not a free function of the
    // same name. Judge its written arguments against the field's signature.
    if let Some(ty) = &receiver_ty {
        if receiver_has_field(ty, &call.simple, sema) {
            if let Some(field_ty) = sema.table.instance_field(ty, &call.simple) {
                if let crate::types::TyKind::Func(signature) = &field_ty.kind {
                    check_initializer_func_args(
                        signature,
                        args,
                        module,
                        loaded,
                        program,
                        diagnostics,
                    );
                }
            }
            return;
        }
    }
    let Some(params) = initializer_params(&call, module, loaded, program) else {
        return;
    };
    match &call.receiver {
        InitializerReceiver::None => {
            for (index, arg) in args.iter().enumerate() {
                let Some(param) = params.get(index) else {
                    break;
                };
                check_initializer_take_element(
                    &arg.value,
                    arg.span,
                    param.mode,
                    sema,
                    diagnostics,
                );
                check_initializer_global_borrow(
                    &arg.value,
                    arg.span,
                    param,
                    module,
                    loaded,
                    program,
                    diagnostics,
                );
            }
        }
        InitializerReceiver::Path { segments, span } => {
            if let Some(param) = params.first() {
                if let Some((global, global_span)) =
                    initializer_global_path(segments, *span, module, loaded)
                {
                    check_initializer_receiver(global, global_span, param, program, diagnostics);
                }
            }
            for (index, arg) in args.iter().enumerate() {
                let Some(param) = params.get(index + 1) else {
                    break;
                };
                check_initializer_take_element(
                    &arg.value,
                    arg.span,
                    param.mode,
                    sema,
                    diagnostics,
                );
                check_initializer_global_borrow(
                    &arg.value,
                    arg.span,
                    param,
                    module,
                    loaded,
                    program,
                    diagnostics,
                );
            }
        }
        InitializerReceiver::Expr(base) => {
            if let Some(param) = params.first() {
                if let Some((global, span)) = initializer_global(base, module, loaded) {
                    check_initializer_receiver(global, span, param, program, diagnostics);
                }
            }
            for (index, arg) in args.iter().enumerate() {
                let Some(param) = params.get(index + 1) else {
                    break;
                };
                check_initializer_take_element(
                    &arg.value,
                    arg.span,
                    param.mode,
                    sema,
                    diagnostics,
                );
                check_initializer_global_borrow(
                    &arg.value,
                    arg.span,
                    param,
                    module,
                    loaded,
                    program,
                    diagnostics,
                );
            }
        }
    }
}

/// Checks a dot receiver's global borrow.
///
/// The call has already been type-checked, so a receiver that reaches this
/// point names the free function, possibly through an implicit conversion. The
/// body checker applies the global rule after that check, and so does this
/// walk. A func-typed field call is filtered by the caller's field guard.
fn check_initializer_receiver(
    global: crate::init_graph::GlobalRef,
    span: Span,
    param: &Param,
    program: &ProgramInfo<'_>,
    diagnostics: &mut Diagnostics,
) {
    check_initializer_global(global, span, param, program, diagnostics);
}

/// Checks a written argument for a global borrowed by the declared mode.
fn check_initializer_global_borrow(
    expr: &Expr,
    span: Span,
    param: &Param,
    module: &Module,
    loaded: &Loaded,
    program: &ProgramInfo<'_>,
    diagnostics: &mut Diagnostics,
) {
    check_initializer_global_borrow_mode(expr, span, param.mode, module, loaded, program, diagnostics);
}

/// Checks a written argument against a parameter mode.
fn check_initializer_global_borrow_mode(
    expr: &Expr,
    span: Span,
    mode: Mode,
    module: &Module,
    loaded: &Loaded,
    program: &ProgramInfo<'_>,
    diagnostics: &mut Diagnostics,
) {
    if mode == Mode::Take {
        return;
    }
    let Some((global, _)) = initializer_global(expr, module, loaded) else {
        return;
    };
    report_initializer_global_borrow(global, span, program, diagnostics);
}

/// Reports the element-move limit for an initializer `take` argument.
///
/// A function body reports `E10026` when a `take` argument projects one
/// element out of an ownerless array result; the initializer walk must apply
/// the same rule so the module-level spelling is not accepted with a leak.
fn check_initializer_take_element(
    expr: &Expr,
    span: Span,
    mode: Mode,
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) {
    if mode != Mode::Take || !initializer_ownerless_element_move(expr, sema) {
        return;
    }
    diagnostics.push(
        Diagnostic::error(
            "E10026",
            "partial moves are limited to top-level fields in v1",
            span,
        )
        .with_help("move the whole value instead"),
    );
}

/// True when an expression moves an owning element from an ownerless root.
///
/// This mirrors the function-body predicate: an index chain rooted at a call
/// selects an element with no place to move out of, and a conditional whose
/// arms contain such a read is judged by its arms. A conditional **root** is
/// not a call root, so `(c ? two() : two())[0]` stays accepted exactly as the
/// body accepts it.
fn initializer_ownerless_element_move(expr: &Expr, sema: &SemaOutput) -> bool {
    fn roots_in_call(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Index { base, .. } | ExprKind::Member { base, .. } => roots_in_call(base),
            ExprKind::Call { .. } => true,
            _ => false,
        }
    }
    fn selects_element(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Index { .. } => true,
            ExprKind::Member { base, .. } => selects_element(base),
            _ => false,
        }
    }
    let selected = selects_element(expr) && roots_in_call(expr);
    let conditional = match &expr.kind {
        ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } => {
            [then_value.as_ref(), else_value.as_ref()]
                .into_iter()
                .any(|arm| {
                    if matches!(arm.kind, ExprKind::Conditional { .. }) {
                        initializer_ownerless_element_move(arm, sema)
                    } else {
                        selects_element(arm)
                            && roots_in_call(arm)
                            && sema
                                .expr_types
                                .get(&(arm as *const Expr as usize))
                                .is_some_and(|ty| !sema.table.duplicable(ty))
                    }
                })
        }
        _ => false,
    };
    (selected || conditional)
        && sema
            .expr_types
            .get(&(expr as *const Expr as usize))
            .is_some_and(|ty| !sema.table.duplicable(ty))
}

/// Checks a func-typed callee's written arguments.
fn check_initializer_func_args(
    signature: &crate::types::FuncTy,
    args: &[Arg],
    module: &Module,
    loaded: &Loaded,
    program: &ProgramInfo<'_>,
    diagnostics: &mut Diagnostics,
) {
    for (index, arg) in args.iter().enumerate() {
        let Some(param) = signature.params.get(index) else {
            break;
        };
        check_initializer_global_borrow_mode(
            &arg.value,
            arg.span,
            param.mode,
            module,
            loaded,
            program,
            diagnostics,
        );
    }
}

/// Reports a non-`const` global borrowed by one parameter.
fn check_initializer_global(
    global: crate::init_graph::GlobalRef,
    span: Span,
    param: &Param,
    program: &ProgramInfo<'_>,
    diagnostics: &mut Diagnostics,
) {
    if param.mode == Mode::Take {
        return;
    }
    report_initializer_global_borrow(global, span, program, diagnostics);
}

/// Reports a non-`const` global borrowed by a call or view builder.
fn report_initializer_global_borrow(
    global: crate::init_graph::GlobalRef,
    span: Span,
    program: &ProgramInfo<'_>,
    diagnostics: &mut Diagnostics,
) {
    let id = GlobalId::new(global.module, global.name);
    let Some(info) = program.globals.get(&id) else {
        return;
    };
    if info.is_const {
        return;
    }
    diagnostics.push(
        Diagnostic::error(
            "E10038",
            format!("cannot borrow global '{}'", info.name),
            span,
        )
        .with_help("borrow from a local owner"),
    );
}

/// Reports a global borrowed as the source of a view builder.
fn check_initializer_global_source(
    expr: &Expr,
    span: Span,
    module: &Module,
    loaded: &Loaded,
    program: &ProgramInfo<'_>,
    diagnostics: &mut Diagnostics,
) {
    let Some((global, _)) = initializer_global(expr, module, loaded) else {
        return;
    };
    report_initializer_global_borrow(global, span, program, diagnostics);
}

/// The type of a dot receiver, when the type stage recorded it.
///
/// A member expression carries its own recorded type, so a projected receiver
/// (`g.h`) uses the projected type. A qualified receiver path has no separate
/// expression node; the global's declared type is projected through the
/// resolved fields.
fn initializer_receiver_type(
    receiver: &InitializerReceiver<'_>,
    module: &Module,
    loaded: &Loaded,
    program: &ProgramInfo<'_>,
    sema: &SemaOutput,
) -> Option<Ty> {
    match receiver {
        InitializerReceiver::None => None,
        InitializerReceiver::Expr(base) => sema
            .expr_types
            .get(&(*base as *const Expr as usize))
            .cloned()
            .or_else(|| {
                let (global, _) = initializer_global(base, module, loaded)?;
                projected_global_type(&global, program, sema)
            }),
        InitializerReceiver::Path { segments, span } => {
            let name = QualifiedName {
                segments: segments.clone(),
                span: *span,
            };
            let global = crate::init_graph::resolve_global(loaded, module, &name)?;
            projected_global_type(&global, program, sema)
        }
    }
}

/// Projects a global binding's type through the resolved field path.
fn projected_global_type(
    global: &crate::init_graph::GlobalRef,
    program: &ProgramInfo<'_>,
    sema: &SemaOutput,
) -> Option<Ty> {
    let id = GlobalId::new(global.module.clone(), global.name.clone());
    let info = program.globals.get(&id)?;
    let mut ty = info.ty.clone();
    for field in &global.fields {
        ty = sema.table.instance_field(&ty, field)?;
    }
    Some(ty)
}

/// Resolves a qualified receiver path to a module global.
fn initializer_global_path(
    segments: &[String],
    span: Span,
    module: &Module,
    loaded: &Loaded,
) -> Option<(crate::init_graph::GlobalRef, Span)> {
    let name = QualifiedName {
        segments: segments.to_vec(),
        span,
    };
    crate::init_graph::resolve_global(loaded, module, &name).map(|global| (global, span))
}

/// Resolves an expression's place to a module global.
fn initializer_global(
    expr: &Expr,
    module: &Module,
    loaded: &Loaded,
) -> Option<(crate::init_graph::GlobalRef, Span)> {
    let segments = initializer_place_segments(expr)?;
    let name = QualifiedName {
        segments,
        span: expr.span,
    };
    crate::init_graph::resolve_global(loaded, module, &name).map(|global| (global, expr.span))
}

/// Collects a name, member, or indexed place into its written segments.
///
/// An index contributes its base only: the element still names the same
/// binding, and the borrow rule judges the binding, not the index.
fn initializer_place_segments(expr: &Expr) -> Option<Vec<String>> {
    match &expr.kind {
        ExprKind::Name(name) => Some(vec![name.clone()]),
        ExprKind::Qualified(name) => Some(name.segments.clone()),
        ExprKind::Member { base, name } => {
            let mut segments = initializer_place_segments(base)?;
            segments.push(name.clone());
            Some(segments)
        }
        ExprKind::Index { base, .. } => initializer_place_segments(base),
        ExprKind::Unsafe(inner) => initializer_place_segments(inner),
        _ => None,
    }
}

/// True when a struct type declares a field with the given name.
fn receiver_has_field(ty: &Ty, field_name: &str, sema: &SemaOutput) -> bool {
    let TyKind::Struct(struct_name, _) = &ty.kind else {
        return false;
    };
    sema.table
        .struct_fields(struct_name)
        .is_some_and(|fields| fields.iter().any(|(name, _)| name == field_name))
}

/// The length of the longest leading module path in a qualified name.
fn longest_module_prefix(loaded: &Loaded, segments: &[String]) -> Option<usize> {
    (1..segments.len())
        .rev()
        .find(|split| loaded.has_module(&segments[..*split].join(".")))
}

/// Rejects a `?` suffix on a struct field whose type cannot be empty.
pub(super) fn check_struct_declarations(
    module: &Module,
    loaded: &Loaded,
    sema: &SemaOutput,
    diagnostics: &mut Diagnostics,
) {
    for item in &module.items {
        if let Item::Struct(decl) = item {
            let params: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
            for field in &decl.fields {
                let ty = resolve_ty_folded(&field.ty, &params, &sema.table, module, loaded);
                report_illegal_nullable(&ty, field.name_span, diagnostics);
            }
        }
    }
}

/// Reports `E10011` when a type carries `?` but cannot be empty.
fn report_illegal_nullable(ty: &Ty, span: Span, diagnostics: &mut Diagnostics) {
    if !ty.nullable || is_nullable_capable(ty) {
        return;
    }
    let base = Ty {
        kind: ty.kind.clone(),
        nullable: false,
    };
    diagnostics.push(
        Diagnostic::error(
            "E10011",
            format!("nullable type is not allowed on '{}'", base.display()),
            span,
        )
        .with_help("`?` applies to handles, `raw`, `cstring`, and views"),
    );
}

/// Collects every module const whose address is taken in the program.
///
/// The walk covers every body-bearing item and every expression child. A miss
/// would let a folded read disagree with an accepted `unsafe &N` write, so the
/// traversal is exhaustive over the AST rather than a best-effort scan.
fn collect_address_taken(loaded: &Loaded, info: &ProgramInfo<'_>) -> BTreeSet<GlobalId> {
    let mut found = BTreeSet::new();
    for module in loaded.modules.values() {
        for item in &module.items {
            match item {
                Item::Func(decl) | Item::Comptime(decl) => {
                    walk_block(&decl.body, loaded, module, info, &mut found);
                }
                Item::GlobalLet(decl) => {
                    walk_expr(&decl.value, loaded, module, info, &mut found);
                }
                Item::GlobalConst(decl) => {
                    walk_expr(&decl.value, loaded, module, info, &mut found);
                }
                _ => {}
            }
        }
    }
    found
}

/// Records one address operand's root when it resolves to a const.
fn note_addressed_root(
    operand: &Expr,
    loaded: &Loaded,
    module: &Module,
    info: &ProgramInfo<'_>,
    found: &mut BTreeSet<GlobalId>,
) {
    let global = match &operand.kind {
        ExprKind::Name(name) => crate::init_graph::resolve_name_global(loaded, module, name),
        ExprKind::Qualified(name) => crate::init_graph::resolve_global(loaded, module, name),
        ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
            return note_addressed_root(base, loaded, module, info, found);
        }
        ExprKind::Unsafe(inner) => {
            return note_addressed_root(inner, loaded, module, info, found);
        }
        _ => None,
    };
    let Some(global) = global else {
        return;
    };
    let id = GlobalId::new(global.module, global.name);
    if info.globals.get(&id).is_some_and(|global| global.is_const) {
        found.insert(id);
    }
}

/// Walks a block and its statements.
fn walk_block(
    block: &Block,
    loaded: &Loaded,
    module: &Module,
    info: &ProgramInfo<'_>,
    found: &mut BTreeSet<GlobalId>,
) {
    for statement in &block.statements {
        walk_stmt(statement, loaded, module, info, found);
    }
}

/// Walks one statement.
fn walk_stmt(
    statement: &Stmt,
    loaded: &Loaded,
    module: &Module,
    info: &ProgramInfo<'_>,
    found: &mut BTreeSet<GlobalId>,
) {
    match statement {
        Stmt::Let(decl) => {
            if let Some(value) = &decl.value {
                walk_expr(value, loaded, module, info, found);
            }
        }
        Stmt::Const(decl) => walk_expr(&decl.value, loaded, module, info, found),
        Stmt::Expr(expr) => walk_expr(expr, loaded, module, info, found),
        Stmt::If(decl) => {
            walk_expr(&decl.condition, loaded, module, info, found);
            walk_block(&decl.then_block, loaded, module, info, found);
            match &decl.else_branch {
                Some(crate::ast::ElseBranch::If(nested)) => {
                    walk_if(nested, loaded, module, info, found)
                }
                Some(crate::ast::ElseBranch::Block(block)) => {
                    walk_block(block, loaded, module, info, found)
                }
                None => {}
            }
        }
        Stmt::While(decl) => {
            walk_expr(&decl.condition, loaded, module, info, found);
            walk_block(&decl.body, loaded, module, info, found);
        }
        Stmt::DoWhile(decl) => {
            walk_block(&decl.body, loaded, module, info, found);
            walk_expr(&decl.condition, loaded, module, info, found);
        }
        Stmt::ForC(decl) => {
            match &decl.init {
                Some(ForInit::Let(init)) => {
                    if let Some(value) = &init.value {
                        walk_expr(value, loaded, module, info, found);
                    }
                }
                Some(ForInit::Const(init)) => {
                    walk_expr(&init.value, loaded, module, info, found)
                }
                Some(ForInit::Expr(expressions)) => {
                    for expression in expressions {
                        walk_expr(expression, loaded, module, info, found);
                    }
                }
                None => {}
            }
            if let Some(condition) = &decl.condition {
                walk_expr(condition, loaded, module, info, found);
            }
            for step in &decl.step {
                walk_expr(step, loaded, module, info, found);
            }
            walk_block(&decl.body, loaded, module, info, found);
        }
        Stmt::ForIn(decl) => {
            walk_expr(&decl.iterable, loaded, module, info, found);
            walk_block(&decl.body, loaded, module, info, found);
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                walk_expr(value, loaded, module, info, found);
            }
        }
        Stmt::Switch(decl) => {
            walk_expr(&decl.scrutinee, loaded, module, info, found);
            for clause in &decl.clauses {
                if let SwitchLabel::Case(value) = &clause.label {
                    walk_expr(value, loaded, module, info, found);
                }
                for statement in &clause.statements {
                    walk_stmt(statement, loaded, module, info, found);
                }
            }
        }
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
            walk_block(block, loaded, module, info, found);
        }
        Stmt::Break(_) | Stmt::Continue(_) => {}
    }
}

/// Walks an `else if` chain.
fn walk_if(
    decl: &crate::ast::IfStmt,
    loaded: &Loaded,
    module: &Module,
    info: &ProgramInfo<'_>,
    found: &mut BTreeSet<GlobalId>,
) {
    walk_expr(&decl.condition, loaded, module, info, found);
    walk_block(&decl.then_block, loaded, module, info, found);
    match &decl.else_branch {
        Some(crate::ast::ElseBranch::If(nested)) => walk_if(nested, loaded, module, info, found),
        Some(crate::ast::ElseBranch::Block(block)) => {
            walk_block(block, loaded, module, info, found)
        }
        None => {}
    }
}

/// Walks an expression and every child expression.
fn walk_expr(
    expr: &Expr,
    loaded: &Loaded,
    module: &Module,
    info: &ProgramInfo<'_>,
    found: &mut BTreeSet<GlobalId>,
) {
    match &expr.kind {
        ExprKind::Unary { op, operand } => {
            if *op == UnaryOp::AddressOf {
                note_addressed_root(operand, loaded, module, info, found);
            }
            walk_expr(operand, loaded, module, info, found);
        }
        ExprKind::Call { callee, args, .. } => {
            walk_expr(callee, loaded, module, info, found);
            for arg in args {
                walk_expr(&arg.value, loaded, module, info, found);
            }
        }
        ExprKind::Index { base, index } => {
            walk_expr(base, loaded, module, info, found);
            walk_expr(index, loaded, module, info, found);
        }
        ExprKind::Member { base, .. } => walk_expr(base, loaded, module, info, found),
        ExprKind::Binary { left, right, .. } => {
            walk_expr(left, loaded, module, info, found);
            walk_expr(right, loaded, module, info, found);
        }
        ExprKind::Assign { target, value, .. } => {
            walk_expr(target, loaded, module, info, found);
            walk_expr(value, loaded, module, info, found);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            walk_expr(condition, loaded, module, info, found);
            walk_expr(then_value, loaded, module, info, found);
            walk_expr(else_value, loaded, module, info, found);
        }
        ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
            walk_expr(value, loaded, module, info, found)
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                walk_expr(&field.value, loaded, module, info, found);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                walk_expr(element, loaded, module, info, found);
            }
            if let Some(fill) = fill {
                walk_expr(fill, loaded, module, info, found);
            }
        }
        ExprKind::Lambda { body, .. } => match body {
            LambdaBody::Expr(inner) => walk_expr(inner, loaded, module, info, found),
            LambdaBody::Block(block) => walk_block(block, loaded, module, info, found),
        },
        ExprKind::Unsafe(inner) => walk_expr(inner, loaded, module, info, found),
        ExprKind::IntrinsicCall { args, .. } => {
            for arg in args {
                if let IntrinsicArg::Expr(inner) = arg {
                    walk_expr(inner, loaded, module, info, found);
                }
            }
        }
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let TemplatePiece::Placeholder(inner) = piece {
                    walk_expr(inner, loaded, module, info, found);
                }
            }
        }
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

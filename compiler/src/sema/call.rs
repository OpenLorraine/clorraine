//! Call checking: type casts, function calls, dot-calls, and generics.
//!
//! A call has four shapes. A callee that names a type is a cast. A callee that
//! is a member access is a dot-call, which prefers a callable field and then a
//! free function with the receiver as its first argument. A bare callee that
//! names a module function is a direct call, and a generic function call also
//! checks its type arguments. Anything else stays opaque, so an unmodelled
//! external function does not cascade into false errors.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Arg, Expr, ExprKind, ScalarType, Type};
use crate::load::DeclKind;
use crate::span::Span;
use crate::types::{implicit_conversion, substitute, view_type, FuncParam, FuncSig, FuncTy, Ty, TyKind};

use super::Checker;

impl Checker<'_, '_> {
    /// Types a call: a type cast, a function call, a dot-call, or an opaque
    /// external call whose signature is not modelled.
    pub(super) fn check_call(
        &mut self,
        callee: &Expr,
        type_args: &[Type],
        args: &[Arg],
        span: Span,
        call_site: usize,
    ) -> Ty {
        // `Box<i32>(p)` is a call-shaped type conversion; a box is not
        // constructible from a value, so the ordinary cast judgment rejects it
        // with the `Box<i32>` spelling rather than an undeclared name.
        if let ExprKind::Name(name) = &callee.kind {
            if self.is_box_name(name) && self.builtin_box_active() && !self.name_is_value(name) {
                if type_args.len() != 1 {
                    self.error(
                        "E10300",
                        format!(
                            "wrong number of type arguments for 'Box': expected 1, found {}",
                            type_args.len()
                        ),
                        "list one type argument",
                        span,
                    );
                }
                let target = match type_args.first() {
                    Some(ty) => Ty::boxed(self.resolve_type_argument(ty)),
                    None => Ty::boxed(Ty::unknown()),
                };
                let arg_ty = match args.first() {
                    Some(arg) => self.check_value(&arg.value, None),
                    None => Ty::unknown(),
                };
                for extra in args.iter().skip(1) {
                    self.check_value(&extra.value, None);
                }
                return self.check_type_cast(&target, arg_ty, span);
            }
        }
        if let Some(target) = self.callee_type(callee) {
            for ty in type_args {
                self.resolve_type_argument(ty);
            }
            let arg_ty = match args.first() {
                Some(arg) => self.check_value(&arg.value, None),
                None => Ty::unknown(),
            };
            for extra in args.iter().skip(1) {
                self.check_value(&extra.value, None);
            }
            return self.check_type_cast(&target, arg_ty, span);
        }
        if let ExprKind::Member { base, name } = &callee.kind {
            let base_ty = self.check_value(base, None);
            return self.check_dot_call(Some(base), base_ty, name, type_args, args, callee.span);
        }
        if let ExprKind::Qualified(name) = &callee.kind {
            // A module path that names a function reaches that declaration
            // directly. A lexical binding of the head still shadows the path.
            // Routing through the direct/generic checker fixes the written type
            // arguments, arity, conversions, and the substituted return type.
            if name.segments.len() >= 2 && !self.is_local(&name.segments[0]) {
                let last = name.segments.last().cloned().unwrap_or_default();
                let module = name.segments[..name.segments.len() - 1].join(".");
                if let Some(signature) = self.table.function(&module, &last) {
                    if signature.type_params.is_empty() {
                        self.check_non_generic_type_args(&last, type_args, span);
                        self.check_args_direct(
                            args,
                            &signature.params,
                            &signature.modes,
                            Some((&last, span)),
                        );
                        return signature.returns.clone();
                    }
                    return self.check_generic_call(&last, &signature, type_args, args, span);
                }
                // An intrinsic or `ffi` declaration is not in the function
                // table. Qualified access reaches both: the compiler-known
                // helpers keep their intrinsic judgments, and an `ffi` call
                // uses the target module's written signature.
                match self.qualified_decl_kind(&module, &last) {
                    Some(DeclKind::Intrinsic) => match last.as_str() {
                        "len" => {
                            self.check_non_generic_intrinsic(
                                &last, args.len(), 1, type_args, span,
                            );
                            for arg in args {
                                let ty = self.check_value(&arg.value, None);
                                self.check_len_operand(&ty, arg.span);
                            }
                            return Ty::usize_ty();
                        }
                        "slice" | "rawSlice" => {
                            return self.check_view_builder(&last, type_args, args, span);
                        }
                        "rawStr" => {
                            self.check_non_generic_intrinsic(
                                &last, args.len(), 2, type_args, span,
                            );
                            for arg in args {
                                self.check_value(&arg.value, None);
                            }
                            return Ty {
                                kind: TyKind::Str,
                                nullable: false,
                            };
                        }
                        "boxAlloc" => {
                            return self.check_box_alloc(type_args, args, span);
                        }
                        _ => {}
                    },
                    Some(DeclKind::Ffi) => {
                        let signature = self
                            .qualified_ffi_signature(&module, &last)
                            .map(|signature| {
                                (
                                    signature
                                        .params
                                        .iter()
                                        .map(|param| (&param.ty, param.mode))
                                        .collect::<Vec<(&crate::ast::Type, crate::ast::Mode)>>(),
                                    &signature.returns,
                                )
                            });
                        if let Some((param_decls, returns)) = signature {
                            let params: Vec<(Ty, crate::ast::Mode)> = param_decls
                                .iter()
                                .map(|(ty, mode)| (self.resolve_type(ty), *mode))
                                .collect();
                            let result = Self::ffi_abi_result(self.resolve_type(returns));
                            self.check_ffi_arguments(
                                &last, &params, args, span, call_site, &result,
                            );
                            return result;
                        }
                    }
                    _ => {}
                }
            }
            // A dotted callee whose receiver is a value or a module-path
            // binding is a dot-call, exactly like the `Member` shape the
            // parser builds elsewhere. A module-path receiver resolves a
            // callable field, the target module's free function, or the
            // intrinsic `len` before it is treated as an undeclared path.
            if name.segments.len() >= 2 {
                let last = name.segments.len() - 1;
                let module_binding = self.module_path_binding(&name.segments[..last]);
                let receiver = self.qualified_receiver_type(name);
                if let Some((module, _)) = &module_binding {
                    let member = name.segments[last].clone();
                    if self.field_type(&receiver, &member).is_some() {
                        return self.check_dot_call(
                            None,
                            receiver,
                            &member,
                            type_args,
                            args,
                            name.span,
                        );
                    }
                    if module == &self.module.name
                        || self.loaded.module_exports(module, &member)
                    {
                        if let Some(signature) = self.table.function(module, &member).cloned() {
                            return self.check_dot_function(
                                None,
                                receiver,
                                &member,
                                type_args,
                                args,
                                &signature,
                                name.span,
                            );
                        }
                    }
                    if member == "len" {
                        self.check_non_generic_type_args("len", type_args, name.span);
                        if !args.is_empty() {
                            self.error(
                                "E10308",
                                "wrong number of arguments for 'len': expected 0, found 1",
                                "call `len` with no written arguments",
                                name.span,
                            );
                        }
                        self.check_len_operand(&receiver, name.span);
                        for arg in args {
                            self.check_value(&arg.value, None);
                        }
                        return Ty::usize_ty();
                    }
                    self.error(
                        "E10219",
                        format!("'{}' is not declared", name.segments.join(".")),
                        "check the spelling, the import, or the module",
                        name.span,
                    );
                    for arg in args {
                        self.check_value(&arg.value, None);
                    }
                    return Ty::unknown();
                }
                if !receiver.is_unknown() {
                    if let Some(member) = name.segments.last() {
                        let member = member.clone();
                        return self.check_dot_call(
                            None,
                            receiver,
                            &member,
                            type_args,
                            args,
                            name.span,
                        );
                    }
                }
            }
        }
        if let Some(name) = call_name(callee) {
            if !self.is_local(name) {
                if let Some(signature) = self.free_function(name) {
                    if !signature.type_params.is_empty() {
                        return self.check_generic_call(name, &signature, type_args, args, span);
                    }
                    self.check_non_generic_type_args(name, type_args, span);
                    self.check_args_direct(
                        args,
                        &signature.params,
                        &signature.modes,
                        Some((name, span)),
                    );
                    return signature.returns;
                }
                // An `ffi` declaration carries no checked signature in the
                // type table, so the ordinary call path checks nothing here.
                // The written signature is still available in the module: the
                // written count is judged against it, and the arguments are
                // walked for their own errors. The ABI crossing of a view or
                // handle argument is the emitter's rule, not a conversion.
                // The call's result carries the declared return type, so the
                // ordinary conversion judgments see it at the result's uses.
                if matches!(self.visible_decl_kind(name), Some(crate::load::DeclKind::Ffi)) {
                    if let Some(signature) = self.ffi_signature(name) {
                        let params = self.ffi_parameter_types(name, signature);
                        let result = self.ffi_result_type(name, signature);
                        self.check_ffi_arguments(name, &params, args, span, call_site, &result);
                        let _ = self.check_value(callee, None);
                        return result;
                    }
                }
                // The compiler-known `text(v)` formatter is not declared in
                // Clorraine; the compiler lowers it to a `String`-returning
                // shim conversion. Registering that return type here lets the
                // `String`-to-`str` widening apply when it is passed to a view.
                if name == "text" {
                    self.check_non_generic_type_args("text", type_args, span);
                    for arg in args {
                        self.check_value(&arg.value, None);
                    }
                    return Ty::handle("String");
                }
                // The view builders are compiler-known when their visible
                // declaration is the intrinsic one; a user or `ffi` function
                // that shadows the spelling keeps its own signature.
                if (name == "slice" || name == "rawSlice")
                    && matches!(
                        self.visible_decl_kind(name),
                        Some(crate::load::DeclKind::Intrinsic)
                    )
                {
                    return self.check_view_builder(&name, type_args, args, span);
                }
                if name == "boxAlloc"
                    && matches!(
                        self.visible_decl_kind(name),
                        Some(crate::load::DeclKind::Intrinsic)
                    )
                {
                    return self.check_box_alloc(type_args, args, span);
                }
            }
        }
        let callee_ty = self.check_value(callee, None);
        // A value callee has no type parameters of its own; a callable-bound
        // type parameter and a bare type parameter keep their own judgments
        // below. Every concrete value callee reports the written type
        // arguments once here, including a non-callable local or parameter.
        if !type_args.is_empty() && !matches!(callee_ty.kind, TyKind::Param(_)) {
            let name = call_name(callee).unwrap_or("the callee");
            self.check_non_generic_type_args(name, type_args, span);
        }
        match &callee_ty.kind {
            TyKind::Func(signature) => {
                let signature = signature.clone();
                let callee_name = call_name(callee).unwrap_or("the callable value");
                self.check_args_func(args, &signature.params, Some((callee_name, span)));
                return (*signature.returns).clone();
            }
            // A direct call of a capturing lambda literal goes through the
            // ephemeral two-word value. Its call result is the closure's own
            // return type, so a fixed-array result reaches the guard, the
            // discard, and the argument positions with its real type.
            TyKind::Closure(closure) => {
                let params: Vec<FuncParam> = closure
                    .params
                    .iter()
                    .cloned()
                    .map(|ty| FuncParam {
                        ty,
                        mode: crate::ast::Mode::None,
                    })
                    .collect();
                self.check_args_func(
                    args,
                    &params,
                    Some(("the callable value", callee.span)),
                );
                return (*closure.returns).clone();
            }
            // A callable-bound type parameter is called through its bound, so
            // the call has the bound's parameter types and return type even
            // though the parameter itself is opaque.
            TyKind::Param(name) if self.callable_bound_params.iter().any(|p| p == name) => {
                if let Some(bound) = self.current_callable_bound(name) {
                    let callee_name = call_name(callee).unwrap_or("the callable value");
                    self.check_args_func(args, &bound.params, Some((callee_name, span)));
                    return (*bound.returns).clone();
                }
            }
            // A bare type parameter carries no call operator: its
            // substitution may be any type, so the call is reported and the
            // fix is to declare a callable bound on the parameter.
            TyKind::Param(name) => {
                self.error(
                    "E10307",
                    format!("type parameter '{name}' is not callable"),
                    "declare a callable bound: `T: func(...)`",
                    callee.span,
                );
            }
            TyKind::Unknown if !callee_ty.nullable => {}
            TyKind::Void => {}
            _ => {
                let type_name = if crate::types::is_null_shaped(&callee_ty) {
                    "null".to_owned()
                } else {
                    callee_ty.display()
                };
                self.error(
                    "E10309",
                    format!("value of type '{type_name}' is not callable"),
                    "call a function or a func-typed value",
                    callee.span,
                );
            }
        }
        for arg in args {
            self.check_value(&arg.value, None);
        }
        Ty::unknown()
    }

    /// Reports the frozen wrong-type-argument count for a callee without type
    /// parameters.
    ///
    /// The grammar parses `a < b > (c)` as a generic call. The covered forms
    /// (free, qualified, dot, func-typed, and concrete value callees) report
    /// the same `E10300` once; an `ffi` declaration, a callable-bound type
    /// parameter, and a type-cast spelling keep their own pre-existing
    /// judgments. The written type arguments are still resolved so their own
    /// errors surface.
    fn check_non_generic_type_args(&mut self, name: &str, type_args: &[Type], span: Span) {
        if type_args.is_empty() {
            return;
        }
        self.error(
            "E10300",
            format!(
                "wrong number of type arguments for '{name}': expected 0, found {}",
                type_args.len()
            ),
            "remove the type arguments",
            span,
        );
        for ty in type_args {
            self.resolve_type_argument(ty);
        }
    }

    /// Checks the fixed arity and absent type parameters of an intrinsic.
    ///
    /// `len` and `rawStr` are declared without type parameters, so a written
    /// type argument is the ordinary wrong-count diagnostic. The argument
    /// count is judged against the declaration; every argument is still
    /// walked by the caller for its own errors.
    fn check_non_generic_intrinsic(
        &mut self,
        name: &str,
        found_args: usize,
        expected_args: usize,
        type_args: &[Type],
        span: Span,
    ) {
        self.check_non_generic_type_args(name, type_args, span);
        if found_args != expected_args {
            self.error(
                "E10308",
                format!(
                    "wrong number of arguments for '{name}': expected {expected_args}, found {found_args}"
                ),
                "pass one argument per parameter",
                span,
            );
        }
    }

    /// Types a `boxAlloc<T>()` intrinsic call.
    ///
    /// The intrinsic is declared with one type parameter and no value
    /// parameters. A wrong count reports the ordinary generic/arity codes; the
    /// result is the built-in `Box<T>` type the lowering turns into the
    /// generated allocation helper.
    fn check_box_alloc(&mut self, type_args: &[Type], args: &[Arg], span: Span) -> Ty {
        for arg in args {
            self.check_value(&arg.value, None);
        }
        if type_args.len() != 1 {
            self.error(
                "E10300",
                format!(
                    "wrong number of type arguments for 'boxAlloc': expected 1, found {}",
                    type_args.len()
                ),
                "list one element type",
                span,
            );
            return Ty::boxed(Ty::unknown());
        }
        if !args.is_empty() {
            self.error(
                "E10308",
                format!(
                    "wrong number of arguments for 'boxAlloc': expected 0, found {}",
                    args.len()
                ),
                "pass no value arguments",
                span,
            );
        }
        let element = self.resolve_type_argument(&type_args[0]);
        Ty::boxed(element)
    }

    /// Types a compiler-known view builder call.
    ///
    /// `slice(v, lo, hi)` and `rawSlice<T>(p, n)` produce a view whose element
    /// is one step of iteration over the source: whole rows for a multi-extent
    /// source, the scalar for a single-extent one; an explicit type argument
    /// names the element directly, and a `raw` source falls back to its
    /// pointee. Recording the element here is what lets the ordinary
    /// conversion rules reject a view offered where a different element is
    /// expected, before the ownership stage lowers it.
    fn check_view_builder(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Arg],
        span: Span,
    ) -> Ty {
        if type_args.len() > 1 {
            self.error(
                "E10300",
                format!(
                    "wrong number of type arguments for '{name}': expected 1, found {}",
                    type_args.len()
                ),
                "list one element type",
                span,
            );
        }
        let expected_args = if name == "slice" { 3 } else { 2 };
        if args.len() != expected_args {
            self.error(
                "E10308",
                format!(
                    "wrong number of arguments for '{name}': expected {expected_args}, found {}",
                    args.len()
                ),
                "pass one argument per parameter",
                span,
            );
        }
        let mut explicit: Option<Ty> = None;
        for ty in type_args {
            let resolved = self.resolve_type_argument(ty);
            if explicit.is_none() {
                explicit = Some(resolved);
            }
        }
        let mut source: Option<Ty> = None;
        for arg in args {
            let ty = self.check_value(&arg.value, None);
            if source.is_none() {
                source = Some(ty);
            }
        }
        let element = explicit.or_else(|| {
            source.as_ref().map(|ty| match &ty.kind {
                TyKind::Array { .. } => ty.iterated_element(),
                TyKind::Raw(inner) => (**inner).clone(),
                _ => Ty::unknown(),
            })
        });
        view_type(element.unwrap_or_else(Ty::unknown))
    }

    /// The resolved callable bound of the current function's type parameter.
    ///
    /// The bound is collected with the function's signature, so a call through
    /// the parameter sees the same parameter types and return type the bound
    /// declares. `None` outside a function body or for an unbounded parameter.
    fn current_callable_bound(&self, name: &str) -> Option<crate::types::FuncTy> {
        let function = self.function_name.as_deref()?;
        let signature = self.table.function(&self.module.name, function)?;
        signature
            .type_params
            .iter()
            .find(|param| param.name == name)
            .and_then(|param| param.bound.clone())
    }

    /// Types a call of a generic function, checking its type arguments.
    ///
    /// With explicit type arguments the arity must match. Without them the
    /// argument types must determine every type parameter; a constructor with
    /// no value argument cannot be inferred. A call inside the same generic
    /// function whose inferred argument grows is an unbounded instantiation.
    fn check_generic_call(
        &mut self,
        name: &str,
        signature: &FuncSig,
        type_args: &[Type],
        args: &[Arg],
        span: Span,
    ) -> Ty {
        let declared: Vec<String> = signature
            .type_params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        // Explicit type arguments are resolved before the written arguments,
        // so a fitting literal takes the substituted parameter's type exactly
        // as the concrete twin does.
        let mut bindings: BTreeMap<String, Ty> = BTreeMap::new();
        if !type_args.is_empty() {
            let mut resolved = Vec::with_capacity(type_args.len());
            for ty in type_args {
                resolved.push(self.resolve_type_argument(ty));
            }
            if type_args.len() != declared.len() {
                self.error(
                    "E10300",
                    format!(
                        "wrong number of type arguments for '{name}': expected {}, found {}",
                        declared.len(),
                        type_args.len()
                    ),
                    "list all type arguments",
                    span,
                );
            }
            for (param, ty) in declared.iter().zip(resolved) {
                bindings.insert(param.clone(), ty);
            }
        }
        // A literal at a parameter that does not name a type parameter is
        // typed by that parameter, exactly as a non-generic call types it: the
        // destination supplies the context, and a value that does not fit
        // reports the fit rule at the literal. An explicit binding supplies the
        // substituted context before inference. Every other argument stays
        // context-free: a parameter that still names a type parameter needs its
        // argument's own type to bind the instance.
        let mut arg_types: Vec<(Ty, Span, bool)> = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                let mode = signature
                    .modes
                    .get(index)
                    .copied()
                    .unwrap_or(crate::ast::Mode::None);
                let expected = signature
                    .params
                    .get(index)
                    .map(|param| substitute(param, &bindings));
                let (ty, judged) = match (signature.params.get(index), expected) {
                    (Some(param), Some(expected))
                        if !self.is_out_slot_param(&expected, mode)
                            && Self::literal_takes_param_context(&expected, &arg.value) =>
                    {
                        if is_cstring_literal_argument(&arg.value, &expected)
                            && !crate::types::contains_param(param)
                        {
                            (self.check_value(&arg.value, None), true)
                        } else {
                            (self.check_into(&arg.value, &expected), true)
                        }
                    }
                    (Some(param), _)
                        if !self.is_out_slot_param(param, mode)
                            && Self::literal_takes_param_context(param, &arg.value) =>
                    {
                        if is_cstring_literal_argument(&arg.value, param) {
                            (self.check_value(&arg.value, None), true)
                        } else {
                            (self.check_into(&arg.value, param), true)
                        }
                    }
                    _ => (self.check_value(&arg.value, None), false),
                };
                (ty, arg.value.span, judged)
            })
            .collect();

        // The written count is judged against the declared parameters like
        // every other direct call: the emitter's prototype would reject a
        // mismatch.
        if args.len() != signature.params.len() {
            self.error(
                "E10308",
                format!(
                    "wrong number of arguments for '{name}': expected {}, found {}",
                    signature.params.len(),
                    args.len()
                ),
                "pass one argument per parameter",
                span,
            );
        }

        if type_args.is_empty() {
            // A non-literal argument keeps its own type and is the stable
            // anchor for inference; a numeric literal may take either side's
            // type, so it binds only after the non-literals have fixed the
            // parameters it does not yet constrain.
            for (index, ((ty, _, _), param)) in
                arg_types.iter().zip(&signature.params).enumerate()
            {
                if !matches!(
                    args[index].value.kind,
                    ExprKind::Int(_)
                        | ExprKind::Float(_)
                        | ExprKind::Char(_)
                        | ExprKind::Str(_)
                        | ExprKind::Bool(_)
                        | ExprKind::Null
                ) {
                    match_param(param, ty, &declared, &mut bindings);
                }
            }
            for (index, ((ty, _, _), param)) in
                arg_types.iter().zip(&signature.params).enumerate()
            {
                if matches!(
                    args[index].value.kind,
                    ExprKind::Int(_)
                        | ExprKind::Float(_)
                        | ExprKind::Char(_)
                        | ExprKind::Str(_)
                        | ExprKind::Bool(_)
                        | ExprKind::Null
                ) {
                    match_param(param, ty, &declared, &mut bindings);
                }
            }
            // The inference fixed a parameter from another argument; a literal
            // argument now takes that substituted type instead of its default
            // one. Only literals are re-typed, so no non-literal expression is
            // evaluated twice.
            for (index, arg) in args.iter().enumerate() {
                if arg_types.get(index).is_some_and(|(_, _, judged)| *judged) {
                    continue;
                }
                let Some(param) = signature.params.get(index) else {
                    continue;
                };
                let expected = substitute(param, &bindings);
                if expected.is_unknown()
                    || crate::types::contains_param(&expected)
                    || is_cstring_literal_argument(&arg.value, &expected)
                    || !Self::literal_takes_param_context(&expected, &arg.value)
                {
                    continue;
                }
                let ty = if is_cstring_literal_argument(&arg.value, &expected) {
                    self.check_value(&arg.value, None)
                } else {
                    self.check_into(&arg.value, &expected)
                };
                arg_types[index] = (ty, arg.value.span, true);
            }
            // A written count that disagrees with the declared parameters is
            // reported on its own: the inference cannot bind from arguments
            // that are missing, so its reports would be a cascade of the
            // arity mistake.
            if args.len() == signature.params.len() {
                if declared.iter().any(|param| !bindings.contains_key(param)) {
                    self.error(
                        "E10301",
                        format!("cannot infer type arguments for '{name}'"),
                        "write the type arguments explicitly",
                        span,
                    );
                } else if self.is_recursive_instantiation(name, &declared, &bindings) {
                    self.error(
                        "E10305",
                        format!("instantiation of '{name}' is recursive beyond the limit"),
                        "break the generic recursion",
                        span,
                    );
                }
            }
        }

        self.check_callable_bounds(&signature.type_params, &bindings, span);
        self.check_ephemeral_bindings(name, &signature.type_params, &bindings, span);
        for (index, arg) in args.iter().enumerate() {
            let mode = signature
                .modes
                .get(index)
                .copied()
                .unwrap_or(crate::ast::Mode::None);
            if mode == crate::ast::Mode::Take {
                self.reject_capture_move(&arg.value);
                self.record_generic_capture_move(&arg.value);
            }
            if let (Some((ty, arg_span, judged)), Some(param)) =
                (arg_types.get(index), signature.params.get(index))
            {
                // A literal at a concrete parameter was judged by the
                // non-generic check above, so its conversion is complete.
                if *judged {
                    continue;
                }
                let expected = crate::types::substitute(param, &bindings);
                if self.is_out_slot_param(&expected, mode) {
                    // The instance makes the parameter a nullable out-slot, so
                    // the same exact-place rule a direct call enforces applies
                    // to the inferred type arguments.
                    self.check_out_slot_place(arg, ty, &expected);
                } else {
                    // The resolved parameter judges the argument exactly as a
                    // direct call's does: a destination or a source that still
                    // mentions a type parameter records the deferred
                    // conversion for the instance audit. A numeric literal
                    // carries its marker, and a borrow argument whose declared
                    // slot still mentions a type parameter keeps the out-slot
                    // place duty's single instance report instead.
                    let literal = Self::conversion_literal(&arg.value);
                    // A borrow slot whose declared type still mentions a
                    // parameter is judged per instance: the audit leaves a
                    // nullable substitution to the out-slot place duty and
                    // reports the non-nullable ones itself.
                    let borrow_slot = mode == crate::ast::Mode::Borrow
                        && signature
                            .params
                            .get(index)
                            .is_some_and(crate::types::contains_param);
                    // A conditional's own duty owns its destination; the
                    // instance audit judges it after substitution.
                    let deferred_conditional =
                        self.note_conditional_destination(&arg.value, &expected);
                    if !deferred_conditional {
                        self.record_deferred_conversion_with(
                            ty,
                            &expected,
                            *arg_span,
                            literal,
                            borrow_slot,
                        );
                    }
                    self.check_conversion(ty, &expected, *arg_span);
                    // The argument was typed before the inference, so a
                    // conditional's null arm faces its destination only now.
                    self.judge_conditional_argument(&arg.value, &expected);
                }
            }
        }
        let returns = crate::types::substitute(&signature.returns, &bindings);
        self.reject_fixed_array_sequence(&returns, span);
        returns
    }

    /// Reports `E10306` when an unbounded generic parameter is bound to a type
    /// that contains an ephemeral closure.
    ///
    /// A callable-bound parameter may receive a capturing closure because the
    /// closure is consumed by the synchronous call. A plain parameter has no
    /// such bound, so an aggregate such as `Holder<closure>` would try to store
    /// the closure and is rejected.
    fn check_ephemeral_bindings(
        &mut self,
        name: &str,
        type_params: &[crate::types::TypeParamSig],
        bindings: &BTreeMap<String, Ty>,
        span: Span,
    ) {
        for param in type_params {
            if param.bound.is_some() {
                continue;
            }
            if let Some(concrete) = bindings.get(&param.name) {
                if concrete.contains_closure() {
                    self.error(
                        "E10306",
                        format!(
                            "'{name}' cannot be instantiated with an ephemeral type argument"
                        ),
                        "use a storable type",
                        span,
                    );
                }
            }
        }
    }

    /// Checks every concrete type argument against a declared callable bound.
    ///
    /// A parameter declared `F: func(T): R` accepts only a function (or a
    /// closure) whose signature matches the bound after the other arguments are
    /// substituted. A scalar or any other type is `E10303`.
    fn check_callable_bounds(
        &mut self,
        type_params: &[crate::types::TypeParamSig],
        bindings: &BTreeMap<String, Ty>,
        span: Span,
    ) {
        for param in type_params {
            let Some(bound) = &param.bound else {
                continue;
            };
            let Some(concrete) = bindings.get(&param.name) else {
                continue;
            };
            let expected = crate::types::substitute(&Ty::func(bound.clone()), bindings);
            let matches = match (&concrete.kind, &expected.kind) {
                (TyKind::Func(found), TyKind::Func(want)) => found == want,
                // A capturing closure satisfies a callable bound by signature;
                // the synchronous call is what makes its environment valid.
                (TyKind::Closure(found), TyKind::Func(want)) => {
                    found.params.len() == want.params.len()
                        && found
                            .params
                            .iter()
                            .zip(&want.params)
                            .all(|(found, want)| *found == want.ty)
                        && found.returns == want.returns
                }
                _ => false,
            };
            if !matches {
                self.error(
                    "E10303",
                    format!(
                        "type argument '{}' does not satisfy '{}: func(...)'",
                        concrete.display(),
                        param.name
                    ),
                    "pass a matching function or lambda",
                    span,
                );
            }
        }
    }

    /// True when a generic call instantiates the function currently being
    /// checked with an argument that is not the function's own parameter.
    ///
    /// Only the inference path is considered: an explicit type argument is a
    /// deliberate instantiation and may terminate.
    fn is_recursive_instantiation(
        &self,
        name: &str,
        declared: &[String],
        inferred: &BTreeMap<String, Ty>,
    ) -> bool {
        if self.function_name.as_deref() != Some(name) {
            return false;
        }
        declared.iter().any(|param| {
            inferred
                .get(param)
                .is_some_and(|ty| !matches!(&ty.kind, TyKind::Param(inner) if inner == param))
        })
    }

    /// Checks a free-function candidate reached with the dot spelling.
    fn check_dot_function(
        &mut self,
        receiver_expr: Option<&Expr>,
        receiver: Ty,
        name: &str,
        type_args: &[Type],
        args: &[Arg],
        signature: &FuncSig,
        span: Span,
    ) -> Ty {
        if signature.params.is_empty() {
            self.error(
                "E10308",
                format!(
                    "wrong number of arguments for '{name}': expected 0, found {}",
                    args.len() + 1
                ),
                "call a function that takes the receiver as its first parameter",
                span,
            );
            for arg in args {
                self.check_value(&arg.value, None);
            }
            return signature.returns.clone();
        }
        if signature.type_params.is_empty() {
            self.check_non_generic_type_args(name, type_args, span);
            let (receiver_param, rest) = signature
                .params
                .split_first()
                .expect("the empty candidate was handled above");
            let receiver_mode = signature
                .modes
                .first()
                .copied()
                .unwrap_or(crate::ast::Mode::None);
            self.check_dot_receiver(
                receiver_expr,
                &receiver,
                receiver_param,
                receiver_mode,
                span,
            );
            let rest_modes = signature.modes.get(1..).unwrap_or(&[]);
            self.check_args_impl(args, rest, rest_modes, true, Some((name, span)));
            signature.returns.clone()
        } else {
            self.check_generic_dot_call(
                receiver_expr,
                &receiver,
                name,
                type_args,
                args,
                signature,
                span,
            )
        }
    }

    /// Types a dot-call `receiver.name(args)`.
    ///
    /// A callable field wins over a free function and carries no receiver
    /// parameter. A free-function candidate takes the receiver as its first
    /// parameter, so the receiver is checked against it and the remaining
    /// parameters are matched against the written arguments.
    fn check_dot_call(
        &mut self,
        receiver_expr: Option<&Expr>,
        receiver: Ty,
        name: &str,
        type_args: &[Type],
        args: &[Arg],
        span: Span,
    ) -> Ty {
        let call = self.dot_call(&receiver, name);
        // Only the intrinsic `len` receiver is a length operand; a callable
        // field (or a user function) named `len` keeps its own signature.
        let intrinsic_len = matches!(
            &call,
            Some(DotCall::Function(signature))
                if signature.params.len() == 1 && signature.params[0].is_unknown()
        );
        if name == "len" && intrinsic_len {
            self.check_len_operand(&receiver, span);
        }
        match call {
            Some(DotCall::Field(signature)) => {
                // A callable field carries its own parameter modes, so a
                // `take` argument still moves and a call-site marker still
                // matches, exactly as on a named call.
                self.check_non_generic_type_args(name, type_args, span);
                self.check_args_impl(
                    args,
                    &signature.params,
                    &signature.modes,
                    false,
                    Some((name, span)),
                );
                signature.returns
            }
            Some(DotCall::Function(signature)) => {
                self.check_dot_function(
                    receiver_expr,
                    receiver,
                    name,
                    type_args,
                    args,
                    &signature,
                    span,
                )
            }
            None => {
                // A field of a bare type parameter is not callable: its
                // substitution may be any type, and the fix is to declare a
                // callable bound on the parameter. A bound parameter's field
                // keeps its call path above.
                if let Some(field_ty) = self.field_type(&receiver, name) {
                    if let TyKind::Param(param) = &field_ty.kind {
                        if !self.callable_bound_params.iter().any(|p| p == param) {
                            self.error(
                                "E10307",
                                format!("type parameter '{param}' is not callable"),
                                "declare a callable bound: `T: func(...)`",
                                span,
                            );
                        }
                    } else if !receiver.is_unknown() {
                        self.error(
                            "E10309",
                            format!("value of type '{}' is not callable", field_ty.display()),
                            "call a function or a func-typed value",
                            span,
                        );
                    }
                } else if !receiver.is_unknown() {
                    // A receiver whose surface holds no field of that name and
                    // no matching function candidate has an unknown member: the
                    // call may not reach the emitter, where it would become an
                    // implicit call.
                    self.error(
                        "E10212",
                        format!("'{name}' is not a member of '{}'", receiver.display()),
                        "check the declaration or the spelling",
                        span,
                    );
                }
                for arg in args {
                    self.check_value(&arg.value, None);
                }
                Ty::unknown()
            }
        }
    }

    /// Checks a dot-call receiver against the candidate's first parameter.
    ///
    /// A nullable borrow receiver is an ownership out-slot and keeps the exact
    /// place rule; every other receiver records the same deferred conversion a
    /// written argument does, so a generic caller's instance judges it.
    fn check_dot_receiver(
        &mut self,
        receiver_expr: Option<&Expr>,
        receiver: &Ty,
        param: &Ty,
        mode: crate::ast::Mode,
        span: Span,
    ) {
        if self.is_out_slot_param(param, mode) {
            self.check_out_slot_receiver(receiver_expr, receiver, param, span);
            return;
        }
        self.record_deferred_conversion(
            receiver,
            param,
            receiver_expr.map(|expr| expr.span).unwrap_or(span),
        );
        self.check_conversion(receiver, param, span);
    }

    /// True when a written literal can take its type from a concrete
    /// parameter.
    ///
    /// An integer literal takes an integer scalar's type, a float literal a
    /// float scalar's, and a static string literal is the direct call's
    /// `cstring` allowance. Every other pairing keeps the literal's default
    /// type so the ordinary conversion reports once.
    fn literal_takes_param_context(param: &Ty, arg: &Expr) -> bool {
        match param.kind {
            TyKind::Scalar(scalar) if crate::types::is_integer_scalar(scalar) => {
                crate::types::int_literal_value(arg).is_some()
            }
            TyKind::Scalar(ScalarType::F32 | ScalarType::F64) => {
                matches!(arg.kind, ExprKind::Float(_))
            }
            TyKind::CString => is_cstring_literal_argument(arg, param),
            _ => false,
        }
    }

    /// The declared type parameters of a candidate that still have no binding.
    ///
    /// A parameter missing from the map has no inferred instance at this call
    /// site, so a parameter type that names it cannot be judged here.
    fn unbound_params(declared: &[String], bindings: &BTreeMap<String, Ty>) -> BTreeSet<String> {
        declared
            .iter()
            .filter(|name| !bindings.contains_key(*name))
            .cloned()
            .collect()
    }

    /// Checks a generic dot-call candidate.
    ///
    /// The receiver is the call's argument at index zero, so it anchors the
    /// candidate's first parameter; together with the written arguments it
    /// fixes the candidate's type arguments exactly as a plain call's
    /// arguments do. An argument whose parameter the receiver already fixed is
    /// judged immediately, like a non-generic candidate's, so a literal keeps
    /// the parameter's context type. The rest are typed once for the type
    /// arguments they contribute and judged after the inference; a parameter
    /// that stays unresolved is left to the instantiation rather than recorded
    /// against the enclosing function.
    fn check_generic_dot_call(
        &mut self,
        receiver_expr: Option<&Expr>,
        receiver: &Ty,
        name: &str,
        type_args: &[Type],
        args: &[Arg],
        signature: &FuncSig,
        span: Span,
    ) -> Ty {
        let declared: Vec<String> = signature
            .type_params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        let receiver_mode = signature
            .modes
            .first()
            .copied()
            .unwrap_or(crate::ast::Mode::None);
        let rest_modes = signature.modes.get(1..).unwrap_or(&[]);
        let rest = signature.params.get(1..).unwrap_or(&[]);
        let mut bindings: BTreeMap<String, Ty> = BTreeMap::new();
        if !type_args.is_empty() {
            let mut resolved = Vec::with_capacity(type_args.len());
            for ty in type_args {
                resolved.push(self.resolve_type_argument(ty));
            }
            if type_args.len() != declared.len() {
                self.error(
                    "E10300",
                    format!(
                        "wrong number of type arguments for '{name}': expected {}, found {}",
                        declared.len(),
                        type_args.len()
                    ),
                    "list all type arguments",
                    span,
                );
            }
            for (param, ty) in declared.iter().zip(resolved) {
                bindings.insert(param.clone(), ty);
            }
        }
        // An explicit type-argument list fixes the instance itself. The
        // receiver and arguments are still checked against the substituted
        // parameters, but they must not infer a binding the author did not
        // write: a short list is judged by its count alone, exactly as the
        // direct call judges it.
        let infer = type_args.is_empty();
        if infer {
            if let Some(param) = signature.params.first() {
                match_param(param, receiver, &declared, &mut bindings);
            }
        }
        // The written count is judged against the declared parameters like
        // every other call; a mismatch reports once and still walks the
        // arguments for their own errors.
        if args.len() != rest.len() {
            self.error(
                "E10308",
                format!(
                    "wrong number of arguments for '{name}': expected {}, found {}",
                    rest.len(),
                    args.len()
                ),
                "pass one argument per parameter",
                span,
            );
            for arg in args {
                self.check_value(&arg.value, None);
            }
            self.check_dot_receiver_call(
                receiver_expr,
                receiver,
                signature,
                receiver_mode,
                &bindings,
                span,
            );
            return signature.returns.clone();
        }
        // Walk the written arguments once. A parameter the receiver already
        // fixed is judged here; the others contribute their type to the
        // inference below.
        let mut pending: Vec<Option<Ty>> = Vec::with_capacity(args.len());
        let receiver_unbound = Self::unbound_params(&declared, &bindings);
        for (index, arg) in args.iter().enumerate() {
            let mode = rest_modes
                .get(index)
                .copied()
                .unwrap_or(crate::ast::Mode::None);
            if mode == crate::ast::Mode::Take {
                self.reject_capture_move(&arg.value);
                self.record_generic_capture_move(&arg.value);
            }
            let expected = rest.get(index).map(|param| substitute(param, &bindings));
            match expected {
                Some(param) if !mentions_unbound(&param, &receiver_unbound) => {
                    if self.is_out_slot_param(&param, mode) {
                        self.check_out_slot_argument(arg, &param);
                    } else {
                        self.check_into(&arg.value, &param);
                    }
                    pending.push(None);
                }
                _ => pending.push(Some(self.check_value(&arg.value, None))),
            }
        }
        if infer {
            for (index, ty) in pending.iter().enumerate() {
                if let Some(ty) = ty {
                    if let Some(param) = rest.get(index) {
                        match_param(param, ty, &declared, &mut bindings);
                    }
                }
            }
        }
        let unbound = Self::unbound_params(&declared, &bindings);
        // The direct call judges the instance it inferred: a parameter that
        // no receiver or argument fixes is `E10301`, and a recursive
        // instantiation is `E10305`. A dot call reaches the same routes. An
        // explicit type-argument list fixes the instance itself, so it is
        // judged by its count instead; the unbound report would duplicate the
        // count report on a short list.
        if infer {
            if unbound.is_empty() {
                if self.is_recursive_instantiation(name, &declared, &bindings) {
                    self.error(
                        "E10305",
                        format!("instantiation of '{name}' is recursive beyond the limit"),
                        "break the generic recursion",
                        span,
                    );
                }
            } else {
                self.error(
                    "E10301",
                    format!("cannot infer type arguments for '{name}'"),
                    "write the type arguments explicitly",
                    span,
                );
            }
        }
        self.check_callable_bounds(&signature.type_params, &bindings, span);
        self.check_ephemeral_bindings(name, &signature.type_params, &bindings, span);
        self.check_dot_receiver_call(
            receiver_expr,
            receiver,
            signature,
            receiver_mode,
            &bindings,
            span,
        );
        for (index, ty) in pending.iter().enumerate() {
            let (Some(ty), Some(arg)) = (ty, args.get(index)) else {
                continue;
            };
            let Some(param) = rest.get(index).map(|param| substitute(param, &bindings)) else {
                continue;
            };
            let mode = rest_modes
                .get(index)
                .copied()
                .unwrap_or(crate::ast::Mode::None);
            if self.is_out_slot_param(&param, mode) {
                self.check_out_slot_place(arg, ty, &param);
                continue;
            }
            if mentions_unbound(&param, &unbound) {
                continue;
            }
            // A borrow slot whose declared type still mentions a parameter is
            // judged per instance: the audit leaves a nullable substitution to
            // the out-slot place duty and reports the non-nullable ones itself.
            let borrow_slot = mode == crate::ast::Mode::Borrow
                && rest
                    .get(index)
                    .is_some_and(crate::types::contains_param);
            // A conditional's own duty owns its destination; the instance
            // audit judges it after substitution.
            let deferred_conditional = self.note_conditional_destination(&arg.value, &param);
            if !deferred_conditional {
                self.record_deferred_conversion_with(
                    ty,
                    &param,
                    arg.value.span,
                    Self::conversion_literal(&arg.value),
                    borrow_slot,
                );
            }
            self.check_conversion(ty, &param, arg.value.span);
            // The argument was typed before the inference, so a
            // conditional's null arm faces its destination only now.
            self.judge_conditional_argument(&arg.value, &param);
        }
        substitute(&signature.returns, &bindings)
    }

    /// Checks the receiver of a generic dot call against its resolved first
    /// parameter, when the call site fixes it.
    fn check_dot_receiver_call(
        &mut self,
        receiver_expr: Option<&Expr>,
        receiver: &Ty,
        signature: &FuncSig,
        mode: crate::ast::Mode,
        bindings: &BTreeMap<String, Ty>,
        span: Span,
    ) {
        let Some(param) = signature
            .params
            .first()
            .map(|param| substitute(param, bindings))
        else {
            return;
        };
        let declared: Vec<String> = signature
            .type_params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        if mentions_unbound(&param, &Self::unbound_params(&declared, bindings)) {
            return;
        }
        self.check_dot_receiver(receiver_expr, receiver, &param, mode, span);
    }

    /// Resolves the candidate a dot-call names.
    ///
    /// A struct field of that name wins outright: a callable field is a
    /// `Field` candidate, and a non-callable field stops the search. Without a
    /// field, a visible free function is a `Function` candidate, and the
    /// receiver-taking `len` intrinsic stands in as one so `xs.len()` works.
    fn dot_call(&self, base: &Ty, name: &str) -> Option<DotCall> {
        // A field lookup follows a raw pointer to its pointee, so a callable
        // field reached through `raw S` is the same candidate as a value
        // receiver's field.
        if let Some(field_ty) = self.field_type(base, name) {
            return match &field_ty.kind {
                TyKind::Func(signature) => Some(DotCall::Field(FuncSig {
                    params: signature
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    modes: signature.params.iter().map(|param| param.mode).collect(),
                    returns: (*signature.returns).clone(),
                    type_params: Vec::new(),
                })),
                _ => None,
            };
        }
        if let TyKind::Struct(struct_name, _) = &base.kind {
            // The growable array's operations are compiler-known standard
            // library functions; a caller reaches them through a dot-call
            // without importing each method name.
            if struct_name == "Array" {
                if let Some(signature) = self.table.function("std.mem", name) {
                    return Some(DotCall::Function(signature.clone()));
                }
            }
        }
        if let Some(signature) = self.free_function(name) {
            // A value-headed dot call needs a receiver-taking parameter. The
            // Array and module-path arms report the zero-parameter arity;
            // a value receiver keeps the unknown-member report.
            if !signature.params.is_empty() {
                return Some(DotCall::Function(signature));
            }
        }
        // A method-shaped call on a value may reach a free function exported
        // from the type's declaring module. The caller needs only the type
        // import; the receiver still has to match the function's first
        // parameter, so the ordinary dot-call check enforces the borrow and
        // ownership rules.
        if let TyKind::Struct(struct_name, _) = &base.kind {
            if let Some(module) = self.struct_module(struct_name) {
                if module == self.module.name || self.loaded.module_exports(module, name) {
                    if let Some(signature) = self.table.function(module, name) {
                        if !signature.params.is_empty() {
                            return Some(DotCall::Function(signature.clone()));
                        }
                    }
                }
            }
        }
        self.intrinsic_dot_call(name)
    }

    /// The signature of an intrinsic reachable as a dot-call.
    ///
    /// `xs.len()` is the documented spelling of `len(xs)`, so `len` is offered
    /// as a receiver-taking free function. Its parameter type is opaque
    /// because it accepts an array, a slice, or a string.
    fn intrinsic_dot_call(&self, name: &str) -> Option<DotCall> {
        match name {
            "len" => Some(DotCall::Function(FuncSig {
                params: vec![Ty::unknown()],
                modes: Vec::new(),
                returns: Ty::usize_ty(),
                type_params: Vec::new(),
            })),
            _ => None,
        }
    }

    /// Checks a call's arguments against a known parameter list.
    ///
    /// `callee` carries the call's name and span when the call names its
    /// callee, so an arity mismatch reports through the same diagnostic every
    /// other direct call uses; a call through a value passes `None`.
    fn check_args(&mut self, args: &[Arg], params: &[Ty], callee: Option<(&str, Span)>) {
        self.check_args_impl(args, params, &[], false, callee);
    }

    /// Checks a call's arguments against a func-value parameter list.
    ///
    /// A call through a `func(...)` value keeps the modes its type carries, so
    /// a `take` parameter still moves the argument and a `borrow` parameter
    /// still accepts the lend. The callee is a value, not a declared name, so
    /// an arity mismatch has no member to report and is left to the C
    /// prototype.
    fn check_args_func(
        &mut self,
        args: &[Arg],
        params: &[FuncParam],
        callee: Option<(&str, Span)>,
    ) {
        let types: Vec<Ty> = params.iter().map(|param| param.ty.clone()).collect();
        let modes: Vec<crate::ast::Mode> = params.iter().map(|param| param.mode).collect();
        self.check_args_impl(args, &types, &modes, false, callee);
    }

    /// Like [`Self::check_args`] for a direct call of a named function.
    ///
    /// Only a direct call lets the `static str` literal to `cstring` allowance
    /// apply: the emitter can see the named callee's parameter type there. A
    /// call through a function-typed value keeps `E10200`, because the emitter
    /// cannot recover the expected type for the argument.
    fn check_args_direct(
        &mut self,
        args: &[Arg],
        params: &[Ty],
        modes: &[crate::ast::Mode],
        callee: Option<(&str, Span)>,
    ) {
        self.check_args_impl(args, params, modes, true, callee);
    }

    /// Checks call arguments, optionally allowing a `cstring` literal argument.
    ///
    /// When a parameter is declared `take`, the argument moves its value to the
    /// callee. Moving a captured binding out of a lambda is `E10401`. A
    /// nullable writable lend is an ownership out-slot and takes a stricter
    /// exact-place rule (see [`Self::check_out_slot_argument`]). A written
    /// count that disagrees with the declared parameters reports once when the
    /// call names its callee, and the written arguments are still checked for
    /// their own errors.
    fn check_args_impl(
        &mut self,
        args: &[Arg],
        params: &[Ty],
        modes: &[crate::ast::Mode],
        allow_cstring_literal: bool,
        callee: Option<(&str, Span)>,
    ) {
        if args.len() == params.len() {
            for (index, (arg, param)) in args.iter().zip(params).enumerate() {
                let mode = modes
                    .get(index)
                    .copied()
                    .unwrap_or(crate::ast::Mode::None);
                if mode == crate::ast::Mode::Take {
                    self.reject_capture_move(&arg.value);
                    self.record_generic_capture_move(&arg.value);
                }
                if allow_cstring_literal && is_cstring_literal_argument(&arg.value, param) {
                    // A `static str` literal is a NUL-terminated C string, so
                    // it is accepted where a `cstring` is expected and emitted
                    // directly. A non-literal `static str` keeps E10200.
                    self.check_value(&arg.value, None);
                    continue;
                }
                if self.is_out_slot_param(param, mode) {
                    self.check_out_slot_argument(arg, param);
                    continue;
                }
                self.check_into(&arg.value, param);
            }
        } else {
            if let Some((name, span)) = callee {
                self.error(
                    "E10308",
                    format!(
                        "wrong number of arguments for '{name}': expected {}, found {}",
                        params.len(),
                        args.len()
                    ),
                    "pass one argument per parameter",
                    span,
                );
            }
            for arg in args {
                self.check_value(&arg.value, None);
            }
        }
    }

    /// The resolved parameter types and modes of an `ffi` declaration,
    /// validated once.
    ///
    /// The declared types resolve at the declaration's first call site and are
    /// cached for the rest of the module, so a declaration whose parameter
    /// type is rejected reports once rather than once per call. The mode
    /// travels with the type because a `borrow` function parameter is the
    /// synchronous C-hook contract.
    fn ffi_parameter_types(
        &mut self,
        name: &str,
        signature: &crate::ast::FuncSignature,
    ) -> Vec<(Ty, crate::ast::Mode)> {
        if let Some(params) = self.ffi_params.get(name) {
            return params.clone();
        }
        let mut params = Vec::with_capacity(signature.params.len());
        for param in &signature.params {
            let resolved = self.resolve_type(&param.ty);
            params.push((resolved, param.mode));
        }
        self.ffi_params.insert(name.to_owned(), params.clone());
        params
    }

    /// The resolved result type of an `ffi` declaration, validated once.
    ///
    /// The declared return type resolves at the declaration's first call site
    /// and is cached for the rest of the module, exactly as the parameter
    /// types are, so the ordinary conversion judgments see the result's real
    /// type instead of an unresolved value. A declaration written `: void`
    /// keeps the void type. A view or slice return types as its ABI pointer
    /// (see [`Self::ffi_abi_result`]).
    fn ffi_result_type(&mut self, name: &str, signature: &crate::ast::FuncSignature) -> Ty {
        if let Some(result) = self.ffi_results.get(name) {
            return result.clone();
        }
        let declared = self.resolve_type(&signature.returns);
        // The declaration pass already reports an invalid nullable return.
        // Keeping the invalid shape here would let a use report the dependent
        // possible-null rule instead, so the call types as unresolved.
        if declared.nullable && !crate::borrow::is_nullable_capable(&declared) {
            self.ffi_results.insert(name.to_owned(), Ty::unknown());
            return Ty::unknown();
        }
        let result = Self::ffi_abi_result(declared);
        self.ffi_results.insert(name.to_owned(), result.clone());
        result
    }

    /// The checked type of an `ffi` call's result, in ABI terms.
    ///
    /// A view or slice crosses the FFI boundary as its pointer only: the ABI
    /// spells it `APTR`, and a view's length is a separate parameter, so the
    /// result types as the pointer to its first element. Every use then judges
    /// through the frozen conversion rules instead of reading a struct the ABI
    /// never returns.
    pub(super) fn ffi_abi_result(ty: Ty) -> Ty {
        match &ty.kind {
            TyKind::Str | TyKind::StaticStr => Ty {
                kind: TyKind::Raw(Box::new(Ty::scalar(crate::ast::ScalarType::U8))),
                nullable: ty.nullable,
            },
            TyKind::Array { .. } => Ty {
                kind: TyKind::Raw(Box::new(ty.iterated_element())),
                nullable: ty.nullable,
            },
            _ => ty,
        }
    }

    /// Checks an `ffi` call's written arguments.
    ///
    /// An `ffi` declaration carries no checked signature in the type table, so
    /// the ordinary call path checks nothing. The written count is judged
    /// against the declared parameters like every other direct call, and the
    /// `cstring` parameter keeps the conversion table's judgment: only the
    /// `static str` literal converts implicitly, so a view, slice, handle, or
    /// `raw` argument reports the conversion the ordinary call path reports
    /// (its bytes carry no zero-terminator guarantee). An unresolved argument
    /// (error recovery or an extern result) keeps the accepted path, and every
    /// argument is still walked for its own errors.
    ///
    /// A function-type parameter is the C callback contract. `call_site` is
    /// the call expression's address, threaded from the expression checker so
    /// an accepted synchronous hook can be keyed for the lowering stage;
    /// `returns` is the call's checked result type, which the generated
    /// wrapper forwards.
    fn check_ffi_arguments(
        &mut self,
        name: &str,
        params: &[(Ty, crate::ast::Mode)],
        args: &[Arg],
        span: Span,
        call_site: usize,
        returns: &Ty,
    ) {
        if args.len() != params.len() {
            self.error(
                "E10308",
                format!(
                    "wrong number of arguments for '{name}': expected {}, found {}",
                    params.len(),
                    args.len()
                ),
                "pass one argument per parameter",
                span,
            );
        }
        let mut callbacks = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            let ty = self.check_value(&arg.value, None);
            let Some((param, mode)) = params.get(index) else {
                continue;
            };
            if let TyKind::Func(expected) = &param.kind {
                let expected = expected.clone();
                if let Some(closure) =
                    self.check_hook_argument(&ty, &expected, *mode, arg)
                {
                    // A borrowing capture stores the binding's address in the
                    // environment. If another argument of the same call moves
                    // that binding, the callee may release it before invoking
                    // the callback, so the environment would dangle.
                    self.reject_hook_capture_of_moved_binding(
                        &closure,
                        index,
                        params,
                        args,
                        arg.value.span,
                    );
                    callbacks.push(super::HookCall { arg: index, closure });
                }
                continue;
            }
            if matches!(param.kind, TyKind::CString)
                && !is_cstring_literal_argument(&arg.value, param)
                && !matches!(ty.kind, TyKind::CString)
            {
                // A member read on an opaque base has no type until the
                // instance is known: the read is recorded and judged at the
                // instance. An FFI argument is not a destination slot, so the
                // view pointer member reports the frozen conversion there too.
                if ty.is_unknown() {
                    if let Some(name) = super::expr::member_read_name(&arg.value) {
                        if let Some(function) = self.function_name.clone() {
                            self.opaque_member_uses.push(super::OpaqueMemberUse {
                                function,
                                name,
                                span: arg.value.span,
                                expected: param.clone(),
                                sanctioned: false,
                                base: super::expr::plain_binding_root(&arg.value),
                            });
                            continue;
                        }
                    }
                }
                // A parameter-typed argument resolves at its instance: the
                // conversion is recorded and judged there, exactly as the
                // ordinary call path judges its instances.
                self.record_deferred_conversion(&ty, param, arg.value.span);
                if !ty.is_unknown() && !crate::types::contains_param(&ty) {
                    self.error(
                        "E10200",
                        format!(
                            "cannot convert '{}' to 'cstring' implicitly",
                            ty.display()
                        ),
                        "pass `toCString(value)`",
                        arg.value.span,
                    );
                }
            }
        }
        if callbacks.is_empty() {
            return;
        }
        let site = self
            .hook_calls
            .entry(call_site)
            .or_insert_with(|| super::HookCallSite {
                callee: name.to_owned(),
                params: params.iter().map(|(ty, _)| ty.clone()).collect(),
                modes: params.iter().map(|(_, mode)| *mode).collect(),
                returns: returns.clone(),
                callbacks: Vec::new(),
            });
        for callback in callbacks {
            if !site
                .callbacks
                .iter()
                .any(|existing| existing.arg == callback.arg)
            {
                site.callbacks.push(callback);
            }
        }
        site.callbacks.sort_by_key(|callback| callback.arg);
    }

    /// Checks one argument offered to an `ffi` function-type parameter.
    ///
    /// A written function or capture-less lambda matches by parameter types
    /// and return type; the callback's own parameter modes are not part of the
    /// contract, exactly as a callable-bound generic parameter judges them. A
    /// capturing closure is a two-word value, never a C function pointer: it
    /// crosses only a synchronous `borrow` hook, where the lowering emits a
    /// trampoline and a caller-frame environment, and only outside a generic
    /// body, whose instances would need separate wrappers. The accepted
    /// signature set is the one whose lifted C spelling is identical to the
    /// `ffi` spelling; anything else is E10210 because the emitted function
    /// pointer would not convert.
    ///
    /// Returns the accepted capturing closure so the caller can record the
    /// hook site; a capture-less function and every rejected form return
    /// `None`.
    fn check_hook_argument(
        &mut self,
        ty: &Ty,
        expected: &FuncTy,
        mode: crate::ast::Mode,
        arg: &Arg,
    ) -> Option<crate::types::ClosureTy> {
        let span = arg.value.span;
        match &ty.kind {
            TyKind::Func(found) => {
                if !Self::hook_signature_matches(expected, found) {
                    self.report_hook_signature_mismatch(expected, ty, span);
                    return None;
                }
                // The value's own C spelling is what the hook receives. A
                // named function written with a borrowed non-duplicable
                // parameter spells `T *` while the hook passes the value by
                // `T`, so the found signature needs the same judgment.
                if let Some(reason) = self.hook_abi_mismatch(found) {
                    self.report_hook_abi_mismatch(&reason, span);
                    return None;
                }
                if let Some(reason) = self.hook_abi_mismatch(expected) {
                    self.report_hook_abi_mismatch(&reason, span);
                }
                None
            }
            TyKind::Closure(found) => {
                // A capturing closure needs a synchronous `borrow` contract
                // and a generated trampoline. Every other mode has no bound
                // lifetime, so the closure can never become a C pointer there.
                if mode != crate::ast::Mode::Borrow {
                    self.report_capturing_hook(span);
                    return None;
                }
                if !Self::hook_closure_signature_matches(expected, &found.params, &found.returns) {
                    self.report_hook_signature_mismatch(expected, ty, span);
                    return None;
                }
                if !self.type_params.is_empty() {
                    self.error(
                        "E10400",
                        "capturing lambda cannot be used as 'func(...)'",
                        "move the callback into a non-generic function; a generic body needs one trampoline per instance",
                        span,
                    );
                    return None;
                }
                if let Some(reason) = self.hook_abi_mismatch(expected) {
                    self.report_hook_abi_mismatch(&reason, span);
                    return None;
                }
                Some(found.clone())
            }
            _ => {
                // A member read on a bare type parameter has no type until
                // the instance is known. Record the use so the instance audit
                // judges the member's function type against the hook; a
                // concrete non-callable still reports here.
                if ty.is_unknown() {
                    if let Some(name) = super::expr::member_read_name(&arg.value) {
                        if let Some(function) = self.function_name.clone() {
                            self.opaque_member_uses.push(super::OpaqueMemberUse {
                                function,
                                name,
                                span,
                                expected: Ty::func(expected.clone()),
                                sanctioned: false,
                                base: super::expr::plain_binding_root(&arg.value),
                            });
                            return None;
                        }
                    }
                }
                self.error(
                    "E10200",
                    format!(
                        "cannot convert '{}' to '{}' implicitly",
                        ty.display(),
                        Ty::func(expected.clone()).display()
                    ),
                    "pass a matching function or capture-less lambda",
                    span,
                );
                None
            }
        }
    }

    /// True when a written function's signature matches the hook.
    ///
    /// The callback's own parameter modes are deliberately ignored: a
    /// `borrow` in a `func(...)` type is a calling convention detail, not part
    /// of the exact C signature the hook declares.
    fn hook_signature_matches(expected: &FuncTy, found: &FuncTy) -> bool {
        expected.params.len() == found.params.len()
            && expected
                .params
                .iter()
                .zip(&found.params)
                .all(|(expected, found)| expected.ty == found.ty)
            && *expected.returns == *found.returns
    }

    /// True when a capturing lambda's parameter types and return match.
    ///
    /// A closure's type carries no parameter modes, so only the types and the
    /// return are comparable.
    fn hook_closure_signature_matches(expected: &FuncTy, params: &[Ty], returns: &Ty) -> bool {
        expected.params.len() == params.len()
            && expected
                .params
                .iter()
                .zip(params)
                .all(|(expected, found)| expected.ty == *found)
            && *expected.returns == *returns
    }

    /// The first reason a declared callback signature is not ABI-identical.
    ///
    /// The lifted lambda spells its parameters with the language C types
    /// (`ClStr`, `ClSlice_T`, row declarators, an environment pointer), while
    /// the `ffi` prototype spells them with the ABI table (`APTR` and
    /// by-value structs). Passing one where the other is expected would emit
    /// an incompatible function pointer, so only the intersection is accepted.
    fn hook_abi_mismatch(&self, signature: &FuncTy) -> Option<String> {
        for (index, param) in signature.params.iter().enumerate() {
            if let Some(reason) = self.hook_parameter_abi_mismatch(&param.ty, param.mode) {
                return Some(format!("callback parameter {} is {reason}", index + 1));
            }
        }
        self.hook_return_abi_mismatch(&signature.returns)
            .map(|reason| format!("callback return is {reason}"))
    }

    /// The ABI reason one callback parameter cannot cross unchanged.
    fn hook_parameter_abi_mismatch(
        &self,
        ty: &Ty,
        mode: crate::ast::Mode,
    ) -> Option<String> {
        // A borrowed nullable value or a borrowed move-only struct lowers to a
        // pointer in the language ABI, while an `ffi` parameter ignores its
        // mode and stays by value.
        if mode == crate::ast::Mode::Borrow
            && (ty.nullable
                || (matches!(ty.kind, TyKind::Struct(..)) && !self.table.duplicable(ty)))
        {
            return Some("a borrowed value ('T *' vs 'T')".to_owned());
        }
        match &ty.kind {
            TyKind::Scalar(_)
            | TyKind::Enum(_)
            | TyKind::Raw(_)
            | TyKind::CString
            | TyKind::Handle(_) => None,
            TyKind::Struct(_, args) if args.is_empty() => None,
            TyKind::Struct(_, _) => Some("a generic struct ('ClWrap_i32' vs 'ClWrap')".to_owned()),
            TyKind::Str | TyKind::StaticStr => {
                Some("a view ('ClStr' vs 'APTR')".to_owned())
            }
            TyKind::Array { extents, .. } if extents.iter().any(Option::is_none) => {
                Some("a view ('ClSlice' vs 'APTR')".to_owned())
            }
            TyKind::Array { .. } => Some("a fixed array ('[N]' vs 'APTR')".to_owned()),
            TyKind::Func(signature) => self
                .hook_abi_mismatch(signature)
                .map(|_| "a non-ABI-identical function pointer".to_owned()),
            TyKind::Box(_) => Some("a Box".to_owned()),
            TyKind::Void => Some("void".to_owned()),
            TyKind::Closure(_) => Some("a closure".to_owned()),
            TyKind::Param(_) | TyKind::Unknown => Some("an unresolved type".to_owned()),
        }
    }

    /// The ABI reason a callback return cannot cross unchanged.
    fn hook_return_abi_mismatch(&self, ty: &Ty) -> Option<String> {
        match &ty.kind {
            TyKind::Void
            | TyKind::Scalar(_)
            | TyKind::Enum(_)
            | TyKind::Raw(_)
            | TyKind::CString
            | TyKind::Handle(_) => None,
            TyKind::Struct(_, args) if args.is_empty() => None,
            TyKind::Struct(_, _) => Some("a generic struct".to_owned()),
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some) => {
                Some("a fixed array".to_owned())
            }
            TyKind::Array { .. } => Some("a view".to_owned()),
            TyKind::Str | TyKind::StaticStr => Some("a view".to_owned()),
            // The `ffi` prototype itself cannot spell a function-pointer
            // return without a named declarator, so a callback that returns
            // one is rejected until the prototype surface supports it.
            TyKind::Func(_) => {
                Some("a function-pointer return ('ret (*)(...) name(...)' is not a C declaration)".to_owned())
            }
            TyKind::Box(_) => Some("a Box".to_owned()),
            TyKind::Closure(_) => Some("a closure".to_owned()),
            TyKind::Param(_) | TyKind::Unknown => Some("an unresolved type".to_owned()),
        }
    }

    /// Rejects a capturing hook whose borrowed capture is moved by the call.
    ///
    /// The environment stores the captured binding's address for a read-only
    /// or writable borrow. A `take` argument in the same call transfers the
    /// binding to the callee, which may release it before it invokes the
    /// callback; the environment pointer would then dangle. A copy capture is
    /// safe because the value is copied when the closure is built.
    fn reject_hook_capture_of_moved_binding(
        &mut self,
        closure: &crate::types::ClosureTy,
        callback_index: usize,
        params: &[(Ty, crate::ast::Mode)],
        args: &[Arg],
        span: Span,
    ) {
        let captures = self
            .closures
            .iter()
            .find(|site| site.id == closure.id)
            .map(|site| site.captures.clone())
            .unwrap_or_default();
        if !captures.iter().any(|capture| {
            matches!(
                capture.mode,
                crate::types::CaptureMode::ReadOnlyBorrow
                    | crate::types::CaptureMode::WriteBorrow
            )
        }) {
            return;
        }
        for (index, (_, mode)) in params.iter().enumerate() {
            if index == callback_index || *mode != crate::ast::Mode::Take {
                continue;
            }
            let Some(arg) = args.get(index) else {
                continue;
            };
            if let Some(capture) = captures.iter().find(|capture| {
                matches!(
                    capture.mode,
                    crate::types::CaptureMode::ReadOnlyBorrow
                        | crate::types::CaptureMode::WriteBorrow
                ) && Self::expression_mentions_binding(&arg.value, &capture.name)
            }) {
                self.error(
                    "E10030",
                    format!("borrow of '{}' does not live long enough", capture.name),
                    "the same call moves the binding before the callback can read it",
                    span,
                );
            }
        }
    }

    /// True when an expression mentions the named binding anywhere.
    ///
    /// A `take` argument that forwards the binding through a call
    /// (`pass(w)`) still moves it before the callback can read the captured
    /// environment, so the whole argument expression is walked rather than
    /// only its outer place root.
    fn expression_mentions_binding(expr: &Expr, name: &str) -> bool {
        match &expr.kind {
            ExprKind::Name(found) => found == name,
            ExprKind::Qualified(qualified) => {
                qualified.segments.first().is_some_and(|segment| segment == name)
            }
            ExprKind::Index { base, index } => {
                Self::expression_mentions_binding(base, name)
                    || Self::expression_mentions_binding(index, name)
            }
            ExprKind::Member { base, .. } => Self::expression_mentions_binding(base, name),
            ExprKind::Unary { operand, .. } => Self::expression_mentions_binding(operand, name),
            ExprKind::Binary { left, right, .. } => {
                Self::expression_mentions_binding(left, name)
                    || Self::expression_mentions_binding(right, name)
            }
            ExprKind::Assign { target, value, .. } => {
                Self::expression_mentions_binding(target, name)
                    || Self::expression_mentions_binding(value, name)
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                Self::expression_mentions_binding(condition, name)
                    || Self::expression_mentions_binding(then_value, name)
                    || Self::expression_mentions_binding(else_value, name)
            }
            ExprKind::Cast { value, .. }
            | ExprKind::RawCast { value, .. }
            | ExprKind::Unsafe(value) => Self::expression_mentions_binding(value, name),
            ExprKind::StructLiteral { fields, .. } => fields
                .iter()
                .any(|field| Self::expression_mentions_binding(&field.value, name)),
            ExprKind::ArrayLiteral { elements, fill } => {
                elements
                    .iter()
                    .any(|element| Self::expression_mentions_binding(element, name))
                    || fill
                        .as_deref()
                        .is_some_and(|fill| Self::expression_mentions_binding(fill, name))
            }
            ExprKind::Template(pieces) => pieces.iter().any(|piece| match piece {
                crate::ast::TemplatePiece::Placeholder(inner) => {
                    Self::expression_mentions_binding(inner, name)
                }
                crate::ast::TemplatePiece::Literal(_) => false,
            }),
            ExprKind::Call { callee, args, .. } => {
                Self::expression_mentions_binding(callee, name)
                    || args
                        .iter()
                        .any(|arg| Self::expression_mentions_binding(&arg.value, name))
            }
            ExprKind::IntrinsicCall { args, .. } => args.iter().any(|arg| match arg {
                crate::ast::IntrinsicArg::Expr(inner) => {
                    Self::expression_mentions_binding(inner, name)
                }
                crate::ast::IntrinsicArg::Type(_) => false,
            }),
            _ => false,
        }
    }

    /// Reports a capturing closure offered where a C function pointer is due.
    fn report_capturing_hook(&mut self, span: Span) {
        self.error(
            "E10400",
            "capturing lambda cannot be used as 'func(...)'",
            "use a capture-less lambda, a `borrow` C hook, or a generic callable bound",
            span,
        );
    }

    /// Reports a callback whose written signature differs from the hook.
    fn report_hook_signature_mismatch(&mut self, expected: &FuncTy, found: &Ty, span: Span) {
        self.error(
            "E10210",
            format!(
                "function type mismatch: expected '{}', found '{}'",
                Ty::func(expected.clone()).display(),
                found.display()
            ),
            "match the exact signature and the ABI-identical callback set",
            span,
        );
    }

    /// Reports a callback whose C spelling differs from the `ffi` spelling.
    fn report_hook_abi_mismatch(&mut self, reason: &str, span: Span) {
        self.error(
            "E10210",
            format!("function type mismatch: {reason}"),
            "use scalar, enum, raw, cstring, handle, non-generic struct, or function-pointer callback types",
            span,
        );
    }

    /// True when a parameter is a nullable writable lend: an ownership
    /// out-slot the callee writes and releases through.
    ///
    /// A parameter type that still mentions a type parameter is left to the
    /// ordinary conversion check (and the instantiation audit) because only an
    /// instantiation knows the concrete slot type.
    fn is_out_slot_param(&self, param: &Ty, mode: crate::ast::Mode) -> bool {
        mode == crate::ast::Mode::Borrow
            && param.nullable
            && !crate::types::contains_param(param)
            && !param.is_unknown()
    }

    /// Checks an argument passed to a nullable writable lend.
    ///
    /// The out-slot lowers to a pointer into the caller's storage, so the
    /// argument must be a place whose type already is the slot type exactly: a
    /// conversion or a nullability widening would produce a temporary whose
    /// address could not carry the callee's write back. Anything else is
    /// rejected with `E10200`, the implicit-conversion diagnostic.
    fn check_out_slot_argument(&mut self, arg: &Arg, param: &Ty) {
        let ty = self.check_value(&arg.value, Some(param));
        self.check_out_slot_place(arg, &ty, param);
    }

    /// Rejects an out-slot argument that is not an exact-typed place.
    ///
    /// The caller supplies the already-computed argument type so the generic
    /// and direct routes share one rule.
    fn check_out_slot_place(&mut self, arg: &Arg, ty: &Ty, param: &Ty) {
        if ty == param && self.is_place(&arg.value) {
            return;
        }
        self.report_out_slot_conversion(ty, param, arg.span);
    }

    /// Rejects a dot-call receiver that is not an exact-typed place.
    ///
    /// A dotted receiver is not a written argument, so its place-ness is
    /// checked on the receiver expression when the caller has one; a qualified
    /// name is a place by construction.
    fn check_out_slot_receiver(
        &mut self,
        receiver: Option<&Expr>,
        ty: &Ty,
        param: &Ty,
        span: Span,
    ) {
        // A receiver that still mentions a type parameter cannot be judged
        // here: the instance audit applies the same rule to the substituted
        // receiver, exactly as it does for a declared argument.
        if crate::types::contains_param(ty) {
            let receiver_span = receiver.map(|expr| expr.span).unwrap_or(span);
            self.record_deferred_conversion(ty, param, receiver_span);
            return;
        }
        let place = receiver.map_or(true, |expr| self.is_place(expr));
        if place && ty == param {
            return;
        }
        self.report_out_slot_conversion(ty, param, span);
    }

    /// Reports the out-slot conversion diagnostic.
    fn report_out_slot_conversion(&mut self, ty: &Ty, param: &Ty, span: Span) {
        self.error(
            "E10200",
            format!(
                "cannot convert '{}' to '{}' implicitly",
                ty.display(),
                param.display()
            ),
            "pass a place of the exact slot type; the callee writes through it",
            span,
        );
    }

    /// Checks a call whose callee is a type name: a struct, enum, or handle
    /// cast.
    fn check_type_cast(&mut self, target: &Ty, arg_ty: Ty, span: Span) -> Ty {
        let allowed = match &target.kind {
            // A struct is an aggregate, not a scalar; a call in cast position
            // cannot build one. A box is also not constructible from a value.
            TyKind::Struct(..) | TyKind::Box(_) => false,
            TyKind::Enum(name) => {
                arg_ty.is_integer()
                    || arg_ty.is_unknown()
                    || arg_ty.is_param()
                    || self
                        .table
                        .enum_backing(name)
                        .is_some_and(|backing| implicit_conversion(&arg_ty, backing))
            }
            TyKind::Handle(_) => arg_ty.is_integer() || arg_ty.is_unknown() || arg_ty.is_param(),
            _ => true,
        };
        if !allowed {
            self.error(
                "E10201",
                format!("cannot cast '{}' to '{}'", arg_ty.display(), target.display()),
                "convert through a supported value or `raw` in `unsafe`",
                span,
            );
        }
        target.clone()
    }
}

/// The candidate a dot-call resolves to.
enum DotCall {
    /// A callable struct field; its signature has no receiver parameter.
    Field(FuncSig),
    /// A free function whose first parameter is the receiver.
    Function(FuncSig),
}

/// True when a type mentions one of the named type parameters.
///
/// The walk mirrors [`crate::types::contains_param`], but only the names the
/// call site left unbound count: a parameter the inference resolved to the
/// caller's own type argument still judges at the call and at its instance.
fn mentions_unbound(ty: &Ty, unbound: &BTreeSet<String>) -> bool {
    if unbound.is_empty() {
        return false;
    }
    match &ty.kind {
        TyKind::Param(name) => unbound.contains(name),
        TyKind::Array { element, .. } => mentions_unbound(element, unbound),
        TyKind::Raw(inner) => mentions_unbound(inner, unbound),
        TyKind::Box(inner) => mentions_unbound(inner, unbound),
        TyKind::Struct(_, args) => args.iter().any(|arg| mentions_unbound(arg, unbound)),
        TyKind::Func(signature) => {
            signature
                .params
                .iter()
                .any(|param| mentions_unbound(&param.ty, unbound))
                || mentions_unbound(&signature.returns, unbound)
        }
        TyKind::Closure(closure) => {
            closure
                .params
                .iter()
                .any(|param| mentions_unbound(param, unbound))
                || mentions_unbound(&closure.returns, unbound)
        }
        _ => false,
    }
}

/// The simple name of a callee, when it is one.
///
/// A dotted callee is a member access or a module path, which the call rules
/// handle separately; only a bare identifier names a module function.
fn call_name(callee: &Expr) -> Option<&str> {
    match &callee.kind {
        ExprKind::Name(name) => Some(name),
        _ => None,
    }
}

/// Infers a generic call's type arguments from its argument types.
///
/// Each declared parameter binds against its argument with
/// [`match_param`], so a nested shape (`T[]`, `Pair<T>`, `func(T)`) binds
/// through its parts. Returns the arguments in declaration order, or `None`
/// when a parameter stays unbound: the callers then treat the call as
/// uninstantiable rather than guessing a key.
pub(crate) fn infer_type_args(
    declared: &[String],
    param_types: &[Ty],
    arg_types: &[Ty],
) -> Option<Vec<Ty>> {
    let mut inferred: BTreeMap<String, Ty> = BTreeMap::new();
    for (param, arg) in param_types.iter().zip(arg_types) {
        match_param(param, arg, declared, &mut inferred);
    }
    let mut result = Vec::with_capacity(declared.len());
    for param in declared {
        result.push(inferred.get(param).cloned()?);
    }
    Some(result)
}

/// Binds type parameters by matching a declared parameter type against an
/// argument type.
///
/// Only the shapes that carry a type parameter are walked; a struct type does
/// not spell out its type arguments here, so a parameter nested in a struct is
/// not recovered. The first binding for a parameter wins.
pub(crate) fn match_param(
    param: &Ty,
    arg: &Ty,
    declared: &[String],
    inferred: &mut BTreeMap<String, Ty>,
) {
    match &param.kind {
        TyKind::Param(name) if declared.iter().any(|known| known == name) => {
            inferred.entry(name.clone()).or_insert_with(|| arg.clone());
        }
        TyKind::Struct(name, args) => {
            if let TyKind::Struct(arg_name, arg_args) = &arg.kind {
                if name == arg_name {
                    for (inner, arg_inner) in args.iter().zip(arg_args) {
                        match_param(inner, arg_inner, declared, inferred);
                    }
                }
            }
        }
        TyKind::Raw(inner) => {
            if let TyKind::Raw(arg_inner) = &arg.kind {
                match_param(inner, arg_inner, declared, inferred);
            }
        }
        TyKind::Box(inner) => {
            if let TyKind::Box(arg_inner) = &arg.kind {
                match_param(inner, arg_inner, declared, inferred);
            }
        }
        TyKind::Array { element, .. } => {
            if let TyKind::Array {
                element: arg_element,
                ..
            } = &arg.kind
            {
                match_param(element, arg_element, declared, inferred);
            }
        }
        TyKind::Func(signature) => {
            if let TyKind::Func(arg_signature) = &arg.kind {
                for (inner, arg_inner) in signature.params.iter().zip(&arg_signature.params) {
                    match_param(&inner.ty, &arg_inner.ty, declared, inferred);
                }
                match_param(&signature.returns, &arg_signature.returns, declared, inferred);
            }
        }
        _ => {}
    }
}

/// True when a string literal is passed where a `cstring` is expected.
///
/// A `static str` literal is a NUL-terminated C string, so it is a valid
/// `cstring` at a call argument; the emitter passes the literal directly. A
/// literal with an embedded NUL byte cannot be a faithful `cstring` (the NUL
/// would truncate it) and a non-literal `static str` is not necessarily
/// NUL-terminated, so both keep `E10200`.
fn is_cstring_literal_argument(expr: &Expr, param: &Ty) -> bool {
    if !matches!(param.kind, TyKind::CString) {
        return false;
    }
    matches!(&expr.kind, ExprKind::Str(bytes) if !bytes.contains(&0))
}

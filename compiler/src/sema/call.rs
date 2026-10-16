//! Call checking: type casts, function calls, dot-calls, and generics.
//!
//! A call has four shapes. A callee that names a type is a cast. A callee that
//! is a member access is a dot-call, which prefers a callable field and then a
//! free function with the receiver as its first argument. A bare callee that
//! names a module function is a direct call, and a generic function call also
//! checks its type arguments. Anything else stays opaque, so an unmodelled
//! external function does not cascade into false errors.

use std::collections::BTreeMap;

use crate::ast::{Arg, Expr, ExprKind, Type};
use crate::span::Span;
use crate::types::{implicit_conversion, view_type, FuncParam, FuncSig, Ty, TyKind};

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
    ) -> Ty {
        if let Some(target) = self.callee_type(callee) {
            for ty in type_args {
                self.resolve_type(ty);
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
            return self.check_dot_call(Some(base), base_ty, name, args, callee.span);
        }
        if let ExprKind::Qualified(name) = &callee.kind {
            // A dotted callee whose head is a value in scope is a dot-call,
            // exactly like the `Member` shape the parser builds elsewhere.
            if name.segments.len() >= 2 && self.name_is_value(&name.segments[0]) {
                let receiver = self.qualified_receiver_type(name);
                if let Some(member) = name.segments.last() {
                    let member = member.clone();
                    return self.check_dot_call(None, receiver, &member, args, name.span);
                }
            }
        }
        if let Some(name) = call_name(callee) {
            if !self.is_local(name) {
                if let Some(signature) = self.free_function(name) {
                    if !signature.type_params.is_empty() {
                        return self.check_generic_call(name, &signature, type_args, args, span);
                    }
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
                        self.check_ffi_arguments(name, &params, args, span);
                        let _ = self.check_value(callee, None);
                        return self.ffi_result_type(name, signature);
                    }
                }
                // The compiler-known `text(v)` formatter is not declared in
                // Clorraine; the compiler lowers it to a `String`-returning
                // shim conversion. Registering that return type here lets the
                // `String`-to-`str` widening apply when it is passed to a view.
                if name == "text" {
                    for ty in type_args {
                        self.resolve_type(ty);
                    }
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
                    return self.check_view_builder(type_args, args);
                }
            }
        }
        let callee_ty = self.check_value(callee, None);
        match &callee_ty.kind {
            TyKind::Func(signature) => {
                let signature = signature.clone();
                self.check_args_func(args, &signature.params);
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
                self.check_args_func(args, &params);
                return (*closure.returns).clone();
            }
            // A callable-bound type parameter is called through its bound, so
            // the call has the bound's parameter types and return type even
            // though the parameter itself is opaque.
            TyKind::Param(name) if self.callable_bound_params.iter().any(|p| p == name) => {
                if let Some(bound) = self.current_callable_bound(name) {
                    self.check_args_func(args, &bound.params);
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
            _ => {}
        }
        for arg in args {
            self.check_value(&arg.value, None);
        }
        Ty::unknown()
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
    fn check_view_builder(&mut self, type_args: &[Type], args: &[Arg]) -> Ty {
        let mut explicit: Option<Ty> = None;
        for ty in type_args {
            let resolved = self.resolve_type(ty);
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
        let arg_types: Vec<(Ty, Span)> = args
            .iter()
            .map(|arg| (self.check_value(&arg.value, None), arg.value.span))
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

        let mut bindings: BTreeMap<String, Ty> = BTreeMap::new();
        if type_args.is_empty() {
            for ((ty, _), param) in arg_types.iter().zip(&signature.params) {
                match_param(param, ty, &declared, &mut bindings);
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
        } else {
            let mut resolved = Vec::with_capacity(type_args.len());
            for ty in type_args {
                resolved.push(self.resolve_type(ty));
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
            if let (Some((ty, arg_span)), Some(param)) =
                (arg_types.get(index), signature.params.get(index))
            {
                let expected = crate::types::substitute(param, &bindings);
                if self.is_out_slot_param(&expected, mode) {
                    // The instance makes the parameter a nullable out-slot, so
                    // the same exact-place rule a direct call enforces applies
                    // to the inferred type arguments.
                    self.check_out_slot_place(arg, ty, &expected);
                } else {
                    self.check_conversion(ty, &expected, *arg_span);
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
        args: &[Arg],
        span: Span,
    ) -> Ty {
        match self.dot_call(&receiver, name) {
            Some(DotCall::Field(signature)) => {
                // A callable field carries its own parameter modes, so a
                // `take` argument still moves and a call-site marker still
                // matches, exactly as on a named call.
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
                if let Some((receiver_param, rest)) = signature.params.split_first() {
                    // The receiver is a written argument at index zero, so a
                    // nullable borrow receiver takes the same exact-place rule
                    // as a direct call's argument.
                    let receiver_mode = signature
                        .modes
                        .first()
                        .copied()
                        .unwrap_or(crate::ast::Mode::None);
                    if self.is_out_slot_param(receiver_param, receiver_mode) {
                        self.check_out_slot_receiver(
                            receiver_expr,
                            &receiver,
                            receiver_param,
                            span,
                        );
                    } else {
                        // The receiver is a written argument at index zero, so
                        // its deferred conversion is recorded exactly like an
                        // argument's and judged at the instance.
                        self.record_deferred_conversion(
                            &receiver,
                            receiver_param,
                            receiver_expr.map(|expr| expr.span).unwrap_or(span),
                        );
                        self.check_conversion(&receiver, receiver_param, span);
                    }
                    // The receiver is not a written argument, so the remaining
                    // modes line up with the written arguments; a nullable
                    // borrow parameter keeps its exact-place rule on this route.
                    let rest_modes = signature.modes.get(1..).unwrap_or(&[]);
                    self.check_args_impl(args, rest, rest_modes, false, Some((name, span)));
                } else {
                    self.check_args(args, &signature.params, Some((name, span)));
                }
                signature.returns
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
                    }
                }
                // A receiver whose surface holds no field of that name and
                // no matching function candidate has an unknown member: the
                // call may not reach the emitter, where it would become an
                // implicit call. A field of the name that is not callable
                // keeps its own path, and an unresolved receiver (error
                // recovery) stays silent so one error is not reported twice.
                if self.field_type(&receiver, name).is_none() && !receiver.is_unknown() {
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

    /// Resolves the candidate a dot-call names.
    ///
    /// A struct field of that name wins outright: a callable field is a
    /// `Field` candidate, and a non-callable field stops the search. Without a
    /// field, a visible free function is a `Function` candidate, and the
    /// receiver-taking `len` intrinsic stands in as one so `xs.len()` works.
    fn dot_call(&self, base: &Ty, name: &str) -> Option<DotCall> {
        if let TyKind::Struct(struct_name, _) = &base.kind {
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
            // A candidate must accept the receiver as its first parameter, so
            // a function with no parameters is not a candidate; the intrinsic
            // lookup still runs, because `len` is offered for the same shape.
            if !signature.params.is_empty() {
                return Some(DotCall::Function(signature));
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
    fn check_args_func(&mut self, args: &[Arg], params: &[FuncParam]) {
        let types: Vec<Ty> = params.iter().map(|param| param.ty.clone()).collect();
        let modes: Vec<crate::ast::Mode> = params.iter().map(|param| param.mode).collect();
        self.check_args_impl(args, &types, &modes, false, None);
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

    /// The resolved parameter types of an `ffi` declaration, validated once.
    ///
    /// The declared types resolve at the declaration's first call site and are
    /// cached for the rest of the module, so a declaration whose parameter
    /// type is rejected reports once rather than once per call.
    fn ffi_parameter_types(
        &mut self,
        name: &str,
        signature: &crate::ast::FuncSignature,
    ) -> Vec<Ty> {
        if let Some(params) = self.ffi_params.get(name) {
            return params.clone();
        }
        let mut params = Vec::with_capacity(signature.params.len());
        for param in &signature.params {
            params.push(self.resolve_type(&param.ty));
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
        let result = Self::ffi_abi_result(self.resolve_type(&signature.returns));
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
    fn ffi_abi_result(ty: Ty) -> Ty {
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
    fn check_ffi_arguments(&mut self, name: &str, params: &[Ty], args: &[Arg], span: Span) {
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
        for (index, arg) in args.iter().enumerate() {
            let ty = self.check_value(&arg.value, None);
            let Some(param) = params.get(index) else {
                continue;
            };
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
            // cannot build one.
            TyKind::Struct(..) => false,
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

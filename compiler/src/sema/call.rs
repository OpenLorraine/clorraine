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
use crate::types::{implicit_conversion, FuncSig, Ty, TyKind};

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
            return self.check_dot_call(base_ty, name, args, base.span);
        }
        if let ExprKind::Qualified(name) = &callee.kind {
            // A dotted callee whose head is a value in scope is a dot-call,
            // exactly like the `Member` shape the parser builds elsewhere.
            if name.segments.len() >= 2 && self.name_is_value(&name.segments[0]) {
                let receiver = self.qualified_receiver_type(name);
                if let Some(member) = name.segments.last() {
                    let member = member.clone();
                    return self.check_dot_call(receiver, &member, args, name.span);
                }
            }
        }
        if let Some(name) = call_name(callee) {
            if !self.is_local(name) {
                if let Some(signature) = self.free_function(name) {
                    if !signature.type_params.is_empty() {
                        return self.check_generic_call(name, &signature, type_args, args, span);
                    }
                    self.check_args_direct(args, &signature.params, &signature.modes);
                    return signature.returns;
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
            }
        }
        let callee_ty = self.check_value(callee, None);
        if let TyKind::Func(signature) = &callee_ty.kind {
            let signature = signature.clone();
            self.check_args(args, &signature.params);
            return (*signature.returns).clone();
        }
        for arg in args {
            self.check_value(&arg.value, None);
        }
        Ty::unknown()
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

        let mut bindings: BTreeMap<String, Ty> = BTreeMap::new();
        if type_args.is_empty() {
            for ((ty, _), param) in arg_types.iter().zip(&signature.params) {
                match_param(param, ty, &declared, &mut bindings);
            }
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
            if signature.modes.get(index).copied() == Some(crate::ast::Mode::Take) {
                self.reject_capture_move(&arg.value);
            }
            if let (Some((ty, arg_span)), Some(param)) =
                (arg_types.get(index), signature.params.get(index))
            {
                let expected = crate::types::substitute(param, &bindings);
                self.check_conversion(ty, &expected, *arg_span);
            }
        }
        crate::types::substitute(&signature.returns, &bindings)
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
                    found.params == want.params && found.returns == want.returns
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
    fn check_dot_call(&mut self, receiver: Ty, name: &str, args: &[Arg], span: Span) -> Ty {
        match self.dot_call(&receiver, name) {
            Some(DotCall::Field(signature)) => {
                self.check_args(args, &signature.params);
                signature.returns
            }
            Some(DotCall::Function(signature)) => {
                if let Some((receiver_param, rest)) = signature.params.split_first() {
                    self.check_conversion(&receiver, receiver_param, span);
                    self.check_args(args, rest);
                } else {
                    self.check_args(args, &signature.params);
                }
                signature.returns
            }
            None => {
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
                        params: signature.params.clone(),
                        modes: Vec::new(),
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
            return Some(DotCall::Function(signature));
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
    /// An arity mismatch is left alone: no diagnostic in this phase owns it.
    fn check_args(&mut self, args: &[Arg], params: &[Ty]) {
        self.check_args_impl(args, params, &[], false);
    }

    /// Like [`Self::check_args`] for a direct call of a named function.
    ///
    /// Only a direct call lets the `static str` literal to `cstring` allowance
    /// apply: the emitter can see the named callee's parameter type there. A
    /// call through a function-typed value keeps `E10200`, because the emitter
    /// cannot recover the expected type for the argument.
    fn check_args_direct(&mut self, args: &[Arg], params: &[Ty], modes: &[crate::ast::Mode]) {
        self.check_args_impl(args, params, modes, true);
    }

    /// Checks call arguments, optionally allowing a `cstring` literal argument.
    ///
    /// When a parameter is declared `take`, the argument moves its value to the
    /// callee. Moving a captured binding out of a lambda is `E10401`.
    fn check_args_impl(
        &mut self,
        args: &[Arg],
        params: &[Ty],
        modes: &[crate::ast::Mode],
        allow_cstring_literal: bool,
    ) {
        if args.len() == params.len() {
            for (index, (arg, param)) in args.iter().zip(params).enumerate() {
                if modes.get(index).copied() == Some(crate::ast::Mode::Take) {
                    self.reject_capture_move(&arg.value);
                }
                if allow_cstring_literal && is_cstring_literal_argument(&arg.value, param) {
                    // A `static str` literal is a NUL-terminated C string, so
                    // it is accepted where a `cstring` is expected and emitted
                    // directly. A non-literal `static str` keeps E10200.
                    self.check_value(&arg.value, None);
                    continue;
                }
                self.check_into(&arg.value, param);
            }
        } else {
            for arg in args {
                self.check_value(&arg.value, None);
            }
        }
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
                    match_param(inner, arg_inner, declared, inferred);
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

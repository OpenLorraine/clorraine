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
                    self.check_args(args, &signature.params);
                    return signature.returns;
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
        let declared = signature.type_params.clone();
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

        for ((ty, arg_span), param) in arg_types.iter().zip(&signature.params) {
            let expected = crate::types::substitute(param, &bindings);
            self.check_conversion(ty, &expected, *arg_span);
        }
        crate::types::substitute(&signature.returns, &bindings)
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
            if let Some(field_ty) = self.table.struct_field(struct_name, name) {
                return match &field_ty.kind {
                    TyKind::Func(signature) => Some(DotCall::Field(FuncSig {
                        params: signature.params.clone(),
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
        if args.len() == params.len() {
            for (arg, param) in args.iter().zip(params) {
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

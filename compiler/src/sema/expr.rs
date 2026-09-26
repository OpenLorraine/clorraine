//! Expression checking.
//!
//! Types flow upward: an expression is checked once and returns the type it
//! produces. The expected type is passed down only where it changes the answer,
//! which is a literal taking its context type. A `void` expression used where a
//! value is needed is reported here as the assignment-result error.
//!
//! The operator, assignment, and conditional rules live in the sibling
//! `ops` module; calls and generics live in `call`; this one owns names,
//! literals, aggregates, intrinsics, and templates.

use std::collections::BTreeSet;

use crate::ast::{
    Expr, ExprKind, FieldInit, IntrinsicArg, LambdaBody, QualifiedName, ScalarType, TemplatePiece,
    Type,
};
use crate::profile::Profile;
use crate::span::Span;
use crate::types::{
    implicit_conversion, int_fits, scalar_cast_allowed, CaptureInfo, CaptureMode, ClosureSite,
    ClosureTy, FuncTy, Ty, TyKind,
};

use super::Checker;

impl Checker<'_, '_> {
    /// Checks an expression, records its type, and returns it.
    ///
    /// The recorded `(node, type)` pair is what the ownership stage reads, so
    /// the two stages cannot disagree about the type of any expression.
    pub(super) fn check_expr(&mut self, expr: &Expr, expected: Option<&Ty>) -> Ty {
        let ty = self.check_expr_inner(expr, expected);
        self.recorded.push((expr as *const Expr as usize, ty.clone()));
        ty
    }

    /// Checks an expression and returns its type.
    ///
    /// `expected` carries the context type where it matters to a literal.
    fn check_expr_inner(&mut self, expr: &Expr, expected: Option<&Ty>) -> Ty {
        match &expr.kind {
            ExprKind::Int(value) => self.check_int(*value, expected, expr.span),
            ExprKind::Float(_) => self.check_float(expected),
            ExprKind::Char(_) => Ty::scalar(ScalarType::U8),
            ExprKind::Str(_) => Ty {
                kind: TyKind::StaticStr,
                nullable: false,
            },
            ExprKind::Bool(_) => Ty::boolean(),
            ExprKind::Null => Ty {
                kind: TyKind::Unknown,
                nullable: true,
            },
            ExprKind::Name(name) => self.name_type(name),
            ExprKind::Qualified(name) => self.check_qualified(name),
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => self.check_call(callee, type_args, args, expr.span),
            ExprKind::Index { base, index } => self.check_index(base, index),
            ExprKind::Member { base, name } => self.check_member(base, name),
            ExprKind::Unary { op, operand } => self.check_unary(*op, operand),
            ExprKind::Binary { op, left, right } => self.check_binary(*op, left, right),
            ExprKind::Assign { op, target, value } => self.check_assign(*op, target, value),
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => self.check_conditional(condition, then_value, else_value),
            ExprKind::Cast { ty, value } => self.check_cast(ty, value),
            ExprKind::RawCast { ty, value } => self.check_raw_cast(ty, value),
            ExprKind::StructLiteral {
                name,
                type_args,
                fields,
            } => self.check_struct_literal(name, type_args, fields, expr.span),
            ExprKind::ArrayLiteral { elements, fill } => {
                self.check_array_literal(elements, fill, expected, expr.span)
            }
            ExprKind::Lambda {
                captures,
                params,
                body,
            } => self.check_lambda(captures, params, body, expr.span),
            ExprKind::Unsafe(inner) => self.check_value(inner, expected),
            ExprKind::IntrinsicCall { name, args } => self.check_intrinsic(name, args),
            ExprKind::Template(pieces) => self.check_template(pieces),
            ExprKind::Error => Ty::unknown(),
        }
    }

    /// Types a lambda, checking its body and classifying each capture.
    ///
    /// A capture-less lambda is a plain function pointer (`func(...)`). A
    /// capturing lambda is the ephemeral `Closure` type: it owns a stack
    /// environment for one synchronous call, so it is accepted only at a
    /// callable-bound generic parameter (see the call rules) and rejected
    /// wherever a storable value is required.
    ///
    /// The body is checked in a child scope that binds the captures and the
    /// parameters. While the body is checked the capture list is active, so a
    /// write or a move of a captured binding is reported here.
    fn check_lambda(
        &mut self,
        captures: &[crate::ast::Capture],
        params: &[crate::ast::LambdaParam],
        body: &LambdaBody,
        _span: Span,
    ) -> Ty {
        let id = self.next_closure_id;
        self.next_closure_id += 1;

        let mut infos: Vec<CaptureInfo> = Vec::new();
        for capture in captures {
            let ty = self.name_type(&capture.name);
            match capture.mode {
                crate::ast::Mode::Take => {
                    self.error(
                        "E10403",
                        "'take' captures are not supported",
                        "copy or borrow the capture",
                        capture.span,
                    );
                }
                crate::ast::Mode::Borrow => infos.push(CaptureInfo {
                    name: capture.name.clone(),
                    ty,
                    mode: CaptureMode::WriteBorrow,
                }),
                // A duplicable value is copied into the environment; a
                // non-duplicable one is borrowed read-only so its owner is not
                // moved. An opaque type stays a copy so an unmodelled
                // standard-library value is not rejected.
                crate::ast::Mode::None | crate::ast::Mode::Const => {
                    let mode = if ty.is_unknown() || self.table.duplicable(&ty) {
                        CaptureMode::Copy
                    } else {
                        CaptureMode::ReadOnlyBorrow
                    };
                    infos.push(CaptureInfo {
                        name: capture.name.clone(),
                        ty,
                        mode,
                    });
                }
            }
        }

        let param_types: Vec<Ty> = params
            .iter()
            .map(|param| self.resolve_type(&param.ty))
            .collect();

        self.push_scope();
        for info in &infos {
            self.declare(&info.name, info.ty.clone());
        }
        for (param, ty) in params.iter().zip(&param_types) {
            self.declare(&param.name, ty.clone());
        }

        self.lambda_captures.push(infos.clone());
        let returns = match body {
            LambdaBody::Expr(value) => {
                // An expression body may be `void` (for example a lambda that
                // only calls a void function), so it is checked as an
                // expression rather than a value.
                let ty = self.check_expr(value, None);
                self.reject_capture_move(value);
                ty
            }
            LambdaBody::Block(block) => {
                // A block body infers its return type from its `return`
                // statements. The enclosing return type is hidden while the
                // body is checked so a `return` inside the lambda is not
                // compared against the enclosing function's type.
                let saved_returns = std::mem::replace(&mut self.returns, Ty::unknown());
                self.lambda_returns.push(None);
                self.check_block(block);
                let inferred = self.lambda_returns.pop().flatten();
                self.returns = saved_returns;
                inferred.unwrap_or_else(Ty::void)
            }
        };
        self.lambda_captures.pop();
        self.pop_scope();

        if infos.is_empty() {
            Ty::func(FuncTy {
                params: param_types,
                returns: Box::new(returns),
            })
        } else {
            self.closures.push(ClosureSite {
                id,
                params: param_types.clone(),
                returns: returns.clone(),
                captures: infos,
            });
            Ty {
                kind: TyKind::Closure(ClosureTy {
                    id,
                    params: param_types,
                    returns: Box::new(returns),
                }),
                nullable: false,
            }
        }
    }

    /// Reports `E10401` when a lambda returns a borrowed capture by value.
    ///
    /// A copy capture is a value the lambda owns, so returning it is fine. A
    /// read-only or writable borrow only lends the enclosing binding, so moving
    /// it out of the lambda is not allowed.
    pub(super) fn reject_capture_move(&mut self, value: &Expr) {
        let Some(root) = super::place_root_name(value) else {
            return;
        };
        if matches!(
            self.capture_mode(root),
            Some(CaptureMode::ReadOnlyBorrow | CaptureMode::WriteBorrow)
        ) {
            self.error(
                "E10401",
                format!("cannot move captured '{root}' in a lambda"),
                "copy the value at capture time",
                value.span,
            );
        }
    }

    /// Checks an expression in a value position, reporting a `void` result.
    pub(super) fn check_value(&mut self, expr: &Expr, expected: Option<&Ty>) -> Ty {
        let ty = self.check_expr(expr, expected);
        if ty.is_void() {
            self.error(
                "E10217",
                "assignment result is 'void' and cannot be used as a value",
                "split the statement; do not chain assignments",
                expr.span,
            );
            return Ty::unknown();
        }
        ty
    }

    /// Checks an expression against an expected type and reports a bad
    /// implicit conversion.
    pub(super) fn check_into(&mut self, expr: &Expr, expected: &Ty) -> Ty {
        let ty = self.check_value(expr, Some(expected));
        if ty.is_void() {
            return ty;
        }
        self.check_conversion(&ty, expected, expr.span);
        ty
    }

    /// Checks an already-computed type against an expected one.
    ///
    /// Used where the caller has already checked the expression and only the
    /// conversion remains, so the expression is never walked twice.
    pub(super) fn check_conversion(&mut self, ty: &Ty, expected: &Ty, span: Span) {
        // A capturing closure owns a stack environment and satisfies only a
        // callable-bound generic parameter, never a concrete `func(...)`
        // value. A capture-less lambda is a `Func` and takes the branch below.
        if let (TyKind::Closure(_), TyKind::Func(_)) = (&ty.kind, &expected.kind) {
            self.error(
                "E10400",
                "capturing lambda cannot be used as 'func(...)'",
                "use a capture-less lambda, or a C-hook trampoline (v1.1)",
                span,
            );
            return;
        }
        if let (TyKind::Func(_), TyKind::Func(_)) = (&ty.kind, &expected.kind) {
            if ty != expected {
                self.error(
                    "E10210",
                    format!(
                        "function type mismatch: expected '{}', found '{}'",
                        expected.display(),
                        ty.display()
                    ),
                    "match the exact signature; capture-less lambdas only",
                    span,
                );
            }
            return;
        }
        if !implicit_conversion(ty, expected) {
            self.error(
                "E10200",
                format!(
                    "cannot convert '{}' to '{}' implicitly",
                    ty.display(),
                    expected.display()
                ),
                "use an explicit cast `T(expr)`",
                span,
            );
        }
    }

    /// Types an integer literal by its context, checking that it fits.
    ///
    /// Only an integer scalar context adapts the literal; a `bool`, float, or
    /// non-scalar context leaves it as the default `i32` so the caller reports
    /// the conversion error. This keeps an integer from silently becoming a
    /// `bool`.
    fn check_int(&mut self, value: u64, expected: Option<&Ty>, span: Span) -> Ty {
        if let Some(expected) = expected {
            if let TyKind::Scalar(scalar) = expected.kind {
                if crate::types::is_integer_scalar(scalar) {
                    if !int_fits(value as i128, scalar) {
                        self.error(
                            "E10202",
                            format!("literal does not fit '{}'", Ty::scalar(scalar).display()),
                            "use a wider type or a hex/binary form",
                            span,
                        );
                    }
                    return Ty::scalar(scalar);
                }
            }
        }
        Ty::i32_ty()
    }

    /// Types a float literal by its context; the default is `f64`.
    fn check_float(&self, expected: Option<&Ty>) -> Ty {
        if let Some(expected) = expected {
            if let TyKind::Scalar(ScalarType::F32) = expected.kind {
                return Ty::scalar(ScalarType::F32);
            }
        }
        Ty::scalar(ScalarType::F64)
    }

    /// Types a dotted name: a value's member chain, an enum member, a
    /// module-qualified function, or an opaque external name.
    fn check_qualified(&mut self, name: &QualifiedName) -> Ty {
        if let Some(first) = name.segments.first() {
            // A dotted name that starts with a value in scope is member access,
            // for example `point.x`; the parser cannot tell that apart from a
            // type path, so the choice is made here once names are known.
            if self.name_is_value(first) {
                let mut ty = self.name_type(first);
                for segment in &name.segments[1..] {
                    ty = self.member_type(&ty, segment, name.span);
                }
                return ty;
            }
            if self.table.is_enum(first) {
                if name.segments.len() >= 2 {
                    let member = name.segments.last().expect("a qualified name");
                    if !self.table.enum_has_member(first, member) {
                        self.error(
                            "E10212",
                            format!("'{member}' is not a member of '{first}'"),
                            "check the declaration or the spelling",
                            name.span,
                        );
                    }
                }
                return Ty::enum_(first);
            }
        }
        if let Some((last, head)) = name.segments.split_last() {
            if !head.is_empty() {
                let module = head.join(".");
                if let Some(signature) = self.table.function(&module, last) {
                    return Ty::func(signature.ty());
                }
            }
        }
        Ty::unknown()
    }

    /// Types the receiver of a value-headed dotted callee.
    ///
    /// For `a.b.c(args)` the receiver is `a.b` and the last segment `c` is the
    /// called member; the receiver is walked the same way a member chain is.
    pub(super) fn qualified_receiver_type(&mut self, name: &QualifiedName) -> Ty {
        let Some(first) = name.segments.first() else {
            return Ty::unknown();
        };
        let mut ty = self.name_type(first);
        let last = name.segments.len().saturating_sub(1);
        for segment in &name.segments[1..last] {
            ty = self.member_type(&ty, segment, name.span);
        }
        ty
    }

    /// Types a member access: a struct field or an enum member.
    fn check_member(&mut self, base: &Expr, name: &str) -> Ty {
        let base_ty = self.check_value(base, None);
        self.member_type(&base_ty, name, base.span)
    }

    /// The type of a named field on a struct value, with the instance's type
    /// arguments substituted into the declaration.
    ///
    /// A generic struct declares its fields in terms of its type parameters, so
    /// reading `Pair<i32>.first` must yield `i32`, not the bare `T`. A `raw`
    /// base peels to its pointee first, so a field reached through a pointer is
    /// typed, not left unknown. A non-generic struct, and a field the struct
    /// does not declare, substitute nothing.
    pub(super) fn field_type(&self, base_ty: &Ty, field: &str) -> Option<Ty> {
        match &base_ty.kind {
            TyKind::Struct(..) => self.table.instance_field(base_ty, field),
            TyKind::Raw(inner) => self.field_type(inner, field),
            _ => None,
        }
    }

    /// Types one member of an already-typed value, reporting an unknown name.
    ///
    /// A field and a free function may share a spelling for a dot-call, so the
    /// field wins and only a name that is neither is reported.
    fn member_type(&mut self, base_ty: &Ty, name: &str, span: Span) -> Ty {
        match &base_ty.kind {
            TyKind::Struct(struct_name, _) => {
                if let Some(field_ty) = self.field_type(base_ty, name) {
                    return field_ty;
                }
                if self.free_function(name).is_none() {
                    self.error(
                        "E10212",
                        format!("'{name}' is not a member of '{struct_name}'"),
                        "check the declaration or the spelling",
                        span,
                    );
                }
                Ty::unknown()
            }
            TyKind::Enum(enum_name) => {
                if !self.table.enum_has_member(enum_name, name) {
                    self.error(
                        "E10212",
                        format!("'{name}' is not a member of '{enum_name}'"),
                        "check the declaration or the spelling",
                        span,
                    );
                }
                base_ty.clone()
            }
            // A field reached through a raw pointer is typed from the pointee;
            // the unsafe gate for the dereference stays in the ownership stage.
            TyKind::Raw(_) => self.field_type(base_ty, name).unwrap_or_else(Ty::unknown),
            _ => Ty::unknown(),
        }
    }

    /// Types an index expression, reporting a non-indexable base or a constant
    /// index past a fixed array's extent.
    fn check_index(&mut self, base: &Expr, index: &Expr) -> Ty {
        let base_ty = self.check_value(base, None);
        self.check_value(index, None);
        match &base_ty.kind {
            TyKind::Array { element, extents } => {
                if let Some(Some(extent)) = extents.first() {
                    if let Some(value) = self.folder.integer(index) {
                        if value < 0 || value >= *extent as i128 {
                            self.error(
                                "E10216",
                                format!(
                                    "index {value} is out of range for '{}[{extent}]'",
                                    element.display()
                                ),
                                "fix the index or the extent",
                                index.span,
                            );
                        }
                    }
                }
                if extents.len() > 1 {
                    Ty::array((**element).clone(), extents[1..].to_vec())
                } else {
                    (**element).clone()
                }
            }
            TyKind::Str | TyKind::CString | TyKind::StaticStr => Ty::scalar(ScalarType::U8),
            // Indexing a raw pointer reaches the pointee, so `p[i]` has the
            // element type. The access is legal only inside `unsafe`, which
            // the ownership stage checks.
            TyKind::Raw(inner) => (**inner).clone(),
            TyKind::Unknown => Ty::unknown(),
            _ => {
                self.error(
                    "E10209",
                    format!("type '{}' cannot be indexed", base_ty.display()),
                    "arrays, slices, and `str` are indexable",
                    base.span,
                );
                Ty::unknown()
            }
        }
    }

    /// Types a scalar cast `T(expr)`.
    fn check_cast(&mut self, ty: &Type, value: &Expr) -> Ty {
        let target = self.resolve_type(ty);
        let source = self.check_value(value, None);
        if !scalar_cast_allowed(&source, &target) {
            self.error(
                "E10201",
                format!(
                    "cannot cast '{}' to '{}'",
                    source.display(),
                    target.display()
                ),
                "convert through a supported value or `raw` in `unsafe`",
                value.span,
            );
        }
        target
    }

    /// Types a raw-value cast `raw T(expr)`.
    fn check_raw_cast(&mut self, ty: &Type, value: &Expr) -> Ty {
        let target = self.resolve_type(ty);
        self.check_value(value, None);
        Ty {
            kind: TyKind::Raw(Box::new(target)),
            nullable: false,
        }
    }

    /// Types a struct literal, checking its fields and its type arguments.
    ///
    /// The field list must name every declared field exactly once, unless it
    /// is empty, which is the explicit zero value. An overlay type cannot be
    /// built as a value; a generic struct needs explicit type arguments.
    fn check_struct_literal(
        &mut self,
        name: &QualifiedName,
        type_args: &[Type],
        fields: &[FieldInit],
        span: Span,
    ) -> Ty {
        let Some(struct_name) = name
            .segments
            .last()
            .filter(|last| self.table.is_struct(last))
            .cloned()
        else {
            for field in fields {
                self.check_value(&field.value, None);
            }
            return Ty::unknown();
        };

        let mut resolved_args = Vec::with_capacity(type_args.len());
        for ty in type_args {
            resolved_args.push(self.resolve_type(ty));
        }
        // A generic aggregate whose type argument is a closure would have to
        // store the closure, whose environment is ephemeral. Only a callable
        // bound may consume a closure inline, and a struct field is not one.
        if resolved_args.iter().any(Ty::contains_closure) {
            self.error(
                "E10306",
                format!("'{struct_name}' cannot be instantiated with an ephemeral type argument"),
                "use a storable type",
                span,
            );
        }
        let target = Ty::struct_with(&struct_name, resolved_args);
        if let Some(params) = self.table.struct_type_params(&struct_name) {
            if type_args.is_empty() && !params.is_empty() {
                self.error(
                    "E10301",
                    format!("cannot infer type arguments for '{struct_name}'"),
                    "write the type arguments explicitly",
                    span,
                );
            } else if type_args.len() != params.len() {
                self.error(
                    "E10300",
                    format!(
                        "wrong number of type arguments for '{struct_name}': expected {}, found {}",
                        params.len(),
                        type_args.len()
                    ),
                    "list all type arguments",
                    span,
                );
            }
        }
        if self.table.is_overlay(&struct_name) {
            self.error(
                "E10207",
                format!("overlay type '{struct_name}' cannot be used by value"),
                "access it through a `raw` pointer inside `unsafe`",
                span,
            );
        }
        if !fields.is_empty() && !self.fields_match(&struct_name, fields) {
            self.error(
                "E10203",
                format!("struct literal for '{struct_name}' does not match its fields"),
                "list every field exactly once, by name",
                span,
            );
        }
        for field in fields {
            if let Some(field_ty) = self.field_type(&target, &field.name) {
                self.check_into(&field.value, &field_ty);
            } else {
                self.check_value(&field.value, None);
            }
        }
        target
    }

    /// True when a struct literal lists every declared field exactly once.
    fn fields_match(&self, struct_name: &str, fields: &[FieldInit]) -> bool {
        let Some(declared) = self.table.struct_fields(struct_name) else {
            return true;
        };
        if fields.len() != declared.len() {
            return false;
        }
        let mut seen = BTreeSet::new();
        for field in fields {
            if !declared.iter().any(|(name, _)| name == &field.name) {
                return false;
            }
            if !seen.insert(field.name.as_str()) {
                return false;
            }
        }
        true
    }

    /// Types an array literal, taking element type and extent from context
    /// where the context provides them.
    ///
    /// A fill form needs a fixed extent, and a literal prefix may not be
    /// longer than the context extent.
    fn check_array_literal(
        &mut self,
        elements: &[Expr],
        fill: &Option<Box<Expr>>,
        expected: Option<&Ty>,
        span: Span,
    ) -> Ty {
        let expected_extents = expected.and_then(|ty| match &ty.kind {
            TyKind::Array { extents, .. } => Some(extents.clone()),
            _ => None,
        });
        let expected_extent = expected_extents
            .as_ref()
            .and_then(|extents| extents.first().copied().flatten());
        // A context type that is already opaque cannot name an extent; the
        // extent error, if any, was reported where the extent was checked.
        let opaque_context = expected.is_some_and(Ty::is_unknown);
        if fill.is_some() && expected_extent.is_none() && !opaque_context {
            self.error(
                "E10215",
                "array fill requires a fixed extent",
                "annotate the array type",
                span,
            );
        }
        if let Some(extent) = expected_extent {
            if elements.len() > extent {
                self.error(
                    "E10215",
                    "array fill requires a fixed extent",
                    "annotate the array type",
                    span,
                );
            }
        }

        // A single-extent element is checked at the element type. For a
        // multi-extent type such as `u8[3][2]`, each element is itself the
        // array of the remaining extents, while the literal's type keeps the
        // base element and every extent.
        let (mut element, result_element) = match expected {
            Some(ty) => match &ty.kind {
                TyKind::Array { element, extents } if extents.len() > 1 => (
                    Some(Ty::array((**element).clone(), extents[1..].to_vec())),
                    Some((**element).clone()),
                ),
                TyKind::Array { element, .. } => {
                    (Some((**element).clone()), Some((**element).clone()))
                }
                _ => (None, None),
            },
            None => (None, None),
        };
        // A fill repeats one value into every element, so the element type must
        // be duplicable: repeating a move-only value would alias one owner into
        // several slots, and each slot would release the same storage. Without a
        // context element type the extent error above already covers the
        // literal.
        if fill.is_some() {
            if let Some(element_ty) = result_element.as_ref() {
                if !element_ty.is_unknown()
                    && !element_ty.is_param()
                    && !self.table.duplicable(element_ty)
                {
                    self.error(
                        "E10215",
                        "array fill requires a fixed extent",
                        "a fill cannot repeat a move-only element; construct each element",
                        span,
                    );
                }
            }
        }
        for expr in elements {
            let ty = match &element {
                Some(element_ty) => self.check_into(expr, element_ty),
                None => self.check_value(expr, None),
            };
            if let Some(element_ty) = &element {
                if ty.is_closure() && !matches!(element_ty.kind, TyKind::Func(_)) {
                    self.reject_ephemeral_storage(&ty, expr.span);
                }
            } else {
                self.reject_ephemeral_storage(&ty, expr.span);
            }
            if element.is_none() {
                element = Some(ty);
            }
        }
        if let Some(fill_expr) = fill {
            match &element {
                Some(element_ty) => {
                    let value_ty = self.check_into(fill_expr, element_ty);
                    if value_ty.is_closure() && !matches!(element_ty.kind, TyKind::Func(_)) {
                        self.reject_ephemeral_storage(&value_ty, fill_expr.span);
                    }
                }
                None => {
                    let value_ty = self.check_value(fill_expr, None);
                    self.reject_ephemeral_storage(&value_ty, fill_expr.span);
                }
            }
        }
        let extents = match expected_extents {
            Some(extents) if !extents.is_empty() => extents,
            _ if elements.is_empty() && fill.is_some() => vec![None],
            _ => vec![Some(elements.len())],
        };
        Ty::array(
            result_element.or(element).unwrap_or_else(Ty::unknown),
            extents,
        )
    }

    /// Types a reserved built-in call. Only the modeled diagnostics are
    /// checked; the rest return the type their signature promises.
    fn check_intrinsic(&mut self, name: &str, args: &[IntrinsicArg]) -> Ty {
        match name {
            "sizeof" => {
                for arg in args {
                    if let IntrinsicArg::Type(ty) = arg {
                        let _ = self.resolve_type(ty);
                    }
                }
                Ty::usize_ty()
            }
            "offsetof" => {
                for arg in args {
                    match arg {
                        IntrinsicArg::Type(ty) => {
                            let _ = self.resolve_type(ty);
                        }
                        IntrinsicArg::Expr(expr) => {
                            self.check_value(expr, None);
                        }
                    }
                }
                Ty::usize_ty()
            }
            "len" => {
                for arg in args {
                    if let IntrinsicArg::Expr(expr) = arg {
                        self.check_value(expr, None);
                    }
                }
                Ty::usize_ty()
            }
            "assert" => {
                if let Some(IntrinsicArg::Expr(condition)) = args.first() {
                    let ty = self.check_value(condition, None);
                    self.require_bool(&ty, condition.span);
                }
                // The first argument was already checked as the condition.
                self.check_intrinsic_expr_args(args.get(1..).unwrap_or(&[]));
                Ty::void()
            }
            "panic" => {
                self.check_intrinsic_expr_args(args);
                Ty::void()
            }
            // The six hardware intrinsics: the address must convert to the
            // matching `raw` pointer and the written value to the matching
            // unsigned scalar; the read returns that scalar. A mismatch is the
            // ordinary conversion diagnostic, so no new code is introduced.
            "readVolatile8" | "readVolatile16" | "readVolatile32" => {
                let scalar = volatile_scalar(name).expect("a readVolatile intrinsic");
                self.check_volatile_address(args, scalar);
                self.check_intrinsic_expr_args(args.get(1..).unwrap_or(&[]));
                Ty::scalar(scalar)
            }
            "writeVolatile8" | "writeVolatile16" | "writeVolatile32" => {
                let scalar = volatile_scalar(name).expect("a writeVolatile intrinsic");
                self.check_volatile_address(args, scalar);
                if let Some(IntrinsicArg::Expr(value)) = args.get(1) {
                    self.check_into(value, &Ty::scalar(scalar));
                }
                self.check_intrinsic_expr_args(args.get(2..).unwrap_or(&[]));
                Ty::void()
            }
            _ => {
                self.check_intrinsic_expr_args(args);
                Ty::unknown()
            }
        }
    }

    /// Checks the address argument of a volatile intrinsic against `raw T`.
    ///
    /// The first argument must be an expression convertible to a `raw` pointer
    /// to the intrinsic's scalar width. Any further arguments are left to the
    /// caller so no expression is checked twice.
    fn check_volatile_address(&mut self, args: &[IntrinsicArg], scalar: ScalarType) {
        if let Some(IntrinsicArg::Expr(addr)) = args.first() {
            let expected = Ty::raw(Ty::scalar(scalar));
            self.check_into(addr, &expected);
        }
    }

    /// Checks the expression arguments of an intrinsic, skipping type ones.
    fn check_intrinsic_expr_args(&mut self, args: &[IntrinsicArg]) {
        for arg in args {
            if let IntrinsicArg::Expr(expr) = arg {
                self.check_value(expr, None);
            }
        }
    }

    /// Types a template: `String` with placeholders, `str` without.
    ///
    /// Every placeholder must hold a formattable type: `str`, `static str`,
    /// `String`, or a scalar (integers, floats, and `bool`). An opaque type is
    /// left alone because the layer cannot name it.
    fn check_template(&mut self, pieces: &[TemplatePiece]) -> Ty {
        let mut has_placeholder = false;
        let mut first_placeholder = None;
        for piece in pieces {
            if let TemplatePiece::Placeholder(expr) = piece {
                has_placeholder = true;
                if first_placeholder.is_none() {
                    first_placeholder = Some(expr.span);
                }
                let ty = self.check_value(expr, None);
                if !ty.is_unknown() && !ty.is_param() && !formattable(&ty) {
                    self.error(
                        "E10500",
                        format!("type '{}' is not formattable", ty.display()),
                        "convert with `text`, `toHex`, or `toDecimal`",
                        expr.span,
                    );
                }
            }
        }
        if has_placeholder {
            // A placeholder lowers to the heap-backed formatter, which the
            // freestanding runtime does not provide. Reject it at compile time
            // rather than emit C that cannot link.
            if self.profile == Profile::Freestanding {
                if let Some(span) = first_placeholder {
                    self.error(
                        "E11200",
                        "'string formatting' is not available on profile 'freestanding'",
                        "use a plain string, or a profile that provides formatting",
                        span,
                    );
                }
            }
            Ty::handle("String")
        } else {
            Ty {
                kind: TyKind::Str,
                nullable: false,
            }
        }
    }

    /// Requires a `bool` operand, reporting `E10218` otherwise.
    pub(super) fn require_bool(&mut self, ty: &Ty, span: Span) {
        if ty.is_bool() || ty.is_unknown() || ty.is_param() {
            return;
        }
        self.error(
            "E10218",
            "condition must be 'bool'",
            "compare explicitly, for example `x != 0`",
            span,
        );
    }
}

/// The unsigned scalar width a volatile intrinsic accesses, keyed by its name.
///
/// The six names share one shape: `readVolatileN` reads a `uN` and
/// `writeVolatileN` writes a `uN` through a `raw uN`. The suffix number maps to
/// `u8`, `u16`, or `u32`.
fn volatile_scalar(name: &str) -> Option<ScalarType> {
    match name {
        "readVolatile8" | "writeVolatile8" => Some(ScalarType::U8),
        "readVolatile16" | "writeVolatile16" => Some(ScalarType::U16),
        "readVolatile32" | "writeVolatile32" => Some(ScalarType::U32),
        _ => None,
    }
}

/// True when a type may appear in a template placeholder.
fn formattable(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Str | TyKind::StaticStr => true,
        TyKind::Handle(name) => name == "String",
        TyKind::Scalar(_) => true,
        _ => false,
    }
}

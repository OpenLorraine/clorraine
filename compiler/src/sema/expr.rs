//! Expression checking.
//!
//! Types flow upward: an expression is checked once and returns the type it
//! produces. The expected type is passed down only where it changes the answer,
//! which is a literal taking its context type. A `void` expression used where a
//! value is needed is reported here as the assignment-result error.
//!
//! The operator, assignment, and conditional rules live in the sibling
//! `ops` module; this one owns names, calls, literals, and aggregates.

use crate::ast::{
    Arg, Expr, ExprKind, FieldInit, IntrinsicArg, QualifiedName, ScalarType, TemplatePiece, Type,
};
use crate::span::Span;
use crate::types::{
    implicit_conversion, int_fits, scalar_cast_allowed, FuncSig, Ty, TyKind,
};

use super::Checker;

impl Checker<'_, '_> {
    /// Checks an expression and returns its type.
    ///
    /// `expected` carries the context type where it matters to a literal.
    pub(super) fn check_expr(&mut self, expr: &Expr, expected: Option<&Ty>) -> Ty {
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
            ExprKind::Call { callee, args, .. } => self.check_call(callee, args, expr.span),
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
            ExprKind::StructLiteral { name, fields, .. } => self.check_struct_literal(name, fields),
            ExprKind::ArrayLiteral { elements, fill } => {
                self.check_array_literal(elements, fill, expected)
            }
            // Lambdas and the staged features are not modeled yet; the opaque
            // type keeps their uses from producing false follow-on errors.
            ExprKind::Lambda { .. } => Ty::unknown(),
            ExprKind::Unsafe(inner) => self.check_value(inner, expected),
            ExprKind::IntrinsicCall { name, args } => self.check_intrinsic(name, args),
            ExprKind::Template(pieces) => self.check_template(pieces),
            ExprKind::Error => Ty::unknown(),
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
        if let (TyKind::Func(_), TyKind::Func(_)) = (&ty.kind, &expected.kind) {
            if ty != *expected {
                self.error(
                    "E10210",
                    format!(
                        "function type mismatch: expected '{}', found '{}'",
                        expected.display(),
                        ty.display()
                    ),
                    "match the exact signature; capture-less lambdas only",
                    expr.span,
                );
            }
            return ty;
        }
        if !implicit_conversion(&ty, expected) {
            self.error(
                "E10200",
                format!(
                    "cannot convert '{}' to '{}' implicitly",
                    ty.display(),
                    expected.display()
                ),
                "use an explicit cast `T(expr)`",
                expr.span,
            );
        }
        ty
    }

    /// Types an integer literal by its context, checking that it fits.
    fn check_int(&mut self, value: u64, expected: Option<&Ty>, span: Span) -> Ty {
        if let Some(expected) = expected {
            if let TyKind::Scalar(scalar) = expected.kind {
                if !crate::types::is_float_scalar(scalar) {
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

    /// Types a dotted name: an enum member, a module-qualified function, or an
    /// opaque external name.
    fn check_qualified(&mut self, name: &QualifiedName) -> Ty {
        if let Some(first) = name.segments.first() {
            if !self.name_is_value(first) && self.table.is_enum(first) {
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

    /// Types a member access: a struct field or an enum member.
    fn check_member(&mut self, base: &Expr, name: &str) -> Ty {
        let base_ty = self.check_value(base, None);
        match &base_ty.kind {
            TyKind::Struct(struct_name) => self
                .table
                .struct_field(struct_name, name)
                .unwrap_or_else(Ty::unknown),
            TyKind::Enum(_) => base_ty.clone(),
            _ => Ty::unknown(),
        }
    }

    /// Types an index expression and reports a non-indexable base.
    fn check_index(&mut self, base: &Expr, index: &Expr) -> Ty {
        let base_ty = self.check_value(base, None);
        self.check_value(index, None);
        match &base_ty.kind {
            TyKind::Array { element, extents } => {
                if extents.len() > 1 {
                    Ty::array((**element).clone(), extents[1..].to_vec())
                } else {
                    (**element).clone()
                }
            }
            TyKind::Str | TyKind::CString | TyKind::StaticStr => Ty::scalar(ScalarType::U8),
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

    /// Types a call: a type cast, a function call, a dot-call, or an opaque
    /// external call whose signature is not modelled.
    fn check_call(&mut self, callee: &Expr, args: &[Arg], span: Span) -> Ty {
        if let Some(target) = self.callee_type(callee) {
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
            if let Some(signature) = self.dot_call(&base_ty, name) {
                self.check_args(args, &signature.params);
                return signature.returns;
            }
            for arg in args {
                self.check_value(&arg.value, None);
            }
            return Ty::unknown();
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

    /// Resolves the free function a dot-call names, when the receiver is a
    /// struct whose field of that name is callable, or a module function.
    fn dot_call(&self, base: &Ty, name: &str) -> Option<FuncSig> {
        if let TyKind::Struct(struct_name) = &base.kind {
            if let Some(field_ty) = self.table.struct_field(struct_name, name) {
                if let TyKind::Func(signature) = &field_ty.kind {
                    return Some(FuncSig {
                        params: signature.params.clone(),
                        returns: (*signature.returns).clone(),
                        type_params: Vec::new(),
                    });
                }
                return None;
            }
        }
        self.free_function(name)
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
            TyKind::Struct(_) => false,
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

    /// Types a struct literal, checking field values against their declared
    /// types where those are known.
    fn check_struct_literal(&mut self, name: &QualifiedName, fields: &[FieldInit]) -> Ty {
        let target = match name.segments.last() {
            Some(last) if self.table.is_struct(last) => Ty::struct_(last),
            _ => Ty::unknown(),
        };
        for field in fields {
            if let TyKind::Struct(struct_name) = &target.kind {
                if let Some(field_ty) = self.table.struct_field(struct_name, &field.name) {
                    self.check_into(&field.value, &field_ty);
                    continue;
                }
            }
            self.check_value(&field.value, None);
        }
        target
    }

    /// Types an array literal, taking element type and extent from context
    /// where the context provides them.
    fn check_array_literal(
        &mut self,
        elements: &[Expr],
        fill: &Option<Box<Expr>>,
        expected: Option<&Ty>,
    ) -> Ty {
        let expected_element = expected.and_then(|ty| match &ty.kind {
            TyKind::Array { element, .. } => Some((**element).clone()),
            _ => None,
        });
        let mut element = expected_element.clone();
        for (index, expr) in elements.iter().enumerate() {
            let ty = match &expected_element {
                Some(element_ty) => self.check_into(expr, element_ty),
                None => self.check_value(expr, None),
            };
            if index == 0 && element.is_none() {
                element = Some(ty);
            }
        }
        if let Some(fill_expr) = fill {
            match &expected_element {
                Some(element_ty) => {
                    self.check_into(fill_expr, element_ty);
                }
                None => {
                    self.check_value(fill_expr, None);
                }
            }
        }
        let extents = if elements.is_empty() && fill.is_some() {
            vec![None]
        } else {
            vec![Some(elements.len())]
        };
        Ty::array(element.unwrap_or_else(Ty::unknown), extents)
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
                self.check_intrinsic_expr_args(args);
                Ty::void()
            }
            "panic" => {
                self.check_intrinsic_expr_args(args);
                Ty::void()
            }
            _ => {
                self.check_intrinsic_expr_args(args);
                Ty::unknown()
            }
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
    fn check_template(&mut self, pieces: &[TemplatePiece]) -> Ty {
        let mut has_placeholder = false;
        for piece in pieces {
            if let TemplatePiece::Placeholder(expr) = piece {
                has_placeholder = true;
                self.check_value(expr, None);
            }
        }
        if has_placeholder {
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

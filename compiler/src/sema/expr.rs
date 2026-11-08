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
use crate::types::{ implicit_conversion, int_fits, scalar_cast_allowed, CaptureInfo, CaptureMode, ClosureSite, ClosureTy, FuncParam, FuncTy, Ty, TyKind, PlacePathStep };

use super::{
    CaptureMoveRequirement, Checker, ConversionLiteral, OpaqueConversion, OpaqueIndex,
    OpaqueMemberUse, OpaqueProjection,
};

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
            ExprKind::Name(name) => self.check_name(name, expr.span),
            ExprKind::Qualified(name) => self.check_qualified(expr, name),
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                // A cast is exactly `T(expr)`: no type arguments and no
                // call-site marker. Any other form stays on the ordinary call
                // path, which is the pre-existing documented limitation.
                let exact_cast = type_args.is_empty()
                    && args.len() == 1
                    && matches!(args[0].marker, crate::ast::Mode::None);
                let ty = self.check_call(callee, type_args, args, expr.span, expr as *const Expr as usize);
                if exact_cast && self.callee_type(callee).is_some() {
                    self.type_casts.insert(expr as *const Expr as usize);
                }
                ty
            }
            ExprKind::Index { base, index } => self.check_index(base, index),
            ExprKind::Member { base, name } => self.check_member(expr, base, name),
            ExprKind::Unary { op, operand } => self.check_unary(expr, *op, operand, expected),
            ExprKind::Binary { op, left, right } => self.check_binary(*op, left, right),
            ExprKind::Assign { op, target, value } => self.check_assign(*op, target, value),
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => self.check_conditional(condition, then_value, else_value, expected, expr.span),
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
            ExprKind::Unsafe(inner) => {
                self.unsafe_depth += 1;
                let ty = self.check_value(inner, expected);
                self.unsafe_depth -= 1;
                ty
            }
            ExprKind::IntrinsicCall { name, args } => {
                self.check_intrinsic(name, args, expr.span)
            }
            ExprKind::Template(pieces) => self.check_template(pieces),
            ExprKind::Error => Ty::unknown(),
        }
    }

    /// Types a bare name read and rejects an unlisted enclosing read.
    ///
    /// A lambda body may read the bindings its capture list names and the
    /// bindings introduced inside the body; any other lexical binding belongs
    /// to an enclosing function or lambda, so the lifted function would
    /// reference a name it does not own. Module-level names have no lexical
    /// scope and stay readable without a listing.
    fn check_name(&mut self, name: &str, span: Span) -> Ty {
        if let Some(&floor) = self.lambda_floors.last() {
            let listed = self
                .lambda_listed
                .last()
                .is_some_and(|names| names.iter().any(|listed| listed == name));
            if !listed && self.name_scope_depth(name).is_some_and(|depth| depth < floor) {
                self.error(
                    "E10405",
                    format!("lambda reads '{name}' without listing it"),
                    &format!("add '{name}' to the capture list"),
                    span,
                );
            }
        }
        if name == "Box" && !self.name_is_value(name) {
            self.error(
                "E10219",
                "'Box' is a type, not a value",
                "use `Box<T>` in a type position, or `boxNew` to create a box",
                span,
            );
            return Ty::unknown();
        }
        self.name_type(name)
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
            // A module-level name needs no capture, and a nested lambda's
            // entry must travel through the enclosing lambda's environment;
            // both are reported before the entry is classified. The rejected
            // entry stays in the raw list, so the body read it would
            // otherwise satisfy stays covered by the listing rule and the
            // site keeps one report.
            if self.check_capture_entry(&capture.name, capture.span) {
                continue;
            }
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
            .map(|param| {
                let ty = self.resolve_type(&param.ty);
                self.check_nullable_type(&ty, param.ty.span);
                ty
            })
            .collect();

        let listed: Vec<String> = captures
            .iter()
            .map(|capture| capture.name.clone())
            .collect();

        let floor = self.scopes.len();
        self.push_scope();
        for info in &infos {
            // A borrowed capture is a writable lend exactly as the enclosing
            // parameter is; a copy capture owns its value.
            let lend = matches!(
                info.mode,
                CaptureMode::ReadOnlyBorrow | CaptureMode::WriteBorrow
            );
            self.declare_with_lend(&info.name, info.ty.clone(), lend);
        }
        for (param, ty) in params.iter().zip(&param_types) {
            // A lambda parameter owns its value; only the enclosing
            // parameters and the borrowed captures are lends.
            self.declare(&param.name, ty.clone());
        }

        self.lambda_captures.push(infos.clone());
        self.lambda_floors.push(floor);
        self.lambda_listed.push(listed);
        self.captured_move_records.push(BTreeSet::new());
        let returns = match body {
            LambdaBody::Expr(value) => {
                // An expression body may be `void` (for example a lambda that
                // only calls a void function), so it is checked as an
                // expression rather than a value.
                let ty = self.check_expr(value, None);
                self.reject_capture_move(value);
                self.record_generic_capture_move(value);
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
        self.lambda_listed.pop();
        self.lambda_floors.pop();
        self.lambda_captures.pop();
        self.captured_move_records.pop();
        self.pop_scope();

        if infos.is_empty() {
            Ty::func(FuncTy {
                params: param_types
                    .iter()
                    .cloned()
                    .map(|ty| FuncParam {
                        ty,
                        mode: crate::ast::Mode::None,
                    })
                    .collect(),
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
                    instance: None,
                }),
                nullable: false,
            }
        }
    }

    /// Reports `E10401` when a lambda moves a borrowed capture by value.
    ///
    /// A copy capture is a value the lambda owns, so returning it is fine. A
    /// read-only or writable borrow only lends the enclosing binding, so moving
    /// it out of the lambda is not allowed. A conditional selects one arm at
    /// run time, so its arms are walked like the ownership stage's conditional
    /// move sources; one report names the first borrowed capture found, so a
    /// site reports once.
    pub(super) fn reject_capture_move(&mut self, value: &Expr) {
        let mut roots = Vec::new();
        super::place_move_paths(value, &[], &mut roots);
        for (root, path) in roots {
            if !matches!(
                self.capture_mode(root),
                Some(CaptureMode::ReadOnlyBorrow | CaptureMode::WriteBorrow)
            ) {
                continue;
            }
            // Only a move is rejected: a projection whose type is duplicable
            // copies out of the borrow, so a read of such a field stays
            // accepted and only a whole-root or owning move reports.
            let projected =
                crate::types::project_place_path(&self.name_type(root), &path, self.table);
            if projected.is_unknown() || self.table.duplicable(&projected) {
                continue;
            }
            self.error(
                "E10401",
                format!("cannot move captured '{root}' in a lambda"),
                "copy the value at capture time",
                value.span,
            );
            return;
        }
    }

    /// Records a by-value capture the lambda returns or consumes while the
    /// capture's duplicability is still open.
    ///
    /// A copy capture is sound only when its type is duplicable; a capture
    /// whose type still mentions the enclosing function's type parameter may
    /// substitute to an owning type, and the copy would then alias the
    /// enclosing binding. The site is recorded and the per-instance audit
    /// applies the concrete judgment after substitution.
    ///
    /// Every body form records the duty: the expression body's value and a
    /// block body's `return` both reach this walk, and a `take` argument, a
    /// `let` initializer, an assignment value, and a composite literal's field
    /// value are recorded at their own sites, because a move there is a move
    /// of the capture at a position no escape covers. A conditional selects
    /// one arm at run time, so its arms are walked like the ownership stage's
    /// conditional move sources; one record per capture keeps the site's
    /// report single, and a composite records at its innermost move site
    /// before the enclosing site's walk reaches the same name.
    pub(super) fn record_generic_capture_move(&mut self, value: &Expr) {
        // A read-only borrow capture is a borrow, so a move at a let or
        // assignment site is the capture rule too; a duplicable capture copies
        // and the per-instance judgment skips it.
        self.record_capture_move(value, true, None);
    }

    /// Records a capture move at a store for the per-instance audit.
    ///
    /// A store consumes its value, so a *borrowed* capture is recorded too: at
    /// an owning instance the store moves the value out of the borrow, which
    /// the concrete rule cannot judge while the capture's type is a parameter.
    /// A duplicable instance copies out of the borrow and stays accepted.
    pub(super) fn record_store_capture_move(&mut self, value: &Expr, dest: &Ty, borrows: bool) {
        self.record_capture_move(value, borrows, Some(dest));
    }

    /// The shared capture-move record for one value.
    fn record_capture_move(&mut self, value: &Expr, borrows: bool, dest: Option<&Ty>) {
        let mut roots = Vec::new();
        super::place_move_paths(value, &[], &mut roots);
        if roots.is_empty() {
            return;
        }
        let Some(function) = self.function_name.clone() else {
            return;
        };
        let mut seen: BTreeSet<(String, Vec<PlacePathStep>)> = BTreeSet::new();
        for (root, path) in roots {
            if !seen.insert((root.to_owned(), path.clone())) {
                continue;
            }
            let borrowed = match self.capture_mode(root) {
                Some(CaptureMode::Copy) => false,
                Some(CaptureMode::ReadOnlyBorrow | CaptureMode::WriteBorrow) => true,
                None => continue,
            };
            if borrowed && !borrows {
                continue;
            }
            // A write-borrow capture has its own store site; a plain local
            // let/assignment in the body keeps the lend-store report unless
            // the move is a read-only capture.
            if dest.is_none() && matches!(self.capture_mode(root), Some(CaptureMode::WriteBorrow)) {
                continue;
            }
            let ty = self.lambda_captures.iter().rev().find_map(|captures| {
                captures
                    .iter()
                    .find(|info| info.name == root)
                    .map(|info| info.ty.clone())
            });
            let Some(ty) = ty else {
                continue;
            };
            // A concrete by-value copy is not a capture move; the generic
            // spelling still records the duty for its instances.
            if !crate::types::contains_param(&ty)
                && (!borrowed || !path.is_empty())
            {
                continue;
            }
            let Some(records) = self.captured_move_records.last_mut() else {
                continue;
            };
            if !records.insert((root.to_owned(), path.clone())) {
                continue;
            }
            self.capture_moves.push(CaptureMoveRequirement {
                function: function.clone(),
                capture: root.to_owned(),
                ty,
                path,
                dest: dest.cloned(),
                borrow: borrowed,
                span: value.span,
            });
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

    /// Checks an expression against a destination slot or a return type.
    ///
    /// A let or module store, an assignment to a field or element, an
    /// array-literal element, and a return are the positions the frozen rules
    /// let a view's pointer member reach a `cstring` without a cast in the
    /// source: the sanctioned conversion. Every other `cstring` position — a
    /// call argument, an FFI argument, a struct-literal field — goes through
    /// [`Checker::check_into`] and reports the frozen conversion rule.
    pub(super) fn check_into_slot(&mut self, expr: &Expr, expected: &Ty) -> Ty {
        self.check_into_position(expr, expected, true)
    }

    /// Checks an expression against an expected type and reports a bad
    /// implicit conversion.
    pub(super) fn check_into(&mut self, expr: &Expr, expected: &Ty) -> Ty {
        self.check_into_position(expr, expected, false)
    }

    /// The shared conversion check, marked with the position it sits at.
    ///
    /// `slot` marks the destination-slot and return positions; only there the
    /// sanctioned view pointer conversion is available. A member read on an
    /// opaque base is recorded with its position so the instance audit judges
    /// the conversion exactly as the concrete read would, including the
    /// position's exemption.
    fn check_into_position(&mut self, expr: &Expr, expected: &Ty, slot: bool) -> Ty {
        let ty = self.check_value(expr, Some(expected));
        if ty.is_void() {
            return ty;
        }
        // A view's pointer member read is the one spelling the frozen
        // conversion table lets reach a `cstring` slot; the sanctioned
        // conversion stays accepted there and the emitter spells the cast.
        if slot
            && matches!(expected.kind, TyKind::CString)
            && self
                .view_pointer_members
                .contains(&(expr as *const Expr as usize))
        {
            return ty;
        }
        // A member read on an opaque base has no type until the instance is
        // known: its conversion is recorded and judged at the instance,
        // exactly as the concrete read would report it. The parser spells a
        // dotted read `Qualified` and a postfix read `Member`, so both shapes
        // name the member.
        if ty.is_unknown() {
            if let Some(name) = member_read_name(expr) {
                if let Some(function) = self.function_name.clone() {
                    self.opaque_member_uses.push(OpaqueMemberUse {
                        function,
                        name,
                        span: expr.span,
                        expected: expected.clone(),
                        sanctioned: slot,
                        base: plain_binding_root(expr),
                    });
                    return ty;
                }
            }
        }
        let literal = Self::conversion_literal(expr);
        // A conditional's own duty owns the arm agreement and the destination,
        // so the ordinary deferred conversion is not recorded for it; the
        // site keeps the destination the instance audit substitutes.
        let deferred_conditional = self.note_conditional_destination(expr, expected);
        if !deferred_conditional {
            self.record_deferred_conversion_with(&ty, expected, expr.span, literal, false);
        }
        self.check_conversion(&ty, expected, expr.span);
        ty
    }

    /// Records a value whose conversion is deferred to its instance.
    ///
    /// A source that mentions a type parameter cannot be judged at the
    /// declaration, so the instance audit applies the ordinary conversion (or
    /// null) judgment to the substituted source and destination. A destination
    /// that still mentions a type parameter judges its instance too: a
    /// concrete source converts to whatever the instance resolves, and the
    /// audit reports exactly as the concrete twin does. A numeric literal is
    /// marked so the audit lets a scalar context supply its type. A null-shaped
    /// source is recorded only when the destination is itself unresolved: a
    /// concrete destination already faces the position's own rules, so
    /// recording it again would report twice or reject where the concrete
    /// spelling accepts.
    pub(super) fn record_deferred_conversion(&mut self, source: &Ty, expected: &Ty, span: Span) {
        self.record_deferred_conversion_with(source, expected, span, None, false);
    }

    /// Records a deferred conversion, carrying its literal and slot shape.
    pub(super) fn record_deferred_conversion_with(
        &mut self,
        source: &Ty,
        expected: &Ty,
        span: Span,
        literal: Option<ConversionLiteral>,
        borrow_slot: bool,
    ) {
        // A destination that is a nullable type parameter is already rejected
        // where it is written (a type parameter cannot be empty), so its
        // recovered substituted shape is never judged again: the declaration
        // keeps its single report instead of a dependent one.
        if expected.nullable && matches!(expected.kind, TyKind::Param(_)) {
            return;
        }
        let null_deferred = crate::types::is_null_shaped(source)
            && (crate::types::contains_param(expected) || expected.is_unknown());
        // A destination that still mentions a type parameter judges its
        // instance too: a concrete source converts to whatever the instance
        // resolves, so the duty records and the audit reports exactly as the
        // concrete twin does.
        if !crate::types::contains_param(source)
            && !null_deferred
            && !crate::types::contains_param(expected)
        {
            return;
        }
        let Some(function) = self.function_name.clone() else {
            return;
        };
        self.opaque_conversions.push(OpaqueConversion {
            function,
            source: source.clone(),
            expected: expected.clone(),
            span,
            literal,
            borrow_slot,
        });
    }

    /// The literal an expression writes, when it writes one.
    ///
    /// A negated literal folds into its value, so `-5` judges like `-5` and
    /// not like an unknown.
    pub(super) fn conversion_literal(expr: &Expr) -> Option<ConversionLiteral> {
        match &expr.kind {
            ExprKind::Int(value) => Some(ConversionLiteral::Int(i128::from(*value))),
            ExprKind::Float(value) => Some(ConversionLiteral::Float(*value)),
            ExprKind::Unary {
                op: crate::ast::UnaryOp::Neg,
                operand,
            } => match &operand.kind {
                ExprKind::Int(value) => Some(ConversionLiteral::Int(-i128::from(*value))),
                ExprKind::Float(value) => Some(ConversionLiteral::Float(-*value)),
                _ => None,
            },
            _ => None,
        }
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
                "use a capture-less lambda, a `borrow` C hook, or a generic callable bound",
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
                    "match the exact signature and the ABI-identical callback set",
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
    fn check_qualified(&mut self, expr: &Expr, name: &QualifiedName) -> Ty {
        if let Some(first) = name.segments.first() {
            // A module path that reaches one of the module's exported bindings
            // is the qualified reading, with or without an import of the same
            // name: the path mentions no name of its own, so it resolves even
            // though an import binds the head. A lexical binding still
            // shadows the path.
            if !self.is_local(first) {
                // An intrinsic or `ffi` declaration is call-only: it has no
                // value symbol of its own, so a value position keeps the
                // ordinary undeclared report instead of emitting a dotted C
                // path. A lexical binding of the head still shadows the path.
                if let Some((last, head)) = name.segments.split_last() {
                    if !head.is_empty() {
                        let module = head.join(".");
                        if matches!(
                            self.qualified_decl_kind(&module, last),
                            Some(crate::load::DeclKind::Ffi | crate::load::DeclKind::Intrinsic)
                        ) {
                            self.error(
                                "E10219",
                                format!("'{}' is not declared", name.segments.join(".")),
                                "call the function directly",
                                name.span,
                            );
                            return Ty::unknown();
                        }
                    }
                }
                // A module function value reaches its declaration before the
                // value-head route: a module path wins over an imported value
                // that shares the head's spelling.
                if let Some((last, head)) = name.segments.split_last() {
                    if !head.is_empty() {
                        let module = head.join(".");
                        if let Some(signature) = self.table.function(&module, last) {
                            return Ty::func(signature.ty());
                        }
                    }
                }
                if let Some(ty) = self.module_path_type(&name.segments, name.span) {
                    return ty;
                }
            }
            // A dotted name that starts with a value in scope is member access,
            // for example `point.x`; the parser cannot tell that apart from a
            // type path, so the choice is made here once names are known.
            if self.name_is_value(first) {
                let mut ty = self.check_name(first, name.span);
                let last = name.segments.len().saturating_sub(1);
                for (index, segment) in name.segments[1..].iter().enumerate() {
                    // The final segment is the member the expression names; a
                    // view's pointer member is the read the conversion table
                    // lets reach a `cstring` destination.
                    if index + 1 == last
                        && segment == "ptr"
                        && (matches!(ty.kind, TyKind::Str | TyKind::StaticStr) || ty.is_slice())
                    {
                        self.view_pointer_members
                            .insert(expr as *const Expr as usize);
                    }
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

    /// The type a dotted path names through a module path.
    ///
    /// The leading segments must spell a loaded module and the next one a
    /// binding the module declares (and exports, unless the module is the
    /// checker's own), so `m.a.holder` reads the binding and a later segment
    /// is one of its fields. The longest module prefix wins, so a binding is
    /// reached even when an earlier split also names a module; `None` lets
    /// the caller keep its value-head or opaque handling.
    fn module_path_type(&mut self, segments: &[String], span: Span) -> Option<Ty> {
        for split in (1..segments.len()).rev() {
            let module = segments[..split].join(".");
            if module.is_empty() {
                continue;
            }
            let binding = &segments[split];
            let visible = if module == self.module.name {
                self.module.decls.contains_key(binding)
            } else {
                self.loaded.has_module(&module) && self.loaded.module_exports(&module, binding)
            };
            if !visible {
                continue;
            }
            let Some(declared) = self.table.global(&module, binding) else {
                continue;
            };
            let mut ty = declared.clone();
            for field in &segments[split + 1..] {
                ty = self.member_type(&ty, field, span);
            }
            return Some(ty);
        }
        None
    }

    /// The module that declares a module-path receiver binding.
    ///
    /// For `m.holder.member(args)` this returns `m` and the type of `holder`;
    /// the caller uses it to distinguish a module-path receiver from a value
    /// receiver when a method-shaped call has no local candidate.
    pub(super) fn module_path_binding(&self, segments: &[String]) -> Option<(String, Ty)> {
        for split in (1..segments.len()).rev() {
            let module = segments[..split].join(".");
            if module.is_empty() {
                continue;
            }
            let binding = &segments[split];
            let visible = if module == self.module.name {
                self.module.decls.contains_key(binding)
            } else {
                self.loaded.has_module(&module) && self.loaded.module_exports(&module, binding)
            };
            if !visible {
                continue;
            }
            if let Some(declared) = self.table.global(&module, binding) {
                return Some((module, declared.clone()));
            }
        }
        None
    }

    /// Types the receiver of a value-headed dotted callee.
    ///
    /// For `a.b.c(args)` the receiver is `a.b` and the last segment `c` is the
    /// called member; the receiver is walked the same way a member chain is.
    pub(super) fn qualified_receiver_type(&mut self, name: &QualifiedName) -> Ty {
        let Some(first) = name.segments.first() else {
            return Ty::unknown();
        };
        let last = name.segments.len().saturating_sub(1);
        // The receiver of a value-headed dotted callee may itself be a module
        // path reaching one of the module's bindings; a lexical binding of the
        // head's name still shadows the path.
        if !self.is_local(first) {
            if let Some(ty) = self.module_path_type(&name.segments[..last], name.span) {
                return ty;
            }
        }
        let mut ty = self.check_name(first, name.span);
        for segment in &name.segments[1..last] {
            ty = self.member_type(&ty, segment, name.span);
        }
        ty
    }

    /// Types a member access: a struct field or an enum member.
    fn check_member(&mut self, expr: &Expr, base: &Expr, name: &str) -> Ty {
        let base_ty = self.check_value(base, None);
        // A view's pointer member is the one read the frozen conversion table
        // lets reach a `cstring` destination; the read is marked so the
        // conversion positions accept it and spell the sanctioned cast.
        if name == "ptr"
            && (matches!(base_ty.kind, TyKind::Str | TyKind::StaticStr) || base_ty.is_slice())
        {
            self.view_pointer_members
                .insert(expr as *const Expr as usize);
        }
        // The projection records the whole read's span, not only the base's:
        // the instance audit must be able to tell a bare opaque member from a
        // longer nested chain that merely starts at the same place.
        self.member_type(&base_ty, name, expr.span)
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
                self.error(
                    "E10212",
                    format!("'{name}' is not a member of '{struct_name}'"),
                    "check the declaration or the spelling",
                    span,
                );
                Ty::unknown()
            }
            // The frozen member table spells an enum member `Type.MEMBER`:
            // a value receiver has no member surface at all, so every read
            // reports the registered unknown-member rule and no member value
            // is invented. The scoped `E.MEMBER` form is handled by the
            // qualified-name path before this dispatch.
            TyKind::Enum(enum_name) => {
                self.error(
                    "E10212",
                    format!("'{name}' is not a member of '{enum_name}'"),
                    "check the declaration or the spelling",
                    span,
                );
                Ty::unknown()
            }
            // A view's members are its ABI fields: `.ptr` is the pointer to
            // the view's element and `.len` the element count, typed by the
            // shared rule so the conversion rules judge a destination the
            // member cannot reach; the renderer already spells the struct
            // fields and the fixed-array `.len` keeps its compile-time fold.
            // A name outside that table has no member surface at all, so the
            // read reports the unknown-member rule like any other base.
            TyKind::Str | TyKind::StaticStr | TyKind::Array { .. } => {
                match crate::types::view_member_ty(base_ty, name) {
                    Some(member_ty) => member_ty,
                    None => {
                        self.report_unknown_member(base_ty, name, span);
                        Ty::unknown()
                    }
                }
            }
            // A null-shaped head has no member surface. The unresolved type
            // carries no field table, so every read and store reports the
            // frozen unknown-member rule rather than emitting a C member path.
            TyKind::Unknown if base_ty.nullable => {
                self.error(
                    "E10212",
                    format!("'{name}' is not a member of 'unknown'"),
                    "check the declaration or the spelling",
                    span,
                );
                Ty::unknown()
            }
            TyKind::Unknown => Ty::unknown(),
            // A field reached through a raw pointer is typed from the
            // pointee, because the dereference is implied: a raw struct's
            // field resolves, while a raw enum's member is the value form of
            // its enum and every read reports the unknown-member rule — the
            // same report the instance path gives the substituted read. The
            // unsafe gate for the dereference stays in the ownership stage.
            TyKind::Raw(inner) => match &inner.kind {
                TyKind::Struct(..) => match self.field_type(base_ty, name) {
                    Some(field_ty) => field_ty,
                    None => {
                        self.report_unknown_member(base_ty, name, span);
                        Ty::unknown()
                    }
                },
                // A raw pointer to a view still implies one dereference, so
                // the view's ABI members remain reachable. Every other raw
                // pointee is not a member surface: accepting `.ptr` or `.len`
                // on `raw u8` would emit a struct-member arrow on a pointer.
                TyKind::Str | TyKind::StaticStr | TyKind::Array { .. } => {
                    match crate::types::view_member_ty(inner, name) {
                        Some(member_ty) => member_ty,
                        None => {
                            self.report_unknown_member(base_ty, name, span);
                            Ty::unknown()
                        }
                    }
                }
                // A raw pointer to a type parameter defers the member to the
                // instance, exactly as the bare parameter does. The audit
                // resolves the pointee and applies the same unsafe gate.
                TyKind::Param(_) => {
                    if let Some(function) = self.function_name.clone() {
                        self.opaque_projections.push(OpaqueProjection {
                            function,
                            ty: base_ty.clone(),
                            name: name.to_owned(),
                            in_unsafe: self.unsafe_depth > 0,
                            span,
                        });
                    }
                    Ty::unknown()
                }
                _ => {
                    self.report_unknown_member(base_ty, name, span);
                    Ty::unknown()
                }
            },
            // A member of an opaque base is resolved after substitution: the
            // instance decides whether the substituted type has the field, so
            // the read stays accepted here and the per-instance audit reports
            // the unknown member exactly as a concrete read would.
            TyKind::Param(_) => {
                if let Some(function) = self.function_name.clone() {
                    self.opaque_projections.push(OpaqueProjection {
                        function,
                        ty: base_ty.clone(),
                        name: name.to_owned(),
                        in_unsafe: self.unsafe_depth > 0,
                        span,
                    });
                }
                Ty::unknown()
            }
            // Every other concrete value — a scalar, a `cstring`, an owned
            // handle, a function value — has no member surface at all, so its
            // read reports the unknown-member rule. An unresolved base keeps
            // its unknown shape so a broken expression does not report twice.
            _ if !base_ty.is_unknown() => {
                self.report_unknown_member(base_ty, name, span);
                Ty::unknown()
            }
            _ => Ty::unknown(),
        }
    }

    /// Reports an unknown member on a concrete base.
    ///
    /// A field and a free function may share a spelling for a dot-call, so a
    /// name that is a free function is not reported; every other unknown
    /// member reports the registered rule with the base as written.
    fn report_unknown_member(&mut self, base_ty: &Ty, name: &str, span: Span) {
        self.error(
            "E10212",
            format!("'{name}' is not a member of '{}'", base_ty.display()),
            "check the declaration or the spelling",
            span,
        );
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
            // An index of an opaque base is judged after substitution: the
            // instance's row type supplies the extent for a constant index and
            // the run-time guard. The read stays accepted here and the
            // per-instance audit reports the out-of-range constant exactly as
            // a concrete read would.
            // An index of an opaque base is judged after substitution: the
            // instance's row type supplies the extent for a constant index and
            // the run-time guard. The read's own type is the base's one-step
            // element placeholder, so a chained index (`xs[i][j]`) judges its
            // inner base at the instance exactly as the outer one, and the
            // per-instance audit reports the out-of-range constant.
            TyKind::Param(name) => {
                if let Some(function) = self.function_name.clone() {
                    self.opaque_indexes.push(OpaqueIndex {
                        function,
                        ty: base_ty.clone(),
                        constant: self.folder.integer(index),
                        span: base.span,
                        index_span: index.span,
                        base: plain_binding_root(base),
                    });
                }
                Ty {
                    kind: TyKind::Param(crate::types::element_param(name)),
                    nullable: false,
                }
            }
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
        self.judge_conditional_argument(value, &target);
        // The null literal's shape is an unresolved pointer, not a scalar.
        // The open-type allowance in `scalar_cast_allowed` must not let it
        // reach an integer destination: the emitted cast would convert a
        // `void *` to an integer and fail `-Werror=pointer-to-int-cast`.
        if crate::types::is_null_shaped(&source) && target.is_bool() {
            self.report_null_literal(value.span);
        } else if crate::types::is_null_shaped(&source) {
            self.error(
                "E10201",
                format!("cannot cast 'null' to '{}'", target.display()),
                "convert through a supported value or `raw` in `unsafe`",
                value.span,
            );
        } else if !scalar_cast_allowed(&source, &target) {
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
        let source = self.check_value(value, None);
        // A box's pointee pointer is an internal escape hatch. Only the
        // standard-library module that implements `boxPtr` may cast a Box to a
        // raw pointer; application code uses `boxWith` so it cannot move the
        // value out or free the block by hand.
        if source.is_box() && !(self.module.embedded && self.module.name == "std.mem") {
            self.error(
                "E10201",
                format!(
                    "cannot cast '{}' to 'raw {}'",
                    source.display(),
                    target.display()
                ),
                "use `boxWith` to access the pointee",
                value.span,
            );
        }
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
            if self.builtin_box_active()
                && name
                    .segments
                    .last()
                    .is_some_and(|last| self.is_box_name(last))
            {
                self.error(
                    "E10203",
                    "struct literal for 'Box' does not match its fields",
                    "create a box with `boxNew`",
                    span,
                );
            }
            for field in fields {
                self.check_value(&field.value, None);
            }
            return Ty::unknown();
        };

        let mut resolved_args = Vec::with_capacity(type_args.len());
        for ty in type_args {
            resolved_args.push(self.resolve_type_argument(ty));
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
            // A field value is a move site of its own: the composite may sit
            // in a position no enclosing move covers (a condition, a
            // projection), and a capture carried into the field is consumed
            // there. The record dedupes per capture, so an enclosing site's
            // walk keeps this innermost span.
            self.record_generic_capture_move(&field.value);
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
    /// A fill form needs a fixed extent and a duplicable element: repeating a
    /// move-only value would alias one owner into every slot. A literal prefix
    /// may not be longer than the context extent.
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
                Some(element_ty) => self.check_into_slot(expr, element_ty),
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
            // An element value is a move site of its own: the literal may sit
            // in a position no enclosing move covers (a call argument, an
            // iterable), and a capture carried into the element is consumed
            // there.
            self.record_generic_capture_move(expr);
        }
        if let Some(fill_expr) = fill {
            match &element {
                Some(element_ty) => {
                    let value_ty = self.check_into_slot(fill_expr, element_ty);
                    if value_ty.is_closure() && !matches!(element_ty.kind, TyKind::Func(_)) {
                        self.reject_ephemeral_storage(&value_ty, fill_expr.span);
                    }
                }
                None => {
                    let value_ty = self.check_value(fill_expr, None);
                    self.reject_ephemeral_storage(&value_ty, fill_expr.span);
                }
            }
            // The fill copies its value into every element, so it is a move
            // site of the same kind as an element.
            self.record_generic_capture_move(fill_expr);
        }
        // An expected slice extent cannot type a fill-free literal: a literal's
        // shape is fixed, so a ``None`` outer extent takes the literal's own
        // element count and the conversion to the view target reports. A fill
        // keeps its own fixed-extent diagnostic instead, and the expected fixed
        // extents still carry into the literal's type.
        let mut extents = match expected_extents {
            Some(extents) if !extents.is_empty() => extents,
            _ if elements.is_empty() && fill.is_some() => vec![None],
            _ => vec![Some(elements.len())],
        };
        if fill.is_none() && extents.first() == Some(&None) {
            extents[0] = Some(elements.len());
        }
        Ty::array(
            result_element.or(element).unwrap_or_else(Ty::unknown),
            extents,
        )
    }

    /// Types a reserved built-in call. Only the modeled diagnostics are
    /// checked; the rest return the type their signature promises.
    fn check_intrinsic(&mut self, name: &str, args: &[IntrinsicArg], span: Span) -> Ty {
        match name {
            "sizeof" => {
                for arg in args {
                    if let IntrinsicArg::Type(ty) = arg {
                        let resolved = self.resolve_type(ty);
                        self.check_nullable_type(&resolved, ty.span);
                    }
                }
                Ty::usize_ty()
            }
            "offsetof" => {
                let mut base = None;
                let mut field = None;
                let mut reported = false;
                if let Some(IntrinsicArg::Type(ty)) = args.first() {
                    let resolved = self.resolve_type(ty);
                    self.check_nullable_type(&resolved, ty.span);
                    base = Some(resolved);
                }
                if let Some(IntrinsicArg::Expr(expr)) = args.get(1) {
                    if let ExprKind::Name(name) = &expr.kind {
                        // The field spelling is not a value read, but the
                        // ownership walk still asks for a recorded type at
                        // this node so it can visit it safely.
                        self.recorded
                            .push((expr as *const Expr as usize, Ty::unknown()));
                        field = Some((name.clone(), expr.span));
                    } else {
                        reported = true;
                        self.error(
                            "E10212",
                            "offsetof requires a field name",
                            "pass the field's identifier, not an expression",
                            expr.span,
                        );
                    }
                }
                match (base, field) {
                    (Some(ty), Some((name, span))) => {
                        // A bare type parameter defers the layout to its
                        // instance; the emitter substitutes the concrete
                        // struct and spells the same `offsetof` macro.
                        if !crate::types::contains_param(&ty)
                            && (!matches!(ty.kind, TyKind::Struct(..))
                                || self.field_type(&ty, &name).is_none())
                        {
                            self.error(
                                "E10212",
                                format!("'{name}' is not a member of '{}'", ty.display()),
                                "check the declaration or the spelling",
                                span,
                            );
                        }
                    }
                    _ if !reported => self.error(
                        "E10212",
                        "offsetof requires a struct type and a field name",
                        "write `offsetof(Type, field)`",
                        span,
                    ),
                    _ => {}
                }
                Ty::usize_ty()
            }
            "len" => {
                for arg in args {
                    if let IntrinsicArg::Expr(expr) = arg {
                        let ty = self.check_value(expr, None);
                        self.check_len_operand(&ty, expr.span);
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
            // Data inclusion is staged out, so the only implemented response
            // is the constant-context rule: the runtime positions report the
            // frozen diagnostic and no translation unit; the module `const`
            // position keeps the constant folder's own report.
            "embed" => {
                self.check_intrinsic_expr_args(args);
                if !self.in_const_context {
                    self.error(
                        "E10213",
                        "'embed' is only allowed in a constant context",
                        "use it in `const` or a module-level initializer",
                        span,
                    );
                }
                Ty::unknown()
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

    /// Requires a length-bearing operand, reporting `E10200` otherwise.
    ///
    /// The frozen length surface is an array or slice, a `str`, a `cstring`,
    /// the owned `String` handle, and the growable `Array<T>`. A scalar has no
    /// length, so the accepted scalar spelling cannot silently fold to zero.
    pub(super) fn check_len_operand(&mut self, ty: &Ty, span: Span) {
        if ty.is_unknown() || ty.is_param() {
            // A generic operand is judged at its instances.
            return;
        }
        if !crate::types::len_operand_capable(ty) {
            self.error(
                "E10200",
                format!("len is not defined for '{}'", ty.display()),
                "pass an array, slice, `str`, or `cstring`",
                span,
            );
        }
    }

    /// Requires a `bool` operand, reporting `E10218` otherwise.
    pub(super) fn require_bool(&mut self, ty: &Ty, span: Span) {
        // A null-shaped value has no boolean surface even though its kind is
        // unresolved; an ordinary unresolved or generic condition stays
        // deferred, as before.
        if ty.is_bool()
            || ty.is_param()
            || (ty.is_unknown() && !crate::types::is_null_shaped(ty))
        {
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

/// The member name of a value-form member read, when the expression is one.
///
/// The parser spells a dotted read `Qualified` and a postfix read `Member`, so
/// both shapes name the member; every other shape is not a member read.
pub(super) fn member_read_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Member { name, .. } => Some(name.clone()),
        ExprKind::Qualified(name) => name.segments.last().cloned(),
        _ => None,
    }
}

/// The written root name of a plain binding expression, when it is one.
///
/// An index base that is not a plain binding has no name the null rule could
/// print, exactly as the concrete spelling judges: only a binding's own
/// storage can be narrowed or may be null.
pub(super) fn plain_binding_root(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Name(name) => Some(name.clone()),
        // A dotted read is spelled as a qualified name; its head is the value
        // binding the rest of the path selects from, so the null rule names
        // that binding exactly as it names a postfix read's root.
        ExprKind::Qualified(name) => name.segments.first().cloned(),
        ExprKind::Member { base, .. }
        | ExprKind::Index { base, .. }
        | ExprKind::Unsafe(base) => plain_binding_root(base),
        _ => None,
    }
}

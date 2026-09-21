//! Place rules, view provenance, and IR lowering for the ownership stage.
//!
//! A place is read, written, or moved according to its binding kind and
//! its live loans, and a view carries the provenance of the storage it
//! points into so escape analysis can reason about it. The same methods
//! lower expressions and finalize the function intrinsic information.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    // -- place rules ---------------------------------------------------------

    /// Reports a read of a moved, partially moved, or uninitialized place.
    ///
    /// A read of a top-level field checks that field's own state, so the
    /// remaining fields of a partially moved or partially initialized struct
    /// stay usable.
    pub(super) fn read_place(&mut self, id: usize, projections: &[ir::Projection], span: Span) {
        self.bindings[id].used = true;
        let state = self.state.place(id);
        let name = self.bindings[id].name.clone();
        if state.moved {
            self.error(
                "E10020",
                format!("'{name}' was moved and cannot be used"),
                "re-initialize the binding or use a view",
                span,
            );
            return;
        }
        if let Some(ir::Projection::Field(field)) = projections.first() {
            if state.partial.contains(field) {
                self.error(
                    "E10020",
                    format!("'{name}.{field}' was moved and cannot be used"),
                    "re-initialize the field or use a view",
                    span,
                );
                return;
            }
            if !state.field_initialized(field) {
                self.error(
                    "E10021",
                    format!("'{name}.{field}' may be used before it is initialized"),
                    "initialize the field on every path",
                    span,
                );
            }
            return;
        }
        if !state.partial.is_empty() {
            self.error(
                "E10025",
                format!("'{name}' is partially moved and cannot be used as a whole"),
                "use the remaining fields or re-initialize the struct",
                span,
            );
            return;
        }
        if !state.initialized && !self.all_fields_initialized(id, &state) {
            self.error(
                "E10021",
                format!("'{name}' may be used before it is initialized"),
                "initialize the binding on every path; out-parameter slots are not exempt",
                span,
            );
        }
    }

    /// True when every declared top-level field of a struct binding is written.
    ///
    /// Field-by-field initialization never sets the whole-binding flag, but a
    /// struct whose fields are all written holds a complete value, so reading
    /// it as a whole is allowed. A non-struct, or one with an unknown field
    /// set, is never treated as complete this way.
    fn all_fields_initialized(&self, id: usize, state: &PlaceState) -> bool {
        let TyKind::Struct(name, _) = &self.bindings[id].ty.kind else {
            return false;
        };
        match self.sema.table.struct_fields(name) {
            Some(fields) => fields
                .iter()
                .all(|(field, _)| state.field_initialized(field)),
            None => false,
        }
    }

    /// Validates a write to a local place.
    ///
    /// A whole write re-initializes the binding. A field write re-initializes
    /// that field and clears its portion of the narrowing facts; writing a
    /// field of a wholly moved value is a use after move.
    pub(super) fn write_place(&mut self, id: usize, projections: &[ir::Projection], span: Span) {
        self.bindings[id].used = true;
        let binding = &self.bindings[id];
        if binding.is_const {
            let name = binding.name.clone();
            self.error(
                "E10023",
                format!("cannot assign to immutable '{name}'"),
                "declare it with `let` (mutable)",
                span,
            );
            return;
        }
        if self.is_readonly_param(id) {
            let name = binding.name.clone();
            self.error(
                "E10024",
                format!("cannot write through read-only parameter '{name}'"),
                "declare the parameter `borrow`",
                span,
            );
            return;
        }
        if self.state.loans.blocks_write(id) {
            let name = self.bindings[id].name.clone();
            self.error(
                "E10031",
                format!("cannot modify '{name}' while it is shared-borrowed"),
                "end the shared borrow first",
                span,
            );
            return;
        }
        if !projections.is_empty() && self.state.place(id).moved {
            let name = self.bindings[id].name.clone();
            self.error(
                "E10020",
                format!("'{name}' was moved and cannot be used"),
                "re-initialize the binding before writing a field",
                span,
            );
            return;
        }
        let first = projections.first().cloned();
        self.with_place(id, |place| {
            match &first {
                None => {
                    place.initialized = true;
                    place.initialized_fields.clear();
                    place.partial.clear();
                    place.moved = false;
                    place.narrowed.clear();
                }
                Some(ir::Projection::Field(field)) => {
                    if !place.initialized {
                        place.initialized_fields.insert(field.clone());
                    }
                    place.partial.remove(field);
                    place.moved = false;
                    place.narrowed.retain(|path| path.first() != first.as_ref());
                }
                Some(ir::Projection::Element(_)) => {
                    place.initialized = true;
                    place.moved = false;
                    place.narrowed.clear();
                }
            }
        });
    }

    /// True when the parameter is a read-only lend.
    pub(super) fn is_readonly_param(&self, id: usize) -> bool {
        let binding = &self.bindings[id];
        binding.kind == BindingKind::Param && matches!(binding.mode, Mode::None | Mode::Const)
    }

    /// Moves out of a place, applying partial-move and loan rules.
    pub(super) fn move_place(&mut self, id: usize, projections: &[ir::Projection], span: Span) {
        self.bindings[id].used = true;
        let binding = &self.bindings[id];
        if binding.kind == BindingKind::Param
            && matches!(binding.mode, Mode::None | Mode::Const | Mode::Borrow)
        {
            let name = binding.name.clone();
            self.error(
                "E10034",
                format!("cannot release or store borrowed parameter '{name}'"),
                "declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt",
                span,
            );
            return;
        }
        if self.state.loans.blocks_move(id) {
            let name = self.bindings[id].name.clone();
            self.error(
                "E10022",
                format!("cannot move '{name}' while it is borrowed"),
                "end the borrow first",
                span,
            );
            return;
        }
        if projections.is_empty() {
            // A read always precedes a move in the same expression, so a move
            // of an already-moved value has already been diagnosed; repeating
            // it here would double-report.
            self.with_place(id, |place| {
                place.moved = true;
                place.partial.clear();
                place.narrowed.clear();
            });
            return;
        }
        // A partial move is a single top-level field of an owned local or a
        // `take`/out parameter.
        let top_level = projections.len() == 1
            && matches!(projections[0], ir::Projection::Field(_));
        let owned = matches!(binding.kind, BindingKind::Local) || binding.mode == Mode::Take;
        if !top_level || !owned {
            self.error(
                "E10026",
                "partial moves are limited to top-level fields in v1",
                "move the whole value instead",
                span,
            );
            return;
        }
        let field = match &projections[0] {
            ir::Projection::Field(name) => name.clone(),
            ir::Projection::Element(_) => return,
        };
        self.with_place(id, |place| {
            place.partial.insert(field.clone());
            place.initialized_fields.remove(&field);
            place.narrowed.retain(|path| path.first() != projections.first());
        });
    }

    /// Moves out of an assignment or binding target.
    pub(super) fn move_target(&mut self, target: PlaceTarget, span: Span) {
        match target {
            PlaceTarget::Local(id, projections) => self.move_place(id, &projections, span),
            PlaceTarget::Global(name, _) => {
                self.error(
                    "E10033",
                    format!("borrow of '{name}' escapes its scope"),
                    "derive the view from a parameter or take ownership",
                    span,
                );
            }
        }
    }

    // -- provenance and lowering --------------------------------------------

    /// Where a view value's storage comes from.
    pub(super) fn view_provenance(&self, expr: &Expr) -> Provenance {
        match &expr.kind {
            ExprKind::Name(name) => self
                .lookup(name)
                .map(|id| self.place_provenance(id))
                .unwrap_or(Provenance::None),
            ExprKind::Qualified(name) => name
                .segments
                .first()
                .and_then(|first| self.lookup(first))
                .map(|id| self.place_provenance(id))
                .unwrap_or(Provenance::None),
            ExprKind::Str(_) => Provenance::Static,
            ExprKind::Unsafe(inner) => self.view_provenance(inner),
            // A raw or scalar cast keeps the storage of its operand, so a view
            // built from the cast still points into the same owner.
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.view_provenance(value)
            }
            ExprKind::Call { callee, args, .. } => self.call_provenance(callee, args),
            // A projection points into the same storage as its base, so the
            // provenance of `h.f` or `xs[0]` is the provenance of `h` or `xs`.
            ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
                self.view_provenance(base)
            }
            _ => Provenance::None,
        }
    }

    /// The loan kind a view-producing expression creates.
    ///
    /// A standard-library helper that yields a writable view is an exclusive
    /// loan; every other view derivation is a shared loan.
    pub(super) fn view_loan_kind(&self, expr: &Expr) -> LoanKind {
        match &expr.kind {
            ExprKind::Unsafe(inner) => self.view_loan_kind(inner),
            ExprKind::Call { callee, .. } => {
                let Some(name) = self.callee_name(callee) else {
                    return LoanKind::Shared;
                };
                match self.program.callee(&name) {
                    Some(Callee::Stdlib {
                        effect: StdlibEffect::ViewOf { kind, .. },
                        ..
                    }) => kind,
                    _ => LoanKind::Shared,
                }
            }
            _ => LoanKind::Shared,
        }
    }

    /// The provenance of a binding's storage.
    pub(super) fn place_provenance(&self, id: usize) -> Provenance {
        let binding = &self.bindings[id];
        if binding.provenance != Provenance::None {
            return binding.provenance.clone();
        }
        match binding.kind {
            BindingKind::Param | BindingKind::OutSlot => Provenance::Param(id),
            BindingKind::Local => Provenance::Local(id),
        }
    }

    /// The provenance of a call result that is a view.
    pub(super) fn call_provenance(&self, callee: &Expr, args: &[Arg]) -> Provenance {
        let Some(name) = self.callee_name(callee) else {
            return Provenance::None;
        };
        match self.program.callee(&name) {
            Some(Callee::User(decl)) => {
                let returns = resolve_ty(
                    &decl.returns,
                    &decl.type_params.iter().map(|p| p.name.clone()).collect::<Vec<_>>(),
                    &self.sema.table,
                );
                if !is_elided_view(&returns) {
                    return Provenance::None;
                }
                let candidates: Vec<usize> = decl
                    .params
                    .iter()
                    .enumerate()
                    .filter(|(_, param)| {
                        param.mode != Mode::Take
                            && is_lend_candidate(&resolve_ty(
                                &param.ty,
                                &decl
                                    .type_params
                                    .iter()
                                    .map(|p| p.name.clone())
                                    .collect::<Vec<_>>(),
                                &self.sema.table,
                            ))
                    })
                    .map(|(index, _)| index)
                    .collect();
                if candidates.len() == 1 {
                    return args
                        .get(candidates[0])
                        .map(|arg| self.view_provenance(&arg.value))
                        .unwrap_or(Provenance::None);
                }
                Provenance::None
            }
            Some(Callee::Stdlib {
                effect: StdlibEffect::ViewOf { arg, .. },
                ..
            }) => args
                .get(arg)
                .map(|written| self.view_provenance(&written.value))
                .unwrap_or(Provenance::None),
            _ => Provenance::None,
        }
    }

    /// True when a value expression is a loan that would escape if stored.
    ///
    /// Only a view value carries a loan; a scalar or owner read from a place
    /// copies or moves and never stores a reference to that place.
    pub(super) fn is_loan_value(&self, expr: &Expr) -> bool {
        if !is_view(&self.ty(expr)) {
            return false;
        }
        matches!(
            self.view_provenance(expr),
            Provenance::Local(_) | Provenance::Param(_) | Provenance::Global(_)
        )
    }

    /// The base binding and projections of a place expression.
    pub(super) fn place_target(&self, expr: &Expr) -> Option<PlaceTarget> {
        match &expr.kind {
            ExprKind::Name(name) => {
                if let Some(id) = self.lookup(name) {
                    Some(PlaceTarget::Local(id, Vec::new()))
                } else if self.program.globals.contains_key(name) {
                    Some(PlaceTarget::Global(name.clone(), Vec::new()))
                } else {
                    None
                }
            }
            ExprKind::Qualified(name) => {
                let first = name.segments.first()?;
                let mut projections = Vec::new();
                for segment in &name.segments[1..] {
                    projections.push(ir::Projection::Field(segment.clone()));
                }
                if let Some(id) = self.lookup(first) {
                    Some(PlaceTarget::Local(id, projections))
                } else if self.program.globals.contains_key(first) {
                    Some(PlaceTarget::Global(first.clone(), projections))
                } else {
                    None
                }
            }
            ExprKind::Member { base, name } => {
                let (target, mut projections) = self.place_target_parts(base)?;
                projections.push(ir::Projection::Field(name.clone()));
                Some(target.with_projections(projections))
            }
            ExprKind::Index { base, index } => {
                let (target, mut projections) = self.place_target_parts(base)?;
                projections.push(ir::Projection::Element(element_index(index)));
                Some(target.with_projections(projections))
            }
            ExprKind::Unsafe(inner) => self.place_target(inner),
            _ => None,
        }
    }

    /// The base target and its projections, for recursion.
    pub(super) fn place_target_parts(&self, expr: &Expr) -> Option<(PlaceTarget, Vec<ir::Projection>)> {
        let target = self.place_target(expr)?;
        let projections = match &target {
            PlaceTarget::Local(_, projections) | PlaceTarget::Global(_, projections) => {
                projections.clone()
            }
        };
        let base = match target {
            PlaceTarget::Local(id, _) => PlaceTarget::Local(id, Vec::new()),
            PlaceTarget::Global(name, _) => PlaceTarget::Global(name, Vec::new()),
        };
        Some((base, projections))
    }

    /// The base binding and projections of a plain (non-projecting) place.
    pub(super) fn plain_binding(&self, expr: &Expr) -> Option<(usize, Vec<ir::Projection>)> {
        match self.place_target(expr)? {
            PlaceTarget::Local(id, projections) => Some((id, projections)),
            PlaceTarget::Global(_, _) => None,
        }
    }

    /// Detects an `== null` / `!= null` test that narrows a place path.
    ///
    /// The narrowed path may be a projected place, so a nullable field can be
    /// narrowed without narrowing its owner.
    pub(super) fn narrowing(&self, cond: &Expr) -> Option<(usize, Vec<ir::Projection>, bool)> {
        let ExprKind::Binary { op, left, right } = &cond.kind else {
            return None;
        };
        let non_null = match op {
            BinaryOp::Ne => true,
            BinaryOp::Eq => false,
            _ => return None,
        };
        let name = match (&left.kind, &right.kind) {
            (ExprKind::Null, _) => right,
            (_, ExprKind::Null) => left,
            _ => return None,
        };
        let (id, projections) = self.plain_binding(name)?;
        // A run-time index names no stable path, so narrowing it would conflate
        // different elements; leave it unnarrowed and let the use be reported.
        if projections
            .iter()
            .any(|projection| matches!(projection, ir::Projection::Element(None)))
        {
            return None;
        }
        Some((id, projections, non_null))
    }

    /// Lowers an expression to a typed IR value.
    ///
    /// Every value records the type the type stage computed for its expression,
    /// and a form the checker does not need to name explicitly becomes a fresh
    /// temporary whose type joins the function's temporary list.
    pub(super) fn lower_value(&mut self, expr: &Expr) -> Value {
        let mut ty = self.ty(expr);
        let kind = match &expr.kind {
            ExprKind::Int(value) => ValueKind::Int(*value as i128),
            ExprKind::Float(value) => ValueKind::Float(*value),
            ExprKind::Char(byte) => ValueKind::Char(*byte),
            ExprKind::Bool(value) => ValueKind::Bool(*value),
            ExprKind::Null => ValueKind::Null,
            ExprKind::Str(bytes) => ValueKind::Str(String::from_utf8_lossy(bytes).into_owned()),
            ExprKind::Name(name) => match self.place_target(expr) {
                Some(target) => ValueKind::Place(self.target_place(&target)),
                None => ValueKind::Path(vec![name.clone()]),
            },
            ExprKind::Qualified(name) => match self.place_target(expr) {
                Some(target) => ValueKind::Place(self.target_place(&target)),
                None => ValueKind::Path(name.segments.clone()),
            },
            ExprKind::Member { base, name } => ValueKind::Member {
                base: Box::new(self.lower_value(base)),
                name: name.clone(),
            },
            ExprKind::Index { base, index } => ValueKind::Index {
                base: Box::new(self.lower_value(base)),
                index: Box::new(self.lower_value(index)),
            },
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                if let Some((intrinsic_ty, intrinsic_kind)) =
                    self.lower_intrinsic_call(callee, type_args, args)
                {
                    ty = intrinsic_ty;
                    intrinsic_kind
                } else {
                    let (name, resolved, lowered) = self.lower_call(callee, type_args, args);
                    ValueKind::Call {
                        callee: name,
                        type_args: resolved,
                        args: lowered,
                    }
                }
            }
            ExprKind::Unary { op, operand } => ValueKind::Unary {
                op: *op,
                operand: Box::new(self.lower_value(operand)),
            },
            ExprKind::Binary { op, left, right } => ValueKind::Binary {
                op: *op,
                left: Box::new(self.lower_value(left)),
                right: Box::new(self.lower_value(right)),
            },
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => ValueKind::Cast {
                to: ty.display(),
                value: Box::new(self.lower_value(value)),
            },
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => ValueKind::Conditional {
                condition: Box::new(self.lower_value(condition)),
                then_value: Box::new(self.lower_value(then_value)),
                else_value: Box::new(self.lower_value(else_value)),
            },
            ExprKind::StructLiteral { name, fields, .. } => ValueKind::StructLit {
                name: name.segments.last().cloned().unwrap_or_default(),
                fields: fields
                    .iter()
                    .map(|field| (field.name.clone(), self.lower_value(&field.value)))
                    .collect(),
            },
            ExprKind::ArrayLiteral { elements, fill } => ValueKind::ArrayLit {
                elements: elements.iter().map(|element| self.lower_value(element)).collect(),
                fill: fill
                    .as_ref()
                    .map(|fill| Box::new(self.lower_value(fill))),
            },
            ExprKind::Unsafe(inner) => return self.lower_value(inner),
            ExprKind::IntrinsicCall { name, args } => {
                let mut values = Vec::new();
                let mut type_arg = None;
                for arg in args {
                    match arg {
                        IntrinsicArg::Expr(inner) => values.push(self.lower_value(inner)),
                        IntrinsicArg::Type(written) => type_arg = Some(self.resolve(written)),
                    }
                }
                ValueKind::Intrinsic {
                    name: name.clone(),
                    args: values,
                    type_arg,
                }
            }
            ExprKind::Template(pieces) => ValueKind::Template(
                pieces
                    .iter()
                    .map(|piece| match piece {
                        TemplatePiece::Literal(bytes) => ValuePiece::Literal(
                            String::from_utf8_lossy(bytes).into_owned(),
                        ),
                        TemplatePiece::Placeholder(inner) => {
                            ValuePiece::Placeholder(self.lower_value(inner))
                        }
                    })
                    .collect(),
            ),
            _ => ValueKind::Aggregate,
        };
        Value { ty, kind }
    }

    // -- shared helpers ------------------------------------------------------

    /// Checks a declared type for an illegal `?` suffix.
    pub(super) fn check_nullable_decl(&mut self, ty: &Ty, span: Span) {
        if ty.nullable && !is_nullable_capable(ty) {
            let base = Ty {
                kind: ty.kind.clone(),
                nullable: false,
            };
            self.error(
                "E10011",
                format!("nullable type is not allowed on '{}'", base.display()),
                "`?` applies to handles, `raw`, `cstring`, and views",
                span,
            );
        }
    }

    /// Counts the view-like lend parameters for return elision.
    pub(super) fn elision_prelude(&mut self) {
        if !is_elided_view(&self.returns) {
            return;
        }
        self.elision_candidates = self
            .decl
            .params
            .iter()
            .filter(|param| {
                param.mode != Mode::Take
                    && is_lend_candidate(&resolve_ty(&param.ty, &self.type_params, &self.sema.table))
            })
            .count();
    }

    /// Reports `E10036` when a view return cannot elide a single parameter.
    pub(super) fn elision_postlude(&mut self) {
        if !is_elided_view(&self.returns) || self.suppress_elision {
            return;
        }
        if self.elision_candidates == 1 {
            return;
        }
        self.error(
            "E10036",
            "view return does not derive from exactly one parameter",
            "make exactly one parameter a view or handle lend",
            self.decl.name_span,
        );
    }

    /// Reports one unsafe-context diagnostic.
    pub(super) fn report_unsafe(&mut self, op: UnsafeOp, span: Span) {
        if !self.report_diagnostics {
            return;
        }
        self.diagnostics.push(
            Diagnostic::error(op.code(), op.message(), span).with_help(op.help()),
        );
    }

    /// Reports an error diagnostic.
    pub(super) fn error(&mut self, code: &'static str, message: impl Into<String>, help: &str, span: Span) {
        if !self.report_diagnostics {
            return;
        }
        self.diagnostics
            .push(Diagnostic::error(code, message, span).with_help(help));
    }

    /// Reports a warning diagnostic.
    pub(super) fn warn(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        if !self.report_diagnostics {
            return;
        }
        self.diagnostics
            .push(Diagnostic::warning(code, message, span));
    }
}

/// The constant index of an element expression, when it is a literal.
fn element_index(expr: &Expr) -> Option<usize> {
    match &expr.kind {
        ExprKind::Int(value) => usize::try_from(*value).ok(),
        _ => None,
    }
}


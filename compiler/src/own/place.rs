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
    pub(super) fn read_place(&mut self, id: usize, span: Span) {
        let state = self.state.place(id);
        if state.moved {
            let name = self.bindings[id].name.clone();
            self.error(
                "E10020",
                format!("'{name}' was moved and cannot be used"),
                "re-initialize the binding or use a view",
                span,
            );
            return;
        }
        if !state.partial.is_empty() {
            let name = self.bindings[id].name.clone();
            self.error(
                "E10025",
                format!("'{name}' is partially moved and cannot be used as a whole"),
                "use the remaining fields or re-initialize the struct",
                span,
            );
            return;
        }
        if !state.initialized {
            let name = self.bindings[id].name.clone();
            self.error(
                "E10021",
                format!("'{name}' may be used before it is initialized"),
                "initialize the binding on every path; out-parameter slots are not exempt",
                span,
            );
        }
    }

    /// Validates a write to a local place.
    pub(super) fn write_place(&mut self, id: usize, whole: bool, span: Span) {
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
        self.with_place(id, |place| {
            place.initialized = true;
            place.moved = false;
            if whole {
                place.partial.clear();
            }
            place.narrowed = false;
        });
    }

    /// True when the parameter is a read-only lend.
    pub(super) fn is_readonly_param(&self, id: usize) -> bool {
        let binding = &self.bindings[id];
        binding.kind == BindingKind::Param && matches!(binding.mode, Mode::None | Mode::Const)
    }

    /// Moves out of a place, applying partial-move and loan rules.
    pub(super) fn move_place(&mut self, id: usize, projections: &[ir::Projection], span: Span) {
        let binding = &self.bindings[id];
        if binding.kind == BindingKind::Param && binding.mode == Mode::Borrow {
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
            if self.state.place(id).moved {
                let name = self.bindings[id].name.clone();
                self.error(
                    "E10020",
                    format!("'{name}' was moved and cannot be used"),
                    "re-initialize the binding or use a view",
                    span,
                );
            }
            self.with_place(id, |place| {
                place.moved = true;
                place.partial.clear();
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
            place.partial.insert(field);
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
            ExprKind::Call { callee, args, .. } => self.call_provenance(callee, args),
            _ => Provenance::None,
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
    pub(super) fn is_loan_value(&self, expr: &Expr) -> bool {
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

    /// Detects an `== null` / `!= null` test that narrows a place.
    pub(super) fn narrowing(&self, cond: &Expr) -> Option<(usize, bool)> {
        let ExprKind::Binary { op, left, right } = &cond.kind else {
            return None;
        };
        let non_null = match op {
            BinaryOp::Ne => true,
            BinaryOp::Eq => false,
            _ => return None,
        };
        let (name, other) = match (&left.kind, &right.kind) {
            (ExprKind::Null, _) => (right, left),
            (_, ExprKind::Null) => (left, right),
            _ => return None,
        };
        let _ = other;
        let (id, projections) = self.plain_binding(name)?;
        if !projections.is_empty() {
            return None;
        }
        Some((id, non_null))
    }

    /// Lowers an expression to an IR value.
    pub(super) fn lower_value(&self, expr: &Expr) -> Value {
        match &expr.kind {
            ExprKind::Int(value) => Value::Int(*value as i128),
            ExprKind::Bool(value) => Value::Bool(*value),
            ExprKind::Null => Value::Null,
            ExprKind::Str(bytes) => Value::Str(String::from_utf8_lossy(bytes).into_owned()),
            ExprKind::Name(_)
            | ExprKind::Qualified(_)
            | ExprKind::Member { .. }
            | ExprKind::Index { .. } => match self.place_target(expr) {
                Some(target) => Value::Place(self.target_place(&target)),
                None => {
                    let temp = self.next_temp.get();
                    self.next_temp.set(temp + 1);
                    Value::Temp(temp)
                }
            },
            ExprKind::Call { callee, args, .. } => Value::Call {
                callee: self.callee_name(callee).unwrap_or_default(),
                args: args.iter().map(|arg| self.lower_value(&arg.value)).collect(),
            },
            ExprKind::Unary { op, operand } => Value::Unary {
                op: *op,
                operand: Box::new(self.lower_value(operand)),
            },
            ExprKind::Binary { op, left, right } => Value::Binary {
                op: *op,
                left: Box::new(self.lower_value(left)),
                right: Box::new(self.lower_value(right)),
            },
            ExprKind::Cast { ty, value } | ExprKind::RawCast { ty, value } => Value::Cast {
                to: self.resolve(ty).display(),
                value: Box::new(self.lower_value(value)),
            },
            _ => Value::Aggregate,
        }
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
        self.diagnostics.push(
            Diagnostic::error(op.code(), op.message(), span).with_help(op.help()),
        );
    }

    /// Reports an error diagnostic.
    pub(super) fn error(&mut self, code: &'static str, message: impl Into<String>, help: &str, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span).with_help(help));
    }
}

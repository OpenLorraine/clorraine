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
        for loan in self.binding_loans(id) {
            self.record_event(LoanEventKind::Used { loan }, span);
        }
        let state = self.state.place(id);
        let name = self.bindings[id].name.clone();
        // A later use after a store that only moves for an owning instance
        // records the duplicability duty; a duplicable instance stays accepted.
        self.note_maybe_moved_use(id, projections, span);
        if state.moved {
            self.report_state_once(
                id,
                projections,
                span,
                "E10020",
                format!("'{name}' was moved and cannot be used"),
                "re-initialize the binding or use a view",
            );
            return;
        }
        if let Some(ir::Projection::Field(field)) = projections.first() {
            if state.partial.contains(field) {
                self.report_state_once(
                    id,
                    projections,
                    span,
                    "E10020",
                    format!("'{name}.{field}' was moved and cannot be used"),
                    "re-initialize the field or use a view",
                );
                return;
            }
            if !state.field_initialized(field) {
                self.report_state_once(
                    id,
                    projections,
                    span,
                    "E10021",
                    format!("'{name}.{field}' may be used before it is initialized"),
                    "initialize the field on every path",
                );
            }
            return;
        }
        if !state.partial.is_empty() {
            self.report_state_once(
                id,
                projections,
                span,
                "E10025",
                format!("'{name}' is partially moved and cannot be used as a whole"),
                "use the remaining fields or re-initialize the struct",
            );
            return;
        }
        if !state.initialized && !self.all_fields_initialized(id, &state) {
            self.report_state_once(
                id,
                projections,
                span,
                "E10021",
                format!("'{name}' may be used before it is initialized"),
                "initialize the binding on every path; out-parameter slots are not exempt",
            );
        }
    }

    /// Reports a place-state error at most once per site.
    ///
    /// The site is the binding, the span's start, and the code: reads of one
    /// site share the span's start even when they select different projections
    /// (an element read also reads its base), and the update paths read a
    /// place twice and write it again, so one site would otherwise repeat the
    /// same state error. The first report at a site emits and later reports
    /// for the same site are suppressed — unless a later report selects more
    /// projections, which is the more specific text for the same site and
    /// replaces the whole-place one already emitted. A report the speculative
    /// pass would suppress leaves the site free for the reporting pass.
    fn report_state_once(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        span: Span,
        code: &'static str,
        message: impl Into<String>,
        help: &str,
    ) {
        if !self.report_diagnostics {
            return;
        }
        let site = (id, span.start, code);
        let rank = projections.len();
        match self.state_reports.get(&site).copied() {
            None => {
                let index = self.diagnostics.items().len();
                self.state_reports.insert(site, (index, rank));
                self.error(code, message, help, span);
            }
            Some((index, recorded)) => {
                if rank > recorded {
                    self.diagnostics.replace_at(
                        index,
                        Diagnostic::error(code, message, span).with_help(help),
                    );
                    self.state_reports.insert(site, (index, rank));
                }
            }
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
            self.report_state_once(
                id,
                projections,
                span,
                "E10023",
                format!("cannot assign to immutable '{name}'"),
                "declare it with `let` (mutable)",
            );
            return;
        }
        if self.is_readonly_param(id) {
            let name = binding.name.clone();
            self.report_state_once(
                id,
                projections,
                span,
                "E10024",
                format!("cannot write through read-only parameter '{name}'"),
                "declare the parameter `borrow`",
            );
            return;
        }
        // An element write through a view of immutable bytes (`str`,
        // `static str`) is rejected here, after the `const` and read-only-mode
        // reports above, so each rejected write keeps exactly one diagnostic.
        if self.check_immutable_view_write(id, projections, span) {
            return;
        }
        // A write through a view is a use of every loan the view carries; a
        // whole write of the view binding is a rebind, so `bind_value` updates
        // its loans instead. A write to a value that is not a view is an owner
        // access, decided against the loan regions in the post-pass.
        if is_view(&self.bindings[id].ty) {
            if !projections.is_empty() {
                for loan in self.binding_loans(id) {
                    self.record_event(LoanEventKind::Used { loan }, span);
                }
            }
        } else {
            self.record_event(
                LoanEventKind::Access {
                    owner: LoanOwner::Binding(id),
                    access: AccessKind::Write,
                },
                span,
            );
        }
        if !projections.is_empty() && self.state.place(id).moved {
            let name = self.bindings[id].name.clone();
            self.report_state_once(
                id,
                projections,
                span,
                "E10020",
                format!("'{name}' was moved and cannot be used"),
                "re-initialize the binding before writing a field",
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
                    place.moved_if_owning.clear();
                    place.narrowed.clear();
                }
                Some(ir::Projection::Field(field)) => {
                    if !place.initialized {
                        place.initialized_fields.insert(field.clone());
                    }
                    place.partial.remove(field);
                    place.moved = false;
                    place.moved_if_owning
                        .retain(|path| path.first() != first.as_ref());
                    place.narrowed.retain(|path| path.first() != first.as_ref());
                }
                Some(ir::Projection::Element(_)) | Some(ir::Projection::ElementTemp(_)) => {
                    place.initialized = true;
                    place.moved = false;
                    // Only the written element and its sub-paths are cleared;
                    // a whole-binding consume or another element stays. A null
                    // narrowing is a fact about the root, and an element write
                    // cannot change it, so it survives the write exactly as it
                    // survives a field write — the guarded block keeps the
                    // narrowed value usable (TY-6).
                    place
                        .moved_if_owning
                        .retain(|path| path.first() != first.as_ref());
                    place
                        .narrowed
                        .retain(|path| path.first() != first.as_ref());
                }
            }
        });
    }

    /// True when the parameter is a read-only lend.
    pub(super) fn is_readonly_param(&self, id: usize) -> bool {
        let binding = &self.bindings[id];
        binding.kind == BindingKind::Param && matches!(binding.mode, Mode::None | Mode::Const)
    }

    /// Reports `E10042` when a local write ends in an element of an immutable
    /// view.
    ///
    /// A `str` or `static str` view names storage the language never lets a
    /// write reach: the bytes may be a literal's static storage or another
    /// owner's memory under a shared loan. The check looks at the type the
    /// final element selection is applied to, so a parameter, local, field,
    /// or module-level view base is caught alike. A whole-view rebind and a
    /// write through a writable (borrow-derived) slice are not element writes
    /// and stay accepted. Returns `true` when the write was reported, so the
    /// caller can stop before any state changes.
    pub(super) fn check_immutable_view_write(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        span: Span,
    ) -> bool {
        let ty = self.bindings[id].ty.clone();
        let name = self.bindings[id].name.clone();
        self.report_immutable_view_write(&ty, &name, projections, span)
    }

    /// Reports `E10042` for a module-level write with the same predicate.
    ///
    /// A module binding is writable storage by default, but its declared type
    /// still decides whether an element write can reach the bytes.
    pub(super) fn check_global_immutable_view_write(
        &mut self,
        id: &ir::GlobalId,
        projections: &[ir::Projection],
        span: Span,
    ) -> bool {
        let Some(global) = self.program.globals.get(id) else {
            return false;
        };
        let ty = global.ty.clone();
        let name = id.name.clone();
        self.report_immutable_view_write(&ty, &name, projections, span)
    }

    /// The shared immutable-view predicate and its report.
    ///
    /// The rule fires only when the final projection selects an element and
    /// the type that selection is applied to is `str`, `static str`, or
    /// `cstring` — the view types whose byte storage is immutable, or, for
    /// `cstring`, may name a literal's read-only bytes. The message names the
    /// binding, or `binding.field` when the selection's base ends in a field
    /// projection.
    fn report_immutable_view_write(
        &mut self,
        ty: &Ty,
        name: &str,
        projections: &[ir::Projection],
        span: Span,
    ) -> bool {
        let Some((last, prefix)) = projections.split_last() else {
            return false;
        };
        if !matches!(
            last,
            ir::Projection::Element(_) | ir::Projection::ElementTemp(_)
        ) {
            return false;
        }
        let prefix_ty = self.projection_ty(ty, prefix);
        if !matches!(
            prefix_ty.kind,
            TyKind::Str | TyKind::StaticStr | TyKind::CString
        ) {
            return false;
        }
        let view = match prefix.last() {
            Some(ir::Projection::Field(field)) => format!("{name}.{field}"),
            _ => name.to_owned(),
        };
        self.error(
            "E10042",
            format!("cannot write through immutable view '{view}'"),
            "copy the bytes into an owned `String` or a `borrow`-derived `u8[]` view",
            span,
        );
        true
    }

    /// The type a projection path selects when applied to `ty`.
    ///
    /// Field steps substitute the instance's type arguments and reach through
    /// a raw pointer to the struct it points at; an element step descends one
    /// extent exactly as an element access types it (a `str` byte is `u8`,
    /// and a raw pointer reaches its pointee). A step the type cannot resolve
    /// stays unknown, so no immutable-view judgment rides on an unresolved
    /// type.
    fn projection_ty(&self, ty: &Ty, projections: &[ir::Projection]) -> Ty {
        let mut current = ty.clone();
        for projection in projections {
            let next = match projection {
                ir::Projection::Field(field) => {
                    let mut base = &current;
                    while let TyKind::Raw(inner) = &base.kind {
                        base = inner;
                    }
                    self.sema
                        .table
                        .instance_field(base, field)
                        .unwrap_or_else(Ty::unknown)
                }
                ir::Projection::Element(_) | ir::Projection::ElementTemp(_) => {
                    match &current.kind {
                        TyKind::Array { .. } => current.iterated_element(),
                        TyKind::Str | TyKind::CString | TyKind::StaticStr => {
                            Ty::scalar(ScalarType::U8)
                        }
                        TyKind::Raw(inner) => (**inner).clone(),
                        _ => Ty::unknown(),
                    }
                }
            };
            current = next;
        }
        current
    }

    /// Moves out of a place, applying partial-move and loan rules.
    pub(super) fn move_place(&mut self, id: usize, projections: &[ir::Projection], span: Span) {
        self.bindings[id].used = true;
        let binding = &self.bindings[id];
        // A borrow element view never owns its element, so an ownership
        // transfer through it is the v1 element-move limit even though the
        // view binds as a local. (The borrowed-parameter rule below covers
        // parameters.)
        if binding.kind == BindingKind::Local && binding.mode == Mode::Borrow {
            self.report_element_move_limit(span);
            return;
        }
        if binding.kind == BindingKind::Param
            && matches!(binding.mode, Mode::None | Mode::Const | Mode::Borrow)
            // A by-value capture owns its environment field, so moving it (or
            // a field of it) is judged by the capture-move rule, exactly as a
            // whole move is; the lend-storage report would name it a borrowed
            // parameter of the enclosing function.
            && !is_by_value_capture_base(&binding.base)
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
        let owner_kind = binding.kind;
        let owner_mode = binding.mode;
        let owns_environment_field = is_by_value_capture_base(&binding.base);
        self.record_event(
            LoanEventKind::Access {
                owner: LoanOwner::Binding(id),
                access: AccessKind::Move,
            },
            span,
        );
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
        // A partial move is a single top-level field of an owned local, a
        // `take`/out parameter, or a by-value capture, which owns its
        // environment field the way a local does.
        let top_level = projections.len() == 1
            && matches!(projections[0], ir::Projection::Field(_));
        let owned = matches!(owner_kind, BindingKind::Local)
            || owner_mode == Mode::Take
            || owns_environment_field;
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
            ir::Projection::Element(_) | ir::Projection::ElementTemp(_) => return,
        };
        self.with_place(id, |place| {
            place.partial.insert(field.clone());
            place.initialized_fields.remove(&field);
            place.narrowed.retain(|path| path.first() != projections.first());
        });
    }

    /// Records the duplicability duty when a use reaches a consumed
    /// maybe-owning path.
    ///
    /// Shared by the read funnel and the borrow-argument check, which reads a
    /// place without the full read walk. A whole use conflicts with any
    /// consumed path; a projected use conflicts with a consumed path it
    /// contains or is contained by.
    pub(super) fn note_maybe_moved_use(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        span: Span,
    ) {
        let state = self.state.place(id);
        if state.moved_if_owning.is_empty() {
            return;
        }
        if projections.is_empty()
            || state.moved_if_owning.iter().any(|path| {
                path.is_empty()
                    || projections.starts_with(path)
                    || path.starts_with(projections)
            })
        {
            self.record_requires_duplicable(id, span);
        }
    }

    /// Records a value-consuming move of a maybe-owning place.
    ///
    /// The flow keeps its copy semantics for a type that mentions a parameter,
    /// so the store is written as a move only when the instance's type argument
    /// owns. A later read of the source deposits the duplicability duty through
    /// [`Self::record_requires_duplicable`]. A move whose root is a lend
    /// parameter additionally records a duty the per-instance audit judges
    /// against the substituted type: moving a lend into an owned position is
    /// legal only for a duplicable instance, and the `borrow T?` out-param slot
    /// stays exempt. A by-value lambda capture is exempt from that duty at
    /// every site — including a return and a `take` argument — because the
    /// capture-move rule judges it instead.
    pub(super) fn mark_moved_if_owning(&mut self, target: PlaceTarget, span: Span) {
        self.mark_moved_if_owning_at(target, span, false);
    }

    /// Records a maybe-owning move whose destination is a writable parameter
    /// slot.
    ///
    /// The destination is the `borrow T?` out-slot carve-out for a nullable
    /// instance, so the per-instance audit exempts a nullable substitution
    /// only for these moves; every other destination keeps the strict
    /// judgment.
    pub(super) fn mark_moved_into_slot_if_owning(&mut self, target: PlaceTarget, span: Span) {
        self.mark_moved_if_owning_at(target, span, true);
    }

    /// Records a value-consuming move of a maybe-owning place.
    ///
    /// `stores_into_slot` flags a writable-parameter-slot destination for the
    /// out-slot carve-out.
    fn mark_moved_if_owning_at(&mut self, target: PlaceTarget, span: Span, stores_into_slot: bool) {
        if let PlaceTarget::Local(id, projections) = target {
            self.with_place(id, |place| {
                place.moved_if_owning.insert(projections.clone());
            });
            let binding = &self.bindings[id];
            // A lend whose value is moved into an owned position owes the
            // post-substitution judgment. Out-parameter slots carry their own
            // binding kind and are not lend moves.
            let lend = binding.kind == BindingKind::Param
                && matches!(binding.mode, Mode::None | Mode::Const | Mode::Borrow);
            if !lend {
                return;
            }
            // A by-value capture is judged by the capture-move rule after
            // substitution (`E10401`), which the semantic stage records at
            // every move site — the return, a `take` argument, a `let` or
            // assignment target, and a composite field value — so the lend
            // duty would only add the wrong report here.
            if is_by_value_capture_base(&binding.base) {
                return;
            }
            let writable_source = binding.mode == Mode::Borrow;
            let parameter = binding.name.clone();
            let ir_name = binding.ir_name.clone();
            let ty = if projections.is_empty() {
                binding.ty.clone()
            } else {
                self.projected_type(id, &projections)
            };
            if ty.is_unknown() || !self.recorded_not_lend.insert((ir_name, span.start, span.end)) {
                return;
            }
            self.not_lend_requirements.push(ir::RequiresNotLend {
                function: self.decl.name.clone(),
                parameter,
                ty,
                writable_source,
                stores_into_slot,
                span,
            });
        }
    }

    /// Records the duplicability duty a body owes for one binding, at most once.
    ///
    /// Only a binding whose declared type mentions a type parameter carries the
    /// duty; the per-instance audit reports when the substituted type owns.
    pub(super) fn record_requires_duplicable(&mut self, id: usize, span: Span) {
        if !self.recorded_requirements.insert(id) {
            return;
        }
        if !self.maybe_owns(&self.bindings[id].ty) {
            return;
        }
        let ir_name = self.bindings[id].ir_name.clone();
        let declared = self.bindings[id].ty.clone();
        self.duplicable_requirements.push(ir::RequiresDuplicable {
            function: self.decl.name.clone(),
            binding: ir_name,
            ty: declared,
            span,
        });
    }

    /// Moves out of an assignment or binding target.
    pub(super) fn move_target(&mut self, target: PlaceTarget, span: Span) {
        match target {
            PlaceTarget::Local(id, projections) => self.move_place(id, &projections, span),
            PlaceTarget::Global(id, _) => {
                self.error(
                    "E10033",
                    format!("borrow of '{}' escapes its scope", id.name),
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
            // A conditional's storage is the chosen arm's storage, so the
            // provenance is shared only when both arms agree.
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                let then = self.view_provenance(then_value);
                let otherwise = self.view_provenance(else_value);
                if then == otherwise {
                    then
                } else {
                    Provenance::None
                }
            }
            // An array literal's storage is its elements' storage, so the
            // result is a view of the elements' common source; a mixed or
            // ownerless element list has no single source. The fill form's
            // value is every remaining element's storage.
            ExprKind::ArrayLiteral { elements, fill } => {
                let mut provenance: Option<Provenance> = None;
                let values = elements.iter().chain(fill.iter().map(|fill| fill.as_ref()));
                for element in values {
                    let current = self.view_provenance(element);
                    match &provenance {
                        None => provenance = Some(current),
                        Some(known) if *known == current => {}
                        Some(_) => return Provenance::None,
                    }
                }
                provenance.unwrap_or(Provenance::None)
            }
            _ => Provenance::None,
        }
    }

    /// The loans a binding may carry at this point.
    ///
    /// The set is empty for a binding that carries no loan, and holds more
    /// than one loan after a join where the paths gave the binding different
    /// owners.
    pub(super) fn binding_loans(&self, id: usize) -> BTreeSet<LoanId> {
        self.state.loans.get(&id).cloned().unwrap_or_default()
    }

    /// Replaces the loans a binding carries.
    pub(super) fn set_binding_loans(&mut self, id: usize, loans: BTreeSet<LoanId>) {
        if loans.is_empty() {
            self.state.loans.remove(&id);
        } else {
            self.state.loans.insert(id, loans);
        }
    }

    /// The loans a view value carries at this point.
    ///
    /// Mirrors [`Self::view_provenance`] over the loan chain: a name or a
    /// projection inherits the loans of its base binding, a view-returning call
    /// inherits the loans at its single elision candidate, and an owner, a
    /// literal, or a fresh view derivation carries none. A conditional shares
    /// the loans of both arms, because either arm's storage may be chosen.
    pub(super) fn view_loans(&self, expr: &Expr) -> BTreeSet<LoanId> {
        match &expr.kind {
            ExprKind::Name(name) => self
                .lookup(name)
                .map(|id| self.binding_loans(id))
                .unwrap_or_default(),
            ExprKind::Qualified(name) => name
                .segments
                .first()
                .and_then(|first| self.lookup(first))
                .map(|id| self.binding_loans(id))
                .unwrap_or_default(),
            ExprKind::Unsafe(inner) => self.view_loans(inner),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.view_loans(value)
            }
            ExprKind::Call { callee, args, .. } => self.call_loans(callee, args),
            ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
                self.view_loans(base)
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                let mut loans = self.view_loans(then_value);
                loans.extend(self.view_loans(else_value));
                loans
            }
            _ => BTreeSet::new(),
        }
    }

    /// The owners a view value's storage derives from.
    ///
    /// The set combines the source expression's own owners (one for a plain
    /// derivation, the union of both arms for a conditional) with every
    /// statement temporary the lowered value reads through, including a
    /// projection carried through a reborrow call. A view binding creates one
    /// loan per owner, so the same liveness rule protects storage owned by a
    /// binding and storage owned by a statement temporary.
    pub(super) fn view_owners(
        &self,
        expr: &Expr,
        lowered: Option<&Value>,
    ) -> BTreeSet<LoanOwner> {
        let mut owners = BTreeSet::new();
        self.collect_view_owners(expr, &mut owners);
        if let Some(lowered) = lowered {
            for temp in self.lowered_temp_owners(lowered) {
                owners.insert(LoanOwner::Temp(temp));
            }
            // A chain the store re-rooted at a bound owner reads through that
            // local, so the view carries a loan of it exactly as it would for a
            // chain that named the place in source. A view array reads through
            // every element's root, so each contributes a loan.
            let mut roots = BTreeSet::new();
            self.lowered_root_bindings(lowered, &mut roots);
            for id in roots {
                owners.insert(LoanOwner::Binding(id));
            }
        }
        owners
    }

    /// The bindings a lowered projection chain roots at, when its roots are
    /// local places.
    ///
    /// A view derivation the store rebinding re-rooted reads through the owner
    /// local that binding declared, so the view must carry a loan of it; the
    /// liveness pass then treats the owner's cleanup as the release the view's
    /// uses depend on, exactly as it does for a view of a named place. A chain
    /// that roots at anything else — a temporary, a module global, or fresh
    /// storage — names no binding here. A view array contributes one root per
    /// element and fill value.
    fn lowered_root_bindings(&self, value: &Value, roots: &mut BTreeSet<usize>) {
        match &value.kind {
            ValueKind::Place(place) => {
                if let ir::PlaceBase::Local(name) = &place.base {
                    if let Some(id) = self
                        .bindings
                        .iter()
                        .position(|binding| binding.ir_name == *name)
                    {
                        roots.insert(id);
                    }
                }
            }
            ValueKind::Index { base, .. } | ValueKind::Member { base, .. } => {
                self.lowered_root_bindings(base, roots);
            }
            ValueKind::Cast { value: inner, .. } => self.lowered_root_bindings(inner, roots),
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.lowered_root_bindings(element, roots);
                }
                if let Some(fill) = fill {
                    self.lowered_root_bindings(fill, roots);
                }
            }
            _ => {}
        }
    }

    /// Collects the owners the source expression derives from, arm by arm.
    ///
    /// A conditional contributes every arm's owners, and a projection points
    /// into the same storage as its base, so the walk descends a member or
    /// element chain to its base before reading the provenance. A conditional
    /// under a projection therefore still contributes each arm's owner of the
    /// storage the projection reads.
    fn collect_view_owners(&self, expr: &Expr, owners: &mut BTreeSet<LoanOwner>) {
        match &expr.kind {
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.collect_view_owners(then_value, owners);
                self.collect_view_owners(else_value, owners);
            }
            ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
                self.collect_view_owners(base, owners);
            }
            ExprKind::Unsafe(inner) => self.collect_view_owners(inner, owners),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.collect_view_owners(value, owners);
            }
            // Every element of a view array contributes its own owners, so a
            // binding loans each element's backing independently.
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.collect_view_owners(element, owners);
                }
                if let Some(fill) = fill {
                    self.collect_view_owners(fill, owners);
                }
            }
            _ => {
                if let Some(owner) = owner_of(&self.view_provenance(expr)) {
                    owners.insert(owner);
                }
            }
        }
    }

    /// The loans a view produced by a call carries.
    ///
    /// A view-returning call with exactly one elision candidate is a reborrow
    /// of that candidate's loans; every other call yields a fresh view whose
    /// loan the caller creates.
    pub(super) fn call_loans(&self, callee: &Expr, args: &[Arg]) -> BTreeSet<LoanId> {
        let Some(name) = self.callee_name(callee) else {
            return BTreeSet::new();
        };
        let is_dot = self.is_dot_call(callee);
        let source_loans = |index: usize| -> BTreeSet<LoanId> {
            if is_dot && index == 0 {
                return self.dot_receiver_loans(callee);
            }
            let written_index = if is_dot { index - 1 } else { index };
            args.get(written_index)
                .map(|arg| self.view_loans(&arg.value))
                .unwrap_or_default()
        };
        match self.program.callee(&name) {
            Some(Callee::User(decl)) => {
                let type_params: Vec<String> =
                    decl.type_params.iter().map(|p| p.name.clone()).collect();
                let returns = self.resolve_with(&decl.returns, &type_params);
                if !is_elided_view(&returns) {
                    return BTreeSet::new();
                }
                let candidates: Vec<usize> = decl
                    .params
                    .iter()
                    .enumerate()
                    .filter(|(_, param)| {
                        param.mode != Mode::Take
                            && is_lend_candidate(&self.resolve_with(&param.ty, &type_params))
                    })
                    .map(|(index, _)| index)
                    .collect();
                if candidates.len() == 1 {
                    source_loans(candidates[0])
                } else {
                    BTreeSet::new()
                }
            }
            Some(Callee::Stdlib {
                effect: StdlibEffect::ViewOf { arg, .. },
                ..
            }) => source_loans(arg),
            _ => BTreeSet::new(),
        }
    }

    /// The loans a dot-call receiver's view value carries.
    fn dot_receiver_loans(&self, callee: &Expr) -> BTreeSet<LoanId> {
        match &callee.kind {
            ExprKind::Member { base, .. } => self.view_loans(base),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => name
                .segments
                .first()
                .and_then(|first| self.lookup(first))
                .map(|id| self.binding_loans(id))
                .unwrap_or_default(),
            _ => BTreeSet::new(),
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

    /// The provenance of a view produced by a call.
    ///
    /// A dot-call's receiver is its first parameter but is not a written
    /// argument, so the parameter index mapping accounts for it: index zero is
    /// the receiver and every later index is one past the written argument.
    pub(super) fn call_provenance(&self, callee: &Expr, args: &[Arg]) -> Provenance {
        let Some(name) = self.callee_name(callee) else {
            return Provenance::None;
        };
        let is_dot = self.is_dot_call(callee);
        // The provenance of the source at a parameter index.
        let source_provenance = |index: usize| -> Provenance {
            if is_dot && index == 0 {
                return self.dot_receiver_provenance(callee);
            }
            let written_index = if is_dot { index - 1 } else { index };
            args.get(written_index)
                .map(|arg| self.view_provenance(&arg.value))
                .unwrap_or(Provenance::None)
        };
        match self.program.callee(&name) {
            Some(Callee::User(decl)) => {
                let type_params: Vec<String> =
                    decl.type_params.iter().map(|p| p.name.clone()).collect();
                let returns = self.resolve_with(&decl.returns, &type_params);
                if !is_elided_view(&returns) {
                    return Provenance::None;
                }
                let candidates: Vec<usize> = decl
                    .params
                    .iter()
                    .enumerate()
                    .filter(|(_, param)| {
                        param.mode != Mode::Take
                            && is_lend_candidate(&self.resolve_with(&param.ty, &type_params))
                    })
                    .map(|(index, _)| index)
                    .collect();
                if candidates.len() == 1 {
                    return source_provenance(candidates[0]);
                }
                Provenance::None
            }
            Some(Callee::Stdlib {
                effect: StdlibEffect::ViewOf { arg, .. },
                ..
            }) => source_provenance(arg),
            _ => Provenance::None,
        }
    }

    /// The provenance of a dot-call receiver's storage.
    ///
    /// A `Member` receiver is read back through [`Self::view_provenance`]; a
    /// qualified receiver names the head binding, so its own place provenance
    /// is used.
    fn dot_receiver_provenance(&self, callee: &Expr) -> Provenance {
        match &callee.kind {
            ExprKind::Member { base, .. } => self.view_provenance(base),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => name
                .segments
                .first()
                .and_then(|first| self.lookup(first))
                .map(|id| self.place_provenance(id))
                .unwrap_or(Provenance::None),
            _ => Provenance::None,
        }
    }

    /// True when a value expression is a loan that would escape if stored.
    ///
    /// Only a view-like value carries a loan: a view, or a fixed array whose
    /// element is a view. A scalar or owner read from a place copies or moves
    /// and never stores a reference to that place.
    pub(super) fn is_loan_value(&self, expr: &Expr) -> bool {
        if !is_view_like(&self.ty(expr)) {
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
                } else if let Some(global) =
                    crate::init_graph::resolve_name_global(self.loaded, self.module, name)
                {
                    Some(PlaceTarget::Global(global_id(global), Vec::new()))
                } else {
                    None
                }
            }
            ExprKind::Qualified(name) => {
                // A fixed array's `.len` is its structural extent, not a
                // place: the length is a compile-time constant and the base
                // reads no storage.
                if self.qualified_structural_len(name).is_some() {
                    return None;
                }
                let first = name.segments.first()?;
                // A local base keeps its own binding; every later segment is a
                // field projection.
                if let Some(id) = self.lookup(first) {
                    let projections = name.segments[1..]
                        .iter()
                        .map(|segment| ir::Projection::Field(segment.clone()))
                        .collect();
                    return Some(PlaceTarget::Local(id, projections));
                }
                // Otherwise the name is a module-level binding: a same-module
                // or imported head with field projections, or a full module
                // path whose last segment is the binding.
                let global = crate::init_graph::resolve_global(self.loaded, self.module, name)?;
                let projections = global
                    .fields
                    .iter()
                    .map(|segment| ir::Projection::Field(segment.clone()))
                    .collect();
                Some(PlaceTarget::Global(global_id(global), projections))
            }
            ExprKind::Member { base, name } => {
                // A fixed array's `.len` is its structural extent, not a
                // place: it names no storage and its base reads nothing.
                if name == "len" && self.ty(base).is_fixed_array() {
                    return None;
                }
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

    /// The structural extent a qualified `.len` path names, when it names one.
    ///
    /// A dotted name whose last segment is `len` and whose base resolves to a
    /// fixed array is the array's compile-time length: the path names no
    /// storage and its base reads nothing, exactly as the `len` intrinsic
    /// folds the same extent.
    pub(super) fn qualified_structural_len(
        &self,
        name: &crate::ast::QualifiedName,
    ) -> Option<usize> {
        let (first, tail) = name.segments.split_first()?;
        let (last, middle) = tail.split_last()?;
        if last != "len" {
            return None;
        }
        let mut ty = if let Some(id) = self.lookup(first) {
            self.bindings[id].ty.clone()
        } else {
            let global = crate::init_graph::resolve_name_global(self.loaded, self.module, first)?;
            self.program
                .globals
                .get(&global_id(global))
                .map(|global| global.ty.clone())?
        };
        for segment in middle {
            ty = self.member_type(&ty, segment);
        }
        match &ty.kind {
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some) => {
                extents.first().and_then(|size| *size)
            }
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
            PlaceTarget::Global(id, _) => PlaceTarget::Global(id, Vec::new()),
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

    /// The identity of the module-level binding a bare name resolves to.
    ///
    /// A same-module declaration wins, then an imported binding, mirroring how
    /// the name resolver reads a simple name. The module travels with the name,
    /// so a same-named global in another module is never mistaken for the one
    /// in scope.
    pub(super) fn resolve_global_id(&self, name: &str) -> Option<ir::GlobalId> {
        crate::init_graph::resolve_name_global(self.loaded, self.module, name).map(global_id)
    }

    /// The checked type of the module-level binding a bare name resolves to.
    pub(super) fn global_type(&self, name: &str) -> Option<Ty> {
        let id = self.resolve_global_id(name)?;
        self.program.globals.get(&id).map(|global| global.ty.clone())
    }

    /// The declared type of the whole module binding a value callee resolves
    /// to.
    ///
    /// A bare name resolves like any simple name — the same module first, then
    /// an import. A qualified name resolves its value path: a same-module or
    /// imported head with field projections, or a full module path whose last
    /// segment names the binding. The declared signature types the call in
    /// every spelling; a path into a field is not a whole binding and yields
    /// `None`.
    pub(super) fn value_callee_global_type(&self, callee: &Expr) -> Option<Ty> {
        let id = match &callee.kind {
            ExprKind::Name(written) => self.resolve_global_id(written)?,
            ExprKind::Qualified(name) => {
                let global = crate::init_graph::resolve_global(self.loaded, self.module, name)?;
                if !global.fields.is_empty() {
                    return None;
                }
                global_id(global)
            }
            _ => return None,
        };
        self.program.globals.get(&id).map(|global| global.ty.clone())
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
        if projections.iter().any(|projection| {
            matches!(
                projection,
                ir::Projection::Element(None) | ir::Projection::ElementTemp(_)
            )
        }) {
            return None;
        }
        Some((id, projections, non_null))
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
    ///
    /// The name of the single candidate is kept so the postlude can compare
    /// the return's actual derivation against it: a view return must derive
    /// from exactly one view-like lend parameter, not merely count one.
    pub(super) fn elision_prelude(&mut self) {
        if !is_elided_view(&self.returns) {
            return;
        }
        let candidates: Vec<String> = self
            .decl
            .params
            .iter()
            .filter(|param| {
                param.mode != Mode::Take
                    && is_lend_candidate(&self.resolve_with(&param.ty, &self.type_params))
            })
            .map(|param| param.name.clone())
            .collect();
        self.elision_candidates = candidates.len();
        self.elision_candidate = if candidates.len() == 1 {
            candidates.into_iter().next()
        } else {
            None
        };
    }

    /// Reports `E10036` when a view return cannot elide a single parameter.
    ///
    /// The count decides the zero-candidate and two-or-more cases; with
    /// exactly one candidate the return's observed derivation must name it,
    /// so a view read through a different parameter is rejected here at the
    /// declaration instead of leaking a borrowed view to the caller.
    pub(super) fn elision_postlude(&mut self) {
        if !is_elided_view(&self.returns) || self.suppress_elision {
            return;
        }
        if self.elision_candidates == 1 && !self.return_derivation_mismatch {
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

/// True when a place base overrides to a by-value lambda capture.
///
/// A lifted lambda binds its captures at environment fields: a by-value
/// capture owns its field, while a `[borrow x]` capture points at the
/// enclosing frame. Such a capture's storage is the environment, not a
/// parameter, so the lend-storage rules must not name it a borrowed parameter.
fn is_by_value_capture_base(base: &Option<ir::PlaceBase>) -> bool {
    matches!(
        base,
        Some(ir::PlaceBase::EnvField {
            by_pointer: false,
            ..
        })
    )
}

/// The identity of a module-level binding the name resolver resolved.
fn global_id(global: crate::init_graph::GlobalRef) -> ir::GlobalId {
    ir::GlobalId::new(global.module, global.name)
}

/// The loan owner a provenance names, when it is a real owner.
///
/// Static storage and an unowned value name no owner, so they carry no loan.
fn owner_of(provenance: &Provenance) -> Option<LoanOwner> {
    match provenance {
        Provenance::Local(id) | Provenance::Param(id) => Some(LoanOwner::Binding(*id)),
        Provenance::Temp(temp) => Some(LoanOwner::Temp(*temp)),
        _ => None,
    }
}

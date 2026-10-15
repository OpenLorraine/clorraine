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
            ExprKind::Name(name) => match self.lookup(name) {
                Some(id) => self.place_provenance(id),
                // A name that resolves to a module-level binding is storage
                // that outlives every frame, exactly like a literal.
                None => crate::init_graph::resolve_name_global(self.loaded, self.module, name)
                    .map(|global| Provenance::Global(global.name))
                    .unwrap_or(Provenance::None),
            },
            ExprKind::Qualified(name) => {
                if let Some(id) = name.segments.first().and_then(|first| self.lookup(first)) {
                    return self.place_provenance(id);
                }
                crate::init_graph::resolve_global(self.loaded, self.module, name)
                    .map(|global| Provenance::Global(global.name))
                    .unwrap_or(Provenance::None)
            }
            ExprKind::Str(_) => Provenance::Static,
            ExprKind::Unsafe(inner) => self.view_provenance(inner),
            // A raw or scalar cast keeps the storage of its operand, so a view
            // built from the cast still points into the same owner.
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.view_provenance(value)
            }
            ExprKind::Call { callee, args, .. } => {
                let provenance = self.call_provenance(callee, args);
                // A call whose declared result is `static str` names static
                // storage, which outlives every frame exactly like a literal;
                // the result has no lend source to reborrow, so the call
                // provenance alone cannot see it.
                if provenance == Provenance::None
                    && matches!(self.ty(expr).kind, TyKind::StaticStr)
                {
                    Provenance::Static
                } else {
                    provenance
                }
            }
            // A projection points into the same storage as its base, so the
            // provenance of `h.f` or `xs[0]` is the provenance of `h` or `xs`.
            ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
                self.view_provenance(base)
            }
            // A conditional's storage is the chosen arm's storage. An arm
            // that names no storage (`null`) contributes nothing; a local
            // arm keeps its provenance even when the other arm differs,
            // because the chosen value then points into a frame that dies at
            // the return, exactly as a plain local view does; naming the
            // local lets the escape and loan rules judge the mixed spelling.
            // Every other mix keeps its components, so a destination judges
            // each arm against its own accept set.
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                let mut atoms = Vec::new();
                for arm in [then_value, else_value] {
                    if matches!(arm.kind, ExprKind::Null) {
                        continue;
                    }
                    atoms.push(self.view_provenance(arm));
                }
                merge_view_provenances(atoms, true)
            }
            // An array literal's storage is its elements' storage, so the
            // result is a view of the elements' common source; a mixed or
            // ownerless element list keeps every component, so a destination
            // judges each element. The fill form's value is every remaining
            // element's storage.
            ExprKind::ArrayLiteral { elements, fill } => {
                let mut atoms = Vec::new();
                for element in elements {
                    if matches!(element.kind, ExprKind::Null) {
                        continue;
                    }
                    atoms.push(self.view_provenance(element));
                }
                if let Some(fill) = fill {
                    if !matches!(fill.kind, ExprKind::Null) {
                        atoms.push(self.view_provenance(fill));
                    }
                }
                merge_view_provenances(atoms, false)
            }
            _ => Provenance::None,
        }
    }

    /// The storage components a provenance stands for.
    ///
    /// A plain provenance is its own single component; a composite contributes
    /// each of its components, so a destination can judge every arm or element
    /// on its own. An empty composite names no storage at all — the scalar
    /// `null` and a composite whose arms are all null — so it contributes no
    /// component: a destination cannot keep storage that does not exist, and
    /// every judgment that walks the components accepts it.
    pub(super) fn storage_atoms(provenance: &Provenance) -> Vec<Provenance> {
        match provenance {
            Provenance::Composite(atoms) => atoms
                .iter()
                .flat_map(|atom| Self::storage_atoms(atom))
                .collect(),
            atom => vec![atom.clone()],
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

    /// The owners of the loans a binding carries.
    ///
    /// A carrier binding's loans are exactly the backings its view-like
    /// positions read, so the owners name every storage an escape of the
    /// binding would hand out.
    pub(super) fn binding_loan_owners(&self, id: usize) -> BTreeSet<LoanOwner> {
        let mut owners = BTreeSet::new();
        self.loan_owners(&self.binding_loans(id), &mut owners);
        owners
    }

    /// Adds the owners a loan set names.
    fn loan_owners(&self, loans: &BTreeSet<LoanId>, owners: &mut BTreeSet<LoanOwner>) {
        for loan in loans {
            for (origin, candidate) in &self.loan_ids {
                if candidate == loan {
                    owners.insert(origin.owner);
                }
            }
        }
    }

    /// The owners a returned carrier's view positions read that die with the
    /// callee frame.
    ///
    /// A carrier binding's loans name every backing its view-like positions
    /// read, so returning the binding hands those views to the caller exactly
    /// as a direct view return does and the loans are created at the return.
    /// Only an owner whose storage dies with this frame makes that return
    /// dangle: a plain or `borrow` parameter's storage belongs to the caller,
    /// which outlives the return (EL-2), so its loan stays on the binding for
    /// a module destination without rejecting the return. A view-like return
    /// keeps its own rule, and a value that carries no views contributes
    /// nothing.
    pub(super) fn carrier_return_owners(
        &self,
        lowered: &Value,
        returns: &Ty,
    ) -> BTreeSet<LoanOwner> {
        let mut owners = BTreeSet::new();
        if is_view_like(returns) || !self.may_carry_view(returns) {
            return owners;
        }
        let mut roots = BTreeSet::new();
        self.lowered_root_bindings(lowered, &mut roots);
        for id in roots {
            for owner in self.binding_loan_owners(id) {
                if self.owner_dies_with_frame(owner) {
                    owners.insert(owner);
                }
            }
        }
        owners
    }

    /// True when a loaned owner's storage dies with the callee frame.
    ///
    /// A local's storage (including a compiler-generated owner) dies with the
    /// frame, and a `take` parameter's storage is this frame's too. A plain,
    /// `borrow`, or out-slot parameter names the caller's storage, which the
    /// return leaves alive. A statement temporary dies at the end of its
    /// statement, well before the caller reads a returned view.
    pub(super) fn owner_dies_with_frame(&self, owner: LoanOwner) -> bool {
        match owner {
            LoanOwner::Binding(id) => {
                let binding = &self.bindings[id];
                match binding.kind {
                    BindingKind::Local => true,
                    BindingKind::Param => binding.mode == Mode::Take,
                    BindingKind::OutSlot => false,
                }
            }
            LoanOwner::Temp(_) => true,
        }
    }

    /// The frame-dying owners a view value's binding loans name.
    ///
    /// A container binding's element loans name the storage its view positions
    /// read, exactly as a carrier's loans do. The binding's recorded
    /// provenance describes the value that first filled it, which a later
    /// element store does not update, so a destination that hands the value's
    /// views out — a return or an escaping store — consults the loans as well.
    /// Only storage that dies with this frame makes the escape dangle; a
    /// parameter-backed element keeps its backing in the caller's frame.
    pub(super) fn view_frame_loan_owners(&self, expr: &Expr) -> BTreeSet<usize> {
        let mut owners = BTreeSet::new();
        let mut roots = BTreeSet::new();
        self.expr_view_roots(expr, &mut roots);
        for id in roots {
            for owner in self.binding_loan_owners(id) {
                if let LoanOwner::Binding(owner_id) = owner {
                    if self.owner_dies_with_frame(owner) {
                        owners.insert(owner_id);
                    }
                }
            }
        }
        owners
    }

    /// The owners a place's recorded view positions name.
    ///
    /// A store that reads a place reads the storage that place's positions
    /// hold, so a destination that copies the value loans the same owners the
    /// source position loans: a constant projection path judges the positions
    /// it selects — the path itself, the positions below it, and a fill that
    /// covers it — while a whole-binding or run-time-index read names no
    /// single position and contributes every recorded one. `None` when the
    /// place roots at no binding with recorded positions.
    pub(super) fn place_recorded_position_owners(
        &self,
        expr: &Expr,
    ) -> Option<BTreeSet<LoanOwner>> {
        let PlaceTarget::Local(id, projections) = self.place_target(expr)? else {
            return None;
        };
        let id = self.alias_resolved_root(id);
        let paths = self.state.element_loans.get(&id)?;
        let read = projections
            .iter()
            .all(|projection| {
                !matches!(
                    projection,
                    ir::Projection::Element(None) | ir::Projection::ElementTemp(_)
                )
            })
            .then_some(projections);
        let mut owners = BTreeSet::new();
        match read {
            Some(read) => {
                for (path, loans) in paths {
                    if path.starts_with(&read) || read.starts_with(path) {
                        self.loan_owners(loans, &mut owners);
                    }
                }
            }
            None => {
                for loans in paths.values() {
                    self.loan_owners(loans, &mut owners);
                }
            }
        }
        Some(owners)
    }

    /// The loan owners a view value's binding reads that a destination cannot
    /// keep.
    ///
    /// A caller slot keeps the caller's own parameter storage, so only a
    /// frame-dying backing reports there; a global outlives every caller, so
    /// every loaned owner reports. Static and module storage carry no loan, so
    /// they never appear here.
    pub(super) fn view_loan_destination_owners(
        &self,
        expr: &Expr,
        global: bool,
    ) -> BTreeSet<LoanOwner> {
        let mut owners = BTreeSet::new();
        let mut roots = BTreeSet::new();
        self.expr_view_roots(expr, &mut roots);
        for id in roots {
            for owner in self.binding_loan_owners(id) {
                if global || self.owner_dies_with_frame(owner) {
                    owners.insert(owner);
                }
            }
        }
        owners
    }

    /// The frame-dying owners a view value's recorded positions name.
    ///
    /// A binding's recorded view positions name the storage its views read, so
    /// the positions are authoritative where they exist: a constant projection
    /// path judges exactly the positions it selects — the path itself, the
    /// positions below it, and a fill that covers it — while a whole-binding
    /// or run-time-index read names no single position and judges every
    /// recorded one. A binding with no recorded position falls back to the
    /// value's own provenance at the caller.
    pub(super) fn recorded_position_frame_owners(
        &self,
        expr: &Expr,
    ) -> Option<BTreeSet<LoanOwner>> {
        let mut roots = BTreeSet::new();
        self.expr_view_roots(expr, &mut roots);
        let mut found = false;
        let mut owners = BTreeSet::new();
        for id in roots {
            let Some(paths) = self.state.element_loans.get(&id) else {
                continue;
            };
            found = true;
            let read = match self.place_target(expr) {
                Some(PlaceTarget::Local(root, projections)) if root == id => {
                    let constant = projections.iter().all(|projection| {
                        !matches!(
                            projection,
                            ir::Projection::Element(None) | ir::Projection::ElementTemp(_)
                        )
                    });
                    constant.then_some(projections)
                }
                _ => None,
            };
            let Some(read) = read else {
                for loans in paths.values() {
                    self.loan_owners(loans, &mut owners);
                }
                continue;
            };
            for (path, loans) in paths {
                if path.starts_with(&read) || read.starts_with(path) {
                    self.loan_owners(loans, &mut owners);
                }
            }
        }
        found.then(|| {
            owners
                .into_iter()
                .filter(|owner| self.owner_dies_with_frame(*owner))
                .collect()
        })
    }

    /// The bindings a view expression's projection chain roots at.
    ///
    /// Mirrors [`Self::lowered_root_bindings`] over the source expression: a
    /// name or a qualified head names its binding, a projection or a cast
    /// names its base's, and a conditional or an array contributes every arm
    /// or element. A `for (borrow e in xs)` alias has no storage of its own,
    /// so its place reaches the iterable element and the walk follows it to
    /// the container exactly as the element-loan records do. A chain that
    /// roots at anything else — a call result, a literal, or module storage —
    /// names no binding here.
    fn expr_view_roots(&self, expr: &Expr, roots: &mut BTreeSet<usize>) {
        match &expr.kind {
            ExprKind::Name(name) => {
                if let Some(id) = self.lookup(name) {
                    roots.insert(self.alias_resolved_root(id));
                }
            }
            ExprKind::Qualified(name) => {
                if let Some(id) = name.segments.first().and_then(|first| self.lookup(first)) {
                    roots.insert(self.alias_resolved_root(id));
                }
            }
            ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
                self.expr_view_roots(base, roots);
            }
            ExprKind::Unsafe(inner) => self.expr_view_roots(inner, roots),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.expr_view_roots(value, roots);
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.expr_view_roots(then_value, roots);
                self.expr_view_roots(else_value, roots);
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.expr_view_roots(element, roots);
                }
                if let Some(fill) = fill {
                    self.expr_view_roots(fill, roots);
                }
            }
            _ => {}
        }
    }

    /// The binding a root reaches when it is a `for` alias.
    ///
    /// An alias binding has no storage of its own: its place reaches the
    /// iterable element, so every owner walk follows the alias chain to the
    /// container exactly as the element-loan records do.
    fn alias_resolved_root(&self, mut id: usize) -> usize {
        while let Some(PlaceTarget::Local(root, _)) = self.bindings[id].alias_root {
            if root == id {
                break;
            }
            id = root;
        }
        id
    }

    /// The container binding an alias place reaches, when it is one.
    ///
    /// A `for (borrow e in xs)` alias has no storage of its own: its place
    /// reaches the iterable element, so the storage its views read is the
    /// container's.
    pub(super) fn alias_container_binding(&self, expr: &Expr) -> Option<usize> {
        let id = match &expr.kind {
            ExprKind::Name(name) => self.lookup(name)?,
            _ => return None,
        };
        let resolved = self.alias_resolved_root(id);
        (resolved != id).then_some(resolved)
    }

    /// Replaces the loans a binding carries.
    ///
    /// A whole-binding store replaces every element the binding held, so the
    /// per-element record is dropped with the union it fed.
    pub(super) fn set_binding_loans(&mut self, id: usize, loans: BTreeSet<LoanId>) {
        self.state.element_loans.remove(&id);
        self.set_binding_loan_union(id, loans);
    }

    /// Replaces the loans one element path contributes to a container.
    ///
    /// Overwriting an element drops the loans of the value it replaced, so a
    /// stale backing stops blocking its owner's writes. The overwrite replaces
    /// the value at the path *and* everything below it: a field's view
    /// positions record under their own field path, so a field overwrite drops
    /// every position the field held. A loan another element path also carries
    /// stays, because that element still reads it. A path with a run-time index
    /// cannot name the element it overwrote, so it keeps the union and only
    /// adds.
    pub(super) fn replace_element_loans(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        loans: BTreeSet<LoanId>,
    ) {
        let constant = projections.iter().all(|projection| {
            !matches!(
                projection,
                ir::Projection::Element(None) | ir::Projection::ElementTemp(_)
            )
        });
        if !constant {
            let mut union = self.binding_loans(id);
            union.extend(loans.iter().copied());
            self.set_binding_loan_union(id, union);
            return;
        }
        let path = projections.to_vec();
        let previous: Vec<BTreeSet<LoanId>> = match self.state.element_loans.get_mut(&id) {
            Some(paths) => {
                let replaced: Vec<Vec<ir::Projection>> = paths
                    .keys()
                    .filter(|key| key.starts_with(&path))
                    .cloned()
                    .collect();
                replaced
                    .into_iter()
                    .filter_map(|key| paths.remove(&key))
                    .collect()
            }
            None => Vec::new(),
        };
        let mut union = self.binding_loans(id);
        for replaced in &previous {
            for loan in replaced {
                let shared = self
                    .state
                    .element_loans
                    .get(&id)
                    .is_some_and(|paths| paths.values().any(|set| set.contains(loan)));
                if !shared {
                    union.remove(loan);
                }
            }
        }
        union.extend(loans.iter().copied());
        if loans.is_empty() {
            let empty = self
                .state
                .element_loans
                .get(&id)
                .is_some_and(|paths| paths.is_empty());
            if empty {
                self.state.element_loans.remove(&id);
            }
        } else {
            self.state
                .element_loans
                .entry(id)
                .or_default()
                .insert(path, loans);
        }
        self.set_binding_loan_union(id, union);
    }

    /// Moves a carrier's recorded view positions onto another binding.
    ///
    /// A carrier copy (`h2 = h1`) hands the destination the same views the
    /// source holds, so every recorded position contributes its loans under
    /// the destination's path: the copy's views read the same storage, and a
    /// later escape reports through the destination exactly as through the
    /// source. The destination's own record under `base` is replaced first —
    /// the store replaced those positions — and a self-assignment keeps its
    /// positions because the source's record is captured before the
    /// replacement.
    pub(super) fn move_element_loans(&mut self, from: usize, to: usize, base: &[ir::Projection]) {
        let snapshot: Vec<(Vec<ir::Projection>, BTreeSet<LoanId>)> = self
            .state
            .element_loans
            .get(&from)
            .map(|paths| {
                paths
                    .iter()
                    .map(|(path, loans)| (path.clone(), loans.clone()))
                    .collect()
            })
            .unwrap_or_default();
        self.replace_element_loans(to, base, BTreeSet::new());
        for (path, loans) in snapshot {
            let mut full = base.to_vec();
            full.extend(path);
            self.replace_element_loans(to, &full, loans);
        }
    }

    /// Writes the union of loans a binding carries.
    fn set_binding_loan_union(&mut self, id: usize, loans: BTreeSet<LoanId>) {
        if loans.is_empty() {
            self.state.loans.remove(&id);
        } else {
            self.state.loans.insert(id, loans);
        }
    }

    /// The adopted view backings a binding's stored value carries.
    ///
    /// The set is empty for a binding whose value reads no adopted backing,
    /// and holds every generated owner or statement temporary a view-like
    /// position of the value reads.
    pub(super) fn binding_carried_owners(&self, id: usize) -> BTreeSet<LoanOwner> {
        self.state
            .carried_owners
            .get(&id)
            .cloned()
            .unwrap_or_default()
    }

    /// Replaces the adopted view backings a binding's stored value carries.
    pub(super) fn set_binding_carried_owners(&mut self, id: usize, owners: BTreeSet<LoanOwner>) {
        if owners.is_empty() {
            self.state.carried_owners.remove(&id);
        } else {
            self.state.carried_owners.insert(id, owners);
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
            // A view array's elements each contribute their loans: a call
            // that returns the array's view reborrows every element source.
            // The binding effect below does not use this union directly — an
            // element list carries independent loans, so it records one loan
            // per owner instead — but the call path reads it to carry the
            // reborrow of the elements' sources.
            ExprKind::ArrayLiteral { elements, fill } => {
                let mut loans = BTreeSet::new();
                for element in elements {
                    loans.extend(self.view_loans(element));
                }
                if let Some(fill) = fill {
                    loans.extend(self.view_loans(fill));
                }
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
            // A view-returning call reborrows the storage of the single lend
            // candidate it elides, so the walk descends into that argument
            // exactly as a direct derivation does: a conditional argument
            // contributes every arm's owners. A call with no single candidate
            // contributes none.
            ExprKind::Call { callee, args, .. } => {
                match self.call_view_source(callee, args) {
                    Some(ViewCallSource::Expr(source)) => {
                        self.collect_view_owners(source, owners);
                    }
                    Some(ViewCallSource::Receiver(provenance)) => {
                        if let Some(owner) = owner_of(&provenance) {
                            owners.insert(owner);
                        }
                    }
                    None => {}
                }
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
                // A mixed value contributes every component's owner, so a
                // frame binding among module-level or literal components
                // still carries its loan.
                for atom in FunctionChecker::storage_atoms(&self.view_provenance(expr)) {
                    if let Some(owner) = owner_of(&atom) {
                        owners.insert(owner);
                    }
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
    /// The call resolves its single reborrowed source through
    /// [`Self::call_view_source`]; the view's provenance is that source's.
    pub(super) fn call_provenance(&self, callee: &Expr, args: &[Arg]) -> Provenance {
        match self.call_view_source(callee, args) {
            Some(ViewCallSource::Expr(source)) => self.view_provenance(source),
            Some(ViewCallSource::Receiver(provenance)) => provenance,
            None => Provenance::None,
        }
    }

    /// True when a view expression derives from a conditional whose arms
    /// disagree.
    ///
    /// The provenance rule keeps a local-backed arm's provenance when the
    /// other arm differs, so the return's local-owner judgment would name the
    /// local even though the value may read the other arm's freshly derived
    /// storage. A value that derives from a single local — a projection of a
    /// carrier binding, a plain local view, or a call that reborrows one — is
    /// not mixed and keeps the local-owner report.
    pub(super) fn view_is_mixed(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.view_provenance(then_value) != self.view_provenance(else_value)
                    || self.view_is_mixed(then_value)
                    || self.view_is_mixed(else_value)
            }
            ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
                self.view_is_mixed(base)
            }
            // A view array repeats the borrows of the single view it names, so
            // a mixed element makes the array spelling mixed too.
            ExprKind::ArrayLiteral { elements, fill } => {
                elements.iter().any(|element| self.view_is_mixed(element))
                    || fill.as_deref().is_some_and(|fill| self.view_is_mixed(fill))
            }
            ExprKind::Unsafe(inner) => self.view_is_mixed(inner),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.view_is_mixed(value)
            }
            // A view-returning call reborrows its single lend source, so a
            // chain through the call is as mixed as the source itself.
            ExprKind::Call { callee, args, .. } => match self.call_view_source(callee, args) {
                Some(ViewCallSource::Expr(source)) => self.view_is_mixed(source),
                _ => false,
            },
            _ => false,
        }
    }

    /// The source a view-returning call reborrows, when it elides exactly one.
    ///
    /// A user callee with exactly one view-like lend parameter reborrows that
    /// argument; a standard-library `ViewOf` effect names its source argument
    /// directly. A dot call's receiver is its first parameter but is not a
    /// written argument, so a `Member` receiver yields its base expression and
    /// a qualified receiver — which names a place rather than an expression —
    /// yields its provenance. Every other call yields none: without a single
    /// reborrowed source there is no storage a view could derive from.
    fn call_view_source<'e>(&self, callee: &'e Expr, args: &'e [Arg]) -> Option<ViewCallSource<'e>> {
        let name = self.callee_name(callee)?;
        let is_dot = self.is_dot_call(callee);
        let source = |index: usize| -> Option<ViewCallSource<'e>> {
            if is_dot && index == 0 {
                return match &callee.kind {
                    ExprKind::Member { base, .. } => Some(ViewCallSource::Expr(base)),
                    ExprKind::Qualified(_) => Some(ViewCallSource::Receiver(
                        self.dot_receiver_provenance(callee),
                    )),
                    _ => None,
                };
            }
            let written_index = if is_dot { index - 1 } else { index };
            args.get(written_index)
                .map(|arg| ViewCallSource::Expr(&arg.value))
        };
        match self.program.callee(&name) {
            Some(Callee::User(decl)) => {
                let type_params: Vec<String> =
                    decl.type_params.iter().map(|p| p.name.clone()).collect();
                let returns = self.resolve_with(&decl.returns, &type_params);
                if !is_elided_view(&returns) {
                    return None;
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
                    source(candidates[0])
                } else {
                    None
                }
            }
            Some(Callee::Stdlib {
                effect: StdlibEffect::ViewOf { arg, .. },
                ..
            }) => source(arg),
            _ => None,
        }
    }

    /// The provenance of a dot-call receiver's storage.
    ///
    /// A `Member` receiver is read back through [`Self::view_provenance`]; a
    /// qualified receiver names the head binding, so its own place provenance
    /// is used, and a head that resolves to module storage names that storage.
    fn dot_receiver_provenance(&self, callee: &Expr) -> Provenance {
        match &callee.kind {
            ExprKind::Member { base, .. } => self.view_provenance(base),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                if let Some(id) = name.segments.first().and_then(|first| self.lookup(first)) {
                    return self.place_provenance(id);
                }
                crate::init_graph::resolve_global(self.loaded, self.module, name)
                    .map(|global| Provenance::Global(global.name))
                    .unwrap_or(Provenance::None)
            }
            _ => Provenance::None,
        }
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

    /// The storage a place ultimately reaches when it may name an element
    /// alias.
    ///
    /// A `for (borrow e in xs)` binding has no storage of its own: a place
    /// that names it reaches the iterable element. The alias root is resolved
    /// when the alias is bound, so one step suffices here; the place's own
    /// projections follow the root's, because the root's projections select
    /// the storage the place continues into.
    pub(super) fn resolve_alias_container(&self, target: PlaceTarget) -> PlaceTarget {
        match target {
            PlaceTarget::Local(id, projections) => match self.bindings[id].alias_root.clone() {
                Some(PlaceTarget::Local(root_id, mut root_projections)) => {
                    root_projections.extend(projections);
                    PlaceTarget::Local(root_id, root_projections)
                }
                Some(PlaceTarget::Global(global, mut root_projections)) => {
                    root_projections.extend(projections);
                    PlaceTarget::Global(global, root_projections)
                }
                None => PlaceTarget::Local(id, projections),
            },
            PlaceTarget::Global(global, projections) => PlaceTarget::Global(global, projections),
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

/// The source expression a view-returning call reborrows.
///
/// A written argument or a `Member` receiver keeps its expression, so a walk
/// can descend into it arm by arm; a qualified receiver names a place rather
/// than a written expression, so its provenance is carried instead.
enum ViewCallSource<'e> {
    /// A written argument or a `Member` receiver's base expression.
    Expr(&'e Expr),
    /// A qualified receiver's place provenance.
    Receiver(Provenance),
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

/// Merges the storage provenances a conditional or an array literal names.
///
/// Components that agree collapse to their single provenance, because the
/// value's storage is then unambiguous. A conditional with `prefer_local` set
/// keeps the old rule: a local component wins over every other one, so the
/// chosen arm's frame relation stays what the escape and loan rules judge. An
/// array literal passes `false`, because every element is stored rather than
/// chosen, and a local component stays one component of the composite.
fn merge_view_provenances(atoms: Vec<Provenance>, prefer_local: bool) -> Provenance {
    let mut unique: Vec<Provenance> = Vec::new();
    for atom in atoms {
        for part in FunctionChecker::storage_atoms(&atom) {
            if !unique.contains(&part) {
                unique.push(part);
            }
        }
    }
    match unique.len() {
        // An aggregate whose every arm or element is null names no storage at
        // all. The empty composite says so apart from the plain `None`
        // sentinel, which every other consumer reads as "storage the walk
        // cannot name" — an ownerless call result, for instance.
        0 => Provenance::Composite(Vec::new()),
        1 => unique.pop().expect("one component"),
        _ => {
            if prefer_local {
                if let Some(local) = unique
                    .iter()
                    .find(|atom| matches!(atom, Provenance::Local(_)))
                {
                    return local.clone();
                }
            }
            Provenance::Composite(unique)
        }
    }
}

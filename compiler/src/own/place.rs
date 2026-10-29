//! Place rules, view provenance, and IR lowering for the ownership stage.
//!
//! A place is read, written, or moved according to its binding kind and
//! its live loans, and a view carries the provenance of the storage it
//! points into so escape analysis can reason about it. The same methods
//! lower expressions and finalize the function intrinsic information.

use super::*;

/// The owner of a place path narrowed by a null check.
///
/// A local binding lives in the frame's flow state; a module binding has its
/// own narrowing set, because it can be read from every function in the
/// module.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum NarrowOwner {
    /// A frame-local binding and its place id.
    Local(usize),
    /// A module-level binding.
    Global(ir::GlobalId),
}

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
        self.note_opaque_owner_write(id, projections, span, false);
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
            // A write to the owner replaces the storage its view loans point
            // into, so the write reports the frozen conflict when one of those
            // loans is live (`E10031`). The owner's old element-loan positions
            // are dead afterwards: a later escape would only repeat the same
            // violation at a second site. The write's own conflict report
            // stands, and the binding loans stay because their uses keep that
            // report reachable.
            self.drop_written_owner_positions(id);
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
        // A nested write below a moved field only re-initializes the written
        // sub-path; the moved prefix still owns the rest of the field. Reporting
        // here keeps the field's moved state intact, so a later read of a
        // sibling path cannot observe the storage the move handed away.
        if projections.len() > 1 {
            if let Some(ir::Projection::Field(field)) = projections.first() {
                if self.state.place(id).partial.contains(field) {
                    let name = self.bindings[id].name.clone();
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
            }
            let maybe_moved_prefix = self.state.place(id).moved_if_owning.iter().any(|path| {
                path.len() < projections.len() && projections.starts_with(path)
            });
            if maybe_moved_prefix {
                // The instance decides whether the prefix owns. Record the
                // later-use duty without clearing the prefix, so an owning
                // instance reports the move and a duplicable one stays
                // accepted.
                self.note_maybe_moved_use(id, projections, span);
                return;
            }
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
                    place.maybe_move_destinations.clear();
                    place.maybe_move_strict.clear();
                    place.narrowed.clear();
                }
                Some(ir::Projection::Field(field)) => {
                    if !place.initialized {
                        place.initialized_fields.insert(field.clone());
                    }
                    // A whole-field write re-initializes the field; a nested
                    // write leaves the moved prefix in place.
                    if projections.len() == 1 {
                        place.partial.remove(field);
                    }
                    place.moved = false;
                    place.moved_if_owning.retain(|path| {
                        !written_path_clears(projections, path)
                    });
                    place.maybe_move_destinations.retain(|path, _| {
                        !written_path_clears(projections, path)
                    });
                    place.maybe_move_strict.retain(|path| {
                        !written_path_clears(projections, path)
                    });
                    place
                        .narrowed
                        .retain(|path| !written_path_clears(projections, path));
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
                    place.moved_if_owning.retain(|path| {
                        !written_path_clears(projections, path)
                    });
                    place.maybe_move_destinations.retain(|path, _| {
                        !written_path_clears(projections, path)
                    });
                    place.maybe_move_strict.retain(|path| {
                        !written_path_clears(projections, path)
                    });
                    place
                        .narrowed
                        .retain(|path| !written_path_clears(projections, path));
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
        self.note_opaque_owner_write(id, projections, span, true);
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
            if self.report_diagnostics
                && !self.reported_store_spans.insert((span.start, span.end))
            {
                return;
            }
            let name = binding.name.clone();
            self.error(
                "E10034",
                format!("cannot release or store borrowed parameter '{name}'"),
                super::LEND_STORE_HELP,
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
            // A whole-binding use of a top-level field move is the concrete
            // partial-move case, so it reports the partial-move code; every
            // other conflict keeps the moved-value code, including the
            // element and nested paths the concrete spelling would have
            // rejected at the move itself.
            let partial = projections.is_empty()
                && !state.moved_if_owning.iter().any(|path| path.is_empty())
                && state
                    .moved_if_owning
                    .iter()
                    .all(|path| matches!(path.as_slice(), [ir::Projection::Field(_)]));
            // The judgment follows the moved paths, not the use path: a whole
            // move conflicts with every later read, and a moved prefix owns
            // even when the read selects a duplicable member below it.
            let mut strict_destination = false;
            let destinations: Vec<Ty> = state
                .moved_if_owning
                .iter()
                .filter(|moved| {
                    moved.is_empty()
                        || projections.is_empty()
                        || projections.starts_with(moved)
                        || moved.starts_with(projections)
                })
                .flat_map(|moved| {
                    if state.maybe_move_strict.contains(moved) {
                        strict_destination = true;
                    }
                    state
                        .maybe_move_destinations
                        .get(moved)
                        .into_iter()
                        .flatten()
                        .cloned()
                })
                .collect();
            let moved: Vec<Vec<crate::types::PlacePathStep>> = state
                .moved_if_owning
                .iter()
                .filter(|moved| {
                    moved.is_empty()
                        || projections.is_empty()
                        || projections.starts_with(moved)
                        || moved.starts_with(projections)
                })
                .map(|moved| {
                    moved
                        .iter()
                        .map(|projection| match projection {
                            ir::Projection::Field(field) => {
                                crate::types::PlacePathStep::Field(field.clone())
                            }
                            ir::Projection::Element(_)
                            | ir::Projection::ElementTemp(_) => {
                                crate::types::PlacePathStep::Element
                            }
                        })
                        .collect()
                })
                .collect();
            self.record_requires_duplicable(
                id,
                span,
                partial,
                moved,
                destinations,
                strict_destination,
            );
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
        self.mark_moved_if_owning_at(target, span, false, None, false, false);
    }

    /// Records a maybe-owning move whose destination type is known.
    ///
    /// A concrete source moved into a maybe-owning destination moves only at
    /// an owning instance; the destination travels with the later-use duty so
    /// a view instance copies instead of reporting a move.
    pub(super) fn mark_moved_if_owning_into(
        &mut self,
        target: PlaceTarget,
        span: Span,
        destination: &Ty,
    ) {
        self.mark_moved_if_owning_at(target, span, false, Some(destination), false, false);
    }

    /// Records a maybe-owning move into a return destination.
    ///
    /// A conditional return owes one duty per lend arm, but the concrete twin
    /// reports the return site once; the flag lets the instance audit keep
    /// that one-report shape while take and store sites keep their per-arm
    /// reports.
    pub(super) fn mark_return_moved_if_owning_into(
        &mut self,
        target: PlaceTarget,
        span: Span,
        destination: &Ty,
    ) {
        self.mark_moved_if_owning_at(target, span, false, Some(destination), true, false);
    }

    /// Records a maybe-owning move into a raw-pointer store destination.
    ///
    /// The concrete raw store reports one lend diagnostic per site, so the
    /// instance audit deduplicates the duty like a return site.
    pub(super) fn mark_raw_store_moved_if_owning_into(
        &mut self,
        target: PlaceTarget,
        span: Span,
        destination: &Ty,
    ) {
        self.mark_moved_if_owning_at(target, span, false, Some(destination), false, true);
    }

    /// Records a maybe-owning move into a writable slot with a destination.
    pub(super) fn mark_moved_into_slot_if_owning_into(
        &mut self,
        target: PlaceTarget,
        span: Span,
        destination: &Ty,
    ) {
        self.mark_moved_if_owning_at(target, span, true, Some(destination), false, false);
    }

    /// Records a value-consuming move of a maybe-owning place.
    ///
    /// `stores_into_slot` flags a writable-parameter-slot destination for the
    /// out-slot carve-out.
    fn mark_moved_if_owning_at(
        &mut self,
        target: PlaceTarget,
        span: Span,
        stores_into_slot: bool,
        destination: Option<&Ty>,
        return_site: bool,
        raw_store_site: bool,
    ) {
        if let PlaceTarget::Local(id, projections) = target {
            self.with_place(id, |place| {
                place.moved_if_owning.insert(projections.clone());
                match destination {
                    Some(destination) => place
                        .maybe_move_destinations
                        .entry(projections.clone())
                        .or_default()
                        .push(destination.clone()),
                    // A move with no maybe-target destination keeps the strict
                    // judgment at every later use.
                    None => {
                        place.maybe_move_strict.insert(projections.clone());
                    }
                }
            });
            // A move through a borrow element view is the v1 element-move
            // limit once the substituted place owns, so every maybe-owning
            // destination — a return, a binding, an assignment, a conditional
            // arm — records the duty the per-instance audit judges.
            let move_ty = if projections.is_empty() {
                self.bindings[id].ty.clone()
            } else {
                self.projected_type(id, &projections)
            };
            self.note_borrowed_element_move(id, &projections, &move_ty, span);
            // A take parameter's deeper field projection is the same v1
            // partial-move limit once the substituted member owns. The
            // declaration cannot resolve the member yet, so it records the
            // duty; the instance audit reports E10026 for an owning
            // substitution and accepts a duplicable one. An element step is
            // the multi-extent row move the returned-row rules own, not this
            // partial-move limit, and a lend root keeps its own E10034 duty
            // below.
            if projections
                .iter()
                .all(|projection| matches!(projection, ir::Projection::Field(_)))
            {
                self.note_take_element_move(id, &projections, &move_ty, span);
            }
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
            // The duty carries the root parameter type and the projection:
            // the instance audit substitutes the root and then resolves the
            // selected member, so a duplicable member of an owning lend stays
            // a copy while an owning member reports.
            let ty = binding.ty.clone();
            if ty.is_unknown() || !self.recorded_not_lend.insert((ir_name, span.start, span.end)) {
                return;
            }
            self.not_lend_requirements.push(ir::RequiresNotLend {
                function: self.decl.name.clone(),
                parameter,
                ty,
                projections: projections.to_vec(),
                writable_source,
                stores_into_slot,
                destination: destination.cloned(),
                return_site,
                raw_store_site,
                span,
            });
        }
    }

    /// Records a partial element move out of a `take` parameter.
    ///
    /// The generic body's selected member has no concrete type yet, so the
    /// per-instance audit substitutes the root and reports `E10026` when the
    /// moved member owns. A duplicable instance stays a copy.
    pub(super) fn note_take_element_move(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        ty: &Ty,
        span: Span,
    ) {
        let binding = &self.bindings[id];
        if binding.kind != BindingKind::Param || binding.mode != Mode::Take {
            return;
        }
        // A whole take value and a single top-level field are legal moves; the
        // v1 partial-move limit judges the deeper projection.
        let top_level = projections.len() == 1
            && matches!(projections.first(), Some(ir::Projection::Field(_)));
        if projections.is_empty() || top_level {
            return;
        }
        if !(self.maybe_owns(ty) || ty.is_unknown()) {
            return;
        }
        if !self
            .recorded_element_moves
            .insert((id, span.start, span.end))
        {
            return;
        }
        let path: Vec<crate::types::PlacePathStep> = projections
            .iter()
            .map(|projection| match projection {
                ir::Projection::Field(field) => {
                    crate::types::PlacePathStep::Field(field.clone())
                }
                ir::Projection::Element(_) | ir::Projection::ElementTemp(_) => {
                    crate::types::PlacePathStep::Element
                }
            })
            .collect();
        self.element_move_requirements.push(ir::RequiresElementMove {
            function: self.decl.name.clone(),
            ty: binding.ty.clone(),
            path,
            span,
        });
    }

    /// Records the duplicability duty a body owes for one binding, at most once.
    ///
    /// Only a binding whose declared type mentions a type parameter carries the
    /// duty; the per-instance audit reports when the substituted type owns.
    /// `partial` marks a whole-binding use whose conflicting moves are member
    /// paths, which the audit reports with the partial-move code. The duty
    /// records the maybe-target destinations of the conflicting moves and a
    /// strict flag for a destination-less move; a concrete source with a
    /// maybe-owning destination still owes the per-instance judgment.
    pub(super) fn record_requires_duplicable(
        &mut self,
        id: usize,
        span: Span,
        partial: bool,
        paths: Vec<Vec<crate::types::PlacePathStep>>,
        destinations: Vec<Ty>,
        strict_destination: bool,
    ) {
        // A concrete source with a maybe-owning destination still needs the
        // duty: the instance decides whether the store moves or copies. A
        // destination-less move keeps the strict judgment.
        if !self.maybe_owns(&self.bindings[id].ty)
            && destinations.is_empty()
            && !strict_destination
        {
            return;
        }
        // One duty per binding and moved-path set: a later use keeps the
        // latest span and unions the destinations, so a later owning move is
        // judged after an earlier view copy without a second report.
        let key = (id, paths.clone());
        if let Some(index) = self.recorded_duplicable.get(&key).copied() {
            let requirement = &mut self.duplicable_requirements[index];
            let mut changed = false;
            for destination in destinations {
                if !requirement.destinations.contains(&destination) {
                    requirement.destinations.push(destination);
                    changed = true;
                }
            }
            if strict_destination && !requirement.strict_destination {
                requirement.strict_destination = true;
                changed = true;
            }
            // Keep the first use's span and partial flag unless a new
            // destination changes the judgment; then the later use is the
            // first one that can report.
            if changed {
                requirement.partial = partial;
                requirement.span = span;
            }
            return;
        }
        let ir_name = self.bindings[id].ir_name.clone();
        let declared = self.bindings[id].ty.clone();
        let index = self.duplicable_requirements.len();
        self.recorded_duplicable.insert(key, index);
        self.duplicable_requirements.push(ir::RequiresDuplicable {
            function: self.decl.name.clone(),
            binding: ir_name,
            ty: declared,
            partial,
            paths,
            destinations,
            strict_destination,
            span,
        });
    }

    /// Records the element-move duty a maybe-owning projected source owes.
    ///
    /// A conditional arm that projects past the top-level field limit is the
    /// v1 element-move limit once the substituted type owns. The body records
    /// the duty while the moved type is a parameter; the per-instance audit
    /// reports `E10026` for an owning substitution and accepts a duplicable
    /// one. One duty per store site keeps the report single even when both
    /// arms name the same projection.
    pub(super) fn note_maybe_element_move(&mut self, source: &PlaceTarget, span: Span) {
        let PlaceTarget::Local(id, projections) = source else {
            return;
        };
        let top_level = projections.len() == 1
            && matches!(projections.first(), Some(ir::Projection::Field(_)));
        if projections.is_empty() || top_level {
            return;
        }
        let moved = self.projected_type(*id, projections);
        if !(self.maybe_owns(&moved) || moved.is_unknown()) {
            return;
        }
        if !self
            .recorded_element_moves
            .insert((*id, span.start, span.end))
        {
            return;
        }
        let path: Vec<crate::types::PlacePathStep> = projections
            .iter()
            .map(|projection| match projection {
                ir::Projection::Field(field) => {
                    crate::types::PlacePathStep::Field(field.clone())
                }
                ir::Projection::Element(_) | ir::Projection::ElementTemp(_) => {
                    crate::types::PlacePathStep::Element
                }
            })
            .collect();
        self.element_move_requirements.push(ir::RequiresElementMove {
            function: self.decl.name.clone(),
            ty: self.bindings[*id].ty.clone(),
            path,
            span,
        });
    }

    /// Records the element-move duty a consumed move through a borrow element
    /// owes, at most once per site.
    ///
    /// A `for (borrow e in xs)` element aliases the iterable's element, so a
    /// value-consuming move of the element or one of its members is the v1
    /// element-move limit for every owning substitution. The concrete
    /// spelling reports it directly; a moved type that is still a parameter
    /// or unresolved records this duty, and the per-instance audit reports
    /// the same limit once the substituted place owns. A duplicable place
    /// copies and stays accepted, exactly as the concrete spelling does.
    pub(super) fn note_borrowed_element_move(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        ty: &Ty,
        span: Span,
    ) {
        let binding = &self.bindings[id];
        if binding.kind != BindingKind::Local || binding.mode != Mode::Borrow {
            return;
        }
        if !(self.maybe_owns(ty) || ty.is_unknown()) {
            return;
        }
        if !self
            .recorded_element_moves
            .insert((id, span.start, span.end))
        {
            return;
        }
        let path: Vec<crate::types::PlacePathStep> = projections
            .iter()
            .map(|projection| match projection {
                ir::Projection::Field(field) => {
                    crate::types::PlacePathStep::Field(field.clone())
                }
                ir::Projection::Element(_) | ir::Projection::ElementTemp(_) => {
                    crate::types::PlacePathStep::Element
                }
            })
            .collect();
        self.element_move_requirements.push(ir::RequiresElementMove {
            function: self.decl.name.clone(),
            ty: binding.ty.clone(),
            path,
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
        // A `static str` value always names module-lifetime bytes: a literal,
        // a `static str` parameter that received one, or a static field. The
        // value's storage cannot die with the current frame, so the view needs
        // no lend source even when it is read through a local or a borrow.
        if matches!(self.ty(expr).kind, TyKind::StaticStr) {
            return Provenance::Static;
        }
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
                let call_ty = self.ty(expr);
                // A call whose result is static storage outlives every frame
                // exactly like a literal. The callee's direct returns prove it
                // even when one of its parameters is named in the return: a
                // helper may read a static field of a borrow and return only
                // that field. The declaration's own `static str` spelling is
                // the same proof without a body scan.
                if self.call_returns_static_view(callee, &call_ty)
                    || matches!(call_ty.kind, TyKind::StaticStr)
                {
                    Provenance::Static
                } else {
                    self.call_provenance(callee, args)
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
            // A loop alias has no storage of its own: its element loans live
            // on the container under the alias root's projections. Resolve the
            // chain so returning the alias hands out the container's views,
            // exactly as a direct element return does.
            let id = self.alias_resolved_root(id);
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
            LoanOwner::Binding(id) => self.binding_dies_with_frame(id),
            LoanOwner::Temp(_) => true,
        }
    }

    /// True when a binding's storage dies with the current frame.
    ///
    /// A local dies with its frame, and so does a `take` parameter, whose
    /// value the callee owns. A capture aliases the enclosing binding named in
    /// its capture list, so its lifespan is that binding's: a captured local
    /// dies with the enclosing frame, while a captured parameter or global
    /// outlives it. A plain or `borrow` parameter and an out-slot name the
    /// caller's storage, which outlives the current frame.
    pub(super) fn binding_dies_with_frame(&self, id: usize) -> bool {
        let binding = &self.bindings[id];
        match binding.kind {
            BindingKind::Local => true,
            BindingKind::Param => {
                binding.mode == Mode::Take || binding.capture_dies_with_frame
            }
            BindingKind::OutSlot => false,
        }
    }

    /// True when the views a captured binding holds die with the enclosing
    /// frame.
    ///
    /// A capture aliases the enclosing binding, but a view it carries points at
    /// that binding's backing, not at the binding's own storage: a local `str`
    /// copied from a parameter survives the frame even though the local does
    /// not. A view into a local (or into an owned local's storage) dies with
    /// it. A non-view binding is judged by its own storage, which dies for a
    /// local or `take` parameter and survives for a plain parameter, an
    /// out-slot, or a module value.
    pub(super) fn capture_source_dies_with_frame(&self, id: usize) -> bool {
        match self.place_provenance(id) {
            Provenance::Static | Provenance::Global(_) => false,
            Provenance::Param(origin) | Provenance::Local(origin) => {
                self.binding_dies_with_frame(origin)
            }
            Provenance::Temp(_) => true,
            Provenance::None => self.binding_dies_with_frame(id),
            Provenance::Composite(atoms) => atoms.iter().any(|atom| match atom {
                Provenance::Static | Provenance::Global(_) => false,
                Provenance::Param(origin) | Provenance::Local(origin) => {
                    self.binding_dies_with_frame(*origin)
                }
                Provenance::Temp(_) | Provenance::None | Provenance::Composite(_) => true,
            }),
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
        let read = constant_path(&projections).then_some(projections);
        let mut owners = BTreeSet::new();
        match read {
            Some(read) => {
                for (path, loans) in paths {
                    if position_starts_with(path, &read) || position_covers(path, &read) {
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
        // A call-mediated view records its loans on the value itself rather
        // than on a name, so the expression-root walk above cannot see it.
        // Consult the value-loan route too; a global destination reports every
        // loaned owner, and a caller slot keeps only a frame-dying one.
        let mut value_owners = BTreeSet::new();
        self.loan_owners(&self.view_loans(expr), &mut value_owners);
        for owner in value_owners {
            if global || self.owner_dies_with_frame(owner) {
                owners.insert(owner);
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
                    constant_path(&projections).then_some(projections)
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
                if position_starts_with(path, &read) || position_covers(path, &read) {
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
    pub(super) fn expr_view_roots(&self, expr: &Expr, roots: &mut BTreeSet<usize>) {
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
            // A view returned through a call reads the storage of the call's
            // single lend source, so the return's loan walk follows that
            // source. Its element loans name the frame backing a local or
            // overwritten container holds.
            // A carrier-returning call hands back the view positions of every
            // source argument it may read; consult them when the call is not a
            // view return. Several sources each contribute their roots, so a
            // pair or an owned/taken argument is followed instead of ignored.
            ExprKind::Call { callee, args, .. } => {
                let view_sources = self.call_view_sources(callee, args);
                if !view_sources.is_empty() {
                    for source in view_sources {
                        self.expr_view_roots_source(callee, source, roots);
                    }
                } else {
                    for source in self.call_carrier_sources(callee, args) {
                        self.expr_view_roots_source(callee, source, roots);
                    }
                }
            }
            _ => {}
        }
    }

    /// The bindings a view expression's projection chain roots at, with the
    /// path from the expression to each root.
    ///
    /// Mirrors [`Self::expr_view_roots`] for a copy that embeds or extracts a
    /// carrier: an aggregate literal prefixes the position of the field or
    /// element, a place projection strips the copied prefix, and a call hands
    /// its sources the expression's own path. A `for (borrow e in xs)` alias
    /// resolves to its container exactly as the root walk does.
    pub(super) fn expr_view_positions(
        &self,
        expr: &Expr,
        path: &mut Vec<ir::Projection>,
        out: &mut Vec<(usize, Vec<ir::Projection>)>,
    ) {
        // A place expression already carries its complete root-to-leaf path,
        // in order, including constant indices. Use it directly so a member
        // over an index (`w.items[0]`) does not rebuild the steps out of
        // order.
        if let Some(PlaceTarget::Local(id, projections)) = self.place_target(expr) {
            let mut full = path.clone();
            full.extend(projections);
            out.push((self.alias_resolved_root(id), full));
            return;
        }
        match &expr.kind {
            ExprKind::Name(name) => {
                if let Some(id) = self.lookup(name) {
                    out.push((self.alias_resolved_root(id), path.clone()));
                }
            }
            ExprKind::Qualified(name) => {
                // A dotted value read lowers to a place chain; keep every
                // selected member in the position. A module path names no
                // binding and contributes nothing.
                if let Some(PlaceTarget::Local(id, projections)) = self.place_target(expr) {
                    out.push((self.alias_resolved_root(id), projections));
                } else if let Some(id) = name.segments.first().and_then(|first| self.lookup(first)) {
                    out.push((self.alias_resolved_root(id), path.clone()));
                }
            }
            ExprKind::Member { base, name } => {
                path.push(ir::Projection::Field(name.clone()));
                self.expr_view_positions(base, path, out);
                path.pop();
            }
            ExprKind::Index { base, index } => {
                // A constant index names one position; a run-time index may
                // reach any element.
                let element = crate::types::int_literal_value(index)
                    .and_then(|value| usize::try_from(value).ok())
                    .map(Some)
                    .unwrap_or(None);
                path.push(ir::Projection::Element(element));
                self.expr_view_positions(base, path, out);
                path.pop();
            }
            ExprKind::Unsafe(inner) => self.expr_view_positions(inner, path, out),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.expr_view_positions(value, path, out);
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.expr_view_positions(then_value, path, out);
                self.expr_view_positions(else_value, path, out);
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    path.push(ir::Projection::Field(field.name.clone()));
                    self.expr_view_positions(&field.value, path, out);
                    path.pop();
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for (index, element) in elements.iter().enumerate() {
                    path.push(ir::Projection::Element(Some(index)));
                    self.expr_view_positions(element, path, out);
                    path.pop();
                }
                if let Some(fill) = fill {
                    path.push(ir::Projection::Element(None));
                    self.expr_view_positions(fill, path, out);
                    path.pop();
                }
            }
            ExprKind::Call { callee, args, .. } => {
                let sources = self.call_view_sources(callee, args);
                if !sources.is_empty() {
                    for source in sources {
                        self.expr_view_positions_source(source, path, out);
                    }
                } else {
                    for source in self.call_carrier_sources(callee, args) {
                        self.expr_view_positions_source(source, path, out);
                    }
                }
            }
            _ => {}
        }
    }

    /// Adds the positions one resolved call source reaches.
    fn expr_view_positions_source(
        &self,
        source: ViewCallSource<'_>,
        path: &mut Vec<ir::Projection>,
        out: &mut Vec<(usize, Vec<ir::Projection>)>,
    ) {
        match source {
            ViewCallSource::Expr(source) => self.expr_view_positions(source, path, out),
            ViewCallSource::Receiver(provenance) | ViewCallSource::Capture(provenance) => {
                if let Some(owner) = owner_of(&provenance) {
                    if let LoanOwner::Binding(id) = owner {
                        let id = self.alias_resolved_root(id);
                        out.push((id, path.clone()));
                    }
                }
            }
        }
    }

    /// Adds the roots one resolved call source reaches.
    fn expr_view_roots_source(
        &self,
        callee: &Expr,
        source: ViewCallSource<'_>,
        roots: &mut BTreeSet<usize>,
    ) {
        match source {
            ViewCallSource::Expr(source) => self.expr_view_roots(source, roots),
            ViewCallSource::Receiver(provenance) => {
                if let Some(owner) = owner_of(&provenance) {
                    if let LoanOwner::Binding(id) = owner {
                        roots.insert(self.alias_resolved_root(id));
                    }
                }
                // A receiver whose own provenance is static (a view array of
                // literals, for example) still reads that binding's element
                // loans; name the receiver root itself.
                match &callee.kind {
                    ExprKind::Member { base, .. } => self.expr_view_roots(base, roots),
                    ExprKind::Qualified(name) => {
                        if let Some(id) = name.segments.first().and_then(|first| self.lookup(first))
                        {
                            roots.insert(self.alias_resolved_root(id));
                        }
                    }
                    _ => {}
                }
            }
            ViewCallSource::Capture(provenance) => {
                if let Some(owner) = owner_of(&provenance) {
                    if let LoanOwner::Binding(id) = owner {
                        roots.insert(self.alias_resolved_root(id));
                    }
                }
            }
        }
    }

    /// The binding a root reaches when it is a `for` alias.
    ///
    /// An alias binding has no storage of its own: its place reaches the
    /// iterable element, so every owner walk follows the alias chain to the
    /// container exactly as the element-loan records do.
    pub(super) fn alias_resolved_root(&self, mut id: usize) -> usize {
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
    /// cannot name the element it overwrote: a fixed extent names its
    /// candidate elements so a later constant store can replace one of them,
    /// while an open or unknown extent keeps the union and only adds.
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
            // A run-time index names no single element it overwrote. When the
            // destination's declared type fixes the extent at every wildcard
            // step, the store can write only those elements: record the new
            // loans at each candidate. A later constant store then replaces
            // exactly the slot it overwrote, and a clean run-time store adds
            // no loan of its own. An existing candidate record stays, because
            // the store may have hit another element. An open or unknown
            // extent has no finite candidate set, so the record keeps its
            // wildcard position and the binding union: a later constant read
            // under the same prefix matches the wildcard and reports the
            // backing's escape, and the binding union stays for the
            // write-conflict reading.
            match self.fixed_runtime_positions(id, projections) {
                Some(candidates) => {
                    if !loans.is_empty() {
                        for candidate in candidates {
                            self.state
                                .element_loans
                                .entry(id)
                                .or_default()
                                .entry(candidate)
                                .or_default()
                                .extend(loans.iter().copied());
                        }
                    }
                }
                None => {
                    if !union.is_empty() {
                        self.state
                            .element_loans
                            .entry(id)
                            .or_default()
                            .insert(projections.to_vec(), union.clone());
                    }
                }
            }
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

    /// The concrete element paths a run-time store may write.
    ///
    /// A run-time index cannot name the element it wrote, so the loan record
    /// would otherwise name a wildcard position. When the destination's
    /// declared type fixes the extent at every wildcard step, the store can
    /// write only those elements: naming each one lets a later constant store
    /// replace exactly the slot it overwrote, while every other candidate
    /// keeps its own record. A field step follows the struct's declared field
    /// type, so a wildcard below a field is expanded at the field's extent.
    /// An open, unknown, or non-array extent has no finite candidate set, so
    /// `None` tells the caller to keep one conservative wildcard record across
    /// the aggregate.
    fn fixed_runtime_positions(
        &self,
        id: usize,
        projections: &[ir::Projection],
    ) -> Option<Vec<Vec<ir::Projection>>> {
        let mut candidates = vec![(self.bindings[id].ty.clone(), Vec::new())];
        for projection in projections {
            let mut next = Vec::new();
            for (ty, path) in candidates {
                match projection {
                    ir::Projection::Element(Some(index)) => {
                        let mut full = path;
                        full.push(ir::Projection::Element(Some(*index)));
                        next.push((crate::types::index_element(&ty), full));
                    }
                    ir::Projection::Element(None) | ir::Projection::ElementTemp(_) => {
                        let TyKind::Array { extents, .. } = &ty.kind else {
                            return None;
                        };
                        let extent = (*extents.first()?)?;
                        let element = crate::types::index_element(&ty);
                        for index in 0..extent {
                            let mut full = path.clone();
                            full.push(ir::Projection::Element(Some(index)));
                            next.push((element.clone(), full));
                        }
                    }
                    ir::Projection::Field(field) => {
                        let TyKind::Struct(name, _) = &ty.kind else {
                            return None;
                        };
                        let field_ty = self
                            .instance_fields(&ty, name)
                            .into_iter()
                            .find(|(candidate, _)| candidate == field)?
                            .1;
                        let mut full = path;
                        full.push(ir::Projection::Field(field.clone()));
                        next.push((field_ty, full));
                    }
                }
            }
            candidates = next;
        }
        Some(candidates.into_iter().map(|(_, path)| path).collect())
    }

    /// Moves a carrier's recorded view positions onto another binding.
    ///
    /// A carrier copy (`h2 = h1`) hands the destination the same views the
    /// source holds, so every recorded position contributes its loans under
    /// the destination's path: the copy's views read the same storage, and a
    /// later escape reports through the destination exactly as through the
    /// source. `source` is the projection path the copied value reads: a
    /// whole-binding copy passes `[]` and keeps every position, while a
    /// projected copy (`let ys: str[2] = rows[0];`) keeps the positions
    /// recorded under the projection and records their remaining steps on the
    /// destination, exactly as the elements of `rows[0]` become the elements
    /// of `ys`. The destination's own record under `base` is replaced first —
    /// the store replaced those positions — and a self-assignment keeps its
    /// positions because the source's record is captured before the
    /// replacement.
    pub(super) fn move_element_loans(
        &mut self,
        from: usize,
        source: &[ir::Projection],
        to: usize,
        base: &[ir::Projection],
    ) {
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
            let Some(rest) = position_rest(&path, source) else {
                continue;
            };
            let mut full = base.to_vec();
            full.extend(rest.iter().cloned());
            if loans.is_empty() {
                // A known position with no loan (static, module, or caller
                // storage) moves with the copy; the destination must keep the
                // known-empty record so its own binding is not mistaken for
                // the backing.
                self.state
                    .element_loans
                    .entry(to)
                    .or_default()
                    .entry(full)
                    .or_default();
            } else {
                self.replace_element_loans(to, &full, loans);
            }
        }
    }

    /// Records the view-position loans a copy source hands its destination.
    ///
    /// A copy reads the source positions and stores their views, so the
    /// destination's positions loan the same storage: a place contributes the
    /// positions recorded under its projection, a conditional both arms, and
    /// an array literal each element under its own index and the fill value
    /// across the remaining positions. A fill over a known fixed extent is
    /// expanded onto each element it stores, so overwriting or reading one
    /// filled element keeps or drops exactly that element's loan; an extent the
    /// declaration leaves open keeps one record across the aggregate. The whole
    /// record is collected before the destination is written, so the
    /// destination carries the union and a later arm never replaces an earlier
    /// arm's positions. A position whose value derives directly (`[a.v]`)
    /// creates its shared loan at the copy site, exactly as an element store
    /// does.
    ///
    /// `lowered` is the store's lowered value when it has one. A non-place
    /// element such as a call's member borrows storage from the statement
    /// temporary the lowering created, and only the lowered chain names that
    /// owner; the collector records it under the destination's path.
    pub(super) fn record_copy_element_loans(
        &mut self,
        to: usize,
        base: &[ir::Projection],
        value: &Expr,
        lowered: Option<&Value>,
        position: &Ty,
        span: Span,
    ) {
        let copied = self.snapshot_copy_element_loans(value, lowered, position, base);
        self.apply_copy_element_loans(to, base, copied, span);
    }

    /// Collects a copy source's position loans without touching the
    /// destination.
    ///
    /// The collector names bindings through the scope it runs in, so gathering
    /// the loans before a shadowing declaration keeps each arm tied to the
    /// outer binding the initializer reads. The result is applied to the fresh
    /// destination once it exists.
    pub(super) fn snapshot_copy_element_loans(
        &self,
        value: &Expr,
        lowered: Option<&Value>,
        position: &Ty,
        base: &[ir::Projection],
    ) -> CopyLoans {
        let mut copied = CopyLoans::default();
        self.collect_copy_element_loans(value, lowered, Some(position), base, &mut copied);
        copied
    }

    /// Applies a collected copy's position loans to a destination binding.
    pub(super) fn apply_copy_element_loans(
        &mut self,
        to: usize,
        base: &[ir::Projection],
        copied: CopyLoans,
        span: Span,
    ) {
        let mut paths = copied.recorded;
        for (path, owners) in copied.owners {
            let loans = paths.entry(path).or_default();
            for owner in owners {
                if let Some(loan) = self.record_view_loan(owner, LoanKind::Shared, span) {
                    loans.insert(loan);
                }
            }
        }
        // The store replaced every position under `base`, so the destination's
        // record there is dropped once before the collected union is written.
        self.replace_element_loans(to, base, BTreeSet::new());
        for (path, loans) in paths {
            if loans.is_empty() {
                // A view position whose backing is static, module-level, or
                // caller-backed is known and carries no loan. Recording the
                // empty position keeps the later judgment from treating the
                // carrier's own binding as the backing.
                self.state
                    .element_loans
                    .entry(to)
                    .or_default()
                    .entry(path)
                    .or_default();
            } else {
                self.replace_element_loans(to, &path, loans);
            }
        }
    }

    /// Collects one copy source's position loans into `copied`.
    ///
    /// A conditional contributes both arms, an array literal each element and
    /// the fill value, and a projection or cast its inner value. `position` is
    /// the declared type of the copied value, which names the fixed extent a
    /// fill covers; without one the fill stays one aggregate record.
    /// `lowered` is the same value after lowering, so each recursive step keeps
    /// the lowered chain that names a statement temporary the source reads
    /// through. Every other expression is read as a place: the positions it
    /// selects are translated onto the destination's path, and a place with no
    /// recorded position contributes the real owners its storage derives from.
    fn collect_copy_element_loans(
        &self,
        value: &Expr,
        lowered: Option<&Value>,
        position: Option<&Ty>,
        base: &[ir::Projection],
        copied: &mut CopyLoans,
    ) {
        match &value.kind {
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                let (then_lowered, else_lowered) = match lowered.map(|lowered| &lowered.kind) {
                    Some(ValueKind::Conditional {
                        then_value,
                        else_value,
                        ..
                    }) => (Some(then_value.as_ref()), Some(else_value.as_ref())),
                    _ => (None, None),
                };
                self.collect_copy_element_loans(
                    then_value,
                    then_lowered,
                    position,
                    base,
                    copied,
                );
                self.collect_copy_element_loans(
                    else_value,
                    else_lowered,
                    position,
                    base,
                    copied,
                );
            }
            ExprKind::StructLiteral { name, fields, .. } => {
                let struct_name = name.segments.last().cloned().unwrap_or_default();
                let mut declared = position
                    .map(|position| self.instance_fields(position, &struct_name))
                    .unwrap_or_default();
                if declared.is_empty() {
                    declared = self.instance_fields(&self.ty(value), &struct_name);
                }
                let lowered_fields = match lowered.map(|lowered| &lowered.kind) {
                    Some(ValueKind::StructLit { fields, .. }) => Some(fields.as_slice()),
                    _ => None,
                };
                for field in fields {
                    let field_ty = declared
                        .iter()
                        .find(|(declared, _)| declared == &field.name)
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or_else(|| self.ty(&field.value));
                    if !self.may_carry_view(&field_ty) {
                        continue;
                    }
                    let mut path = base.to_vec();
                    path.push(ir::Projection::Field(field.name.clone()));
                    let lowered_field = lowered_fields.and_then(|fields| {
                        fields
                            .iter()
                            .find(|(written, _)| written == &field.name)
                            .map(|(_, value)| value)
                    });
                    self.collect_copy_element_loans(
                        &field.value,
                        lowered_field,
                        Some(&field_ty),
                        &path,
                        copied,
                    );
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                let extent = match position.map(|position| &position.kind) {
                    Some(TyKind::Array { extents, .. }) => {
                        extents.first().copied().flatten()
                    }
                    _ => None,
                };
                let (lowered_elements, lowered_fill) = match lowered.map(|lowered| &lowered.kind) {
                    Some(ValueKind::ArrayLit { elements, fill }) => {
                        (Some(elements.as_slice()), fill.as_deref())
                    }
                    _ => (None, None),
                };
                // The element at this nesting level keeps the remaining
                // extents, so a nested literal expands its own fill at its own
                // dimension instead of collapsing to the folded leaf type.
                let element = position.map(Ty::iterated_element);
                for (index, element_value) in elements.iter().enumerate() {
                    let mut path = base.to_vec();
                    path.push(ir::Projection::Element(Some(index)));
                    let lowered_element = lowered_elements.and_then(|elements| elements.get(index));
                    self.collect_copy_element_loans(
                        element_value,
                        lowered_element,
                        element.as_ref(),
                        &path,
                        copied,
                    );
                }
                if let Some(fill) = fill {
                    match extent {
                        // A known extent names every element the fill stores,
                        // so an overwrite or a read keeps or drops exactly
                        // that element's loan.
                        Some(extent) => {
                            for index in elements.len()..extent {
                                let mut path = base.to_vec();
                                path.push(ir::Projection::Element(Some(index)));
                                self.collect_copy_element_loans(
                                    fill,
                                    lowered_fill,
                                    element.as_ref(),
                                    &path,
                                    copied,
                                );
                            }
                        }
                        // An open extent keeps one record across the
                        // aggregate, which covers every remaining element.
                        None => {
                            self.collect_copy_element_loans(
                                fill,
                                lowered_fill,
                                element.as_ref(),
                                base,
                                copied,
                            )
                        }
                    }
                }
            }
            ExprKind::Unsafe(inner) => {
                self.collect_copy_element_loans(inner, lowered, position, base, copied)
            }
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                let inner_lowered = match lowered.map(|lowered| &lowered.kind) {
                    Some(ValueKind::Cast { value, .. }) => Some(value.as_ref()),
                    _ => lowered,
                };
                self.collect_copy_element_loans(value, inner_lowered, position, base, copied);
            }
            _ => self.collect_place_copy_loans(value, lowered, base, copied),
        }
    }

    /// True when a value contains a call under any structural wrapper.
    ///
    /// A carrier-returning call supplies the returned view positions even when
    /// its own view-loan set is empty, so the copy gate admits a call wrapped
    /// in `unsafe`, a cast, or a conditional arm.
    pub(super) fn mentions_call(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Call { .. } => true,
            ExprKind::Unsafe(inner) => Self::mentions_call(inner),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                Self::mentions_call(value)
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => Self::mentions_call(then_value) || Self::mentions_call(else_value),
            ExprKind::Index { base, index } => {
                Self::mentions_call(base) || Self::mentions_call(index)
            }
            ExprKind::Member { base, .. } => Self::mentions_call(base),
            ExprKind::Unary { operand, .. } => Self::mentions_call(operand),
            ExprKind::Binary { left, right, .. } => {
                Self::mentions_call(left) || Self::mentions_call(right)
            }
            ExprKind::Assign { target, value, .. } => {
                Self::mentions_call(target) || Self::mentions_call(value)
            }
            ExprKind::StructLiteral { fields, .. } => {
                fields.iter().any(|field| Self::mentions_call(&field.value))
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                elements.iter().any(Self::mentions_call)
                    || fill.as_deref().is_some_and(Self::mentions_call)
            }
            ExprKind::IntrinsicCall { args, .. } => args.iter().any(|arg| match arg {
                crate::ast::IntrinsicArg::Expr(expr) => Self::mentions_call(expr),
                _ => false,
            }),
            ExprKind::Template(pieces) => pieces.iter().any(|piece| match piece {
                crate::ast::TemplatePiece::Placeholder(expr) => Self::mentions_call(expr),
                _ => false,
            }),
            _ => false,
        }
    }

    /// True when a copy source names its view positions arm by arm.
    ///
    /// A conditional or an array literal — under any cast or unsafe wrapper —
    /// contributes independent positions, a run-time element read names no
    /// single recorded position, and a call that reborrows its argument's
    /// storage carries that argument's loans. Every other value keeps the
    /// binding effect's own loan creation, so a call result's exclusive view
    /// keeps its kind and its single loan request.
    pub(super) fn collects_copy_positions(&self, value: &Expr) -> bool {
        match &value.kind {
            ExprKind::Conditional { .. }
            | ExprKind::StructLiteral { .. }
            | ExprKind::ArrayLiteral { .. } => true,
            ExprKind::Unsafe(inner) => self.collects_copy_positions(inner),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.collects_copy_positions(value)
            }
            ExprKind::Call { .. } => !self.view_loans(value).is_empty(),
            _ => self.place_target(value).is_some(),
        }
    }

    /// Collects the position loans one place read contributes to a copy.
    ///
    /// A binding with recorded positions is authoritative: a constant read
    /// keeps every position at or below it, translated onto the destination's
    /// path, while a run-time read cannot name the element it selects and
    /// contributes every recorded position at the destination's own path. A
    /// local place with no recorded position contributes the loans the value
    /// already carries, or else the real owners of its storage; a value that
    /// names no place contributes the loans it carries, or else the owners its
    /// lowered chain reads through. That last route names a statement
    /// temporary the source expression cannot, such as the call result behind
    /// `mk().v`, so a later read of the destination reports the temporary's
    /// release. A value that derives a fresh view without an owner — a literal,
    /// or a module backing — still contributes nothing.
    fn collect_place_copy_loans(
        &self,
        value: &Expr,
        lowered: Option<&Value>,
        base: &[ir::Projection],
        copied: &mut CopyLoans,
    ) {
        if let Some(PlaceTarget::Local(source, projections)) = self.place_target(value) {
            // A loop alias has no storage of its own: its element loans live
            // on the container under the alias root's projections.
            let (loan_source, loan_projections) =
                match &self.bindings[source].alias_root {
                    Some(PlaceTarget::Local(root, root_projections)) => {
                        (*root, root_projections.clone())
                    }
                    _ => (source, projections.clone()),
                };
            if let Some(paths) = self.state.element_loans.get(&loan_source) {
                let read =
                    constant_path(&loan_projections).then_some(loan_projections.as_slice());
                let mut recorded = false;
                for (path, loans) in paths {
                    let destination = match read {
                        Some(read) => {
                            if let Some(rest) = position_rest(path, read) {
                                let mut full = base.to_vec();
                                full.extend(rest.iter().cloned());
                                Some(full)
                            } else if position_covers(path, read) {
                                // The read selects a position a fill record
                                // covers, so the copied value is one view at
                                // the destination's own path.
                                Some(base.to_vec())
                            } else {
                                None
                            }
                        }
                        None => Some(base.to_vec()),
                    };
                    if let Some(destination) = destination {
                        copied
                            .recorded
                            .entry(destination)
                            .or_default()
                            .extend(loans.iter().copied());
                        recorded = true;
                    }
                }
                if !recorded {
                    // The selected position exists but names no loan, so it
                    // is known caller-, module-, or static-backed.
                    copied.recorded.entry(base.to_vec()).or_default();
                }
                return;
            }
            let loans = self.view_loans(value);
            if !loans.is_empty() {
                copied
                    .recorded
                    .entry(base.to_vec())
                    .or_default()
                    .extend(loans);
                return;
            }
            let owners = self.provenance_loan_owners(value);
            if !owners.is_empty() {
                copied.owners.entry(base.to_vec()).or_default().extend(owners);
                return;
            }
            // A parameter-backed view position is caller storage and carries
            // no frame loan. A static-str local points at static storage. A
            // local with no recorded position stays unknown, so the binding's
            // own provenance judges it instead.
            let root = self.alias_resolved_root(source);
            if matches!(
                self.bindings[root].kind,
                BindingKind::Param | BindingKind::OutSlot
            ) || matches!(self.ty(value).kind, TyKind::StaticStr)
            {
                copied.recorded.entry(base.to_vec()).or_default();
            }
            return;
        }
        if let Some(PlaceTarget::Global(_, _)) = self.place_target(value) {
            copied.recorded.entry(base.to_vec()).or_default();
            return;
        }
        let loans = self.view_loans(value);
        if !loans.is_empty() {
            copied
                .recorded
                .entry(base.to_vec())
                .or_default()
                .extend(loans);
            return;
        }
        // A carrier-returning call hands back the view positions of every
        // argument it may read. The copy reads those views, so the destination
        // loans each argument's backings exactly as a direct place copy would;
        // a later write through a backing reports the live loan. A projected
        // chain rooted at the call (`make(a.v).names`) reads the same positions,
        // so the walk resolves its call root too.
        if let Some((callee, args)) = self.carrier_call_root(value) {
            let sources = self.call_carrier_sources(callee, args);
            if !sources.is_empty() {
                for source in sources {
                    match source {
                        ViewCallSource::Expr(source) => {
                            self.collect_copy_element_loans(source, None, None, base, copied);
                        }
                        ViewCallSource::Receiver(provenance)
                        | ViewCallSource::Capture(provenance) => {
                            for atom in Self::storage_atoms(&provenance) {
                                if let Some(owner) = owner_of(&atom) {
                                    copied
                                        .owners
                                        .entry(base.to_vec())
                                        .or_default()
                                        .insert(owner);
                                }
                            }
                        }
                    }
                }
                if !copied.owners.is_empty() || !copied.recorded.is_empty() {
                    return;
                }
            }
        }
        // A non-place value that reads no loan still derives its view from
        // storage the expression walk cannot name: a projection of a lowered
        // call result reads the statement temporary the lowering created.
        // The lowered chain names that owner, so the destination loans it and
        // a later escape reports the temporary's release.
        let owners = self.view_owners(value, lowered);
        if !owners.is_empty() {
            copied.owners.entry(base.to_vec()).or_default().extend(owners);
            return;
        }
        // A carrier-returning call whose view-like arguments are all static,
        // module, or caller-backed hands back a view of that storage. Mark the
        // destination position known so the carrier binding is not mistaken
        // for the backing; a local or fresh argument keeps the frame judgment.
        if let ExprKind::Call { args, .. } = &value.kind {
            let view_args: Vec<&Expr> = args
                .iter()
                .map(|arg| &arg.value)
                .filter(|arg| self.may_carry_view(&self.ty(arg)))
                .collect();
            if !view_args.is_empty()
                && view_args.iter().all(|arg| {
                    matches!(
                        self.view_provenance(arg),
                        Provenance::Static | Provenance::Global(_) | Provenance::Param(_)
                    )
                })
            {
                copied.recorded.entry(base.to_vec()).or_default();
                return;
            }
        }
        // A static literal view names no owner, but its position is known and
        // safe. Mark it so the carrier's own binding is not mistaken for the
        // backing. A call-derived view keeps its lower-chain judgment: an
        // unrecorded owner there is not assumed static.
        if matches!(self.view_provenance(value), Provenance::Static)
            || matches!(value.kind, ExprKind::Str(_))
            || matches!(self.ty(value).kind, TyKind::StaticStr)
        {
            copied.recorded.entry(base.to_vec()).or_default();
        }
    }

    /// The real owners a value's storage provenance names.
    ///
    /// Module storage, literals, and ownerless values name no owner, so they
    /// contribute no loan.
    fn provenance_loan_owners(&self, value: &Expr) -> BTreeSet<LoanOwner> {
        let mut owners = BTreeSet::new();
        for atom in Self::storage_atoms(&self.view_provenance(value)) {
            if let Some(owner) = owner_of(&atom) {
                owners.insert(owner);
            }
        }
        owners
    }

    /// Drops the recorded view positions that read a written owner's storage.
    ///
    /// A write to a non-view binding replaces the storage its view loans point
    /// into; when one of those loans is live, the write reports the frozen
    /// conflict (`E10031`). The positions that name the owner are stale
    /// afterwards: a later escape of one of those views would repeat the same
    /// violation at a second site. The binding loans stay, because their uses
    /// keep the write's conflict report reachable; only the per-position
    /// record is dropped, exactly as overwriting a view element drops the
    /// loans of the value it replaced.
    fn drop_written_owner_positions(&mut self, owner: usize) {
        let stale: BTreeSet<LoanId> = self
            .loan_ids
            .iter()
            .filter(|(origin, _)| origin.owner == LoanOwner::Binding(owner))
            .map(|(_, loan)| *loan)
            .collect();
        if stale.is_empty() {
            return;
        }
        self.state.element_loans.retain(|_, paths| {
            for loans in paths.values_mut() {
                loans.retain(|loan| !stale.contains(loan));
            }
            paths.retain(|_, loans| !loans.is_empty());
            !paths.is_empty()
        });
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

    /// The frame-dying owners a value's carried loans name.
    ///
    /// A view value that already carries loans reads the storage those loans
    /// point into, exactly as a view binding does. A caller-backed owner
    /// outlives this frame, so only a frame-dying owner makes an escape, while
    /// a statement temporary always dies with the statement that created it.
    /// This is the direct spelling's route: a call result reborrows its
    /// argument's loans on the value itself and records no binding positions.
    pub(super) fn value_loan_frame_owners(&self, expr: &Expr) -> BTreeSet<LoanOwner> {
        let mut owners = BTreeSet::new();
        self.loan_owners(&self.view_loans(expr), &mut owners);
        owners.retain(|owner| self.owner_dies_with_frame(*owner));
        owners
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
            roots.extend(self.lowered_owner_bindings(lowered));
            for id in roots {
                // Only a generated owning owner is itself storage a view can
                // name. A generated view carrier and a named source container
                // own no storage of their own: the views they hold loan the
                // owners their binding records, so the leaf owner is the one a
                // later use must outlive.
                if self.bindings[id].generated && self.bindings[id].needs_drop {
                    owners.insert(LoanOwner::Binding(id));
                }
                // The leaf loans a generated view carrier holds are the
                // storage a call-mediated view reads. A named source's loans
                // are already contributed by the value walk, and adding the
                // binding's existing loan here would request it twice.
                if self.bindings[id].generated {
                    owners.extend(self.binding_loan_owners(id));
                }
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
            ValueKind::Place(place) => self.note_lowered_place_root(place, roots),
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
            // A view returned through a call reads the storage of the call's
            // single lend source, not every argument: an unrelated scalar or
            // aggregate argument is not a loan. A func-typed or closure call
            // has no declared lend source, so it contributes no root here.
            ValueKind::Call {
                callee,
                args,
                receiver: None,
                ..
            } => {
                if let Some(index) = self.lowered_lend_source(callee) {
                    if let Some(source) = args.get(index) {
                        self.lowered_root_bindings(source, roots);
                    }
                }
            }
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. } => {}
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.lowered_root_bindings(then_value, roots);
                self.lowered_root_bindings(else_value, roots);
            }
            // A materialized owner fills a named destination; that binding is
            // the storage the view reads. A temporary materialization keeps
            // the value chain below it.
            ValueKind::Materialize {
                destination: Some(place),
                ..
            } => self.note_lowered_place_root(place, roots),
            ValueKind::Materialize { value: inner, .. } => {
                self.lowered_root_bindings(inner, roots);
            }
            _ => {}
        }
    }

    /// Adds one lowered place's local root binding to `roots`.
    fn note_lowered_place_root(&self, place: &ir::Place, roots: &mut BTreeSet<usize>) {
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
                    Some(ViewCallSource::Receiver(provenance))
                    | Some(ViewCallSource::Capture(provenance)) => {
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
        let mut loans = BTreeSet::new();
        for source in self.call_view_sources(callee, args) {
            match source {
                ViewCallSource::Expr(source) => loans.extend(self.view_loans(source)),
                ViewCallSource::Receiver(_) => {
                    loans.extend(self.dot_receiver_loans(callee));
                }
                ViewCallSource::Capture(provenance) => {
                    if let Provenance::Param(id) | Provenance::Local(id) = provenance {
                        loans.extend(self.binding_loans(id));
                    }
                }
            }
        }
        loans
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
                match self.resolved_callee(&name) {
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

    /// The parameter source a stored value keeps after local copies.
    ///
    /// A view local already carries its source in `provenance`. A maybe-owning
    /// local copy does not: the copy semantics stay local until the instance
    /// is known, so `copy_source` records the source separately for the
    /// whole-out-slot judgment.
    pub(super) fn store_source_provenance(&self, expr: &Expr) -> Provenance {
        if let ExprKind::Name(_) = &expr.kind {
            if let Some(PlaceTarget::Local(id, projections)) = self.place_target(expr) {
                if projections.is_empty() {
                    if self.bindings[id].rebound {
                        return Provenance::Local(id);
                    }
                    if let Some(source) = &self.bindings[id].copy_source {
                        return source.clone();
                    }
                }
            }
        }
        self.view_provenance(expr)
    }

    /// The flattened source atoms of a stored value, following rebound copies.
    ///
    /// A rebound local names its own binding id, but the storage it holds is
    /// whatever its last assignment carried. Following that binding's source
    /// set gives the store judge the original parameter or local instead of a
    /// generic local atom. A local whose set is empty is a real frame-backed
    /// view and stays a local atom. The visiting set breaks a self-copy cycle
    /// conservatively by keeping the local atom.
    pub(super) fn source_atoms(&self, source: &Provenance) -> Vec<Provenance> {
        let mut atoms = Vec::new();
        self.collect_source_atoms(source, &mut BTreeSet::new(), &mut atoms);
        atoms.retain(|atom| !matches!(atom, Provenance::None));
        atoms
    }

    /// Recursively flattens one provenance into the source atoms it names.
    fn collect_source_atoms(
        &self,
        source: &Provenance,
        visiting: &mut BTreeSet<usize>,
        out: &mut Vec<Provenance>,
    ) {
        for atom in Self::storage_atoms(source) {
            match &atom {
                Provenance::Local(id) => {
                    let id = *id;
                    let held = self
                        .state
                        .places
                        .get(&id)
                        .map(|place| place.sources.clone())
                        .unwrap_or_default();
                    if !held.is_empty() && visiting.insert(id) {
                        for inner in held {
                            self.collect_source_atoms(&inner, visiting, out);
                        }
                        visiting.remove(&id);
                    } else if !out.contains(&atom) {
                        out.push(atom);
                    }
                }
                _ => {
                    if !out.contains(&atom) {
                        out.push(atom);
                    }
                }
            }
        }
    }

    /// The frame-local or fresh source name of a stored value.
    ///
    /// A view over a local owner or a statement temporary has no parameter
    /// root. The concrete spelling reports `E10030`; a parameter-typed slot
    /// records this name so the instance audit can report the same escape.
    pub(super) fn store_local_source_name(&self, expr: &Expr) -> Option<String> {
        for atom in Self::storage_atoms(&self.store_source_provenance(expr)) {
            match atom {
                Provenance::Local(owner) => return Some(self.bindings[owner].name.clone()),
                Provenance::Temp(_) => return Some("a statement temporary".to_owned()),
                _ => {}
            }
        }
        None
    }

    /// The provenance of a view produced by a call.
    ///
    /// The call resolves its single reborrowed source through
    /// [`Self::call_view_source`]; the view's provenance is that source's.
    pub(super) fn call_provenance(&self, callee: &Expr, args: &[Arg]) -> Provenance {
        let mut atoms = Vec::new();
        for source in self.call_view_sources(callee, args) {
            atoms.push(match source {
                ViewCallSource::Expr(source) => self.view_provenance(source),
                ViewCallSource::Receiver(provenance)
                | ViewCallSource::Capture(provenance) => provenance,
            });
        }
        match atoms.len() {
            0 => Provenance::None,
            1 => atoms.pop().unwrap_or(Provenance::None),
            _ => Provenance::Composite(atoms),
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
            ExprKind::Call { callee, args, .. } => {
                let sources = self.call_view_sources(callee, args);
                if sources.len() > 1 {
                    // Several value sources are mixed whenever their storage
                    // differs; a destination cannot attribute one backing.
                    let mut provenances = Vec::new();
                    for source in &sources {
                        provenances.push(match source {
                            ViewCallSource::Expr(source) => self.view_provenance(source),
                            ViewCallSource::Receiver(provenance)
                            | ViewCallSource::Capture(provenance) => provenance.clone(),
                        });
                    }
                    return provenances
                        .windows(2)
                        .any(|pair| pair[0] != pair[1])
                        || sources.iter().any(|source| match source {
                            ViewCallSource::Expr(source) => self.view_is_mixed(source),
                            _ => false,
                        });
                }
                match sources.into_iter().next() {
                    Some(ViewCallSource::Expr(source)) => self.view_is_mixed(source),
                    _ => false,
                }
            }
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
    ///
    /// A value callable — a `func`-typed binding, a closure, or a
    /// call-returned callable — resolves through its own signature, in the same
    /// order as the call check, so a closure or wrapper call names the same
    /// source a direct named call does. A generic declaration whose return or
    /// parameter mentions a type parameter is resolved at its instance by the
    /// call-site substitution, exactly as the lowered walk does.
    pub(super) fn call_view_source<'e>(&self, callee: &'e Expr, args: &'e [Arg]) -> Option<ViewCallSource<'e>> {
        // A func-typed field is called through its own signature, which has no
        // receiver parameter; only a named method or a stdlib helper treats
        // the dot receiver as parameter zero.
        let field_call = self
            .callee_field_type(callee)
            .is_some_and(|ty| self.callable_type_signature(&ty).is_some());
        let is_dot = self.is_dot_call(callee) && !field_call;
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
        if let Some((params, returns)) = self.callable_call_signature(callee) {
            if !Self::returns_a_view(&returns) {
                return None;
            }
            // A capturing lambda's return may read a capture rather than a
            // parameter. The capture's backing is named by the enclosing
            // binding's provenance, so it takes precedence over the parameter
            // source the signature alone would suggest.
            let captures = self.lambda_capture_sources(callee);
            if let Some(source) = captures.into_iter().next() {
                return Some(source);
            }
            // A lambda's returned view may name one of several view
            // parameters. The body's return names the parameter it reads, so
            // the source resolves even when the signature alone lists more than
            // one lend candidate.
            if let Some(index) = self.lambda_returned_param_index(callee) {
                return source(index);
            }
            let effective = self.call_effective_params(callee, args, is_dot, &params);
            let include_take = super::lower::mentions_type_param(&returns);
            return self
                .single_lend_candidate(&effective, &params, include_take)
                .and_then(source);
        }
        let name = self.callee_name(callee)?;
        match self.resolved_callee(&name) {
            Some(Callee::User(decl)) => {
                let type_params: Vec<String> =
                    decl.type_params.iter().map(|p| p.name.clone()).collect();
                let returns = self.resolve_with(&decl.returns, &type_params);
                if !Self::returns_a_view(&returns) {
                    return None;
                }
                let params: Vec<(Mode, Ty)> = decl
                    .params
                    .iter()
                    .map(|param| (param.mode, self.resolve_with(&param.ty, &type_params)))
                    .collect();
                let effective = self.call_effective_params(callee, args, is_dot, &params);
                // A return that still mentions a type parameter may be a view
                // at its instance; the argument itself is then the only source
                // the call can name, including a `take` argument an identity
                // function hands back.
                let include_take = super::lower::mentions_type_param(&returns);
                let Some(index) =
                    self.single_lend_candidate(&effective, &params, include_take)
                else {
                    return None;
                };
                // The signature alone does not prove the return reads the
                // chosen lend: a helper returning static storage may have one
                // unused lend parameter. Use the same return-expression summary
                // the carrier path uses, so that argument is not a view source.
                let mentioned = body_return_flow_names(&decl.body);
                let reads_parameter = decl
                    .params
                    .get(index)
                    .is_some_and(|param| mentioned.contains(&param.name));
                if !reads_parameter {
                    return None;
                }
                source(index)
            }
            Some(Callee::Stdlib {
                effect: StdlibEffect::ViewOf { arg, .. },
                ..
            }) => source(arg),
            _ => None,
        }
    }

    /// True when a named user call's view return reads none of its parameters.
    ///
    /// The return-expression summary is the same one the carrier path uses. A
    /// helper that returns a static literal or a module value names no
    /// parameter, so its result is not a view of a lend argument; treating it
    /// as static keeps a static store accepted. A local return is rejected by
    /// the callee's own view-return rule.
    fn call_returns_static_view(&self, callee: &Expr, return_ty: &Ty) -> bool {
        if !is_view_like(return_ty) && !super::lower::mentions_type_param(return_ty) {
            return false;
        }
        if self.func_binding_callee(callee).is_some() || self.callee_field_type(callee).is_some()
        {
            return false;
        }
        let Some(name) = self.callee_name(callee) else {
            return false;
        };
        let Some(Callee::User(decl)) = self.resolved_callee(&name) else {
            return false;
        };
        // Every direct return may name a parameter and still hand back static
        // storage: `return h.tag` reads a `static str` field of a borrow. The
        // recorded type of each returned expression proves the value's
        // storage; one non-static return keeps the conservative answer.
        if body_returns_static_views(&decl.body, &self.sema.expr_types) {
            return true;
        }
        let mentioned = body_return_flow_names(&decl.body);
        decl.params
            .iter()
            .all(|param| !mentioned.contains(&param.name))
    }

    /// The backing of a capturing lambda's returned view.
    ///
    /// A lambda's return may read a capture rather than a parameter. The
    /// capture list names the enclosing bindings, and the enclosing checker
    /// still holds their provenance at the call site, so the returned view's
    /// backing resolves through the same place-provenance rule a local read
    /// uses. A conditional over lambdas contributes both arms' captures.
    fn lambda_capture_sources<'e>(&self, callee: &'e Expr) -> Vec<ViewCallSource<'e>> {
        match &super::expr::structural_wrapper_inner(callee).kind {
            ExprKind::Lambda { captures, body, .. } => {
                let mut mentioned = BTreeSet::new();
                collect_lambda_return_names(body, &mut mentioned);
                let mut sources = Vec::new();
                for capture in captures {
                    if !mentioned.contains(&capture.name) {
                        continue;
                    }
                    let Some(id) = self.lookup(&capture.name) else {
                        continue;
                    };
                    let provenance = self.place_provenance(id);
                    if provenance != Provenance::None {
                        sources.push(ViewCallSource::Capture(provenance));
                    }
                }
                sources
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                let mut sources = self.lambda_capture_sources(then_value);
                sources.extend(self.lambda_capture_sources(else_value));
                sources
            }
            _ => Vec::new(),
        }
    }

    /// The parameter a lambda's returned view derives from, when it names one.
    ///
    /// A lambda signature may list several view-like parameters; only the one
    /// the return expression names contributes the view. A conditional over
    /// lambdas resolves only when both arms name the same parameter position.
    fn lambda_returned_param_index(&self, callee: &Expr) -> Option<usize> {
        match &super::expr::structural_wrapper_inner(callee).kind {
            ExprKind::Lambda { params, body, .. } => {
                let mut mentioned = BTreeSet::new();
                collect_lambda_return_names(body, &mut mentioned);
                let mut indices = params
                    .iter()
                    .enumerate()
                    .filter(|(_, param)| mentioned.contains(&param.name))
                    .map(|(index, _)| index);
                let first = indices.next()?;
                indices.next().is_none().then_some(first)
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                let then_index = self.lambda_returned_param_index(then_value)?;
                let else_index = self.lambda_returned_param_index(else_value)?;
                (then_index == else_index).then_some(then_index)
            }
            _ => None,
        }
    }

    /// Every source a view-returning call may read.
    ///
    /// A named view return elides exactly one parameter. A lambda's return may
    /// read several parameters or captures, so every value-position source is
    /// collected; a destination judges each one. The single-source helper is
    /// the first component and keeps the named-function rule unchanged.
    pub(super) fn call_view_sources<'e>(
        &self,
        callee: &'e Expr,
        args: &'e [Arg],
    ) -> Vec<ViewCallSource<'e>> {
        let mut sources = Vec::new();
        if let Some(source) = self.call_view_source(callee, args) {
            sources.push(source);
        }
        for source in self.lambda_view_sources(callee, args) {
            sources.push(source);
        }
        sources
    }

    /// The parameters and captures a lambda's returned value reads.
    ///
    /// Only value positions count. A conditional over lambdas contributes both
    /// arms, and a non-lambda callee contributes nothing.
    fn lambda_view_sources<'e>(
        &self,
        callee: &'e Expr,
        args: &'e [Arg],
    ) -> Vec<ViewCallSource<'e>> {
        match &super::expr::structural_wrapper_inner(callee).kind {
            ExprKind::Lambda {
                captures,
                params,
                body,
                ..
            } => {
                let mut names = BTreeSet::new();
                collect_lambda_return_names(body, &mut names);
                let mut sources = Vec::new();
                for (index, param) in params.iter().enumerate() {
                    if !names.contains(&param.name) {
                        continue;
                    }
                    if let Some(arg) = args.get(index) {
                        sources.push(ViewCallSource::Expr(&arg.value));
                    }
                }
                for capture in captures {
                    if !names.contains(&capture.name) {
                        continue;
                    }
                    let Some(id) = self.lookup(&capture.name) else {
                        continue;
                    };
                    let provenance = self.place_provenance(id);
                    if provenance != Provenance::None {
                        sources.push(ViewCallSource::Capture(provenance));
                    }
                }
                sources
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                let mut sources = self.lambda_view_sources(then_value, args);
                sources.extend(self.lambda_view_sources(else_value, args));
                sources
            }
            _ => Vec::new(),
        }
    }

    /// The call at the root of a projection chain, when the chain has one.
    ///
    /// A carrier-returning call's view positions are read through a member or
    /// element projection exactly as the whole value is: `make(a.v).names`
    /// reads the returned carrier's `names` position. Wrappers contribute no
    /// storage of their own, so they are stripped too. A fixed array's `.len`
    /// is its structural extent, not a position, and yields no call root.
    pub(super) fn carrier_call_root<'e>(&self, expr: &'e Expr) -> Option<(&'e Expr, &'e [Arg])> {
        match &expr.kind {
            ExprKind::Call { callee, args, .. } => Some((callee, args)),
            ExprKind::Member { base, name } => {
                if name == "len" && self.ty(base).is_fixed_array() {
                    return None;
                }
                self.carrier_call_root(base)
            }
            ExprKind::Index { base, .. } => self.carrier_call_root(base),
            ExprKind::Unsafe(inner) => self.carrier_call_root(inner),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.carrier_call_root(value)
            }
            _ => None,
        }
    }

    /// The source arguments a carrier-returning call derives view positions
    /// from.
    ///
    /// A carrier return may hold a view of any argument whose storage a view
    /// can read: a view-like lend parameter, an owned aggregate with a
    /// handle-bearing field, or a `take` argument, whose storage the callee
    /// owns and may hand back through the returned value. Every such argument
    /// contributes its positions, so a call with several candidates records
    /// each one instead of contributing nothing. A view return is handled by
    /// [`Self::call_view_source`]; a call that cannot carry a view contributes
    /// no source.
    pub(super) fn call_carrier_sources<'e>(
        &self,
        callee: &'e Expr,
        args: &'e [Arg],
    ) -> Vec<ViewCallSource<'e>> {
        let field_call = self
            .callee_field_type(callee)
            .is_some_and(|ty| self.callable_type_signature(&ty).is_some());
        let is_dot = self.is_dot_call(callee) && !field_call;
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
        let (params, returns, filter) = if let Some((params, returns)) =
            self.callable_call_signature(callee)
        {
            let filter = match &super::expr::structural_wrapper_inner(callee).kind {
                ExprKind::Lambda { params, body, .. } => {
                    let mut mentioned = BTreeSet::new();
                    collect_lambda_body_names(body, &mut mentioned);
                    Some((
                        params.iter().map(|param| param.name.clone()).collect(),
                        mentioned,
                    ))
                }
                // A conditional or bound value callee may name its parameters
                // differently per arm; keep every candidate rather than filter
                // by a name that belongs to only one arm.
                _ => None,
            };
            (params, returns, filter)
        } else {
            let Some(name) = self.callee_name(callee) else {
                return Vec::new();
            };
            let Some(Callee::User(decl)) = self.resolved_callee(&name) else {
                return Vec::new();
            };
            let type_params: Vec<String> =
                decl.type_params.iter().map(|p| p.name.clone()).collect();
            let returns = self.resolve_with(&decl.returns, &type_params);
            let params: Vec<(Mode, Ty)> = decl
                .params
                .iter()
                .map(|param| (param.mode, self.resolve_with(&param.ty, &type_params)))
                .collect();
            let names: Vec<String> = decl
                .params
                .iter()
                .map(|param| param.name.clone())
                .collect();
            (params, returns, Some((names, body_return_flow_names(&decl.body))))
        };
        if is_view_like(&returns)
            || (!self.may_carry_view(&returns) && !super::lower::mentions_type_param(&returns))
        {
            return Vec::new();
        }
        let effective = self.call_effective_params(callee, args, is_dot, &params);
        let mut candidates = self.carrier_candidates(&effective, &params);
        // A parameter the callee's return expressions never mention cannot
        // contribute a position to the returned carrier. Filtering by that
        // summary keeps an unused owned argument from being treated as a
        // source, while a pass-through, a field read, and a call argument all
        // mention the parameter and stay candidates.
        if let Some((names, mentioned)) = filter {
            candidates.retain(|index| {
                names
                    .get(*index)
                    .is_some_and(|name| mentioned.contains(name))
            });
        }
        candidates.into_iter().filter_map(source).collect()
    }

    /// The parameter indices a carrier return may derive a view position from.
    ///
    /// A concrete call-site type decides: a view-like parameter or an owned
    /// aggregate whose storage a view can read is a candidate, and a `take`
    /// parameter is one too because the callee may hand its storage back. A
    /// call made inside a generic body whose argument still mentions a type
    /// parameter adds the declared parameter as well, so the per-instance
    /// judgment sees the same source its concrete twin does.
    fn carrier_candidates(&self, effective: &[(Mode, Ty)], declared: &[(Mode, Ty)]) -> Vec<usize> {
        let candidate = |ty: &Ty| is_lend_candidate(ty) || self.may_view_from_storage(ty);
        let mut candidates: Vec<usize> = effective
            .iter()
            .enumerate()
            .filter(|(_, (_, ty))| candidate(ty))
            .map(|(index, _)| index)
            .collect();
        let unresolved = effective
            .iter()
            .any(|(_, ty)| super::lower::mentions_type_param(ty));
        if unresolved {
            for (index, (_, ty)) in declared.iter().enumerate() {
                if super::lower::mentions_type_param(ty) && !candidates.contains(&index) {
                    candidates.push(index);
                }
            }
        }
        candidates
    }

    /// The single lend candidate a call may reborrow.
    ///
    /// A concrete lend candidate is authoritative: a type parameter's
    /// instantiation is unknown at the definition, and a call that also passes
    /// a concrete view names that view by the concrete parameter's position.
    /// Only when no concrete candidate exists does a parameter mention count,
    /// which covers a generic pass-through whose only lend is the type
    /// parameter itself. A return that mentions a type parameter may be a view
    /// at its instance, so the caller also admits a `take` argument, which an
    /// identity function hands back to the caller.
    fn single_lend_candidate(
        &self,
        effective: &[(Mode, Ty)],
        declared: &[(Mode, Ty)],
        include_take: bool,
    ) -> Option<usize> {
        let candidate = is_lend_candidate;
        let included = |mode: Mode| include_take || mode != Mode::Take;
        let concrete: Vec<usize> = effective
            .iter()
            .enumerate()
            .filter(|(_, (mode, ty))| included(*mode) && candidate(ty))
            .map(|(index, _)| index)
            .collect();
        // A concrete call-site argument type resolves the substitution; only
        // a call made inside a generic body, whose argument type still
        // mentions a parameter, falls back to the declared parameter type.
        let unresolved = effective
            .iter()
            .any(|(_, ty)| super::lower::mentions_type_param(ty));
        let candidates = if concrete.is_empty() && unresolved {
            declared
                .iter()
                .enumerate()
                .filter(|(_, (mode, ty))| {
                    included(*mode) && super::lower::mentions_type_param(ty)
                })
                .map(|(index, _)| index)
                .collect::<Vec<usize>>()
        } else {
            concrete
        };
        (candidates.len() == 1).then(|| candidates[0])
    }

    /// The parameter list with each written argument's call-site type.
    ///
    /// A generic declaration's parameter type is a type parameter, while the
    /// argument may already be a concrete view, an owned aggregate, or a
    /// scalar. Substituting the argument type at the call site lets the
    /// single-source walk judge the actual instantiation; a parameter whose
    /// argument the call does not write keeps its declared type.
    fn call_effective_params(
        &self,
        callee: &Expr,
        args: &[Arg],
        is_dot: bool,
        params: &[(Mode, Ty)],
    ) -> Vec<(Mode, Ty)> {
        let mut effective = params.to_vec();
        for (index, (_, ty)) in effective.iter_mut().enumerate() {
            let argument_ty = if is_dot && index == 0 {
                Some(self.dot_receiver_type(callee))
            } else {
                let written = if is_dot { index - 1 } else { index };
                args.get(written).map(|arg| self.ty(&arg.value))
            };
            if let Some(argument_ty) = argument_ty {
                if !argument_ty.is_unknown() {
                    *ty = argument_ty;
                }
            }
        }
        effective
    }

    /// The parameter list and return type of a value callable.
    ///
    /// A dot-call field, a call-returned callable, a `func`-typed binding, and
    /// a closure each carry their own signature. A `func`-typed parameter reads
    /// its bound's signature. A named module function is not a value callable
    /// and is resolved by its declaration instead.
    fn callable_call_signature(&self, callee: &Expr) -> Option<(Vec<(Mode, Ty)>, Ty)> {
        if let Some(field_ty) = self.callee_field_type(callee) {
            if let Some(signature) = self.callable_type_signature(&field_ty) {
                return Some(signature);
            }
        }
        if matches!(callee.kind, ExprKind::Call { .. }) {
            if let Some(signature) = self.callable_type_signature(&self.ty(callee)) {
                return Some(signature);
            }
        }
        // A lambda literal, a conditional over callables, a cast, and an
        // indexed callable all carry their signature on the expression's own
        // recorded type. The type stage types each of them, so the value
        // callee resolves exactly as a func-typed binding does.
        if matches!(
            callee.kind,
            ExprKind::Lambda { .. }
                | ExprKind::Conditional { .. }
                | ExprKind::Unsafe(_)
                | ExprKind::RawCast { .. }
                | ExprKind::Cast { .. }
                | ExprKind::Index { .. }
        ) {
            if let Some(signature) = self.callable_type_signature(&self.ty(callee)) {
                return Some(signature);
            }
        }
        let ExprKind::Name(written) = &callee.kind else {
            return None;
        };
        if let Some(id) = self.lookup(written) {
            return self.callable_type_signature(&self.bindings[id].ty);
        }
        // A bare name may name a module-level func-typed global; its value is
        // the callable, exactly as the call check resolves it.
        self.value_callee_global_type(callee)
            .and_then(|ty| self.callable_type_signature(&ty))
    }

    /// The signature a callable type spells, if any.
    fn callable_type_signature(&self, ty: &Ty) -> Option<(Vec<(Mode, Ty)>, Ty)> {
        match &ty.kind {
            TyKind::Func(signature) => Some((
                signature
                    .params
                    .iter()
                    .map(|param| (param.mode, param.ty.clone()))
                    .collect(),
                (*signature.returns).clone(),
            )),
            TyKind::Closure(closure) => Some((
                closure
                    .params
                    .iter()
                    .map(|param| (Mode::None, param.clone()))
                    .collect(),
                (*closure.returns).clone(),
            )),
            TyKind::Param(name) => {
                let bound = self
                    .decl
                    .type_params
                    .iter()
                    .find(|param| &param.name == name)?
                    .bound
                    .as_ref()?;
                Some((
                    bound
                        .params
                        .iter()
                        .map(|param| (Mode::None, self.resolve_with(param, &self.type_params)))
                        .collect(),
                    self.resolve_with(&bound.returns, &self.type_params),
                ))
            }
            _ => None,
        }
    }

    /// True when a callable return participates in view elision.
    ///
    /// A concrete view and a view-like fixed array participate directly. A
    /// type parameter is a view only at some instances, so the per-instance
    /// judgment decides; the call's single-source walk still names the
    /// argument, exactly as the lowered walk does.
    fn returns_a_view(ty: &Ty) -> bool {
        is_elided_view(ty) || ty.is_param() || super::lower::mentions_type_param(ty)
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
                // A module-qualified function value is a path, not a binding:
                // a module path wins over an imported value that shares the
                // head's spelling. Without this check the imported head would
                // project the remaining segments as fields of that value.
                if matches!(
                    self.resolved_callee(&name.segments.join(".")),
                    Some(Callee::User(_)) | Some(Callee::Ffi(_))
                ) {
                    return None;
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

    /// The segments of a dotted read, with the binding's identity.
    ///
    /// A dotted name reads a binding and the fields after it. The binding is
    /// the one the module-aware resolution reaches — a leading module path
    /// first, then a same-module or imported head — and its declaring module
    /// is spelled in front of it, so the emitter names the same binding the
    /// front end resolved even when several modules export the same name. A
    /// local, parameter, or unresolved path keeps its written segments.
    pub(super) fn global_head_segments(&self, segments: &[String]) -> Vec<String> {
        let Some((head, _)) = segments.split_first() else {
            return segments.to_vec();
        };
        if self.lookup(head).is_some() {
            return segments.to_vec();
        }
        let Some((id, fields)) = self.dotted_global(segments) else {
            return segments.to_vec();
        };
        let mut qualified: Vec<String> = id.module.split('.').map(str::to_owned).collect();
        qualified.push(id.name);
        qualified.extend(fields);
        qualified
    }

    /// The module binding a dotted path reaches, when the path resolves.
    ///
    /// The resolution is module-aware, exactly as the checker's is: a leading
    /// module path reaches the binding it declares, and otherwise the head is
    /// a same-module or imported binding; the segments after the binding are
    /// field projections.
    pub(super) fn dotted_global(&self, segments: &[String]) -> Option<(ir::GlobalId, Vec<String>)> {
        let name = crate::ast::QualifiedName {
            segments: segments.to_vec(),
            span: Span::default(),
        };
        let global = crate::init_graph::resolve_global(self.loaded, self.module, &name)?;
        Some((
            ir::GlobalId::new(global.module.clone(), global.name.clone()),
            global.fields,
        ))
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
    pub(super) fn narrowing(&self, cond: &Expr) -> Option<(NarrowOwner, Vec<ir::Projection>, bool)> {
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
        let (owner, projections) = match self.place_target(name)? {
            PlaceTarget::Local(id, projections) => (NarrowOwner::Local(id), projections),
            PlaceTarget::Global(id, projections) => (NarrowOwner::Global(id), projections),
        };
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
        Some((owner, projections, non_null))
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
        // A type-parameter return becomes a view at its view instances; the
        // declaration still collects the candidates so the instance audit can
        // apply the same one-candidate rule there.
        let type_param_return = super::lower::mentions_type_param(&self.returns);
        if !is_elided_view(&self.returns) && !type_param_return {
            return;
        }
        // The declaration-time count keeps the concrete twin's candidates: a
        // view/handle/array lend is a candidate, and an explicit `borrow T` is
        // a maybe-candidate. A plain type-parameter parameter is not counted
        // here because its concrete twin's plain argument may be a
        // non-candidate (a cstring helper's bool `y`); the per-instance duty
        // below re-counts every non-`take` parameter after substitution.
        let all_candidates: Vec<(String, Ty)> = self
            .decl
            .params
            .iter()
            .filter(|param| param.mode != Mode::Take)
            .map(|param| {
                (
                    param.name.clone(),
                    self.resolve_with(&param.ty, &self.type_params),
                )
            })
            .collect();
        let candidates: Vec<(String, Ty)> = all_candidates
            .iter()
            .filter(|(_, ty)| {
                is_lend_candidate(ty)
                    || (super::lower::mentions_type_param(ty))
            })
            .cloned()
            .collect();
        self.elision_candidates = candidates.len();
        self.elision_candidate = if candidates.len() == 1 {
            Some(candidates[0].0.clone())
        } else {
            None
        };
        // A `borrow T` candidate is only a lend at the instances where T is a
        // view-like type. The declaration records the candidates and the
        // returned parameter; the instance audit re-counts them after the
        // substitution. A concrete-only candidate set stays a declaration-time
        // judgment.
        self.elision_type_param_candidate = type_param_return
            || all_candidates
                .iter()
                .any(|(_, ty)| super::lower::mentions_type_param(ty));
        self.elision_candidate_types = all_candidates;
    }

    /// Reports `E10036` when a view return cannot elide a single parameter.
    ///
    /// The count decides the zero-candidate and two-or-more cases; with
    /// exactly one candidate the return's observed derivation must name it,
    /// so a view read through a different parameter is rejected here at the
    /// declaration instead of leaking a borrowed view to the caller.
    pub(super) fn elision_postlude(&mut self) {
        if !is_elided_view(&self.returns)
            && !super::lower::mentions_type_param(&self.returns)
        {
            return;
        }
        // A candidate set that still mentions a type parameter is judged per
        // instance: the substituted type decides whether the `borrow T` is a
        // view-like lend. The duty carries the candidates and the returned
        // parameter; a return that derives from no parameter (a local or a
        // static value) keeps its own report and owes no candidate duty.
        if self.elision_type_param_candidate {
            let returned = self.view_return_source.clone().and_then(|returned| {
                if self.view_return_source_mixed {
                    None
                } else {
                    Some(returned)
                }
            });
            if self.view_return_source.is_some()
                || self.view_return_local_owner.is_some()
                || self.view_return_fresh
            {
                self.view_return_requirements
                    .push(ir::RequiresViewReturnCandidates {
                        function: self.decl.name.clone(),
                        candidates: self.elision_candidate_types.clone(),
                        returned,
                        returns: self.view_return_paths.clone(),
                        return_ty: self.returns.clone(),
                        local_owner: self.view_return_local_owner.clone(),
                        fresh_return: self.view_return_fresh,
                        span: self.decl.name_span,
                    });
            }
            return;
        }
        // A return that derives from the wrong source is the per-return
        // E10036 judgment. A static or local return in another branch must not
        // hide it: those branches have their own reports, but this branch's
        // view still needs an elision candidate.
        if self.return_derivation_mismatch {
            self.error(
                "E10036",
                "view return does not derive from exactly one parameter",
                "make exactly one parameter a view or handle lend",
                self.decl.name_span,
            );
            return;
        }
        if self.suppress_elision {
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

/// The source expression a view-returning call reborrows.
///
/// A written argument or a `Member` receiver keeps its expression, so a walk
/// can descend into it arm by arm; a qualified receiver names a place rather
/// than a written expression, so its provenance is carried instead. A captured
/// lambda return has no argument expression at all, so the capture's backing
/// provenance is carried too.
pub(super) enum ViewCallSource<'e> {
    /// A written argument or a `Member` receiver's base expression.
    Expr(&'e Expr),
    /// A qualified receiver's place provenance.
    Receiver(Provenance),
    /// A capturing lambda's return-value backing.
    Capture(Provenance),
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

/// True when a write clears a recorded moved or narrowed path.
///
/// A write to a path replaces that path and everything below it, but a strict
/// prefix of the write stays valid: writing `h.name.x` re-initializes `x`
/// while `h.name` itself is still the value that was moved or narrowed. An
/// unrelated path is untouched. A run-time index writes any element, so it
/// clears every element path at that position; a constant element path keeps
/// its own identity.
fn written_path_clears(written: &[ir::Projection], path: &[ir::Projection]) -> bool {
    if path.len() < written.len() && written_starts_with(written, path) {
        return false;
    }
    written_starts_with(path, written)
}

/// True when a path starts with a written prefix, wildcard-aware.
///
/// A written run-time element (`Element(None)` or `ElementTemp`) selects any
/// element, so it matches every element step of the recorded path. Every other
/// step compares structurally.
fn written_starts_with(path: &[ir::Projection], written: &[ir::Projection]) -> bool {
    path.len() >= written.len()
        && written
            .iter()
            .zip(path)
            .all(|(written, path)| written_step_matches(written, path))
}

/// True when one written step matches one recorded path step.
fn written_step_matches(written: &ir::Projection, path: &ir::Projection) -> bool {
    written == path
        || (matches!(
            written,
            ir::Projection::Element(None) | ir::Projection::ElementTemp(_)
        ) && matches!(
            path,
            ir::Projection::Element(_) | ir::Projection::ElementTemp(_)
        ))
}

/// The parameter names a function body's return values may read.
///
/// A carrier return can hand back a view position of a parameter the body
/// reads, directly or through a local that carries the view. The summary starts
/// from the names every return expression mentions and propagates backward
/// through `let`, `const`, and assignment initializers until it stops growing.
/// A parameter that only feeds an unrelated computation is not in the set, so
/// it is not treated as a carrier source; a view carried through a local still
/// reaches its parameter.
fn body_return_flow_names(body: &Block) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_return_block_names(body, &mut names);
    loop {
        let mut changed = false;
        propagate_block_names(body, &mut names, &mut changed);
        if !changed {
            break;
        }
    }
    names
}

/// Propagates a live name set backward through a block's initializers.
fn propagate_block_names(block: &Block, live: &mut BTreeSet<String>, changed: &mut bool) {
    for statement in &block.statements {
        propagate_stmt_names(statement, live, changed);
    }
}

/// Propagates a live name set backward through one statement.
fn propagate_stmt_names(stmt: &Stmt, live: &mut BTreeSet<String>, changed: &mut bool) {
    match stmt {
        Stmt::Let(decl) => {
            if let Some(value) = &decl.value {
                propagate_initializer_names(&decl.name, value, live, changed);
            }
        }
        Stmt::Const(decl) => {
            propagate_initializer_names(&decl.name, &decl.value, live, changed);
        }
        Stmt::Expr(expr) => propagate_expr_names(expr, live, changed),
        Stmt::If(decl) => {
            propagate_expr_names(&decl.condition, live, changed);
            propagate_if_names(decl, live, changed);
        }
        Stmt::While(decl) => {
            propagate_expr_names(&decl.condition, live, changed);
            propagate_block_names(&decl.body, live, changed);
        }
        Stmt::DoWhile(decl) => {
            propagate_block_names(&decl.body, live, changed);
            propagate_expr_names(&decl.condition, live, changed);
        }
        Stmt::ForC(decl) => {
            match &decl.init {
                Some(ForInit::Expr(expressions)) => {
                    for expression in expressions {
                        propagate_expr_names(expression, live, changed);
                    }
                }
                Some(ForInit::Let(binding)) => {
                    if let Some(value) = &binding.value {
                        propagate_initializer_names(&binding.name, value, live, changed);
                    }
                }
                Some(ForInit::Const(binding)) => {
                    propagate_initializer_names(&binding.name, &binding.value, live, changed);
                }
                None => {}
            }
            if let Some(condition) = &decl.condition {
                propagate_expr_names(condition, live, changed);
            }
            for step in &decl.step {
                propagate_expr_names(step, live, changed);
            }
            propagate_block_names(&decl.body, live, changed);
        }
        Stmt::ForIn(decl) => {
            // The loop binding aliases the iterable's element, so a live use of
            // the alias reaches the iterable's names.
            propagate_initializer_names(&decl.name, &decl.iterable, live, changed);
            propagate_block_names(&decl.body, live, changed);
        }
        Stmt::Switch(decl) => {
            propagate_expr_names(&decl.scrutinee, live, changed);
            for clause in &decl.clauses {
                for statement in &clause.statements {
                    propagate_stmt_names(statement, live, changed);
                }
            }
        }
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
            propagate_block_names(block, live, changed)
        }
        _ => {}
    }
}

/// Propagates a live name set through an `if` statement and any chained
/// `else if`.
fn propagate_if_names(decl: &IfStmt, live: &mut BTreeSet<String>, changed: &mut bool) {
    propagate_block_names(&decl.then_block, live, changed);
    match &decl.else_branch {
        Some(ElseBranch::If(nested)) => {
            propagate_expr_names(&nested.condition, live, changed);
            propagate_if_names(nested, live, changed);
        }
        Some(ElseBranch::Block(block)) => propagate_block_names(block, live, changed),
        None => {}
    }
}

/// Propagates a live name set through assignments nested in an expression.
fn propagate_expr_names(expr: &Expr, live: &mut BTreeSet<String>, changed: &mut bool) {
    match &expr.kind {
        ExprKind::Assign { target, value, .. } => {
            // A projected assignment writes the root binding, so a live root
            // makes the assigned value's names live too.
            if let Some(root) = assignment_root_name(target) {
                propagate_initializer_names(root, value, live, changed);
            }
            propagate_expr_names(target, live, changed);
            propagate_expr_names(value, live, changed);
        }
        ExprKind::Call { callee, args, .. } => {
            propagate_expr_names(callee, live, changed);
            for arg in args {
                propagate_expr_names(&arg.value, live, changed);
            }
        }
        ExprKind::Index { base, index } => {
            propagate_expr_names(base, live, changed);
            propagate_expr_names(index, live, changed);
        }
        ExprKind::Member { base, .. } => propagate_expr_names(base, live, changed),
        ExprKind::Unary { operand, .. } => propagate_expr_names(operand, live, changed),
        ExprKind::Binary { left, right, .. } => {
            propagate_expr_names(left, live, changed);
            propagate_expr_names(right, live, changed);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            propagate_expr_names(condition, live, changed);
            propagate_expr_names(then_value, live, changed);
            propagate_expr_names(else_value, live, changed);
        }
        ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
            propagate_expr_names(value, live, changed)
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                propagate_expr_names(&field.value, live, changed);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                propagate_expr_names(element, live, changed);
            }
            if let Some(fill) = fill {
                propagate_expr_names(fill, live, changed);
            }
        }
        ExprKind::Unsafe(inner) => propagate_expr_names(inner, live, changed),
        ExprKind::IntrinsicCall { args, .. } => {
            for arg in args {
                if let IntrinsicArg::Expr(inner) = arg {
                    propagate_expr_names(inner, live, changed);
                }
            }
        }
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let TemplatePiece::Placeholder(inner) = piece {
                    propagate_expr_names(inner, live, changed);
                }
            }
        }
        _ => {}
    }
}

/// The root binding name of an assignment target, when it has one.
fn assignment_root_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Name(name) => Some(name),
        ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
            assignment_root_name(base)
        }
        ExprKind::Qualified(name) => name.segments.first().map(|segment| segment.as_str()),
        ExprKind::Unsafe(inner) => assignment_root_name(inner),
        _ => None,
    }
}

/// Adds an initializer's names when the defined name is live.
///
/// The propagation is backward: a local used by a return makes the names its
/// initializer reads live too, so a parameter that reaches the return through
/// a local stays a carrier source. The reverse direction would only mark the
/// local from an unrelated read.
fn propagate_initializer_names(
    defined: &str,
    value: &Expr,
    live: &mut BTreeSet<String>,
    changed: &mut bool,
) {
    if !live.contains(defined) {
        return;
    }
    let mut names = BTreeSet::new();
    collect_expr_names(value, &mut names);
    for name in names {
        *changed |= live.insert(name);
    }
}

/// Collects every name a block mentions.
fn collect_block_names(block: &Block, names: &mut BTreeSet<String>) {
    for statement in &block.statements {
        collect_stmt_names(statement, names);
    }
}

/// Collects every name one statement mentions.
fn collect_stmt_names(stmt: &Stmt, names: &mut BTreeSet<String>) {
    match stmt {
        Stmt::Let(decl) => {
            if let Some(value) = &decl.value {
                collect_expr_names(value, names);
            }
        }
        Stmt::Const(decl) => collect_expr_names(&decl.value, names),
        Stmt::Expr(expr) => collect_expr_names(expr, names),
        Stmt::If(decl) => collect_if_names(decl, names),
        Stmt::While(decl) => {
            collect_expr_names(&decl.condition, names);
            collect_block_names(&decl.body, names);
        }
        Stmt::DoWhile(decl) => {
            collect_block_names(&decl.body, names);
            collect_expr_names(&decl.condition, names);
        }
        Stmt::ForC(decl) => {
            match &decl.init {
                Some(ForInit::Expr(expressions)) => {
                    for expression in expressions {
                        collect_expr_names(expression, names);
                    }
                }
                Some(ForInit::Let(binding)) => {
                    if let Some(value) = &binding.value {
                        collect_expr_names(value, names);
                    }
                }
                Some(ForInit::Const(binding)) => collect_expr_names(&binding.value, names),
                None => {}
            }
            if let Some(condition) = &decl.condition {
                collect_expr_names(condition, names);
            }
            for step in &decl.step {
                collect_expr_names(step, names);
            }
            collect_block_names(&decl.body, names);
        }
        Stmt::ForIn(decl) => {
            collect_expr_names(&decl.iterable, names);
            collect_block_names(&decl.body, names);
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                collect_expr_names(value, names);
            }
        }
        Stmt::Switch(decl) => {
            collect_expr_names(&decl.scrutinee, names);
            for clause in &decl.clauses {
                for statement in &clause.statements {
                    collect_stmt_names(statement, names);
                }
            }
        }
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
            collect_block_names(block, names)
        }
        Stmt::Break(_) | Stmt::Continue(_) => {}
    }
}

/// Collects every name an `if` statement mentions, including a chained
/// `else if`.
fn collect_if_names(decl: &IfStmt, names: &mut BTreeSet<String>) {
    collect_expr_names(&decl.condition, names);
    collect_block_names(&decl.then_block, names);
    match &decl.else_branch {
        Some(ElseBranch::If(nested)) => collect_if_names(nested, names),
        Some(ElseBranch::Block(block)) => collect_block_names(block, names),
        None => {}
    }
}

/// Collects every name an expression mentions.
fn collect_expr_names(expr: &Expr, names: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name(name) => {
            names.insert(name.clone());
        }
        ExprKind::Qualified(name) => {
            if let Some(first) = name.segments.first() {
                names.insert(first.clone());
            }
        }
        ExprKind::Call { callee, args, .. } => {
            collect_expr_names(callee, names);
            for arg in args {
                collect_expr_names(&arg.value, names);
            }
        }
        ExprKind::Index { base, index } => {
            collect_expr_names(base, names);
            collect_expr_names(index, names);
        }
        ExprKind::Member { base, .. } => collect_expr_names(base, names),
        ExprKind::Unary { operand, .. } => collect_expr_names(operand, names),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_names(left, names);
            collect_expr_names(right, names);
        }
        ExprKind::Assign { target, value, .. } => {
            collect_expr_names(target, names);
            collect_expr_names(value, names);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            collect_expr_names(condition, names);
            collect_expr_names(then_value, names);
            collect_expr_names(else_value, names);
        }
        ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
            collect_expr_names(value, names)
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_expr_names(&field.value, names);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                collect_expr_names(element, names);
            }
            if let Some(fill) = fill {
                collect_expr_names(fill, names);
            }
        }
        // A returned lambda's captures are read in its body, so the body's
        // names are part of what the returned value may hold.
        ExprKind::Lambda { body, .. } => match body {
            LambdaBody::Expr(value) => collect_expr_names(value, names),
            LambdaBody::Block(block) => collect_block_names(block, names),
        },
        ExprKind::Unsafe(inner) => collect_expr_names(inner, names),
        ExprKind::IntrinsicCall { args, .. } => {
            for arg in args {
                if let IntrinsicArg::Expr(inner) = arg {
                    collect_expr_names(inner, names);
                }
            }
        }
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let TemplatePiece::Placeholder(inner) = piece {
                    collect_expr_names(inner, names);
                }
            }
        }
        _ => {}
    }
}

/// Collects the names a lambda body's return value mentions.
fn collect_lambda_body_names(body: &LambdaBody, names: &mut BTreeSet<String>) {
    match body {
        LambdaBody::Expr(value) => collect_expr_names(value, names),
        LambdaBody::Block(block) => collect_block_names(block, names),
    }
}

/// Collects the names a lambda body's return value mentions.
///
/// Only value positions count: a conditional's condition and an index or
/// callee expression never supply the returned storage, so a capture used only
/// there must not displace the parameter the value actually reads.
fn collect_lambda_return_names(body: &LambdaBody, names: &mut BTreeSet<String>) {
    match body {
        LambdaBody::Expr(value) => collect_expr_value_names(value, names),
        LambdaBody::Block(block) => collect_return_value_block_names(block, names),
    }
}

/// Collects every name one expression mentions in a value position.
fn collect_expr_value_names(expr: &Expr, names: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name(name) => {
            names.insert(name.clone());
        }
        ExprKind::Qualified(name) => {
            if let Some(first) = name.segments.first() {
                names.insert(first.clone());
            }
        }
        // The chosen arm supplies the returned value; the condition does not.
        ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } => {
            collect_expr_value_names(then_value, names);
            collect_expr_value_names(else_value, names);
        }
        // A callee is a function value, not the returned storage; the written
        // arguments may carry the view a helper reborrows.
        ExprKind::Call { args, .. } => {
            for arg in args {
                collect_expr_value_names(&arg.value, names);
            }
        }
        // The base names the storage; the index does not.
        ExprKind::Index { base, .. } => collect_expr_value_names(base, names),
        ExprKind::Member { base, .. } => collect_expr_value_names(base, names),
        ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
            collect_expr_value_names(value, names)
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_expr_value_names(&field.value, names);
            }
        }
        ExprKind::ArrayLiteral { elements, fill } => {
            for element in elements {
                collect_expr_value_names(element, names);
            }
            if let Some(fill) = fill {
                collect_expr_value_names(fill, names);
            }
        }
        ExprKind::Unsafe(inner) => collect_expr_value_names(inner, names),
        ExprKind::Template(pieces) => {
            for piece in pieces {
                if let TemplatePiece::Placeholder(inner) = piece {
                    collect_expr_value_names(inner, names);
                }
            }
        }
        _ => {}
    }
}

/// Collects the names every `return` in a block mentions in a value position.
fn collect_return_value_block_names(block: &Block, names: &mut BTreeSet<String>) {
    for statement in &block.statements {
        collect_return_value_stmt_names(statement, names);
    }
}

/// Collects the names one statement's reachable returns mention in a value
/// position.
fn collect_return_value_stmt_names(stmt: &Stmt, names: &mut BTreeSet<String>) {
    match stmt {
        Stmt::Return {
            value: Some(value), ..
        } => collect_expr_value_names(value, names),
        Stmt::If(decl) => collect_return_value_if_names(decl, names),
        Stmt::While(decl) => collect_return_value_block_names(&decl.body, names),
        Stmt::DoWhile(decl) => collect_return_value_block_names(&decl.body, names),
        Stmt::ForC(decl) => collect_return_value_block_names(&decl.body, names),
        Stmt::ForIn(decl) => collect_return_value_block_names(&decl.body, names),
        Stmt::Switch(decl) => {
            for clause in &decl.clauses {
                for statement in &clause.statements {
                    collect_return_value_stmt_names(statement, names);
                }
            }
        }
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
            collect_return_value_block_names(block, names)
        }
        _ => {}
    }
}

/// Collects the names an `if` statement's returns mention in a value position,
/// including a chained `else if`.
fn collect_return_value_if_names(decl: &IfStmt, names: &mut BTreeSet<String>) {
    collect_return_value_block_names(&decl.then_block, names);
    match &decl.else_branch {
        Some(ElseBranch::If(nested)) => collect_return_value_if_names(nested, names),
        Some(ElseBranch::Block(block)) => collect_return_value_block_names(block, names),
        None => {}
    }
}

/// True when every return in `body` hands back static storage.
///
/// The recorded type of each returned expression decides: a `static str` (or
/// a fixed array of static views) points at module-lifetime bytes, even when
/// the expression reads a field of a borrow parameter. A body with no return,
/// or with any return outside that shape, keeps the conservative answer. The
/// walk visits only this function's own returns; a lambda body's returns
/// belong to the lifted function.
fn body_returns_static_views(
    body: &Block,
    expr_types: &std::collections::HashMap<usize, Ty>,
) -> bool {
    fn static_view(ty: &Ty) -> bool {
        match &ty.kind {
            TyKind::StaticStr => true,
            TyKind::Array { element, .. } => static_view(element),
            _ => false,
        }
    }
    fn walk_statements(
        statements: &[Stmt],
        expr_types: &std::collections::HashMap<usize, Ty>,
        returns: &mut usize,
        all_static: &mut bool,
    ) {
        for statement in statements {
            match statement {
                Stmt::Return {
                    value: Some(value), ..
                } => {
                    *returns += 1;
                    if !expr_types
                        .get(&(value as *const Expr as usize))
                        .is_some_and(static_view)
                    {
                        *all_static = false;
                    }
                }
                Stmt::Return { value: None, .. } => {
                    *returns += 1;
                    *all_static = false;
                }
                Stmt::If(decl) => {
                    walk_statements(
                        &decl.then_block.statements,
                        expr_types,
                        returns,
                        all_static,
                    );
                    let mut branch = &decl.else_branch;
                    while let Some(ElseBranch::If(nested)) = branch {
                        walk_statements(
                            &nested.then_block.statements,
                            expr_types,
                            returns,
                            all_static,
                        );
                        branch = &nested.else_branch;
                    }
                    if let Some(ElseBranch::Block(block)) = branch {
                        walk_statements(&block.statements, expr_types, returns, all_static);
                    }
                }
                Stmt::While(decl) => {
                    walk_statements(&decl.body.statements, expr_types, returns, all_static)
                }
                Stmt::DoWhile(decl) => {
                    walk_statements(&decl.body.statements, expr_types, returns, all_static)
                }
                Stmt::ForC(decl) => {
                    walk_statements(&decl.body.statements, expr_types, returns, all_static)
                }
                Stmt::ForIn(decl) => {
                    walk_statements(&decl.body.statements, expr_types, returns, all_static)
                }
                Stmt::Switch(decl) => {
                    for clause in &decl.clauses {
                        walk_statements(&clause.statements, expr_types, returns, all_static);
                    }
                }
                Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
                    walk_statements(&block.statements, expr_types, returns, all_static)
                }
                _ => {}
            }
        }
    }
    let mut returns = 0usize;
    let mut all_static = true;
    walk_statements(&body.statements, expr_types, &mut returns, &mut all_static);
    returns > 0 && all_static
}

/// Collects the names every `return` in a block mentions.
fn collect_return_block_names(block: &Block, names: &mut BTreeSet<String>) {
    for statement in &block.statements {
        collect_return_stmt_names(statement, names);
    }
}

/// Collects the names one statement's reachable returns mention.
fn collect_return_stmt_names(stmt: &Stmt, names: &mut BTreeSet<String>) {
    match stmt {
        Stmt::Return {
            value: Some(value), ..
        } => collect_expr_names(value, names),
        Stmt::If(decl) => collect_return_if_names(decl, names),
        Stmt::While(decl) => collect_return_block_names(&decl.body, names),
        Stmt::DoWhile(decl) => collect_return_block_names(&decl.body, names),
        Stmt::ForC(decl) => collect_return_block_names(&decl.body, names),
        Stmt::ForIn(decl) => collect_return_block_names(&decl.body, names),
        Stmt::Switch(decl) => {
            for clause in &decl.clauses {
                for statement in &clause.statements {
                    collect_return_stmt_names(statement, names);
                }
            }
        }
        Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
            collect_return_block_names(block, names)
        }
        _ => {}
    }
}

/// Collects the names an `if` statement's returns mention, including a
/// chained `else if`.
fn collect_return_if_names(decl: &IfStmt, names: &mut BTreeSet<String>) {
    collect_return_block_names(&decl.then_block, names);
    match &decl.else_branch {
        Some(ElseBranch::If(nested)) => collect_return_if_names(nested, names),
        Some(ElseBranch::Block(block)) => collect_return_block_names(block, names),
        None => {}
    }
}

/// True when a recorded position path starts with a read path.
///
/// A recorded `Element(None)` step names any element, because a run-time index
/// cannot record the slot it wrote; every other step compares structurally.
/// The read is constant whenever this test runs, so only the recorded step can
/// be the wildcard.
fn position_starts_with(path: &[ir::Projection], read: &[ir::Projection]) -> bool {
    path.len() >= read.len()
        && path.iter().zip(read).all(|(recorded, read)| position_step_matches(recorded, read))
}

/// True when a recorded position path covers a read path.
///
/// The mirror of [`position_starts_with`], used where a fill record covers the
/// element read through it. The recorded step still supplies the wildcard.
fn position_covers(path: &[ir::Projection], read: &[ir::Projection]) -> bool {
    read.len() >= path.len()
        && path.iter().zip(read).all(|(recorded, read)| position_step_matches(recorded, read))
}

/// True when one recorded position step matches one read step.
///
/// Only a recorded wildcard is broad: a read whose own path carries a run-time
/// index keeps the whole-read union instead of matching step by step.
fn position_step_matches(recorded: &ir::Projection, read: &ir::Projection) -> bool {
    recorded == read
        || (matches!(recorded, ir::Projection::Element(None))
            && matches!(read, ir::Projection::Element(Some(_))))
}

/// The recorded path's steps below a constant read, when the read selects them.
///
/// A recorded wildcard step matches the read's element at the same position, so
/// a position written through a run-time index still transfers under the
/// constant prefix that reaches it.
fn position_rest<'path>(
    path: &'path [ir::Projection],
    read: &[ir::Projection],
) -> Option<&'path [ir::Projection]> {
    position_starts_with(path, read).then(|| &path[read.len()..])
}

/// True when every projection step names one storage position.
///
/// A run-time index and an element materialization name no single position, so
/// a path over one cannot key or translate a loan record: such a read
/// contributes every recorded position instead.
pub(super) fn constant_path(projections: &[ir::Projection]) -> bool {
    projections.iter().all(|projection| {
        !matches!(
            projection,
            ir::Projection::Element(None) | ir::Projection::ElementTemp(_)
        )
    })
}

/// True when a value is a projection chain rooted at a raw dereference.
///
/// A `(*p).field` or `(*p).items[i].field` read names the caller's pointee
/// exactly as the bare `*p` read does, so every move and release rule that
/// treats a deref read as caller storage must see the whole chain. The walk
/// follows only projections: a field of a fresh call result (`mk().field`)
/// roots at the call and owns its own storage.
pub(super) fn deref_rooted(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Unary {
            op: UnaryOp::Deref,
            ..
        } => true,
        ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => deref_rooted(base),
        _ => false,
    }
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

/// The view-position loans a copy source hands its destination.
///
/// A copied value contributes two kinds of position: a loan the source
/// already recorded, keyed by the destination path it translates to, and the
/// owner of a position the copy derives directly, which creates its shared
/// loan at the copy site. Keeping both apart until the destination record is
/// written lets the copy union every arm before it replaces the old record.
#[derive(Default)]
pub(super) struct CopyLoans {
    /// Recorded source loans, translated onto the destination's paths.
    recorded: BTreeMap<Vec<ir::Projection>, BTreeSet<LoanId>>,
    /// Owners of positions the copy derives directly.
    owners: BTreeMap<Vec<ir::Projection>, BTreeSet<LoanOwner>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use ir::Projection::{Element, Field};

    /// The recorded position is the broad side of the match.
    ///
    /// A store through a run-time index records the wildcard step, so the
    /// recorded step matches any constant element index. The read side is
    /// always constant: a read wildcard is not recorded and must not match a
    /// concrete recorded step.
    #[test]
    fn position_step_matches_accepts_only_a_recorded_wildcard() {
        assert!(position_step_matches(&Element(None), &Element(Some(3))));
        assert!(position_step_matches(&Element(Some(3)), &Element(Some(3))));
        assert!(!position_step_matches(&Element(Some(3)), &Element(None)));
        assert!(!position_step_matches(&Element(Some(1)), &Element(Some(2))));
        assert!(!position_step_matches(
            &Field("rows".to_owned()),
            &Element(Some(0))
        ));
    }

    /// A path matches itself at equal length, and the remainder is empty.
    #[test]
    fn position_prefix_and_cover_agree_at_equal_length() {
        let path = [Element(Some(0)), Field("rows".to_owned())];
        assert!(position_starts_with(&path, &path));
        assert!(position_covers(&path, &path));
        assert_eq!(position_rest(&path, &path), Some(&[][..]));
    }

    /// A recorded path below the read starts with it; the reader keeps the
    /// steps below the read to translate them onto the destination.
    #[test]
    fn position_starts_with_keeps_the_steps_below_the_read() {
        let path = [Element(None), Element(Some(2))];
        let read = [Element(Some(1))];
        assert!(position_starts_with(&path, &read));
        assert!(!position_covers(&path, &read));
        assert_eq!(position_rest(&path, &read), Some(&[Element(Some(2))][..]));
        assert!(position_starts_with(&[Element(Some(0))], &[]));
        assert!(!position_starts_with(&read, &path));
    }

    /// A recorded fill prefix covers a deeper read; the read selects one
    /// element of what the fill stored.
    #[test]
    fn position_covers_accepts_a_recorded_fill_prefix() {
        let path = [Element(Some(0))];
        let read = [Element(Some(0)), Element(Some(1))];
        assert!(position_covers(&path, &read));
        assert!(!position_starts_with(&path, &read));
        assert_eq!(position_rest(&path, &read), None);
    }

    /// A read wildcard never matches a concrete recorded step.
    ///
    /// The readers only pass constant read paths; this pins the direction so a
    /// later reader cannot broaden a recorded concrete position by mistake.
    #[test]
    fn position_read_wildcard_is_not_recorded() {
        assert!(!position_starts_with(&[Element(Some(0))], &[Element(None)]));
        assert!(!position_covers(
            &[Element(Some(0)), Element(Some(1))],
            &[Element(None)]
        ));
        assert_eq!(position_rest(&[Element(Some(0))], &[Element(None)]), None);
    }
}

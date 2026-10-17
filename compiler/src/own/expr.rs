//! Expression checking and call handling for the ownership stage.
//!
//! This walk reads every expression with the types the type stage
//! recorded, applies the move and initialization rules to reads, assigns
//! each assignment its place rules, and checks calls against the declared
//! parameter modes, the `ffi` contracts, and the standard-library view
//! helpers.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    // -- expressions ---------------------------------------------------------

    /// The recorded type of an expression.
    ///
    /// The type stage records every expression the ownership walk can reach,
    /// so a miss means the two stages disagree about the tree shape. That is
    /// an internal error: it must not silently become the opaque type, because
    /// the opaque type disables every ownership check.
    pub(super) fn ty(&self, expr: &Expr) -> Ty {
        self.sema
            .expr_types
            .get(&(expr as *const Expr as usize))
            .cloned()
            .expect("internal error: the type stage recorded no type for this expression")
    }

    /// The recorded type of an expression, when the type stage recorded one.
    ///
    /// Unlike [`Self::ty`], this never panics: a callee expression such as a
    /// function name is not always recorded, so a caller that only asks whether
    /// the callee is a closure reads it through this method.
    pub(super) fn recorded_ty(&self, expr: &Expr) -> Option<Ty> {
        self.sema
            .expr_types
            .get(&(expr as *const Expr as usize))
            .cloned()
    }

    /// Records a value-consuming read of an owned place as a move.
    ///
    /// A struct-literal field consumes its value: an owned (non-duplicable)
    /// place is moved out, so the source field must be treated as moved. A
    /// duplicable place is copied, and a type that still mentions a generic
    /// parameter is left to the instantiation, matching the lowering's guard.
    /// The move makes a second read the moved-value error and a
    /// deeper-than-top-level path the v1 partial-move error.
    fn consume_owned_place(&mut self, expr: &Expr, span: Span) {
        let ty = self.ty(expr);
        let target = self.place_target(expr);
        // A move through a borrow element view is the v1 element-move limit
        // once the moved place owns; while its type is still a parameter the
        // site records the duty for the per-instance audit.
        if let Some(PlaceTarget::Local(id, projections)) = &target {
            self.note_borrowed_element_move(*id, projections, &ty, span);
        }
        if self.maybe_owns(&ty) {
            // The field store writes a move for an owning instance, so a later
            // use of the source deposits the duplicability duty.
            if let Some(target) = target {
                self.mark_moved_if_owning(target, span);
            }
            return;
        }
        if self.sema.table.duplicable(&ty) {
            return;
        }
        if let Some(target) = target {
            self.move_target(target, span);
        } else if self.is_ownerless_element_move(expr) {
            self.report_element_move_limit(span);
        }
    }

    /// The whole-binding places a maybe-owning conditional may consume.
    ///
    /// Mirrors [`Self::conditional_move_sources`] for a type that mentions a
    /// parameter: every arm whose type may own becomes a source, so the caller
    /// can mark it consumed without rejecting a duplicable instance. A nested
    /// conditional recurses.
    pub(super) fn maybe_owning_move_sources(&self, expr: &Expr) -> Vec<PlaceTarget> {
        let ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } = &expr.kind
        else {
            return Vec::new();
        };
        let mut sources = Vec::new();
        for arm in [then_value.as_ref(), else_value.as_ref()] {
            if matches!(arm.kind, ExprKind::Conditional { .. }) {
                sources.extend(self.maybe_owning_move_sources(arm));
                continue;
            }
            let ty = self.ty(arm);
            if !self.maybe_owns(&ty) {
                continue;
            }
            if let Some(target) = self.place_target(arm) {
                sources.push(target);
            }
        }
        sources
    }

    /// The places a still-unresolved conditional's arms name, with each arm's
    /// recorded type.
    ///
    /// A `take` argument whose member type is unresolved selects one arm at
    /// run time, so each arm is judged per instance: the caller marks an arm
    /// that owns its value for its per-instance move, and an arm rooted in a
    /// borrow element records the element-move duty. A nested conditional
    /// recurses; an arm that names no place contributes nothing. `None` when
    /// the expression is not a conditional.
    pub(super) fn unresolved_conditional_place_arms(
        &self,
        expr: &Expr,
    ) -> Option<Vec<(PlaceTarget, Ty)>> {
        let ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } = &expr.kind
        else {
            return None;
        };
        let mut arms = Vec::new();
        for arm in [then_value.as_ref(), else_value.as_ref()] {
            if matches!(arm.kind, ExprKind::Conditional { .. }) {
                arms.extend(self.unresolved_conditional_place_arms(arm)?);
                continue;
            }
            if let Some(target) = self.place_target(arm) {
                arms.push((target, self.ty(arm)));
            }
        }
        Some(arms)
    }

    /// True when a move target's root is storage the move rules may empty.
    ///
    /// An owned local and a `take`/out parameter own their value; a borrow
    /// element alias, a read-only lend parameter, a global, and a temporary
    /// do not.
    pub(super) fn root_owns_moves(&self, target: &PlaceTarget) -> bool {
        let PlaceTarget::Local(id, _) = target else {
            return false;
        };
        self.binding_owns_moves(&self.bindings[*id])
    }

    /// True when a binding owns the value a move may leave.
    ///
    /// An owned local and a `take`/out parameter own their value; a borrow
    /// element alias, a read-only lend parameter, a global, a capture, and a
    /// temporary do not.
    pub(super) fn binding_owns_moves(&self, binding: &Binding) -> bool {
        match binding.kind {
            BindingKind::Local => binding.mode != Mode::Borrow,
            BindingKind::OutSlot => true,
            BindingKind::Param => binding.mode == Mode::Take,
        }
    }

    /// The owned place sources a conditional may move out of.
    ///
    /// A conditional's runtime value is its chosen arm. When an arm is a place
    /// root of a non-duplicable type, consuming the conditional moves whichever
    /// arm the condition selects, so the checker treats every such arm as
    /// maybe-moved: a later use reports the moved-value error, exactly as a
    /// conditional move does under move semantics. A nested conditional
    /// recurses; a duplicable arm is copied, and a type that still mentions a
    /// generic parameter is left to its instantiation.
    pub(super) fn conditional_move_sources(&self, expr: &Expr) -> Vec<PlaceTarget> {
        let ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } = &expr.kind
        else {
            return Vec::new();
        };
        let mut sources = Vec::new();
        for arm in [then_value.as_ref(), else_value.as_ref()] {
            if matches!(arm.kind, ExprKind::Conditional { .. }) {
                sources.extend(self.conditional_move_sources(arm));
                continue;
            }
            let ty = self.ty(arm);
            if self.sema.table.duplicable(&ty) || super::lower::mentions_type_param(&ty) {
                continue;
            }
            if let Some(target) = self.place_target(arm) {
                sources.push(target);
                continue;
            }
            // An arm that is itself a projection over a conditional names no
            // single place (`d ? (c ? b : mk()).v : z`), so the projected
            // places of that arm's own arms are collected instead; consuming
            // the outer conditional marks the moved member either way.
            sources.extend(self.conditional_projection_move_sources(arm));
        }
        sources
    }

    /// The owned places a consumed projection of a conditional may move out of.
    ///
    /// A consuming position that takes a member or element projection of a
    /// conditional (`(c ? a : b).v`) moves whichever arm the condition selects,
    /// so every arm that is itself a place contributes the projected member as
    /// a maybe-moved source. A nested conditional recurses, an arm that is a
    /// fresh value contributes nothing, the chain above the conditional is
    /// applied to each arm, and duplicable or still-generic arms are left to
    /// their own rules, mirroring the conditional move rule.
    pub(super) fn conditional_projection_move_sources(&self, expr: &Expr) -> Vec<PlaceTarget> {
        let mut steps: Vec<ir::Projection> = Vec::new();
        let mut cursor = expr;
        loop {
            match &cursor.kind {
                ExprKind::Member { base, name } => {
                    // A fixed array's `.len` is its structural extent, not a
                    // place: it names no storage and its base reads nothing.
                    if name == "len" && self.ty(base).is_fixed_array() {
                        return Vec::new();
                    }
                    steps.insert(0, ir::Projection::Field(name.clone()));
                    cursor = base;
                }
                ExprKind::Index { base, index } => {
                    let element = match &index.kind {
                        ExprKind::Int(value) => {
                            ir::Projection::Element(usize::try_from(*value).ok())
                        }
                        _ => ir::Projection::Element(None),
                    };
                    steps.insert(0, element);
                    cursor = base;
                }
                ExprKind::Conditional { .. } => break,
                _ => return Vec::new(),
            }
        }
        let mut sources = Vec::new();
        self.projection_move_arm_sources(cursor, &steps, &mut sources);
        sources
    }

    /// Collects one conditional level's maybe-moved projected places.
    ///
    /// A leaf arm contributes its projected place when the arm owns storage —
    /// non-duplicable and not still generic; a nested conditional recurses
    /// with the same chain applied to its own arms; and a projection arm under
    /// the conditional — the nested spelling `d ? (c ? b : mk()).v : z` —
    /// descends to its own conditional root with the segment appended, so
    /// every arm's projected place is recorded at every depth.
    fn projection_move_arm_sources(
        &self,
        expr: &Expr,
        steps: &[ir::Projection],
        sources: &mut Vec<PlaceTarget>,
    ) {
        match &expr.kind {
            ExprKind::Member { base, name } => {
                // A fixed array's `.len` is its structural extent, not a
                // place: it names no storage and its base reads nothing.
                if name == "len" && self.ty(base).is_fixed_array() {
                    return;
                }
                let mut steps = steps.to_vec();
                steps.insert(0, ir::Projection::Field(name.clone()));
                self.projection_move_arm_sources(base, &steps, sources);
            }
            ExprKind::Index { base, index } => {
                let element = match &index.kind {
                    ExprKind::Int(value) => {
                        ir::Projection::Element(usize::try_from(*value).ok())
                    }
                    _ => ir::Projection::Element(None),
                };
                let mut steps = steps.to_vec();
                steps.insert(0, element);
                self.projection_move_arm_sources(base, &steps, sources);
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                for arm in [then_value.as_ref(), else_value.as_ref()] {
                    self.projection_move_arm_sources(arm, steps, sources);
                }
            }
            _ => {
                let ty = self.ty(expr);
                if self.sema.table.duplicable(&ty) || super::lower::mentions_type_param(&ty) {
                    return;
                }
                let Some(target) = self.place_target(expr) else {
                    return;
                };
                let mut projections = match &target {
                    PlaceTarget::Local(_, projections)
                    | PlaceTarget::Global(_, projections) => projections.clone(),
                };
                projections.extend_from_slice(steps);
                sources.push(target.with_projections(projections));
            }
        }
    }

    /// The maybe-owning projected places a consumed conditional projection may
    /// move out of.
    ///
    /// Mirrors [`Self::conditional_projection_move_sources`] for a type that
    /// mentions a parameter: every arm whose projected root may own becomes a
    /// source, so the caller can mark it consumed without rejecting a
    /// duplicable instance. A duplicable arm contributes nothing, and a fresh
    /// arm has no place to mark. The projected source is judged at each
    /// instantiation, exactly as the concrete twin's is.
    pub(super) fn maybe_owning_projection_move_sources(&self, expr: &Expr) -> Vec<PlaceTarget> {
        let mut steps: Vec<ir::Projection> = Vec::new();
        let mut cursor = expr;
        loop {
            match &cursor.kind {
                ExprKind::Member { base, name } => {
                    // A fixed array's `.len` is its structural extent, not a
                    // place: it names no storage and its base reads nothing.
                    if name == "len" && self.ty(base).is_fixed_array() {
                        return Vec::new();
                    }
                    steps.insert(0, ir::Projection::Field(name.clone()));
                    cursor = base;
                }
                ExprKind::Index { base, index } => {
                    let element = match &index.kind {
                        ExprKind::Int(value) => {
                            ir::Projection::Element(usize::try_from(*value).ok())
                        }
                        _ => ir::Projection::Element(None),
                    };
                    steps.insert(0, element);
                    cursor = base;
                }
                ExprKind::Conditional { .. } => break,
                _ => return Vec::new(),
            }
        }
        let mut sources = Vec::new();
        self.maybe_owning_projection_arm_sources(cursor, &steps, &mut sources);
        sources
    }

    /// Collects one conditional level's maybe-owning projected places.
    ///
    /// Mirrors [`Self::projection_move_arm_sources`] with the maybe-owning
    /// predicate: a leaf arm contributes its projected place when the arm's
    /// type may own once substituted, and a nested conditional recurses with
    /// the same chain applied to its own arms.
    fn maybe_owning_projection_arm_sources(
        &self,
        expr: &Expr,
        steps: &[ir::Projection],
        sources: &mut Vec<PlaceTarget>,
    ) {
        match &expr.kind {
            ExprKind::Member { base, name } => {
                if name == "len" && self.ty(base).is_fixed_array() {
                    return;
                }
                let mut steps = steps.to_vec();
                steps.insert(0, ir::Projection::Field(name.clone()));
                self.maybe_owning_projection_arm_sources(base, &steps, sources);
            }
            ExprKind::Index { base, index } => {
                let element = match &index.kind {
                    ExprKind::Int(value) => {
                        ir::Projection::Element(usize::try_from(*value).ok())
                    }
                    _ => ir::Projection::Element(None),
                };
                let mut steps = steps.to_vec();
                steps.insert(0, element);
                self.maybe_owning_projection_arm_sources(base, &steps, sources);
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                for arm in [then_value.as_ref(), else_value.as_ref()] {
                    self.maybe_owning_projection_arm_sources(arm, steps, sources);
                }
            }
            _ => {
                let ty = self.ty(expr);
                if !self.maybe_owns(&ty) {
                    return;
                }
                let Some(target) = self.place_target(expr) else {
                    return;
                };
                let mut projections = match &target {
                    PlaceTarget::Local(_, projections)
                    | PlaceTarget::Global(_, projections) => projections.clone(),
                };
                projections.extend_from_slice(steps);
                sources.push(target.with_projections(projections));
            }
        }
    }

    /// Marks the owned arms a materialized conditional read may move.
    ///
    /// A read position that needs a place materializes a fixed-array
    /// conditional through the branch-aware store, which moves the chosen arm.
    /// The walk records the same maybe-moved effect the binding and return
    /// paths record, so a later use of either arm is the moved-value error.
    pub(super) fn mark_read_conditional_move(&mut self, expr: &Expr) {
        if !matches!(expr.kind, ExprKind::Conditional { .. }) {
            return;
        }
        let ty = self.ty(expr);
        if !ty.is_fixed_array() {
            return;
        }
        if self.maybe_owns(&ty) {
            // A read that materializes the conditional moves the chosen arm for
            // an owning instance, so a later use of either arm deposits the
            // duplicability duty.
            for source in self.maybe_owning_move_sources(expr) {
                self.mark_moved_if_owning(source, expr.span);
            }
            return;
        }
        if self.sema.table.duplicable(&ty) {
            return;
        }
        for source in self.conditional_move_sources(expr) {
            self.move_target(source, expr.span);
        }
    }

    /// True when a conditional arm is an ownerless element move.
    ///
    /// A conditional whose arms read elements out of ownerless call results
    /// can consume only one element at runtime, and no arm has a place to move
    /// out of, so the same v1 element-move limit applies as for a plain
    /// element read. A nested conditional recurses.
    pub(super) fn conditional_ownerless_element_move(&self, expr: &Expr) -> bool {
        let ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } = &expr.kind
        else {
            return false;
        };
        [then_value.as_ref(), else_value.as_ref()]
            .into_iter()
            .any(|arm| {
                if matches!(arm.kind, ExprKind::Conditional { .. }) {
                    self.conditional_ownerless_element_move(arm)
                } else {
                    self.is_ownerless_element_move(arm)
                }
            })
    }

    /// True when the expression moves an element out of an ownerless call
    /// result.
    ///
    /// The root of an index chain decides ownership: a local, parameter, or
    /// global root is a place whose move is checked at the binding sites and
    /// at projected-target assignments, while a call root owns its storage
    /// only until the statement ends. Moving a non-duplicable element out of
    /// a call result is the v1 element-move limit; a value with no element
    /// selection is left to the ordinary field rules.
    pub(super) fn is_ownerless_element_move(&self, expr: &Expr) -> bool {
        fn roots_in_call(expr: &Expr) -> bool {
            match &expr.kind {
                ExprKind::Index { base, .. } | ExprKind::Member { base, .. } => roots_in_call(base),
                ExprKind::Call { .. } => true,
                _ => false,
            }
        }
        fn selects_element(expr: &Expr) -> bool {
            match &expr.kind {
                ExprKind::Index { .. } => true,
                ExprKind::Member { base, .. } => selects_element(base),
                _ => false,
            }
        }
        selects_element(expr) && roots_in_call(expr) && self.place_target(expr).is_none()
    }

    /// Reports the v1 element-move limit, naming the same remedy as a partial
    /// move outside a top-level field.
    pub(super) fn report_element_move_limit(&mut self, span: Span) {
        self.error(
            "E10026",
            "partial moves are limited to top-level fields in v1",
            "move the whole value instead",
            span,
        );
    }

    /// True when a call's callee expression lowers to a closure value.
    ///
    /// A capturing lambda's recorded type is a closure, but a capture-less
    /// lambda is a `func`; its literal form is recognized by shape so a direct
    /// `((x: i32) => x + 1)(4)` still becomes a closure call.
    pub(super) fn is_closure_callee(&self, callee: &Expr) -> bool {
        matches!(callee.kind, ExprKind::Lambda { .. })
            || self.recorded_ty(callee).is_some_and(|ty| ty.is_closure())
    }

    /// Resolves a written type in the current generic scope, folding constant
    /// array extents through the module's constant environment.
    pub(super) fn resolve(&self, ty: &crate::ast::Type) -> Ty {
        self.resolve_with(ty, &self.type_params)
    }

    /// Resolves a written type in a given generic scope, folding constant array
    /// extents through the module's constant environment.
    ///
    /// A callee's declaration still names its own type parameters, so its
    /// written types resolve against the callee's parameter list rather than
    /// the scope of the calling body.
    pub(super) fn resolve_with(&self, ty: &crate::ast::Type, type_params: &[String]) -> Ty {
        resolve_ty_folded(ty, type_params, &self.sema.table, self.module, self.loaded)
    }

    /// Checks and walks an expression, returning its type.
    pub(super) fn eval(&mut self, expr: &'a Expr) -> Ty {
        let ty = self.ty(expr);
        match &expr.kind {
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Char(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Error
            | ExprKind::Lambda { .. } => {}
            ExprKind::Name(name) => {
                if let Some(id) = self.lookup(name) {
                    self.read_place(id, &[], expr.span);
                }
            }
            ExprKind::Qualified(name) => self.eval_qualified(name, expr.span),
            ExprKind::Member { base, .. } => {
                let base_ty = self.eval(base);
                if base_ty.nullable {
                    self.check_nonnull_expr(base, expr.span);
                }
                self.check_member_raw(&base_ty, expr.span);
            }
            ExprKind::Index { base, index } => {
                let base_ty = self.eval(base);
                if matches!(base_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
                    self.report_unsafe(UnsafeOp::PointerArithmetic, expr.span);
                } else if base_ty.nullable && self.unsafe_depth == 0 {
                    self.check_nonnull_expr(base, expr.span);
                }
                self.eval(index);
            }
            ExprKind::Unary { op, operand } => {
                let operand_ty = self.eval(operand);
                if op.is_update() {
                    self.check_update_value_write(operand);
                }
                // A fixed array's structural `.len` is a compile-time extent,
                // not a place: its address does not exist.
                if matches!(op, UnaryOp::AddressOf) && self.structural_len_target(operand) {
                    self.reject_structural_len_target(operand);
                }
                if matches!(op, UnaryOp::AddressOf) && self.unsafe_depth == 0 {
                    self.report_unsafe(UnsafeOp::AddressOf, expr.span);
                }
                if matches!(op, UnaryOp::Deref) {
                    if self.unsafe_depth == 0 {
                        self.report_unsafe(UnsafeOp::Deref, expr.span);
                    }
                    if operand_ty.nullable {
                        self.check_nonnull_expr(operand, expr.span);
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                let left_ty = self.eval(left);
                let right_ty = self.eval(right);
                if matches!(op, BinaryOp::Add | BinaryOp::Sub)
                    && (matches!(left_ty.kind, TyKind::Raw(_))
                        || matches!(right_ty.kind, TyKind::Raw(_)))
                    && self.unsafe_depth == 0
                {
                    self.report_unsafe(UnsafeOp::PointerArithmetic, expr.span);
                }
            }
            ExprKind::Assign { op, target, value } => self.check_assign(*op, target, value),
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.eval(condition);
                self.eval(then_value);
                self.eval(else_value);
            }
            ExprKind::Cast { value, .. } => {
                self.eval(value);
            }
            ExprKind::RawCast { value, .. } => {
                let value_ty = self.eval(value);
                if matches!(value_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
                    self.report_unsafe(UnsafeOp::PointerCast, expr.span);
                }
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    // A discarded arm's literal field is a read, and so is an
                    // array literal inside it: nothing stores either value,
                    // and the source bindings stay usable afterwards.
                    let saved_discarded_field = self.discarded_literal_field;
                    self.discarded_literal_field = self.discarded_conditional;
                    self.eval(&field.value);
                    self.discarded_literal_field = saved_discarded_field;
                    // A field of a discarded conditional's arm is never
                    // stored: the statement drops the whole literal, so a
                    // place-valued field is a read and its binding stays
                    // usable, exactly like the direct `ok ? t : u;` control.
                    if !self.discarded_conditional {
                        self.consume_owned_place(&field.value, field.value.span);
                    }
                    // A view-like field value stored in a struct field is
                    // storage in a field: a backing whose storage dies before
                    // the destination would leave the field dangling, so the
                    // field reports here instead. Every component is judged
                    // on its own, so a mixed array of frame-backed views is
                    // caught exactly as the field assignment's per-element
                    // rule catches it.
                    //
                    // The destination decides which backings survive: a frame
                    // destination keeps this frame's and the caller's storage,
                    // and a returned value keeps the caller's parameters,
                    // whose storage belongs to the caller's frame (EL-2); a
                    // global, a field, or an argument keeps module storage
                    // alone. A frame destination also hands the backings to
                    // the destination's binding as loans below.
                    let view_backings: Vec<usize> = if is_view_like(&self.ty(&field.value)) {
                        Self::storage_atoms(&self.view_provenance(&field.value))
                            .into_iter()
                            .filter_map(|atom| match atom {
                                Provenance::Local(id) | Provenance::Param(id) => Some(id),
                                _ => None,
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                    if !view_backings.is_empty() {
                        match self.literal_views {
                            // A frame literal's accepted view fields are the
                            // destination binding's storage positions; the
                            // store records their loans under each position's
                            // own path.
                            LiteralViews::Frame => {}
                            LiteralViews::Return => {
                                // A parameter's storage belongs to the caller
                                // and survives the return; a local or a taken
                                // parameter dies with this frame.
                                let escapes = view_backings.iter().any(|id| {
                                    self.owner_dies_with_frame(LoanOwner::Binding(*id))
                                });
                                if escapes {
                                    self.error(
                                        "E10033",
                                        format!("borrow of '{}' escapes its scope", field.name),
                                        "derive the view from a parameter or take ownership",
                                        field.value.span,
                                    );
                                }
                            }
                            LiteralViews::Escaping => {
                                self.error(
                                    "E10033",
                                    format!("borrow of '{}' escapes its scope", field.name),
                                    "derive the view from a parameter or take ownership",
                                    field.value.span,
                                );
                            }
                        }
                    }
                    // An owned conditional field is an owning destination: the
                    // chosen arm moves into the field, so every owned place arm
                    // counts as maybe-moved afterwards; a maybe-owning field
                    // records the duplicability duty instead. A discarded arm
                    // stores nothing, so its fields keep their bindings.
                    if self.discarded_conditional {
                        continue;
                    }
                    let field_ty = self.ty(&field.value);
                    if self.maybe_owns(&field_ty) {
                        for source in self.maybe_owning_move_sources(&field.value) {
                            self.mark_moved_if_owning(source, field.value.span);
                        }
                    } else {
                        for source in self.conditional_move_sources(&field.value) {
                            self.move_target(source, field.value.span);
                        }
                    }
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                // A view-element array binds its elements as views, so each
                // element is a read of the source the view names, never a
                // move out of it; the consuming marks stay for an owned
                // element list only. An element list whose own type is
                // view-like is such an array in every position — a binding,
                // an argument, a field, or a return — so the element sources
                // are never consumed, whatever the surrounding expression is.
                let view_elements = self.view_array_literal_depth > 0 || is_view_like(&ty);
                for element in elements {
                    self.eval(element);
                    if view_elements {
                        continue;
                    }
                    // An array literal inside a discarded literal's field
                    // never stores its elements either, so the sources are
                    // reads; an array literal that is the discarded arm
                    // itself keeps the ordinary element rules.
                    if self.discarded_literal_field {
                        continue;
                    }
                    self.consume_owned_place(element, element.span);
                    // An owned conditional element moves its chosen arm into
                    // the element; a maybe-owning element records the duty.
                    let element_ty = self.ty(element);
                    if self.maybe_owns(&element_ty) {
                        for source in self.maybe_owning_move_sources(element) {
                            self.mark_moved_if_owning(source, element.span);
                        }
                    } else {
                        for source in self.conditional_move_sources(element) {
                            // A projected arm (an array row) travels through
                            // the literal's row store, not the scalar
                            // partial-move rule, so only a whole-binding arm
                            // is marked here.
                            if let PlaceTarget::Local(_, projections) = &source {
                                if projections.is_empty() {
                                    self.move_target(source, element.span);
                                }
                            }
                        }
                    }
                }
                if let Some(fill) = fill {
                    self.eval(fill);
                }
            }
            ExprKind::Unsafe(inner) => {
                self.unsafe_depth += 1;
                self.eval(inner);
                self.unsafe_depth -= 1;
            }
            ExprKind::IntrinsicCall { name, args } => {
                if ffi::is_volatile_intrinsic(name) && self.unsafe_depth == 0 {
                    self.report_unsafe(UnsafeOp::Volatile, expr.span);
                }
                // An intrinsic's arguments are its operands, not discarded
                // values, so the read rule does not reach them either.
                let saved_discarded = self.discarded_conditional;
                let saved_discarded_field = self.discarded_literal_field;
                self.discarded_conditional = false;
                self.discarded_literal_field = false;
                for arg in args {
                    if let crate::ast::IntrinsicArg::Expr(inner) = arg {
                        self.eval(inner);
                        // `len` reads its operand through a view, so a
                        // possibly-null operand widens its handle before that
                        // read; the widening is a use and reports like a
                        // direct use. The other view builders are declared
                        // functions and are checked at their call site.
                        if name == "len" && self.ty(inner).nullable {
                            self.check_nonnull_expr(inner, expr.span);
                        }
                    }
                }
                self.discarded_conditional = saved_discarded;
                self.discarded_literal_field = saved_discarded_field;
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let crate::ast::TemplatePiece::Placeholder(inner) = piece {
                        self.eval(inner);
                    }
                }
            }
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                // A call consumes its callee and arguments, so the
                // discarded-conditional read rule does not reach them: a
                // literal argument is still a store destination for its
                // fields.
                let saved_discarded = self.discarded_conditional;
                let saved_discarded_field = self.discarded_literal_field;
                self.discarded_conditional = false;
                self.discarded_literal_field = false;
                self.check_call_expr(callee, type_args, args, expr.span);
                self.discarded_conditional = saved_discarded;
                self.discarded_literal_field = saved_discarded_field;
            }
        }
        ty
    }

    /// Walks a dotted name, checking each hop's read and raw dereference.
    ///
    /// Every segment is walked, not just the first, so a raw pointer reached
    /// part way through a chain (`o.inner.x` where `inner: raw Inner`) still
    /// reports the implied dereference.
    pub(super) fn eval_qualified(&mut self, name: &crate::ast::QualifiedName, span: Span) {
        let Some(first) = name.segments.first() else {
            return;
        };
        let Some(id) = self.lookup(first) else {
            return;
        };
        let projections: Vec<ir::Projection> = name.segments[1..]
            .iter()
            .map(|segment| ir::Projection::Field(segment.clone()))
            .collect();
        self.read_place(id, &projections, span);
        let mut current = self.bindings[id].ty.clone();
        for segment in &name.segments[1..] {
            self.check_member_raw(&current, span);
            current = self.member_type(&current, segment);
        }
    }

    /// The type of a field selected from `base`, following a raw pointer.
    ///
    /// The instance's type arguments are substituted into the declaration, so a
    /// field of an instantiated generic struct has its concrete type rather than
    /// the bare parameter.
    pub(super) fn member_type(&self, base: &Ty, field: &str) -> Ty {
        match &base.kind {
            TyKind::Struct(..) => self
                .sema
                .table
                .instance_field(base, field)
                .unwrap_or_else(Ty::unknown),
            TyKind::Raw(inner) => self.member_type(inner, field),
            // A view's ABI members mirror the type-stage dispatch through the
            // shared rule: `.ptr` is the pointer to the view's element and
            // `.len` its element count.
            TyKind::Str | TyKind::StaticStr | TyKind::Array { .. } => {
                crate::types::view_member_ty(base, field).unwrap_or_else(Ty::unknown)
            }
            _ => Ty::unknown(),
        }
    }

    /// Reports `E10600` when a member is reached through a raw pointer.
    pub(super) fn check_member_raw(&mut self, base_ty: &Ty, span: Span) {
        if matches!(base_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
            self.report_unsafe(UnsafeOp::Deref, span);
        }
    }

    /// Reports `E10600` when a store target dereferences a raw pointer.
    ///
    /// The bare deref target names no member, so the place walk never reaches
    /// it: the pointer's own type decides whether the store sits outside an
    /// `unsafe` block, exactly as a field write through a raw does, and a
    /// nullable pointer reports the same non-null requirement the read
    /// spelling reports.
    pub(super) fn check_deref_raw(&mut self, pointer: &Expr, span: Span) {
        let pointer_ty = self.ty(pointer);
        if matches!(pointer_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
            self.report_unsafe(UnsafeOp::Deref, span);
        }
        if pointer_ty.nullable {
            self.check_nonnull_expr(pointer, span);
        }
    }

    /// Reports a nullable place used where a non-null value is required.
    ///
    /// The check covers projected places too, so a nullable struct field is
    /// reported unless that exact field path was narrowed.
    pub(super) fn check_nonnull_expr(&mut self, expr: &Expr, span: Span) {
        if let Some((id, projections)) = self.plain_binding(expr) {
            if !self.state.place(id).is_narrowed(&projections) {
                let name = self.bindings[id].name.clone();
                self.error(
                    "E10010",
                    format!("value '{name}' may be null here"),
                    "check `!= null` before the use",
                    span,
                );
            }
        }
    }

    /// Requires a write target's element base to be non-null.
    ///
    /// A store never evaluates its target as a value, so the read path's null
    /// requirement would not otherwise reach it. Walking the target's base
    /// chain applies the same rule: an index base that is nullable and not
    /// narrowed reports `E10010` unless the store sits inside `unsafe`.
    fn check_store_index_nonnull(&mut self, target: &'a Expr, span: Span) {
        match &target.kind {
            ExprKind::Index { base, .. } => {
                let base_ty = self.ty(base);
                if !matches!(base_ty.kind, TyKind::Raw(_))
                    && base_ty.nullable
                    && self.unsafe_depth == 0
                {
                    self.check_nonnull_expr(base, span);
                }
                self.check_store_index_nonnull(base, span);
            }
            ExprKind::Member { base, .. } => self.check_store_index_nonnull(base, span),
            ExprKind::Unsafe(inner) => {
                self.unsafe_depth += 1;
                self.check_store_index_nonnull(inner, span);
                self.unsafe_depth -= 1;
            }
            _ => {}
        }
    }

    /// Checks an expression against an expected type for nullability.
    ///
    /// A value inferred from `null` has no concrete value surface at all: a
    /// narrowing silences the possible-null rule but does not give it a type,
    /// so the null rule reports where a concrete type is required. The null
    /// shape's own store accepts only the null shape (or `null`), and every
    /// other nullable value keeps the possible-null rule.
    pub(super) fn check_value_into(&mut self, expr: &Expr, expected: &Ty, span: Span) {
        if matches!(expr.kind, ExprKind::Null) {
            if !expected.nullable && !expected.is_unknown() {
                self.report_null_rule(span);
            }
            return;
        }
        let value_ty = self.ty(expr);
        // A type that still mentions a parameter is not judged here: the
        // declaration has no concrete substitution to compare against.
        let concrete = !expected.nullable
            && !expected.is_unknown()
            && !crate::types::contains_param(expected);
        if crate::types::is_null_shaped(&value_ty) {
            if concrete {
                self.report_null_rule(span);
            }
            return;
        }
        if crate::types::is_null_shaped(expected) {
            self.report_null_rule(span);
            return;
        }
        if crate::types::nullable_value_needs_narrowing(&value_ty, expected) {
            self.check_nonnull_expr(expr, span);
        }
    }

    /// Reports the null rule at a position that requires a concrete value.
    pub(super) fn report_null_rule(&mut self, span: Span) {
        self.error(
            "E10012",
            "'null' is not allowed here",
            "declare the type `T?` or avoid the empty state",
            span,
        );
    }

    /// Checks an assignment expression.
    ///
    /// A compound assignment `x op= y` is `x = x op y` (Ch 04 §4), so it emits
    /// the same read-modify-write as the equivalent plain assignment: the
    /// current value of the place is read, combined with the right-hand side,
    /// and stored back. The place is lowered once, so a side-effecting element
    /// index runs a single time.
    pub(super) fn check_assign(&mut self, op: AssignOp, target: &'a Expr, value: &'a Expr) {
        let target_ty = self.ty(target);
        let compound = op.binary_op();
        if compound.is_some() && matches!(target_ty.kind, TyKind::Raw(_)) {
            self.report_unsafe(UnsafeOp::PointerArithmetic, target.span);
        }
        self.check_place_raw(target);
        self.check_store_index_nonnull(target, target.span);
        // A dereferenced raw pointer is a store target: the pointer names the
        // storage, so the store writes through it in the same shape an indexed
        // element store uses. The pointer expression evaluates first, an
        // updating value settles at its own position before the store, and the
        // read path is unchanged.
        if let Some(pointer_expr) = raw_deref_target(target) {
            self.check_deref_raw(pointer_expr, target.span);
            // The store reads the pointer's value, so the binding that names
            // it counts as used and its own state checks run exactly once.
            self.eval(pointer_expr);
            let pin = super::lower::contains_update(value);
            let element_ty = target_ty.clone();
            let pointer = self.lower_value(pointer_expr);
            let pointer = if super::lower::contains_update(pointer_expr) {
                self.pin_update_value(pointer)
            } else {
                pointer
            };
            // The pointer is bound once whenever it is not a trivial place: the
            // target spelling, the release, and the clear all read the same
            // binding, so an impure pointer expression evaluates exactly once
            // and every use names the same address. A named place keeps its
            // spelling.
            let pointer = self.materialize_operand(pointer);
            // The store target's own type decides whether the replaced pointee
            // is released, so the pointer is spelled as the pointee's address
            // exactly as the indexed store's element address is: the address of
            // the dereferenced place rather than the raw pointer value.
            let pointee = Value::new(
                element_ty.clone(),
                ValueKind::Unary {
                    op: UnaryOp::Deref,
                    operand: Box::new(pointer),
                },
            );
            let pointer = Value::new(
                element_ty.clone(),
                ValueKind::Unary {
                    op: UnaryOp::AddressOf,
                    operand: Box::new(pointee.clone()),
                },
            );
            let stored = match compound {
                Some(binary) => {
                    // The compound store reads the pointee through that same
                    // address, before the side runs; an updating side
                    // materializes the read first.
                    let element = pointee;
                    let element = if pin {
                        self.pin_update_value(element)
                    } else {
                        element
                    };
                    let right = self.lower_value(value);
                    let right = if pin { self.pin_update_value(right) } else { right };
                    self.combine(binary, element, right, &element_ty)
                }
                None => {
                    // The store consumes an owned value exactly as a named
                    // destination does, so a later use of the source reports
                    // the moved-value error and the emission moves it in.
                    self.consume_owned_place(value, value.span);
                    let lowered = self.lower_value(value);
                    let lowered = if pin {
                        self.pin_update_value(lowered)
                    } else {
                        lowered
                    };
                    if is_view(&element_ty)
                        || (self.view_destination_adopts && is_view_like(&element_ty))
                    {
                        if self.projects_from_place(&lowered) {
                            lowered
                        } else {
                            self.bind_view_projection_root(lowered)
                        }
                    } else {
                        self.consume_place(lowered)
                    }
                }
            };
            self.emit(Statement::StorePointer {
                pointer,
                value: stored,
            });
            self.check_value_into(value, &target_ty, value.span);
            return;
        }
        match self.place_target(target) {
            Some(PlaceTarget::Local(id, projections)) => {
                // A view-element array target binds its elements as views, so
                // the evaluation must not consume an owned element source —
                // but only when the destination is a local of this frame: a
                // borrowed slot or a projected target can outlive the
                // element's backing, so it keeps the derived element in place.
                let saved_adopts = self.view_destination_adopts;
                // A `for (borrow e in xs)` element alias is a local whose
                // place override points into the iterable, which outlives
                // the iteration, so only a genuine frame local may adopt a
                // backing here. A projected destination under a frame local
                // shares the frame, so it may adopt too.
                self.view_destination_adopts = self.bindings[id].kind == BindingKind::Local
                    && self.bindings[id].base.is_none();
                // A view-typed store is judged against the frame its storage
                // lives in: a slot the caller keeps carries the escape, while
                // this frame's own container element records the backing's
                // loans below. A `for (borrow e in xs)` element alias has no
                // storage of its own: the store writes the iterable element,
                // so the judgment and the element loan both root at the
                // container the alias names.
                let alias_root = self.bindings[id].alias_root.clone();
                let caller_slot = match &alias_root {
                    Some(PlaceTarget::Local(root_id, _)) => self.is_caller_slot(*root_id),
                    Some(PlaceTarget::Global(_, _)) => false,
                    None => self.is_caller_slot(id),
                };
                // A struct literal materializes its fields in this frame, so
                // a frame destination accepts a same-frame or caller backing
                // and the loans follow the destination below. A caller slot, a
                // global container, and every non-literal value keep the
                // field-escape report.
                let saved_literal_views = self.literal_views;
                self.literal_views = if alias_root.is_none()
                    && !caller_slot
                    && Self::is_frame_literal(value)
                {
                    LiteralViews::Frame
                } else {
                    LiteralViews::Escaping
                };
                let view_elements = self.view_destination_adopts
                    && is_view_like(&target_ty)
                    && matches!(&value.kind, ExprKind::ArrayLiteral { .. });
                if view_elements {
                    self.view_array_literal_depth += 1;
                }
                self.eval(value);
                if view_elements {
                    self.view_array_literal_depth -= 1;
                }
                self.literal_views = saved_literal_views;
                self.check_lend_target_store(id, &projections, value, target.span);
                // A projected target consumes a non-duplicable element value:
                // a place-rooted element is a partial move and reports E10026
                // through the shared consume point, while an ownerless element
                // is reported by the store preparation. A conditional moves
                // whichever owned arm it selects, so every arm becomes
                // maybe-moved except an arm that names the target path itself.
                // A view-typed target stores a view, which moves nothing: the
                // backing stays where it is and the container loans it, so a
                // conditional or place source is not consumed.
                if !projections.is_empty() && !is_view_like(&target_ty) {
                    if self.place_target(value).is_some() {
                        self.consume_owned_place(value, value.span);
                    } else {
                        let value_ty = self.ty(value);
                        let maybe_owning = self.maybe_owns(&value_ty);
                        let sources = if maybe_owning {
                            let mut sources = self.maybe_owning_move_sources(value);
                            if sources.is_empty() {
                                sources = self.maybe_owning_projection_move_sources(value);
                            }
                            sources
                        } else {
                            self.conditional_move_sources(value)
                        };
                        for source in sources {
                            if let PlaceTarget::Local(source_id, source_projections) = &source {
                                if *source_id == id && source_projections == &projections {
                                    continue;
                                }
                            }
                            if maybe_owning {
                                self.mark_moved_if_owning(source, value.span);
                            } else {
                                self.move_target(source, value.span);
                            }
                        }
                    }
                }
                if compound.is_some() {
                    self.read_place(id, &projections, target.span);
                }
                if is_view_like(&target_ty) || is_view_like(&self.ty(value)) {
                    match &alias_root {
                        // A global container outlives every frame, so the
                        // stored view must derive from module-level storage.
                        Some(PlaceTarget::Global(global, _)) => {
                            let name = global.name.clone();
                            self.check_escaping_view_store(
                                value,
                                &target_ty,
                                &name,
                                true,
                                target.span,
                            );
                        }
                        Some(PlaceTarget::Local(root_id, _)) if caller_slot => {
                            let name = self.bindings[*root_id].name.clone();
                            self.check_escaping_view_store(
                                value,
                                &target_ty,
                                &name,
                                false,
                                target.span,
                            );
                        }
                        _ if caller_slot => {
                            let name = self.bindings[id].name.clone();
                            self.check_escaping_view_store(
                                value,
                                &target_ty,
                                &name,
                                false,
                                target.span,
                            );
                        }
                        _ => {}
                    }
                }
                self.write_place(id, &projections, target.span);
                let lowered = if has_runtime_element(&projections) {
                    self.emit_indexed_store(target, value, compound)
                } else {
                    if let Some(guards) = self.guard_statement_for_store_target(target) {
                        self.emit(guards);
                    }
                    let place = self.target_place(&PlaceTarget::Local(id, projections.clone()));
                    self.emit_assigned(place, &target_ty, value, compound)
                };
                if projections.is_empty() {
                    self.bind_value(id, &target_ty, value, lowered.as_ref(), target.span);
                }
                // A carrier binding keeps the backings its view-like positions
                // read. A whole store replaces the set; a projected store adds
                // to it, because the binding's other positions still hold their
                // own views. A view-like target records its owners as view
                // loans through `bind_value` instead.
                if !is_view_like(&target_ty) && self.bindings[id].kind == BindingKind::Local {
                    if let Some(lowered) = &lowered {
                        let mut owners = if projections.is_empty() {
                            BTreeSet::new()
                        } else {
                            self.binding_carried_owners(id)
                        };
                        owners.extend(self.carried_view_owners(lowered, false));
                        self.set_binding_carried_owners(id, owners);
                    }
                }
                // A view stored into this frame's own container element carries
                // the backing's loans on the container, so a later read of the
                // element reaches the liveness rule when the backing ends first.
                if !projections.is_empty() && !caller_slot && is_view_like(&target_ty) {
                    self.record_element_view_loans(
                        id,
                        &projections,
                        value,
                        lowered.as_ref(),
                        target.span,
                    );
                }
                // An alias store writes the iterable's element, so the loan is
                // recorded on the container: a later read of the iterable then
                // reaches the backing's scope. The element is selected by the
                // loop index, which the path records as a run-time index.
                if let Some(PlaceTarget::Local(root_id, root_projections)) = &alias_root {
                    if !caller_slot && (is_view_like(&target_ty) || is_view_like(&self.ty(value)))
                    {
                        let mut path = root_projections.clone();
                        path.push(ir::Projection::Element(None));
                        self.record_element_view_loans(
                            *root_id,
                            &path,
                            value,
                            lowered.as_ref(),
                            target.span,
                        );
                    }
                }
                // A literal carrier's accepted view positions follow the
                // destination binding: each position's loan records under its
                // own path, so a later overwrite replaces exactly the
                // positions it replaced. The store drops every record under
                // its target path first — a whole store therefore drops them
                // all — and the new value's positions record after.
                // A copy from a carrier place hands its positions over: the
                // destination's views read the same storage, so the source's
                // element loans follow the copy — for a whole view-like array
                // exactly as for the single view. A projected source
                // (`ys = rows[0]`) contributes the positions recorded under its
                // projection, translated to the destination's paths; a single
                // view destination keeps its own loan machinery.
                let moved = match self.place_target(value) {
                    Some(PlaceTarget::Local(source, source_projections))
                        if super::place::constant_path(&source_projections)
                            && (source_projections.is_empty() || !is_view(&target_ty))
                            && (!is_view_like(&target_ty) || projections.is_empty()) =>
                    {
                        self.move_element_loans(source, &source_projections, id, &projections);
                        true
                    }
                    _ => false,
                };
                if !moved && !is_view_like(&target_ty) {
                    self.replace_element_loans(id, &projections, BTreeSet::new());
                    let positions = self.literal_view_loan_positions(value);
                    if !positions.is_empty() {
                        self.record_literal_view_positions(
                            id,
                            &projections,
                            &positions,
                            target.span,
                        );
                    }
                }
                self.view_destination_adopts = saved_adopts;
            }
            Some(PlaceTarget::Global(id, projections)) => {
                // A global outlives this frame, so a view-element array stored
                // here never adopts a backing owned by this function: the
                // derived element stays in place instead.
                let saved_adopts = self.view_destination_adopts;
                self.view_destination_adopts = false;
                let is_const = self
                    .program
                    .globals
                    .get(&id)
                    .is_some_and(|global| global.is_const);
                // A field write through a `raw` global reaches the pointee, not
                // the constant pointer binding, so the const does not forbid it.
                let through_pointer = self.writes_through_raw_global(&id, &projections);
                if is_const && !through_pointer {
                    self.error(
                        "E10023",
                        format!("cannot assign to immutable '{}'", id.name),
                        "declare it with `let` (mutable)",
                        target.span,
                    );
                }
                // The element-write rule runs after the `const` report above,
                // so a rejected write keeps exactly one diagnostic; a reported
                // write skips the emission below.
                let immutable_view_write = (!is_const || through_pointer)
                    && self.check_global_immutable_view_write(&id, &projections, target.span);
                self.eval(value);
                if !projections.is_empty() && self.place_target(value).is_some() {
                    self.consume_owned_place(value, value.span);
                }
                // A global or a field outlives every frame, so a stored view
                // must derive from module-level storage. A write an earlier
                // rule already rejected — an immutable `const` target or an
                // immutable-view element — keeps its one diagnostic and
                // skips the judgment below.
                let rejected_write = (is_const && !through_pointer) || immutable_view_write;
                let value_ty = self.ty(value);
                if !rejected_write
                    && (is_view_like(&target_ty) || is_view_like(&value_ty))
                {
                    let name = id.name.clone();
                    self.check_escaping_view_store(value, &target_ty, &name, true, target.span);
                } else if !rejected_write
                    && !is_view_like(&target_ty)
                    && !is_view_like(&value_ty)
                    && self.may_carry_view(&target_ty)
                {
                    // A carrier value keeps the views of its view-like
                    // positions, so a carrier binding read from this frame
                    // stores them into the global exactly as a direct view
                    // store would. Its loans name the backings those
                    // positions read, and every one of them dies with a
                    // frame.
                    if let Some(PlaceTarget::Local(source_id, _)) = self.place_target(value) {
                        if !self.binding_loan_owners(source_id).is_empty() {
                            self.error(
                                "E10033",
                                format!("borrow of '{}' escapes its scope", id.name),
                                "derive the view from a parameter or take ownership",
                                target.span,
                            );
                        }
                    } else if self.literal_view_loan_positions(value).is_empty()
                        && self.names_fresh_view_storage(value, &target_ty)
                    {
                        // A value that creates its own view storage — a fresh
                        // call result, or a literal or conditional of them —
                        // dies with the statement, so the global cannot keep
                        // it. A local- or parameter-backed position already
                        // reported through its field's own judgment.
                        self.error(
                            "E10033",
                            format!("borrow of '{}' escapes its scope", id.name),
                            "derive the view from a parameter or take ownership",
                            target.span,
                        );
                    }
                }
                if (!is_const || through_pointer) && !immutable_view_write {
                    if has_runtime_element(&projections) {
                        self.emit_indexed_store(target, value, compound);
                    } else {
                        if let Some(guards) = self.guard_statement_for_store_target(target) {
                            self.emit(guards);
                        }
                        let place = self.target_place(&PlaceTarget::Global(id, projections));
                        let _ = self.emit_assigned(place, &target_ty, value, compound);
                    }
                }
                self.view_destination_adopts = saved_adopts;
            }
            None => {
                // A fixed array's structural `.len` reads no storage, so it is
                // not an assignment target: the length is a compile-time extent
                // and cannot be written. Rejecting it keeps the diagnostic
                // loud instead of emitting a store that has no target.
                if self.structural_len_target(target) {
                    self.reject_structural_len_target(target);
                }
                self.eval(target);
                self.eval(value);
            }
        }
        self.check_value_into(value, &target_ty, value.span);
    }

    /// The local binding a structural-`.len` target roots at, when it does.
    fn structural_len_base_binding(&self, target: &Expr) -> Option<usize> {
        let head = match &target.kind {
            ExprKind::Qualified(name) => name.segments.first()?,
            ExprKind::Member { base, .. } => match &base.kind {
                ExprKind::Name(name) => name,
                _ => return None,
            },
            _ => return None,
        };
        self.lookup(head)
    }

    /// Reports the diagnostic for a target that is a fixed array's structural
    /// `.len`.
    ///
    /// The length is a compile-time extent, not a place. A const or read-only
    /// parameter base keeps the mode's own report, exactly as for any other
    /// write to the binding; every other structural-`.len` target is the
    /// not-a-place error.
    pub(super) fn reject_structural_len_target(&mut self, target: &'a Expr) {
        if let Some(id) = self.structural_len_base_binding(target) {
            if self.bindings[id].is_const || self.is_readonly_param(id) {
                self.write_place(id, &[], target.span);
                return;
            }
        }
        self.error(
            "E10221",
            "left side of an assignment must be a place",
            "assign to a variable, field, or element",
            target.span,
        );
    }

    /// True when a store target is a fixed array's structural `.len`.
    ///
    /// The parser spells `value.len` as a dotted name, and a `Member` shape
    /// also reaches the store path; both name the array's compile-time extent,
    /// which has no storage to write.
    pub(super) fn structural_len_target(&self, target: &Expr) -> bool {
        match &target.kind {
            ExprKind::Qualified(name) => self.qualified_structural_len(name).is_some(),
            ExprKind::Member { base, name } => name == "len" && self.ty(base).is_fixed_array(),
            _ => false,
        }
    }

    /// True when the store's value is the target place itself.
    ///
    /// The binding and the projection list must both match: a projected store
    /// reads or writes part of the place rather than the whole binding, and a
    /// different binding is an ordinary move or copy.
    fn is_exact_self_assign(
        &self,
        id: usize,
        projections: &[ir::Projection],
        value: &Expr,
    ) -> bool {
        match self.place_target(value) {
            Some(PlaceTarget::Local(source_id, source_projections)) => {
                source_id == id && source_projections == projections
            }
            _ => false,
        }
    }

    /// Reports a view store that escapes the frame its destination keeps.
    ///
    /// A view-typed destination stores view storage, so the judgment follows
    /// the frame the destination holds. A global or a field lives for the
    /// whole program: only module-level storage survives it, so a parameter-,
    /// local-, fresh-, or `take`-derived view reports `E10033`. A writable
    /// parameter slot holds the caller's frame: module-level and the caller's
    /// parameter storage survive it, while a view of this frame's storage — or
    /// of a temporary — reports the loan-outlives `E10030`.
    ///
    /// A conditional or an array names every storage component it may store,
    /// so each component is judged on its own; the first component the
    /// destination cannot keep supplies the report, so one store keeps one
    /// diagnostic.
    fn check_escaping_view_store(
        &mut self,
        value: &Expr,
        target_ty: &Ty,
        name: &str,
        global: bool,
        span: Span,
    ) {
        if !is_view_like(target_ty) && !is_view_like(&self.ty(value)) {
            return;
        }
        // The callee's own view-return judgment already rejects the value when
        // its declaration elides no single parameter, so the destination's
        // consequence report is withheld: one site carries the rejection.
        if self.callee_elides_no_single_parameter(value) {
            return;
        }
        // The empty view names no storage, so it cannot escape.
        if matches!(value.kind, ExprKind::Null) {
            return;
        }
        let failing = Self::storage_atoms(&self.view_provenance(value))
            .into_iter()
            .find(|atom| !self.provenance_fits_destination(atom, global));
        // The storage component the destination cannot keep supplies the
        // report. A container binding's element loans name the storage its
        // view positions read, exactly as a carrier's loans do, and the
        // binding's recorded provenance describes the value that first filled
        // it, which a later element store does not update — so a loaned
        // backing the destination cannot keep reports even when every recorded
        // component fits: a frame-dying one at a caller slot, and every one at
        // a global, which outlives the caller that owns it.
        let subject = match failing {
            Some(atom) => Some(match &atom {
                Provenance::Local(owner) | Provenance::Param(owner) => {
                    super::liveness::owner_subject(
                        &self.bindings[*owner].name,
                        self.bindings[*owner].generated,
                    )
                }
                _ => "a statement temporary".to_owned(),
            }),
            None => self
                .view_loan_destination_owners(value, global)
                .into_iter()
                .next()
                .map(|owner| match owner {
                    LoanOwner::Binding(id) => super::liveness::owner_subject(
                        &self.bindings[id].name,
                        self.bindings[id].generated,
                    ),
                    LoanOwner::Temp(_) => "a statement temporary".to_owned(),
                }),
        };
        let Some(subject) = subject else { return };
        if global {
            self.error(
                "E10033",
                format!("borrow of '{name}' escapes its scope"),
                "derive the view from a parameter or take ownership",
                span,
            );
            return;
        }
        self.error(
            "E10030",
            format!("borrow of {subject} does not live long enough"),
            "finish the use before the owner's scope ends",
            span,
        );
    }

    /// True when a storage component survives the destination's frame.
    ///
    /// Module-level storage and literals survive every frame, so a global
    /// destination accepts them. A caller slot additionally keeps the caller's
    /// own storage, so a parameter fits — unless it is a `take` parameter,
    /// whose storage is this callee's frame and dies when the callee returns.
    fn provenance_fits_destination(&self, provenance: &Provenance, global: bool) -> bool {
        match provenance {
            Provenance::Static | Provenance::Global(_) => true,
            Provenance::Param(id) => !global && self.bindings[*id].mode != Mode::Take,
            _ => false,
        }
    }

    /// True when a call's callee elides no single lend parameter.
    ///
    /// A view return whose declaration names zero or several lend parameters
    /// carries the return judgment at its own postlude, so every destination
    /// of that result would otherwise repeat it as a consequence report: the
    /// destination withholds and the declaration's cause stands alone. A
    /// callable field, a call-returned func, a func-typed binding, or a
    /// module-global func value is not the named function its spelling may
    /// share, so no declaration judgment is reached through it and the
    /// destination keeps its own report. The callable spellings are resolved
    /// in the same order as the call check, so a call reached through a
    /// callable value never takes the withholding.
    fn callee_elides_no_single_parameter(&self, value: &Expr) -> bool {
        let ExprKind::Call { callee, .. } = &value.kind else {
            return false;
        };
        if let Some(field_ty) = self.callee_field_type(callee) {
            if self.callable_signature(&field_ty).is_some() {
                return false;
            }
        }
        if matches!(callee.kind, ExprKind::Call { .. })
            && self.callable_signature(&self.ty(callee)).is_some()
        {
            return false;
        }
        if let Some(id) = self.func_binding_callee(callee) {
            if self.callable_signature(&self.bindings[id].ty).is_some() {
                return false;
            }
        }
        let Some(name) = self.callee_name(callee) else {
            return false;
        };
        // A bare name can also name a module-level func-typed global, whose
        // value is the callable rather than a declaration to judge; anything
        // unresolved reaches no postlude either.
        let Some(resolved) = self.resolved_callee(&name) else {
            return false;
        };
        let Callee::User(decl) = resolved else {
            return false;
        };
        let declared: Vec<String> = decl
            .type_params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        let returns = self.resolve_with(&decl.returns, &declared);
        if !is_elided_view(&returns) {
            return false;
        }
        let candidates = decl
            .params
            .iter()
            .filter(|param| {
                param.mode != Mode::Take
                    && is_lend_candidate(&self.resolve_with(&param.ty, &declared))
            })
            .count();
        candidates != 1
    }

    /// True when a write to the binding reaches storage the caller keeps.
    ///
    /// A writable lend and an out-slot name the caller's storage, and a
    /// capture environment field is outside this frame too. A `take` parameter
    /// owns its value in this frame, and a read-only lend rejects the write on
    /// its own, so both take the frame path.
    fn is_caller_slot(&self, id: usize) -> bool {
        self.bindings[id].kind == BindingKind::OutSlot
            || (self.bindings[id].kind == BindingKind::Param
                && self.bindings[id].mode == Mode::Borrow)
            || matches!(self.bindings[id].base, Some(ir::PlaceBase::EnvField { .. }))
    }

    /// Records the backing loans a view element store gives its container.
    ///
    /// A view stored into an element reads the backing's storage, so the
    /// container carries one loan per backing owner exactly as a view binding
    /// would. A later read of the element then reaches the liveness rule when
    /// the backing's scope ends first: a backing from the same frame stays
    /// accepted, while a loop-iteration or nested-block backing reports
    /// `E10030`.
    fn record_element_view_loans(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        value: &Expr,
        lowered: Option<&Value>,
        span: Span,
    ) {
        let mut owners = match lowered {
            Some(lowered) => self.carried_view_owners(lowered, true),
            None => BTreeSet::new(),
        };
        // A store that reads a place reads the storage that place's recorded
        // positions hold, so the destination's loan names them too: a copied
        // element's loan follows the copy exactly as a carrier copy's loans do.
        if let Some(position_owners) = self.place_recorded_position_owners(value) {
            owners.extend(position_owners);
        }
        // Every storage component is judged on its own, so a component that
        // names a frame or caller backing contributes its loan even when the
        // other components of a mixed value would not. `owner_dies_with_frame`
        // keeps the caller-backed components' destinations accepted.
        for atom in Self::storage_atoms(&self.view_provenance(value)) {
            if let Provenance::Local(owner) | Provenance::Param(owner) = atom {
                owners.insert(LoanOwner::Binding(owner));
            }
        }
        self.record_literal_view_loans(id, projections, owners, span);
    }

    /// Records the shared loans a value's backings give a carrier binding.
    ///
    /// The loans replace what the carrier's element path held before, so an
    /// overwritten backing stops blocking its owner's writes while a loan
    /// another element still carries stays live.
    pub(super) fn record_literal_view_loans(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        owners: BTreeSet<LoanOwner>,
        span: Span,
    ) {
        let mut loans = BTreeSet::new();
        for owner in owners {
            if let Some(loan) = self.record_view_loan(owner, LoanKind::Shared, span) {
                loans.insert(loan);
            }
        }
        self.replace_element_loans(id, projections, loans);
    }

    /// The view positions a value carries, with the path that reaches each one.
    ///
    /// A carrier's view-like positions are its storage: each one records a loan
    /// under its own path, so a later overwrite replaces exactly the positions
    /// it replaced. An aggregate descends position by position — a struct
    /// field, an array element, and either arm of a conditional — and a leaf
    /// whose declared position is a view contributes the bindings and
    /// temporaries its storage derives from. The declared position decides, not
    /// the leaf's own type, because an owned value widens to the view the
    /// position holds. Only real owners matter here, so a literal or a module
    /// binding contributes nothing.
    pub(super) fn literal_view_loan_positions(
        &self,
        expr: &Expr,
    ) -> Vec<(Vec<ir::Projection>, LoanOwner)> {
        let mut positions = Vec::new();
        let position = self.ty(expr);
        self.collect_literal_view_positions(expr, &position, &mut Vec::new(), &mut positions);
        positions
    }

    /// Collects one value's view-position owners into `positions`.
    fn collect_literal_view_positions(
        &self,
        expr: &Expr,
        position: &Ty,
        path: &mut Vec<ir::Projection>,
        positions: &mut Vec<(Vec<ir::Projection>, LoanOwner)>,
    ) {
        match &expr.kind {
            ExprKind::StructLiteral { name, fields, .. } => {
                let struct_name = name.segments.last().cloned().unwrap_or_default();
                let mut declared = self.instance_fields(position, &struct_name);
                if declared.is_empty() {
                    declared = self.instance_fields(&self.ty(expr), &struct_name);
                }
                for field in fields {
                    let field_ty = declared
                        .iter()
                        .find(|(declared, _)| declared == &field.name)
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or_else(|| self.ty(&field.value));
                    path.push(ir::Projection::Field(field.name.clone()));
                    self.collect_literal_view_positions(&field.value, &field_ty, path, positions);
                    path.pop();
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                let element = position.iterated_element();
                for (index, element_value) in elements.iter().enumerate() {
                    path.push(ir::Projection::Element(Some(index)));
                    self.collect_literal_view_positions(element_value, &element, path, positions);
                    path.pop();
                }
                // A fill value stands for every remaining element, so it
                // covers the aggregate's own position.
                if let Some(fill) = fill {
                    self.collect_literal_view_positions(fill, &element, path, positions);
                }
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.collect_literal_view_positions(then_value, position, path, positions);
                self.collect_literal_view_positions(else_value, position, path, positions);
            }
            ExprKind::Unsafe(inner) => {
                self.collect_literal_view_positions(inner, position, path, positions);
            }
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.collect_literal_view_positions(value, position, path, positions);
            }
            _ => {
                if !is_view_like(position) && !is_view_like(&self.ty(expr)) {
                    return;
                }
                for atom in Self::storage_atoms(&self.view_provenance(expr)) {
                    if let Provenance::Local(owner) | Provenance::Param(owner) = atom {
                        positions.push((path.clone(), LoanOwner::Binding(owner)));
                    }
                }
            }
        }
    }

    /// True when a value's view-like positions read storage the walk cannot
    /// name.
    ///
    /// A fresh call result's member (`mk().v`) or an intrinsic view names no
    /// binding, so its storage belongs to this frame: a destination outside the
    /// frame cannot keep it. An aggregate contributes each view-carrying
    /// position, a conditional each arm, and a view-returning call its single
    /// lend source, so that source's storage counts too.
    fn names_fresh_view_storage(&self, expr: &Expr, position: &Ty) -> bool {
        if (is_view_like(position) || is_view_like(&self.ty(expr)))
            && Self::storage_atoms(&self.view_provenance(expr))
                .iter()
                .any(|atom| matches!(atom, Provenance::None))
        {
            return true;
        }
        match &expr.kind {
            ExprKind::StructLiteral { name, fields, .. } => {
                let struct_name = name.segments.last().cloned().unwrap_or_default();
                let mut declared = self.instance_fields(position, &struct_name);
                if declared.is_empty() {
                    declared = self.instance_fields(&self.ty(expr), &struct_name);
                }
                fields.iter().any(|field| {
                    let field_ty = declared
                        .iter()
                        .find(|(declared, _)| declared == &field.name)
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or_else(|| self.ty(&field.value));
                    self.names_fresh_view_storage(&field.value, &field_ty)
                })
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                let element = position.iterated_element();
                elements
                    .iter()
                    .any(|element_value| self.names_fresh_view_storage(element_value, &element))
                    || fill
                        .as_deref()
                        .is_some_and(|fill| self.names_fresh_view_storage(fill, &element))
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.names_fresh_view_storage(then_value, position)
                    || self.names_fresh_view_storage(else_value, position)
            }
            ExprKind::Unsafe(inner) => self.names_fresh_view_storage(inner, position),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.names_fresh_view_storage(value, position)
            }
            // A view-returning call reborrows its single lend source, so the
            // result reads that source's storage; a call the walk cannot
            // attribute names fresh storage too.
            ExprKind::Call { callee, args, .. } => {
                if !is_view_like(&self.ty(expr)) {
                    return false;
                }
                match self.call_view_source(callee, args) {
                    Some(super::place::ViewCallSource::Expr(source)) => {
                        matches!(self.view_provenance(source), Provenance::None)
                            || self.names_fresh_view_storage(source, &self.ty(source))
                    }
                    Some(super::place::ViewCallSource::Receiver(provenance)) => {
                        matches!(provenance, Provenance::None)
                    }
                    None => true,
                }
            }
            _ => false,
        }
    }

    /// Records the loans a value's view positions give a carrier binding.
    ///
    /// Each position's owners record under the path that reaches it, prefixed
    /// by the destination's own path, so a later overwrite of one position
    /// replaces that position's loan and leaves the others live. The caller
    /// drops the destination's previous records first, because the store
    /// replaced every position under its target path.
    pub(super) fn record_literal_view_positions(
        &mut self,
        id: usize,
        base: &[ir::Projection],
        positions: &[(Vec<ir::Projection>, LoanOwner)],
        span: Span,
    ) {
        let mut by_path: BTreeMap<Vec<ir::Projection>, BTreeSet<LoanOwner>> = BTreeMap::new();
        for (path, owner) in positions {
            let mut full = base.to_vec();
            full.extend(path.iter().cloned());
            by_path.entry(full).or_default().insert(*owner);
        }
        for (path, owners) in by_path {
            let mut loans = BTreeSet::new();
            for owner in owners {
                if let Some(loan) = self.record_view_loan(owner, LoanKind::Shared, span) {
                    loans.insert(loan);
                }
            }
            self.replace_element_loans(id, &path, loans);
        }
    }

    /// True when a value is a struct literal the current frame materializes.
    ///
    /// The literal's fields live in the value's own frame storage, so a frame
    /// destination accepts a same-frame field backing. A conditional or an
    /// array of literals is materialized in the frame the same way; any other
    /// value — a call, a place, or a mix — is not, and its fields keep the
    /// escape report.
    pub(super) fn is_frame_literal(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::StructLiteral { .. } => true,
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => Self::is_frame_literal(then_value) && Self::is_frame_literal(else_value),
            ExprKind::ArrayLiteral { elements, fill } => {
                elements.iter().all(Self::is_frame_literal)
                    && fill
                        .as_deref()
                        .is_none_or(Self::is_frame_literal)
            }
            ExprKind::Unsafe(inner) => Self::is_frame_literal(inner),
            _ => false,
        }
    }

    /// Judges a store whose target is a lend parameter or an out-parameter
    /// slot.
    ///
    /// A whole-value store into a lend parameter that owns storage would
    /// release the caller's value, so it is rejected — unless the right side
    /// is a lend-place move the existing lend-move path already reports at the
    /// same site, which keeps one diagnostic per assignment. A slot whose type
    /// is still a parameter defers the judgment to its instances.
    ///
    /// A nullable out-parameter slot accepts the store, because it owns its
    /// incoming value. An exact self-assignment through it is the exception:
    /// the store would release that value and write the emptied source back,
    /// so the caller would lose it. The self-assignment test compares the
    /// recorded projection lists: two run-time index projections compare equal
    /// (a run-time index records no index value), so any same-root run-time
    /// pair — `xs[i] = xs[j]` as well as `xs[i] = xs[i]` — is judged a
    /// self-assign, while literal indices distinguish (`xs[1] = xs[0]` is
    /// accepted).
    fn check_lend_target_store(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        value: &'a Expr,
        span: Span,
    ) {
        if self.bindings[id].kind == BindingKind::OutSlot {
            if self.is_exact_self_assign(id, projections, value) {
                let name = self.bindings[id].name.clone();
                self.error(
                    "E10020",
                    format!("'{name}' was moved and cannot be used"),
                    "re-initialize the binding or use a view",
                    span,
                );
            }
            return;
        }
        if !projections.is_empty() {
            return;
        }
        let binding = &self.bindings[id];
        if binding.kind != BindingKind::Param || binding.mode != Mode::Borrow {
            return;
        }
        let ty = binding.ty.clone();
        // A whole-binding self-assign through a still-parameter lend root is
        // judged after substitution: the concrete spellings reject an owning
        // or nullable instance while a duplicable or view instance stays a
        // copy. Recording the site before the deferral below is essential —
        // the self source would otherwise look like an already-reported lend
        // move and the judgment would be skipped entirely.
        if self.maybe_owns(&ty) && self.is_exact_self_assign(id, projections, value) {
            let parameter = binding.name.clone();
            let ir_name = binding.ir_name.clone();
            if self.recorded_self_assign.insert((ir_name, span.start, span.end)) {
                self.self_assign_requirements.push(ir::RequiresSelfAssign {
                    function: self.decl.name.clone(),
                    parameter,
                    ty,
                    span,
                });
            }
            return;
        }
        if self.lend_moves_judged(&PlaceTarget::Local(id, projections.to_vec()), &ty, value) {
            return;
        }
        let parameter = binding.name.clone();
        let ir_name = binding.ir_name.clone();
        if self.maybe_owns(&ty) {
            if ty.is_unknown()
                || !self
                    .recorded_storable_slot
                    .insert((ir_name, span.start, span.end))
            {
                return;
            }
            self.storable_slot_requirements.push(ir::RequiresStorableSlot {
                function: self.decl.name.clone(),
                parameter,
                ty,
                span,
            });
            return;
        }
        if self.owns_drop(&ty) {
            self.error(
                "E10034",
                format!("cannot release or store borrowed parameter '{parameter}'"),
                "declare the parameter `take` or by value; the `borrow T?` out-param slot is exempt",
                span,
            );
        }
    }

    /// True when the lend-move path already judges a source the store's right
    /// side would consume.
    ///
    /// The source set mirrors the binding effect the assignment applies: a
    /// maybe-owning target consumes a place root or the arms of a conditional
    /// as maybe-moves, while an owning target consumes them as moves. An arm
    /// that names the exact target path is exempt before the lend test, again
    /// mirroring the marking rule: storing the target into itself moves
    /// nothing out of it, so the target-side store rule must judge the site.
    /// Every other source keeps the existing suppression — a direct place
    /// reports through the place-move path, and a read-only lend arm reports
    /// through the source side, directly for a concrete type and through the
    /// per-instance duty while the type is still a parameter — so the site
    /// keeps exactly one diagnostic.
    fn lend_moves_judged(&self, target: &PlaceTarget, target_ty: &Ty, value: &Expr) -> bool {
        let direct_place = self.place_target(value).is_some();
        let mut sources = if self.maybe_owns(target_ty) {
            match self.place_target(value) {
                Some(source) => vec![source],
                None => {
                    let mut sources = self.maybe_owning_move_sources(value);
                    if sources.is_empty() {
                        sources = self.maybe_owning_projection_move_sources(value);
                    }
                    sources
                }
            }
        } else if is_view(target_ty) || self.sema.table.duplicable(target_ty) {
            Vec::new()
        } else {
            match self.place_target(value) {
                Some(source) => vec![source],
                None => self.conditional_move_sources(value),
            }
        };
        if !direct_place {
            if let PlaceTarget::Local(target_id, target_projections) = target {
                sources.retain(|source| match source {
                    PlaceTarget::Local(source_id, source_projections) => {
                        source_id != target_id || source_projections != target_projections
                    }
                    PlaceTarget::Global(_, _) => true,
                });
            }
        }
        !sources.is_empty()
            && sources.iter().any(|source| match source {
                PlaceTarget::Local(source_id, _) => {
                    let binding = &self.bindings[*source_id];
                    binding.kind == BindingKind::Param
                        && matches!(binding.mode, Mode::None | Mode::Const | Mode::Borrow)
                }
                PlaceTarget::Global(_, _) => false,
            })
    }

    /// Checks an increment or decrement used as a statement.
    ///
    /// `x++`, `++x`, `x--`, and `--x` all reduce to `x = x op 1` (Ch 04 §4), so
    /// they emit the same read-modify-write as a compound assignment. The result
    /// value is discarded, so a postfix and a prefix form lower identically
    /// here; the value forms go through the unary value path instead.
    pub(super) fn check_update(&mut self, op: AssignOp, operand: &'a Expr) {
        let ty = self.ty(operand);
        self.check_place_raw(operand);
        let binary = op.binary_op().expect("the update operators are compound");
        let one = Value::new(ty.clone(), ValueKind::Int(1));
        match self.place_target(operand) {
            Some(PlaceTarget::Local(id, projections)) => {
                self.read_place(id, &projections, operand.span);
                self.write_place(id, &projections, operand.span);
                if has_runtime_element(&projections) {
                    self.emit_indexed_update(operand, binary, one);
                } else {
                    if let Some(guards) = self.guard_statement_for_store_target(operand) {
                        self.emit(guards);
                    }
                    let place = self.target_place(&PlaceTarget::Local(id, projections));
                    let left = Value::place(place.clone());
                    self.emit_compound_store(binary, place, &ty, left, one);
                }
            }
            Some(PlaceTarget::Global(id, projections)) => {
                let is_const = self
                    .program
                    .globals
                    .get(&id)
                    .is_some_and(|global| global.is_const);
                // A field update through a `raw` global reaches the pointee,
                // not the constant pointer binding.
                let through_pointer = self.writes_through_raw_global(&id, &projections);
                if is_const && !through_pointer {
                    self.error(
                        "E10023",
                        format!("cannot assign to immutable '{}'", id.name),
                        "declare it with `let` (mutable)",
                        operand.span,
                    );
                    return;
                }
                // The element-write rule runs after the `const` report above,
                // so a rejected write keeps exactly one diagnostic; a reported
                // write skips the emission below.
                if self.check_global_immutable_view_write(&id, &projections, operand.span) {
                    return;
                }
                if has_runtime_element(&projections) {
                    self.emit_indexed_update(operand, binary, one);
                } else {
                    if let Some(guards) = self.guard_statement_for_store_target(operand) {
                        self.emit(guards);
                    }
                    let place = self.target_place(&PlaceTarget::Global(id, projections));
                    let left = Value::place(place.clone());
                    self.emit_compound_store(binary, place, &ty, left, one);
                }
            }
            None => {
                // A fixed array's structural `.len` is not an update target:
                // the length is a compile-time extent and cannot be written.
                if self.structural_len_target(operand) {
                    self.reject_structural_len_target(operand);
                    return;
                }
                self.eval(operand);
            }
        }
    }

    /// Routes an update used as a value through the statement-form write
    /// checks.
    ///
    /// A value-position `x++` reads the place and stores the result back, so
    /// it contains the same element write the statement form judges through
    /// the write path. The value runs through the same taxonomy — locals
    /// through the read and write place checks (immutable `const`, read-only
    /// parameter, immutable view, place state), globals through the `const`
    /// check with its through-`raw` carve-out and the immutable-view check —
    /// so an update reports exactly what the statement form reports at the
    /// site: one write-mutability report per update, and the same place-state
    /// reports the statement form produces. The statement path evaluates its
    /// operand directly and judges the write in `check_update`, so the two
    /// forms stay aligned by construction.
    fn check_update_value_write(&mut self, operand: &'a Expr) {
        match self.place_target(operand) {
            Some(PlaceTarget::Local(id, projections)) => {
                self.read_place(id, &projections, operand.span);
                self.write_place(id, &projections, operand.span);
            }
            Some(PlaceTarget::Global(id, projections)) => {
                let is_const = self
                    .program
                    .globals
                    .get(&id)
                    .is_some_and(|global| global.is_const);
                // A field update through a `raw` global reaches the pointee,
                // not the constant pointer binding.
                let through_pointer = self.writes_through_raw_global(&id, &projections);
                if is_const && !through_pointer {
                    self.error(
                        "E10023",
                        format!("cannot assign to immutable '{}'", id.name),
                        "declare it with `let` (mutable)",
                        operand.span,
                    );
                    return;
                }
                self.check_global_immutable_view_write(&id, &projections, operand.span);
            }
            None => {
                // A fixed array's structural `.len` is not an update target in
                // a value position either.
                if self.structural_len_target(operand) {
                    self.reject_structural_len_target(operand);
                }
            }
        }
    }

    /// True when an assignment to a global place goes through a `raw` pointer.
    ///
    /// A module-level `const` of `raw T` is an immutable pointer binding, but
    /// the memory it names is not part of the binding: `BASE.field = ...`
    /// writes the pointee, which the const does not forbid. Only a write with
    /// no projection reaches the binding itself.
    pub(super) fn writes_through_raw_global(&self, id: &ir::GlobalId, projections: &[ir::Projection]) -> bool {
        !projections.is_empty()
            && self
                .program
                .globals
                .get(id)
                .is_some_and(|global| matches!(global.ty.kind, TyKind::Raw(_)))
    }

    /// The guard statement a constant-index element store target needs.
    ///
    /// A store target never evaluates as a value, so its element bounds checks
    /// cannot ride on the stored value; the statement carries the target's
    /// index chain and the ordinary guard pass visits every level in
    /// evaluation order. A target whose element selections all sit on fixed
    /// arrays needs no runtime check — the semantic layer validated the
    /// constant indices — so it builds no statement and its emission stays
    /// unchanged.
    fn guard_statement_for_store_target(&mut self, target: &'a Expr) -> Option<Statement> {
        if !self.store_target_has_dynamic_element(target) {
            return None;
        }
        let check = self.lower_value(target);
        Some(Statement::Guards {
            checks: vec![check],
        })
    }

    /// True when any element selection of a store target sits on a
    /// dynamically bounded view.
    ///
    /// A fixed-array selection is range-checked when the program is validated,
    /// so only a slice, growable array, or string view needs a runtime guard.
    fn store_target_has_dynamic_element(&self, target: &Expr) -> bool {
        match &target.kind {
            ExprKind::Index { base, .. } => {
                let base_ty = self.ty(base);
                let dynamic = match &base_ty.kind {
                    TyKind::Array { extents, .. } => extents.iter().any(Option::is_none),
                    TyKind::Str | TyKind::StaticStr => true,
                    _ => false,
                };
                dynamic || self.store_target_has_dynamic_element(base)
            }
            ExprKind::Member { base, .. } => self.store_target_has_dynamic_element(base),
            ExprKind::Unsafe(inner) => self.store_target_has_dynamic_element(inner),
            _ => false,
        }
    }

    /// Emits the store for an assignment, plain or compound.
    fn emit_assigned(
        &mut self,
        place: Place,
        ty: &Ty,
        value: &'a Expr,
        compound: Option<BinaryOp>,
    ) -> Option<Value> {
        match compound {
            Some(binary) => {
                // The current-value read runs before the right-hand side, and
                // the side's update completes before the store. When the side
                // contains an update, the read and the whole side travel
                // through statement temporaries in that order, so the emitted
                // C has one defined sequence instead of one unsequenced store
                // expression.
                let pin = super::lower::contains_update(value);
                let left = Value::place(place.clone());
                let left = if pin { self.pin_update_value(left) } else { left };
                let right = self.lower_value(value);
                let right = if pin { self.pin_update_value(right) } else { right };
                self.emit_compound_store(binary, place, ty, left, right.clone());
                Some(right)
            }
            None => self.emit_store(place, ty, value),
        }
    }

    /// Emits a read-modify-write `place = left op right`.
    fn emit_compound_store(
        &mut self,
        op: BinaryOp,
        place: Place,
        ty: &Ty,
        left: Value,
        right: Value,
    ) {
        let value = self.combine(op, left, right, ty);
        self.emit(Statement::Assign { target: place, value });
    }

    /// Builds a binary value, materializing a guarded operand once.
    ///
    /// The emitted guard for `/` and `%` re-reads both operands, so a
    /// side-effecting operand is stored in a temporary the guard and the
    /// operation share.
    fn combine(&mut self, op: BinaryOp, left: Value, right: Value, ty: &Ty) -> Value {
        let (left, right) = if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
            (
                self.materialize_guard_operand(left),
                self.materialize_guard_operand(right),
            )
        } else {
            (left, right)
        };
        Value::new(
            ty.clone(),
            ValueKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
        )
    }

    /// Lowers a member store target to its current value, its place, and its
    /// type.
    ///
    /// The target is a member chain over a run-time element selection
    /// (`a[i].b.y`, `w.grid[i][j].name`). Every base and index is materialized
    /// once, here, before the right-hand side is lowered, so the target's
    /// location evaluates first and a later side effect cannot change the slot
    /// the store reaches. The element read, the place, and the guard share
    /// those temporaries, so a compound store or an update cannot run a
    /// side-effecting operand twice.
    ///
    /// The place keeps each run-time index on an `ElementTemp` projection, so
    /// the store renders the element's own lvalue (`ps[i].y`) exactly as the
    /// constant-index twin does, while the emitter still emits the element's
    /// bounds check. The recorded type is the target's own type; a generic
    /// declaration records it opaquely and the emitter resolves it through the
    /// place's projections at the instance.
    fn indexed_member_target(&mut self, target: &'a Expr) -> (Value, Place, Ty) {
        let element = self.lower_value(target);
        let element = self.pin_store_target(element);
        debug_assert!(
            matches!(element.kind, ValueKind::Member { .. }) && has_element_selection(&element),
            "a member store target must lower to a projected element selection"
        );
        let element_ty = self.ty(target);
        let place = self.store_target_place(&element, element_ty.clone());
        (element, place, element_ty)
    }

    /// True when a store target ends in a member selection.
    ///
    /// A member target stores through the element's own lvalue, so its
    /// run-time index travels on a place projection. A whole-element target
    /// keeps the element-address form its existing emission pins.
    fn is_member_store_target(&self, target: &Expr) -> bool {
        let mut target = target;
        while let ExprKind::Unsafe(inner) = &target.kind {
            target = inner;
        }
        matches!(target.kind, ExprKind::Member { .. })
    }

    /// Materializes the base and index of every element selection in a store
    /// target.
    ///
    /// The target's location is evaluated before the right-hand side, so each
    /// index is read into a temporary where the target stands and an impure
    /// base is computed once. A member chain only recurses: copying a member's
    /// base would point the store at the copy instead of the storage.
    pub(super) fn pin_store_target(&mut self, value: Value) -> Value {
        let ty = value.ty.clone();
        match value.kind {
            ValueKind::Member { base, name } => {
                let base = self.pin_store_target(*base);
                Value::new(
                    ty,
                    ValueKind::Member {
                        base: Box::new(base),
                        name,
                    },
                )
            }
            ValueKind::Index { base, index } => {
                let base = self.pin_store_target(*base);
                let base = self.materialize_operand(base);
                let index = self.settle_projection_base(*index);
                let index = self.materialize_store_index(index);
                Value::new(
                    ty,
                    ValueKind::Index {
                        base: Box::new(base),
                        index: Box::new(index),
                    },
                )
            }
            other => Value::new(ty, other),
        }
    }

    /// Binds one store-target index to an eager temporary.
    ///
    /// A place index is pinned even though reading it has no side effect: the
    /// right-hand side may update the place, and the store must reach the
    /// element the index named before that update. A constant or an already
    /// materialized index keeps its spelling.
    fn materialize_store_index(&mut self, index: Value) -> Value {
        if matches!(index.kind, ValueKind::Int(_) | ValueKind::Temp(_)) {
            return index;
        }
        self.materialize_value(index)
    }

    /// Builds the projected place a pinned store target names.
    ///
    /// The pinned value keeps each run-time index in a temporary, so the place
    /// records it as an `ElementTemp` and the emitter renders the same index it
    /// guarded. The root is the place the target started from.
    pub(super) fn store_target_place(&self, value: &Value, ty: Ty) -> Place {
        let mut place = store_target_place_parts(value)
            .expect("an indexed store target must root in a place");
        place.ty = ty;
        place
    }

    /// Lowers a projected place with a run-time element into a pinned place.
    ///
    /// A move whose source is an element read must keep the index the read
    /// evaluates: an element projection without an index would move element
    /// zero. The index is materialized here, and its bounds guard is emitted
    /// before the move exactly where the read's own guard would run.
    pub(super) fn pinned_element_place(&mut self, expr: &'a Expr) -> Option<(Place, Ty)> {
        let target = self.place_target(expr)?;
        let projections = match &target {
            PlaceTarget::Local(_, projections) | PlaceTarget::Global(_, projections) => {
                projections
            }
        };
        if !has_runtime_element(projections) {
            return None;
        }
        let lowered = self.lower_value(expr);
        let lowered = self.pin_store_target(lowered);
        self.emit(Statement::Guards {
            checks: vec![lowered.clone()],
        });
        let ty = self.ty(expr);
        let place = self.store_target_place(&lowered, ty.clone());
        Some((place, ty))
    }

    /// Lowers an indexed element target to its element read and its address.
    ///
    /// The base and the run-time index are materialized once so the element
    /// read and the store address share one evaluation: a side-effecting index
    /// such as `a[next()]` cannot pass a bounds guard for one value and store
    /// through another. The address keeps the index, so the emitter still
    /// emits the element's bounds check; a plain element projection would lose
    /// the index and hardcode element zero.
    ///
    /// When `shared` is set the element value is used twice (read and address),
    /// so an impure operand is computed into a real temporary to keep its side
    /// effect to one run; a plain store passes `false` and leaves the operand
    /// in its single guarded position. When `pin_location` is set the target's
    /// element locations are materialized before the right-hand side is
    /// lowered, because the target's location evaluates first and a later
    /// update must not change the slot the store reaches.
    fn indexed_target(
        &mut self,
        target: &'a Expr,
        shared: bool,
        pin_location: bool,
    ) -> Option<(Value, Value, Ty)> {
        let target = match &target.kind {
            ExprKind::Unsafe(inner) => inner,
            _ => target,
        };
        let ExprKind::Index { base, index } = &target.kind else {
            debug_assert!(
                false,
                "a whole-element store target must be an element selection"
            );
            return None;
        };
        let element_ty = self.ty(target);
        let base_value = self.lower_value(base);
        // An assignment through a conditional base materializes it, which
        // moves the chosen owned arm.
        self.mark_read_conditional_move(base);
        let index_value = self.lower_value(index);
        // A compound store reads the element and takes its address, so the
        // index and base appear in two guarded positions. An impure operand is
        // computed into a real temporary first so it runs once; the plain store
        // leaves the operand inline, where its single guarded position suffices.
        let (base_value, index_value) = if shared {
            (
                self.materialize_operand(base_value),
                self.materialize_operand(index_value),
            )
        } else {
            (
                self.materialize_guard_operand(base_value),
                self.materialize_guard_operand(index_value),
            )
        };
        let base_value = self.materialize_read_place(base_value);
        let element = Value::new(
            element_ty.clone(),
            ValueKind::Index {
                base: Box::new(base_value),
                index: Box::new(index_value),
            },
        );
        let element = if pin_location {
            self.pin_operand_location(element)
        } else {
            element
        };
        let pointer = Value::new(
            element_ty.clone(),
            ValueKind::Unary {
                op: UnaryOp::AddressOf,
                operand: Box::new(element.clone()),
            },
        );
        Some((element, pointer, element_ty))
    }

    /// Lowers an indexed assignment, plain or compound.
    ///
    /// The returned value is the prepared right-hand side when the store
    /// copies a view: the caller records the element loans from it, exactly as
    /// the constant-index store does. Every other store returns none — an
    /// owned value is moved into the element and nothing remains to record.
    fn emit_indexed_store(
        &mut self,
        target: &'a Expr,
        value: &'a Expr,
        compound: Option<BinaryOp>,
    ) -> Option<Value> {
        // The target's location evaluates before the right-hand side. A member
        // target pins every base and index at this position; a whole-element
        // target keeps its existing guarded position.
        let pin = super::lower::contains_update(value);
        if self.is_member_store_target(target) {
            let (element, place, element_ty) = self.indexed_member_target(target);
            self.emit(Statement::Guards {
                checks: vec![element],
            });
            return self.emit_assigned(place, &element_ty, value, compound);
        }
        let Some((element, pointer, element_ty)) =
            self.indexed_target(target, compound.is_some(), pin)
        else {
            return None;
        };
        let mut prepared_view = None;
        let stored = match compound {
            Some(binary) => {
                // The element's current value is read before the side runs, so
                // an updating side materializes it first.
                let element = if pin {
                    self.pin_update_value(element)
                } else {
                    element
                };
                let right = self.lower_value(value);
                let right = if pin { self.pin_update_value(right) } else { right };
                self.combine(binary, element, right, &element_ty)
            }
            // A view element store copies the view and leaves the backing where
            // it is, exactly as the constant-index spelling does: the element
            // reads the backing's storage while the backing's owner releases it
            // at its own scope. A view derived from fresh ownerless storage
            // binds its root first, so the root releases in the view's scope
            // and an escape reports instead of dangling. An owned value stored
            // into an element is a move, exactly as it is for a named
            // destination: the checker consumed the source, so the lowering
            // empties it and the element keeps the value. Without this both the
            // source and the element release the same storage.
            None => {
                let lowered = self.lower_value(value);
                let lowered = if pin {
                    self.pin_update_value(lowered)
                } else {
                    lowered
                };
                if is_view(&element_ty)
                    || (self.view_destination_adopts && is_view_like(&element_ty))
                {
                    let lowered = if self.projects_from_place(&lowered) {
                        lowered
                    } else {
                        self.bind_view_projection_root(lowered)
                    };
                    prepared_view = Some(lowered.clone());
                    lowered
                } else {
                    self.consume_place(lowered)
                }
            }
        };
        self.emit(Statement::StorePointer {
            pointer,
            value: stored,
        });
        prepared_view
    }

    /// Lowers an indexed increment or decrement as a read-modify-write.
    ///
    /// The target builder materializes the element's base and index, so an
    /// updating index (`arr[i++]`) runs before the read and the store, and a
    /// side effect on a later operand cannot redirect the update. A member
    /// target reduces to the element's own lvalue, exactly as the
    /// constant-index twin does.
    fn emit_indexed_update(&mut self, target: &'a Expr, binary: BinaryOp, one: Value) {
        if self.is_member_store_target(target) {
            let (element, place, element_ty) = self.indexed_member_target(target);
            self.emit(Statement::Guards {
                checks: vec![element],
            });
            let left = Value::place(place.clone());
            self.emit_compound_store(binary, place, &element_ty, left, one);
            return;
        }
        let Some((element, pointer, element_ty)) = self.indexed_target(target, true, false) else {
            return;
        };
        let stored = self.combine(binary, element, one, &element_ty);
        self.emit(Statement::StorePointer {
            pointer,
            value: stored,
        });
    }

    /// Reports raw operations implied by an assignment target's projections.
    ///
    /// Walking the projections rather than the whole expression keeps the
    /// target from being read as a value, which would be wrong for a field of
    /// a partially initialized or partially moved struct.
    pub(super) fn check_place_raw(&mut self, target: &'a Expr) {
        let Some(base) = self.place_target(target) else {
            return;
        };
        // A field write through a raw pointer is a dereference whether the
        // pointer is a local or a module-level binding, so both roots are
        // walked for the unsafe gate.
        let (mut current, projections) = match base {
            PlaceTarget::Local(id, projections) => (self.bindings[id].ty.clone(), projections),
            PlaceTarget::Global(id, projections) => {
                let ty = self
                    .program
                    .globals
                    .get(&id)
                    .map(|global| global.ty.clone())
                    .unwrap_or_else(Ty::unknown);
                (ty, projections)
            }
        };
        for projection in &projections {
            match projection {
                ir::Projection::Field(field) => {
                    self.check_member_raw(&current, target.span);
                    current = self.member_type(&current, field);
                }
                ir::Projection::Element(_) | ir::Projection::ElementTemp(_) => {
                    if matches!(current.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
                        self.report_unsafe(UnsafeOp::PointerArithmetic, target.span);
                    }
                    current = Ty::unknown();
                }
            }
        }
    }


}

/// True when a projection chain selects an element with a run-time index.
pub(super) fn has_runtime_element(projections: &[ir::Projection]) -> bool {
    projections.iter().any(|projection| {
        matches!(
            projection,
            ir::Projection::Element(None) | ir::Projection::ElementTemp(_)
        )
    })
}

/// True when a lowered value selects an array or view element.
///
/// An indexed store target reaches this module only when its place carries a
/// run-time element, so the lowered chain must contain an element selection;
/// the debug assertion in the target builder pins that agreement.
fn has_element_selection(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Index { .. } => true,
        ValueKind::Member { base, .. } => has_element_selection(base),
        _ => false,
    }
}

/// The place a pinned store-target value names, when it roots in storage.
///
/// The value walk appends the same field and element selections in the same
/// order, so the rendered place reaches the slot the value's guards checked. A
/// run-time index keeps its temporary on an `ElementTemp` projection; the
/// constant form stays on `Element`.
fn store_target_place_parts(value: &Value) -> Option<Place> {
    match &value.kind {
        ValueKind::Place(place) => Some(place.clone()),
        ValueKind::Member { base, name } => {
            let mut place = store_target_place_parts(base)?;
            place.projections.push(ir::Projection::Field(name.clone()));
            Some(place)
        }
        ValueKind::Index { base, index } => {
            let mut place = store_target_place_parts(base)?;
            let projection = match &index.kind {
                ValueKind::Int(value) => ir::Projection::Element(usize::try_from(*value).ok()),
                ValueKind::Temp(temp) => ir::Projection::ElementTemp(*temp),
                ValueKind::Materialize { temp, .. } => ir::Projection::ElementTemp(*temp),
                _ => return None,
            };
            place.projections.push(projection);
            Some(place)
        }
        _ => None,
    }
}

/// The pointer expression when an assignment target dereferences a raw
/// pointer.
///
/// A bare `*p` target names no place of its own: the pointer names the storage
/// the store writes through. The walk unwraps the `unsafe` expression the
/// parser keeps around the dereference and returns the pointer operand; every
/// other target shape returns `None`.
fn raw_deref_target(target: &Expr) -> Option<&Expr> {
    let mut expr = target;
    while let ExprKind::Unsafe(inner) = &expr.kind {
        expr = inner;
    }
    match &expr.kind {
        ExprKind::Unary {
            op: UnaryOp::Deref,
            operand,
        } => Some(operand),
        _ => None,
    }
}

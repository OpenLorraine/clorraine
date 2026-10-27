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
            self.note_take_element_move(*id, projections, &ty, span);
        }
        // A member of an opaque type parameter has no concrete type yet. The
        // per-instance audit decides whether the selected member owns: record
        // the lend move now, exactly as the maybe-owning path does, so an
        // owning substitution reports E10034 and a duplicable one stays a copy.
        if ty.is_unknown() {
            if let Some(target) = target {
                self.mark_moved_if_owning(target, span);
            }
            return;
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
        self.maybe_owning_source_arms(expr)
            .into_iter()
            .map(|(_, target)| target)
            .collect()
    }

    /// The maybe-owning conditional arms paired with their expressions.
    ///
    /// A maybe-owning source is judged per instance, and a self-named arm is
    /// skipped only when its expression proves the same place (a literal
    /// index). The expression is therefore kept beside the place target.
    pub(super) fn maybe_owning_source_arms<'e>(
        &self,
        expr: &'e Expr,
    ) -> Vec<(&'e Expr, PlaceTarget)> {
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
                sources.extend(self.maybe_owning_source_arms(arm));
                continue;
            }
            let ty = self.ty(arm);
            let parameter_root = self.place_target(arm).is_some_and(|target| match target {
                PlaceTarget::Local(id, _) => {
                    super::lower::mentions_type_param(&self.bindings[id].ty)
                }
                PlaceTarget::Global(_, _) => false,
            });
            if !self.maybe_owns(&ty)
                && !super::lower::mentions_type_param(&ty)
                && !parameter_root
            {
                continue;
            }
            if let Some(target) = self.place_target(arm) {
                sources.push((arm, target));
            }
        }
        sources
    }

    /// The conditional arms whose type or root declaration mentions a type
    /// parameter.
    ///
    /// A concrete owned arm is judged at the definition; these arms are the
    /// ones an instance must resolve. The walk mirrors
    /// [`Self::maybe_owning_move_sources`] and keeps a projected member of a
    /// type-parameter place.
    pub(super) fn type_param_conditional_sources(&self, expr: &Expr) -> Vec<PlaceTarget> {
        let expr = structural_wrapper_inner(expr);
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
            let arm = structural_wrapper_inner(arm);
            if matches!(arm.kind, ExprKind::Conditional { .. }) {
                sources.extend(self.type_param_conditional_sources(arm));
                continue;
            }
            let Some(target) = self.place_target(arm) else {
                continue;
            };
            let root_param = match &target {
                PlaceTarget::Local(id, _) => {
                    super::lower::mentions_type_param(&self.bindings[*id].ty)
                }
                PlaceTarget::Global(_, _) => false,
            };
            if root_param || super::lower::mentions_type_param(&self.ty(arm)) {
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
        let expr = structural_wrapper_inner(expr);
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
            let arm = structural_wrapper_inner(arm);
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

    /// The owned place arms a conditional may move, paired with their expressions.
    ///
    /// A conditional store may move a place arm. The arm's expression is
    /// needed to decide whether it names the store's own destination: the same
    /// projection path can be spelled with a different index expression, and
    /// only a literal or an unchanged local index proves a self-copy.
    pub(super) fn conditional_move_source_arms<'e>(
        &self,
        expr: &'e Expr,
    ) -> Vec<(&'e Expr, PlaceTarget)> {
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
                sources.extend(self.conditional_move_source_arms(arm));
                continue;
            }
            let ty = self.ty(arm);
            if self.sema.table.duplicable(&ty) || super::lower::mentions_type_param(&ty) {
                continue;
            }
            if let Some(target) = self.place_target(arm) {
                sources.push((arm, target));
                continue;
            }
            for target in self.conditional_projection_move_sources(arm) {
                sources.push((arm, target));
            }
        }
        sources
    }

    /// True when a store's element index may name a different element after
    /// the condition.
    ///
    /// A literal or a plain local/parameter index can be named twice without
    /// an effect; a module global, a call, or a computed index can change, so
    /// a self-named conditional arm is an element move rather than a self-copy.
    fn destination_index_may_change(&self, target: &Expr) -> bool {
        let target = unwrap_unsafe_expr(target);
        match &target.kind {
            // A multi-index target can have a stable outer index over an
            // unstable inner one, so every index in the chain is checked.
            ExprKind::Index { base, index } => {
                self.index_expression_may_change(index)
                    || self.destination_index_may_change(base)
            }
            ExprKind::Member { base, .. } => self.destination_index_may_change(base),
            _ => false,
        }
    }

    /// True when an index expression is not a literal.
    ///
    /// Only a literal index proves the same element at both the store and the
    /// arm: a local or parameter can be changed by the condition (an `i++`, a
    /// raw write through an escaped address, or a call), a global can be
    /// changed anywhere, and a computed index is not stable. The conservative
    /// rule follows the frozen element-move rule and reports an element move
    /// for every non-literal self-named arm.
    fn index_expression_may_change(&self, index: &Expr) -> bool {
        !matches!(index.kind, ExprKind::Int(_))
    }

    /// True when a conditional has an arm with no place.
    ///
    /// The owned-source frame walk names each place arm separately; a fresh
    /// arm (a call result or a literal) still contributes a statement
    /// temporary backing, so the view escape must see it. A null arm carries
    /// no storage and contributes nothing.
    fn conditional_has_fresh_arm(&self, expr: &Expr) -> bool {
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
                    self.conditional_has_fresh_arm(arm)
                } else if matches!(arm.kind, ExprKind::Null) {
                    false
                } else {
                    self.place_target(arm).is_none()
                }
            })
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
        self.note_opaque_use(expr, expr.span);
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
                    // A conditional field whose arm is rooted at a type
                    // parameter owns only at some instances, so the field
                    // records the per-instance duty the audit resolves.
                    for source in self.type_param_conditional_sources(&field.value) {
                        self.mark_moved_if_owning(source, field.value.span);
                    }
                    for source in self.maybe_owning_projection_move_sources(&field.value) {
                        self.mark_moved_if_owning(source, field.value.span);
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
                    // An element whose arm is rooted at a type parameter owns
                    // only at some instances, so the store records the
                    // per-instance duty exactly as a struct-literal field does.
                    for source in self.type_param_conditional_sources(element) {
                        self.mark_moved_if_owning(source, element.span);
                    }
                    for source in self.maybe_owning_projection_move_sources(element) {
                        self.mark_moved_if_owning(source, element.span);
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
        // A conditional whose destination still mentions a parameter is judged
        // per instance. Flow narrowing is substitution-independent, so record
        // which arms the declaration's flow already proved non-null.
        if crate::types::contains_param(expected) {
            if let ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } = &expr.kind
            {
                self.conditional_narrowing_requirements.push(
                    ir::RequiresConditionalArmNarrowing {
                        function: self.decl.name.clone(),
                        then_span: then_value.span,
                        else_span: else_value.span,
                        then_narrowed: self.expr_is_narrowed(then_value),
                        else_narrowed: self.expr_is_narrowed(else_value),
                    },
                );
            }
        }
        if crate::types::nullable_value_needs_narrowing(&value_ty, expected) {
            // A conditional's nullable result comes from its arms: each
            // nullable arm is a separate possible-null use and reports once,
            // exactly as the concrete twin does. A non-conditional value
            // keeps its own site.
            if let ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } = &expr.kind
            {
                // A declared non-null arm is not a possible-null use; the
                // whole-value judgment above already saw it. Only the nullable
                // arm can supply the conditional's null value.
                if self.ty(then_value).nullable {
                    self.check_nonnull_expr(then_value, then_value.span);
                }
                if self.ty(else_value).nullable {
                    self.check_nonnull_expr(else_value, else_value.span);
                }
            } else {
                self.check_nonnull_expr(expr, span);
            }
        }
    }

    /// True when an expression is a local place its flow already narrowed.
    fn expr_is_narrowed(&self, expr: &Expr) -> bool {
        self.plain_binding(expr)
            .is_some_and(|(id, projections)| self.state.place(id).is_narrowed(&projections))
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
        // A raw read of the target place (`x = *(&x)`,
        // `a[i].inner = *(&a[i].inner)`) copies the place onto itself: the
        // read would empty the just-stored pointee. The skip runs after the
        // target's writability and lend checks and still emits the target's
        // index guard, so the no-op transfer keeps its observable checks.
        let raw_self = self.is_raw_self_alias(target, value);
        // An identical projected target and value (`w.items[i] = w.items[i]`)
        // is a self-copy. The skip is limited to a generic body (the instance
        // decides ownership) or a run-time element; a concrete constant-index
        // self-copy keeps the checker's partial-move limit. A duplicable
        // instance keeps its ordinary copy.
        let repeated = self.place_target(target);
        let (projected, runtime_element) = match &repeated {
            Some(PlaceTarget::Local(_, path)) | Some(PlaceTarget::Global(_, path)) => (
                !path.is_empty(),
                path.iter()
                    .any(|projection| matches!(projection, ir::Projection::Element(None))),
            ),
            None => (false, false),
        };
        let same_place = projected
            && (target_ty.is_unknown() || runtime_element)
            && same_place_expr(target, value)
            && (target_ty.is_unknown()
                || !self.sema.table.duplicable(&target_ty)
                || self.maybe_owns(&target_ty));
        let skip_transfer = compound.is_none() && (raw_self || same_place);
        // A dereferenced raw pointer is a store target: the pointer names the
        // storage, so the store writes through it in the same shape an indexed
        // element store uses. A member or element over the dereference selects
        // part of the pointee (`(*p).f`, `(*p).arr[i].x`), so the whole chain
        // is a place (Ch 04 §4). The pointer expression evaluates first, an
        // updating value settles at its own position before the store, and the
        // target's member and element selections are pinned before the
        // right-hand side is lowered, so a side effect cannot change the slot
        // the store reaches. The structural `.len` target keeps its own
        // rejection below.
        if !self.structural_len_target(target) {
            if let Some((deref, projections)) = raw_deref_store_target(target) {
                let pointer_expr = match &deref.kind {
                    ExprKind::Unary {
                        op: UnaryOp::Deref,
                        operand,
                    } => operand,
                    _ => unreachable!("a raw store target roots at a dereference"),
                };
                // A prefix `unsafe` on a projected target covers the
                // dereference exactly as the enclosing block does. A bare
                // prefix target keeps its existing limitation.
                let prefixed =
                    !projections.is_empty() && matches!(target.kind, ExprKind::Unsafe(_));
                if prefixed {
                    self.unsafe_depth += 1;
                }
                self.check_deref_raw(pointer_expr, target.span);
                if prefixed {
                    self.unsafe_depth -= 1;
                }
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
                // The pointer is bound once whenever it is not a trivial place:
                // the target spelling, the release, and the clear all read the
                // same binding, so an impure pointer expression evaluates
                // exactly once and every use names the same address. A named
                // place keeps its spelling.
                let pointer = self.materialize_operand(pointer);
                let root_ty = self.ty(deref);
                let mut projected = Value::new(
                    root_ty,
                    ValueKind::Unary {
                        op: UnaryOp::Deref,
                        operand: Box::new(pointer),
                    },
                );
                for projection in projections {
                    let (kind, whole) = match projection {
                        RawStoreProjection::Field { name, whole } => (
                            ValueKind::Member {
                                base: Box::new(projected),
                                name: name.to_owned(),
                            },
                            whole,
                        ),
                        RawStoreProjection::Index { index, whole } => {
                            let index = self.lower_value(index);
                            (
                                ValueKind::Index {
                                    base: Box::new(projected),
                                    index: Box::new(index),
                                },
                                whole,
                            )
                        }
                    };
                    projected = Value::new(self.ty(whole), kind);
                }
                projected.ty = element_ty.clone();
                // A side-effecting value may change an intermediate raw field
                // before the C lvalue is evaluated. Pin the intermediate
                // member's address first, then store through that address.
                let projected = if Self::mentions_call(value)
                    || super::lower::contains_update(value)
                {
                    self.pin_intermediate_raw_member(projected)
                } else {
                    projected
                };
                // Pin every member/element base and index before the value is
                // lowered; the pointer itself is already materialized.
                let projected = self.pin_store_target(projected);
                // A raw-deref target that names the same chain as its value is
                // a self-copy. The pointer and every index still evaluate and
                // guard; only the transfer is skipped.
                if compound.is_none()
                    && (same_place_expr(target, value) || raw_self)
                {
                    // The no-op still reads the selected storage: its pointer
                    // and index expressions evaluate, the bounds guard runs,
                    // and a local pointer stays visibly used in the emitted C.
                    self.emit(Statement::Discard {
                        value: projected,
                    });
                    self.check_value_into(value, &target_ty, value.span);
                    return;
                }
                let pointer = Value::new(
                    element_ty.clone(),
                    ValueKind::Unary {
                        op: UnaryOp::AddressOf,
                        operand: Box::new(projected.clone()),
                    },
                );
                let stored = match compound {
                    Some(binary) => {
                        // The compound store reads the selected storage through
                        // that same address, before the side runs; an updating
                        // side materializes the read first.
                        let element = if pin {
                            self.pin_update_value(projected)
                        } else {
                            projected
                        };
                        let right = self.lower_value(value);
                        let right = if pin { self.pin_update_value(right) } else { right };
                        self.combine(binary, element, right, &element_ty)
                    }
                    None => {
                        // The store consumes an owned value exactly as a named
                        // destination does, so a later use of the source reports
                        // the moved-value error and the emission moves it in. A
                        // view target only borrows, so it leaves the owner
                        // readable.
                        let view_target = is_view(&element_ty)
                            || (self.view_destination_adopts && is_view_like(&element_ty));
                        if !view_target {
                            self.consume_owned_place(value, value.span);
                        }
                        let lowered = self.lower_value(value);
                        let lowered = self.materialize_owning_deref(lowered);
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
                let deferred_owned_move = self.record_opaque_store(
                    id,
                    &projections,
                    value,
                    &target_ty,
                    caller_slot,
                    target.span,
                );
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
                if !projections.is_empty() && !is_view_like(&target_ty) && !deferred_owned_move && !skip_transfer {
                    if self.place_target(value).is_some() {
                        self.consume_owned_place(value, value.span);
                    } else {
                        let value_ty = self.ty(value);
                        let maybe_owning = self.maybe_owns(&value_ty);
                        if maybe_owning {
                            let direct = self.maybe_owning_source_arms(value);
                            if direct.is_empty() {
                                for source in self.maybe_owning_projection_move_sources(value) {
                                    if let PlaceTarget::Local(source_id, source_projections) = &source
                                    {
                                        if *source_id == id
                                            && source_projections == &projections
                                        {
                                            continue;
                                        }
                                    }
                                    self.note_maybe_element_move(&source, value.span);
                                    self.mark_moved_if_owning(source, value.span);
                                }
                            } else {
                                for (arm, source) in direct {
                                    if let PlaceTarget::Local(source_id, _) = &source {
                                        if *source_id == id
                                            && same_place_expr(target, arm)
                                            && !self.destination_index_may_change(target)
                                        {
                                            continue;
                                        }
                                    }
                                    self.note_maybe_element_move(&source, value.span);
                                    self.mark_moved_if_owning(source, value.span);
                                }
                            }
                        } else {
                            // A conditional arm that spells the destination's
                            // own place with a stable index is a self-copy.
                            // A different spelling, or a global/computed index
                            // the condition may change, is an element move that
                            // the frozen v1 rule rejects.
                            for (arm, source) in self.conditional_move_source_arms(value) {
                                if let PlaceTarget::Local(source_id, _) = &source {
                                    if *source_id == id
                                        && same_place_expr(target, arm)
                                        && !self.destination_index_may_change(target)
                                    {
                                        continue;
                                    }
                                }
                                self.move_target(source, value.span);
                            }
                        }
                        // A conditional arm rooted at a type parameter is a
                        // move only at some instances; the projected store
                        // records the per-instance duty for the audit.
                        for source in self.type_param_conditional_sources(value) {
                            self.mark_moved_if_owning(source, value.span);
                        }
                        for source in self.maybe_owning_projection_move_sources(value) {
                            self.mark_moved_if_owning(source, value.span);
                        }
                    }
                }
                if compound.is_some() {
                    self.read_place(id, &projections, target.span);
                }
                if is_view_like(&target_ty)
                    || is_view_like(&self.ty(value))
                    || (projections.is_empty() && self.is_out_slot_binding(id))
                {
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
                                None,
                                false,
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
                                Some(*root_id),
                                self.is_out_slot_binding(*root_id) && projections.is_empty(),
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
                                Some(id),
                                self.is_out_slot_binding(id) && projections.is_empty(),
                            );
                        }
                        _ => {}
                    }
                }
                let target_binding_ty = &self.bindings[id].ty;
                let generic_maybe_nullable = target_binding_ty.is_unknown()
                    || crate::types::contains_param(target_binding_ty);
                let value_ty = self.ty(value);
                let concrete_reports = self.is_out_slot_binding(id)
                    && (is_whole_slot_view(&target_ty)
                        || is_whole_slot_view(&value_ty));
                if projections.is_empty()
                    && self.bindings[id].mode == Mode::Borrow
                    && generic_maybe_nullable
                    && !concrete_reports
                {
                    let parameter_root = self.whole_slot_store_root(value, id, true);
                    let local_source = if parameter_root.is_none() {
                        self.store_local_source_name(value)
                    } else {
                        None
                    };
                    if parameter_root.is_some() || local_source.is_some() {
                        let destination = self.bindings[id].name.clone();
                        let ir_name = self.bindings[id].ir_name.clone();
                        if self.recorded_whole_out_slot_store.insert((
                            ir_name,
                            target.span.start,
                            target.span.end,
                        )) {
                            let function = self.decl.name.clone();
                            let source = parameter_root
                                .map(|owner| self.bindings[owner].name.clone())
                                .or(local_source)
                                .unwrap_or_default();
                            let source_ty = self.ty(value);
                            let source_is_capture = parameter_root.is_some_and(|owner| {
                                matches!(
                                    self.bindings[owner].base,
                                    Some(ir::PlaceBase::EnvField { .. })
                                )
                            });
                            let source_is_take = parameter_root
                                .is_some_and(|owner| self.bindings[owner].mode == Mode::Take);
                            let source_is_copy = matches!(&value.kind, ExprKind::Name(_))
                                && self
                                    .place_target(value)
                                    .is_some_and(|target| match target {
                                        PlaceTarget::Local(owner, projections) => {
                                            projections.is_empty()
                                                && self.bindings[owner].copy_source.is_some()
                                        }
                                        _ => false,
                                    });
                            let writable_source = parameter_root
                                .is_some_and(|owner| self.bindings[owner].mode == Mode::Borrow);
                            let slot_ty = self.bindings[id].ty.clone();
                            self.whole_out_slot_store_requirements.push(
                                ir::RequiresWholeOutSlotStore {
                                    function,
                                    destination,
                                    source,
                                    slot_ty,
                                    source_ty,
                                    source_is_capture,
                                    source_is_take,
                                    source_is_local: parameter_root.is_none(),
                                    source_is_copy,
                                    writable_source,
                                    span: target.span,
                                },
                            );
                        }
                    }
                }
                self.write_place(id, &projections, target.span);
                if !projections.is_empty() || !is_view_like(&target_ty) {
                    let root = self.bindings[id].ir_name.clone();
                    let mut roots = BTreeSet::new();
                    self.expr_view_roots(value, &mut roots);
                    let aliases_target = roots
                        .iter()
                        .any(|source| self.bindings[*source].ir_name == root);
                    // A whole-root assignment replaces the target's view even
                    // when the right side carries no duty of its own. A
                    // projected store only replaces a carrier position when
                    // the value is itself a carrier.
                    let source_has_duties = roots.iter().any(|source| {
                        let name = &self.bindings[*source].ir_name;
                        self.opaque_store_requirements
                            .iter()
                            .any(|duty| duty.root == *name)
                    });
                    if projections.is_empty() && !aliases_target {
                        self.supersede_opaque_duties(&root, &[]);
                    } else if !projections.is_empty() && source_has_duties {
                        self.supersede_opaque_duties(&root, &projections);
                    }
                    if source_has_duties {
                        self.transfer_opaque_store_duties(
                            id,
                            &roots,
                            value,
                            &projections,
                            target.span,
                        );
                    }
                }
                let lowered = if has_runtime_element(&projections) {
                    self.emit_indexed_store(target, value, compound, skip_transfer)
                } else {
                    if let Some(guards) = self.guard_statement_for_store_target(target) {
                        self.emit(guards);
                    }
                    if skip_transfer {
                        self.check_value_into(value, &target_ty, value.span);
                        return;
                    }
                    let place = self.target_place(&PlaceTarget::Local(id, projections.clone()));
                    self.emit_assigned(place, &target_ty, value, compound)
                };
                if projections.is_empty() {
                    self.bind_value(id, &target_ty, value, lowered.as_ref(), target.span);
                } else if compound.is_none()
                    && (is_view_like(&target_ty) || is_view_like(&self.ty(value)))
                {
                    // An element store changes one position of a view array.
                    // The whole binding keeps the union of its element sources,
                    // so a later element read sees the stored view. The union
                    // is conservative for a sibling element; it never accepts
                    // a frame-backed source.
                    let atoms = self.value_source_atoms(value);
                    if !atoms.is_empty() {
                        self.state.places.entry(id).or_default().join_sources(&atoms);
                        self.bindings[id].rebound = true;
                    }
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
                // exactly as for the single view, and for a projected
                // destination exactly as for the whole binding. A value that is
                // not a place read (a conditional or an array literal) supplies
                // its positions arm by arm and is written as one union. A
                // run-time destination keeps a wildcard position, because it
                // cannot name the element it wrote.
                let moved = match self.place_target(value) {
                    Some(PlaceTarget::Local(source, source_projections))
                        if super::place::constant_path(&source_projections)
                            && (!is_view_like(&target_ty) || projections.is_empty()) =>
                    {
                        self.move_element_loans(source, &source_projections, id, &projections);
                        true
                    }
                    _ => false,
                };
                if !moved && is_view_like(&target_ty) && self.collects_copy_positions(value) {
                    self.record_copy_element_loans(
                        id,
                        &projections,
                        value,
                        lowered.as_ref(),
                        &target_ty,
                        target.span,
                    );
                } else if !moved && !is_view_like(&target_ty) {
                    // A composite copy must be collected before the target's
                    // record is cleared: the source may be the target itself
                    // (`h = c ? h : h`), and clearing first would lose its
                    // loans. The collected union is applied after the clear.
                    let copy_loans = if self.may_carry_view(&target_ty)
                        && (self.collects_copy_positions(value)
                            || FunctionChecker::mentions_call(value))
                    {
                        Some(self.snapshot_copy_element_loans(
                            value,
                            lowered.as_ref(),
                            &target_ty,
                            &projections,
                        ))
                    } else {
                        None
                    };
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
                    if let Some(copied) = copy_loans {
                        self.apply_copy_element_loans(
                            id,
                            &projections,
                            copied,
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
                    self.check_escaping_view_store(
                        value,
                        &target_ty,
                        &name,
                        true,
                        target.span,
                        None,
                        false,
                    );
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
                    let frame_backed = if let Some(PlaceTarget::Local(source_id, _)) =
                        self.place_target(value)
                    {
                        !self.binding_loan_owners(source_id).is_empty()
                    } else {
                        // A value that creates its own view storage — a fresh
                        // call result, or a literal or conditional of them —
                        // dies with the statement, so the global cannot keep
                        // it. A local- or parameter-backed position already
                        // reported through its field's own judgment. A
                        // conditional contributes each arm the same way, so a
                        // local carrier arm is judged even though no single
                        // place names the whole value.
                        (self.literal_view_loan_positions(value).is_empty()
                            && self.names_fresh_view_storage(value, &target_ty))
                            || self.carrier_copy_escapes(value)
                    };
                    if frame_backed {
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
                        self.emit_indexed_store(target, value, compound, skip_transfer);
                    } else {
                        if let Some(guards) = self.guard_statement_for_store_target(target) {
                            self.emit(guards);
                        }
                        if skip_transfer {
                            self.check_value_into(value, &target_ty, value.span);
                            return;
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
    pub(super) fn is_exact_self_assign(
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

    /// True when the store's value is a raw read of the target place itself.
    ///
    /// `x = *(&x)` and `a[i].inner = *(&a[i].inner)` copy a place onto itself
    /// through a raw pointer. The read would empty the pointer's pointee after
    /// the copy, so the store is a no-op. The recognizer unwraps
    /// `Deref(AddressOf(place))` (through `Unsafe`) and compares the inner
    /// place with the target; a different binding or projection is an ordinary
    /// move. `place_target` stays `Deref`-free: this recognizer runs only on
    /// the store path.
    pub(super) fn is_raw_self_alias(&self, target: &Expr, value: &Expr) -> bool {
        let value = unwrap_unsafe_expr(value);
        let ExprKind::Unary {
            op: UnaryOp::Deref,
            operand,
        } = &value.kind
        else {
            return false;
        };
        let operand = unwrap_unsafe_expr(operand);
        let ExprKind::Unary {
            op: UnaryOp::AddressOf,
            operand,
        } = &operand.kind
        else {
            return false;
        };
        same_place_expr(target, operand)
    }

    /// Records a store into a member the declaration sees through a type
    /// parameter.
    ///
    /// The destination type is unresolved here, so the declaration cannot tell
    /// whether the instance makes the member a view or an owner. A view-like
    /// source (or an owned `String`, which widens to `str`) records its
    /// frame-dying backing; a type-parameter value defers only its view
    /// judgment and keeps the ordinary maybe-owning store path. The
    /// per-instance audit applies the instance's conversion and reports the
    /// escape when the root is returned or when a caller-retained field tries
    /// to keep a view of another lend.
    ///
    /// An owned source is not moved here: the instance decides whether the
    /// destination owns it (a move) or holds a widened view (a copy). The
    /// recorded source lets the audit report the concrete moved-value or
    /// lend-move error when the destination owns, and lets the body judge a
    /// later read or write of the source. Returns true when the caller must
    /// skip the declaration's move.
    fn record_opaque_store(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        value: &Expr,
        target_ty: &Ty,
        caller_slot: bool,
        span: Span,
    ) -> bool {
        if self.speculative_lowering {
            return false;
        }
        if projections.is_empty() {
            return false;
        }
        let declared_root = self.bindings[id].ty.clone();
        if !(declared_root.is_unknown() || crate::types::contains_param(&declared_root)) {
            return false;
        }
        let value_ty = self.ty(value);
        let maybe_value = value_ty.is_unknown() || crate::types::contains_param(&value_ty);
        let view_source = is_view_like(&value_ty) || maybe_value;
        let owned_source = matches!(&value_ty.kind, TyKind::Handle(name) if name == "String");
        let null_source = crate::types::is_null_shaped(&value_ty);
        if !view_source && !owned_source && !null_source {
            return false;
        }
        // The declaration cannot judge a type-parameter value; the instance
        // decides whether it is a view, a duplicable value, or an owner. Record
        // the duty for the view judgment but keep the declaration's ordinary
        // maybe-owning path, so an owning instance still gets the concrete move
        // judgment.
        let maybe_view_only = maybe_value && !owned_source;
        // A concrete target under a generic root is still a view destination
        // at every instance (for example a fixed `str[2]` field). A concrete
        // non-view target keeps the ordinary store path.
        if !(target_ty.is_unknown()
            || crate::types::contains_param(target_ty)
            || is_view_like(target_ty))
        {
            return false;
        }
        // An owned source that names a place can be moved or widened; a call
        // result or literal has no later use to report but still records the
        // view's frame so a returned carrier reports the escape.
        let source_place = if owned_source || maybe_view_only {
            match self.place_target(value) {
                Some(PlaceTarget::Local(source, source_path)) => {
                    let source = self.alias_resolved_root(source);
                    Some((source, source_path))
                }
                _ => None,
            }
        } else {
            None
        };
        // A conditional's arms are separate sources. The declaration cannot
        // tell whether the destination owns them or widens them to a view, so
        // every arm that may move is recorded for the instance audit. A
        // caller-backed lend parameter contributes no dying frame; a local or
        // by-value parameter keeps one.
        let source_arm_pairs: Vec<(usize, bool, ir::OpaqueSource)> =
            if owned_source && source_place.is_none() {
                self.conditional_move_sources(value)
                    .into_iter()
                    .filter_map(|target| match target {
                        PlaceTarget::Local(source, source_path) => {
                            let source = self.alias_resolved_root(source);
                            let binding = &self.bindings[source];
                            let caller_backed = binding.kind == BindingKind::Param
                                && matches!(binding.mode, Mode::Borrow | Mode::Const);
                            let param = binding.kind == BindingKind::Param
                                && binding.mode != Mode::Take;
                            Some((
                                source,
                                caller_backed,
                                ir::OpaqueSource {
                                    ir: binding.ir_name.clone(),
                                    name: binding.name.clone(),
                                    path: source_path,
                                    param,
                                    later_uses: Vec::new(),
                                },
                            ))
                        }
                        PlaceTarget::Global(_, _) => None,
                    })
                    .collect()
            } else {
                Vec::new()
            };
        // A `for (borrow e in xs)` alias has no storage of its own: the store
        // reaches the container element, so the duty roots at the container
        // with the alias's own path followed by the written path.
        let (root_id, mut path) = match &self.bindings[id].alias_root {
            Some(PlaceTarget::Local(root, root_path)) => (*root, root_path.clone()),
            _ => (id, Vec::new()),
        };
        path.extend_from_slice(projections);
        let root = self.bindings[root_id].ir_name.clone();
        let root_name = self.bindings[root_id].name.clone();
        let whole_out_slot =
            self.is_out_slot_binding(id) && projections.is_empty();
        if !self
            .recorded_opaque_stores
            .insert((root.clone(), span.start, span.end))
        {
            return false;
        }
        // A store at or above an earlier path replaces whatever view that path
        // held. A duty that already recorded a use or a backing write keeps
        // that report; a duty that carries an owned-source move keeps the move
        // judgment; every other overwritten duty is dropped.
        self.supersede_opaque_duties(&root, &path);
        let mut frames: Vec<ir::OpaqueFrame> = Vec::new();
        for atom in Self::storage_atoms(&self.view_provenance(value)) {
            if self.provenance_fits_destination(&atom, false) {
                continue;
            }
            let frame = match atom {
                Provenance::Local(owner) | Provenance::Param(owner) => {
                    let name = self.bindings[owner].name.clone();
                    ir::OpaqueFrame {
                        binding: Some(self.bindings[owner].ir_name.clone()),
                        name: Some(name.clone()),
                        subject: super::liveness::owner_subject(
                            &name,
                            self.bindings[owner].generated,
                        ),
                        scope_end: self.bindings[owner].scope_end,
                    }
                }
                // A value the provenance walk cannot name is a temporary only
                // when it actually owns or derives from fresh storage; a
                // no-source view call or an aggregate of static views names
                // no backing and must stay accepted.
                _ => continue,
            };
            if !frames.contains(&frame) {
                frames.push(frame);
            }
        }
        for (source, caller_backed, _) in &source_arm_pairs {
            if *caller_backed {
                continue;
            }
            let name = self.bindings[*source].name.clone();
            let frame = ir::OpaqueFrame {
                binding: Some(self.bindings[*source].ir_name.clone()),
                name: Some(name.clone()),
                subject: super::liveness::owner_subject(
                    &name,
                    self.bindings[*source].generated,
                ),
                scope_end: self.bindings[*source].scope_end,
            };
            if !frames.contains(&frame) {
                frames.push(frame);
            }
        }
        let fresh_arm = owned_source
            && source_place.is_none()
            && self.conditional_has_fresh_arm(value);
        // A fresh arm always contributes a statement-temporary backing, even
        // beside a named local arm: the view may select the fresh value, which
        // dies at the statement. A named local arm contributes its own frame
        // separately, so the synthetic frame is added in addition.
        if fresh_arm
            || (frames.is_empty()
                && source_arm_pairs.is_empty()
                && ((owned_source && source_place.is_none())
                    || (!owned_source && self.frame_owned_view_owner(value).is_some())))
        {
            frames.push(ir::OpaqueFrame {
                binding: None,
                name: None,
                subject: "a statement temporary".to_owned(),
                scope_end: Span::default(),
            });
        }
        let frame_owner = frames.first().map(|frame| frame.subject.clone());
        // A caller-retained container field or element cannot keep a view of a
        // different lend parameter: the lend is call-scoped, so the instance
        // audit reports the field escape. Record every parameter backing the
        // value derives from; a copy within the same carrier is exempt.
        let mut parameter_backed_sources = std::collections::BTreeSet::new();
        for atom in Self::storage_atoms(&self.view_provenance(value)) {
            if let Provenance::Param(owner) = atom {
                parameter_backed_sources.insert(self.bindings[owner].ir_name.clone());
            }
        }
        if let Some((source, _)) = &source_place {
            let binding = &self.bindings[*source];
            if binding.kind == BindingKind::Param && binding.mode != Mode::Take {
                parameter_backed_sources.insert(binding.ir_name.clone());
            }
        }
        for (source, _, _) in &source_arm_pairs {
            let binding = &self.bindings[*source];
            if binding.kind == BindingKind::Param && binding.mode != Mode::Take {
                parameter_backed_sources.insert(binding.ir_name.clone());
            }
        }
        if maybe_view_only {
            for owner in self.value_source_roots(value) {
                let binding = &self.bindings[owner];
                if binding.kind == BindingKind::Param && binding.mode != Mode::Take {
                    parameter_backed_sources.insert(binding.ir_name.clone());
                }
            }
        }
        let fresh_arm = owned_source
            && source_place.is_none()
            && self.conditional_has_fresh_arm(value);
        let (source_ir, source_name, source_path, source_param) = match source_place {
            Some((source, source_path)) => {
                let binding = &self.bindings[source];
                let parameter =
                    binding.kind == BindingKind::Param && binding.mode != Mode::Take;
                (
                    Some(binding.ir_name.clone()),
                    Some(binding.name.clone()),
                    source_path,
                    parameter,
                )
            }
            None => (None, None, Vec::new(), false),
        };
        self.opaque_store_requirements.push(ir::RequiresOpaqueStore {
            function: self.decl.name.clone(),
            ty: declared_root,
            path,
            value: value_ty,
            frame_owner,
            frames,
            source_ir,
            source_name,
            source_path,
            source_param,
            source_arms: source_arm_pairs
                .into_iter()
                .map(|(_, _, source)| source)
                .collect(),
            fresh_arm,
            maybe_view_only,
            whole_out_slot,
            parameter_backed_sources: parameter_backed_sources.into_iter().collect(),
            root_name,
            later_uses: Vec::new(),
            late_uses: Vec::new(),
            invalidations: Vec::new(),
            moved_owner: Vec::new(),
            superseded: false,
            store_span: span,
            return_span: None,
            caller_slot,
            root,
        });
        // An owned String source with no single place — a conditional, a call
        // result, or a literal — is also decided per instance: the destination
        // may be a view (a widening) or an owner (a move). The caller must not
        // run the declaration's move in either case. A type-parameter value
        // keeps the ordinary maybe-owning path; only its view judgment is
        // deferred to the audit.
        if maybe_view_only {
            false
        } else {
            owned_source
        }
    }

    /// Ends the view watches a replaced opaque store no longer owns.
    ///
    /// A duty that already recorded a use or a backing write keeps that
    /// report, and a duty that carries an owned-source move keeps the move
    /// judgment; every other overwritten duty is dropped.
    fn supersede_opaque_duties(&mut self, root: &str, path: &[ir::Projection]) {
        let mut kept = Vec::new();
        for mut duty in std::mem::take(&mut self.opaque_store_requirements) {
            let replaced = duty.function == self.decl.name
                && duty.root == root
                && projection_path_covers(path, &duty.path);
            if !replaced {
                kept.push(duty);
                continue;
            }
            if !duty.late_uses.is_empty() || !duty.invalidations.is_empty() {
                kept.push(duty);
                continue;
            }
            if duty.source_ir.is_some() || !duty.source_arms.is_empty() {
                duty.superseded = true;
                kept.push(duty);
            }
        }
        self.opaque_store_requirements = kept;
    }

    /// Records a read that reaches an opaque store's destination or source.
    ///
    /// A read of the carrier after the backing's scope closes hands the caller
    /// a dangling view, and a read of an owned source after a store that may
    /// have moved it is the concrete moved-value error at an owning instance.
    /// The per-instance audit decides which kind the destination actually is.
    fn note_opaque_use(&mut self, expr: &'a Expr, span: Span) {
        if self.speculative_lowering || self.opaque_store_requirements.is_empty() {
            return;
        }
        let Some(PlaceTarget::Local(id, projections)) = self.place_target(expr) else {
            return;
        };
        let id = self.alias_resolved_root(id);
        let root = self.bindings[id].ir_name.clone();
        // Snapshot the flow-sensitive re-initialization set: the duty loop
        // mutates the check state, and a definite write on an earlier path is
        // what makes this read legal.
        let reinitialized = self.state.reinitialized_sources.clone();
        for duty in &mut self.opaque_store_requirements {
            if duty.function != self.decl.name {
                continue;
            }
            if duty.root == root && !duty.superseded {
                // A temporary backing dies at the store's statement, so every
                // later read is dangling. A named backing dies at its scope's
                // close.
                let dying = duty
                    .frames
                    .iter()
                    .find(|frame| match &frame.binding {
                        Some(_) => frame.scope_end.end <= span.start,
                        None => true,
                    });
                if let Some(frame) = dying {
                    push_opaque_use(&mut duty.late_uses, span, &frame.subject);
                }
            }
            if duty.source_ir.as_deref() == Some(root.as_str())
                && projection_paths_overlap(&duty.source_path, &projections)
                && !reinitialized.iter().any(|(source, written)| {
                    *source == id && projection_path_covers(written, &duty.source_path)
                })
            {
                push_opaque_span(&mut duty.later_uses, span);
            }
            for source in &mut duty.source_arms {
                if source.ir == root
                    && projection_paths_overlap(&source.path, &projections)
                    && !reinitialized.iter().any(|(owner, written)| {
                        *owner == id && projection_path_covers(written, &source.path)
                    })
                {
                    source.later_uses.push((span, projections.clone()));
                    break;
                }
            }
        }
    }

    /// Records a write or move that reaches an opaque store's backing.
    ///
    /// Writing the backing after a view was stored into the carrier frees or
    /// replaces the storage the view reads. At a view instance the write is
    /// the concrete shared-loan conflict and a move the borrow conflict; at an
    /// owning instance the same access is a use after the move the store
    /// performed. A write that re-initializes the owned source ends its moved
    /// state instead.
    pub(super) fn note_opaque_owner_write(
        &mut self,
        id: usize,
        projections: &[ir::Projection],
        span: Span,
        moved: bool,
    ) {
        if self.speculative_lowering || self.opaque_store_requirements.is_empty() {
            return;
        }
        let id = self.alias_resolved_root(id);
        let root = self.bindings[id].ir_name.clone();
        for duty in &mut self.opaque_store_requirements {
            if duty.function != self.decl.name {
                continue;
            }
            let frame = duty
                .frames
                .iter()
                .find(|frame| frame.binding.as_deref() == Some(root.as_str()))
                .map(|frame| frame.subject.clone());
            if let Some(subject) = frame {
                if moved {
                    push_opaque_use(&mut duty.moved_owner, span, &subject);
                } else {
                    push_opaque_use(&mut duty.invalidations, span, &subject);
                }
            }
            if duty.source_ir.as_deref() == Some(root.as_str())
                && projection_paths_overlap(&duty.source_path, projections)
                && moved
            {
                push_opaque_span(&mut duty.later_uses, span);
            }
            for source in &mut duty.source_arms {
                if source.ir == root
                    && projection_paths_overlap(&source.path, projections)
                    && moved
                {
                    source.later_uses.push((span, projections.to_vec()));
                    break;
                }
            }
        }
        // Track the write itself in the flow state. A definite write covering
        // a source makes a later read legal; a branch write joins by
        // intersection, so only an all-path write qualifies. A move removes
        // the covered entries instead.
        if moved {
            self.state.reinitialized_sources.retain(|(source, written)| {
                *source != id || !projection_paths_overlap(written, projections)
            });
        } else {
            self.state
                .reinitialized_sources
                .insert((id, projections.to_vec()));
        }
    }

    /// Reports a view store that escapes the frame its destination keeps.
    ///
    /// A view-typed destination stores view storage, so the judgment follows
    /// the frame the destination holds. A global or a field lives for the
    /// whole program: only module-level storage survives it, so a parameter-,
    /// The place roots a value can derive a view from.
    ///
    /// A conditional recurses into its arms. A call result may return a view of
    /// one of its arguments (a generic identity, for example), so its arguments
    /// and receiver contribute when the call has no recorded provenance of its
    /// own; a call whose result is already known to be static or module-backed
    /// contributes nothing. A call/literal with no roots has no caller-backed
    /// source.
    fn value_source_roots(&self, value: &Expr) -> Vec<usize> {
        let mut roots = Vec::new();
        self.collect_value_source_roots(value, &mut roots);
        roots.sort_unstable();
        roots.dedup();
        roots
    }

    /// Recurses through the shapes a view or owned source can hide in.
    fn collect_value_source_roots(&self, value: &Expr, roots: &mut Vec<usize>) {
        match &value.kind {
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.collect_value_source_roots(then_value, roots);
                self.collect_value_source_roots(else_value, roots);
            }
            ExprKind::Unsafe(inner) => self.collect_value_source_roots(inner, roots),
            ExprKind::Call { callee, args, .. } => {
                let provenance = self.view_provenance(value);
                if matches!(
                    provenance,
                    Provenance::Static | Provenance::Global(_)
                ) {
                    return;
                }
                // A named user or stdlib call has a signature and a return
                // expression summary. If it named no source, the result does
                // not derive from an argument; a static-returning helper with
                // an unused lend argument must not become a false escape.
                if self.func_binding_callee(callee).is_none()
                    && self.callee_field_type(callee).is_none()
                {
                    if let Some(name) = self.callee_name(callee) {
                        if self.resolved_callee(&name).is_some() {
                            return;
                        }
                    }
                }
                for arg in args {
                    let ty = self.ty(&arg.value);
                    if is_view_like(&ty)
                        || self.may_view_from_storage(&ty)
                        || ty.is_unknown()
                        || crate::types::contains_param(&ty)
                    {
                        self.collect_value_source_roots(&arg.value, roots);
                    }
                }
            }
            _ => {
                if let Some(PlaceTarget::Local(owner, _)) = self.place_target(value) {
                    roots.push(owner);
                }
            }
        }
    }

    /// The parameters a stored value's view or owned place derives from.
    ///
    /// A view's provenance names its backing parameter; an owned `String`
    /// place names its binding, and a conditional recurses into its arms. A
    /// local, a `take` parameter, a fresh result, and module storage are not
    /// caller-backed lend sources.
    fn dynamic_parameter_roots(&self, value: &Expr) -> Vec<usize> {
        let mut roots = Vec::new();
        for atom in Self::storage_atoms(&self.view_provenance(value)) {
            if let Provenance::Param(owner) = atom {
                roots.push(owner);
            }
        }
        self.owned_source_roots(value, &mut roots);
        roots.sort_unstable();
        roots.dedup();
        roots
            .into_iter()
            .filter(|owner| {
                let binding = &self.bindings[*owner];
                (binding.kind == BindingKind::Param && binding.mode != Mode::Take)
                    || binding.kind == BindingKind::OutSlot
            })
            .collect()
    }

    /// The first foreign lend root a whole out-slot store would keep.
    ///
    /// A `borrow T?` slot owns its own incoming value, but a different lend's
    /// value or view is call-scoped. A `take` source owns its value; a capture
    /// that dies with the enclosing frame is the existing frame-backed case.
    /// The value is static when its type is `static str` or every conditional
    /// arm is static; a static position outlives the call and is accepted.
    fn foreign_lend_store_root(&self, value: &Expr, destination: usize) -> Option<usize> {
        self.whole_slot_store_root(value, destination, false)
    }

    /// The source root of a whole-slot store.
    ///
    /// `include_take` records a `take` root for the generic duty, which must
    /// decide per instance whether the taken source is an accepted owning
    /// value or a view that dies with the frame. The concrete judgment keeps
    /// its existing `take` handling and passes `false`.
    fn whole_slot_store_root(
        &self,
        value: &Expr,
        destination: usize,
        include_take: bool,
    ) -> Option<usize> {
        let value = super::expr::structural_wrapper_inner(value);
        if self.is_static_view_source(value) {
            return None;
        }
        if let ExprKind::Conditional {
            then_value,
            else_value,
            ..
        } = &value.kind
        {
            // A foreign non-`take` arm wins over a taken arm: the plan gives
            // `E10034` precedence when a conditional mixes both kinds.
            if let Some(owner) = self
                .whole_slot_store_root(then_value, destination, false)
                .or_else(|| self.whole_slot_store_root(else_value, destination, false))
            {
                return Some(owner);
            }
            if include_take {
                if let Some(owner) = self
                    .whole_slot_store_root(then_value, destination, true)
                    .or_else(|| self.whole_slot_store_root(else_value, destination, true))
                {
                    return Some(owner);
                }
            }
        }
        for atom in Self::storage_atoms(&self.store_source_provenance(value)) {
            let Provenance::Param(owner) = atom else {
                continue;
            };
            if owner == destination {
                continue;
            }
            let binding = &self.bindings[owner];
            if binding.kind == BindingKind::Param
                && binding.mode == Mode::Take
                && !include_take
            {
                continue;
            }
            if binding.capture_dies_with_frame {
                continue;
            }
            return Some(owner);
        }
        None
    }

    /// True when a value's storage is static for every possible result.
    ///
    /// A `static str` type names module-level storage, and a conditional is
    /// static only when both arms are; `null` names no storage. A `str` field
    /// of a by-value aggregate is not static-typed even when its initializer is
    /// a literal, so the caller cannot prove it and the store remains a
    /// recorded precision loss.
    fn is_static_view_source(&self, value: &Expr) -> bool {
        if matches!(self.ty(value).kind, TyKind::StaticStr) {
            return true;
        }
        match &value.kind {
            ExprKind::Str(_) | ExprKind::Null => true,
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.is_static_view_source(then_value) && self.is_static_view_source(else_value)
            }
            _ => false,
        }
    }

    /// Collects the bindings an owned place value or conditional names.
    fn owned_source_roots(&self, value: &Expr, roots: &mut Vec<usize>) {
        match &value.kind {
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.owned_source_roots(then_value, roots);
                self.owned_source_roots(else_value, roots);
            }
            ExprKind::Unsafe(inner) => self.owned_source_roots(inner, roots),
            _ => {
                if let Some(PlaceTarget::Local(owner, _)) = self.place_target(value) {
                    roots.push(owner);
                }
            }
        }
    }

    /// The storage atoms a store value names, following rebound copies and
    /// every composite spelling.
    ///
    /// A bare rebound local resolves through its joined set. A conditional
    /// unions its arms, a projection follows its base, and a call or literal
    /// unions its arguments or fields, so a store through any of those shapes
    /// sees the same storage a bare name would. A local whose set is empty is a
    /// real frame-backed view and stays a local atom.
    fn value_source_atoms(&self, value: &Expr) -> Vec<Provenance> {
        let mut atoms = Vec::new();
        self.collect_value_source_atoms(super::expr::structural_wrapper_inner(value), &mut atoms);
        atoms
    }

    fn collect_value_source_atoms(&self, value: &Expr, out: &mut Vec<Provenance>) {
        if self.is_static_view_source(value) {
            return;
        }
        let push =
            |out: &mut Vec<Provenance>, atoms: Vec<Provenance>| {
                for atom in atoms {
                    if !out.contains(&atom) {
                        out.push(atom);
                    }
                }
            };
        match &value.kind {
            ExprKind::Name(_) => {
                if let Some(PlaceTarget::Local(id, projections)) = self.place_target(value) {
                    if projections.is_empty() && self.bindings[id].rebound {
                        if let Some(place) = self.state.places.get(&id) {
                            push(out, place.store_sources());
                        }
                        return;
                    }
                }
                push(out, self.source_atoms(&self.store_source_provenance(value)));
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.collect_value_source_atoms(then_value, out);
                self.collect_value_source_atoms(else_value, out);
            }
            ExprKind::Index { base, .. } | ExprKind::Member { base, .. } => {
                self.collect_value_source_atoms(base, out);
            }
            ExprKind::Call { args, .. } => {
                push(out, self.source_atoms(&self.store_source_provenance(value)));
                for arg in args {
                    self.collect_value_source_atoms(&arg.value, out);
                }
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.collect_value_source_atoms(&field.value, out);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.collect_value_source_atoms(element, out);
                }
                if let Some(fill) = fill {
                    self.collect_value_source_atoms(fill, out);
                }
            }
            _ => push(out, self.source_atoms(&self.store_source_provenance(value))),
        }
    }

    /// True when a store value contains a local rebound at least once.
    ///
    /// The joined-set judgment is only the missing half of the ordinary rule:
    /// a value with no rebound local already reaches the declaration
    /// provenance path, and reporting it here would duplicate that report.
    fn mentions_rebound_source(&self, value: &Expr) -> bool {
        let value = super::expr::structural_wrapper_inner(value);
        match &value.kind {
            ExprKind::Name(_) => {
                if let Some(PlaceTarget::Local(id, projections)) = self.place_target(value) {
                    projections.is_empty() && self.bindings[id].rebound
                } else {
                    false
                }
            }
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.mentions_rebound_source(then_value) || self.mentions_rebound_source(else_value)
            }
            ExprKind::Index { base, .. } | ExprKind::Member { base, .. } => {
                self.mentions_rebound_source(base)
            }
            ExprKind::Call { args, .. } => {
                args.iter().any(|arg| self.mentions_rebound_source(&arg.value))
            }
            ExprKind::StructLiteral { fields, .. } => fields
                .iter()
                .any(|field| self.mentions_rebound_source(&field.value)),
            ExprKind::ArrayLiteral { elements, fill } => {
                elements
                    .iter()
                    .any(|element| self.mentions_rebound_source(element))
                    || fill
                        .as_deref()
                        .is_some_and(|fill| self.mentions_rebound_source(fill))
            }
            _ => false,
        }
    }

    /// The first source a store value names for a whole-slot store.
    ///
    /// A destination or value that still mentions a type parameter defers to
    /// the per-instance duty, and a `take` parameter keeps the ordinary path,
    /// which accepts an owned take and reports a take view through the view
    /// rule. The destination's own parameter is skipped, exactly as the
    /// ordinary whole-slot rule skips it. Parameter roots win over local roots
    /// so the store keeps one report.
    fn rebound_source_store_root(
        &self,
        value: &Expr,
        destination: usize,
        target_ty: &Ty,
    ) -> Option<SourceSetRoot> {
        if !self.mentions_rebound_source(value) {
            return None;
        }
        if crate::types::contains_param(target_ty) || crate::types::contains_param(&self.ty(value))
        {
            return None;
        }
        let atoms = self.value_source_atoms(value);
        if atoms.is_empty() {
            return None;
        }
        // A `Local` or `Temp` atom names the storage of a view only when the
        // stored value or the destination is view-like. An owned local moved
        // into an owning slot is a legal transfer, not a lend escape; the
        // ordinary path applies the same gate before its view rule.
        let view_context = is_view_like(target_ty) || is_view_like(&self.ty(value));
        let mut local = None;
        for atom in &atoms {
            match atom {
                Provenance::Param(owner) => {
                    if *owner == destination || self.bindings[*owner].mode == Mode::Take {
                        continue;
                    }
                    return Some(SourceSetRoot::Parameter(*owner));
                }
                Provenance::Local(owner) => {
                    if !view_context || *owner == destination {
                        continue;
                    }
                    if local.is_none() {
                        local = Some(SourceSetRoot::Local(*owner));
                    }
                }
                Provenance::Temp(_) => {
                    if !view_context {
                        continue;
                    }
                    if local.is_none() {
                        local = Some(SourceSetRoot::Temporary);
                    }
                }
                Provenance::None
                | Provenance::Static
                | Provenance::Global(_)
                | Provenance::Composite(_) => {}
            }
        }
        local
    }

    /// local-, fresh-, or `take`-derived view reports `E10033`. A writable
    /// parameter slot holds the caller's frame: module-level and the caller's
    /// parameter storage survive it, while a view of this frame's storage — or
    /// of a temporary — reports the loan-outlives `E10030`.
    ///
    /// A caller-retained container field or element is stricter: a view of a
    /// different lend parameter escapes the lend's call scope even though the
    /// caller's frame outlives the callee, so it reports `E10033`. A duplicable
    /// struct root is copied at the call and keeps the copy-local acceptance; a
    /// fixed array or slice still reaches the caller's storage and is not a
    /// copy. A whole `borrow T?` ownership slot is the separate out-slot
    /// exemption.
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
        root_id: Option<usize>,
        whole_out_slot: bool,
    ) {
        // A whole `borrow T?` slot owns only its incoming value. A value or
        // view of a different lend is call-scoped, so the caller can reassign
        // its backing after the call and leave the slot dangling; report the
        // source parameter once, before the view-return withholding can hide
        // the store. An owned non-nullable lend into an owning slot is left to
        // the lend-move duty, which already reports that move.
        if whole_out_slot {
            if let (Some(destination), false) = (root_id, global) {
                if let Some(owner) = self.foreign_lend_store_root(value, destination) {
                    let value_ty = self.ty(value);
                    // A writable nullable owned lend is a whole-slot move; the
                    // lenient lend-move exemption for a slot destination does
                    // not apply when the store keeps it in the caller's slot.
                    let writable_owned =
                        value_ty.nullable && self.bindings[owner].mode == Mode::Borrow;
                    if is_whole_slot_view(target_ty)
                        || is_whole_slot_view(&value_ty)
                        || writable_owned
                    {
                        let name = self.bindings[owner].name.clone();
                        self.error(
                            "E10034",
                            format!("cannot release or store borrowed parameter '{name}'"),
                            super::LEND_STORE_HELP,
                            span,
                        );
                        return;
                    }
                }
            }
        }
        // A local rebound on one branch keeps a joined source set. The ordinary
        // whole-slot rule only sees the binding's declaration provenance, so a
        // foreign parameter or a frame-backed local in the set would otherwise
        // be lost. The set is consulted only for a concrete destination and
        // value; a type-parameter pair stays with the per-instance duty.
        if whole_out_slot {
            if let (Some(destination), false) = (root_id, global) {
                if let Some(root) =
                    self.rebound_source_store_root(value, destination, target_ty)
                {
                    match root {
                        SourceSetRoot::Parameter(owner) => {
                            let name = self.bindings[owner].name.clone();
                            self.error(
                                "E10034",
                                format!("cannot release or store borrowed parameter '{name}'"),
                                super::LEND_STORE_HELP,
                                span,
                            );
                        }
                        SourceSetRoot::Local(owner) => {
                            let subject = super::liveness::owner_subject(
                                &self.bindings[owner].name,
                                self.bindings[owner].generated,
                            );
                            self.error(
                                "E10030",
                                format!("borrow of {subject} does not live long enough"),
                                "finish the use before the owner's scope ends",
                                span,
                            );
                        }
                        SourceSetRoot::Temporary => {
                            self.error(
                                "E10030",
                                "borrow of a statement temporary does not live long enough",
                                "finish the use before the owner's scope ends",
                                span,
                            );
                        }
                    }
                    return;
                }
            }
        }
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
        // A caller-retained container field or element cannot keep a view of a
        // different lend parameter. The lend is call-scoped (BR-5), so the
        // store is the field-escape rule even though the caller's frame
        // outlives the callee. A duplicable container root is copied at the
        // call, so only the callee's copy is filled; a whole `borrow T?`
        // out-slot assignment is the ownership-slot exemption.
        if let (false, false, Some(root_id)) = (global, whole_out_slot, root_id) {
            let root_ty = self.bindings[root_id].ty.clone();
            let root_is_concrete = !crate::types::contains_param(&root_ty);
            if root_is_concrete && !self.sema.table.copies_whole_value(&root_ty) {
                let foreign_parameter = self
                    .dynamic_parameter_roots(value)
                    .into_iter()
                    .any(|owner| owner != root_id);
                if foreign_parameter {
                    self.error(
                        "E10033",
                        format!("borrow of '{name}' escapes its scope"),
                        "derive the view from a parameter or take ownership",
                        span,
                    );
                    return;
                }
            }
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
            Provenance::Param(id) => !global && !self.binding_dies_with_frame(*id),
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
    /// True when the binding is a whole `borrow T?` ownership slot.
    ///
    /// A capture carries the original kind because its lifted-body binding is
    /// a plain parameter; the ownership-slot exemption follows the captured
    /// destination too.
    fn is_out_slot_binding(&self, id: usize) -> bool {
        self.bindings[id].kind == BindingKind::OutSlot
            || self.bindings[id].capture_whole_out_slot
    }

    fn is_caller_slot(&self, id: usize) -> bool {
        if matches!(
            self.bindings[id].base,
            Some(ir::PlaceBase::EnvField { .. })
        ) {
            // A capture aliases the enclosing binding: a captured out-slot or
            // writable parameter is the caller's destination, while a captured
            // local stays in the enclosing frame.
            return self.bindings[id].capture_caller_slot;
        }
        self.bindings[id].kind == BindingKind::OutSlot
            || (self.bindings[id].kind == BindingKind::Param
                && self.bindings[id].mode == Mode::Borrow)
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

    /// True when a carrier copy hands frame-backed views to a global.
    ///
    /// A conditional contributes every arm, an `unsafe` or cast wrapper its
    /// inner value, a local carrier its binding loans, a literal its view
    /// positions, and a call its single source's storage. A parameter- or
    /// module-backed arm contributes nothing, so the all-safe control keeps
    /// its acceptance.
    fn carrier_copy_escapes(&self, value: &Expr) -> bool {
        match &value.kind {
            ExprKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.carrier_copy_escapes(then_value) || self.carrier_copy_escapes(else_value)
            }
            ExprKind::Unsafe(inner) => self.carrier_copy_escapes(inner),
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => {
                self.carrier_copy_escapes(value)
            }
            // A place-shaped arm may project the carrier through a member or
            // an element (`hs[0]`, `s.holder`); the projected place reaches
            // the same binding loans the whole name does.
            ExprKind::Name(_) | ExprKind::Qualified(_) => {
                match self.place_target(value) {
                    Some(PlaceTarget::Local(id, _)) => {
                        let id = self.alias_resolved_root(id);
                        !self.binding_loan_owners(id).is_empty()
                    }
                    _ => false,
                }
            }
            ExprKind::Member { base, .. } | ExprKind::Index { base, .. } => {
                self.carrier_copy_escapes(base)
            }
            // A literal's own field judgment already reports a frame-backed
            // view at the field site; adding the destination's report would
            // repeat one store's rejection. Only a place arm, a call arm, or a
            // nested conditional contributes the destination report.
            ExprKind::StructLiteral { .. } | ExprKind::ArrayLiteral { .. } => false,
            ExprKind::Call { callee, args, .. } => {
                let sources = self.call_carrier_sources(callee, args);
                if !sources.is_empty() {
                    // A global outlives every caller, so a caller-backed
                    // parameter escapes it exactly as this frame's local does;
                    // only static and module storage survive. Every source the
                    // carrier may read is judged.
                    return sources.iter().any(|source| match source {
                        super::place::ViewCallSource::Expr(source) => !matches!(
                            self.view_provenance(source),
                            Provenance::Static | Provenance::Global(_)
                        ),
                        super::place::ViewCallSource::Receiver(provenance)
                        | super::place::ViewCallSource::Capture(provenance) => !matches!(
                            provenance,
                            Provenance::Static | Provenance::Global(_)
                        ),
                    });
                }
                self.frame_owned_view_owner(value).is_some()
                    || self.names_fresh_view_storage(value, &self.ty(value))
            }
            _ => false,
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
                let sources = self.call_view_sources(callee, args);
                if sources.is_empty() {
                    true
                } else {
                    sources.iter().any(|source| match source {
                        super::place::ViewCallSource::Expr(source) => {
                            matches!(self.view_provenance(source), Provenance::None)
                                || self.names_fresh_view_storage(source, &self.ty(source))
                        }
                        super::place::ViewCallSource::Receiver(provenance)
                        | super::place::ViewCallSource::Capture(provenance) => {
                            matches!(provenance, Provenance::None)
                        }
                    })
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
                super::LEND_STORE_HELP,
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

    /// Pins an intermediate member over a raw dereference by its address.
    ///
    /// A chain such as `(*p).next.x` names the field through an intermediate
    /// member. The C store would re-evaluate that member after a side-effecting
    /// right-hand side, so the address of `(*p).next` is bound first and the
    /// store writes through the bound pointer. Only a member or element base is
    /// pinned; a direct `(*p).x` already evaluates one dereference.
    fn pin_intermediate_raw_member(&mut self, value: Value) -> Value {
        let ValueKind::Member { base, name } = value.kind else {
            return value;
        };
        if !matches!(base.kind, ValueKind::Member { .. } | ValueKind::Index { .. })
            || !Self::roots_at_raw_deref(&base)
        {
            return Value::new(value.ty, ValueKind::Member { base, name });
        }
        let base_ty = base.ty.clone();
        let address_ty = Ty::raw(base_ty.clone());
        let temp = self.new_temp(address_ty.clone());
        let target = Place {
            base: ir::PlaceBase::Temp(temp),
            projections: Vec::new(),
            ty: address_ty.clone(),
        };
        let address = Value::new(
            address_ty,
            ValueKind::Unary {
                op: UnaryOp::AddressOf,
                operand: base.clone(),
            },
        );
        self.emit(Statement::Assign {
            target,
            value: address,
        });
        let pinned = Value::new(
            base_ty.clone(),
            ValueKind::Unary {
                op: UnaryOp::Deref,
                operand: Box::new(Value::new(
                    Ty::raw(base_ty),
                    ValueKind::Temp(temp),
                )),
            },
        );
        Value::new(
            value.ty,
            ValueKind::Member {
                base: Box::new(pinned),
                name,
            },
        )
    }

    /// True when a value's projection chain roots at a raw dereference.
    fn roots_at_raw_deref(value: &Value) -> bool {
        match &value.kind {
            ValueKind::Unary {
                op: UnaryOp::Deref,
                ..
            } => true,
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                Self::roots_at_raw_deref(base)
            }
            _ => false,
        }
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
        skip_transfer: bool,
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
            if skip_transfer {
                return None;
            }
            return self.emit_assigned(place, &element_ty, value, compound);
        }
        // The target's location evaluates before the right-hand side, so its
        // base and index are always pinned: a call in the right-hand side may
        // change the index, and the store must reach the element the target
        // named first.
        let Some((element, pointer, element_ty)) =
            self.indexed_target(target, compound.is_some(), true)
        else {
            return None;
        };
        if skip_transfer {
            // The no-op transfer still evaluates the target's base and index
            // and runs the bounds guard.
            self.emit(Statement::Guards {
                checks: vec![pointer],
            });
            return None;
        }
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
                // An owning raw-deref read moves its pointee: the pointer is
                // pinned once, so the store and the emptying name the same
                // address instead of evaluating an impure pointer twice.
                let lowered = self.materialize_owning_deref(lowered);
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
                } else if (element_ty.is_unknown()
                    || crate::types::contains_param(&element_ty))
                    && !self.ty(value).is_fixed_array()
                    && self.projects_from_place(&lowered)
                {
                    // The instance decides whether this element owns or
                    // borrows. A place source stays a place: an owned
                    // instance moves it in the emitter, while a view
                    // instance widens it in place. Consuming it here would
                    // move the caller's storage into a transfer temporary
                    // that the view instance never releases.
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
                ValueKind::Materialize {
                    destination: None,
                    temp: Some(temp),
                    ..
                } => ir::Projection::ElementTemp(*temp),
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
/// True when two store expressions name the same place.
///
/// The comparison is structural and conservative: identifiers, dotted names,
/// literals, members, elements, and unary operators match when their parts
/// match. A call or another side-effecting form never matches, so the runtime
/// address guard still sees it. Spans are ignored.
fn same_place_expr(left: &Expr, right: &Expr) -> bool {
    let left = unwrap_unsafe_expr(left);
    let right = unwrap_unsafe_expr(right);
    match (&left.kind, &right.kind) {
        (ExprKind::Int(left), ExprKind::Int(right)) => left == right,
        (ExprKind::Float(left), ExprKind::Float(right)) => left == right,
        (ExprKind::Char(left), ExprKind::Char(right)) => left == right,
        (ExprKind::Str(left), ExprKind::Str(right)) => left == right,
        (ExprKind::Bool(left), ExprKind::Bool(right)) => left == right,
        (ExprKind::Null, ExprKind::Null) => true,
        (ExprKind::Name(left), ExprKind::Name(right)) => left == right,
        (ExprKind::Qualified(left), ExprKind::Qualified(right)) => {
            left.segments == right.segments
        }
        (
            ExprKind::Member {
                base: left_base,
                name: left_name,
            },
            ExprKind::Member {
                base: right_base,
                name: right_name,
            },
        ) => left_name == right_name && same_place_expr(left_base, right_base),
        (
            ExprKind::Index {
                base: left_base,
                index: left_index,
            },
            ExprKind::Index {
                base: right_base,
                index: right_index,
            },
        ) => {
            same_place_expr(left_base, right_base)
                && same_place_expr(left_index, right_index)
        }
        (
            ExprKind::Unary {
                op: left_op,
                operand: left_operand,
            },
            ExprKind::Unary {
                op: right_op,
                operand: right_operand,
            },
        ) => {
            !left_op.is_update()
                && left_op == right_op
                && same_place_expr(left_operand, right_operand)
        }
        _ => false,
    }
}

/// Strips the transparent `unsafe` prefix from a store expression.
fn unwrap_unsafe_expr(mut expr: &Expr) -> &Expr {
    while let ExprKind::Unsafe(inner) = &expr.kind {
        expr = inner;
    }
    expr
}

/// Strips the transparent structural wrappers from an expression.
///
/// `unsafe` and the raw/scalar casts keep the operand's storage, so a
/// conditional behind one is the same conditional for every source and move
/// judgment. The walk returns the innermost unwrapped expression.
pub(super) fn structural_wrapper_inner(mut expr: &Expr) -> &Expr {
    loop {
        match &expr.kind {
            ExprKind::Unsafe(inner) => expr = inner,
            ExprKind::RawCast { value, .. } | ExprKind::Cast { value, .. } => expr = value,
            _ => return expr,
        }
    }
}

/// One member or element selection over a raw-deref store target.
enum RawStoreProjection<'a> {
    /// A named struct field and the member expression it selects.
    Field {
        /// The field name.
        name: &'a str,
        /// The member expression, for its checked type.
        whole: &'a Expr,
    },
    /// An array or slice element with the written index expression.
    Index {
        /// The written index expression.
        index: &'a Expr,
        /// The element expression, for its checked type.
        whole: &'a Expr,
    },
}

/// The dereference and projections of a `Deref`-rooted store target.
///
/// `(*p).f`, `(*p).f.x`, and `(*p).arr[i].x` are places: the dereference names
/// the storage and each member or element selects part of it. The walker
/// returns the dereference expression and the projections from the root
/// outward, or `None` when the target does not root at a raw dereference. A
/// bare `*p` returns an empty projection list. An `unsafe` wrapper is
/// transparent, exactly as it is for the bare target.
fn raw_deref_store_target(target: &Expr) -> Option<(&Expr, Vec<RawStoreProjection<'_>>)> {
    fn walk<'a>(expr: &'a Expr, out: &mut Vec<RawStoreProjection<'a>>) -> Option<&'a Expr> {
        match &expr.kind {
            ExprKind::Unsafe(inner) => walk(inner, out),
            ExprKind::Member { base, name } => {
                let deref = walk(base, out)?;
                out.push(RawStoreProjection::Field {
                    name: name.as_str(),
                    whole: expr,
                });
                Some(deref)
            }
            ExprKind::Index { base, index } => {
                let deref = walk(base, out)?;
                out.push(RawStoreProjection::Index {
                    index,
                    whole: expr,
                });
                Some(deref)
            }
            ExprKind::Unary {
                op: UnaryOp::Deref,
                ..
            } => Some(expr),
            _ => None,
        }
    }
    let mut projections = Vec::new();
    let deref = walk(target, &mut projections)?;
    Some((deref, projections))
}


/// Appends one span to an opaque-store watch list, once.
fn push_opaque_span(list: &mut Vec<Span>, span: Span) {
    if !list.iter().any(|known| known.start == span.start && known.end == span.end) {
        list.push(span);
    }
}

/// Appends one span and its subject to an opaque-store watch list, once.
fn push_opaque_use(list: &mut Vec<(Span, String)>, span: Span, subject: &str) {
    if !list
        .iter()
        .any(|(known, _)| known.start == span.start && known.end == span.end)
    {
        list.push((span, subject.to_owned()));
    }
}

/// True when one projection path definitely covers another.
///
/// A store at a path replaces every position below it, so a prefix covers the
/// longer path. A run-time element step covers every element; a constant step
/// covers only the same constant, because writing one element leaves its
/// siblings' views in place.
pub(super) fn projection_path_covers(prefix: &[ir::Projection], path: &[ir::Projection]) -> bool {
    prefix.len() <= path.len()
        && prefix
            .iter()
            .zip(path)
            .all(|(left, right)| projection_step_covers(left, right))
}

/// True when one projection step covers another position.
///
/// Only a field with the same name or the same constant element covers: a
/// run-time element store *may* write any element, so it cannot replace a
/// constant element's view, and two run-time stores cannot prove they reach the
/// same position.
fn projection_step_covers(left: &ir::Projection, right: &ir::Projection) -> bool {
    match (left, right) {
        (ir::Projection::Field(a), ir::Projection::Field(b)) => a == b,
        (ir::Projection::Element(Some(a)), ir::Projection::Element(Some(b))) => a == b,
        _ => false,
    }
}

/// True when two projection paths select an overlapping position.
pub(super) fn projection_paths_overlap(left: &[ir::Projection], right: &[ir::Projection]) -> bool {
    left.is_empty()
        || right.is_empty()
        || left
            .iter()
            .zip(right)
            .all(|(a, b)| projection_step_overlaps(a, b))
}

/// True when one projection step may reach the same position as another.
fn projection_step_overlaps(left: &ir::Projection, right: &ir::Projection) -> bool {
    left == right
        || (matches!(left, ir::Projection::Element(_))
            && matches!(right, ir::Projection::Element(_))
            && (matches!(left, ir::Projection::Element(None))
                || matches!(right, ir::Projection::Element(None))))
}

/// The first storage root a rebound local's joined source set names.
///
/// A parameter root takes precedence over a local or temporary root, so a
/// mixed set keeps the foreign-parameter report and never emits a second
/// diagnostic for the same store.
enum SourceSetRoot {
    /// A `borrow` or by-value parameter the caller cannot keep.
    Parameter(usize),
    /// A frame-backed local view.
    Local(usize),
    /// A statement temporary that dies with the statement.
    Temporary,
}

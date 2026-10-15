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
        if self.maybe_owns(&ty) {
            // The field store writes a move for an owning instance, so a later
            // use of the source deposits the duplicability duty.
            if let Some(target) = self.place_target(expr) {
                self.mark_moved_if_owning(target, span);
            }
            return;
        }
        if self.sema.table.duplicable(&ty) {
            return;
        }
        if let Some(target) = self.place_target(expr) {
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
                    self.eval(&field.value);
                    self.consume_owned_place(&field.value, field.value.span);
                    // A view-like field value stored in a struct field is
                    // storage in a field: the loan would escape its origin,
                    // so it reports here instead of dangling once the
                    // backing's scope ends.
                    if self.is_loan_value(&field.value) {
                        self.error(
                            "E10033",
                            format!("borrow of '{}' escapes its scope", field.name),
                            "derive the view from a parameter or take ownership",
                            field.value.span,
                        );
                    }
                    // An owned conditional field is an owning destination: the
                    // chosen arm moves into the field, so every owned place arm
                    // counts as maybe-moved afterwards; a maybe-owning field
                    // records the duplicability duty instead.
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
                self.check_call_expr(callee, type_args, args, expr.span);
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
            _ => Ty::unknown(),
        }
    }

    /// Reports `E10600` when a member is reached through a raw pointer.
    pub(super) fn check_member_raw(&mut self, base_ty: &Ty, span: Span) {
        if matches!(base_ty.kind, TyKind::Raw(_)) && self.unsafe_depth == 0 {
            self.report_unsafe(UnsafeOp::Deref, span);
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
        if value_ty.nullable && concrete {
            self.check_nonnull_expr(expr, span);
        }
    }

    /// Reports the null rule at a position that requires a concrete value.
    fn report_null_rule(&mut self, span: Span) {
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
                self.check_lend_target_store(id, &projections, value, target.span);
                // A projected target consumes a non-duplicable element value:
                // a place-rooted element is a partial move and reports E10026
                // through the shared consume point, while an ownerless element
                // is reported by the store preparation. A conditional moves
                // whichever owned arm it selects, so every arm becomes
                // maybe-moved except an arm that names the target path itself.
                if !projections.is_empty() {
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
                if !projections.is_empty() && self.is_loan_value(value) {
                    let name = self.bindings[id].name.clone();
                    self.error(
                        "E10033",
                        format!("borrow of '{name}' escapes its scope"),
                        "derive the view from a parameter or take ownership",
                        target.span,
                    );
                }
                self.write_place(id, &projections, target.span);
                let lowered = if has_runtime_element(&projections) {
                    self.emit_indexed_store(target, value, compound);
                    None
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
                if self.is_loan_value(value) {
                    self.error(
                        "E10033",
                        format!("borrow of '{}' escapes its scope", id.name),
                        "derive the view from a parameter or take ownership",
                        target.span,
                    );
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
                    self.emit_compound_store(binary, place, &ty, one);
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
                    self.emit_compound_store(binary, place, &ty, one);
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
                let right = self.lower_value(value);
                self.emit_compound_store(binary, place, ty, right.clone());
                Some(right)
            }
            None => self.emit_store(place, ty, value),
        }
    }

    /// Emits a read-modify-write `place = place op right`.
    fn emit_compound_store(&mut self, op: BinaryOp, place: Place, ty: &Ty, right: Value) {
        let left = Value::place(place.clone());
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

    /// Lowers an indexed target to its element read and its address.
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
    /// in its single guarded position.
    fn indexed_target(&mut self, target: &'a Expr, shared: bool) -> Option<(Value, Value, Ty)> {
        let target = match &target.kind {
            ExprKind::Unsafe(inner) => inner,
            _ => target,
        };
        let ExprKind::Index { base, index } = &target.kind else {
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
    fn emit_indexed_store(&mut self, target: &'a Expr, value: &'a Expr, compound: Option<BinaryOp>) {
        let Some((element, pointer, element_ty)) = self.indexed_target(target, compound.is_some())
        else {
            return;
        };
        let stored = match compound {
            Some(binary) => {
                let right = self.lower_value(value);
                self.combine(binary, element, right, &element_ty)
            }
            // An owned value stored into an element is a move, exactly as it is
            // for a named destination: the checker consumed the source, so the
            // lowering empties it and the element keeps the value. Without this
            // both the source and the element release the same storage.
            None => {
                let lowered = self.lower_value(value);
                self.consume_place(lowered)
            }
        };
        self.emit(Statement::StorePointer {
            pointer,
            value: stored,
        });
    }

    /// Lowers an indexed increment or decrement as a read-modify-write.
    fn emit_indexed_update(&mut self, target: &'a Expr, binary: BinaryOp, one: Value) {
        let Some((element, pointer, element_ty)) = self.indexed_target(target, true) else {
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
fn has_runtime_element(projections: &[ir::Projection]) -> bool {
    projections.iter().any(|projection| {
        matches!(
            projection,
            ir::Projection::Element(None) | ir::Projection::ElementTemp(_)
        )
    })
}

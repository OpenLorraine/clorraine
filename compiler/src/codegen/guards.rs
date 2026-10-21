//! Runtime-guard emission for lowered values.
//!
//! A guard is a division check, a bounds check, or a float-to-integer range
//! check. Most guards are emitted as statements before the guarded statement,
//! but the right operand of `&&`/`||` and the arms of `?:` are evaluated only
//! sometimes, so their guards embed inside the expression instead. This module
//! owns both spellings and the conditions they share.

use crate::ast::{BinaryOp, ScalarType};
use crate::ir::{Value, ValueKind, ValuePiece};
use crate::types::{is_signed_scalar, Ty, TyKind};

use super::function::{arm_has_storage_root, value_moves_pointee, ArrayTarget};
use super::render::{bounds_condition, signed_min};
use super::types::scalar_of;
use super::Emitter;

impl<'a> Emitter<'a> {

    /// Emits every runtime guard a value's evaluation needs.
    ///
    /// The right operand of `&&`/`||` and the arms of `?:` are evaluated only
    /// sometimes, so their guards are embedded by [`Self::value`] instead of
    /// hoisted here; hoisting would check an operand the program never
    /// evaluates and trap a correct program.
    pub(super) fn emit_guards(&mut self, value: &Value) {
        match &value.kind {
            ValueKind::Binary {
                op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr,
                left,
                right,
            } => {
                self.emit_guards(left);
                self.emit_conditional_drop_hoists(right);
            }
            ValueKind::Binary { op, left, right } => {
                self.emit_guards(left);
                self.emit_guards(right);
                if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                    self.emit_division_guard(left, right);
                }
            }
            ValueKind::Index { base, index } => {
                self.emit_guards(base);
                self.emit_guards(index);
                self.emit_bounds_guard(base, index);
            }
            ValueKind::Cast { value: inner, .. } => {
                self.emit_guards(inner);
                let target = self.substitute(&value.ty);
                let source = self.substitute(&inner.ty);
                if target.is_integer() && source.is_numeric() && !source.is_integer() {
                    self.emit_float_guard(inner, &target);
                }
            }
            ValueKind::Call {
                args, receiver, ..
            } => {
                if let Some(receiver) = receiver {
                    self.emit_guards(receiver);
                }
                for arg in args {
                    self.emit_guards(arg);
                }
            }
            ValueKind::Unary { operand, .. } => self.emit_guards(operand),
            // Only the condition always runs; the arms' guards are embedded.
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.emit_guards(condition);
                self.emit_conditional_drop_hoists(then_value);
                self.emit_conditional_drop_hoists(else_value);
            }
            ValueKind::Member { base, .. } => self.emit_guards(base),
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.emit_guards(field);
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.emit_guards(element);
                }
                if let Some(fill) = fill {
                    self.emit_guards(fill);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.emit_guards(inner);
                    }
                }
            }
            ValueKind::Intrinsic { name, args, .. } => {
                for arg in args {
                    self.emit_guards(arg);
                }
                if name == "slice" {
                    self.emit_slice_guard(args);
                }
            }
            // The carried effects run on this path and yield the constant.
            // The expression is a comma sequence, so it remains a statement
            // here and an inline guard under a short-circuit or `?:` arm.
            ValueKind::DiscardThen { effects, .. } => {
                self.emit_discard_effects(effects);
            }
            // A materialized operand is always evaluated where it stands, so
            // it is filled by a statement before the guarded construct. A
            // fixed-array call writes through its hidden out-parameter, so the
            // temporary is the call's destination instead of an assignment
            // target.
            ValueKind::Materialize {
                temp,
                destination,
                value: inner,
                move_outs,
                value_move_outs,
                adopting,
            } => {
                // An adopting carrier's consumer owns the value, so the
                // previous value must not be released on a re-fill: the
                // destination's own cleanup is the sole owner.
                // The inner value's type can still be opaque in a generic
                // body — an unresolved conditional take's selected member,
                // for example — so the fill resolves it through the value the
                // instance substitutes, exactly as the declaration does.
                let inner_ty = self.resolved_value_ty(inner);
                let destination_c = self.materialize_destination(*temp, destination);
                if !adopting {
                    self.emit_reassigned_materialize_drop(
                        *temp,
                        destination.as_ref(),
                        &inner_ty,
                    );
                }
                // A deferred `len` effect needs the statement-form release for
                // a fresh growable container, which has no C expression form.
                // Fill the temporary on the evaluated path, and skip the
                // ordinary value fill that would drop the effects.
                if self.needs_statement_discard(inner)
                    && !inner_ty.is_fixed_array()
                    && !matches!(inner.kind, ValueKind::Conditional { .. })
                {
                    self.emit_statement_discard_fill(&destination_c, inner);
                    return;
                }
                self.emit_guards(inner);
                if let ValueKind::Conditional {
                    condition,
                    then_value,
                    else_value,
                } = &inner.kind
                {
                    // A deferred conditional operand fills the temporary
                    // through the owned branch store, so the chosen arm moves
                    // and the skipped arm is never evaluated. A fixed-array
                    // conditional has no C value expression — a call arm
                    // cannot pass its hidden out-parameter through `?:` — so
                    // each arm fills the temporary on its own path instead.
                    let ty = inner_ty.clone();
                    if ty.is_fixed_array() {
                        let destination = ArrayTarget::object(&destination_c);
                        self.emit_fixed_array_conditional_assign(
                            &destination,
                            &ty,
                            condition,
                            then_value,
                            else_value,
                            false,
                        );
                    } else if self.needs_drop(&ty) {
                        self.emit_conditional_move_assign(
                            &destination_c,
                            &ty,
                            condition,
                            then_value,
                            else_value,
                        );
                    } else {
                        self.push(&format!("    {destination_c} = {};", self.value(inner)));
                    }
                } else if let ValueKind::ArrayLit { elements, fill } = &inner.kind {
                    if inner_ty.is_fixed_array() {
                        // A brace list has no C place, so a deferred array
                        // literal is stored element by element into the
                        // temporary, on the path that evaluates the operand.
                        self.emit_array_assign_to(
                            &super::function::ArrayTarget::object(&destination_c),
                            &inner_ty,
                            elements,
                            fill,
                        );
                    } else {
                        self.push(&format!("    {destination_c} = {};", self.value(inner)));
                    }
                } else {
                    let destination = destination_c.clone();
                    // A literal whose fixed-array or view-array field has no C
                    // initializer form stores its fields through the same core
                    // the direct spelling uses, on the path that evaluates the
                    // operand: the call-filled field passes the destination and
                    // a view element reads the root the deferred fill bound.
                    let literal_stored = match &inner.kind {
                        ValueKind::StructLit { fields, .. }
                            if self.struct_literal_needs_guard_stores(&inner_ty, fields) =>
                        {
                            self.emit_struct_literal_fields(&destination, &inner_ty, fields);
                            true
                        }
                        _ => false,
                    };
                    if !literal_stored {
                        match &inner.kind {
                        ValueKind::Call {
                            callee,
                            type_args,
                            args,
                            receiver,
                        } if inner_ty.is_fixed_array() => {
                            let call = self.call_with_destination(
                                callee,
                                type_args,
                                args,
                                receiver.as_deref(),
                                &destination,
                            );
                            self.push(&format!("    {call};"));
                        }
                        // A fixed-array closure call also writes through a
                        // hidden destination, which the lifted function takes
                        // after its environment and arguments.
                        ValueKind::ClosureCall { callable, args }
                            if inner_ty.is_fixed_array() =>
                        {
                            let call = self.closure_call_with_destination(
                                callable,
                                args,
                                Some(&destination),
                            );
                            self.push(&format!("    {call};"));
                        }
                        // A fixed-array object has no C assignment form: the
                        // fill copies the row's bytes from the object its
                        // guards already evaluated, and the carried move-outs
                        // empty the source. The store core is not used here
                        // because it would emit the inner guards a second
                        // time, running a side-effecting source twice.
                        _ if inner_ty.is_fixed_array() => {
                            let target = ArrayTarget::object(&destination);
                            let value_c = self.value(inner);
                            let source = self.array_source_address(inner, &value_c);
                            self.push(&format!(
                                "    memcpy({}, {source}, {});",
                                target.address, target.bytes
                            ));
                        }
                        _ => self.push(&format!("    {destination} = {};", self.value(inner))),
                        }
                    }
                }
                // The call's `take` arguments and a consume carrier's source
                // slot are emptied only now, on the evaluated path, so a
                // skipped guard leaves them owned. A duplicable instance
                // copies, so its source keeps its value and is not zeroed;
                // only a type that owns storage moves, and a place recorded
                // opaquely resolves through its own base and projections.
                for place in move_outs {
                    let ty = self.resolved_place_ty(place);
                    if !self.needs_drop(&ty) {
                        continue;
                    }
                    let source = self.place(place);
                    self.emit_move_zero_place(place, &source, &ty);
                }
                // A consumed value that names no place — a raw-deref take
                // argument — is emptied through its own address only now, on
                // the evaluated path, so a skipped guard leaves the pointee
                // owned. A duplicable pointee copies and the emptying is a
                // no-op.
                for value in value_move_outs {
                    let ty = self.resolved_value_ty(value);
                    if !self.needs_drop(&ty) {
                        continue;
                    }
                    self.emit_take_alias(value, &ty);
                    self.emit_arm_source_moves(value, &ty);
                }
            }
            ValueKind::Closure { captures, .. } => {
                for capture in captures {
                    self.emit_guards(capture);
                }
            }
            ValueKind::ClosureCall { callable, args } => {
                self.emit_guards(callable);
                for arg in args {
                    self.emit_guards(arg);
                }
            }
            _ => {}
        }
    }

    /// Hoists the statement-form drop of a conditionally evaluated operand.
    ///
    /// A materialized operand inside a short-circuit right side or a `?:` arm
    /// stores its side effect as an expression, and a type whose drop has no
    /// expression form would skip the previous value on every re-store. The
    /// statement form runs before the construct: the temporary is
    /// zero-initialized, so the first evaluation drops nothing, and the
    /// post-construct release stays null-safe, so each stored value is released
    /// exactly once. A type whose drop already has an expression form keeps it
    /// and is not hoisted, an adopting carrier is skipped because its
    /// consumer's cleanup is the sole release, and the walk descends the whole
    /// subtree because nested guards embed at their own evaluation sites.
    fn emit_conditional_drop_hoists(&mut self, value: &Value) {
        match &value.kind {
            ValueKind::Materialize {
                temp,
                destination,
                value: inner,
                adopting,
                ..
            } => {
                let inner_ty = self.substitute(&inner.ty);
                let destination_c = self.materialize_destination(*temp, destination);
                if !*adopting
                    && self.needs_drop(&inner_ty)
                    && self.drop_expr_text(&destination_c, &inner_ty).is_none()
                {
                    // A guard temporary is re-filled on each evaluation, so
                    // its previous value must be released before the fill
                    // overwrites it. An adopting carrier's consumer owns the
                    // value, so the consumer's own cleanup is the sole owner:
                    // releasing the carrier here would free the consumer's
                    // storage again when the guarded statement re-executes.
                    self.emit_reassigned_materialize_drop(
                        *temp,
                        destination.as_ref(),
                        &inner.ty,
                    );
                }
                self.emit_conditional_drop_hoists(inner);
            }
            ValueKind::Binary { left, right, .. } => {
                self.emit_conditional_drop_hoists(left);
                self.emit_conditional_drop_hoists(right);
            }
            ValueKind::Unary { operand, .. } => self.emit_conditional_drop_hoists(operand),
            ValueKind::Cast { value, .. } => self.emit_conditional_drop_hoists(value),
            ValueKind::Index { base, index } => {
                self.emit_conditional_drop_hoists(base);
                self.emit_conditional_drop_hoists(index);
            }
            ValueKind::Member { base, .. } => self.emit_conditional_drop_hoists(base),
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.emit_conditional_drop_hoists(condition);
                self.emit_conditional_drop_hoists(then_value);
                self.emit_conditional_drop_hoists(else_value);
            }
            ValueKind::Call { args, receiver, .. } => {
                if let Some(receiver) = receiver {
                    self.emit_conditional_drop_hoists(receiver);
                }
                for arg in args {
                    self.emit_conditional_drop_hoists(arg);
                }
            }
            ValueKind::ClosureCall { callable, args } => {
                self.emit_conditional_drop_hoists(callable);
                for arg in args {
                    self.emit_conditional_drop_hoists(arg);
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    self.emit_conditional_drop_hoists(arg);
                }
            }
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.emit_conditional_drop_hoists(field);
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.emit_conditional_drop_hoists(element);
                }
                if let Some(fill) = fill {
                    self.emit_conditional_drop_hoists(fill);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.emit_conditional_drop_hoists(inner);
                    }
                }
            }
            ValueKind::Closure { captures, .. } => {
                for capture in captures {
                    self.emit_conditional_drop_hoists(capture);
                }
            }
            _ => {}
        }
    }

    /// Releases a temporary's previous value before it is re-materialized.
    ///
    /// A guarded operand inside a loop reuses one temporary, so the previous
    /// iteration's value must be released before the new one is stored; the
    /// drop is null-safe, so the first evaluation releases nothing.
    fn emit_reassigned_temp_drop(&mut self, temp: crate::ir::TempId, ty: &Ty) {
        let ty = self.substitute(ty);
        if !self.needs_drop(&ty) {
            return;
        }
        let place = crate::ir::Place {
            base: crate::ir::PlaceBase::Temp(temp),
            projections: Vec::new(),
            ty,
        };
        self.emit_drop(&place);
    }

    /// Releases a materialization's previous value before it is re-filled.
    ///
    /// A named destination's binding outlives the statement, so a re-fill must
    /// release the previous value exactly as the temporary form does. The drop
    /// is null-safe, so the first fill releases nothing.
    fn emit_reassigned_materialize_drop(
        &mut self,
        temp: Option<crate::ir::TempId>,
        destination: Option<&crate::ir::Place>,
        ty: &Ty,
    ) {
        let ty = self.substitute(ty);
        if !self.needs_drop(&ty) {
            return;
        }
        match destination {
            Some(place) => {
                let text = self.place(place);
                self.emit_drop_expr(&text, &ty);
            }
            None => {
                let temp = temp.expect("a materialization names a temporary or a destination");
                self.emit_reassigned_temp_drop(temp, &ty);
            }
        }
    }

    /// The C lvalue a materialization fills.
    ///
    /// A named destination is filled through its own place, whose scope owns
    /// the value; every other materialization fills its compiler temporary.
    fn materialize_destination(
        &self,
        temp: Option<crate::ir::TempId>,
        destination: &Option<crate::ir::Place>,
    ) -> String {
        match (temp, destination) {
            (_, Some(place)) => self.place(place),
            (Some(temp), None) => format!("__cl_tmp{temp}"),
            (None, None) => {
                unreachable!("a materialization names a temporary or a destination")
            }
        }
    }

    /// Renders a value whose evaluation is conditional, with its guards
    /// embedded as expressions.
    ///
    /// A short-circuit operand is evaluated only when its controlling test
    /// holds, so its guard cannot be a statement before the enclosing
    /// construct. The guard becomes a comma operand: it traps when the check
    /// fails and evaluates to a harmless value otherwise. A nested `&&`, `||`,
    /// or `?:` keeps its own conditional guards nested inside the same
    /// expression.
    pub(super) fn guarded_value(&self, value: &Value) -> String {
        match &value.kind {
            ValueKind::Binary {
                op: BinaryOp::LogicalAnd,
                left,
                right,
            } => format!(
                "({} ? {} : 0)",
                self.guarded_value(left),
                self.guarded_value(right)
            ),
            ValueKind::Binary {
                op: BinaryOp::LogicalOr,
                left,
                right,
            } => format!(
                "({} ? 1 : {})",
                self.guarded_value(left),
                self.guarded_value(right)
            ),
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let result = self.substitute(&value.ty);
                format!(
                    "({} ? {} : {})",
                    self.guarded_value(condition),
                    self.conditional_arm(then_value, &result),
                    self.conditional_arm(else_value, &result)
                )
            }
            _ => {
                let (rendered, guards) = self.guarded_parts(value);
                if guards.is_empty() {
                    rendered
                } else {
                    format!("(({}), {})", guards.join(", "), rendered)
                }
            }
        }
    }

    /// Fills a temporary with a value whose deferred effects need statements.
    ///
    /// A `&&`/`||` keeps the right side on the left's path; every other shape
    /// emits its ordinary guards (which run a `DiscardThen` effect where it
    /// stands) and then stores the value. The destination is a scalar
    /// temporary, so the chosen value is copied and the effects are not
    /// rendered a second time.
    fn emit_statement_discard_fill(&mut self, destination_c: &str, value: &Value) {
        if let ValueKind::Binary {
            op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr,
            left,
            right,
        } = &value.kind
        {
            let and = matches!(
                value.kind,
                ValueKind::Binary {
                    op: BinaryOp::LogicalAnd,
                    ..
                }
            );
            self.emit_guards(left);
            let left_c = self.value(left);
            self.push(&format!("    if ({left_c}) {{"));
            if and {
                self.emit_guards(right);
                self.push(&format!("        {destination_c} = {};", self.value(right)));
            } else {
                self.push(&format!("        {destination_c} = 1;"));
            }
            self.push("    } else {");
            if and {
                self.push(&format!("        {destination_c} = 0;"));
            } else {
                self.emit_guards(right);
                self.push(&format!("        {destination_c} = {};", self.value(right)));
            }
            self.push("    }");
            return;
        }
        self.emit_guards(value);
        self.push(&format!("    {destination_c} = {};", self.value(value)));
    }

    /// Emits one value's effects as statements on the current path.
    ///
    /// A conditional arm can carry statements inside its branch, so a
    /// materialized fresh value uses the ordinary fill and statement-form
    /// release, including a growable container whose element loop has no C
    /// expression form. A conditional recurses in an `if`/`else`, so only the
    /// selected arm runs. A literal or template evaluates its parts in source
    /// order. A place projection only emits its guards.
    fn emit_discard_effects(&mut self, value: &Value) {
        match &value.kind {
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.emit_guards(condition);
                let condition_c = self.value(condition);
                self.push(&format!("    if ({condition_c}) {{"));
                self.emit_discard_effects(then_value);
                self.push("    } else {");
                self.emit_discard_effects(else_value);
                self.push("    }");
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.emit_discard_effects(element);
                }
                if let Some(fill) = fill {
                    self.emit_discard_effects(fill);
                }
            }
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.emit_discard_effects(field);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.emit_discard_effects(inner);
                    }
                }
            }
            ValueKind::DiscardThen { effects, .. } => self.emit_discard_effects(effects),
            // The ordinary fill releases the previous value with the
            // statement-form drop, so a growable container is safe across a
            // loop test. The final value's scope release is already pending.
            ValueKind::Materialize { .. } => self.emit_guards(value),
            _ => {
                let discarded = self.discard_expression(value);
                if discarded != "0" {
                    self.push(&format!("    {discarded};"));
                }
            }
        }
    }

    /// Renders one value's effects as an inline comma sequence.
    ///
    /// The result is a C expression that evaluates the value, discards its
    /// result, and yields `0`. A conditional keeps each arm behind the test, a
    /// literal or template evaluates its parts in source order, and a
    /// materialized value runs its guarded fill. A place is only read; no move
    /// or release of caller storage happens here.
    fn discard_expression(&self, value: &Value) -> String {
        match &value.kind {
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let mut guards = Vec::new();
                self.guard_expressions(condition, &mut guards);
                let condition_c = self.value(condition);
                let then = self.discard_expression(then_value);
                let else_value = self.discard_expression(else_value);
                let selected = format!("({condition_c} ? (void)({then}) : (void)({else_value}))");
                if guards.is_empty() {
                    selected
                } else {
                    format!("(({}), {selected})", guards.join(", "))
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                let mut parts: Vec<String> = elements
                    .iter()
                    .map(|element| self.discard_expression(element))
                    .filter(|part| part != "0")
                    .collect();
                if let Some(fill) = fill {
                    let part = self.discard_expression(fill);
                    if part != "0" {
                        parts.push(part);
                    }
                }
                if parts.is_empty() {
                    "0".to_owned()
                } else {
                    format!("({})", parts.into_iter().map(|part| format!("(void)({part})")).collect::<Vec<_>>().join(", "))
                }
            }
            ValueKind::StructLit { fields, .. } => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(_, field)| self.discard_expression(field))
                    .filter(|part| part != "0")
                    .collect();
                if parts.is_empty() {
                    "0".to_owned()
                } else {
                    format!("({})", parts.into_iter().map(|part| format!("(void)({part})")).collect::<Vec<_>>().join(", "))
                }
            }
            ValueKind::Template(pieces) => {
                let parts: Vec<String> = pieces
                    .iter()
                    .filter_map(|piece| match piece {
                        ValuePiece::Literal(_) => None,
                        ValuePiece::Placeholder(inner) => {
                            let part = self.discard_expression(inner);
                            (part != "0").then_some(part)
                        }
                    })
                    .collect();
                if parts.is_empty() {
                    "0".to_owned()
                } else {
                    format!("({})", parts.into_iter().map(|part| format!("(void)({part})")).collect::<Vec<_>>().join(", "))
                }
            }
            ValueKind::DiscardThen { effects, .. } => self.discard_expression(effects),
            ValueKind::Int(_)
            | ValueKind::Float(_)
            | ValueKind::Char(_)
            | ValueKind::Bool(_)
            | ValueKind::Null
            | ValueKind::Str(_)
            | ValueKind::Path(_)
            | ValueKind::Aggregate
            | ValueKind::Temp(_)
            | ValueKind::Closure { .. } => "0".to_owned(),
            _ => {
                let (rendered, guards) = self.guarded_parts(value);
                if guards.is_empty() {
                    format!("(void)({rendered})")
                } else {
                    format!("(({}), (void)({rendered}))", guards.join(", "))
                }
            }
        }
    }

    /// Renders an owned conditional store as one expression.
    ///
    /// The expression spelling is used where the conditional is evaluated only
    /// on some paths, so the store cannot be a statement: the chosen arm moves
    /// into the destination through a nested ternary, and that arm's source is
    /// emptied in the same comma expression. The arm's own guards embed where
    /// the arm is evaluated, so the condition decides what runs.
    fn conditional_move_expr(
        &self,
        target_c: &str,
        ty: &Ty,
        condition: &Value,
        then_value: &Value,
        else_value: &Value,
    ) -> String {
        // Each arm is parenthesized: a field-wise arm returns a bare comma
        // sequence, and a `?:`'s third operand cannot carry a top-level comma
        // — unparenthesized, the sequence's tail would run after the whole
        // conditional on every path.
        format!(
            "({} ? ({}) : ({}))",
            self.guarded_value(condition),
            self.conditional_move_arm_expr(target_c, ty, then_value),
            self.conditional_move_arm_expr(target_c, ty, else_value)
        )
    }

    /// Renders one arm of an expression-form owned conditional store.
    ///
    /// A call arm produces a fresh value. A place, temporary, or materialized
    /// call result is copied into the destination and then emptied, so only the
    /// chosen arm is moved. A nested conditional recurses.
    fn conditional_move_arm_expr(&self, target_c: &str, ty: &Ty, arm: &Value) -> String {
        if let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &arm.kind
        {
            return self.conditional_move_expr(target_c, ty, condition, then_value, else_value);
        }
        // A struct-literal arm whose owned field moves from caller storage
        // stores field by field: the field's move-out empties the source in
        // the same comma sequence, so the destination is the only owner and
        // the source's own cleanup finds nothing. The same form covers a
        // materialized root, a place, a raw-deref read, a member or element
        // selection, and a nested literal or array carrying one of those. A
        // literal without such a field keeps the compact compound-literal
        // spelling.
        if let ValueKind::StructLit { name, fields } = &arm.kind {
            let hands_over = fields.iter().any(|(field, value)| {
                self.value_hands_over_storage(&self.field_type(ty, field), value)
            });
            if hands_over || self.struct_literal_needs_guard_stores(ty, fields) {
                if let Some(stored) = self.struct_assign_expr(target_c, ty, name, fields) {
                    return stored;
                }
            }
        }
        let value_text = self.guarded_value(arm);
        // A place, temporary, materialized value, projection of storage, or
        // raw-deref read names caller storage the chosen arm moves: the copy
        // is emptied in the same expression, so only the destination owns it.
        let storage = arm_has_storage_root(arm);
        if storage {
            let source = self.value(arm);
            format!(
                "({target_c} = {value_text}, {})",
                self.move_zero_text_expr(&source, ty)
            )
        } else {
            format!("({target_c} = {value_text})")
        }
    }

    /// Renders one row of an array literal as a C expression.
    ///
    /// A row that is not itself a literal cannot use a brace list, which has no
    /// C place. It becomes a comma expression instead, in source order, with
    /// its guards embedded where the row is evaluated and its owned source
    /// emptied after the copy. A conditional row nests, and a call row writes
    /// through the row's hidden destination. Returns `None` only when the row's
    /// literal form has no expression form; the caller propagates that to the
    /// deferred store's own fallback.
    fn array_row_expr(&self, target_c: &str, row_ty: &Ty, value: &Value) -> Option<String> {
        match &value.kind {
            ValueKind::ArrayLit { elements, fill } => {
                self.array_assign_expr(target_c, row_ty, elements, fill)
            }
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let then_row = self.array_row_expr(target_c, row_ty, then_value)?;
                let else_row = self.array_row_expr(target_c, row_ty, else_value)?;
                // Both arms end in the same value, so the ternary has one
                // operand type; the row's own value is discarded later.
                Some(format!(
                    "({} ? ({then_row}, 0) : ({else_row}, 0))",
                    self.guarded_value(condition)
                ))
            }
            ValueKind::Call {
                callee,
                type_args,
                args,
                receiver,
            } => {
                let call = self.call_with_destination(
                    callee,
                    type_args,
                    args,
                    receiver.as_deref(),
                    target_c,
                );
                Some(self.row_call_expr(value, &call))
            }
            ValueKind::ClosureCall { callable, args } => {
                let call = self.closure_call_with_destination(callable, args, Some(target_c));
                Some(self.row_call_expr(value, &call))
            }
            // A place, temporary, or otherwise rendered value with storage is
            // copied by bytes; anything else has no row storage form, so the
            // same fallback keeps the renderer total.
            _ => self.row_memcpy_expr(target_c, row_ty, value),
        }
    }

    /// Wraps a row-writing call with its guards.
    ///
    /// The call writes the row through its hidden destination, so the
    /// expression only has to run the call. No value tail is appended: a
    /// discarded `0` would trip the unused-value warning in the strict build,
    /// and a conditional arm wraps the whole row with the value it needs.
    fn row_call_expr(&self, value: &Value, call: &str) -> String {
        let mut guards: Vec<String> = Vec::new();
        self.guard_expressions(value, &mut guards);
        if guards.is_empty() {
            call.to_owned()
        } else {
            format!("(({}), {call})", guards.join(", "))
        }
    }

    /// Renders a row copy from a value that already has storage.
    ///
    /// The row's bytes travel through `memcpy`; when the row is owned the
    /// source is emptied in the same comma expression, so only the destination
    /// owns the moved values. A place that roots at a C-adjusted array
    /// parameter is addressed by its name, and its move-out walks the elements
    /// instead of zeroing the adjusted pointer.
    fn row_memcpy_expr(&self, target_c: &str, row_ty: &Ty, value: &Value) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        self.guard_expressions(value, &mut parts);
        let source = self.value(value);
        let address = self.array_source_address(value, &source);
        parts.push(format!(
            "memcpy({target_c}, {address}, sizeof({target_c}))"
        ));
        // An owned row is a move only when the source really has storage; a
        // rendered expression is a fresh value the destination already took.
        let storage = matches!(
            &value.kind,
            ValueKind::Place(_) | ValueKind::Temp(_) | ValueKind::Materialize { .. }
        ) || ((matches!(
            &value.kind,
            ValueKind::Member { .. } | ValueKind::Index { .. }
        )) && arm_has_storage_root(value));
        if storage && self.needs_drop(row_ty) {
            let zero = match &value.kind {
                ValueKind::Place(place) => self.move_zero_expr(place),
                _ => self.move_zero_text_expr(&source, &self.substitute(&value.ty)),
            };
            parts.push(zero);
        }
        Some(format!("({})", parts.join(", ")))
    }

    /// Renders an array literal as comma-separated element stores.
    ///
    /// A conditionally evaluated array literal cannot use a brace list, which
    /// has no C place. Each element becomes an assignment expression instead,
    /// in source order, with its guards embedded ahead of its own store, so a
    /// skipped path evaluates no element. A multi-extent literal stores each
    /// row through [`Self::array_row_expr`]; a fill covers every index the
    /// prefix did not. Returns `None` for a shape with no expression form, so
    /// the caller can fall back.
    fn array_assign_expr(
        &self,
        target_c: &str,
        ty: &Ty,
        elements: &[Value],
        fill: &Option<Box<Value>>,
    ) -> Option<String> {
        let TyKind::Array { element, extents } = &ty.kind else {
            return None;
        };
        let extent = extents.first().and_then(|size| *size).unwrap_or(0);
        let nested_ty =
            (extents.len() > 1).then(|| Ty::array((**element).clone(), extents[1..].to_vec()));
        let mut parts: Vec<String> = Vec::new();
        for (index, value) in elements.iter().enumerate() {
            let sub = format!("{target_c}[{index}]");
            if let Some(element_ty) = &nested_ty {
                if let ValueKind::ArrayLit { elements, fill } = &value.kind {
                    parts.push(self.array_assign_expr(&sub, element_ty, elements, fill)?);
                    continue;
                }
                // A whole row that is not itself a literal is stored through
                // the row expression, which keeps its guards on the path that
                // evaluates the row.
                parts.push(self.array_row_expr(&sub, element_ty, value)?);
                continue;
            }
            parts.push(self.array_element_store(&sub, element, value)?);
        }
        let Some(fill) = fill else {
            return if parts.is_empty() {
                Some("0".to_owned())
            } else {
                Some(parts.join(", "))
            };
        };
        let prefix = elements.len();
        if let Some(element_ty) = &nested_ty {
            if let ValueKind::ArrayLit { elements, fill } = &fill.kind {
                for index in prefix..extent {
                    let sub = format!("{target_c}[{index}]");
                    parts.push(self.array_assign_expr(&sub, element_ty, elements, fill)?);
                }
                return Some(if parts.is_empty() {
                    "0".to_owned()
                } else {
                    parts.join(", ")
                });
            }
            // A non-literal fill row also uses the expression form, for every
            // index the prefix did not cover.
            for index in prefix..extent {
                let sub = format!("{target_c}[{index}]");
                parts.push(self.array_row_expr(&sub, element_ty, fill)?);
            }
            return Some(if parts.is_empty() {
                "0".to_owned()
            } else {
                parts.join(", ")
            });
        }
        let mut guards: Vec<String> = Vec::new();
        self.guard_expressions(fill, &mut guards);
        let lowered = self.store_text(fill, element);
        for index in prefix..extent {
            let assignment = format!("{target_c}[{index}] = {lowered}");
            parts.push(if guards.is_empty() {
                assignment
            } else {
                format!("(({}), {})", guards.join(", "), assignment)
            });
        }
        Some(if parts.is_empty() {
            "0".to_owned()
        } else {
            parts.join(", ")
        })
    }

    /// Stores one array element inside a guard.
    ///
    /// A struct-typed element that is a literal stores field by field through
    /// [`Self::struct_assign_expr`]: the compound spelling would brace-elide the
    /// element's own nested fixed-array field, which the strict build rejects
    /// and which loses the nested view's length. Every other element keeps the
    /// plain store with its own guards and its owned-source move.
    fn array_element_store(&self, sub: &str, element: &Ty, value: &Value) -> Option<String> {
        if matches!(element.kind, TyKind::Struct(..)) {
            if let ValueKind::StructLit { name, fields } = &value.kind {
                if let Some(stored) = self.struct_assign_expr(sub, element, name, fields) {
                    return Some(stored);
                }
            }
        }
        // A conditional element moves the chosen arm into the element and
        // empties that arm, exactly as the unguarded per-arm store does: each
        // arm's own release then finds nothing to free, so the element stays
        // the single owner of the moved fields.
        if let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &value.kind
        {
            if self.needs_drop(element) {
                return Some(self.conditional_move_expr(
                    sub,
                    element,
                    condition,
                    then_value,
                    else_value,
                ));
            }
        }
        let mut guards: Vec<String> = Vec::new();
        self.guard_expressions(value, &mut guards);
        let lowered = self.store_text(value, element);
        let mut store = format!("{sub} = {lowered}");
        // An owned element source is a move: the store leaves the element
        // owning, so the source is emptied in the same comma expression.
        if self.needs_drop(element) && arm_has_storage_root(value) {
            let source = self.value(value);
            store = format!(
                "({store}, {})",
                self.move_zero_text_expr(&source, element)
            );
        }
        Some(if guards.is_empty() {
            store
        } else {
            format!("(({}), {})", guards.join(", "), store)
        })
    }

    /// True when an owned value moves from caller storage at any depth.
    ///
    /// A compound literal copies a field's handle but leaves the source
    /// storage owning the same value, so the field must be stored through a
    /// move: a place, a compiler temporary, a materialized root, a member or
    /// element selection, or a raw-deref read names that storage directly. A
    /// nested struct literal or array literal is searched through its own
    /// positions, because a place reached inside it has the same problem.
    fn value_hands_over_storage(&self, ty: &Ty, value: &Value) -> bool {
        self.value_hands_over_storage_with(ty, value, true)
    }

    /// True when an owned literal field moves from caller storage.
    ///
    /// An adopting materialized carrier is compiler storage whose consumer
    /// already owns the value, so the direct literal fill keeps its compact
    /// copy for that form. A place, a deref, a member, an element, a
    /// non-adopting carrier, or a nested literal that reaches one still forces
    /// the move.
    fn value_hands_over_literal_storage(&self, ty: &Ty, value: &Value) -> bool {
        self.value_hands_over_storage_with(ty, value, false)
    }

    /// The shared walk for the two storage judgments.
    fn value_hands_over_storage_with(
        &self,
        ty: &Ty,
        value: &Value,
        include_materialized: bool,
    ) -> bool {
        if !self.needs_drop(ty) {
            return false;
        }
        if arm_has_storage_root(value) {
            return include_materialized
                || !matches!(
                    value.kind,
                    ValueKind::Materialize {
                        adopting: true,
                        ..
                    }
                );
        }
        match &value.kind {
            ValueKind::StructLit { fields, .. } => fields.iter().any(|(field, value)| {
                self.value_hands_over_storage_with(
                    &self.field_type(ty, field),
                    value,
                    include_materialized,
                )
            }),
            ValueKind::ArrayLit { elements, fill } => {
                let element = ty.iterated_element();
                elements.iter().any(|value| {
                    self.value_hands_over_storage_with(&element, value, include_materialized)
                }) || fill.as_deref().is_some_and(|value| {
                    self.value_hands_over_storage_with(&element, value, include_materialized)
                })
            }
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.value_hands_over_storage_with(ty, then_value, include_materialized)
                    || self.value_hands_over_storage_with(
                        ty,
                        else_value,
                        include_materialized,
                    )
            }
            _ => false,
        }
    }

    /// True when a fixed-array value is entirely literal leaves.
    ///
    /// A brace list of constants needs no statement form. A nested row is
    /// checked recursively, so an all-literal grid keeps its brace list while
    /// a row with a call, a conditional, or a storage read does not.
    fn fixed_array_value_is_literal(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::ArrayLit { elements, fill } => {
                elements
                    .iter()
                    .all(|value| self.fixed_array_value_is_literal(value))
                    && fill
                        .as_deref()
                        .map_or(true, |value| self.fixed_array_value_is_literal(value))
            }
            ValueKind::StructLit { fields, .. } => fields
                .iter()
                .all(|(_, value)| self.fixed_array_value_is_literal(value)),
            ValueKind::Int(_)
            | ValueKind::Float(_)
            | ValueKind::Char(_)
            | ValueKind::Bool(_)
            | ValueKind::Str(_)
            | ValueKind::Null => true,
            _ => false,
        }
    }

    /// True when a struct literal needs field-by-field stores inside a guard.
    ///
    /// Only a fixed-array field whose element is itself an aggregate forces the
    /// per-field form: a C compound literal would brace-elide its initializer
    /// into the element's first member, which the strict build rejects, and a
    /// nested literal that names one has no initializer form either. Every
    /// other literal keeps its compound spelling, so an emission that names no
    /// such field stays unchanged.
    fn struct_literal_needs_guard_stores(&self, ty: &Ty, fields: &[(String, Value)]) -> bool {
        fields.iter().any(|(field, value)| {
            let field_ty = self.field_type(ty, field);
            let field_ty = if field_ty.is_unknown() {
                value.ty.clone()
            } else {
                field_ty
            };
            if field_ty.is_fixed_array() {
                if self.fixed_array_value_needs_guard_stores(value, &field_ty) {
                    return true;
                }
            }
            // An owned field that moves from caller storage — a place, a
            // raw-deref read, a member or element selection, or a nested
            // literal carrying one — must store through the move, because the
            // compound literal cannot empty the source. A materialized carrier
            // is already owned by its consumer, so its compact copy stays.
            if self.value_hands_over_literal_storage(&field_ty, value) {
                return true;
            }
            if let ValueKind::StructLit {
                fields: inner_fields,
                ..
            } = &value.kind
            {
                if matches!(field_ty.kind, TyKind::Struct(..)) {
                    return self.struct_literal_needs_guard_stores(&field_ty, inner_fields);
                }
            }
            false
        })
    }

    /// True when a fixed-array field's value has no C initializer form here.
    ///
    /// A field whose element is an aggregate takes the per-field form, because
    /// the strict build rejects the brace elision a compound literal would
    /// apply. A field that is not a brace list is forced too: a conditional or
    /// a call writing through the hidden destination has no array value. A
    /// nested row must itself be a brace list, so a call or a conditional at
    /// any depth forces the store form. An element or fill that moves an owned
    /// value from caller storage forces it as well, and an all-literal nested
    /// grid keeps its brace list because every leaf is a constant.
    fn fixed_array_value_needs_guard_stores(&self, value: &Value, ty: &Ty) -> bool {
        let element = ty.iterated_element();
        if matches!(
            element.kind,
            TyKind::Struct(..) | TyKind::Str | TyKind::StaticStr
        ) {
            return true;
        }
        let ValueKind::ArrayLit { elements, fill } = &value.kind else {
            return true;
        };
        if element.is_fixed_array() {
            if self.fixed_array_value_is_literal(value) {
                return false;
            }
            let needs = |inner: &Value| {
                if !matches!(inner.kind, ValueKind::ArrayLit { .. }) {
                    return true;
                }
                self.fixed_array_value_needs_guard_stores(inner, &element)
            };
            return elements.iter().any(needs) || fill.as_deref().is_some_and(needs);
        }
        let needs = |inner: &Value| self.value_hands_over_literal_storage(&element, inner);
        elements.iter().any(needs) || fill.as_deref().is_some_and(needs)
    }

    /// Renders a struct literal as field-by-field stores inside a guard.
    ///
    /// A struct literal whose view-array field would take a brace-elided
    /// initializer cannot materialize as one compound literal under the strict
    /// build, and a field whose value carries its own guards must be evaluated
    /// where the guard reaches it. Each declared field is therefore stored in a
    /// comma sequence: a fixed-array field assigns element by element, a nested
    /// literal recurses, and every other field stores its value with its own
    /// guards first. A field the literal omits takes its empty state. Returns
    /// `None` when the type names no declared fields, so the caller keeps the
    /// compound-literal spelling.
    fn struct_assign_expr(
        &self,
        target_c: &str,
        ty: &Ty,
        name: &str,
        fields: &[(String, Value)],
    ) -> Option<String> {
        if !matches!(ty.kind, TyKind::Struct(..)) {
            return None;
        }
        let declared = self
            .sema
            .table
            .struct_fields(name)
            .map(<[(String, Ty)]>::to_vec)
            .unwrap_or_default();
        if declared.is_empty() {
            return None;
        }
        let mut parts: Vec<String> = Vec::new();
        // The literal's fields evaluate in source order (Ch 04 §3), so the
        // stores follow the written order, not the declaration order. Omitted
        // fields are zeroed afterwards; zeroing has no side effects.
        for (field, value) in fields {
            let field_ty = self.field_type(ty, field);
            let sub = format!("{target_c}.{field}");
            if field_ty.is_fixed_array() {
                if let ValueKind::ArrayLit { elements, fill } = &value.kind {
                    if let Some(stored) = self.array_assign_expr(&sub, &field_ty, elements, fill) {
                        parts.push(stored);
                        continue;
                    }
                }
                // Every other fixed-array value stores through the row core:
                // a conditional fills per arm, a call writes through the hidden
                // destination, and a place source copies its bytes and empties.
                if let Some(stored) = self.array_row_expr(&sub, &field_ty, value) {
                    parts.push(stored);
                    continue;
                }
            }
            // An owned conditional field moves the chosen arm into the field
            // and empties that arm's source, exactly as an element store does.
            // A plain store would copy the handle and leave the chosen arm
            // owning the same value until its own cleanup released it again.
            if self.needs_drop(&field_ty) {
                if let ValueKind::Conditional {
                    condition,
                    then_value,
                    else_value,
                } = &value.kind
                {
                    parts.push(self.conditional_move_expr(
                        &sub,
                        &field_ty,
                        condition,
                        then_value,
                        else_value,
                    ));
                    continue;
                }
            }
            if matches!(field_ty.kind, TyKind::Struct(..)) {
                if let ValueKind::StructLit {
                    name: inner,
                    fields: inner_fields,
                } = &value.kind
                {
                    if let Some(stored) =
                        self.struct_assign_expr(&sub, &field_ty, inner, inner_fields)
                    {
                        parts.push(stored);
                        continue;
                    }
                }
            }
            let mut guards: Vec<String> = Vec::new();
            self.guard_expressions(value, &mut guards);
            let lowered = self.store_text(value, &field_ty);
            let mut store = format!("{sub} = {lowered}");
            // An owned field source is a move: the store leaves the field
            // owning, so the source is emptied in the same comma expression.
            if self.needs_drop(&field_ty) && arm_has_storage_root(value) {
                let source = self.value(value);
                store = format!(
                    "({store}, {})",
                    self.move_zero_text_expr(&source, &field_ty)
                );
            }
            parts.push(if guards.is_empty() {
                store
            } else {
                format!("(({}), {})", guards.join(", "), store)
            });
        }
        for (field, _) in &declared {
            if fields.iter().any(|(written, _)| written == field) {
                continue;
            }
            let field_ty = self.field_type(ty, field);
            let sub = format!("{target_c}.{field}");
            // C cannot assign an array, so an omitted fixed-array field is
            // zeroed through its address; every other omitted field takes its
            // empty state as a store.
            if field_ty.is_fixed_array() {
                parts.push(format!("cl_zero(&{sub})"));
            } else {
                parts.push(format!("{sub} = {}", self.empty_state(&field_ty)));
            }
        }
        Some(parts.join(", "))
    }

    /// The move-out of a consumed non-place value as a comma operand.
    ///
    /// A raw-deref `take` argument transfers its pointee, so the caller's
    /// storage is emptied through the same rendered address the read used.
    /// A projected member (`(*p).name`) empties only the selected member, so
    /// the pointee's sibling fields stay owned. The guard expression runs
    /// after the call, on the evaluated path only.
    fn deref_move_zero_expr(&self, value: &Value, ty: &Ty) -> Option<String> {
        if !value_moves_pointee(value) {
            return None;
        }
        let source = self.value(value);
        let move_out = self.move_zero_text_expr(&source, ty);
        let Some(capture) = &self.take_capture else {
            return Some(move_out);
        };
        // The flag is set here, in the move-out's own comma sequence, so a
        // skipped take never marks the save consumed. The addresses are cast to
        // `void *`: a raw pointer cast can reach the destination's storage
        // through a different nominal type, and an unrelated object simply
        // compares unequal without a cross-type pointer warning.
        let address = self.deref_value_address(value)?;
        Some(format!(
            "({flag} = ({flag} || (((void *)({address})) == ((void *)(&({target})))))), {move_out}",
            flag = capture.flag,
            target = capture.target,
        ))
    }

    /// The plain rendering of a value and its always-evaluated operand guards.
    ///
    /// The guards — a materialize fill, a bounds or division trap — run once,
    /// ahead of the value they check; the rendering itself is the plain
    /// expression a caller may read again. A null guard's test reads the
    /// rendering twice, so separating the two keeps the guards from running
    /// twice.
    pub(super) fn guarded_parts(&self, value: &Value) -> (String, Vec<String>) {
        let mut guards: Vec<String> = Vec::new();
        self.guard_expressions(value, &mut guards);
        (self.value(value), guards)
    }

    /// Collects the guard expressions of a value's always-evaluated part.
    ///
    /// The right operand of a logical operator and the arms of `?:` are left
    /// out because [`Self::guarded_value`] embeds their guards where the
    /// operand is actually evaluated.
    fn guard_expressions(&self, value: &Value, out: &mut Vec<String>) {
        match &value.kind {
            ValueKind::Binary {
                op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr,
                left,
                ..
            }
            | ValueKind::Conditional {
                condition: left, ..
            } => self.guard_expressions(left, out),
            ValueKind::Binary { op, left, right } => {
                self.guard_expressions(left, out);
                self.guard_expressions(right, out);
                if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                    if let Some(condition) = self.division_guard_condition(left, right) {
                        out.push(trap_expression(&condition, "E11210", "integer division by zero"));
                    }
                }
            }
            ValueKind::Index { base, index } => {
                self.guard_expressions(base, out);
                self.guard_expressions(index, out);
                if let Some(condition) = self.bounds_guard_condition(base, index) {
                    out.push(trap_expression(&condition, "E11211", "index out of range"));
                }
            }
            ValueKind::Cast { value: inner, .. } => {
                self.guard_expressions(inner, out);
                let target = self.substitute(&value.ty);
                let source = self.substitute(&inner.ty);
                if target.is_integer() && source.is_numeric() && !source.is_integer() {
                    if let Some(condition) = self.float_guard_condition(inner, &target) {
                        out.push(trap_expression(
                            &condition,
                            "E11212",
                            "float-to-integer conversion out of range",
                        ));
                    }
                }
            }
            ValueKind::Unary { operand, .. } => self.guard_expressions(operand, out),
            ValueKind::Member { base, .. } => self.guard_expressions(base, out),
            ValueKind::Call {
                args, receiver, ..
            } => {
                if let Some(receiver) = receiver {
                    self.guard_expressions(receiver, out);
                }
                for arg in args {
                    self.guard_expressions(arg, out);
                }
            }
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.guard_expressions(field, out);
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.guard_expressions(element, out);
                }
                if let Some(fill) = fill {
                    self.guard_expressions(fill, out);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.guard_expressions(inner, out);
                    }
                }
            }
            ValueKind::Intrinsic { name, args, .. } => {
                for arg in args {
                    self.guard_expressions(arg, out);
                }
                if name == "slice" {
                    if let Some(condition) = self.slice_guard_condition(args) {
                        out.push(trap_expression(&condition, "E11211", "index out of range"));
                    }
                }
            }
            ValueKind::DiscardThen { effects, value } => {
                let discarded = self.discard_expression(effects);
                if discarded != "0" {
                    out.push(discarded);
                }
                self.guard_expressions(value, out);
            }
            // A materialized operand inside a conditionally evaluated value
            // (a short-circuit operand or a `?:` arm) carries its side effect
            // as an expression, so it runs only on that path. A fixed-array
            // call writes through its hidden out-parameter, so the guard
            // expression is the call itself; every guard's value is discarded
            // ahead of the guarded value, so no result expression is needed.
            ValueKind::Materialize {
                temp,
                destination: destination_place,
                value: inner,
                move_outs,
                value_move_outs,
                adopting,
            } => {
                // The carried value's type can still be opaque in a generic
                // body — an unresolved conditional take's selected member —
                // so the expression fill resolves it through the value the
                // instance substitutes, exactly as the declaration does.
                let inner_ty = self.resolved_value_ty(inner);
                let destination = self.materialize_destination(*temp, destination_place);
                let owned_conditional = self.needs_drop(&inner_ty)
                    && !inner_ty.is_fixed_array()
                    && matches!(inner.kind, ValueKind::Conditional { .. });
                if let (
                    true,
                    ValueKind::Conditional {
                        condition,
                        then_value,
                        else_value,
                    },
                ) = (owned_conditional, &inner.kind)
                {
                    // The conditional moves its chosen arm. Its condition's
                    // guards embed with the store expression, and each arm's
                    // guards embed where the arm is evaluated, so nothing is
                    // hoisted out of the conditionally evaluated path. An
                    // adopting carrier's consumer owns the value, so a re-fill
                    // must not release the previous value; the consumer's own
                    // cleanup is the sole owner.
                    if !*adopting {
                        if let Some(drop) = self.drop_expr_text(&destination, &inner_ty) {
                            out.push(drop);
                        }
                    }
                    out.push(self.conditional_move_expr(
                        &destination,
                        &inner_ty,
                        condition,
                        then_value,
                        else_value,
                    ));
                } else {
                    // A deferred literal embeds each element's or field's own
                    // guards in its store, so walking the literal here would
                    // materialize a side-effecting element twice and empty a
                    // `take` source before the second evaluation. The previous
                    // temporary is dropped before the element stores run,
                    // unless the carrier is adopting: its consumer owns the
                    // value, so a re-fill must not release what the consumer
                    // stored.
                    // A fixed-array value that is not a call embeds its own
                    // guards in its store text: the row core renders each
                    // arm's and each element's guards where the value is
                    // evaluated, so walking the value here would run them a
                    // second time. A call keeps the walk because its
                    // arguments' guards are not part of the call text.
                    let literal_stores = inner_ty.is_fixed_array()
                        && !matches!(
                            &inner.kind,
                            ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
                        );
                    let struct_store = match &inner.kind {
                        ValueKind::StructLit { name, fields }
                            if self.struct_literal_needs_guard_stores(&inner_ty, fields) =>
                        {
                            self.struct_assign_expr(&destination, &inner_ty, name, fields)
                        }
                        _ => None,
                    };
                    if !literal_stores && struct_store.is_none() {
                        self.guard_expressions(inner, out);
                    }
                    if !*adopting && self.needs_drop(&inner_ty) {
                        if let Some(drop) = self.drop_expr_text(&destination, &inner_ty) {
                            out.push(drop);
                        }
                    }
                    let stored = match struct_store {
                        Some(stored) => stored,
                        None => match &inner.kind {
                            ValueKind::Call {
                                callee,
                                type_args,
                                args,
                                receiver,
                            } if inner_ty.is_fixed_array() => self.call_with_destination(
                                callee,
                                type_args,
                                args,
                                receiver.as_deref(),
                                &destination,
                            ),
                            // A fixed-array closure call in a conditionally
                            // evaluated operand also writes through its hidden
                            // destination, taken after the environment and the
                            // lambda's own arguments.
                            ValueKind::ClosureCall { callable, args }
                                if inner_ty.is_fixed_array() =>
                            {
                                self.closure_call_with_destination(
                                    callable,
                                    args,
                                    Some(&destination),
                                )
                            }
                            ValueKind::ArrayLit { elements, fill }
                                if inner_ty.is_fixed_array() =>
                            {
                                self.array_assign_expr(&destination, &inner_ty, elements, fill)
                                    .unwrap_or_else(|| {
                                        format!("({destination} = {})", self.value(inner))
                                    })
                            }
                            // A fixed-array value with no simpler expression
                            // form — a conditional of places or calls, or a
                            // place projection — fills the destination through
                            // the row core: each arm copies on its own path, a
                            // call arm passes the hidden destination, and a
                            // place source empties once the bytes travel.
                            _ if inner_ty.is_fixed_array() => self
                                .array_row_expr(&destination, &inner_ty, inner)
                                .unwrap_or_else(|| {
                                    format!("({destination} = {})", self.value(inner))
                                }),
                            _ => format!("({destination} = {})", self.value(inner)),
                        },
                    };
                    out.push(stored);
                }
                // The call runs only when this operand is evaluated, so its
                // `take` move-outs are emptied here, after the store. Only an
                // owning place is emptied: a duplicable instance leaves the
                // caller its copy.
                for place in move_outs {
                    let ty = self.resolved_place_ty(place);
                    if !self.needs_drop(&ty) {
                        continue;
                    }
                    out.push(self.move_zero_expr(place));
                }
                // A consumed value that names no place is emptied through its
                // own address in the same comma sequence, after the call.
                for value in value_move_outs {
                    let ty = self.resolved_value_ty(value);
                    if !self.needs_drop(&ty) {
                        continue;
                    }
                    if let Some(move_out) = self.deref_move_zero_expr(value, &ty) {
                        out.push(move_out);
                    }
                }
            }
            ValueKind::Closure { captures, .. } => {
                for capture in captures {
                    self.guard_expressions(capture, out);
                }
            }
            ValueKind::ClosureCall { callable, args } => {
                self.guard_expressions(callable, out);
                for arg in args {
                    self.guard_expressions(arg, out);
                }
            }
            _ => {}
        }
    }

    /// Emits the integer division and modulo guard.
    ///
    /// Float division is IEEE-defined and never faults, so only an integer
    /// operation gets a guard.
    pub(super) fn emit_division_guard(&mut self, left: &Value, right: &Value) {
        if let Some(condition) = self.division_guard_condition(left, right) {
            self.push(&format!(
                "    if ({condition}) cl_trap(\"E11210\", \"integer division by zero\");"
            ));
        }
    }

    /// The condition under which an integer division or modulo aborts.
    fn division_guard_condition(&self, left: &Value, right: &Value) -> Option<String> {
        // A generic body records the operand as an unresolved projection, so
        // the instance's concrete scalar decides whether the operation is an
        // integer division that needs the guard.
        let ty = self.resolved_value_ty(left);
        if !ty.is_integer() {
            return None;
        }
        let dividend = self.value(left);
        let divisor = self.value(right);
        Some(match scalar_of(&ty) {
            Some(scalar) if is_signed_scalar(scalar) => {
                let min = signed_min(scalar);
                format!("{divisor} == 0 || ({dividend} == {min} && {divisor} == -1)")
            }
            _ => format!("{divisor} == 0"),
        })
    }

    /// Emits the index bounds check for a fixed array, slice, or string view.
    ///
    /// A signed index also has to be non-negative: the upper-bound comparison
    /// alone lets `xs[-1]` through. The comparison casts both sides to the
    /// unsigned `usize` so a signed index does not trigger
    /// `-Werror=sign-compare`; the negative check runs first, so the cast
    /// never sees a value that wrapped.
    pub(super) fn emit_bounds_guard(&mut self, base: &Value, index: &Value) {
        if let Some(condition) = self.bounds_guard_condition(base, index) {
            self.push(&format!(
                "    if ({condition}) cl_trap(\"E11211\", \"index out of range\");"
            ));
        }
    }

    /// The condition under which an index is out of range.
    fn bounds_guard_condition(&self, base: &Value, index: &Value) -> Option<String> {
        // The base resolves like any value: a chained index (`xs[i][j]`)
        // carries the element placeholder, and a projection carries the
        // opaque type, so the guard decides on the substituted row exactly as
        // the access does.
        let ty = self.resolved_value_ty(base);
        let index_c = self.value(index);
        let signed = scalar_of(&self.substitute(&index.ty)).is_some_and(is_signed_scalar);
        match &ty.kind {
            TyKind::Array { extents, .. } => {
                if extents.iter().all(Option::is_some) {
                    // The index selects the outermost dimension, so the bound
                    // is the first extent; the constant-index check and the
                    // semantic layer use the same bound.
                    let extent = extents.first().and_then(|size| *size).unwrap_or(0);
                    // A constant index into a fixed array is already
                    // range-checked when the program is validated, so the
                    // emitted guard would be dead code. Leaving it out also
                    // keeps a constant trap expression from folding inside a
                    // short-circuit operand, where its value is discarded and
                    // trips `-Wunused-value`.
                    if let ValueKind::Int(value) = &index.kind {
                        if *value >= 0 && (*value as u128) < extent as u128 {
                            return None;
                        }
                    }
                    Some(bounds_condition(&index_c, signed, &extent.to_string()))
                } else {
                    let base_c = self.value(base);
                    Some(bounds_condition(&index_c, signed, &format!("({base_c}).len")))
                }
            }
            TyKind::Str | TyKind::StaticStr => {
                let base_c = self.value(base);
                Some(bounds_condition(&index_c, signed, &format!("({base_c}).len")))
            }
            _ => None,
        }
    }

    /// Emits the float-to-integer range guard.
    pub(super) fn emit_float_guard(&mut self, value: &Value, target: &Ty) {
        if let Some(condition) = self.float_guard_condition(value, target) {
            self.push(&format!(
                "    if ({condition}) cl_trap(\"E11212\", \"float-to-integer conversion out of range\");"
            ));
        }
    }

    /// The condition under which a float-to-integer conversion is out of range.
    fn float_guard_condition(&self, value: &Value, target: &Ty) -> Option<String> {
        let value_c = self.value(value);
        let (low, high) = match scalar_of(target) {
            Some(ScalarType::I8) => ("-128.0", "127.0"),
            Some(ScalarType::I16) => ("-32768.0", "32767.0"),
            Some(ScalarType::I32) | Some(ScalarType::Isize) => ("-2147483648.0", "2147483647.0"),
            Some(ScalarType::U8) => ("0.0", "255.0"),
            Some(ScalarType::U16) => ("0.0", "65535.0"),
            Some(ScalarType::U32) | Some(ScalarType::Usize) => ("0.0", "4294967295.0"),
            _ => return None,
        };
        Some(format!(
            "cl_isnan({value_c}) || !({value_c} >= {low} && {value_c} <= {high})"
        ))
    }

}

/// The C expression that aborts when `condition` holds.
///
/// A guard that must embed inside an expression cannot use an `if` statement,
/// so it becomes a ternary whose trap branch runs `cl_trap` through the comma
/// operator and yields zero. The expression always has type `int`, so it is a
/// valid comma operand wherever a guard belongs.
fn trap_expression(condition: &str, code: &str, message: &str) -> String {
    format!("(({condition}) ? (cl_trap(\"{code}\", \"{message}\"), 0) : 1)")
}

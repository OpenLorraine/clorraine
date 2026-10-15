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

use super::function::{arm_has_storage_root, ArrayTarget};
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
            // A materialized operand is always evaluated where it stands, so
            // it is filled by a statement before the guarded construct. A
            // fixed-array call writes through its hidden out-parameter, so the
            // temporary is the call's destination instead of an assignment
            // target.
            ValueKind::Materialize {
                temp,
                value: inner,
                move_outs,
                adopting,
            } => {
                self.emit_guards(inner);
                // An adopting carrier's consumer owns the value, so the
                // previous value must not be released on a re-fill: the
                // destination's own cleanup is the sole owner.
                if !adopting {
                    self.emit_reassigned_temp_drop(*temp, &inner.ty);
                }
                let inner_ty = self.substitute(&inner.ty);
                if let ValueKind::Conditional {
                    condition,
                    then_value,
                    else_value,
                } = &inner.kind
                {
                    // A deferred conditional operand fills the temporary
                    // through the owned branch store, so the chosen arm moves
                    // and the skipped arm is never evaluated. A fixed-array
                    // conditional has no expression form; the fallback keeps
                    // the renderer total.
                    let ty = self.substitute(&inner.ty);
                    if !ty.is_fixed_array() && self.needs_drop(&ty) {
                        let destination = format!("__cl_tmp{temp}");
                        self.emit_conditional_move_assign(
                            &destination,
                            &ty,
                            condition,
                            then_value,
                            else_value,
                        );
                    } else {
                        self.push(&format!("    __cl_tmp{temp} = {};", self.value(inner)));
                    }
                } else if let ValueKind::ArrayLit { elements, fill } = &inner.kind {
                    if inner_ty.is_fixed_array() {
                        // A brace list has no C place, so a deferred array
                        // literal is stored element by element into the
                        // temporary, on the path that evaluates the operand.
                        let destination = format!("__cl_tmp{temp}");
                        self.emit_array_assign_to(
                            &super::function::ArrayTarget::object(&destination),
                            &inner_ty,
                            elements,
                            fill,
                        );
                    } else {
                        self.push(&format!("    __cl_tmp{temp} = {};", self.value(inner)));
                    }
                } else {
                    let destination = format!("__cl_tmp{temp}");
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
                // The call's `take` arguments and a consume carrier's source
                // slot are emptied only now, on the evaluated path, so a
                // skipped guard leaves them owned. A duplicable instance
                // copies, so its source keeps its value and is not zeroed;
                // only a type that owns storage moves.
                for place in move_outs {
                    let ty = self.substitute(&place.ty);
                    if !self.needs_drop(&ty) {
                        continue;
                    }
                    let source = self.place(place);
                    self.emit_move_zero_place(place, &source, &ty);
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
                value: inner,
                adopting,
                ..
            } => {
                let inner_ty = self.substitute(&inner.ty);
                let destination = format!("__cl_tmp{temp}");
                if !*adopting
                    && self.needs_drop(&inner_ty)
                    && self.drop_expr_text(&destination, &inner_ty).is_none()
                {
                    // A guard temporary is re-filled on each evaluation, so
                    // its previous value must be released before the fill
                    // overwrites it. An adopting carrier's consumer owns the
                    // value, so the consumer's own cleanup is the sole owner:
                    // releasing the carrier here would free the consumer's
                    // storage again when the guarded statement re-executes.
                    self.emit_reassigned_temp_drop(*temp, &inner.ty);
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
                let mut guards: Vec<String> = Vec::new();
                self.guard_expressions(value, &mut guards);
                let rendered = self.value(value);
                if guards.is_empty() {
                    rendered
                } else {
                    format!("(({}), {})", guards.join(", "), rendered)
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
        format!(
            "({} ? {} : {})",
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
        let value_text = self.guarded_value(arm);
        let storage = matches!(
            &arm.kind,
            ValueKind::Place(_) | ValueKind::Temp(_) | ValueKind::Materialize { .. }
        ) || ((matches!(&arm.kind, ValueKind::Member { .. } | ValueKind::Index { .. }))
            && arm_has_storage_root(arm));
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
                let element = field_ty.iterated_element();
                if matches!(
                    element.kind,
                    TyKind::Struct(..) | TyKind::Str | TyKind::StaticStr
                ) {
                    return true;
                }
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
        for (field, _) in &declared {
            let field_ty = self.field_type(ty, field);
            let sub = format!("{target_c}.{field}");
            let Some((_, value)) = fields.iter().find(|(written, _)| written == field) else {
                // C cannot assign an array, so an omitted fixed-array field is
                // zeroed through its address; every other omitted field takes
                // its empty state as a store.
                if field_ty.is_fixed_array() {
                    parts.push(format!("cl_zero(&{sub})"));
                } else {
                    parts.push(format!("{sub} = {}", self.empty_state(&field_ty)));
                }
                continue;
            };
            if field_ty.is_fixed_array() {
                if let ValueKind::ArrayLit { elements, fill } = &value.kind {
                    if let Some(stored) = self.array_assign_expr(&sub, &field_ty, elements, fill) {
                        parts.push(stored);
                        continue;
                    }
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
        Some(parts.join(", "))
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
            // A materialized operand inside a conditionally evaluated value
            // (a short-circuit operand or a `?:` arm) carries its side effect
            // as an expression, so it runs only on that path. A fixed-array
            // call writes through its hidden out-parameter, so the guard
            // expression is the call itself; every guard's value is discarded
            // ahead of the guarded value, so no result expression is needed.
            ValueKind::Materialize {
                temp,
                value: inner,
                move_outs,
                adopting,
            } => {
                let inner_ty = self.substitute(&inner.ty);
                let destination = format!("__cl_tmp{temp}");
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
                    let literal_stores = inner_ty.is_fixed_array()
                        && matches!(&inner.kind, ValueKind::ArrayLit { .. });
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
                                        format!("(__cl_tmp{temp} = {})", self.value(inner))
                                    })
                            }
                            _ => format!("(__cl_tmp{temp} = {})", self.value(inner)),
                        },
                    };
                    out.push(stored);
                }
                // The call runs only when this operand is evaluated, so its
                // `take` move-outs are emptied here, after the store.
                for place in move_outs {
                    out.push(self.move_zero_expr(place));
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
        let ty = self.substitute(&left.ty);
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
        let ty = self.substitute(&base.ty);
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

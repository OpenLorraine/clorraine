//! Function-body emission from the typed control-flow IR.
//!
//! This module owns the function prologue, the hoisted local declarations, the
//! statements with a run-time effect, and the local-read analysis used to mark
//! an unused local. The cleanup ladder and the block walk live in the sibling
//! [`super::cleanup`] module, the drop lowering in [`super::drop`], and the
//! structured control-flow walk in [`super::control`].
//!
//! Locals are hoisted to the top of the function with their empty-state
//! initializer, so a cleanup path may reference any of them. The type of every
//! place and value is read from the typed IR rather than re-inferred, and every
//! statement is attributed to its source line with a `#line` directive.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{self, Place, Statement, Terminator, Value, ValueKind, ValuePiece};
use crate::ast::UnaryOp;
use crate::types::{resolve_ty, substitute_instance, Ty, TyKind};

use super::render::escape_c_string;
use super::types::{declarator, function_declarator, is_struct_view};
use super::{
    is_pointer_param, sanitize_identifier, substitute_in, type_param_names, Emitter, FuncInfo,
};

impl<'a> Emitter<'a> {
    // -- function emission ---------------------------------------------------

    /// Emits one function body from its lowered IR.
    ///
    /// `instance` carries the type substitution and symbol key of a generic
    /// instantiation; a non-generic body passes `None`.
    pub(super) fn emit_function(
        &mut self,
        info: &FuncInfo<'a>,
        instance: Option<(BTreeMap<String, Ty>, String)>,
    ) {
        self.subst = instance
            .as_ref()
            .map(|(subst, _)| subst.clone())
            .unwrap_or_default();
        self.instance_key = instance.as_ref().map(|(_, key)| key.clone());
        self.used_names.clear();
        self.local_names.clear();
        self.pointer_params.clear();
        self.scope_actions.clear();
        self.rungs.clear();
        self.labels.clear();
        self.used_rung_labels.clear();
        self.deferred_result_store = false;
        self.result_var = None;
        self.reachable.clear();
        self.emitted.clear();
        self.pending_labels.clear();
        self.defer_copies.clear();
        self.active_defers.clear();
        self.cleanup_frames.clear();
        self.label_copy = 0;
        self.copy_region = None;
        self.loop_entry.clear();
        self.suppress_output = false;

        let Some(function) = self
            .ownership
            .functions
            .iter()
            .find(|function| function.name == info.decl.name && function.module == info.module)
        else {
            // Every clean compile lowers a body for each declared function; a
            // miss means an earlier stage rejected the program, so there is
            // nothing to emit.
            return;
        };
        self.seed_generated_names(function.temps.len());
        self.function = Some(function);
        self.current_module = info.module.clone();
        self.return_ty = self.substitute(&function.ret);
        // A by-value fixed-array return is written into the hidden trailing
        // out-parameter, so the caller's destination is the result variable on
        // every path, with or without a cleanup ladder.
        if self.return_ty.is_fixed_array() {
            self.result_var = Some("__cl_ret".to_owned());
        }

        let param_names = self.unique_param_names(
            function.temps.len(),
            function.params.iter().map(|param| param.name.as_str()),
        );
        for (param, c_name) in function.params.iter().zip(param_names) {
            self.used_names.insert(c_name.clone());
            self.local_names.insert(param.name.clone(), c_name);
            let ty = self.substitute(&param.ty);
            if is_pointer_param(&ty, param.mode, &self.sema.table) {
                self.pointer_params.insert(param.name.clone());
            }
        }
        let mut declared: BTreeSet<String> = BTreeSet::new();
        for local in &function.locals {
            if !declared.insert(local.name.clone()) {
                continue;
            }
            let c_name = self.unique_local(&local.name);
            self.local_names.insert(local.name.clone(), c_name);
        }

        self.collect_cleanup(function);
        let has_cleanup = self.scope_actions.values().any(|actions| !actions.is_empty());
        if has_cleanup && !self.return_ty.is_void() {
            self.result_var = Some("__cl_ret".to_owned());
        }
        self.check_stack_frame(function, info.decl.name_span);

        let symbol = match &instance {
            Some((_, key)) => format!("cl_{}__{}", info.decl.name, key),
            None => self.function_symbol(&info.module, &info.decl.name),
        };
        let signature = self.signature_with_symbol(&info.module, &info.decl, &self.subst, &symbol);
        self.push(&signature);
        self.push("{");
        self.emit_local_declarations(function);
        self.emit_structured_body(function, has_cleanup);
        self.emit_closing_return();
        self.push("}");
        self.push("");

        self.function = None;
    }

    /// Emits one lambda's lifted function body.
    ///
    /// A lifted function has no source declaration, so its symbol comes from the
    /// lambda site and its captures are read through the leading environment
    /// parameter. A capture-less lambda's function takes only the lambda's own
    /// parameters. A lifted function in a generic body carries the enclosing
    /// instance's substitution when specialized, so its parameter, local, and
    /// return types resolve to the instance's concrete types and its C name
    /// carries the instance key.
    pub(super) fn emit_lifted_function(
        &mut self,
        function: &'a ir::Function,
        instance: Option<(&BTreeMap<String, Ty>, &str)>,
    ) {
        let Some(site) = function.lambda_site else {
            return;
        };
        let capturing = self
            .sema
            .closures
            .get(&site)
            .is_some_and(|closure| !closure.captures.is_empty());

        // The shared emission has neither a substitution nor an instance key;
        // a specialized emission installs both so every type and closure name
        // resolves to the instance.
        self.subst = instance
            .map(|(subst, _)| subst.clone())
            .unwrap_or_default();
        self.instance_key = instance.map(|(_, key)| key.to_owned());
        self.used_names.clear();
        self.seed_generated_names(function.temps.len());
        self.local_names.clear();
        self.pointer_params.clear();
        self.scope_actions.clear();
        self.rungs.clear();
        self.labels.clear();
        self.used_rung_labels.clear();
        self.deferred_result_store = false;
        self.result_var = None;
        self.reachable.clear();
        self.emitted.clear();
        self.pending_labels.clear();
        self.defer_copies.clear();
        self.active_defers.clear();
        self.cleanup_frames.clear();
        self.label_copy = 0;
        self.copy_region = None;
        self.loop_entry.clear();
        self.suppress_output = false;

        self.function = Some(function);
        self.current_module = function.module.clone();
        self.return_ty = self.substitute(&function.ret);
        // A by-value fixed-array return is written into the hidden trailing
        // out-parameter, so the caller's destination is the result on every
        // path, with or without a cleanup ladder.
        if self.return_ty.is_fixed_array() {
            self.result_var = Some("__cl_ret".to_owned());
        }

        let param_names = self.unique_param_names(
            function.temps.len(),
            function.params.iter().map(|param| param.name.as_str()),
        );
        for (param, c_name) in function.params.iter().zip(param_names) {
            self.used_names.insert(c_name.clone());
            self.local_names.insert(param.name.clone(), c_name);
        }
        let mut declared: BTreeSet<String> = BTreeSet::new();
        for local in &function.locals {
            if !declared.insert(local.name.clone()) {
                continue;
            }
            let c_name = self.unique_local(&local.name);
            self.local_names.insert(local.name.clone(), c_name);
        }

        self.collect_cleanup(function);
        let has_cleanup = self.scope_actions.values().any(|actions| !actions.is_empty());
        if has_cleanup && !self.return_ty.is_void() {
            self.result_var = Some("__cl_ret".to_owned());
        }

        let symbol = match instance {
            Some((_, key)) => format!("cl_lambda_{site}__{key}"),
            None => format!("cl_lambda_{site}"),
        };
        let signature = self.lifted_signature(function, site, capturing, &symbol);
        self.push(&signature);
        self.push("{");
        self.emit_local_declarations(function);
        if capturing {
            // The environment parameter is only read when the body touches a
            // capture, so name it to keep `-Wunused-parameter` quiet.
            self.push("    if (sizeof(&__env)) { }");
        }
        self.emit_structured_body(function, has_cleanup);
        self.emit_closing_return();
        self.push("}");
        self.push("");

        self.function = None;
    }

    /// The C signature of one lifted lambda function.
    ///
    /// A capturing lambda's function takes a leading environment pointer named
    /// `__env`; a capture-less one has only the lambda parameters. The function
    /// is always `static`, because a lifted lambda is reached only through its
    /// value, never by name from another translation unit. The shared
    /// declarator adds the hidden destination for a by-value fixed-array return
    /// and nests a func-typed return, so the lifted function's signature cannot
    /// drift from the value that calls it. The parameter and return types are
    /// substituted, so a specialization takes the instance's concrete types.
    pub(super) fn lifted_signature(
        &self,
        function: &ir::Function,
        site: usize,
        capturing: bool,
        symbol: &str,
    ) -> String {
        let param_names = self.unique_param_names(
            function.temps.len(),
            function.params.iter().map(|param| param.name.as_str()),
        );
        let params: Vec<String> = function
            .params
            .iter()
            .zip(param_names)
            .map(|(param, c_name)| {
                declarator(&self.substitute(&param.ty), &c_name, &self.sema.table)
            })
            .collect();
        let env = capturing.then(|| format!("{} *__env", self.env_struct_name(site)));
        let ret = self.substitute(&function.ret);
        let signature = function_declarator(&ret, params, symbol, env, &self.sema.table);
        format!("static {signature}")
    }

    /// The C name of a lifted lambda's environment struct.
    ///
    /// The instance spelling mirrors the callable and lifted names, so the
    /// environment a creation site builds is the one the specialized lifted
    /// function expects.
    pub(super) fn env_struct_name(&self, site: usize) -> String {
        match &self.instance_key {
            Some(key) => format!("struct ClEnv_{site}__{key}"),
            None => format!("struct ClEnv_{site}"),
        }
    }

    /// Writes the hoisted local declarations, the temporaries, and the result.
    fn emit_local_declarations(&mut self, function: &'a ir::Function) {
        let unread = unread_locals(
            function,
            |ty| self.needs_drop(&self.substitute(ty)),
            |ty| crate::types::is_null_shaped(&self.substitute(ty)),
            |ty| self.carries_null_value(ty),
        );
        let mut written: BTreeSet<String> = BTreeSet::new();
        for local in &function.locals {
            if !written.insert(local.name.clone()) {
                continue;
            }
            let c_name = self
                .local_names
                .get(&local.name)
                .cloned()
                .unwrap_or_else(|| sanitize_identifier(&local.name));
            let ty = self.substitute(&local.ty);
            let empty = self.empty_state(&ty);
            self.push(&format!("    {} = {};", declarator(&ty, &c_name, &self.sema.table), empty));
            // A local the emitted code never reads would otherwise fail
            // `-Wunused-but-set-variable` under `-Werror`, because the hoisted
            // declaration initializes it. A `sizeof` test marks the value as
            // deliberately unused without an effect; a read local is
            // unaffected. The test is portable: both the host and m68k GCC
            // accept a bare `(void)` cast, but vbcc reports it as a
            // no-effect statement, so the standard cast is not used here.
            if unread.contains(&local.name) {
                self.push(&format!("    if (sizeof({c_name})) {{ }}"));
            }
        }
        for (index, ty) in function.temps.iter().enumerate() {
            let ty = self.substitute(ty);
            let empty = self.empty_state(&ty);
            self.push(&format!(
                "    {} = {};",
                declarator(&ty, &format!("__cl_tmp{index}"), &self.sema.table),
                empty
            ));
            // A compiler temporary may only be written on a path that never
            // reads it, so mark it deliberately used to keep the emitted C
            // free of `-Wunused-but-set-variable`. A `sizeof` test is used
            // instead of a `(void)` cast because vbcc flags the cast as a
            // no-effect statement.
            self.push(&format!("    if (sizeof(__cl_tmp{index})) {{ }}"));
        }
        if let Some(result) = self.result_var.clone() {
            // A fixed-array result is the hidden out-parameter the caller
            // passed in, so it already exists as a parameter and must not be
            // redeclared as a local. It may go unwritten when every path
            // panics, so it is marked used to keep the emitted C free of
            // `-Wunused-parameter`. Every other result is a hoisted local.
            if self.return_ty.is_fixed_array() {
                self.push("    if (sizeof(&__cl_ret)) { }");
            } else {
                let ret = self.return_ty.clone();
                let empty = self.empty_state(&ret);
                self.push(&format!("    {} = {};", declarator(&ret, &result, &self.sema.table), empty));
            }
        }
        // Reference every parameter so an unused one does not fail the
        // `-Wunused-parameter` check the emitted C must pass. A `sizeof` test
        // on the parameter's address names it without reading it, which
        // satisfies both the host and m68k GCC and vbcc; the address form also
        // avoids `-Wsizeof-array-argument` for an array parameter. A used
        // parameter is unaffected by the extra statement.
        for param in &function.params {
            let c_name = self
                .local_names
                .get(&param.name)
                .cloned()
                .unwrap_or_else(|| sanitize_identifier(&param.name));
            self.push(&format!("    if (sizeof(&{c_name})) {{ }}"));
        }
    }

    /// Reports W10004 when the summed frame of the function is too large.
    ///
    /// Every local, parameter, and temporary contributes; a size the type
    /// table does not model yet suppresses the warning rather than guessing.
    fn check_stack_frame(&mut self, function: &'a ir::Function, span: crate::span::Span) {
        let limit = self.profile.stack_frame_limit();
        let mut total = 0usize;
        for ty in function
            .params
            .iter()
            .map(|param| &param.ty)
            .chain(function.locals.iter().map(|local| &local.ty))
            .chain(function.temps.iter())
        {
            match self.sema.table.size_of(&self.substitute(ty)) {
                Some(size) => total = total.saturating_add(size),
                None => return,
            }
        }
        if total > limit {
            self.warn(
                "W10004",
                format!("stack frame of '{}' is {total} bytes", function.name),
                span,
            );
        }
    }

    // -- statements -----------------------------------------------------------

    /// Emits one IR statement that has a run-time effect.
    pub(super) fn emit_statement(&mut self, span: crate::span::Span, statement: &Statement) {
        match statement {
            Statement::Assign { target, value } => {
                if let ValueKind::ArrayLit { elements, fill } = &value.kind {
                    let ty = self.substitute(&target.ty);
                    if matches!(&ty.kind, TyKind::Array { extents, .. } if extents.iter().all(Option::is_some))
                    {
                        self.emit_line_directive(span);
                        self.emit_reassigned_target_drop(target);
                        self.emit_array_assign(target, &ty, elements, fill);
                        return;
                    }
                }
                // A by-value fixed-array call is lowered through a hidden
                // out-parameter: the call names the caller's destination as its
                // trailing argument and the assignment itself is dropped. The
                // call writes into a fresh temporary first, because the callee
                // may read the target (`s = dup(borrow s)`), so the old target
                // is released only after the call has returned.
                if let ValueKind::Call {
                    callee,
                    type_args,
                    args,
                    receiver,
                } = &value.kind
                {
                    let ty = self.substitute(&target.ty);
                    if ty.is_fixed_array() {
                        self.emit_fixed_array_call_assign(
                            span,
                            target,
                            &ty,
                            callee,
                            type_args,
                            args,
                            receiver.as_deref(),
                        );
                        return;
                    }
                }
                self.emit_assign(span, target, value);
            }
            Statement::Move { target, source } => {
                self.emit_line_directive(span);
                self.emit_reassigned_target_drop(target);
                let target_c = self.place(target);
                let source_c = self.place(source);
                // C cannot assign an array, so a fixed-array move copies the
                // bytes instead; the zeroed source below empties the original.
                // A fixed-array parameter is adjusted to a pointer to its
                // first element, so the parameter name already is the array's
                // address and the byte count comes from the element type.
                if self.substitute(&target.ty).is_fixed_array() {
                    let target_param = self.is_array_param_place(target);
                    let source_param = self.is_array_param_place(source);
                    let target_value = if target_param {
                        target_c.clone()
                    } else {
                        format!("&({target_c})")
                    };
                    let source_value = if source_param {
                        source_c.clone()
                    } else {
                        format!("&({source_c})")
                    };
                    let bytes = if !target_param {
                        format!("sizeof({target_c})")
                    } else if !source_param {
                        format!("sizeof({source_c})")
                    } else {
                        let target_ty = self.substitute(&target.ty);
                        self.array_param_bytes(&target_c, &target_ty)
                    };
                    self.push(&format!(
                        "    memcpy({target_value}, {source_value}, {bytes});"
                    ));
                } else if crate::types::is_null_shaped(&self.substitute(&source.ty))
                    && !crate::types::is_null_shaped(&self.substitute(&target.ty))
                    && self.carries_null_value(&target.ty)
                {
                    // A null-shaped source lowers to the target's own empty
                    // value: a view target needs its zero view and a handle
                    // target its null, not the shape's `void *`.
                    let empty = self.null_value(&self.substitute(&target.ty));
                    self.push(&format!("    {target_c} = {empty};"));
                } else {
                    self.push(&format!("    {target_c} = {source_c};"));
                }
                // A move whose source type does not own at this instantiation
                // is a plain copy: the source stays intact for a duplicable
                // instance, while an owning one empties it.
                let source_ty = self.substitute(&source.ty);
                if self.needs_drop(&source_ty) {
                    self.emit_move_zero_place(source, &source_c, &source_ty);
                }
            }
            Statement::MoveOut { place } => {
                self.emit_line_directive(span);
                // The callee took the value; only an owning argument place is
                // emptied, because a duplicable instance leaves the caller its
                // copy.
                let ty = self.substitute(&place.ty);
                if self.needs_drop(&ty) {
                    let source = self.place(place);
                    self.emit_move_zero_place(place, &source, &ty);
                }
            }
            Statement::StorePointer { pointer, value } => {
                self.emit_line_directive(span);
                // The pointer expression carries the element's run-time index,
                // so its guards (including the bounds check) must run first.
                self.emit_guards(pointer);
                let pointer_c = self.value(pointer);
                let pointer_ty = self.substitute(&pointer.ty);
                if pointer_ty.is_fixed_array() {
                    // A fixed-array element is not assignable in C, so the
                    // store goes through the byte-copy/branch-store core
                    // against the dereferenced pointer. The pointer may alias
                    // the value, so the old bytes are always saved.
                    let target_c = format!("(*({pointer_c}))");
                    let target = ArrayTarget::object(&target_c);
                    self.emit_fixed_array_assign_to(&target, &pointer_ty, value, true);
                } else if self.needs_drop(&pointer_ty) {
                    // An owned conditional moves the chosen arm into the
                    // element per path, exactly as it does for a named
                    // destination; a value expression would copy the handle and
                    // leave the arm owning the same storage.
                    if let ValueKind::Conditional {
                        condition,
                        then_value,
                        else_value,
                    } = &value.kind
                    {
                        let target_c = format!("*({pointer_c})");
                        self.emit_conditional_move_assign(
                            &target_c,
                            &pointer_ty,
                            condition,
                            then_value,
                            else_value,
                        );
                    } else {
                        // The replaced element owns a value, so it is released
                        // (and its storage cleared) before the value is
                        // evaluated, exactly as a named destination is: a value
                        // that reads the element — including the element itself
                        // — then reads the emptied storage instead of the freed
                        // one. The value's own source is emptied once the
                        // element holds it.
                        self.emit_drop_expr(&format!("*({pointer_c})"), &pointer_ty);
                        self.emit_guards(value);
                        let lowered = self.store_text(value, &pointer_ty);
                        self.push(&format!("    *({pointer_c}) = {lowered};"));
                        self.emit_arm_source_moves(value, &pointer_ty);
                    }
                } else {
                    self.emit_guards(value);
                    let value_c = self.store_text(value, &pointer_ty);
                    self.push(&format!("    *({pointer_c}) = {value_c};"));
                }
            }
            Statement::Call {
                callee,
                type_args,
                args,
                receiver,
                result,
            } => {
                self.emit_call(
                    span,
                    callee,
                    type_args,
                    args,
                    receiver.as_deref(),
                    result.as_ref(),
                );
            }
            Statement::Discard { value } => {
                self.emit_line_directive(span);
                // A discarded literal with a fixed-array field that only a
                // store can fill has no C initializer form: the literal is
                // built in a statement-local destination and released.
                if let ValueKind::StructLit { fields, .. } = &value.kind {
                    if self.fixed_array_field_needs_stores(fields) {
                        let ty = self.substitute(&value.ty);
                        let dest = self.unique_local("__cl_discard");
                        self.push("    {");
                        self.push(&format!(
                            "        {} = {};",
                            declarator(&ty, &dest, &self.sema.table),
                            self.empty_state(&ty)
                        ));
                        self.emit_struct_literal_fields(&dest, &ty, fields);
                        if self.needs_drop(&ty) {
                            self.emit_drop_expr(&dest, &ty);
                        }
                        self.push("    }");
                        return;
                    }
                }
                // A discarded fixed-array value with no C expression result
                // writes into a statement-local destination that the cleanup
                // releases when the elements own storage: a closure call
                // through its hidden out-parameter, and a conditional per
                // arm, because a `?:` cannot pass the hidden out-parameter a
                // fixed-array call arm needs.
                if value.ty.is_fixed_array() {
                    if let ValueKind::ClosureCall { callable, args } = &value.kind {
                        self.emit_guards(value);
                        let ty = self.substitute(&value.ty);
                        let dest = self.unique_local("__cl_discard");
                        self.push("    {");
                        self.push(&format!(
                            "        {} = {};",
                            declarator(&ty, &dest, &self.sema.table),
                            self.empty_state(&ty)
                        ));
                        let call =
                            self.closure_call_with_destination(callable, args, Some(&dest));
                        self.push(&format!("        {call};"));
                        if self.needs_drop(&ty) {
                            self.emit_drop_expr(&dest, &ty);
                        }
                        self.push("    }");
                        return;
                    }
                    if let ValueKind::Conditional {
                        condition,
                        then_value,
                        else_value,
                    } = &value.kind
                    {
                        let ty = self.substitute(&value.ty);
                        let dest = self.unique_local("__cl_discard");
                        self.push("    {");
                        self.push(&format!(
                            "        {} = {};",
                            declarator(&ty, &dest, &self.sema.table),
                            self.empty_state(&ty)
                        ));
                        let target = ArrayTarget::object(&dest);
                        self.emit_fixed_array_conditional_assign(
                            &target,
                            &ty,
                            condition,
                            then_value,
                            else_value,
                        );
                        if self.needs_drop(&ty) {
                            self.emit_drop_expr(&dest, &ty);
                        }
                        self.push("    }");
                        return;
                    }
                }
                // A literal has no C expression form: its leaves materialize
                // into statement temporaries the statement's release frees, so
                // the guards that fill them run here and no value expression is
                // emitted.
                if matches!(
                    value.kind,
                    ValueKind::ArrayLit { .. } | ValueKind::StructLit { .. }
                ) {
                    self.emit_guards(value);
                    return;
                }
                self.emit_guards(value);
                let value_c = self.value(value);
                self.push(&format!("    (void)({value_c});"));
            }
            Statement::ReleaseTemp(release) => {
                self.emit_line_directive(span);
                let place = crate::ir::Place {
                    base: crate::ir::PlaceBase::Temp(release.temp),
                    projections: Vec::new(),
                    ty: release.ty.clone(),
                };
                self.emit_drop(&place);
            }
            Statement::Guards { checks } => {
                self.emit_line_directive(span);
                for check in checks {
                    self.emit_guards(check);
                }
            }
            Statement::Borrow { .. }
            | Statement::Drop { .. }
            | Statement::Defer { .. }
            | Statement::ScopeEnd { .. }
            | Statement::ScopeExit { .. } => {}
            Statement::Nop => self.emit_line_directive(span),
        }
    }

    /// Emits a call, a discarded expression statement, or a trap intrinsic.
    fn emit_call(
        &mut self,
        span: crate::span::Span,
        callee: &str,
        type_args: &[Ty],
        args: &[ir::Value],
        receiver: Option<&ir::Value>,
        result: Option<&Place>,
    ) {
        match callee {
            "assert" => {
                self.emit_line_directive(span);
                if let Some(condition) = args.first() {
                    self.emit_guards(condition);
                    let condition_c = self.value(condition);
                    self.push(&format!(
                        "    if (!({condition_c})) cl_trap(\"E11213\", \"assertion failed\");"
                    ));
                }
            }
            "panic" => {
                self.emit_line_directive(span);
                self.emit_panic(args.first());
            }
            _ => {
                self.emit_line_directive(span);
                // A callable field's receiver may carry a runtime index, so
                // its guards run before the call exactly like an argument's.
                if let Some(receiver) = receiver {
                    self.emit_guards(receiver);
                }
                for arg in args {
                    self.emit_guards(arg);
                }
                // A by-value fixed-array result is C's hidden out-parameter:
                // the call writes through the caller's destination instead of
                // assigning the (unassignable) array result. A discarded result
                // still needs storage, so it writes into a statement-local
                // destination that the cleanup releases.
                let array_result = match result {
                    Some(place) => self.substitute(&place.ty).is_fixed_array(),
                    None => self
                        .callee_result_type(callee, receiver)
                        .is_some_and(|ty| ty.is_fixed_array()),
                };
                if array_result {
                    if let Some(place) = result {
                        let destination = self.place(place);
                        let call = self.call_with_destination(
                            callee,
                            type_args,
                            args,
                            receiver,
                            &destination,
                        );
                        self.push(&format!("    {call};"));
                    } else if let Some(ty) = self.callee_result_type(callee, receiver) {
                        let dest = self.unique_local("__cl_discard");
                        self.push("    {");
                        self.push(&format!(
                            "        {} = {};",
                            declarator(&ty, &dest, &self.sema.table),
                            self.empty_state(&ty)
                        ));
                        let call = self.call_with_destination(
                            callee,
                            type_args,
                            args,
                            receiver,
                            &dest,
                        );
                        self.push(&format!("        {call};"));
                        if self.needs_drop(&ty) {
                            self.emit_drop_expr(&dest, &ty);
                        }
                        self.push("    }");
                    }
                    return;
                }
                let call = self.call(callee, type_args, args, receiver);
                let prefix = match result {
                    Some(place) => format!("{} = ", self.place(place)),
                    None => String::new(),
                };
                self.push(&format!("    {prefix}{call};"));
            }
        }
    }

    /// Renders a `panic` message for the C runtime.
    ///
    /// The runtime takes a pointer and a length, so no NUL terminator is
    /// required. A string literal's bytes live in static storage and are
    /// passed directly. Every other form is evaluated once into a
    /// block-local of its own type, so the pointer and the length read the
    /// same value and a side-effecting message expression runs once: a `str`
    /// view passes its pointer and length, an owned `String` passes its
    /// handle through the string accessors, and any other type keeps today's
    /// behavior.
    fn emit_panic(&mut self, arg: Option<&Value>) {
        let Some(arg) = arg else {
            self.push("    cl_panic((const unsigned char *)\"panic\", (usize)5);");
            return;
        };
        // The message argument may carry runtime guards and a deferred
        // materialization (an element read or an index), so they run first,
        // exactly as they do for a call argument.
        self.emit_guards(arg);
        if let ValueKind::Str(text) = &arg.kind {
            let escaped = escape_c_string(text.as_bytes());
            self.push(&format!(
                "    cl_panic((const unsigned char *){escaped}, (usize){});",
                text.len()
            ));
            return;
        }
        let ty = self.substitute(&arg.ty);
        let local = self.unique_local("__cl_panic_msg");
        let decl = declarator(&ty, &local, &self.sema.table);
        let lowered = self.store_text(arg, &ty);
        self.push("    {");
        self.push(&format!("        {decl} = {lowered};"));
        match &ty.kind {
            TyKind::Handle(name) if name == "String" => {
                self.push(&format!(
                    "        cl_panic((const unsigned char *)cl_string_ptr({local}), (usize)cl_string_len({local}));"
                ));
            }
            TyKind::Str | TyKind::StaticStr => {
                self.push(&format!(
                    "        cl_panic((const unsigned char *)({local}).ptr, (usize)(({local}).len));"
                ));
            }
            TyKind::CString | TyKind::Raw(_) => {
                // A C string pointer is NUL-terminated, so its byte length is
                // measured at the call. This keeps the pointer forms printing
                // their text, as they did before the length-based runtime.
                self.push(&format!(
                    "        cl_panic((const unsigned char *)({local}), (usize)cl_cstring_len((const char *)({local})));"
                ));
            }
            _ => {
                self.push(&format!("        cl_panic({local}, (usize)0);"));
            }
        }
        self.push("    }");
    }

    /// The declared parameter modes of a known user or `ffi` function.
    pub(super) fn callee_modes(&self, callee: &str) -> Option<Vec<crate::ast::Mode>> {
        if let Some(info) = self.resolve_function(callee) {
            return Some(info.decl.params.iter().map(|param| param.mode).collect());
        }
        self.ffi_functions
            .get(callee)
            .map(|signature| signature.params.iter().map(|param| param.mode).collect())
    }

    /// The declared parameter types of a known user or `ffi` function.
    ///
    /// A user signature resolves through the emitter's current generic
    /// substitution, so a call inside a generic body sees the concrete
    /// parameter types. The call-site renderer uses this to widen a `String`
    /// argument to the `str` view a parameter expects.
    pub(super) fn callee_param_types(&self, callee: &str) -> Option<Vec<Ty>> {
        if let Some(info) = self.resolve_function(callee) {
            let names = type_param_names(info.decl);
            return Some(
                info.decl
                    .params
                    .iter()
                    .map(|param| {
                        let ty = self.resolve_in_module(&info.module, &param.ty, &names);
                        substitute_in(&ty, &self.subst)
                    })
                    .collect(),
            );
        }
        self.ffi_functions.get(callee).map(|signature| {
            signature
                .params
                .iter()
                .map(|param| resolve_ty(&param.ty, &[], &self.sema.table))
                .collect()
        })
    }

    /// The parameter types of a known user function under the call's explicit
    /// type arguments.
    ///
    /// The call's own type arguments fix the callee's parameters, so the
    /// caller's substitution must not leak into them: the declared parameter
    /// types are resolved fresh and the instance substitution is applied. A
    /// type argument that mentions the caller's parameter is substituted first.
    /// Without type arguments the call is inferred and
    /// [`Self::callee_param_types`] already carries the caller's substitution;
    /// an `ffi` callee has no type parameters and keeps its own signature.
    pub(super) fn instance_param_types(
        &self,
        callee: &str,
        type_args: &[Ty],
    ) -> Option<Vec<Ty>> {
        if type_args.is_empty() {
            return self.callee_param_types(callee);
        }
        let info = self.resolve_function(callee)?;
        let names = type_param_names(info.decl);
        let concrete: Vec<Ty> = type_args.iter().map(|ty| self.substitute(ty)).collect();
        Some(
            info.decl
                .params
                .iter()
                .map(|param| {
                    let ty = self.resolve_in_module(&info.module, &param.ty, &names);
                    substitute_instance(&ty, &names, &concrete)
                })
                .collect(),
        )
    }

    /// The declared return type of a known user function, with the active
    /// generic substitution applied.
    ///
    /// `None` for an unknown callee or an intrinsic, whose result type the
    /// emitter does not need to name here. The call-site renderer uses this to
    /// learn whether a result is a by-value fixed array, which C lowers through
    /// a hidden destination argument.
    pub(super) fn callee_return_type(&self, callee: &str) -> Option<Ty> {
        self.resolve_function(callee).map(|info| {
            let names = type_param_names(info.decl);
            let ty = self.resolve_in_module(&info.module, &info.decl.returns, &names);
            self.substitute(&ty)
        })
    }

    /// The declared return type of a call whose callee is a value.
    ///
    /// A func-typed local or parameter wins over a same-named module function,
    /// because a binding shadows a module name; the local's signature then
    /// decides whether a fixed-array result needs the hidden destination. A
    /// callable field carries its signature on the receiver value, a lambda
    /// capture through the environment, a module-level func-typed global on
    /// its own symbol, and a callable-bound closure parameter on its two-word
    /// value. Every other callee resolves as a named function.
    pub(super) fn callee_result_type(
        &self,
        callee: &str,
        receiver: Option<&ir::Value>,
    ) -> Option<Ty> {
        if let Some(ty) = self.callee_func_type(callee, receiver) {
            if let TyKind::Func(signature) = &ty.kind {
                return Some((*signature.returns).clone());
            }
        }
        if let Some(ty) = self.closure_param_type(callee) {
            if let TyKind::Closure(closure) = &ty.kind {
                return Some((*closure.returns).clone());
            }
        }
        self.callee_return_type(callee)
    }

    /// The function type of a call whose callee is a func value.
    ///
    /// A callable receiver (a struct field or a call result), a func-typed
    /// local or parameter, a lambda capture, and a module-level func-typed
    /// global all carry the signature the argument renderer needs. A declared
    /// function, an `ffi` function, and an intrinsic yield `None`, so those
    /// calls keep the declared-signature path.
    pub(super) fn callee_func_type(&self, callee: &str, receiver: Option<&ir::Value>) -> Option<Ty> {
        if let Some(receiver) = receiver {
            let ty = self.substitute(&receiver.ty);
            if matches!(ty.kind, TyKind::Func(_)) {
                return Some(ty);
            }
        }
        if let Some(ty) = self.func_value_type(callee) {
            return Some(ty);
        }
        // A module-level func-typed global resolves through its own identity:
        // a qualified or imported spelling names the exact binding, and a
        // bare name is taken only when the current module declares it or a
        // single module does.
        self.func_global_binding(callee).map(|(_, ty)| ty)
    }

    /// The checked type of a func-typed local, parameter, or lambda capture.
    ///
    /// The declaration order mirrors the call-resolution order in the
    /// lowering: a local or parameter shadows a capture, which shadows a
    /// module name. `None` when the name is not a func-typed binding.
    pub(super) fn func_value_type(&self, name: &str) -> Option<Ty> {
        if let Some(function) = self.function {
            let declared = function
                .locals
                .iter()
                .map(|local| (&local.name, &local.ty))
                .chain(
                    function
                        .params
                        .iter()
                        .map(|param| (&param.name, &param.ty)),
                );
            for (local, ty) in declared {
                if local == name
                    || self
                        .local_names
                        .get(local)
                        .is_some_and(|c_name| c_name == name)
                {
                    let ty = self.substitute(ty);
                    return matches!(ty.kind, TyKind::Func(_)).then_some(ty);
                }
            }
        }
        let site = self.function?.lambda_site?;
        let closure = self.sema.closures.get(&site)?;
        let capture = closure
            .captures
            .iter()
            .find(|capture| capture.name == name)?;
        let ty = self.substitute(&capture.ty);
        matches!(ty.kind, TyKind::Func(_)).then_some(ty)
    }

    /// Drops the previous value of an assignment target before it is overwritten.
    ///
    /// An assignment to a live owned place must release the old value first, or
    /// it leaks. The drop is null-safe, so writing a fresh (empty) target is
    /// unaffected; a scalar target owns nothing and is skipped.
    fn emit_reassigned_target_drop(&mut self, target: &Place) {
        let ty = self.resolved_place_ty(target);
        if self.needs_drop(&ty) {
            self.emit_drop(target);
        }
    }

    /// Emits an assignment, releasing the old owned value without a use-after-free.
    ///
    /// The right-hand side may read the target, as in `f = dup(borrow f)`, so
    /// the old value cannot be released before the right-hand side is
    /// evaluated. An owned target is therefore handled through a scratch local:
    /// the old value is saved, the right-hand side is stored into the target,
    /// and only then is the saved value released. A target that owns nothing is
    /// overwritten directly.
    fn emit_assign(&mut self, span: crate::span::Span, target: &Place, value: &Value) {
        let ty = self.resolved_place_ty(target);
        self.emit_line_directive(span);
        if ty.is_fixed_array() {
            self.emit_fixed_array_assign(target, &ty, value);
            return;
        }
        // A literal with a field that has no C initializer form stores its
        // fields through statements, whether or not the type owns anything;
        // a fixed-array field is such a field even for a duplicable type.
        if let ValueKind::StructLit { fields, .. } = &value.kind {
            if self.struct_literal_needs_stores(fields) {
                self.emit_struct_literal_store(target, &ty, fields);
                return;
            }
        }
        if self.needs_drop(&ty) {
            // An owned conditional moves the chosen arm into the destination
            // per path: a value expression would copy the handle and leave the
            // arm owning the same storage.
            if let ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } = &value.kind
            {
                let target_c = self.place(target);
                self.emit_conditional_move_assign(
                    &target_c,
                    &ty,
                    condition,
                    then_value,
                    else_value,
                );
                return;
            }
        }
        // A conditional of structs stores its arms per field when an arm
        // cannot use a C aggregate initializer, even though the type owns
        // nothing (a fixed-array field makes a `?:` on two struct values
        // invalid C).
        if let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &value.kind
        {
            if self.conditional_struct_needs_stores(&ty, then_value, else_value) {
                let target_c = self.place(target);
                self.emit_struct_conditional_assign(
                    &target_c,
                    &ty,
                    condition,
                    then_value,
                    else_value,
                );
                return;
            }
        }
        if !self.emits_drop(&ty) {
            self.emit_guards(value);
            let target_c = self.place(target);
            let lowered = self.store_text(value, &ty);
            self.push(&format!("    {target_c} = {lowered};"));
            return;
        }
        let target_c = self.place(target);
        if !value_reads_base(value, target) {
            // The right-hand side does not read the target, so the old value
            // is released before the right-hand side is evaluated.
            self.emit_drop(target);
            self.emit_guards(value);
            let lowered = self.store_text(value, &ty);
            self.push(&format!("    {target_c} = {lowered};"));
            return;
        }
        // The right-hand side reads the target (`f = dup(borrow f)`), so the old
        // value must survive until the right-hand side has been evaluated. Save
        // it, store the new value, then release the saved old value.
        let save = self.unique_local("__cl_save");
        self.push("    {");
        self.push(&format!(
            "        {} = {};",
            declarator(&ty, &save, &self.sema.table),
            target_c
        ));
        self.emit_guards(value);
        let lowered = self.store_text(value, &ty);
        self.push(&format!("        {target_c} = {lowered};"));
        self.emit_drop_expr(&save, &ty);
        self.push("    }");
    }

    /// Emits a whole-array copy for a fixed-array assignment.
    ///
    /// The target place is rendered and delegated to
    /// [`Self::emit_fixed_array_assign_to`] together with whether the value
    /// reads it, so a value that reads the target survives until the copy has
    /// run.
    fn emit_fixed_array_assign(&mut self, target: &Place, ty: &Ty, value: &Value) {
        let target_c = self.place(target);
        let reads_target = value_reads_base(value, target);
        let target = self.array_target(target, &target_c);
        self.emit_fixed_array_assign_to(&target, ty, value, reads_target);
    }

    /// Emits a whole-array store into a rendered C lvalue.
    ///
    /// C arrays are not assignable, so the bytes travel with `memcpy`. A
    /// literal stores element by element, a conditional branches per arm, and
    /// a by-value call writes into a temporary first, because the callee may
    /// read the target. A right-hand side that renders to the target itself
    /// (`a = a;`) already holds the value, so only its guards can have an
    /// effect. When the target owns elements and the value may read it
    /// (`reads_target`), the old bytes are saved into a zeroed local first and
    /// released once after the copy; otherwise the old value is released
    /// before the copy. The rendered form lets a run-time element target, a
    /// struct-literal field, and a plain place share one core.
    pub(super) fn emit_fixed_array_assign_to(
        &mut self,
        target: &ArrayTarget,
        ty: &Ty,
        value: &Value,
        reads_target: bool,
    ) {
        if let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &value.kind
        {
            self.emit_fixed_array_conditional_assign(
                target,
                ty,
                condition,
                then_value,
                else_value,
            );
            return;
        }
        if let ValueKind::ArrayLit { elements, fill } = &value.kind {
            // An array literal has one initializer per element, so it is
            // stored element by element; a `memcpy` cannot name its braces.
            if self.needs_drop(ty) {
                self.emit_drop_expr(&target.text, ty);
            }
            self.emit_array_assign_to(target, ty, elements, fill);
            return;
        }
        if let ValueKind::Call {
            callee,
            type_args,
            args,
            receiver,
        } = &value.kind
        {
            // A by-value fixed-array call writes through a hidden out-parameter,
            // so it needs storage before the target is released. The call runs
            // into a fresh temporary first, because the callee may read the
            // target.
            if let Some(receiver) = receiver.as_deref() {
                self.emit_guards(receiver);
            }
            for arg in args {
                self.emit_guards(arg);
            }
            let temp = self.unique_local("__cl_call");
            self.push("    {");
            self.push(&format!(
                "        {} = {};",
                declarator(ty, &temp, &self.sema.table),
                self.empty_state(ty)
            ));
            let call =
                self.call_with_destination(callee, type_args, args, receiver.as_deref(), &temp);
            self.push(&format!("        {call};"));
            if self.needs_drop(ty) {
                self.emit_drop_expr(&target.text, ty);
            }
            self.push(&format!(
                "        memcpy({}, &({temp}), {});",
                target.address, target.bytes
            ));
            self.emit_move_zero(&temp, ty);
            self.push("    }");
            return;
        }
        if let ValueKind::ClosureCall { callable, args } = &value.kind {
            // A lifted closure with a by-value fixed-array return uses the same
            // hidden out-parameter ABI, with the destination after the
            // environment and the lambda's own arguments.
            self.emit_guards(callable);
            for arg in args {
                self.emit_guards(arg);
            }
            let temp = self.unique_local("__cl_call");
            self.push("    {");
            self.push(&format!(
                "        {} = {};",
                declarator(ty, &temp, &self.sema.table),
                self.empty_state(ty)
            ));
            let call = self.closure_call_with_destination(callable, args, Some(&temp));
            self.push(&format!("        {call};"));
            if self.needs_drop(ty) {
                self.emit_drop_expr(&target.text, ty);
            }
            self.push(&format!(
                "        memcpy({}, &({temp}), {});",
                target.address, target.bytes
            ));
            self.emit_move_zero(&temp, ty);
            self.push("    }");
            return;
        }
        let value_c = self.value(value);
        if value_c == target.text {
            self.emit_guards(value);
            return;
        }
        if !self.needs_drop(ty) {
            self.emit_guards(value);
            let value_c = self.store_text(value, ty);
            let source = self.array_source_address(value, &value_c);
            self.push(&format!(
                "    memcpy({}, {source}, {});",
                target.address, target.bytes
            ));
            return;
        }
        if !reads_target {
            // The value does not read the target, so the old value is released
            // before the value is evaluated.
            self.emit_drop_expr(&target.text, ty);
            self.emit_guards(value);
            let value_c = self.store_text(value, ty);
            let source = self.array_source_address(value, &value_c);
            self.push(&format!(
                "    memcpy({}, {source}, {});",
                target.address, target.bytes
            ));
            self.emit_arm_source_moves(value, ty);
            return;
        }
        // The value reads the target, so the old value must survive until the
        // bytes have been copied. Save it by byte copy, copy the new value in,
        // then release the saved old value.
        let save = self.unique_local("__cl_save");
        self.push("    {");
        self.push(&format!(
            "        {} = {};",
            declarator(ty, &save, &self.sema.table),
            self.empty_state(ty)
        ));
        self.push(&format!(
            "        memcpy(&({save}), {}, {});",
            target.address, target.bytes
        ));
        self.emit_guards(value);
        let value_c = self.store_text(value, ty);
        let source = self.array_source_address(value, &value_c);
        self.push(&format!(
            "        memcpy({}, {source}, {});",
            target.address, target.bytes
        ));
        self.emit_arm_source_moves(value, ty);
        self.emit_drop_expr(&save, ty);
        self.push("    }");
    }

    /// Emits a fixed-array conditional into a rendered target.
    ///
    /// The condition is evaluated once and each arm runs inside its own
    /// branch, so an arm's guards and calls execute only when the arm is
    /// chosen. A duplicable destination takes a byte copy per arm. An owned
    /// destination saves its old value first: it stays alive while the chosen
    /// arm is stored, an arm that names the target copies the saved value
    /// back, and the saved value is released once afterwards.
    pub(super) fn emit_fixed_array_conditional_assign(
        &mut self,
        target: &ArrayTarget,
        ty: &Ty,
        condition: &Value,
        then_value: &Value,
        else_value: &Value,
    ) {
        if !self.needs_drop(ty) {
            self.emit_guards(condition);
            let condition_c = self.value(condition);
            self.push(&format!("    if ({condition_c}) {{"));
            self.emit_fixed_array_assign_arm(target, ty, then_value, None);
            self.push("    } else {");
            self.emit_fixed_array_assign_arm(target, ty, else_value, None);
            self.push("    }");
            return;
        }
        // The old target must survive until the chosen arm has been stored,
        // because an arm may name the target itself. The old bytes are saved
        // first and released after the branch; an arm that names the target
        // copies the saved value back, which empties the save so its release
        // is a no-op.
        let save = self.unique_local("__cl_save");
        self.push("    {");
        self.push(&format!(
            "        {} = {};",
            declarator(ty, &save, &self.sema.table),
            self.empty_state(ty)
        ));
        self.push(&format!(
            "        memcpy(&({save}), {}, {});",
            target.address, target.bytes
        ));
        self.emit_guards(condition);
        let condition_c = self.value(condition);
        self.push(&format!("        if ({condition_c}) {{"));
        self.emit_fixed_array_assign_arm(target, ty, then_value, Some(&save));
        self.push("        } else {");
        self.emit_fixed_array_assign_arm(target, ty, else_value, Some(&save));
        self.push("        }");
        self.emit_drop_expr(&save, ty);
        self.push("    }");
    }

    /// Emits one arm of a fixed-array conditional assignment.
    ///
    /// The arm is stored into `target`; `save` holds the destination's old
    /// value while the branch runs, so an arm that names the destination can
    /// restore it. A nested conditional recurses, and a call writes into a
    /// fresh statement-local before its bytes are copied in, so a callee that
    /// reads the destination still sees the old value.
    fn emit_fixed_array_assign_arm(
        &mut self,
        target: &ArrayTarget,
        ty: &Ty,
        arm: &Value,
        save: Option<&str>,
    ) {
        if let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &arm.kind
        {
            self.emit_guards(condition);
            let condition_c = self.value(condition);
            self.push(&format!("                if ({condition_c}) {{"));
            self.emit_fixed_array_assign_arm(target, ty, then_value, save);
            self.push("                } else {");
            self.emit_fixed_array_assign_arm(target, ty, else_value, save);
            self.push("                }");
            return;
        }
        if self.value(arm) == target.text {
            // The arm names the destination. Without a save the destination
            // already holds its own value; with one, the old value is restored
            // and the save emptied so the later release is a no-op.
            if let Some(save) = save {
                self.push(&format!(
                    "                memcpy({}, &({save}), {});",
                    target.address, target.bytes
                ));
                self.emit_move_zero(save, ty);
            }
            return;
        }
        match &arm.kind {
            ValueKind::ArrayLit { elements, fill } => {
                self.emit_array_assign_to(target, ty, elements, fill);
            }
            ValueKind::Call {
                callee,
                type_args,
                args,
                receiver,
            } => {
                let temp = self.unique_local("__cl_arm");
                self.push(&format!(
                    "                {} = {};",
                    declarator(ty, &temp, &self.sema.table),
                    self.empty_state(ty)
                ));
                if let Some(receiver) = receiver.as_deref() {
                    self.emit_guards(receiver);
                }
                for arg in args {
                    self.emit_guards(arg);
                }
                let call = self.call_with_destination(
                    callee,
                    type_args,
                    args,
                    receiver.as_deref(),
                    &temp,
                );
                self.push(&format!("                {call};"));
                self.push(&format!(
                    "                memcpy({}, &({temp}), {});",
                    target.address, target.bytes
                ));
                self.emit_move_zero(&temp, ty);
            }
            ValueKind::ClosureCall { callable, args } => {
                let temp = self.unique_local("__cl_arm");
                self.push(&format!(
                    "                {} = {};",
                    declarator(ty, &temp, &self.sema.table),
                    self.empty_state(ty)
                ));
                self.emit_guards(callable);
                for arg in args {
                    self.emit_guards(arg);
                }
                let call = self.closure_call_with_destination(callable, args, Some(&temp));
                self.push(&format!("                {call};"));
                self.push(&format!(
                    "                memcpy({}, &({temp}), {});",
                    target.address, target.bytes
                ));
                self.emit_move_zero(&temp, ty);
            }
            _ => {
                self.emit_guards(arm);
                let arm_c = self.store_text(arm, ty);
                let source = self.array_source_address(arm, &arm_c);
                self.push(&format!(
                    "                memcpy({}, {source}, {});",
                    target.address, target.bytes
                ));
                // An owned arm is a move: the bytes travel to the destination
                // and the source is emptied, so only the chosen arm is moved.
                if self.needs_drop(ty) {
                    let source_ty = self.substitute(&arm.ty);
                    if self.needs_drop(&source_ty) {
                        if let ValueKind::Place(place) = &arm.kind {
                            self.emit_move_zero_place(place, &arm_c, &source_ty);
                        } else {
                            self.emit_move_zero(&arm_c, &source_ty);
                        }
                    }
                }
            }
        }
    }

    /// True when a conditional of structs needs per-arm store forms.
    ///
    /// A C `?:` on two struct values copies bytes, which cannot initialize a
    /// fixed-array field. The value must be stored per arm instead, with any
    /// arm that is a struct literal storing its fields. An owned struct takes
    /// the owned branch store, so this predicate is only consulted for a
    /// duplicable struct.
    pub(super) fn conditional_struct_needs_stores(
        &self,
        ty: &Ty,
        then_value: &Value,
        else_value: &Value,
    ) -> bool {
        if !matches!(&ty.kind, TyKind::Struct(..)) {
            return false;
        }
        [then_value, else_value].into_iter().any(|arm| {
            matches!(
                &arm.kind,
                ValueKind::StructLit { fields, .. } if self.struct_literal_needs_stores(fields)
            )
        })
    }

    /// Emits a duplicable struct conditional through one branch per arm.
    ///
    /// The condition runs once. An arm that is a struct literal whose fields
    /// need stores is built in a fresh local and moved in; every other arm is
    /// a plain struct assignment, which C allows for a type with no ownership.
    pub(super) fn emit_struct_conditional_assign(
        &mut self,
        target_c: &str,
        ty: &Ty,
        condition: &Value,
        then_value: &Value,
        else_value: &Value,
    ) {
        self.emit_guards(condition);
        let condition_c = self.value(condition);
        self.push(&format!("    if ({condition_c}) {{"));
        self.emit_struct_value_store_arm(target_c, ty, then_value);
        self.push("    } else {");
        self.emit_struct_value_store_arm(target_c, ty, else_value);
        self.push("    }");
    }

    /// Stores one arm of a duplicable struct conditional into the target.
    ///
    /// A struct literal that needs stores is built in a fresh local first, so
    /// a field that reads the target still sees the old value. Every other arm
    /// is a plain struct assignment.
    fn emit_struct_value_store_arm(&mut self, target_c: &str, ty: &Ty, arm: &Value) {
        if let ValueKind::StructLit { fields, .. } = &arm.kind {
            if self.struct_literal_needs_stores(fields) {
                let local = self.unique_local("__cl_arm");
                self.push("        {");
                self.push(&format!(
                    "            {} = {};",
                    declarator(ty, &local, &self.sema.table),
                    self.empty_state(ty)
                ));
                self.emit_struct_literal_fields(&local, ty, fields);
                self.push(&format!("            {target_c} = {local};"));
                self.push("        }");
                return;
            }
        }
        self.emit_guards(arm);
        let arm_c = self.store_text(arm, ty);
        self.push(&format!("        {target_c} = {arm_c};"));
    }

    /// True when a literal holds a fixed-array field with no C initializer form.
    ///
    /// A C aggregate initializer cannot name the result of a call that writes
    /// through the fixed-array hidden out-parameter, and a fixed-array
    /// conditional needs one store per path. A literal with such a field takes
    /// the per-field store form, exactly as the direct store spelling does. A
    /// nested literal renders inside the same initializer, so its own fields
    /// are searched too.
    fn fixed_array_field_needs_stores(&self, fields: &[(String, Value)]) -> bool {
        fields.iter().any(|(_, field)| {
            let field_ty = self.substitute(&field.ty);
            if field_ty.is_fixed_array()
                && matches!(
                    field.kind,
                    ValueKind::Call { .. }
                        | ValueKind::ClosureCall { .. }
                        | ValueKind::Conditional { .. }
                )
            {
                return true;
            }
            if let ValueKind::StructLit { fields, .. } = &field.kind {
                return self.fixed_array_field_needs_stores(fields);
            }
            // An array-literal element that carries a call-filled field takes
            // the same statement-form store route: the brace list would call
            // the function without its hidden fixed-array destination.
            if let ValueKind::ArrayLit { elements, fill } = &field.kind {
                return elements
                    .iter()
                    .any(|element| self.array_element_needs_stores(element))
                    || fill
                        .as_deref()
                        .is_some_and(|fill| self.array_element_needs_stores(fill));
            }
            false
        })
    }

    /// True when an array element carries a literal whose fields need stores.
    ///
    /// The element is judged as a literal at its own position, so a nested
    /// literal and each arm of an element conditional are searched the same
    /// way.
    fn array_element_needs_stores(&self, value: &Value) -> bool {
        match &value.kind {
            ValueKind::StructLit { fields, .. } => self.fixed_array_field_needs_stores(fields),
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.array_element_needs_stores(then_value)
                    || self.array_element_needs_stores(else_value)
            }
            _ => false,
        }
    }

    /// True when a struct literal needs the per-field store form.
    ///
    /// A C aggregate initializer cannot name a fixed-array field, and it
    /// cannot move an owned conditional's chosen arm; a nested literal that
    /// itself needs stores has no initializer form either. Any of those makes
    /// the whole literal store every field through statements, in source
    /// order, which a C initializer cannot mix with without reordering
    /// side-effecting fields.
    pub(super) fn struct_literal_needs_stores(&self, fields: &[(String, Value)]) -> bool {
        fields.iter().any(|(_, field)| {
            let field_ty = self.substitute(&field.ty);
            if field_ty.is_fixed_array()
                || (self.needs_drop(&field_ty)
                    && matches!(field.kind, ValueKind::Conditional { .. }))
            {
                return true;
            }
            // A consumed projection or a nested literal that carries one has
            // its fill and move-out deferred, so the field must store through
            // statements to emit them at its turn.
            if has_deferred_fill(field) {
                return true;
            }
            if let ValueKind::StructLit { fields, .. } = &field.kind {
                return self.struct_literal_needs_stores(fields);
            }
            false
        })
    }

    /// Emits a struct literal as per-field stores into an owned destination.
    ///
    /// Every field is stored in source order, because a C aggregate initializer
    /// cannot mix with later stores without reordering side-effecting fields.
    /// A target that no field reads is dropped first. When a field's value
    /// reads the target, the whole literal is built in a fresh local first, so
    /// every field still reads the old value; the old target is released after
    /// the fields have been evaluated, and the local is then moved in.
    fn emit_struct_literal_store(
        &mut self,
        target: &Place,
        ty: &Ty,
        fields: &[(String, Value)],
    ) {
        let target_c = self.place(target);
        let reads_target = fields
            .iter()
            .any(|(_, field)| value_reads_base(field, target));
        if !reads_target {
            self.emit_drop(target);
            self.emit_struct_literal_fields(&target_c, ty, fields);
            return;
        }
        let local = self.unique_local("__cl_lit");
        self.push("    {");
        self.push(&format!(
            "        {} = {};",
            declarator(ty, &local, &self.sema.table),
            self.empty_state(ty)
        ));
        self.emit_struct_literal_fields(&local, ty, fields);
        self.emit_drop(target);
        self.push(&format!("        {target_c} = {local};"));
        self.push("    }");
    }

    /// Stores every field of a struct literal into a rendered C lvalue, in
    /// source order.
    ///
    /// A fixed-array field goes through the fixed-array store core, because C
    /// cannot assign an array field. A nested literal that needs stores is
    /// stored field by field into the field lvalue. An owned conditional field
    /// branches through [`Self::emit_conditional_move_assign`] so the chosen
    /// arm moves into the field; every other field is stored as a plain
    /// assignment, and a moved source is emptied.
    pub(super) fn emit_struct_literal_fields(
        &mut self,
        target_c: &str,
        _ty: &Ty,
        fields: &[(String, Value)],
    ) {
        for (name, field) in fields {
            let field_ty = self.substitute(&field.ty);
            let field_c = format!("{target_c}.{name}");
            if field_ty.is_fixed_array() {
                let field_target = ArrayTarget::object(&field_c);
                self.emit_fixed_array_assign_to(&field_target, &field_ty, field, false);
                continue;
            }
            if let ValueKind::StructLit { fields: inner, .. } = &field.kind {
                if self.struct_literal_needs_stores(inner) {
                    self.emit_struct_literal_fields(&field_c, &field_ty, inner);
                    continue;
                }
            }
            if self.needs_drop(&field_ty) {
                if let ValueKind::Conditional {
                    condition,
                    then_value,
                    else_value,
                } = &field.kind
                {
                    self.emit_conditional_move_assign(
                        &field_c,
                        &field_ty,
                        condition,
                        then_value,
                        else_value,
                    );
                    continue;
                }
            }
            self.emit_guards(field);
            let lowered = self.store_text(field, &field_ty);
            self.push(&format!("    {field_c} = {lowered};"));
            if self.needs_drop(&field_ty) {
                self.emit_arm_source_moves(field, &field_ty);
            }
        }
    }

    /// Emits an owned conditional store through one branch per arm.
    ///
    /// The destination is saved first, because an arm may name it: that arm
    /// restores the saved value and empties the save, so the single release
    /// after the branch is a no-op. A fresh destination saves its empty state,
    /// which releases nothing. Only the chosen arm's source is evaluated and
    /// emptied, so the destination owns exactly one value.
    pub(super) fn emit_conditional_move_assign(
        &mut self,
        target_c: &str,
        ty: &Ty,
        condition: &Value,
        then_value: &Value,
        else_value: &Value,
    ) {
        let save = self.unique_local("__cl_save");
        self.push("    {");
        self.push(&format!(
            "        {} = {};",
            declarator(ty, &save, &self.sema.table),
            target_c
        ));
        self.emit_conditional_move_store(
            target_c,
            ty,
            condition,
            then_value,
            else_value,
            Some(&save),
        );
        self.emit_drop_expr(&save, ty);
        self.push("    }");
    }

    /// Emits the branch of an owned conditional store into a rendered lvalue.
    ///
    /// The condition is evaluated once before the branch. Each arm is emitted
    /// by [`Self::emit_conditional_move_arm`]; a nested conditional recurses
    /// through the same rule, so the arm that runs is the only one evaluated.
    fn emit_conditional_move_store(
        &mut self,
        target_c: &str,
        ty: &Ty,
        condition: &Value,
        then_value: &Value,
        else_value: &Value,
        save: Option<&str>,
    ) {
        self.emit_guards(condition);
        // A condition whose evaluation consumed the destination already
        // transferred the old value to the callee, so the saved old value must
        // be emptied rather than released again.
        if let Some(save) = save {
            if self.arm_consumes_target(condition, target_c) {
                self.emit_move_zero(save, ty);
            }
        }
        let condition_c = self.value(condition);
        self.push(&format!("        if ({condition_c}) {{"));
        self.emit_conditional_move_arm(target_c, ty, then_value, save);
        self.push("        } else {");
        self.emit_conditional_move_arm(target_c, ty, else_value, save);
        self.push("        }");
    }

    /// Emits one arm of an owned conditional store.
    ///
    /// A call arm assigns its result directly, because the call produces a
    /// fresh value. A deferred call temporary is moved into the destination and
    /// emptied, so the statement-end release is a no-op. A place arm is copied
    /// and then emptied, so only the chosen arm is moved. An arm that names the
    /// destination restores the saved value when a save exists.
    fn emit_conditional_move_arm(
        &mut self,
        target_c: &str,
        ty: &Ty,
        arm: &Value,
        save: Option<&str>,
    ) {
        if let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &arm.kind
        {
            self.emit_guards(condition);
            // A condition whose evaluation consumed the destination already
            // transferred the old value to the callee.
            if let Some(save) = save {
                if self.arm_consumes_target(condition, target_c) {
                    self.emit_move_zero(save, ty);
                }
            }
            let condition_c = self.value(condition);
            self.push(&format!("            if ({condition_c}) {{"));
            self.emit_conditional_move_arm(target_c, ty, then_value, save);
            self.push("            } else {");
            self.emit_conditional_move_arm(target_c, ty, else_value, save);
            self.push("            }");
            return;
        }
        if self.value(arm) == target_c {
            if let Some(save) = save {
                self.push(&format!("            {target_c} = {save};"));
                self.emit_move_zero(save, ty);
            }
            return;
        }
        // A struct-literal arm is built in a fresh local through per-field
        // stores, so a conditional field evaluates its condition once and a
        // target-reading condition still sees the old destination. A
        // fixed-array field stores through the same core as an owned
        // conditional field, because a C aggregate initializer cannot name it.
        if let ValueKind::StructLit { fields, .. } = &arm.kind {
            let local = self.unique_local("__cl_arm");
            self.push("            {");
            self.push(&format!(
                "                {} = {};",
                declarator(ty, &local, &self.sema.table),
                self.empty_state(ty)
            ));
            self.emit_struct_literal_fields(&local, ty, fields);
            self.push(&format!("                {target_c} = {local};"));
            self.push("            }");
            return;
        }
        match &arm.kind {
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. } => {
                self.emit_guards(arm);
                let call = self.value(arm);
                self.push(&format!("            {target_c} = {call};"));
            }
            ValueKind::Materialize { .. } => {
                self.emit_guards(arm);
                let temp = self.value(arm);
                self.push(&format!("            {target_c} = {temp};"));
                self.emit_move_zero(&temp, ty);
                // An arm whose call consumed the destination already
                // transferred the old value to the callee, so the saved old
                // value must be emptied rather than released again.
                if let Some(save) = save {
                    if self.arm_consumes_target(arm, target_c) {
                        self.emit_move_zero(save, ty);
                    }
                }
            }
            _ => {
                self.emit_guards(arm);
                let arm_c = self.store_text(arm, ty);
                self.push(&format!("            {target_c} = {arm_c};"));
                self.emit_arm_source_moves(arm, ty);
            }
        }
    }

    /// True when evaluating a value consumes the destination lvalue through a
    /// deferred `take` move-out.
    ///
    /// The scan walks the whole subtree, because a `take` call inside a
    /// condition or a nested arm consumes the destination before the branch
    /// that stores the new value runs. The comparison is on the rendered C
    /// lvalue, so it matches the branch store's own target spelling.
    fn arm_consumes_target(&self, value: &Value, target_c: &str) -> bool {
        match &value.kind {
            ValueKind::Materialize {
                value: inner,
                move_outs,
                ..
            } => {
                move_outs.iter().any(|place| self.place(place) == target_c)
                    || self.arm_consumes_target(inner, target_c)
            }
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.arm_consumes_target(condition, target_c)
                    || self.arm_consumes_target(then_value, target_c)
                    || self.arm_consumes_target(else_value, target_c)
            }
            ValueKind::Member { base, .. } => self.arm_consumes_target(base, target_c),
            ValueKind::Index { base, index } => {
                self.arm_consumes_target(base, target_c)
                    || self.arm_consumes_target(index, target_c)
            }
            ValueKind::Unary { operand, .. } => self.arm_consumes_target(operand, target_c),
            ValueKind::Binary { left, right, .. } => {
                self.arm_consumes_target(left, target_c)
                    || self.arm_consumes_target(right, target_c)
            }
            ValueKind::Cast { value: inner, .. } => self.arm_consumes_target(inner, target_c),
            ValueKind::Call { args, receiver, .. } => {
                receiver
                    .as_deref()
                    .is_some_and(|receiver| self.arm_consumes_target(receiver, target_c))
                    || args
                        .iter()
                        .any(|arg| self.arm_consumes_target(arg, target_c))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .any(|arg| self.arm_consumes_target(arg, target_c)),
            ValueKind::StructLit { fields, .. } => fields
                .iter()
                .any(|(_, field)| self.arm_consumes_target(field, target_c)),
            ValueKind::ArrayLit { elements, fill } => {
                elements
                    .iter()
                    .any(|element| self.arm_consumes_target(element, target_c))
                    || fill
                        .as_deref()
                        .is_some_and(|fill| self.arm_consumes_target(fill, target_c))
            }
            ValueKind::Template(pieces) => pieces.iter().any(|piece| match piece {
                ValuePiece::Literal(_) => false,
                ValuePiece::Placeholder(inner) => self.arm_consumes_target(inner, target_c),
            }),
            ValueKind::Closure { captures, .. } => captures
                .iter()
                .any(|capture| self.arm_consumes_target(capture, target_c)),
            ValueKind::ClosureCall { callable, args } => {
                self.arm_consumes_target(callable, target_c)
                    || args
                        .iter()
                        .any(|arg| self.arm_consumes_target(arg, target_c))
            }
            _ => false,
        }
    }

    /// Empties the storage an arm's rendered value copied into the destination.
    ///
    /// A place or compiler temporary is copied and then zeroed, so the
    /// destination is the only owner. A composite value is rendered as an
    /// initializer that copies every storage-reading field, so each such source
    /// is emptied after the store; a fresh call or literal field has no caller
    /// storage to empty. A nested conditional empties only the arm its
    /// condition selects. This is what makes the statement-end release of a
    /// copied transfer temporary a no-op.
    pub(super) fn emit_arm_source_moves(&mut self, arm: &Value, ty: &Ty) {
        // A null-shaped arm lowers to the destination's own empty value, so it
        // read no storage the store must empty.
        if self.carries_null_shape(arm) && self.carries_null_value(ty) {
            return;
        }
        match &arm.kind {
            ValueKind::Place(place) => {
                let source = self.value(arm);
                self.emit_move_zero_place(place, &source, ty);
            }
            ValueKind::Temp(_) | ValueKind::Materialize { .. } => {
                let source = self.value(arm);
                self.emit_move_zero(&source, ty);
            }
            // A raw-deref read used as a store value moves the pointee: the
            // source is emptied through its own address, exactly as a named
            // place source is emptied, so only the destination owns the value.
            ValueKind::Unary {
                op: UnaryOp::Deref,
                operand,
            } => {
                let source = format!("*({})", self.value(operand));
                self.emit_move_zero(&source, ty);
            }
            // A projection is a move source only when it roots at storage the
            // caller owns; a field of a fresh call result has no caller
            // storage to empty.
            ValueKind::Member { .. } | ValueKind::Index { .. }
                if arm_has_storage_root(arm) =>
            {
                let source = self.value(arm);
                self.emit_move_zero(&source, ty);
            }
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    let field_ty = self.substitute(&field.ty);
                    if self.needs_drop(&field_ty) {
                        self.emit_arm_source_moves(field, &field_ty);
                    }
                }
            }
            _ => {}
        }
    }

    /// Emits a fixed-array assignment whose right-hand side is a call.
    ///
    /// A by-value fixed-array call writes through a hidden trailing
    /// out-parameter, so the result needs storage before the target is
    /// released. The call runs into a fresh statement-local temporary first,
    /// because the callee may read the target (`s = dup(borrow s)`); the old
    /// target is then released and the temporary is moved into it by byte
    /// copy, mirroring a fixed-array [`Statement::Move`]. The temporary is
    /// zeroed afterwards so its handles are not released a second time.
    fn emit_fixed_array_call_assign(
        &mut self,
        span: crate::span::Span,
        target: &Place,
        ty: &Ty,
        callee: &str,
        type_args: &[Ty],
        args: &[ir::Value],
        receiver: Option<&ir::Value>,
    ) {
        self.emit_line_directive(span);
        // An argument's guards (bounds, division, float, slice) run before the
        // call, exactly as they do on the ordinary call path; the call may only
        // execute once its arguments are known to be valid. A callable field's
        // receiver is guarded the same way, so a runtime receiver index traps
        // before the call.
        if let Some(receiver) = receiver {
            self.emit_guards(receiver);
        }
        for arg in args {
            self.emit_guards(arg);
        }
        let temp = self.unique_local("__cl_call");
        self.push("    {");
        self.push(&format!(
            "        {} = {};",
            declarator(ty, &temp, &self.sema.table),
            self.empty_state(ty)
        ));
        let call = self.call_with_destination(callee, type_args, args, receiver, &temp);
        self.push(&format!("        {call};"));
        self.emit_reassigned_target_drop(target);
        let target_c = self.place(target);
        let target = self.array_target(target, &target_c);
        self.push(&format!(
            "        memcpy({}, &({temp}), {});",
            target.address, target.bytes
        ));
        self.emit_move_zero(&temp, ty);
        self.push("    }");
    }

    /// True when a possibly-null owned handle reaches a view destination.
    ///
    /// The view constructor dereferences the handle, so a null one must select
    /// the destination's empty view instead of widening: the guard applies
    /// exactly where the value can be null and the destination is a view.
    pub(super) fn needs_view_guard(&self, value_ty: &Ty, target: &Ty) -> bool {
        let value_ty = self.substitute(value_ty);
        value_ty.nullable
            && is_struct_view(&self.substitute(target))
            && matches!(&value_ty.kind, TyKind::Handle(name) if name == "String")
    }

    /// True when re-reading a value reads storage that already exists.
    ///
    /// A place, a statement temporary, and a deferred materialization all name
    /// storage the program already holds, and a field or element of one reads
    /// that storage again. A field or element of a fresh call or closure
    /// result re-runs the call on every read, so a guard that reads it twice
    /// would evaluate the call twice and keep the first result; a conditional
    /// re-reads the arm it selected, so it is stable exactly when every
    /// reachable arm is.
    pub(super) fn reads_stable_storage(&self, value: &Value) -> bool {
        match &value.kind {
            // The null literal re-reads nothing, so testing it twice costs
            // nothing and cannot change its value.
            ValueKind::Null => true,
            ValueKind::Place(_) | ValueKind::Temp(_) | ValueKind::Materialize { .. } => true,
            ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
                self.reads_stable_storage(base)
            }
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                self.reads_stable_storage(then_value) && self.reads_stable_storage(else_value)
            }
            _ => false,
        }
    }

    /// Renders a nullable owned handle at a view destination under its guard.
    ///
    /// The view constructor would dereference a null handle, so a source that
    /// can be null tests it first and selects the destination's empty view:
    /// `(handle ? cl_string_view(handle) : cl_zero_ClStr())`. The test reads
    /// the source twice, so only a stable source spells it; a source that
    /// reads a fresh value — a call, or a field of one — binds to a temporary
    /// before the store instead. Every other source keeps the plain widening.
    /// A flat store already emitted the source's operand guards as statements,
    /// so the test reads the plain place; a caller with no statement position
    /// for them calls [`Self::guarded_view_widening_inline`] instead.
    pub(super) fn guarded_view_widening(&self, value: &Value, target: &Ty) -> Option<String> {
        self.widening_with_guards(value, target, false)
    }

    /// Renders the same guard with the source's operand guards inline.
    ///
    /// A conditional arm and a call argument are evaluated inside an
    /// expression, so they have no statement position for the source's
    /// materialize fill or traps: the guard carries them once, ahead of the
    /// test. A flat store's caller already emitted them as statements.
    pub(super) fn guarded_view_widening_inline(
        &self,
        value: &Value,
        target: &Ty,
    ) -> Option<String> {
        self.widening_with_guards(value, target, true)
    }

    /// Renders the guarded view widening, optionally with the guards inline.
    fn widening_with_guards(
        &self,
        value: &Value,
        target: &Ty,
        inline_guards: bool,
    ) -> Option<String> {
        if !self.reads_stable_storage(value) || !self.needs_view_guard(&value.ty, target) {
            return None;
        }
        let (text, guards) = self.guarded_parts(value);
        let guarded = format!(
            "({text} ? cl_string_view({text}) : {})",
            self.null_value(target)
        );
        Some(if inline_guards && !guards.is_empty() {
            format!("(({}), {guarded})", guards.join(", "))
        } else {
            guarded
        })
    }

    /// Renders a value for a store into a target of type `ty`.
    ///
    /// An owned `String` widens implicitly to the `str` view the target holds,
    /// through the same shim view the call-argument widening uses, and a
    /// view's pointer member converts to the `cstring` the target declares.
    /// Every other store renders the value directly.
    pub(super) fn store_text(&self, value: &Value, ty: &Ty) -> String {
        // A null literal is typed by its destination, and a struct-typed view
        // destination needs its zero struct rather than a null pointer.
        if matches!(value.kind, ValueKind::Null) {
            return self.null_value(ty);
        }
        // A conditional at a null-capable destination renders per arm: each
        // arm keeps its own nullness (an owned arm widens to the view, a null
        // arm carries the destination's empty value). A conditional whose
        // arms are all null carries the null literal's unknown type, so its
        // empty spelling must come from the destination; and rendering an
        // owned arm as one value and widening the whole conditional after
        // would construct a view over whichever arm ran, including the null
        // one.
        if matches!(value.kind, ValueKind::Conditional { .. })
            && self.carries_null_value(ty)
        {
            return self.conditional_arm(value, ty);
        }
        // A local inferred from `null` carries the null literal's shape, so it
        // lowers to the destination's empty value too. A conditional renders
        // per arm (its own path above), and a copy whose destination still is
        // the shape keeps the value's own rendering.
        if self.is_null_shaped(value)
            && !matches!(value.kind, ValueKind::Conditional { .. })
            && !crate::types::is_null_shaped(&self.substitute(ty))
            && self.carries_null_value(ty)
        {
            let empty = self.null_value(ty);
            // A value that is not the literal's own carry — a lambda call
            // whose inferred return is the shape, for example — still
            // computes something: it runs first, and the result is the
            // destination's empty value. A conditional renders per arm
            // instead, so it keeps its own path.
            if self.carries_null_shape(value) {
                return empty;
            }
            return format!("({}, {empty})", self.value(value));
        }
        let value_ty = self.substitute(&value.ty);
        // A possibly-null handle keeps its nullness through the widening: a
        // view constructed over the null handle would dereference it, so the
        // null guard selects the destination's empty view instead. The guard
        // reads the source twice, so only a place-rooted source spells it; a
        // fresh call root binds into an owner before a view store, and every
        // other value keeps the plain constructor.
        if let Some(guarded) = self.guarded_view_widening(value, ty) {
            return guarded;
        }
        let text = self.value(value);
        let text = self.widen_text_view(text, &value_ty, ty);
        // A view's pointer member is a `const u8 *` in the C while a
        // `cstring` destination is a `char *`: the store spells the
        // conversion the declaration expects. The member's own checked type
        // (a byte pointer) selects the conversion, so every byte pointer
        // reaching a `cstring` destination converts.
        if matches!(ty.kind, TyKind::CString)
            && matches!(self.resolved_value_ty(value).kind, TyKind::Raw(_))
        {
            return format!("(char *)({text})");
        }
        text
    }

    /// Wraps a rendered owned `String` in the shim view constructor.
    ///
    /// An owned `String` widens implicitly to the `str` view in a value
    /// position, and the C spells the widening through the shim view. A store
    /// and a conditional arm both need the wrapper: one C `?:` must carry one
    /// type in both arms, so the owned arm takes the same shape the store
    /// spells.
    pub(super) fn widen_text_view(&self, text: String, value_ty: &Ty, target: &Ty) -> String {
        if matches!(target.kind, TyKind::Str | TyKind::StaticStr)
            && matches!(&value_ty.kind, TyKind::Handle(name) if name == "String")
        {
            return format!("cl_string_view({text})");
        }
        text
    }

    /// Emits an array literal as element-by-element assignments to a place.
    fn emit_array_assign(
        &mut self,
        target: &Place,
        ty: &Ty,
        elements: &[ir::Value],
        fill: &Option<Box<ir::Value>>,
    ) {
        let target_c = self.place(target);
        let target = self.array_target(target, &target_c);
        self.emit_array_assign_to(&target, ty, elements, fill);
    }

    /// Emits an array literal as element-by-element assignments to a target.
    ///
    /// The target carries the C spelling of the array, so the same lowering
    /// can write a hoisted local, an adjusted array parameter, or the hidden
    /// out-parameter that carries a by-value fixed-array return. C cannot
    /// assign an array in one statement, so a multi-extent type is filled one
    /// dimension deeper: an element that is itself an array literal recurses
    /// against the indexed sub-target instead of writing a rejected brace
    /// list.
    pub(super) fn emit_array_assign_to(
        &mut self,
        target: &ArrayTarget,
        ty: &Ty,
        elements: &[ir::Value],
        fill: &Option<Box<ir::Value>>,
    ) {
        let TyKind::Array { element, extents } = &ty.kind else {
            return;
        };
        let extent = extents.first().and_then(|size| *size).unwrap_or(0);
        // A multi-extent type keeps its base element and every extent, so the
        // element type of the outer dimension is the array of the remaining
        // extents. A one-extent array has a scalar element.
        let nested_ty = (extents.len() > 1)
            .then(|| Ty::array((**element).clone(), extents[1..].to_vec()));
        for (index, value) in elements.iter().enumerate() {
            let sub = target.element(index);
            if let Some(element_ty) = &nested_ty {
                match &value.kind {
                    ValueKind::ArrayLit { elements, fill } => {
                        self.emit_array_assign_to(&sub, element_ty, elements, fill);
                    }
                    // A whole row that is not a literal has no scalar form; it
                    // travels through the fixed-array store core, one dimension
                    // in.
                    _ => self.emit_fixed_array_assign_to(&sub, element_ty, value, false),
                }
                continue;
            }
            // A struct-literal element whose fields need stores has no C
            // initializer form; it is stored per field into the element.
            if let ValueKind::StructLit { fields, .. } = &value.kind {
                if self.struct_literal_needs_stores(fields) {
                    self.emit_struct_literal_fields(&sub.text, element, fields);
                    continue;
                }
            }
            // An owned conditional element moves the chosen arm into the
            // element per path; a plain copy would leave the chosen arm owning
            // the same storage, so the element and the arm would both release
            // it. The fill form cannot reach this: a non-duplicable fill value
            // is rejected when the literal is checked.
            if let ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } = &value.kind
            {
                if self.needs_drop(element) {
                    self.emit_conditional_move_assign(
                        &sub.text,
                        element,
                        condition,
                        then_value,
                        else_value,
                    );
                    continue;
                }
                // A duplicable element conditional whose arm is a literal that
                // needs stores takes the per-arm store form too: the C `?:`
                // would carry the literal whose fixed-array field has no
                // initializer form, so each arm stores on its own path.
                if self.conditional_struct_needs_stores(element, then_value, else_value) {
                    self.emit_struct_conditional_assign(
                        &sub.text,
                        element,
                        condition,
                        then_value,
                        else_value,
                    );
                    continue;
                }
            }
            self.emit_guards(value);
            let lowered = self.store_text(value, element);
            self.push(&format!("    {} = {lowered};", sub.text));
            if self.needs_drop(element) {
                self.emit_arm_source_moves(value, element);
            }
        }
        let prefix = elements.len();
        let Some(fill) = fill else {
            return;
        };
        // A fill covers every index the prefix did not: `[a, b; fill]` fills
        // from the prefix length to the extent, and `[; fill]` fills them all.
        if let Some(element_ty) = &nested_ty {
            match &fill.kind {
                ValueKind::ArrayLit { elements, fill } => {
                    for index in prefix..extent {
                        let sub = target.element(index);
                        self.emit_array_assign_to(&sub, element_ty, elements, fill);
                    }
                }
                _ => {
                    for index in prefix..extent {
                        let sub = target.element(index);
                        self.emit_fixed_array_assign_to(&sub, element_ty, fill, false);
                    }
                }
            }
            return;
        }
        self.emit_guards(fill);
        let lowered = self.store_text(fill, element);
        for index in prefix..extent {
            self.push(&format!("    {}[{index}] = {lowered};", target.text));
        }
    }

    /// Zeroes a moved-out source: the null literal for a handle, `cl_zero`
    /// otherwise.
    pub(super) fn emit_move_zero(&mut self, source: &str, ty: &Ty) {
        match &ty.kind {
            TyKind::Handle(_) | TyKind::Raw(_) | TyKind::CString => {
                let null = self.null_literal();
                self.push(&format!("    {source} = {null};"));
            }
            _ => self.push(&format!("    cl_zero(&{source});")),
        }
    }

    /// True when a place is the root of a fixed-array parameter.
    ///
    /// C adjusts an array parameter to a pointer to its first element, so the
    /// parameter name is the array's address: `&(name)` would address the
    /// pointer slot and `sizeof(name)` would be the pointer's size. A
    /// projection reaches a real element or row object, which needs no
    /// adjustment, so only the bare root counts.
    pub(super) fn is_array_param_place(&self, place: &Place) -> bool {
        place.projections.is_empty()
            && matches!(place.base, ir::PlaceBase::Param(_))
            && self.substitute(&place.ty).is_fixed_array()
    }

    /// The store target of a fixed-array place.
    ///
    /// A true array object is addressed with `&(name)` and measured with
    /// `sizeof(name)`. A fixed-array parameter is C-adjusted, so its name
    /// already is the array's address and the byte count comes from the
    /// element type instead.
    fn array_target(&self, place: &Place, c_name: &str) -> ArrayTarget {
        if self.is_array_param_place(place) {
            let ty = self.substitute(&place.ty);
            return ArrayTarget {
                text: c_name.to_owned(),
                address: c_name.to_owned(),
                bytes: self.array_param_bytes(c_name, &ty),
            };
        }
        ArrayTarget::object(c_name)
    }

    /// The address of a fixed-array copy source.
    ///
    /// A place that roots at an adjusted array parameter already names the
    /// array's address; every other source needs the address-of operator.
    pub(super) fn array_source_address(&self, value: &Value, c_name: &str) -> String {
        if let ValueKind::Place(place) = &value.kind {
            if self.is_array_param_place(place) {
                return c_name.to_owned();
            }
        }
        format!("&({c_name})")
    }

    /// The byte count of a fixed array through its C-adjusted parameter name.
    ///
    /// `name[0]` has every extent but the first, so `sizeof(name[0])` times the
    /// first extent is the whole array's size without a `sizeof` on the
    /// adjusted pointer itself.
    pub(super) fn array_param_bytes(&self, c_name: &str, ty: &Ty) -> String {
        let TyKind::Array { extents, .. } = &ty.kind else {
            return format!("sizeof({c_name}[0])");
        };
        let first = extents.first().and_then(|extent| *extent).unwrap_or(1);
        format!("(sizeof({c_name}[0]) * {first})")
    }

    /// Zeroes a moved-out place, walking a parameter's elements when the place
    /// is a C-adjusted fixed-array parameter.
    ///
    /// A parameter name cannot be passed to `cl_zero`: the macro would take
    /// `sizeof` of the adjusted pointer. The element walk zeroes each deepest
    /// element instead, which empties every handle the move transferred.
    pub(super) fn emit_move_zero_place(&mut self, place: &Place, source: &str, ty: &Ty) {
        if self.is_array_param_place(place) {
            let TyKind::Array { element, extents } = &ty.kind else {
                return;
            };
            let dims: Vec<usize> = extents.iter().filter_map(|extent| *extent).collect();
            self.emit_array_param_zero(source, element, &dims);
            return;
        }
        self.emit_move_zero(source, ty);
    }

    /// Emits one `cl_zero` per deepest element of a fixed array.
    fn emit_array_param_zero(&mut self, c_name: &str, element: &Ty, extents: &[usize]) {
        match extents.split_first() {
            None => self.push(&format!("    cl_zero(&{c_name});")),
            Some((head, tail)) => {
                for index in 0..*head {
                    self.emit_array_param_zero(
                        &format!("{c_name}[{index}]"),
                        element,
                        tail,
                    );
                }
            }
        }
    }
}

/// The rendered sides of a fixed-array store.
///
/// `text` names the array for indexing and drops, `address` is the array's
/// address for a `memcpy`, and `bytes` is its byte count. A true array object
/// spells `&(name)` and `sizeof(name)`; a C-adjusted array parameter spells
/// only its name and takes the byte count from the element type, because
/// `sizeof` would measure the adjusted pointer.
pub(super) struct ArrayTarget {
    /// The C expression that names the array.
    text: String,
    /// The C expression that is the array's address.
    pub(super) address: String,
    /// The C expression for the array's byte count.
    pub(super) bytes: String,
}

impl ArrayTarget {
    /// The target of an array object named by `text`.
    pub(super) fn object(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            address: format!("&({text})"),
            bytes: format!("sizeof({text})"),
        }
    }

    /// The target of a C-adjusted array parameter named by `text`.
    ///
    /// The parameter is a pointer to the array's first element, so its name
    /// is already the address; `bytes` comes from the element type, because
    /// `sizeof` would measure the adjusted pointer.
    pub(super) fn parameter(text: &str, bytes: String) -> Self {
        Self {
            text: text.to_owned(),
            address: text.to_owned(),
            bytes,
        }
    }

    /// The target of one element or row inside this array object.
    fn element(&self, index: usize) -> Self {
        Self::object(&format!("{}[{}]", self.text, index))
    }
}

/// The declared locals the emitted C never reads as a value.
///
/// A declaration target and a `sizeof` unused marker already use a local, so
/// only the value positions count: a right-hand side, a call argument, a store,
/// a move source, a dropped place, and a terminator's condition or return. A
/// local reached only from a write is absent from the result, so the emitter
/// can mark it unused and keep the generated C free of
/// `-Wunused-but-set-variable`. `drop_reads` decides whether a drop actually
/// reads its place at this instance: a maybe-owning type that substitutes to a
/// duplicable one emits no drop, so the local stays unread.
fn unread_locals(
    function: &ir::Function,
    drop_reads: impl Fn(&Ty) -> bool,
    null_shape: impl Fn(&Ty) -> bool,
    null_capable: impl Fn(&Ty) -> bool,
) -> BTreeSet<String> {
    let mut read: BTreeSet<String> = BTreeSet::new();
    for block in &function.blocks {
        for instr in &block.statements {
            match &instr.statement {
                Statement::Assign { target, value } => {
                    note_store_value(value, &target.ty, &mut read, &null_shape, &null_capable);
                }
                Statement::Move { target, source } => {
                    // A null-shaped source lowers to the target's own empty
                    // value, so the move does not read it.
                    if !(null_shape(&source.ty)
                        && !null_shape(&target.ty)
                        && null_capable(&target.ty))
                    {
                        note_place(source, &mut read);
                    }
                }
                Statement::StorePointer { pointer, value } => {
                    note_value(pointer, &mut read, &null_shape);
                    note_value(value, &mut read, &null_shape);
                }
                Statement::Call { args, receiver, .. } => {
                    if let Some(receiver) = receiver {
                        note_value(receiver, &mut read, &null_shape);
                    }
                    for arg in args {
                        // A null-shaped argument lowers to its parameter's own
                        // null value, so the call does not read it.
                        if !null_shape(&arg.ty) {
                            note_value(arg, &mut read, &null_shape);
                        }
                    }
                }
                Statement::Discard { value } => note_value(value, &mut read, &null_shape),
                Statement::Guards { checks } => {
                    for check in checks {
                        note_value(check, &mut read, &null_shape);
                    }
                }
                Statement::Drop { place, .. } => {
                    if drop_reads(&place.ty) {
                        note_place(place, &mut read);
                    }
                }
                _ => {}
            }
        }
        match &block.terminator {
            Terminator::Branch { condition, .. } => note_value(condition, &mut read, &null_shape),
            Terminator::Switch { value, arms, .. } => {
                note_value(value, &mut read, &null_shape);
                for (label, _) in arms {
                    note_value(label, &mut read, &null_shape);
                }
            }
            Terminator::Return {
                value: Some(value), ..
            } => {
                note_store_value(value, &function.ret, &mut read, &null_shape, &null_capable);
            }
            _ => {}
        }
    }
    function
        .locals
        .iter()
        .map(|local| local.name.clone())
        .filter(|name| !read.contains(name))
        .collect()
}

/// True when a lowered value reads caller-owned storage a move must empty.
///
/// A place or compiler temporary is storage; a member or element whose base
/// chain roots at one is a projection of that storage. A fresh call result or
/// literal owns no caller storage, so a field of a call result is copied
/// without being emptied.
pub(super) fn arm_has_storage_root(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Place(_) | ValueKind::Temp(_) | ValueKind::Materialize { .. } => true,
        ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
            arm_has_storage_root(base)
        }
        _ => false,
    }
}

/// True when a value reads a deferred fill anywhere in its projection chain.
///
/// A [`ValueKind::Materialize`] is filled by a statement before the construct
/// that reads it, so a C aggregate initializer would hoist the fill ahead of
/// the fields it is written between; the per-field store form keeps the
/// source order. The walk descends projections and nested literals, because a
/// field that projects through a literal (`(Box { v: mk().a }).v`) carries
/// the inner field's deferred fill.
fn has_deferred_fill(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Materialize { .. } => true,
        ValueKind::Member { base, .. } => has_deferred_fill(base),
        ValueKind::Index { base, index } => {
            has_deferred_fill(base) || has_deferred_fill(index)
        }
        ValueKind::StructLit { fields, .. } => {
            fields.iter().any(|(_, field)| has_deferred_fill(field))
        }
        _ => false,
    }
}

/// True when a value reads any part of the place rooted at `target`'s base.
///
/// The check is deliberately broad: any projection of the same base counts,
/// because releasing the target would invalidate every projection of it.
fn value_reads_base(value: &Value, target: &Place) -> bool {
    match &value.kind {
        ValueKind::Place(place) => place.base == target.base,
        ValueKind::Member { base, .. } => value_reads_base(base, target),
        ValueKind::Index { base, index } => {
            value_reads_base(base, target) || value_reads_base(index, target)
        }
        ValueKind::Unary { operand, .. } => value_reads_base(operand, target),
        ValueKind::Binary { left, right, .. } => {
            value_reads_base(left, target) || value_reads_base(right, target)
        }
        ValueKind::Cast { value: inner, .. } => value_reads_base(inner, target),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            value_reads_base(condition, target)
                || value_reads_base(then_value, target)
                || value_reads_base(else_value, target)
        }
        ValueKind::StructLit { fields, .. } => fields
            .iter()
            .any(|(_, field)| value_reads_base(field, target)),
        ValueKind::ArrayLit { elements, fill } => {
            elements
                .iter()
                .any(|element| value_reads_base(element, target))
                || fill
                    .as_deref()
                    .is_some_and(|fill| value_reads_base(fill, target))
        }
        ValueKind::Template(pieces) => pieces.iter().any(|piece| match piece {
            ValuePiece::Literal(_) => false,
            ValuePiece::Placeholder(inner) => value_reads_base(inner, target),
        }),
        ValueKind::Call {
            args, receiver, ..
        } => {
            receiver
                .as_deref()
                .is_some_and(|receiver| value_reads_base(receiver, target))
                || args.iter().any(|arg| value_reads_base(arg, target))
        }
        ValueKind::Intrinsic { args, .. } => args.iter().any(|arg| value_reads_base(arg, target)),
        ValueKind::Materialize { value: inner, .. } => value_reads_base(inner, target),
        ValueKind::Closure { captures, .. } => {
            captures.iter().any(|capture| value_reads_base(capture, target))
        }
        ValueKind::ClosureCall { callable, args } => {
            value_reads_base(callable, target)
                || args.iter().any(|arg| value_reads_base(arg, target))
        }
        ValueKind::Int(_)
        | ValueKind::Float(_)
        | ValueKind::Char(_)
        | ValueKind::Bool(_)
        | ValueKind::Null
        | ValueKind::Str(_)
        | ValueKind::Path(_)
        | ValueKind::Aggregate
        | ValueKind::Temp(_) => false,
    }
}

/// Records the local a place names, if any.
fn note_place(place: &Place, read: &mut BTreeSet<String>) {
    if let ir::PlaceBase::Local(name) = &place.base {
        read.insert(name.clone());
    }
}

/// Records the locals a store or return value reads.
///
/// A conditional renders per arm, so the destination distributes over the
/// arms: a null-shaped arm at a null-capable destination lowers to the
/// destination's own empty value and is not read, while the condition and
/// every other arm keep the ordinary walk. A null-shaped value outside a
/// conditional is skipped the same way.
fn note_store_value(
    value: &Value,
    dest: &Ty,
    read: &mut BTreeSet<String>,
    null_shape: &impl Fn(&Ty) -> bool,
    null_capable: &impl Fn(&Ty) -> bool,
) {
    if let ValueKind::Conditional {
        condition,
        then_value,
        else_value,
    } = &value.kind
    {
        note_value(condition, read, null_shape);
        for arm in [then_value, else_value] {
            note_store_value(arm, dest, read, null_shape, null_capable);
        }
        return;
    }
    if null_shape(&value.ty) && !null_shape(dest) && null_capable(dest) {
        return;
    }
    note_value(value, read, null_shape);
}

/// Records every local named anywhere inside a value.
///
/// `null_shape` marks the null literal's shape: a value that lowers to its
/// destination's own empty value is not read by the emitted code, so it is
/// skipped everywhere the destination is not that shape itself.
fn note_value(value: &Value, read: &mut BTreeSet<String>, null_shape: &impl Fn(&Ty) -> bool) {
    match &value.kind {
        ValueKind::Place(place) => note_place(place, read),
        ValueKind::Member { base, .. } => note_value(base, read, null_shape),
        ValueKind::Index { base, index } => {
            note_value(base, read, null_shape);
            note_value(index, read, null_shape);
        }
        ValueKind::Unary { operand, .. } => note_value(operand, read, null_shape),
        ValueKind::Binary { left, right, .. } => {
            note_value(left, read, null_shape);
            note_value(right, read, null_shape);
        }
        ValueKind::Cast { value: inner, .. } => note_value(inner, read, null_shape),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            note_value(condition, read, null_shape);
            // A null-shaped arm lowers to the destination's own empty value,
            // so the arm is not read.
            for arm in [then_value, else_value] {
                if !null_shape(&arm.ty) {
                    note_value(arm, read, null_shape);
                }
            }
        }
        ValueKind::StructLit { fields, .. } => {
            for (_, field) in fields {
                // A null-shaped field stores the destination's own empty
                // value, so it is not read.
                if !null_shape(&field.ty) {
                    note_value(field, read, null_shape);
                }
            }
        }
        ValueKind::ArrayLit { elements, fill } => {
            for element in elements {
                // A null-shaped element stores the destination's own empty
                // value, so it is not read.
                if !null_shape(&element.ty) {
                    note_value(element, read, null_shape);
                }
            }
            if let Some(fill) = fill {
                if !null_shape(&fill.ty) {
                    note_value(fill, read, null_shape);
                }
            }
        }
        ValueKind::Template(pieces) => {
            for piece in pieces {
                if let ValuePiece::Placeholder(inner) = piece {
                    note_value(inner, read, null_shape);
                }
            }
        }
        ValueKind::Call {
            args, receiver, ..
        } => {
            if let Some(receiver) = receiver {
                note_value(receiver, read, null_shape);
            }
            for arg in args {
                // A null-shaped argument lowers to its parameter's own null
                // value, exactly as the call emission renders it; a
                // conditional keeps its own path.
                // The emission renders the argument through the parameter's
                // type, which the walk does not carry here: a null-shaped
                // argument is skipped, and an unnecessary marker only uses
                // the local harmlessly.
                if !(null_shape(&arg.ty)
                    && !matches!(arg.kind, ValueKind::Conditional { .. }))
                {
                    note_value(arg, read, null_shape);
                }
            }
        }
        ValueKind::Intrinsic { args, .. } => {
            for arg in args {
                note_value(arg, read, null_shape);
            }
        }
        ValueKind::Materialize { value: inner, .. } => note_value(inner, read, null_shape),
        ValueKind::Closure { captures, .. } => {
            for capture in captures {
                note_value(capture, read, null_shape);
            }
        }
        ValueKind::ClosureCall { callable, args } => {
            note_value(callable, read, null_shape);
            for arg in args {
                note_value(arg, read, null_shape);
            }
        }
        ValueKind::Int(_)
        | ValueKind::Float(_)
        | ValueKind::Char(_)
        | ValueKind::Bool(_)
        | ValueKind::Null
        | ValueKind::Str(_)
        | ValueKind::Path(_)
        | ValueKind::Aggregate
        | ValueKind::Temp(_) => {}
    }
}

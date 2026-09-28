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
use crate::types::{resolve_ty, Ty, TyKind};

use super::types::{c_type, declarator};
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
        self.used_names.clear();
        self.local_names.clear();
        self.pointer_params.clear();
        for param in &info.decl.params {
            let ty = resolve_ty(&param.ty, &type_param_names(info.decl), &self.sema.table);
            let ty = substitute_in(&ty, &self.subst);
            if is_pointer_param(&ty, param.mode, &self.sema.table) {
                self.pointer_params.insert(param.name.clone());
            }
        }
        self.scope_actions.clear();
        self.rungs.clear();
        self.labels.clear();
        self.result_var = None;
        self.reachable.clear();
        self.emitted.clear();
        self.pending_labels.clear();
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
        self.function = Some(function);
        self.current_module = info.module.clone();
        self.return_ty = self.substitute(&function.ret);
        // A by-value fixed-array return is written into the hidden trailing
        // out-parameter, so the caller's destination is the result variable on
        // every path, with or without a cleanup ladder.
        if self.return_ty.is_fixed_array() {
            self.result_var = Some("__cl_ret".to_owned());
        }

        for param in &function.params {
            let c_name = sanitize_identifier(&param.name);
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
        self.check_stack_frame(function, info.decl.name_span);

        let symbol = match &instance {
            Some((_, key)) => format!("cl_{}__{}", info.decl.name, key),
            None => self.function_symbol(&info.module, &info.decl.name),
        };
        let signature = self.signature_with_symbol(&info.decl, &self.subst, &symbol);
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
    /// parameters.
    pub(super) fn emit_lifted_function(&mut self, function: &'a ir::Function) {
        let Some(site) = function.lambda_site else {
            return;
        };
        let capturing = self
            .sema
            .closures
            .get(&site)
            .is_some_and(|closure| !closure.captures.is_empty());

        // A lifted function is never generic: its types were fixed when the
        // enclosing body was lowered, so no substitution is active.
        self.subst.clear();
        self.used_names.clear();
        self.local_names.clear();
        self.pointer_params.clear();
        self.scope_actions.clear();
        self.rungs.clear();
        self.labels.clear();
        self.result_var = None;
        self.reachable.clear();
        self.emitted.clear();
        self.pending_labels.clear();
        self.loop_entry.clear();
        self.suppress_output = false;

        self.function = Some(function);
        self.current_module = function.module.clone();
        self.return_ty = function.ret.clone();

        for param in &function.params {
            let c_name = sanitize_identifier(&param.name);
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

        let signature = self.lifted_signature(function, site, capturing);
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
    /// A capturing lambda's function takes a leading `struct ClEnv_<site> *`
    /// named `__env`; a capture-less one has only the lambda parameters. The
    /// function is always `static`, because a lifted lambda is reached only
    /// through its value, never by name from another translation unit.
    pub(super) fn lifted_signature(
        &self,
        function: &ir::Function,
        site: usize,
        capturing: bool,
    ) -> String {
        let ret = if function.ret.is_void() {
            "void".to_owned()
        } else {
            c_type(&function.ret)
        };
        let mut params: Vec<String> = Vec::new();
        if capturing {
            params.push(format!("struct ClEnv_{site} *__env"));
        }
        for param in &function.params {
            params.push(declarator(&param.ty, &sanitize_identifier(&param.name)));
        }
        if params.is_empty() {
            params.push("void".to_owned());
        }
        format!("static {ret} cl_lambda_{site}({})", params.join(", "))
    }

    /// Writes the hoisted local declarations, the temporaries, and the result.
    fn emit_local_declarations(&mut self, function: &'a ir::Function) {
        let unread = unread_locals(function);
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
            self.push(&format!("    {} = {};", declarator(&ty, &c_name), empty));
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
                declarator(&ty, &format!("__cl_tmp{index}")),
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
            // redeclared as a local. Every other result is a hoisted local.
            if !self.return_ty.is_fixed_array() {
                let ret = self.return_ty.clone();
                let empty = self.empty_state(&ret);
                self.push(&format!("    {} = {};", declarator(&ret, &result), empty));
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
                if self.substitute(&target.ty).is_fixed_array() {
                    self.push(&format!(
                        "    memcpy(&({target_c}), &({source_c}), sizeof({target_c}));"
                    ));
                } else {
                    self.push(&format!("    {target_c} = {source_c};"));
                }
                let source_ty = self.substitute(&source.ty);
                self.emit_move_zero(&source_c, &source_ty);
            }
            Statement::MoveOut { place } => {
                self.emit_line_directive(span);
                let source = self.place(place);
                let ty = self.substitute(&place.ty);
                self.emit_move_zero(&source, &ty);
            }
            Statement::StorePointer { pointer, value } => {
                self.emit_line_directive(span);
                // The pointer expression carries the element's run-time index,
                // so its guards (including the bounds check) must run first.
                self.emit_guards(pointer);
                self.emit_guards(value);
                let pointer_c = self.value(pointer);
                let pointer_ty = self.substitute(&pointer.ty);
                let value_c = self.store_text(value, &pointer_ty);
                self.push(&format!("    *({pointer_c}) = {value_c};"));
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
                self.emit_guards(value);
                let value_c = self.value(value);
                self.push(&format!("    (void)({value_c});"));
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
                let message = args
                    .first()
                    .map(|arg| self.value(arg))
                    .unwrap_or_else(|| "panic".to_owned());
                self.push(&format!("    cl_panic({message});"));
            }
            _ => {
                self.emit_line_directive(span);
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
                        .callee_return_type(callee)
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
                    } else if let Some(ty) = self.callee_return_type(callee) {
                        let dest = self.unique_local("__cl_discard");
                        self.push("    {");
                        self.push(&format!(
                            "        {} = {};",
                            declarator(&ty, &dest),
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
                        let ty = resolve_ty(&param.ty, &names, &self.sema.table);
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
            let ty = resolve_ty(&info.decl.returns, &names, &self.sema.table);
            self.substitute(&ty)
        })
    }

    /// Drops the previous value of an assignment target before it is overwritten.
    ///
    /// An assignment to a live owned place must release the old value first, or
    /// it leaks. The drop is null-safe, so writing a fresh (empty) target is
    /// unaffected; a scalar target owns nothing and is skipped.
    fn emit_reassigned_target_drop(&mut self, target: &Place) {
        let ty = self.substitute(&target.ty);
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
        let ty = self.substitute(&target.ty);
        self.emit_line_directive(span);
        if ty.is_fixed_array() {
            self.emit_fixed_array_assign(target, &ty, value);
            return;
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
            declarator(&ty, &save),
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
    /// C arrays are not assignable, so the value's bytes travel with the C
    /// runtime `memcpy`. A right-hand side that renders to the target itself
    /// (`a = a;`) already holds the value, so only its guards can have an
    /// effect. When the target owns elements and the right-hand side reads it,
    /// the old value is saved into a zeroed local first, so it survives until
    /// the copy is done and is then released once; an unread old target is
    /// released before the copy. `sizeof(target)` is the byte count of the
    /// whole array, which C computes from the declared type.
    fn emit_fixed_array_assign(&mut self, target: &Place, ty: &Ty, value: &Value) {
        let target_c = self.place(target);
        let value_c = self.value(value);
        if value_c == target_c {
            self.emit_guards(value);
            return;
        }
        if !self.needs_drop(ty) {
            self.emit_guards(value);
            let value_c = self.store_text(value, ty);
            self.push(&format!(
                "    memcpy(&({target_c}), &({value_c}), sizeof({target_c}));"
            ));
            return;
        }
        if !value_reads_base(value, target) {
            // The right-hand side does not read the target, so the old value
            // is released before the right-hand side is evaluated.
            self.emit_drop(target);
            self.emit_guards(value);
            let value_c = self.store_text(value, ty);
            self.push(&format!(
                "    memcpy(&({target_c}), &({value_c}), sizeof({target_c}));"
            ));
            return;
        }
        // The right-hand side reads the target, so the old value must survive
        // until the bytes have been copied. Save it by byte copy, copy the new
        // value in, then release the saved old value.
        let save = self.unique_local("__cl_save");
        self.push("    {");
        self.push(&format!(
            "        {} = {};",
            declarator(ty, &save),
            self.empty_state(ty)
        ));
        self.push(&format!(
            "        memcpy(&({save}), &({target_c}), sizeof({target_c}));"
        ));
        self.emit_guards(value);
        let value_c = self.store_text(value, ty);
        self.push(&format!(
            "        memcpy(&({target_c}), &({value_c}), sizeof({target_c}));"
        ));
        self.emit_drop_expr(&save, ty);
        self.push("    }");
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
        // execute once its arguments are known to be valid.
        for arg in args {
            self.emit_guards(arg);
        }
        let temp = self.unique_local("__cl_call");
        self.push("    {");
        self.push(&format!(
            "        {} = {};",
            declarator(ty, &temp),
            self.empty_state(ty)
        ));
        let call = self.call_with_destination(callee, type_args, args, receiver, &temp);
        self.push(&format!("        {call};"));
        self.emit_reassigned_target_drop(target);
        let target_c = self.place(target);
        self.push(&format!(
            "        memcpy(&({target_c}), &({temp}), sizeof({target_c}));"
        ));
        self.emit_move_zero(&temp, ty);
        self.push("    }");
    }

    /// Renders a value for a store into a target of type `ty`.
    ///
    /// An owned `String` widens implicitly to the `str` view the target holds,
    /// through the same shim view the call-argument widening uses. Every other
    /// store renders the value directly.
    pub(super) fn store_text(&mut self, value: &Value, ty: &Ty) -> String {
        let text = self.value(value);
        let value_ty = self.substitute(&value.ty);
        if matches!(ty.kind, TyKind::Str | TyKind::StaticStr)
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
        self.emit_array_assign_to(&target_c, ty, elements, fill);
    }

    /// Emits an array literal as element-by-element assignments to a C lvalue.
    ///
    /// The target is a rendered C lvalue rather than a place, so the same
    /// lowering can write a hoisted local or the hidden out-parameter that
    /// carries a by-value fixed-array return. C cannot assign an array in one
    /// statement, so a multi-extent type is filled one dimension deeper: an
    /// element that is itself an array literal recurses against the indexed
    /// sub-target instead of writing a rejected brace list.
    pub(super) fn emit_array_assign_to(
        &mut self,
        target_c: &str,
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
            let sub = format!("{target_c}[{index}]");
            if let Some(element_ty) = &nested_ty {
                if let ValueKind::ArrayLit { elements, fill } = &value.kind {
                    self.emit_array_assign_to(&sub, element_ty, elements, fill);
                    continue;
                }
            }
            self.emit_guards(value);
            let lowered = self.store_text(value, element);
            self.push(&format!("    {sub} = {lowered};"));
        }
        let prefix = elements.len();
        let Some(fill) = fill else {
            return;
        };
        // A fill covers every index the prefix did not: `[a, b; fill]` fills
        // from the prefix length to the extent, and `[; fill]` fills them all.
        if let Some(element_ty) = &nested_ty {
            if let ValueKind::ArrayLit { elements, fill } = &fill.kind {
                for index in prefix..extent {
                    let sub = format!("{target_c}[{index}]");
                    self.emit_array_assign_to(&sub, element_ty, elements, fill);
                }
                return;
            }
        }
        self.emit_guards(fill);
        let lowered = self.store_text(fill, element);
        for index in prefix..extent {
            self.push(&format!("    {target_c}[{index}] = {lowered};"));
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
}

/// The declared locals the emitted C never reads as a value.
///
/// A declaration target and a `sizeof` unused marker already use a local, so
/// only the value positions count: a right-hand side, a call argument, a store,
/// a move source, a dropped place, and a terminator's condition or return. A
/// local reached only from a write is absent from the result, so the emitter
/// can mark it unused and keep the generated C free of
/// `-Wunused-but-set-variable`.
fn unread_locals(function: &ir::Function) -> BTreeSet<String> {
    let mut read: BTreeSet<String> = BTreeSet::new();
    for block in &function.blocks {
        for instr in &block.statements {
            match &instr.statement {
                Statement::Assign { value, .. } => note_value(value, &mut read),
                Statement::Move { source, .. } => note_place(source, &mut read),
                Statement::StorePointer { pointer, value } => {
                    note_value(pointer, &mut read);
                    note_value(value, &mut read);
                }
                Statement::Call { args, receiver, .. } => {
                    if let Some(receiver) = receiver {
                        note_value(receiver, &mut read);
                    }
                    for arg in args {
                        note_value(arg, &mut read);
                    }
                }
                Statement::Discard { value } => note_value(value, &mut read),
                Statement::Drop { place, .. } => note_place(place, &mut read),
                _ => {}
            }
        }
        match &block.terminator {
            Terminator::Branch { condition, .. } => note_value(condition, &mut read),
            Terminator::Switch { value, arms, .. } => {
                note_value(value, &mut read);
                for (label, _) in arms {
                    note_value(label, &mut read);
                }
            }
            Terminator::Return(Some(value)) => note_value(value, &mut read),
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

/// Records every local named anywhere inside a value.
fn note_value(value: &Value, read: &mut BTreeSet<String>) {
    match &value.kind {
        ValueKind::Place(place) => note_place(place, read),
        ValueKind::Member { base, .. } => note_value(base, read),
        ValueKind::Index { base, index } => {
            note_value(base, read);
            note_value(index, read);
        }
        ValueKind::Unary { operand, .. } => note_value(operand, read),
        ValueKind::Binary { left, right, .. } => {
            note_value(left, read);
            note_value(right, read);
        }
        ValueKind::Cast { value: inner, .. } => note_value(inner, read),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            note_value(condition, read);
            note_value(then_value, read);
            note_value(else_value, read);
        }
        ValueKind::StructLit { fields, .. } => {
            for (_, field) in fields {
                note_value(field, read);
            }
        }
        ValueKind::ArrayLit { elements, fill } => {
            for element in elements {
                note_value(element, read);
            }
            if let Some(fill) = fill {
                note_value(fill, read);
            }
        }
        ValueKind::Template(pieces) => {
            for piece in pieces {
                if let ValuePiece::Placeholder(inner) = piece {
                    note_value(inner, read);
                }
            }
        }
        ValueKind::Call {
            args, receiver, ..
        } => {
            if let Some(receiver) = receiver {
                note_value(receiver, read);
            }
            for arg in args {
                note_value(arg, read);
            }
        }
        ValueKind::Intrinsic { args, .. } => {
            for arg in args {
                note_value(arg, read);
            }
        }
        ValueKind::Materialize { value: inner, .. } => note_value(inner, read),
        ValueKind::Closure { captures, .. } => {
            for capture in captures {
                note_value(capture, read);
            }
        }
        ValueKind::ClosureCall { callable, args } => {
            note_value(callable, read);
            for arg in args {
                note_value(arg, read);
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

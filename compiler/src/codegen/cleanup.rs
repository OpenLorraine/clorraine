//! Cleanup collection, the ladder, and the block walk.
//!
//! A scope that owns a droppable value or registers a `defer` gets one cleanup
//! entry. The entries of a scope run in one reverse registration order, so the
//! last registered drop or `defer` runs first and the two kinds interleave
//! exactly as the checker registered them. A scope outside a loop takes a rung
//! of the function-end ladder only when an early edge actually leaves it; every
//! other scope runs its cleanup inline. This module also owns the block walk
//! that decides which cleanup an edge carries and the terminators that leave a
//! function.

use std::collections::BTreeSet;

use crate::ir::{self, BlockId, Place, Statement, Terminator, ValueKind};
use crate::types::{Ty, TyKind};

use super::control::FlowContext;
use super::function::value_moves_pointee;
use super::{CleanupFrame, Emitter};

/// One cleanup entry of a scope, in registration order.
#[derive(Clone)]
pub(crate) enum Action {
    /// Drop a place through its type's deleter.
    Drop(Place),
    /// Run a deferred block's statements: the body's root block and one past
    /// its last block.
    Defer(BlockId, BlockId),
}

impl<'a> Emitter<'a> {
    // -- cleanup collection ---------------------------------------------------

    /// Collects the cleanup actions and ladder rungs of one body.
    ///
    /// Every scope that owns a `drop` or a `defer` runs that cleanup inline at
    /// its `ScopeEnd` on the normal fall-through path. A scope also takes a
    /// ladder rung only when an early edge actually leaves it, which is the
    /// `ScopeExit` markers a `return`, `break`, or `continue` emits. A scope
    /// that is never left early therefore gets no label, so the emitted C has
    /// no unused label.
    pub(super) fn collect_cleanup(&mut self, function: &'a ir::Function) {
        for block in &function.blocks {
            for instr in &block.statements {
                match &instr.statement {
                    Statement::Drop { place, scope, order } => {
                        self.push_action(*scope, *order, Action::Drop(place.clone()));
                    }
                    Statement::Defer {
                        block,
                        scope,
                        order,
                        end,
                    } => {
                        self.push_action(*scope, *order, Action::Defer(*block, *end));
                    }
                    _ => {}
                }
            }
        }

        let reachable = reachable_blocks(function);
        let mut edge_scopes: BTreeSet<usize> = BTreeSet::new();
        for block in &function.blocks {
            if !reachable.contains(&block.id) {
                continue;
            }
            for instr in &block.statements {
                if let Statement::ScopeExit { scope } = &instr.statement {
                    edge_scopes.insert(*scope);
                }
            }
        }

        let mut rungs: Vec<usize> = self
            .scope_actions
            .iter()
            .filter(|(scope, actions)| {
                !actions.is_empty()
                    && !function.scopes[**scope].in_loop
                    && edge_scopes.contains(scope)
                    && !defer_region_has_out_edge(function, actions)
            })
            .map(|(scope, _)| *scope)
            .collect();
        rungs.sort_by(|left, right| {
            scope_depth(function, *right)
                .cmp(&scope_depth(function, *left))
                .then(left.cmp(right))
        });
        for (index, scope) in rungs.iter().enumerate() {
            self.labels.insert(*scope, index + 1);
        }
        self.rungs = rungs;
    }

    /// Records a cleanup action for a scope, skipping an exact duplicate.
    ///
    /// The list is kept sorted by registration order, so running it backwards
    /// interleaves a scope's drops and `defer` entries exactly as the checker
    /// registered them.
    fn push_action(&mut self, scope: usize, order: u32, action: Action) {
        let actions = self.scope_actions.entry(scope).or_default();
        let duplicate = actions.iter().any(|(_, known)| match (known, &action) {
            (Action::Drop(a), Action::Drop(b)) => a == b,
            (Action::Defer(a, _), Action::Defer(b, _)) => a == b,
            _ => false,
        });
        if !duplicate {
            actions.push((order, action));
            actions.sort_by_key(|(order, _)| *order);
        }
    }

    /// True when a scope's cleanup runs inline rather than from the ladder.
    fn scope_is_inline(&self, function: &'a ir::Function, scope: usize) -> bool {
        function.scopes[scope].in_loop || !self.labels.contains_key(&scope)
    }

    // -- block emission -------------------------------------------------------

    /// Emits one block's statements and returns the ladder entry for its exit.
    ///
    /// A `return` in the block jumps to the label of the innermost scope it
    /// leaves, so the returned value is that scope's ladder number when the
    /// block leaves through a non-inline scope. `jump_out` is true when the
    /// block's edge is a `break` or `continue`, which cannot use the ladder
    /// (the ladder returns), so their cleanup runs inline before the jump. The
    /// caller emits the block's terminator, because the structured emitter
    /// turns most terminators into C control flow rather than a `goto`.
    ///
    /// A value return also returns the inline cleanup it deferred: the
    /// terminator stores the value before that cleanup releases the storage the
    /// value reads.
    pub(super) fn emit_block_body(
        &mut self,
        function: &'a ir::Function,
        id: BlockId,
        jump_out: bool,
        ctx: FlowContext,
    ) -> (Option<usize>, Vec<usize>) {
        let block = &function.blocks[id];
        // A value return reads the storage its inline cleanup releases, so the
        // inline cleanup is deferred to the terminator, which stores the value
        // first. A void return has no value to read and runs it in place.
        let value_return = matches!(
            &block.terminator,
            Terminator::Return { value: Some(_), .. }
        );
        let mut pending_scope: Option<usize> = None;
        let mut inlined_edge: BTreeSet<usize> = BTreeSet::new();
        let mut edge_scopes: Vec<usize> = Vec::new();
        let mut deferred_inline: Vec<usize> = Vec::new();
        for instr in &block.statements {
            match &instr.statement {
                Statement::ScopeEnd { scope } => {
                    pending_scope = Some(*scope);
                    // Cleanup runs at the closing brace on the normal
                    // fall-through path, so it happens for every scope and not
                    // only for an edge that jumps into the ladder.
                    self.emit_scope_inline(*scope, ctx);
                }
                Statement::ScopeExit { scope } => {
                    // An early edge carries the scopes it leaves. A scope with
                    // a ladder rung is reached through the ladder; any other
                    // scope — one inside a loop, one whose deferred bodies
                    // carry a control-flow out-edge, or one whose cleanup is
                    // running right now — runs its actions inline here.
                    pending_scope = None;
                    let running = self.active_frame_position(*scope).is_some();
                    if !jump_out && !running && self.labels.contains_key(scope) {
                        if !edge_scopes.contains(scope) {
                            edge_scopes.push(*scope);
                        }
                    } else if inlined_edge.insert(*scope) {
                        // An inline scope that owns cleanup runs the whole
                        // edge's cleanup inline, innermost first. Jumping to a
                        // rung for an inner scope would skip this scope's
                        // actions, and running this scope before an inner rung
                        // would invert the cleanup order — a deferred `break`
                        // here would preempt the rung jump entirely. A scope
                        // that owns no cleanup contributes nothing, so inner
                        // rungs stay deferred to the ladder.
                        let owns_cleanup = self
                            .scope_actions
                            .get(scope)
                            .is_some_and(|actions| !actions.is_empty());
                        if owns_cleanup {
                            let inline_scopes = std::mem::take(&mut edge_scopes);
                            for edge_scope in inline_scopes {
                                self.emit_or_defer_cleanup(
                                    edge_scope,
                                    value_return,
                                    &mut deferred_inline,
                                    ctx,
                                );
                            }
                            self.emit_or_defer_cleanup(
                                *scope,
                                value_return,
                                &mut deferred_inline,
                                ctx,
                            );
                        }
                    }
                }
                Statement::Drop { scope, .. } => {
                    if pending_scope == Some(*scope) {
                        // The fall-through cleanup of a scope end was already
                        // handled at the scope end, so a matching drop is a
                        // no-op here.
                        continue;
                    }
                    let running = self.active_frame_position(*scope).is_some();
                    if jump_out || running || self.scope_is_inline(function, *scope) {
                        if inlined_edge.insert(*scope) {
                            self.emit_or_defer_cleanup(
                                *scope,
                                value_return,
                                &mut deferred_inline,
                                ctx,
                            );
                        }
                    } else if !edge_scopes.contains(scope) {
                        edge_scopes.push(*scope);
                    }
                }
                Statement::Defer { .. } => {
                    // Deferred bodies run at the scope's cleanup point, never
                    // at the registration site.
                    pending_scope = None;
                }
                other => {
                    pending_scope = None;
                    self.emit_statement(instr.span, other);
                }
            }
        }
        let jump = edge_scopes
            .first()
            .and_then(|scope| self.labels.get(scope).copied());
        (jump, deferred_inline)
    }

    /// Emits a scope's cleanup now, or defers it to a value return's store.
    ///
    /// A value return must read its value before the cleanup that releases the
    /// storage it reads, so the inline cleanup of the scopes it leaves is
    /// collected and emitted by the terminator after the value store.
    fn emit_or_defer_cleanup(
        &mut self,
        scope: usize,
        defer: bool,
        deferred: &mut Vec<usize>,
        ctx: FlowContext,
    ) {
        if defer {
            deferred.push(scope);
        } else {
            self.emit_scope_inline(scope, ctx);
        }
    }

    /// Emits one copy of a deferred block at its cleanup activation.
    ///
    /// Every activation of the owning scope's cleanup emits its own copy of
    /// the block, and the activation ordinal selects the copy's label
    /// namespace. A nested activation of a root that is already emitting is a
    /// no-op: a copy's own return cleanup can re-enter the running scope, and
    /// re-emitting the same block there would recurse without end. The ambient
    /// flow context lets an edge that leaves the body (a `break` or `continue`)
    /// print as the enclosing loop's edge instead of a raw `goto`.
    fn emit_defer(&mut self, root: BlockId, end: BlockId, ctx: FlowContext) {
        let Some(function) = self.function else {
            return;
        };
        if self.active_defers.contains(&root) {
            return;
        }
        let activation = self.defer_copies.entry(root).or_insert(0);
        *activation += 1;
        let activation = *activation;
        self.active_defers.push(root);
        self.emit_closure(function, root, end, activation, ctx);
        self.active_defers.pop();
    }

    /// Emits a scope's cleanup actions inline, once.
    ///
    /// A scope re-entered from inside its own running cleanup runs only the
    /// actions older than the running one: the newer ones already ran, and a
    /// second run would double their effects.
    fn emit_scope_inline(&mut self, scope: usize, ctx: FlowContext) {
        self.emit_line_for_scope(scope);
        let actions = self.scope_actions.get(&scope).cloned().unwrap_or_default();
        let remaining = match self.active_frame_position(scope) {
            Some(position) => actions[..position].to_vec(),
            None => actions,
        };
        self.emit_actions(&remaining, scope, ctx);
    }

    /// Writes a `#line` mapping for generated cleanup to the scope's close.
    ///
    /// Inserted cleanup belongs to the scope's closing source line, so a
    /// debugger stepping the generated drop stays on the closing line of the
    /// scope rather than entering a generated statement.
    fn emit_line_for_scope(&mut self, scope: usize) {
        let Some(function) = self.function else {
            return;
        };
        let Some(info) = function.scopes.get(scope) else {
            return;
        };
        let span = info.close_span;
        self.emit_close_line(span);
    }

    /// Writes a `#line` directive for the last line of a span.
    ///
    /// A scope's close span covers its closing brace, so the closing line is
    /// the line of the span's last covered byte.
    pub(super) fn emit_close_line(&mut self, span: crate::span::Span) {
        let last = span.end.saturating_sub(1);
        if span.is_empty() && last == 0 {
            return;
        }
        let line = self.sources.location(span.file, last as usize).line;
        let path = self.path_of(span).to_owned();
        self.push(&format!("#line {line} \"{path}\""));
    }

    /// Runs a list of cleanup actions in reverse registration order.
    ///
    /// Drops and `defer` entries interleave exactly as the checker registered
    /// them, and the whole list runs backwards, so the last registered cleanup
    /// runs first. Running drops and defers as one reversed list is what honours
    /// the single reverse-registration cleanup order. `ctx` is the ambient flow
    /// context at the cleanup site, threaded into a deferred copy so its
    /// out-edges print as the enclosing loop's edges.
    fn emit_actions(&mut self, actions: &[(u32, Action)], scope: usize, ctx: FlowContext) {
        for (position, (_, action)) in actions.iter().enumerate().rev() {
            self.cleanup_frames.push(CleanupFrame { scope, position });
            match action {
                Action::Defer(root, end) => self.emit_defer(*root, *end, ctx),
                Action::Drop(place) => self.emit_drop(place),
            }
            self.cleanup_frames.pop();
        }
    }

    /// The position of the innermost running action of `scope`, when its
    /// cleanup is re-entered from inside itself.
    fn active_frame_position(&self, scope: usize) -> Option<usize> {
        self.cleanup_frames
            .iter()
            .rev()
            .find(|frame| frame.scope == scope)
            .map(|frame| frame.position)
    }

    // -- terminators ----------------------------------------------------------

    /// Emits a `return`, using `jump` as the ladder entry for a cleanup exit.
    ///
    /// A function with a cleanup ladder stores the value in the result variable
    /// and jumps to the innermost scope label it leaves, so the ladder can drop
    /// the scopes on the way out. A function without one returns directly.
    /// Statement temporaries the returned expression read are released after
    /// the value has been evaluated and before control leaves. The scopes in
    /// `deferred` are the inline cleanup of a value return's edge: they run
    /// after the value store, because the value may read the storage they
    /// release.
    pub(super) fn emit_return(
        &mut self,
        span: crate::span::Span,
        value: Option<&ir::Value>,
        releases: &[ir::Release],
        jump: Option<usize>,
        deferred: &[usize],
        ctx: FlowContext,
    ) {
        self.emit_line_directive(span);
        if let Some(value) = value {
            if self.return_ty.is_fixed_array() {
                if let Some(result) = self.result_var.clone() {
                    let ret_ty = self.return_ty.clone();
                    self.emit_superseded_result_release(&result, &ret_ty);
                }
                self.emit_fixed_array_return(value);
            } else {
                // A literal whose fields need stores, and an owned conditional,
                // move the chosen value into the result per path. A result
                // variable keeps the ordinary tail: the releases and the ladder
                // jump below run with the value already stored.
                let ret_ty = self.return_ty.clone();
                let mut value_stored = false;
                if let ValueKind::StructLit { name, fields } = &value.kind {
                    if self.struct_literal_needs_stores(name, fields) {
                        if self.emit_struct_literal_return(&ret_ty, fields, releases, deferred, ctx)
                        {
                            return;
                        }
                        value_stored = true;
                    }
                }
                if !value_stored && self.needs_drop(&ret_ty) {
                    if let ValueKind::Conditional {
                        condition,
                        then_value,
                        else_value,
                    } = &value.kind
                    {
                        if self.emit_conditional_move_return(
                            &ret_ty,
                            condition,
                            then_value,
                            else_value,
                            releases,
                            deferred,
                            ctx,
                        ) {
                            return;
                        }
                        value_stored = true;
                    }
                }
                if !value_stored {
                    if let ValueKind::Conditional {
                        condition,
                        then_value,
                        else_value,
                    } = &value.kind
                    {
                        if self.conditional_struct_needs_stores(&ret_ty, then_value, else_value) {
                            if self.emit_struct_conditional_return(
                                &ret_ty,
                                condition,
                                then_value,
                                else_value,
                                releases,
                                deferred,
                                ctx,
                            ) {
                                return;
                            }
                            value_stored = true;
                        }
                    }
                }
                if !value_stored {
                    self.emit_guards(value);
                    // A returned `String` widens to the `str` view the return
                    // type declares, exactly as a store into a `str` place does.
                    let lowered = self.store_text(value, &ret_ty);
                    match self.result_var.clone() {
                        Some(result) => {
                            self.emit_superseded_result_release(&result, &ret_ty);
                            self.push(&format!("    {result} = {lowered};"));
                            // Returning an owned value transfers it to the
                            // caller, so the source is emptied before the
                            // cleanup ladder runs; otherwise the callee would
                            // release what it just handed back. A projected
                            // place (a field or an element) is emptied through
                            // the same arm walk, and a duplicable instance's
                            // source owns nothing to empty.
                            if self.needs_drop(&ret_ty) {
                                let source_ty = self.substitute(&value.ty);
                                if self.needs_drop(&source_ty) {
                                    self.emit_arm_source_moves(value, &source_ty);
                                }
                            }
                        }
                        None if !releases.is_empty()
                            || self.return_source_needs_emptying(value, &ret_ty) =>
                        {
                            // Without a cleanup ladder the value would return
                            // directly, and a release cannot run after a
                            // `return`. The value is stored in a block-local
                            // first, so the releases run after it has been read.
                            // A storage-rooted source is emptied before the
                            // releases, exactly as the result-var branch does:
                            // otherwise a release of the temporary would free
                            // the element the returned value still aliases. A
                            // return that copies caller storage takes the same
                            // form even without releases, so the caller's slot
                            // is emptied before control leaves.
                            let local = self.unique_local("__cl_return");
                            let decl = super::types::declarator(&ret_ty, &local, &self.sema.table);
                            self.push("    {");
                            self.push(&format!("        {decl} = {lowered};"));
                            if self.needs_drop(&ret_ty) {
                                let source_ty = self.substitute(&value.ty);
                                if self.needs_drop(&source_ty) {
                                    self.emit_arm_source_moves(value, &source_ty);
                                }
                            }
                            self.emit_temp_releases(releases);
                            self.emit_deferred_return_cleanup(deferred, ctx);
                            self.push(&format!("        return {local};"));
                            self.push("    }");
                            return;
                        }
                        None => {
                            self.emit_deferred_return_cleanup(deferred, ctx);
                            self.push(&format!("    return {lowered};"));
                            return;
                        }
                    }
                }
            }
        }
        self.emit_temp_releases(releases);
        self.emit_deferred_return_cleanup(deferred, ctx);
        if let Some(label) = jump {
            if !self.suppress_output {
                self.used_rung_labels.insert(label);
            }
            self.push(&format!("    goto __cl_c{label};"));
        } else if self.return_ty.is_fixed_array() {
            // A fixed-array return lowers to a `void` function whose result
            // already sits in the caller's out-parameter.
            self.push("    return;");
        } else if let Some(result) = self.result_var.clone() {
            self.push(&format!("    return {result};"));
        } else if self.return_ty.is_void() {
            self.push("    return;");
        } else {
            // A value-less return in a non-void function is unreachable: it is
            // the tail after a diverging `panic`, or a body with no return on
            // some path. Returning the type's empty state keeps the emitted C
            // valid without changing any reachable behavior.
            let value = self.empty_return_value();
            self.push(&format!("    return {value};"));
        }
    }

    /// True when a direct return must empty storage the caller owns.
    ///
    /// A returned value that copied caller storage — the pointee of a `borrow`
    /// pointer parameter, or the pointee of a raw-deref read — leaves the
    /// caller's slot and the result aliasing one allocation, so the direct
    /// branch stores the value in a block local and empties the source before
    /// control leaves. A by-value parameter, a local, a view, and a fresh
    /// temporary have no caller storage to empty and keep the direct emission.
    fn return_source_needs_emptying(&self, value: &ir::Value, ret_ty: &Ty) -> bool {
        if !self.needs_drop(ret_ty) {
            return false;
        }
        // A raw-deref read returns a copy of its pointee, so the pointee must
        // be emptied or the caller and the read would share one owner. A
        // projected member (`(*p).s`) empties that member through the pointer,
        // not the whole pointee.
        if value_moves_pointee(value) {
            return self.needs_drop(&self.substitute(&value.ty));
        }
        let ir::ValueKind::Place(place) = &value.kind else {
            return false;
        };
        let ir::PlaceBase::Param(name) = &place.base else {
            return false;
        };
        self.pointer_params.contains(name) && self.needs_drop(&self.substitute(&value.ty))
    }

    /// Emits an owned conditional return through the branch store.
    ///
    /// With a cleanup ladder the result variable receives the chosen arm and
    /// the caller continues with the releases and the ladder jump, so `false`
    /// is returned. Without one the value is stored in a block-local, the
    /// statement releases run after it has been read, and the block returns the
    /// local; `true` means the whole return was emitted.
    fn emit_conditional_move_return(
        &mut self,
        ret_ty: &Ty,
        condition: &ir::Value,
        then_value: &ir::Value,
        else_value: &ir::Value,
        releases: &[ir::Release],
        deferred: &[usize],
        ctx: FlowContext,
    ) -> bool {
        if let Some(result) = self.result_var.clone() {
            self.emit_superseded_result_release(&result, ret_ty);
            self.emit_conditional_move_assign(
                &result,
                ret_ty,
                condition,
                then_value,
                else_value,
            );
            return false;
        }
        let local = self.unique_local("__cl_return");
        let decl = super::types::declarator(ret_ty, &local, &self.sema.table);
        self.push("    {");
        self.push(&format!(
            "        {decl} = {};",
            self.empty_state(ret_ty)
        ));
        self.emit_conditional_move_assign(&local, ret_ty, condition, then_value, else_value);
        self.emit_temp_releases(releases);
        self.emit_deferred_return_cleanup(deferred, ctx);
        self.push(&format!("        return {local};"));
        self.push("    }");
        true
    }

    /// Emits a struct literal that needs per-field stores as a return value.
    ///
    /// With a result variable the fields are stored into it and the caller
    /// continues with the releases and the ladder jump, so `false` is
    /// returned. Without one the literal is built in a block-local, the
    /// statement releases run after it has been read, and the block returns the
    /// local; `true` means the whole return was emitted.
    fn emit_struct_literal_return(
        &mut self,
        ret_ty: &Ty,
        fields: &[(String, ir::Value)],
        releases: &[ir::Release],
        deferred: &[usize],
        ctx: FlowContext,
    ) -> bool {
        if let Some(result) = self.result_var.clone() {
            self.emit_superseded_result_release(&result, ret_ty);
            self.emit_struct_literal_fields(&result, ret_ty, fields);
            return false;
        }
        let local = self.unique_local("__cl_return");
        let decl = super::types::declarator(ret_ty, &local, &self.sema.table);
        self.push("    {");
        self.push(&format!(
            "        {decl} = {};",
            self.empty_state(ret_ty)
        ));
        self.emit_struct_literal_fields(&local, ret_ty, fields);
        self.emit_temp_releases(releases);
        self.emit_deferred_return_cleanup(deferred, ctx);
        self.push(&format!("        return {local};"));
        self.push("    }");
        true
    }

    /// Emits a struct conditional that needs per-arm stores as a return value.
    ///
    /// With a result variable the branches store into it and the caller
    /// continues with the releases and the ladder jump, so `false` is
    /// returned. Without one the conditional is built in a block-local, the
    /// statement releases run after it has been read, and the block returns the
    /// local; `true` means the whole return was emitted.
    fn emit_struct_conditional_return(
        &mut self,
        ret_ty: &Ty,
        condition: &ir::Value,
        then_value: &ir::Value,
        else_value: &ir::Value,
        releases: &[ir::Release],
        deferred: &[usize],
        ctx: FlowContext,
    ) -> bool {
        if let Some(result) = self.result_var.clone() {
            self.emit_superseded_result_release(&result, ret_ty);
            self.emit_struct_conditional_assign(&result, ret_ty, condition, then_value, else_value);
            return false;
        }
        let local = self.unique_local("__cl_return");
        let decl = super::types::declarator(ret_ty, &local, &self.sema.table);
        self.push("    {");
        self.push(&format!(
            "        {decl} = {};",
            self.empty_state(ret_ty)
        ));
        self.emit_struct_conditional_assign(&local, ret_ty, condition, then_value, else_value);
        self.emit_temp_releases(releases);
        self.emit_deferred_return_cleanup(deferred, ctx);
        self.push(&format!("        return {local};"));
        self.push("    }");
        true
    }

    /// Releases the pending result value a return is about to replace.
    ///
    /// A deferred return stores into the live result variable and continues the
    /// running cleanup; a deferred `break` or `continue` among the older
    /// actions can then preempt the pending return, so control reaches an
    /// ordinary store that supersedes it. A function with any out-edge defer
    /// releases at every ordinary store, because such a defer can preempt the
    /// return anywhere; a function whose only deferred stores exist releases
    /// after the first of them. Both cases release the pending owner first,
    /// exactly as an assignment to a live owner does. The release is
    /// null-safe and gated: a function with no out-edge defer and no deferred
    /// store keeps its unchanged shape.
    fn emit_superseded_result_release(&mut self, result: &str, ret_ty: &Ty) {
        if !self.needs_drop(ret_ty) {
            return;
        }
        if self.active_defers.is_empty() {
            if !self.deferred_result_store && !self.out_edge_defer {
                return;
            }
        } else {
            self.deferred_result_store = true;
        }
        self.emit_drop_expr(result, ret_ty);
    }

    /// Emits the inline cleanup a value return deferred past its value store.
    ///
    /// The scopes are emitted in the order the walk met them (innermost
    /// first), after the value has been stored and its statement temporaries
    /// released. A scope with no remaining cleanup emits nothing — skipping it
    /// keeps the emitted bytes free of a lone line directive. The cleanup can
    /// preempt the return it belongs to; the function-wide out-edge gate
    /// already armed the supersede release, so a later store frees the
    /// abandoned owner.
    fn emit_deferred_return_cleanup(&mut self, deferred: &[usize], ctx: FlowContext) {
        for scope in deferred {
            let remaining = match self.active_frame_position(*scope) {
                Some(position) => position,
                None => self.scope_actions.get(scope).map_or(0, Vec::len),
            };
            if remaining == 0 {
                continue;
            }
            self.emit_scope_inline(*scope, ctx);
        }
    }

    /// Drops every statement temporary a return carries, in list order.
    ///
    /// The checker records the releases newest first, and each drop is
    /// null-safe and empties its temporary, so an unevaluated guarded path
    /// releases nothing and a second drop would be a no-op.
    fn emit_temp_releases(&mut self, releases: &[ir::Release]) {
        for release in releases {
            let place = ir::Place {
                base: ir::PlaceBase::Temp(release.temp),
                projections: Vec::new(),
                ty: release.ty.clone(),
            };
            self.emit_drop(&place);
        }
    }

    /// Copies a by-value fixed-array return into the hidden out-parameter.
    ///
    /// C cannot assign or return an array, so the value is copied into the
    /// caller's destination. An array literal writes its elements through
    /// [`Emitter::emit_array_assign_to`]; a returned place travels through the
    /// statement store core, which copies every extent by bytes and empties the
    /// source once when it owns elements, so the scope cleanup does not release
    /// the value the caller now owns. A conditional evaluates only its chosen
    /// arm into the destination.
    fn emit_fixed_array_return(&mut self, value: &ir::Value) {
        self.emit_fixed_array_return_arm(value);
    }

    /// Emits one arm or value of a fixed-array return.
    fn emit_fixed_array_return_arm(&mut self, value: &ir::Value) {
        let ty = self.return_ty.clone();
        match &value.kind {
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.emit_guards(condition);
                let condition_c = self.value(condition);
                self.push(&format!("    if ({condition_c}) {{"));
                self.emit_fixed_array_return_arm(then_value);
                self.push("    } else {");
                self.emit_fixed_array_return_arm(else_value);
                self.push("    }");
            }
            ValueKind::ArrayLit { elements, fill } => {
                let destination = super::function::ArrayTarget::parameter(
                    "__cl_ret",
                    self.array_param_bytes("__cl_ret", &ty),
                );
                self.emit_array_assign_to(&destination, &ty, elements, fill, true);
            }
            ValueKind::Place(_)
            | ValueKind::Member { .. }
            | ValueKind::Index { .. }
            | ValueKind::Temp(_)
            | ValueKind::Materialize { .. } => {
                // The statement store core already copies every extent by
                // bytes, releases the (empty) destination first when the type
                // owns elements, and empties the source once. A projected row
                // (`m()[i]`) takes the same path: the source address is the
                // projected element and the arm walk empties exactly that row.
                // A resolved conditional arm is a temporary (or a deferred
                // carrier), and C cannot assign an array, so the same copy core
                // fills the hidden out-parameter from its bytes. The caller's
                // destination is the hidden out-parameter, never the source, so
                // `reads_target` is fixed false.
                let destination = super::function::ArrayTarget::parameter(
                    "__cl_ret",
                    self.array_param_bytes("__cl_ret", &ty),
                );
                self.emit_fixed_array_assign_to(&destination, &ty, value, false, false);
            }
            ValueKind::Call {
                callee,
                type_args,
                args,
                receiver,
            } => {
                // A returned fixed-array call writes straight into the same
                // destination the enclosing function returns through. The
                // callable field's receiver is guarded exactly as an argument
                // is, so a runtime receiver index traps before the call.
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
                    "__cl_ret",
                );
                self.push(&format!("    {call};"));
            }
            ValueKind::ClosureCall { callable, args } => {
                // A returned fixed-array closure call writes straight into the
                // caller's out-parameter, after the environment and the
                // lambda's own arguments.
                self.emit_guards(callable);
                for arg in args {
                    self.emit_guards(arg);
                }
                let call = self.closure_call_with_destination(callable, args, Some("__cl_ret"));
                self.push(&format!("    {call};"));
            }
            _ => {
                // A form other than a literal or a place has no C array value
                // to copy. Keep the generic store, so an unsupported form fails
                // the C compiler loudly rather than silently leaving the
                // caller's destination untouched.
                self.emit_guards(value);
                let lowered = self.value(value);
                self.push(&format!("    __cl_ret = {lowered};"));
            }
        }
    }

    /// Writes the function-end cleanup ladder.
    ///
    /// Only rungs an early edge (or the chain from one) reaches are emitted:
    /// an edge that ran its scopes inline leaves its outer rungs unreachable,
    /// and a defined-but-unused label fails the strict build. The planning
    /// pass, which writes nothing, still walks every rung so the local names a
    /// rung's cleanup allocates keep the same numbering they have in the
    /// emitted output. The chain runs through the emitted rungs in order and
    /// ends at the function tail.
    pub(super) fn emit_ladder(&mut self) {
        let rungs = self.rungs.clone();
        // A directly reached rung keeps every outer rung alive through the
        // chain, so the emitted set starts at the innermost reached label.
        let start = self.used_rung_labels.iter().min().copied();
        let emitted: Vec<usize> = if self.suppress_output {
            rungs.clone()
        } else {
            let Some(start) = start else {
                return;
            };
            rungs
                .iter()
                .copied()
                .filter(|scope| self.labels[scope] >= start)
                .collect()
        };
        for (index, scope) in emitted.iter().enumerate() {
            let label = self.labels[scope];
            self.emit_line_for_scope(*scope);
            self.push(&format!("__cl_c{label}:"));
            let actions = self.scope_actions.get(scope).cloned().unwrap_or_default();
            // A ladder rung sits at the function level, outside any loop or
            // switch, so its copies have no ambient edge context. A scope whose
            // deferred bodies carry an out-edge never takes a rung (the rung
            // filter keeps it inline).
            self.emit_actions(&actions, *scope, FlowContext::default());
            if index + 1 < emitted.len() {
                let next = self.labels[&emitted[index + 1]];
                self.push(&format!("    goto __cl_c{next};"));
            } else if self.return_ty.is_fixed_array() {
                // The result already sits in the caller's out-parameter and
                // the function is `void`, so the ladder ends with a bare return.
                self.push("    return;");
            } else if let Some(result) = self.result_var.clone() {
                self.push(&format!("    return {result};"));
            } else {
                self.push("    return;");
            }
        }
    }

    /// Writes a final return so a non-void function never falls off its end.
    ///
    /// Every reachable path already returns; this line only satisfies the C
    /// compiler's return-type analysis when the last emitted block ends with a
    /// `goto` into the ladder.
    pub(super) fn emit_closing_return(&mut self) {
        // A fixed-array return lowers to a `void` function, so it may fall off
        // the end like any other `void` function.
        if self.return_ty.is_void() || self.return_ty.is_fixed_array() {
            return;
        }
        if let Some(result) = self.result_var.clone() {
            self.push(&format!("    return {result};"));
            return;
        }
        let value = self.empty_return_value();
        self.push(&format!("    return {value};"));
    }

    /// The C empty-state value of the declared return type.
    ///
    /// A handle, raw pointer, `cstring`, or func value is the null literal; a
    /// view or struct is its all-zero brace form; a slice is its zero struct;
    /// every scalar is `0`.
    fn empty_return_value(&self) -> String {
        let ret = self.return_ty.clone();
        match &ret.kind {
            TyKind::Handle(_) | TyKind::Raw(_) | TyKind::CString | TyKind::Func(_) => {
                self.null_literal().to_owned()
            }
            // A view returns its zero value through the helper emitted with the
            // view typedefs; the C99-subset contract forbids a compound literal.
            TyKind::Str | TyKind::StaticStr => format!(
                "{}()",
                super::types::zero_view_helper(&ret).unwrap_or_else(|| "0".to_owned())
            ),
            TyKind::Array { extents, .. } if extents.iter().any(Option::is_none) => format!(
                "{}()",
                super::types::zero_view_helper(&ret).unwrap_or_else(|| "0".to_owned())
            ),
            // An owned struct returns its zero value through the helper
            // emitted with its definition; the C99-subset contract forbids a
            // compound literal.
            TyKind::Struct(..) => format!(
                "cl_zero_{}()",
                super::types::c_type(&ret, &self.sema.table)
            ),
            _ => "0".to_owned(),
        }
    }
}

/// True when a scope's deferred bodies contain a control-flow edge that leaves
/// the body's own block range.
///
/// Such an edge is a `break` or `continue` (the checker emits only those as
/// `goto`s out of a deferred body), and it can only be spelled from a site
/// inside the enclosing loop or switch. A scope whose deferred bodies carry
/// one therefore runs its cleanup inline at each exit edge instead of from the
/// function-level ladder, where the ambient context is unavailable.
pub(super) fn defer_region_has_out_edge(function: &ir::Function, actions: &[(u32, Action)]) -> bool {
    for action in actions.iter().map(|(_, action)| action) {
        let Action::Defer(root, end) = action else {
            continue;
        };
        for block in function.blocks.iter().filter(|block| block.id >= *root && block.id < *end) {
            if let Terminator::Goto(target) = &block.terminator {
                if *target < *root || *target >= *end {
                    return true;
                }
            }
        }
    }
    false
}

/// The block ids reachable from a function's entry through its terminators.
pub(super) fn reachable_blocks(function: &ir::Function) -> BTreeSet<BlockId> {
    closure_from(function, function.entry)
}

/// The block ids reachable from `root` through terminators.
pub(super) fn closure_from(function: &ir::Function, root: BlockId) -> BTreeSet<BlockId> {
    let mut seen: BTreeSet<BlockId> = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        for successor in successors(&function.blocks[id].terminator) {
            stack.push(successor);
        }
    }
    seen
}

/// The blocks a terminator can transfer control to.
fn successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Goto(target) => vec![*target],
        Terminator::Branch {
            on_true, on_false, ..
        } => vec![*on_true, *on_false],
        Terminator::Switch { arms, default, .. } => {
            let mut targets: Vec<BlockId> = arms.iter().map(|(_, block)| *block).collect();
            targets.push(*default);
            targets
        }
        Terminator::Return { .. } | Terminator::Unreachable => Vec::new(),
    }
}

/// The lexical depth of a scope, counted from the function body scope.
fn scope_depth(function: &ir::Function, scope: usize) -> usize {
    let mut depth = 0;
    let mut current = scope;
    while let Some(parent) = function.scopes[current].parent {
        depth += 1;
        current = parent;
    }
    depth
}

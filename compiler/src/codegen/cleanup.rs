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
use crate::types::TyKind;

use super::Emitter;

/// One cleanup entry of a scope, in registration order.
#[derive(Clone)]
pub(crate) enum Action {
    /// Drop a place through its type's deleter.
    Drop(Place),
    /// Run a deferred block's statements.
    Defer(BlockId),
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
                    Statement::Defer { block, scope, order } => {
                        self.push_action(*scope, *order, Action::Defer(*block));
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
            (Action::Defer(a), Action::Defer(b)) => a == b,
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
    pub(super) fn emit_block_body(
        &mut self,
        function: &'a ir::Function,
        id: BlockId,
        jump_out: bool,
    ) -> Option<usize> {
        let block = &function.blocks[id];
        let mut pending_scope: Option<usize> = None;
        let mut inlined_edge: BTreeSet<usize> = BTreeSet::new();
        let mut edge_scopes: Vec<usize> = Vec::new();
        for instr in &block.statements {
            match &instr.statement {
                Statement::ScopeEnd { scope } => {
                    pending_scope = Some(*scope);
                    // Cleanup runs at the closing brace on the normal
                    // fall-through path, so it happens for every scope and not
                    // only for an edge that jumps into the ladder.
                    self.emit_scope_inline(*scope);
                }
                Statement::ScopeExit { scope } => {
                    // An early edge carries the scopes it leaves. A scope with
                    // a ladder rung is reached through the ladder; any other
                    // scope, including one inside a loop and one whose only
                    // cleanup is a `defer`, runs its actions inline here.
                    pending_scope = None;
                    if !jump_out && self.labels.contains_key(scope) {
                        if !edge_scopes.contains(scope) {
                            edge_scopes.push(*scope);
                        }
                    } else if inlined_edge.insert(*scope) {
                        self.emit_scope_inline(*scope);
                    }
                }
                Statement::Drop { scope, .. } => {
                    if pending_scope == Some(*scope) {
                        // The fall-through cleanup of a scope end was already
                        // handled at the scope end, so a matching drop is a
                        // no-op here.
                        continue;
                    }
                    if jump_out || self.scope_is_inline(function, *scope) {
                        if inlined_edge.insert(*scope) {
                            self.emit_scope_inline(*scope);
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
        edge_scopes
            .first()
            .and_then(|scope| self.labels.get(scope).copied())
    }

    /// Emits one deferred block and its closure at a ladder rung.
    fn emit_defer(&mut self, root: BlockId) {
        let Some(function) = self.function else {
            return;
        };
        self.emit_closure(function, root);
    }

    /// Emits a scope's cleanup actions inline, once.
    fn emit_scope_inline(&mut self, scope: usize) {
        self.emit_line_for_scope(scope);
        let actions = self.scope_actions.get(&scope).cloned().unwrap_or_default();
        self.emit_actions(&actions);
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
    /// the single reverse-registration cleanup order.
    fn emit_actions(&mut self, actions: &[(u32, Action)]) {
        for (_, action) in actions.iter().rev() {
            match action {
                Action::Defer(block) => self.emit_defer(*block),
                Action::Drop(place) => self.emit_drop(place),
            }
        }
    }

    // -- terminators ----------------------------------------------------------

    /// Emits a `return`, using `jump` as the ladder entry for a cleanup exit.
    ///
    /// A function with a cleanup ladder stores the value in the result variable
    /// and jumps to the innermost scope label it leaves, so the ladder can drop
    /// the scopes on the way out. A function without one returns directly.
    /// Statement temporaries the returned expression read are released after
    /// the value has been evaluated and before control leaves.
    pub(super) fn emit_return(
        &mut self,
        span: crate::span::Span,
        value: Option<&ir::Value>,
        releases: &[ir::Release],
        jump: Option<usize>,
    ) {
        self.emit_line_directive(span);
        if let Some(value) = value {
            if self.return_ty.is_fixed_array() {
                self.emit_fixed_array_return(value);
            } else {
                self.emit_guards(value);
                // A returned `String` widens to the `str` view the return type
                // declares, exactly as a store into a `str` place does.
                let ret_ty = self.return_ty.clone();
                let lowered = self.store_text(value, &ret_ty);
                match self.result_var.clone() {
                    Some(result) => {
                        self.push(&format!("    {result} = {lowered};"));
                        // Returning an owned place transfers it to the caller,
                        // so the source is emptied before the cleanup ladder
                        // runs; otherwise the callee would release what it just
                        // handed back.
                        if let ValueKind::Place(place) = &value.kind {
                            let ty = self.substitute(&place.ty);
                            if self.needs_drop(&ty) {
                                let source = self.place(place);
                                self.emit_move_zero(&source, &ty);
                            }
                        }
                    }
                    None if !releases.is_empty() => {
                        // Without a cleanup ladder the value would return
                        // directly, and a release cannot run after a `return`.
                        // The value is stored in a block-local first, so the
                        // releases run after it has been read.
                        let local = self.unique_local("__cl_return");
                        let decl = super::types::declarator(&ret_ty, &local);
                        self.push("    {");
                        self.push(&format!("        {decl} = {lowered};"));
                        self.emit_temp_releases(releases);
                        self.push(&format!("        return {local};"));
                        self.push("    }");
                        return;
                    }
                    None => {
                        self.push(&format!("    return {lowered};"));
                        return;
                    }
                }
            }
        }
        self.emit_temp_releases(releases);
        if let Some(label) = jump {
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
    /// C cannot assign or return an array, so the value is written to the
    /// caller's destination element by element. An array literal writes its
    /// elements through [`Emitter::emit_array_assign_to`]; a returned owned
    /// place is copied element by element and then emptied, so the scope
    /// cleanup does not release the value the caller now owns. A conditional
    /// evaluates only its chosen arm into the destination.
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
                self.emit_array_assign_to("__cl_ret", &ty, elements, fill);
            }
            ValueKind::Place(place) => {
                self.emit_guards(value);
                let source = self.place(place);
                let extent = match &ty.kind {
                    TyKind::Array { extents, .. } => {
                        extents.first().and_then(|size| *size).unwrap_or(0)
                    }
                    _ => 0,
                };
                for index in 0..extent {
                    self.push(&format!("    __cl_ret[{index}] = {source}[{index}];"));
                }
                let source_ty = self.substitute(&place.ty);
                if self.needs_drop(&source_ty) {
                    self.emit_move_zero(&source, &source_ty);
                }
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
    pub(super) fn emit_ladder(&mut self) {
        let rungs = self.rungs.clone();
        for (index, scope) in rungs.iter().enumerate() {
            let label = self.labels[scope];
            self.emit_line_for_scope(*scope);
            self.push(&format!("__cl_c{label}:"));
            let actions = self.scope_actions.get(scope).cloned().unwrap_or_default();
            self.emit_actions(&actions);
            if index + 1 < rungs.len() {
                let next = self.labels[&rungs[index + 1]];
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
            // A view returns its pointer-and-length struct in its empty state.
            TyKind::Str | TyKind::StaticStr | TyKind::Struct(..) => {
                format!("({}){{0}}", super::types::c_type(&ret))
            }
            TyKind::Array { extents, .. } if extents.iter().any(Option::is_none) => {
                format!("({}){{0}}", super::types::c_type(&ret))
            }
            _ => "0".to_owned(),
        }
    }
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

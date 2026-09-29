//! Structured C control-flow emission.
//!
//! The ownership fixpoint runs on a flat block graph, so the graph alone says
//! only that one block jumps to another. The checker also records the shape a
//! branch came from, and this module follows that shape to print C `if`,
//! `while`, `do ... while`, `for`, and `switch` statements. A branch with no
//! recorded shape is printed as a label plus `goto`.
//!
//! # Planning and emitting labels
//!
//! Structured emission removes most labels, but a surviving `goto` needs a
//! label printed before the block it names. Because a forward `goto` may point
//! at a block that has not printed yet, the emitter walks the body twice. The
//! first walk suppresses output and records the labels the surviving `goto`s
//! need; the second walk prints the body with those labels already known.
//!
//! # Loop forms
//!
//! A `while` and a `for` enter at the branch block that tests the condition, so
//! the emitter follows the branch in place. A `do ... while` enters at its
//! body, so its test runs after the body; the emitter recognizes that entry
//! from the loop table and prints the body before the test. A `for` step lives
//! in its own block, printed as the third clause of the C `for`.

use std::collections::BTreeSet;

use crate::ir::{self, BlockId, BranchShape, LoopKind, Statement, Terminator, Value};

use super::Emitter;

/// The enclosing `break` and `continue` targets while a region is emitted.
#[derive(Clone, Copy, Default)]
struct FlowContext {
    /// Where a `continue` edge goes: a loop head, step, or condition block.
    continue_target: Option<BlockId>,
    /// Where a `break` edge goes: the innermost loop or switch exit.
    break_target: Option<BlockId>,
}

impl FlowContext {
    /// A context inside a loop body.
    fn in_loop(self, continue_target: BlockId, break_target: BlockId) -> Self {
        Self {
            continue_target: Some(continue_target),
            break_target: Some(break_target),
        }
    }

    /// A context inside a `switch` arm, which keeps the enclosing loop.
    fn in_switch(self, break_target: BlockId) -> Self {
        Self {
            break_target: Some(break_target),
            ..self
        }
    }
}

/// What a `goto` edge becomes once the surrounding structure is known.
enum Jump {
    /// A `continue` to the enclosing loop's next test.
    Continue,
    /// A `break` out of the enclosing loop or switch.
    Break,
    /// The edge reconverges with the enclosing construct.
    Fallthrough,
    /// The edge enters a structured loop, named by its branch block.
    Loop(BlockId),
    /// No surrounding construct absorbs the edge; print a label and `goto`.
    Plain,
}

impl<'a> Emitter<'a> {
    /// Emits the function body and its cleanup ladder as structured C.
    ///
    /// The first walk runs with output suppressed to record the surviving
    /// `goto` labels; the second walk prints the body with those labels known.
    pub(super) fn emit_structured_body(
        &mut self,
        function: &'a ir::Function,
        has_cleanup: bool,
    ) {
        self.prepare_regions(function);
        self.pending_labels.clear();
        self.suppress_output = true;
        self.emitted.clear();
        self.emit_regions(function, has_cleanup);
        self.suppress_output = false;
        self.emitted.clear();
        self.emit_regions(function, has_cleanup);
    }

    /// Records the reachable blocks and the entry of every structured loop.
    fn prepare_regions(&mut self, function: &'a ir::Function) {
        self.reachable = super::cleanup::reachable_blocks(function);
        self.loop_entry.clear();
        for block in &function.blocks {
            if let Terminator::Branch {
                shape: BranchShape::Loop { kind, body, .. },
                ..
            } = &block.terminator
            {
                // A `while`/`for` is entered at its test; a `do ... while` is
                // entered at its body, because its test runs after the body.
                let entry = if *kind == LoopKind::DoWhile {
                    *body
                } else {
                    block.id
                };
                self.loop_entry.insert(entry, block.id);
            }
        }
    }

    /// Emits the reachable body and then the cleanup ladder.
    fn emit_regions(&mut self, function: &'a ir::Function, has_cleanup: bool) {
        let empty = BTreeSet::new();
        let ctx = FlowContext::default();
        self.emit_flow(function, function.entry, &empty, ctx);
        // A `for ... in` loop and a `Plain` branch keep the flat form, so some
        // reachable blocks are reached only by a `goto`; print them as labelled
        // blocks in a stable order.
        for id in 0..function.blocks.len() {
            if self.reachable.contains(&id) && !self.emitted.contains(&id) {
                self.emit_flow(function, id, &empty, ctx);
            }
        }
        if has_cleanup {
            self.emit_ladder();
        }
    }

    /// Emits a deferred block's closure at its ladder rung.
    ///
    /// A deferred block is reached only through the ladder, not from the
    /// function entry, so its closure becomes the reachable set for the walk.
    pub(super) fn emit_closure(&mut self, function: &'a ir::Function, root: BlockId) {
        let closure = super::cleanup::closure_from(function, root);
        let saved = std::mem::replace(&mut self.reachable, closure.clone());
        let empty = BTreeSet::new();
        let ctx = FlowContext::default();
        self.emit_flow(function, root, &empty, ctx);
        for id in 0..function.blocks.len() {
            if closure.contains(&id) && !self.emitted.contains(&id) {
                self.emit_flow(function, id, &empty, ctx);
            }
        }
        self.reachable = saved;
    }

    /// Emits one block followed by the structured constructs that reach from it.
    ///
    /// `stop` holds the blocks that end the current linear chain, such as the
    /// join of an `if`; `ctx` carries the enclosing loop and switch targets.
    fn emit_flow(
        &mut self,
        function: &'a ir::Function,
        start: BlockId,
        stop: &BTreeSet<BlockId>,
        ctx: FlowContext,
    ) {
        let mut current = start;
        loop {
            if stop.contains(&current) || !self.reachable.contains(&current) {
                return;
            }
            if !self.emitted.insert(current) {
                return;
            }
            self.emit_block_label(function.entry, current);
            let jump_out = match &function.blocks[current].terminator {
                Terminator::Goto(target) => matches!(
                    self.classify_jump(*target, stop, ctx),
                    Jump::Break | Jump::Continue
                ),
                _ => false,
            };
            let jump = self.emit_block_body(function, current, jump_out);
            let (terminator, span) = {
                let block = &function.blocks[current];
                (&block.terminator, block.terminator_span)
            };
            match terminator {
                Terminator::Goto(target) => match self.classify_jump(*target, stop, ctx) {
                    Jump::Continue => {
                        self.push("    continue;");
                        return;
                    }
                    Jump::Break => {
                        self.push("    break;");
                        return;
                    }
                    Jump::Fallthrough => return,
                    Jump::Loop(branch) => current = self.emit_loop(function, branch, ctx),
                    Jump::Plain => {
                        self.emit_goto(*target);
                        return;
                    }
                },
                Terminator::Branch {
                    condition,
                    on_true,
                    on_false,
                    shape,
                } => match shape {
                    BranchShape::If { join } => {
                        self.emit_conditional(
                            function, span, condition, *on_true, *on_false, *join, stop, ctx,
                        );
                        current = *join;
                    }
                    BranchShape::Loop { .. } => current = self.emit_loop(function, current, ctx),
                    BranchShape::Plain => {
                        self.emit_plain_branch(span, condition, *on_true, *on_false);
                        return;
                    }
                },
                Terminator::Switch {
                    value,
                    arms,
                    default,
                    join,
                } => {
                    self.emit_switch(function, span, value, arms, *default, *join, ctx);
                    current = *join;
                }
                Terminator::Return { value, releases } => {
                    self.emit_return(span, value.as_ref(), releases, jump);
                    return;
                }
                Terminator::Unreachable => return,
            }
        }
    }

    /// Prints a `__cl_b` label for a block a surviving `goto` reaches.
    fn emit_block_label(&mut self, entry: BlockId, id: BlockId) {
        if id != entry && self.pending_labels.contains(&id) {
            self.push(&format!("__cl_b{id}:"));
        }
    }

    /// Classifies a `goto` edge against the surrounding structure.
    fn classify_jump(
        &self,
        target: BlockId,
        stop: &BTreeSet<BlockId>,
        ctx: FlowContext,
    ) -> Jump {
        if ctx.continue_target == Some(target) {
            return Jump::Continue;
        }
        if ctx.break_target == Some(target) {
            return Jump::Break;
        }
        if stop.contains(&target) {
            return Jump::Fallthrough;
        }
        if let Some(branch) = self.loop_entry.get(&target) {
            return Jump::Loop(*branch);
        }
        Jump::Plain
    }

    /// Prints a surviving `goto` and records the label it needs.
    fn emit_goto(&mut self, target: BlockId) {
        self.pending_labels.insert(target);
        self.push(&format!("    goto __cl_b{target};"));
    }

    /// Prints a branch with no structured form as a label-and-`goto` pair.
    fn emit_plain_branch(
        &mut self,
        span: crate::span::Span,
        condition: &Value,
        on_true: BlockId,
        on_false: BlockId,
    ) {
        self.emit_line_directive(span);
        self.emit_guards(condition);
        let condition_c = self.value(condition);
        self.pending_labels.insert(on_true);
        self.pending_labels.insert(on_false);
        self.push(&format!(
            "    if ({condition_c}) goto __cl_b{on_true}; else goto __cl_b{on_false};"
        ));
    }

    /// Prints an `if`/`else` and lets the caller continue at the join.
    #[allow(clippy::too_many_arguments)]
    fn emit_conditional(
        &mut self,
        function: &'a ir::Function,
        span: crate::span::Span,
        condition: &Value,
        on_true: BlockId,
        on_false: BlockId,
        join: BlockId,
        stop: &BTreeSet<BlockId>,
        ctx: FlowContext,
    ) {
        self.emit_line_directive(span);
        self.emit_guards(condition);
        let condition_c = self.value(condition);
        self.push(&format!("    if ({condition_c}) {{"));
        let mut inner = stop.clone();
        inner.insert(join);
        self.emit_flow(function, on_true, &inner, ctx);
        self.push("    } else {");
        self.emit_flow(function, on_false, &inner, ctx);
        self.push("    }");
    }

    /// Prints a structured loop and returns its exit block.
    fn emit_loop(
        &mut self,
        function: &'a ir::Function,
        branch: BlockId,
        ctx: FlowContext,
    ) -> BlockId {
        let Terminator::Branch {
            condition,
            shape: BranchShape::Loop {
                kind,
                body,
                exit,
                step,
            },
            ..
        } = &function.blocks[branch].terminator
        else {
            // Only a shaped loop branch reaches here; the fallback keeps the
            // emitter total without inventing a form.
            return branch;
        };
        let (kind, body, exit) = (*kind, *body, *exit);
        let step = *step;
        let span = function.blocks[branch].terminator_span;
        // The branch block is the printed loop header, so it is never printed
        // as a separate labelled block.
        self.emitted.insert(branch);
        let mut body_stop = BTreeSet::new();
        body_stop.insert(exit);
        match kind {
            LoopKind::While => {
                self.emit_line_directive(span);
                if ir::value_needs_guards(condition) {
                    // The guard has to run before every test, so the test moves
                    // into the body and the header becomes unconditional.
                    self.push("    while (1) {");
                    self.emit_guards(condition);
                    let condition_c = self.value(condition);
                    self.push(&format!("    if (!({condition_c})) break;"));
                } else {
                    self.emit_guards(condition);
                    let condition_c = self.value(condition);
                    self.push(&format!("    while ({condition_c}) {{"));
                }
                let inner = ctx.in_loop(branch, exit);
                self.emit_flow(function, body, &body_stop, inner);
                self.push("    }");
            }
            LoopKind::DoWhile => {
                self.emit_line_directive(span);
                self.push("    do {");
                let inner = ctx.in_loop(branch, exit);
                let mut inner_stop = body_stop.clone();
                // A `do` body falls into its test at `branch`; the continue
                // edge reaches the same block, so both are handled by the loop
                // context and the block is never printed on its own.
                inner_stop.insert(branch);
                self.emit_flow(function, body, &inner_stop, inner);
                self.emit_guards(condition);
                let condition_c = self.value(condition);
                self.push(&format!("    }} while ({condition_c});"));
            }
            LoopKind::For => {
                self.emit_line_directive(span);
                let step_c = match step {
                    Some(step) => self.for_step_text(function, step),
                    None => String::new(),
                };
                if ir::value_needs_guards(condition) {
                    // As with `while`, a guarded test moves into the body; the
                    // step still runs before each test in the C `for` header.
                    self.push(&format!("    for (; ; {step_c}) {{"));
                    self.emit_guards(condition);
                    let condition_c = self.value(condition);
                    self.push(&format!("    if (!({condition_c})) break;"));
                } else {
                    self.emit_guards(condition);
                    let condition_c = self.value(condition);
                    self.push(&format!("    for (; {condition_c}; {step_c}) {{"));
                }
                let continue_target = step.unwrap_or(branch);
                let inner = ctx.in_loop(continue_target, exit);
                self.emit_flow(function, body, &body_stop, inner);
                self.push("    }");
                if let Some(step) = step {
                    self.emitted.insert(step);
                }
            }
        }
        exit
    }

    /// Renders a `for` step's assignments as a C comma expression.
    fn for_step_text(&self, function: &'a ir::Function, step: BlockId) -> String {
        function.blocks[step]
            .statements
            .iter()
            .filter_map(|instr| match &instr.statement {
                Statement::Assign { target, value } => {
                    Some(format!("{} = {}", self.place(target), self.value(value)))
                }
                _ => None,
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Prints a C `switch` over the arm blocks, preserving IR fallthrough.
    fn emit_switch(
        &mut self,
        function: &'a ir::Function,
        span: crate::span::Span,
        value: &Value,
        arms: &[(Value, BlockId)],
        default: BlockId,
        join: BlockId,
        ctx: FlowContext,
    ) {
        self.emit_line_directive(span);
        self.emit_guards(value);
        let value_c = self.value(value);
        self.push(&format!("    switch ({value_c})"));
        self.push("    {");
        let mut clauses: Vec<(Option<&Value>, BlockId)> =
            arms.iter().map(|(label, block)| (Some(label), *block)).collect();
        // A switch with no written default uses the join as its no-match edge,
        // so there is no default clause to print.
        if default != join {
            clauses.push((None, default));
        }
        clauses.sort_by_key(|(_, block)| *block);
        let mut stop: BTreeSet<BlockId> = clauses.iter().map(|(_, block)| *block).collect();
        stop.insert(join);
        let inner = ctx.in_switch(join);
        let last = clauses.len().saturating_sub(1);
        for (index, (label, block)) in clauses.into_iter().enumerate() {
            match label {
                Some(label) => self.push(&format!("    case {}: {{", self.value(label))),
                None => self.push("    default: {"),
            }
            // The arm's own chain ends at any other arm or the join. A jump to
            // another arm is IR fallthrough, so it is left to the next `case`.
            let mut arm_stop = stop.clone();
            arm_stop.remove(&block);
            self.emit_flow(function, block, &arm_stop, inner);
            self.push("    }");
            // GCC and Clang recognize this comment as an intentional
            // fallthrough. It silences `-Werror=implicit-fallthrough` when an
            // arm reaches the next clause and is inert when the arm exited.
            if index < last {
                self.push("    /* fallthrough */");
            }
        }
        self.push("    }");
    }
}

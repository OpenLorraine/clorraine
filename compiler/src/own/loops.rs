//! Loop checking for the ownership stage.
//!
//! A loop body is analyzed to a fixed point so a value moved in one
//! iteration is seen as moved in the next. Each loop opens an exit frame so
//! a `break` or `continue` runs the cleanup of the scopes it leaves. The same
//! walk lowers the loop into the typed control-flow IR.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    /// Checks a `while` loop.
    ///
    /// The body may run zero or more times, so the state at the head is the
    /// join of the pre-state and the body's effect, iterated to a fixed point.
    /// `break` leaves through the exit block and `continue` returns to the
    /// head, each running the scopes it leaves.
    pub(super) fn check_while(&mut self, decl: &'a crate::ast::WhileStmt) {
        let head = self.new_block();
        self.terminate(Terminator::Goto(head));
        self.switch_to(head);
        self.eval(&decl.condition);
        let body_block = self.new_block();
        let exit = self.new_block();
        let condition = self.lower_value(&decl.condition);
        let shape = self.loop_shape(LoopKind::While, body_block, exit, &condition, head);
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
            shape,
        });

        let pre = self.state.clone();
        let floor = self.scopes.len();
        let (entry, post) = self.loop_entry_state(pre.clone(), |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(head),
                floor,
                floor,
            ));
            checker.check_block(&decl.body);
            checker.exits.pop();
        });

        self.state = entry.clone();
        self.exits
            .push(ExitFrame::new(exit, Some(head), floor, floor));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        let body_terminated = self.is_terminated();
        if !body_terminated {
            self.terminate(Terminator::Goto(head));
        }
        let frame = self.exits.pop().expect("loop frame");
        let always_true = condition_is_true(&decl.condition);
        self.state = loop_exit_state(entry, post, body_terminated, always_true, &frame);
        self.switch_to(exit);
    }

    /// Checks a `do ... while` loop, whose body runs at least once.
    pub(super) fn check_do_while(&mut self, decl: &'a crate::ast::DoWhileStmt) {
        let body_block = self.new_block();
        self.terminate(Terminator::Goto(body_block));
        let condition_block = self.new_block();
        let exit = self.new_block();

        let pre = self.state.clone();
        let floor = self.scopes.len();
        let (entry, post) = self.loop_entry_state(pre, |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(condition_block),
                floor,
                floor,
            ));
            checker.check_block(&decl.body);
            if !checker.is_terminated() {
                checker.eval(&decl.condition);
            }
            checker.exits.pop();
        });

        self.state = entry;
        self.exits.push(ExitFrame::new(
            exit,
            Some(condition_block),
            floor,
            floor,
        ));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(condition_block));
        }
        self.switch_to(condition_block);
        self.eval(&decl.condition);
        let condition = self.lower_value(&decl.condition);
        let shape = self.loop_shape(
            LoopKind::DoWhile,
            body_block,
            exit,
            &condition,
            condition_block,
        );
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
            shape,
        });
        let frame = self.exits.pop().expect("loop frame");
        self.state = join_breaks(post, &frame);
        self.switch_to(exit);
    }

    /// Checks a three-clause `for` loop.
    ///
    /// The header is lowered to an init (in the loop's own scope), a head that
    /// tests the condition, a body, and a step block. `continue` runs the step,
    /// so the entry state joins the pre-state, which covers the zero-iteration
    /// case.
    pub(super) fn check_for_c(&mut self, decl: &'a ForCStmt) {
        let floor = self.scopes.len();
        self.push_scope(decl.span);
        match &decl.init {
            Some(ForInit::Let(binding)) => self.check_let(binding),
            Some(ForInit::Const(binding)) => self.check_const(binding),
            Some(ForInit::Expr(expressions)) => {
                for expression in expressions {
                    self.eval(expression);
                }
            }
            None => {}
        }
        let head = self.new_block();
        self.terminate(Terminator::Goto(head));
        self.switch_to(head);
        if let Some(condition) = &decl.condition {
            self.eval(condition);
        }
        let body_block = self.new_block();
        let step_block = self.new_block();
        let exit = self.new_block();
        let condition = match &decl.condition {
            Some(condition) => self.lower_value(condition),
            None => Value::bool(Ty::boolean(), true),
        };
        let condition_ok = !value_needs_guards(&condition) && !self.block_has_statements(head);
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
            shape: BranchShape::Plain,
        });

        let pre = self.state.clone();
        let continue_floor = floor + 1;
        let (entry, post) = self.loop_entry_state(pre.clone(), |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(step_block),
                floor,
                continue_floor,
            ));
            checker.check_block(&decl.body);
            if !checker.is_terminated() {
                for step in &decl.step {
                    checker.eval(step);
                }
            }
            checker.exits.pop();
        });

        self.state = entry.clone();
        self.exits.push(ExitFrame::new(
            exit,
            Some(step_block),
            floor,
            continue_floor,
        ));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        let body_terminated = self.is_terminated();
        if !body_terminated {
            self.terminate(Terminator::Goto(step_block));
        }
        self.switch_to(step_block);
        for step in &decl.step {
            self.eval(step);
        }
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(head));
        }
        let shape = if condition_ok && self.for_step_is_structured(step_block) {
            BranchShape::Loop {
                kind: LoopKind::For,
                body: body_block,
                exit,
            }
        } else {
            BranchShape::Plain
        };
        self.builder.set_branch_shape(head, shape);
        let frame = self.exits.pop().expect("loop frame");
        let always_true = decl
            .condition
            .as_ref()
            .map_or(true, condition_is_true);
        self.state = loop_exit_state(entry, post, body_terminated, always_true, &frame);
        self.switch_to(exit);
        self.pop_scope();
    }

    /// Declares the loop element in its own scope and returns its binding.
    ///
    /// The element is bound fresh on every iteration, so the declaration and
    /// its initial state live inside the loop's speculative body and are re-run
    /// on each fixpoint pass.
    fn declare_for_in_element(&mut self, decl: &'a ForInStmt, element: &Ty) -> usize {
        self.push_scope(decl.span);
        let kind = if decl.borrow {
            BindingKind::OutSlot
        } else {
            BindingKind::Local
        };
        let id = self.declare(
            &decl.name,
            element.clone(),
            kind,
            if decl.borrow { Mode::Borrow } else { Mode::None },
            false,
            false,
        );
        self.set_place(
            id,
            PlaceState {
                initialized: true,
                ..PlaceState::default()
            },
        );
        id
    }

    /// Checks an element loop, requiring `borrow` for non-duplicable elements.
    ///
    /// The element binding and the body are analyzed to a fixed point just like
    /// the other loops, so a value moved in one iteration is seen as moved in
    /// the next. The loop may run zero times, so the head also joins the
    /// pre-loop state.
    pub(super) fn check_for_in(&mut self, decl: &'a ForInStmt) {
        let iterable_ty = self.eval(&decl.iterable);
        let element = match &iterable_ty.kind {
            TyKind::Array { element, .. } => (**element).clone(),
            _ => Ty::unknown(),
        };
        if !decl.borrow && !self.sema.table.duplicable(&element) {
            self.error(
                "E10040",
                format!(
                    "element loop requires 'borrow' for non-duplicable '{}'",
                    element.display()
                ),
                "write `for (borrow e in xs)`",
                decl.span,
            );
        }
        let pre = self.state.clone();
        let floor = self.scopes.len();
        let body_block = self.new_block();
        let exit = self.new_block();
        self.terminate(Terminator::Goto(body_block));

        let (entry, _post) = self.loop_entry_state(pre.clone(), |checker| {
            checker.declare_for_in_element(decl, &element);
            checker
                .exits
                .push(ExitFrame::new(exit, Some(body_block), floor, floor));
            checker.switch_to(body_block);
            checker.check_block(&decl.body);
            checker.exits.pop();
            checker.pop_scope();
        });

        self.state = entry.clone();
        self.declare_for_in_element(decl, &element);
        self.exits
            .push(ExitFrame::new(exit, Some(body_block), floor, floor));
        self.switch_to(body_block);
        self.check_block(&decl.body);
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(body_block));
        }
        let frame = self.exits.pop().expect("loop frame");
        self.pop_scope();
        self.state = join_breaks(entry, &frame);
        self.switch_to(exit);
    }

    /// True when a block already holds at least one statement.
    ///
    /// A loop header must be empty for a structured C loop: any statement there
    /// runs once per test, which the printed `while`/`for` header cannot carry.
    fn block_has_statements(&self, block: BlockId) -> bool {
        !self.builder.function().blocks[block].statements.is_empty()
    }

    /// The structured shape of a loop, or `Plain` when C cannot print it.
    ///
    /// A loop whose test needs a runtime guard cannot put that test in a C
    /// `while`/`for` header, because the guard must run before every test; the
    /// same is true when the header block carries statements. Such a loop keeps
    /// the flat label-and-`goto` form.
    fn loop_shape(
        &self,
        kind: LoopKind,
        body: BlockId,
        exit: BlockId,
        condition: &Value,
        header: BlockId,
    ) -> BranchShape {
        if value_needs_guards(condition) || self.block_has_statements(header) {
            BranchShape::Plain
        } else {
            BranchShape::Loop { kind, body, exit }
        }
    }

    /// True when a three-clause `for` step can print in the C `for` header.
    ///
    /// The step is a comma-separated list of C expressions, so it can only
    /// carry plain assignments whose right-hand side needs no runtime guard. A
    /// step that fails this test keeps the flat label-and-`goto` loop, where
    /// its statements and guards print in their own block.
    fn for_step_is_structured(&self, step_block: BlockId) -> bool {
        self.builder.function().blocks[step_block]
            .statements
            .iter()
            .all(|instr| match &instr.statement {
                Statement::Assign { value, .. } => {
                    !value_needs_guards(value)
                        && !matches!(value.kind, ValueKind::ArrayLit { .. })
                }
                Statement::Nop => true,
                _ => false,
            })
    }
}

/// True when evaluating a value emits a runtime guard before it can run.
///
/// A structured C loop evaluates its condition and step in the loop header, so
/// a condition or step that needs a guard (a division, a bounds check, or a
/// float-to-integer range check) cannot be printed there: the guard would have
/// to run before every test. The checker keeps such a loop in the flat
/// label-and-`goto` form instead. The walk mirrors the emitter's guard
/// placement exactly, and errs toward reporting a guard when it is unsure, so
/// a value that is wrongly called unguarded would be the only way to drop one.
fn value_needs_guards(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Binary { op, left, right } => {
            let division = matches!(op, BinaryOp::Div | BinaryOp::Rem) && left.ty.is_integer();
            division || value_needs_guards(left) || value_needs_guards(right)
        }
        ValueKind::Index { base, index } => {
            let bounds = matches!(
                base.ty.kind,
                TyKind::Array { .. } | TyKind::Str | TyKind::StaticStr
            );
            bounds || value_needs_guards(base) || value_needs_guards(index)
        }
        ValueKind::Cast { value: inner, .. } => {
            let target = &value.ty;
            let source = &inner.ty;
            let range = target.is_integer() && source.is_numeric() && !source.is_integer();
            range || value_needs_guards(inner)
        }
        ValueKind::Unary { operand, .. } => value_needs_guards(operand),
        ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            value_needs_guards(condition)
                || value_needs_guards(then_value)
                || value_needs_guards(else_value)
        }
        ValueKind::Member { base, .. } => value_needs_guards(base),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            args.iter().any(value_needs_guards)
        }
        ValueKind::StructLit { fields, .. } => {
            fields.iter().any(|(_, field)| value_needs_guards(field))
        }
        ValueKind::ArrayLit { elements, fill } => {
            elements.iter().any(value_needs_guards)
                || fill.as_deref().is_some_and(value_needs_guards)
        }
        ValueKind::Template(pieces) => pieces.iter().any(|piece| match piece {
            ValuePiece::Literal(_) => false,
            ValuePiece::Placeholder(inner) => value_needs_guards(inner),
        }),
        ValueKind::Place(_)
        | ValueKind::Int(_)
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

/// Joins the loop-entry state with the state of every `break` edge.
fn join_breaks(entry: FlowState, frame: &ExitFrame) -> FlowState {    if frame.saw_break {
        FunctionChecker::merge(entry, frame.break_state.clone())
    } else {
        entry
    }
}

/// True when a loop condition is the literal `true`.
///
/// Such a loop can only leave through a `break` or a `return`, never through a
/// false condition, so its condition edge does not reach the code after it.
fn condition_is_true(condition: &Expr) -> bool {
    matches!(condition.kind, ExprKind::Bool(true))
}

/// The state that reaches the code after a loop.
///
/// Normally the exit joins the loop-entry state with every `break` edge,
/// because the condition may be false before the body runs. When the body
/// always terminates and the condition is the literal `true`, the loop cannot
/// leave through its condition, so the only reachable exit is the body's
/// terminator, whose state is `post`.
fn loop_exit_state(
    entry: FlowState,
    post: FlowState,
    body_terminated: bool,
    condition_always_true: bool,
    frame: &ExitFrame,
) -> FlowState {
    if body_terminated && condition_always_true && frame.saw_break {
        post
    } else {
        join_breaks(entry, frame)
    }
}

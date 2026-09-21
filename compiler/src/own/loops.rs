//! Loop checking for the ownership stage.
//!
//! A loop body is analyzed to a fixed point so a value moved in one
//! iteration is seen as moved in the next. Each loop opens an exit frame so
//! a `break` or `continue` runs the cleanup of the scopes it leaves. The same
//! walk lowers the loop into the typed control-flow IR.

use super::*;
use crate::ir::value_needs_guards;

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
        let shape = self.loop_shape(LoopKind::While, body_block, exit, head);
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
        let condition_ok = !self.block_has_statements(head);
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

    /// Binds the loop element to the element at the current index.
    ///
    /// The element is bound fresh on every iteration, so the declaration and
    /// its initial state live inside the loop body. The element scope is pushed
    /// here and popped by the caller after the body, which gives the element
    /// its own per-iteration cleanup.
    fn bind_for_in_element(
        &mut self,
        decl: &'a ForInStmt,
        element: &Ty,
        iterable: &Value,
        index: &Value,
    ) -> usize {
        self.push_scope(decl.span);
        let id = self.declare(
            &decl.name,
            decl.name_span,
            element.clone(),
            BindingKind::Local,
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
        let read = Value::new(
            element.clone(),
            ValueKind::Index {
                base: Box::new(iterable.clone()),
                index: Box::new(index.clone()),
            },
        );
        let target = self.binding_place(id);
        self.emit(Statement::Assign {
            target,
            value: read,
        });
        id
    }

    /// Attaches the element write-back to the innermost exit frame.
    ///
    /// Only a `borrow` element is written back; a copied element has no view to
    /// flush. A `break` or `continue` reads this from the frame so it can run
    /// the write-back before the loop is left.
    fn set_for_in_write(
        &mut self,
        borrow: bool,
        id: usize,
        element: &Ty,
        iterable: &Value,
        index: &Value,
    ) {
        if !borrow {
            return;
        }
        if let Some(frame) = self.exits.last_mut() {
            frame.for_in = Some(ForInWrite {
                element: element.clone(),
                iterable: iterable.clone(),
                index: index.clone(),
                id,
            });
        }
    }

    /// Copies a `borrow` element's final value back into the iterable.
    ///
    /// A `borrow` element is an exclusive view of the element, so a write
    /// through it must reach the array. The binding is copied in at the top of
    /// the iteration and copied back here, at its end and on any early edge a
    /// `break` or `continue` takes.
    pub(super) fn store_for_in_element(
        &mut self,
        element: &Ty,
        iterable: &Value,
        index: &Value,
        id: usize,
    ) {
        self.bindings[id].used = true;
        let selected = Value::new(
            element.clone(),
            ValueKind::Index {
                base: Box::new(iterable.clone()),
                index: Box::new(index.clone()),
            },
        );
        let pointer = Value::new(
            element.clone(),
            ValueKind::Unary {
                op: UnaryOp::AddressOf,
                operand: Box::new(selected),
            },
        );
        let value = Value::new(element.clone(), ValueKind::Place(self.binding_place(id)));
        self.emit(Statement::StorePointer { pointer, value });
    }

    /// Checks an element loop, requiring `borrow` for non-duplicable elements.
    ///
    /// The loop lowers to an index loop: a hidden counter runs from zero to the
    /// length of the iterable, the element binds to the element at that index,
    /// and the body runs. `continue` advances the counter, so an element view
    /// is created and ended once per iteration. The element binding and the body
    /// are analyzed to a fixed point like the other loops, so a value moved in
    /// one iteration is seen as moved in the next. The loop may run zero times,
    /// so the exit joins the pre-loop state.
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

        let floor = self.scopes.len();
        self.push_scope(decl.span);
        let index_temp = self.new_temp(Ty::usize_ty());
        let index_place = Place {
            base: ir::PlaceBase::Temp(index_temp),
            projections: Vec::new(),
            ty: Ty::usize_ty(),
        };
        let index_value = Value::new(Ty::usize_ty(), ValueKind::Temp(index_temp));
        let iterable = self.lower_value(&decl.iterable);
        self.emit(Statement::Assign {
            target: index_place.clone(),
            value: Value::new(Ty::usize_ty(), ValueKind::Int(0)),
        });

        let head = self.new_block();
        self.terminate(Terminator::Goto(head));
        self.switch_to(head);
        let condition = self.for_in_condition(&iterable, &index_value);
        let body_block = self.new_block();
        let step_block = self.new_block();
        let exit = self.new_block();
        let shape = if self.block_has_statements(head) {
            BranchShape::Plain
        } else {
            BranchShape::Loop {
                kind: LoopKind::For,
                body: body_block,
                exit,
            }
        };
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
            shape,
        });

        let pre = self.state.clone();
        let continue_floor = floor + 1;
        let (entry, _post) = self.loop_entry_state(pre.clone(), |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(step_block),
                floor,
                continue_floor,
            ));
            let id = checker.bind_for_in_element(decl, &element, &iterable, &index_value);
            checker.set_for_in_write(decl.borrow, id, &element, &iterable, &index_value);
            checker.check_block(&decl.body);
            checker.exits.pop();
            checker.pop_scope();
        });

        self.state = entry.clone();
        self.exits.push(ExitFrame::new(
            exit,
            Some(step_block),
            floor,
            continue_floor,
        ));
        self.switch_to(body_block);
        let id = self.bind_for_in_element(decl, &element, &iterable, &index_value);
        self.set_for_in_write(decl.borrow, id, &element, &iterable, &index_value);
        self.check_block(&decl.body);
        if !self.is_terminated() {
            if decl.borrow {
                self.store_for_in_element(&element, &iterable, &index_value, id);
            }
            self.terminate(Terminator::Goto(step_block));
        }
        self.pop_scope();
        self.switch_to(step_block);
        self.emit_index_step(&index_place, &index_value);
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(head));
        }
        let frame = self.exits.pop().expect("loop frame");
        self.state = join_breaks(entry, &frame);
        self.switch_to(exit);
        self.pop_scope();
    }

    /// The `index < len(iterable)` condition of an element loop.
    fn for_in_condition(&self, iterable: &Value, index: &Value) -> Value {
        let length = Value::new(
            Ty::usize_ty(),
            ValueKind::Intrinsic {
                name: "len".to_owned(),
                args: vec![iterable.clone()],
                type_arg: None,
            },
        );
        Value::new(
            Ty::boolean(),
            ValueKind::Binary {
                op: BinaryOp::Lt,
                left: Box::new(index.clone()),
                right: Box::new(length),
            },
        )
    }

    /// Emits `index = index + 1` in the loop's step block.
    fn emit_index_step(&mut self, index_place: &Place, index: &Value) {
        let one = Value::new(Ty::usize_ty(), ValueKind::Int(1));
        let next = Value::new(
            Ty::usize_ty(),
            ValueKind::Binary {
                op: BinaryOp::Add,
                left: Box::new(index.clone()),
                right: Box::new(one),
            },
        );
        self.emit(Statement::Assign {
            target: index_place.clone(),
            value: next,
        });
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
    /// A loop whose condition needs a runtime guard is still structured: the
    /// emitter prints a `while (1)` header and moves the guard and the test
    /// into the body. Only a loop whose header block carries statements, which
    /// a C loop header cannot hold, keeps the flat label-and-`goto` form.
    fn loop_shape(
        &self,
        kind: LoopKind,
        body: BlockId,
        exit: BlockId,
        header: BlockId,
    ) -> BranchShape {
        if self.block_has_statements(header) {
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

/// Joins the loop-entry state with the state of every `break` edge.
fn join_breaks(entry: FlowState, frame: &ExitFrame) -> FlowState {
    if frame.saw_break {
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

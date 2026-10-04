//! Loop checking for the ownership stage.
//!
//! A loop body is analyzed to a fixed point so a value moved in one
//! iteration is seen as moved in the next. Each loop opens an exit frame so
//! a `break` or `continue` runs the cleanup of the scopes it leaves. The same
//! walk lowers the loop into the typed control-flow IR.

use super::*;
use crate::ir::{value_needs_guards, ValuePiece};

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
        let body_block = self.new_block();
        let exit = self.new_block();

        let pre = self.state.clone();
        let floor = self.scopes.len();
        let (entry, post) = self.loop_entry_state(pre.clone(), Some(head), |checker| {
            // The condition runs before the body on every iteration, so its own
            // effect (a move, for example) belongs to the back edge and must be
            // part of the fixpoint.
            checker.eval(&decl.condition);
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

        // The condition runs at the head on every iteration, so it is checked
        // with the head state: a view bound on the back edge carries its loan
        // here, and a use of it in the condition is recorded against that loan.
        self.state = entry.clone();
        self.switch_to(head);
        self.eval(&decl.condition);
        let mut condition = self.lower_value(&decl.condition);
        self.materialize_owned_calls(&mut condition);
        // The condition may move an owner, so the state after evaluating it is
        // the exit state on the false path, not the head state.
        let condition_state = self.state.clone();
        let shape = self.loop_shape(LoopKind::While, body_block, exit, head);
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
            shape,
        });

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
        self.state = loop_exit_state(condition_state, post, body_terminated, always_true, &frame);
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
        let (entry, _post) = self.loop_entry_state(pre, Some(condition_block), |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(condition_block),
                floor,
                floor,
            ));
            checker.check_block(&decl.body);
            // The condition runs after the body on the fall-through path and on
            // every continue edge, so it is evaluated with the join of both;
            // its own effect then belongs to the back edge.
            let mut condition_state = if checker.is_terminated() {
                None
            } else {
                Some(checker.state.clone())
            };
            if let Some(state) = checker.continue_states.get(&condition_block).cloned() {
                condition_state = Some(match condition_state {
                    Some(fall) => FunctionChecker::merge(fall, state),
                    None => state,
                });
            }
            if let Some(state) = condition_state {
                checker.state = state;
                checker.switch_to(condition_block);
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
        // Blocks created from here on belong to the body, so a return among
        // them is a place the condition's later releases have to travel to.
        let body_floor = self.builder.function().blocks.len();
        self.check_block(&decl.body);
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(condition_block));
        }
        self.switch_to(condition_block);
        // The condition runs on the fall-through path and on every continue
        // edge, so it is checked with the join of both.
        if let Some(state) = self.continue_states.get(&condition_block).cloned() {
            self.state = Self::merge(self.state.clone(), state);
        }
        // The condition's materialized temporaries are registered while it is
        // evaluated, so only the entries added after this point belong to it.
        let release_floor = self.pending_releases.len();
        self.eval(&decl.condition);
        let mut condition = self.lower_value(&decl.condition);
        self.materialize_owned_calls(&mut condition);
        self.append_condition_releases(body_block, body_floor, release_floor);
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
        // The condition may move an owner, so the state after evaluating it is
        // the exit state on the false path.
        let condition_state = self.state.clone();
        self.state = join_breaks(condition_state, &frame);
        self.switch_to(exit);
    }

    /// Sends a do-while condition's new releases to the body's returns.
    ///
    /// The body is checked before the condition, so a `return` inside it copied
    /// a release list that does not name the temporaries the condition stores.
    /// The pending list is sliced at `release_floor` (its length just before
    /// the condition was evaluated) and reversed to newest-first; the body's
    /// return blocks — the blocks created since `body_floor` that end in a
    /// return and are reachable from the body's entry — receive the copies.
    /// A deferred block runs through the scope cleanup ladder rather than a
    /// CFG edge, so the walk also follows the `defer` markers found in the
    /// reachable blocks: a return inside an executed defer must release the
    /// condition's temporaries exactly like a direct return.
    /// The same release events are recorded at each return, so the borrow
    /// checker sees that a view of a condition temporary cannot outlive it.
    fn append_condition_releases(
        &mut self,
        body_block: BlockId,
        body_floor: usize,
        release_floor: usize,
    ) {
        if self.pending_releases.len() <= release_floor {
            return;
        }
        let successors = super::liveness::successors(self.builder.function());
        let mut returns: Vec<BlockId> = Vec::new();
        let mut seen = vec![false; successors.len()];
        let mut stack = vec![body_block];
        while let Some(block) = stack.pop() {
            let visited = &mut seen[block as usize];
            if *visited {
                continue;
            }
            *visited = true;
            if block as usize >= body_floor
                && matches!(
                    self.builder.function().blocks[block as usize].terminator,
                    Terminator::Return { .. }
                )
            {
                returns.push(block);
            }
            // The marker in a registering block names its deferred block, and
            // that block holds the deferred statements and their terminators.
            // It is not a terminator successor, so it joins the walk here.
            let deferred: Vec<BlockId> = self.builder.function().blocks[block as usize]
                .statements
                .iter()
                .filter_map(|instr| match &instr.statement {
                    Statement::Defer { block: defer_block, .. } => Some(*defer_block),
                    _ => None,
                })
                .collect();
            for defer_block in deferred {
                stack.push(defer_block);
            }
            for next in &successors[block as usize] {
                stack.push(*next);
            }
        }
        if returns.is_empty() {
            return;
        }
        returns.sort_unstable();
        let releases: Vec<Release> = self.pending_releases[release_floor..]
            .iter()
            .rev()
            .cloned()
            .collect();
        let current = self.builder.current();
        self.builder.append_return_releases(&returns, &releases);
        for block in &returns {
            self.switch_to(*block);
            self.record_release_events(&releases);
        }
        self.switch_to(current);
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
        let body_block = self.new_block();
        let step_block = self.new_block();
        let exit = self.new_block();

        let pre = self.state.clone();
        let continue_floor = floor + 1;
        let (entry, post) = self.loop_entry_state(pre.clone(), Some(step_block), |checker| {
            // The condition runs before the body on every iteration, so its own
            // effect belongs to the back edge and must be part of the fixpoint.
            if let Some(condition) = &decl.condition {
                checker.eval(condition);
            }
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(step_block),
                floor,
                continue_floor,
            ));
            checker.check_block(&decl.body);
            // The step runs on the fall-through path and on every continue
            // edge, so it is checked with the join of both; its effect then
            // feeds the head on the next iteration. The block is left
            // unterminated so `loop_entry_state` sees the step's state as the
            // back edge rather than as an exiting edge.
            let mut step_state = if checker.is_terminated() {
                None
            } else {
                Some(checker.state.clone())
            };
            if let Some(state) = checker.continue_states.get(&step_block).cloned() {
                step_state = Some(match step_state {
                    Some(fall) => FunctionChecker::merge(fall, state),
                    None => state,
                });
            }
            if let Some(state) = step_state {
                checker.state = state;
                checker.switch_to(step_block);
                for step in &decl.step {
                    checker.check_expr_statement(step);
                }
            }
            checker.exits.pop();
        });

        // The condition runs at the head on every iteration, so it is checked
        // with the head state: a view bound on the back edge carries its loan
        // here, and a use of it in the condition is recorded against that loan.
        self.state = entry.clone();
        self.switch_to(head);
        if let Some(condition) = &decl.condition {
            self.eval(condition);
        }
        let condition = match &decl.condition {
            Some(condition) => {
                let mut lowered = self.lower_value(condition);
                self.materialize_owned_calls(&mut lowered);
                lowered
            }
            None => Value::bool(Ty::boolean(), true),
        };
        // The condition may move an owner, so the state after evaluating it is
        // the exit state on the false path, not the head state.
        let condition_state = self.state.clone();
        let condition_ok = !self.block_has_statements(head);
        self.terminate(Terminator::Branch {
            condition,
            on_true: body_block,
            on_false: exit,
            shape: BranchShape::Plain,
        });

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
        if let Some(state) = self.continue_states.get(&step_block).cloned() {
            self.state = Self::merge(self.state.clone(), state);
        }
        for step in &decl.step {
            self.check_expr_statement(step);
        }
        if !self.is_terminated() {
            self.terminate(Terminator::Goto(head));
        }
        let shape = if condition_ok && self.for_step_is_structured(step_block) {
            BranchShape::Loop {
                kind: LoopKind::For,
                body: body_block,
                exit,
                step: Some(step_block),
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
        self.state = loop_exit_state(condition_state, post, body_terminated, always_true, &frame);
        self.switch_to(exit);
        self.pop_scope();
    }

    /// Binds the loop element to the element at the current index.
    ///
    /// The element is bound fresh on every iteration, so the declaration and
    /// its initial state live inside the loop body. The element scope is pushed
    /// here and popped by the caller after the body, which gives the element
    /// its own per-iteration cleanup.
    ///
    /// A `borrow` element is an alias of the iterable element: its place is
    /// rooted at the compiler temporary that holds the element's address, and
    /// that address is refreshed at the top of every iteration. Reads and
    /// writes through the binding therefore reach the iterable directly, and
    /// no copy-in or write-back step exists. A copied element keeps the
    /// per-iteration copy into its own local.
    fn bind_for_in_element(
        &mut self,
        decl: &'a ForInStmt,
        element: &Ty,
        iterable: &Value,
        index: &Value,
        address_temp: ir::TempId,
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
        let selected = Value::new(
            element.clone(),
            ValueKind::Index {
                base: Box::new(iterable.clone()),
                index: Box::new(index.clone()),
            },
        );
        if decl.borrow {
            self.bindings[id].base = Some(ir::PlaceBase::DerefTemp {
                temp: address_temp,
                ty: element.clone(),
            });
            // The binding is the element itself, so the iterable keeps owning
            // the value even when the body never touches the binding; mark it
            // used so an unused element alias is not reported as a dead local.
            self.bindings[id].used = true;
            let target = Place {
                base: ir::PlaceBase::Temp(address_temp),
                projections: Vec::new(),
                ty: Ty::raw(element.clone()),
            };
            let address = Value::new(
                Ty::raw(element.clone()),
                ValueKind::Unary {
                    op: UnaryOp::AddressOf,
                    operand: Box::new(selected),
                },
            );
            self.emit(Statement::Assign {
                target,
                value: address,
            });
        } else {
            let target = self.binding_place(id);
            self.emit(Statement::Assign {
                target,
                value: selected,
            });
        }
        id
    }

    /// Checks an element loop, requiring `borrow` for non-duplicable elements.
    ///
    /// The loop lowers to an index loop: a hidden counter runs from zero to the
    /// length of the iterable, the element binds to the element at that index,
    /// and the body runs. `continue` advances the counter, so an element view
    /// is created and ended once per iteration; a `borrow` element refreshes
    /// its aliasing address temporary there. The element binding and the body
    /// are analyzed to a fixed point like the other loops, so a value moved in
    /// one iteration is seen as moved in the next. The loop may run zero times,
    /// so the exit joins the pre-loop state.
    pub(super) fn check_for_in(&mut self, decl: &'a ForInStmt) {
        let iterable_ty = self.eval(&decl.iterable);
        let element = iterable_ty.iterated_element();
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
        // A `borrow` element aliases the iterable through one address
        // temporary per loop. It is allocated before both the speculative and
        // the real binding pass, exactly like the index temporary, so the two
        // passes agree on its id.
        let address_temp = self.new_temp(Ty::raw(element.clone()));
        let index_place = Place {
            base: ir::PlaceBase::Temp(index_temp),
            projections: Vec::new(),
            ty: Ty::usize_ty(),
        };
        let index_value = Value::new(Ty::usize_ty(), ValueKind::Temp(index_temp));
        let iterable = self.lower_value(&decl.iterable);
        // A materialized ownerless conditional iterable moves the owned arm
        // the condition selects, so that arm becomes maybe-moved.
        if self.place_target(&decl.iterable).is_none() {
            self.mark_read_conditional_move(&decl.iterable);
        }
        let iterable = self.materialize_for_in_iterable(&decl.iterable, iterable);
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
                step: Some(step_block),
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
        let (entry, _post) = self.loop_entry_state(pre.clone(), Some(step_block), |checker| {
            checker.switch_to(body_block);
            checker.exits.push(ExitFrame::new(
                exit,
                Some(step_block),
                floor,
                continue_floor,
            ));
            checker.bind_for_in_element(
                decl,
                &element,
                &iterable,
                &index_value,
                address_temp,
            );
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
        self.bind_for_in_element(decl, &element, &iterable, &index_value, address_temp);
        self.check_block(&decl.body);
        if !self.is_terminated() {
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

    /// Materializes an ownerless iterable so the loop reads one stored value.
    ///
    /// A place-rooted iterable already names storage, so it is used as it is.
    /// An ownerless value — a call result, a literal, or a conditional — has
    /// no storage of its own, and the loop reads its length and its elements,
    /// so leaving it inline would evaluate it once per iteration and render an
    /// invalid element expression. The value is stored once into a temporary
    /// before the loop, and the temporary is released after the loop
    /// statement. A type that mentions a generic parameter materializes like
    /// any other candidate: the temporary is declared with the substituted
    /// type per instance and a maybe-owning release is a no-op for a
    /// duplicable one.
    fn materialize_for_in_iterable(&mut self, expr: &Expr, value: Value) -> Value {
        if self.place_target(expr).is_some() {
            return value;
        }
        let ty = self.materialized_ty(&value);
        self.materialize_value_type(value, ty)
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
            BranchShape::Loop {
                kind,
                body,
                exit,
                step: None,
            }
        }
    }

    /// Materializes every owned call result in an eagerly evaluated value.
    ///
    /// A loop condition is printed in a C header and an eagerly stored read
    /// source is filled by a statement, so neither can leave an owned call
    /// result inline: it has to travel through a temporary, whose guarded
    /// store drops the previous value before a re-store and releases the last
    /// value after the enclosing construct. A call that returns a view, a
    /// slice, a duplicable value, or an unknown type is left inline because it
    /// needs no release. A call that is already the direct stored value of a
    /// materialized operand is left alone; wrapping it again would store it
    /// twice and register two releases.
    pub(super) fn materialize_owned_calls(&mut self, value: &mut Value) {
        if matches!(
            value.kind,
            ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
        ) && (self.owns_drop(&value.ty) || self.maybe_owns(&value.ty))
        {
            let call = value.clone();
            *value = self.materialize_guard_operand(call);
            return;
        }
        match &mut value.kind {
            ValueKind::Binary { left, right, .. } => {
                self.materialize_owned_calls(left);
                self.materialize_owned_calls(right);
            }
            ValueKind::Index { base, index } => {
                self.materialize_owned_calls(base);
                self.materialize_owned_calls(index);
            }
            ValueKind::Cast { value: inner, .. } => {
                self.materialize_owned_calls(inner);
            }
            ValueKind::Unary { operand, .. } => {
                self.materialize_owned_calls(operand);
            }
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.materialize_owned_calls(condition);
                self.materialize_owned_calls(then_value);
                self.materialize_owned_calls(else_value);
            }
            ValueKind::Member { base, .. } => {
                self.materialize_owned_calls(base);
            }
            ValueKind::Call { args, receiver, .. } => {
                if let Some(receiver) = receiver {
                    self.materialize_owned_calls(receiver);
                }
                for arg in args {
                    self.materialize_owned_calls(arg);
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    self.materialize_owned_calls(arg);
                }
            }
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.materialize_owned_calls(field);
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.materialize_owned_calls(element);
                }
                if let Some(fill) = fill {
                    self.materialize_owned_calls(fill);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.materialize_owned_calls(inner);
                    }
                }
            }
            ValueKind::Materialize { value: inner, .. } => {
                // The direct stored value already runs once through the
                // materialize guard; a call nested deeper still needs its own
                // temporary and release.
                if !matches!(
                    inner.kind,
                    ValueKind::Call { .. } | ValueKind::ClosureCall { .. }
                ) {
                    self.materialize_owned_calls(inner);
                }
            }
            ValueKind::Closure { captures, .. } => {
                for capture in captures {
                    self.materialize_owned_calls(capture);
                }
            }
            ValueKind::ClosureCall { callable, args } => {
                self.materialize_owned_calls(callable);
                for arg in args {
                    self.materialize_owned_calls(arg);
                }
            }
            ValueKind::Place(_)
            | ValueKind::Int(_)
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
/// `condition_exit` is the state after the condition was evaluated at the head:
/// it is what reaches the code after the loop when the condition is false,
/// including any move the condition made. The exit joins that state with every
/// `break` edge. When the body always terminates and the condition is the
/// literal `true`, the loop cannot leave through its condition, so the only
/// reachable exits are the `break` edges; their join is used directly, because
/// the last body path is not the state of every break. `post` is unused in that
/// case and kept out so a break-only path cannot hide another break path's
/// moves or loans.
fn loop_exit_state(
    condition_exit: FlowState,
    _post: FlowState,
    body_terminated: bool,
    condition_always_true: bool,
    frame: &ExitFrame,
) -> FlowState {
    if body_terminated && condition_always_true {
        if frame.saw_break {
            frame.break_state.clone()
        } else {
            condition_exit
        }
    } else {
        join_breaks(condition_exit, frame)
    }
}

//! Typed control-flow intermediate representation.
//!
//! The checker lowers each function body into this representation so the
//! ownership and borrow decisions become explicit statements rather than
//! something the emitter has to rediscover. The statement set is fixed by the
//! codegen contract: `assign`, `move`, `borrow`, `call`, `drop`, `return`,
//! `branch`, `switch`, and `defer`. Every owned place is released by an
//! explicit [`Statement::Drop`] on each path that leaves its scope, and a
//! moved value is zeroed so its later drop is a no-op.
//!
//! A function is a graph of [`BasicBlock`]s. Each block holds a list of
//! statements and exactly one [`Terminator`]; the terminator is what makes the
//! block a graph node, so control flow is never implicit. The representation
//! is also the unit-test surface: a test can build a small function and pin
//! its block and statement shape without involving the C emitter.

use crate::ast::{BinaryOp, UnaryOp};

/// Index of a block inside a [`Function`].
pub type BlockId = usize;

/// Index of a compiler-created temporary inside a function.
pub type TempId = u32;

/// A memory location the program can read, write, move, or drop.
///
/// A place is a base plus zero or more projections. `x` is a local base with
/// no projection, `p.left` is a local base with a field projection, and
/// `xs[0]` is a local base with an element projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// The root of the place: a local, a parameter, a global, or a temporary.
    pub base: PlaceBase,
    /// Field and element selections applied from the base outwards.
    pub projections: Vec<Projection>,
}

impl Place {
    /// A place that names a plain local.
    pub fn local(name: impl Into<String>) -> Self {
        Self {
            base: PlaceBase::Local(name.into()),
            projections: Vec::new(),
        }
    }

    /// A place that names a parameter.
    pub fn param(name: impl Into<String>) -> Self {
        Self {
            base: PlaceBase::Param(name.into()),
            projections: Vec::new(),
        }
    }

    /// A place that names a module-level binding.
    pub fn global(name: impl Into<String>) -> Self {
        Self {
            base: PlaceBase::Global(name.into()),
            projections: Vec::new(),
        }
    }

    /// True when the place names a top-level local or parameter.
    pub fn is_plain(&self) -> bool {
        self.projections.is_empty()
    }
}

/// What a place is rooted in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceBase {
    /// A local binding.
    Local(String),
    /// A function parameter.
    Param(String),
    /// A module-level `let` or `const`.
    Global(String),
    /// A compiler-created temporary.
    Temp(TempId),
}

/// One selection applied to a place.
///
/// The ordering is total so a set of projection paths can be kept sorted; the
/// order itself has no meaning beyond giving narrowing facts a stable key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Projection {
    /// A named struct field.
    Field(String),
    /// An array or slice element; `None` is a run-time index.
    Element(Option<usize>),
}

/// The permission a [`Statement::Borrow`] records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoanKind {
    /// Many readers, no writer.
    Shared,
    /// One writer, no other access.
    Exclusive,
}

/// A value produced or consumed by a statement.
///
/// The IR keeps places, literals, and calls; it does not duplicate the whole
/// expression grammar. A form the checker does not need to name in the graph
/// is represented by a temporary, which is sufficient because the ownership
/// decisions have already been made by the time the statement is emitted.
#[derive(Clone, Debug)]
pub enum Value {
    /// A place read as a value.
    Place(Place),
    /// An integer literal.
    Int(i128),
    /// A boolean literal.
    Bool(bool),
    /// The `null` literal.
    Null,
    /// A string literal.
    Str(String),
    /// A call, with its arguments lowered in order.
    Call {
        /// Name of the called function or intrinsic.
        callee: String,
        /// Lowered arguments in source order.
        args: Vec<Value>,
    },
    /// A prefix or postfix unary operator.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: Box<Value>,
    },
    /// A binary operator.
    Binary {
        /// The operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<Value>,
        /// Right operand.
        right: Box<Value>,
    },
    /// A conversion to the named type.
    Cast {
        /// Display spelling of the target type.
        to: String,
        /// The converted value.
        value: Box<Value>,
    },
    /// A struct or array literal whose elements were lowered separately.
    Aggregate,
    /// A reference to a compiler-created temporary.
    Temp(TempId),
}

/// One statement of the typed control-flow IR.
#[derive(Clone, Debug)]
pub enum Statement {
    /// Copy a duplicable value, or store a fresh result; the previous value of
    /// `target` is dropped first.
    Assign {
        /// The place written.
        target: Place,
        /// The value stored.
        value: Value,
    },
    /// Transfer an owned value: copy into `target`, then zero `source`.
    Move {
        /// The place written.
        target: Place,
        /// The place whose ownership is transferred.
        source: Place,
    },
    /// Begin a loan over a place. This is a checker statement and has no
    /// runtime code of its own.
    Borrow {
        /// The borrowed place.
        place: Place,
        /// The permission the loan grants.
        kind: LoanKind,
    },
    /// Invoke a function or intrinsic.
    Call {
        /// Name of the callee.
        callee: String,
        /// Lowered arguments in source order.
        args: Vec<Value>,
        /// The place that receives an owned result, when there is one.
        result: Option<Place>,
    },
    /// Release an owned place, null-safely and exactly once.
    Drop(Place),
    /// A registered cleanup entry. The referenced block holds the deferred
    /// statements and joins the scope's ladder in registration order.
    Defer {
        /// The block holding the deferred statements.
        block: BlockId,
    },
    /// A placeholder with no effect, used where a statement was elided.
    Nop,
}

/// How a basic block transfers control.
#[derive(Clone, Debug)]
pub enum Terminator {
    /// Unconditional jump.
    Goto(BlockId),
    /// Two-way branch on a condition.
    Branch {
        /// The tested condition.
        condition: Value,
        /// Taken when the condition is true.
        on_true: BlockId,
        /// Taken when the condition is false.
        on_false: BlockId,
    },
    /// Multi-way branch with an explicit default.
    Switch {
        /// The matched value.
        value: Value,
        /// Literal-to-block arms.
        arms: Vec<(i128, BlockId)>,
        /// The block reached when no arm matches.
        default: BlockId,
    },
    /// Leave the function with an optional value.
    Return(Option<Value>),
    /// No successor; only reachable in a malformed graph.
    Unreachable,
}

/// A block of statements with a single terminator.
#[derive(Clone, Debug)]
pub struct BasicBlock {
    /// Index of this block in the enclosing function.
    pub id: BlockId,
    /// Statements executed in order before the terminator.
    pub statements: Vec<Statement>,
    /// The block's single exit.
    pub terminator: Terminator,
}

/// A lowered function: its parameters and its control-flow graph.
#[derive(Clone, Debug, Default)]
pub struct Function {
    /// The function's declared name.
    pub name: String,
    /// Parameter places in declaration order.
    pub params: Vec<Place>,
    /// The blocks of the graph; the entry block is `entry`.
    pub blocks: Vec<BasicBlock>,
    /// Index of the entry block.
    pub entry: BlockId,
}

/// Builds a [`Function`] block by block.
///
/// The builder always keeps one current block. Straight-line code is emitted
/// into it; starting a branch means creating the successor blocks and ending
/// the current one with a terminator, so a finished function never has a block
/// without an explicit exit.
///
/// The builder is cloneable so a speculative analysis pass can emit into a
/// copy and then discard it, leaving the real graph untouched.
#[derive(Clone)]
pub struct Builder {
    function: Function,
    current: BlockId,
}

impl Builder {
    /// Starts a function whose entry block is empty.
    pub fn new(name: impl Into<String>, params: Vec<Place>) -> Self {
        let entry = BasicBlock {
            id: 0,
            statements: Vec::new(),
            terminator: Terminator::Unreachable,
        };
        Self {
            function: Function {
                name: name.into(),
                params,
                blocks: vec![entry],
                entry: 0,
            },
            current: 0,
        }
    }

    /// The block statements are currently emitted into.
    pub fn current(&self) -> BlockId {
        self.current
    }

    /// Appends a statement to the current block.
    ///
    /// A statement emitted after the block already has a real exit is
    /// unreachable, so it is dropped rather than appended past the terminator.
    pub fn emit(&mut self, statement: Statement) {
        if matches!(
            self.function.blocks[self.current].terminator,
            Terminator::Unreachable
        ) {
            self.function.blocks[self.current].statements.push(statement);
        }
    }

    /// Creates an empty block and returns its index without switching to it.
    pub fn new_block(&mut self) -> BlockId {
        let id = self.function.blocks.len();
        self.function.blocks.push(BasicBlock {
            id,
            statements: Vec::new(),
            terminator: Terminator::Unreachable,
        });
        id
    }

    /// Makes `id` the block further statements go into.
    pub fn switch_to(&mut self, id: BlockId) {
        self.current = id;
    }

    /// Ends the current block with a terminator.
    pub fn terminate(&mut self, terminator: Terminator) {
        self.function.blocks[self.current].terminator = terminator;
    }

    /// Borrows the function built so far.
    pub fn function(&self) -> &Function {
        &self.function
    }

    /// Finishes the build and returns the function.
    pub fn finish(self) -> Function {
        self.function
    }
}

/// One concrete instantiation of a generic function.
///
/// A generic body is checked once; every call that instantiates it produces an
/// instance, and codegen emits one copy per distinct instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    /// The generic function's name.
    pub function: String,
    /// The concrete type arguments, in declaration order.
    pub args: Vec<String>,
}

impl Instance {
    /// The key that identifies this instance uniquely.
    ///
    /// Two calls with the same function and the same type arguments share one
    /// emitted copy, so the key is the deduplication identity codegen uses.
    pub fn dedupe_key(&self) -> String {
        format!("{}<{}>", self.function, self.args.join(", "))
    }
}

/// A copy a generic body must perform at each instantiation.
///
/// A writable lend parameter instantiated with a duplicable type binds a copy
/// instead of a lend, so the body records the obligation here and the checker
/// discharges it against each concrete instance. The instance key names the
/// instantiation site so a diagnostic can point at it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyObligation {
    /// The generic function that carries the obligation.
    pub function: String,
    /// The instantiation the obligation was discharged for.
    pub instance: String,
    /// The parameter whose value must be copied.
    pub parameter: String,
    /// The concrete duplicable type bound at the instantiation.
    pub ty: String,
}

/// The instantiation set and copy obligations of a generic program.
///
/// This is the input codegen consumes: the set of concrete function copies to
/// emit, each with a stable deduplication key, plus the copies a generic body
/// owes. The type arguments are stored as their rendered spellings because the
/// emitter names symbols from them.
#[derive(Clone, Debug, Default)]
pub struct Monomorphization {
    instances: Vec<Instance>,
    obligations: Vec<CopyObligation>,
}

impl Monomorphization {
    /// Records an instance, returning true when it was not already present.
    pub fn record(&mut self, instance: Instance) -> bool {
        if self
            .instances
            .iter()
            .any(|known| known.dedupe_key() == instance.dedupe_key())
        {
            return false;
        }
        self.instances.push(instance);
        true
    }

    /// Every distinct instance recorded so far.
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    /// Records a copy obligation the generic body owes.
    pub fn require_copy(&mut self, obligation: CopyObligation) {
        self.obligations.push(obligation);
    }

    /// Every copy obligation recorded so far.
    pub fn obligations(&self) -> &[CopyObligation] {
        &self.obligations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_line_function_keeps_one_block_with_an_explicit_exit() {
        let mut builder = Builder::new("demo", vec![Place::param("s")]);
        let local = Place::local("t");
        builder.emit(Statement::Move {
            target: local.clone(),
            source: Place::param("s"),
        });
        builder.emit(Statement::Drop(local.clone()));
        builder.terminate(Terminator::Return(None));
        let function = builder.finish();

        assert_eq!(function.entry, 0);
        assert_eq!(function.blocks.len(), 1);
        assert_eq!(function.params, vec![Place::param("s")]);
        assert!(matches!(function.blocks[0].statements[0], Statement::Move { .. }));
        assert!(matches!(function.blocks[0].statements[1], Statement::Drop(_)));
        assert!(matches!(function.blocks[0].terminator, Terminator::Return(None)));
    }

    #[test]
    fn branching_creates_separate_blocks_and_a_join() {
        let mut builder = Builder::new("pick", vec![Place::param("flag")]);
        let then_block = builder.new_block();
        let else_block = builder.new_block();
        let join = builder.new_block();
        builder.emit(Statement::Nop);
        builder.terminate(Terminator::Branch {
            condition: Value::Bool(true),
            on_true: then_block,
            on_false: else_block,
        });
        builder.switch_to(then_block);
        builder.terminate(Terminator::Goto(join));
        builder.switch_to(else_block);
        builder.terminate(Terminator::Goto(join));
        builder.switch_to(join);
        builder.terminate(Terminator::Return(None));

        let function = builder.finish();
        assert_eq!(function.blocks.len(), 4);
        assert!(matches!(
            function.blocks[0].terminator,
            Terminator::Branch { .. }
        ));
        assert!(matches!(function.blocks[3].terminator, Terminator::Return(None)));
    }

    #[test]
    fn a_projected_place_reports_its_shape() {
        let place = Place {
            base: PlaceBase::Local("pair".to_owned()),
            projections: vec![Projection::Field("left".to_owned())],
        };
        assert!(!place.is_plain());
        assert_eq!(Place::local("pair").is_plain(), true);
    }

    #[test]
    fn instances_deduplicate_by_function_and_type_arguments() {
        let mut mono = Monomorphization::default();
        let first = Instance {
            function: "identity".to_owned(),
            args: vec!["i32".to_owned()],
        };
        assert!(mono.record(first.clone()));
        assert!(!mono.record(first.clone()));
        assert!(mono.record(Instance {
            function: "identity".to_owned(),
            args: vec!["u8".to_owned()],
        }));
        assert_eq!(mono.instances().len(), 2);
        assert_eq!(first.dedupe_key(), "identity<i32>");
    }

    #[test]
    fn copy_obligations_name_their_instantiation() {
        let mut mono = Monomorphization::default();
        mono.require_copy(CopyObligation {
            function: "append".to_owned(),
            instance: "append<u8>".to_owned(),
            parameter: "value".to_owned(),
            ty: "u8".to_owned(),
        });
        assert_eq!(mono.obligations().len(), 1);
        assert_eq!(mono.obligations()[0].instance, "append<u8>");
    }
}

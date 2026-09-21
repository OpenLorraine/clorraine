//! Typed control-flow intermediate representation.
//!
//! The checker lowers each function body into this representation so the
//! ownership and borrow decisions become explicit statements rather than
//! something the emitter has to rediscover. The statement set is fixed by the
//! codegen contract: `assign`, `store`, `move`, `borrow`, `call`, `drop`,
//! `return`, `branch`, `switch`, and `defer`. Every owned place is released by an
//! explicit [`Statement::Drop`] on each path that leaves its scope, and a
//! moved value is zeroed so its later drop is a no-op.
//!
//! Cleanup is grouped by lexical scope. Each function records its scopes, from
//! the body outwards, in [`Function::scopes`]; every drop and `defer` names the
//! scope whose exit emits it, and a [`Statement::ScopeEnd`] marks the normal
//! fall-through end of a scope. An edge that leaves through `return`, `break`,
//! or `continue` emits a [`Statement::ScopeExit`] for each scope it leaves and
//! carries the drops itself. This is enough for the emitter to build one
//! cleanup label per owning scope, innermost first, and to run the cleanup of a
//! scope inside a loop on each iteration.
//!
//! Every place and value carries its checked type, and every statement and
//! terminator carries the source span it came from. The type lets the emitter
//! declare a local without re-inferring it, and the span lets it emit a
//! `#line` directive before each construct.
//!
//! A function is a graph of [`BasicBlock`]s. Each block holds a list of
//! statements and exactly one [`Terminator`]; the terminator is what makes the
//! block a graph node, so control flow is never implicit. The representation
//! is also the unit-test surface: a test can build a small function and pin
//! its block and statement shape without involving the C emitter.

use crate::ast::{BinaryOp, Mode, UnaryOp};
use crate::span::Span;
use crate::types::Ty;

mod control;
mod guards;
mod mono;

pub use control::{BranchShape, LoopKind};
pub use guards::value_needs_guards;
pub use mono::{CopyObligation, Instance, Monomorphization};

/// Index of a block inside a [`Function`].
pub type BlockId = usize;

/// Index of a lexical scope inside a [`Function`].
///
/// Scope `0` is the function body; each nested block opens a higher-numbered
/// scope. The number is the index into [`Function::scopes`], so it is also the
/// key a drop, a `defer`, or a scope-end marker uses to name its scope.
pub type ScopeId = usize;

/// Index of a compiler-created temporary inside a function.
pub type TempId = u32;

/// A memory location the program can read, write, move, or drop.
///
/// A place is a base plus zero or more projections, plus the type of the value
/// it addresses. `x` is a local base with no projection, `p.left` is a local
/// base with a field projection, and `xs[0]` is a local base with an element
/// projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// The root of the place: a local, a parameter, a global, or a temporary.
    pub base: PlaceBase,
    /// Field and element selections applied from the base outwards.
    pub projections: Vec<Projection>,
    /// The checked type of the value the place addresses.
    pub ty: Ty,
}

impl Place {
    /// A place that names a plain local, with an as-yet unknown type.
    pub fn local(name: impl Into<String>) -> Self {
        Self::with_base(PlaceBase::Local(name.into()))
    }

    /// A place that names a parameter, with an as-yet unknown type.
    pub fn param(name: impl Into<String>) -> Self {
        Self::with_base(PlaceBase::Param(name.into()))
    }

    /// A place that names a module-level binding, with an unknown type.
    pub fn global(name: impl Into<String>) -> Self {
        Self::with_base(PlaceBase::Global(name.into()))
    }

    /// A place with no projections and an unknown type.
    fn with_base(base: PlaceBase) -> Self {
        Self {
            base,
            projections: Vec::new(),
            ty: Ty::unknown(),
        }
    }

    /// Attaches the checked type to the place.
    pub fn with_type(mut self, ty: Ty) -> Self {
        self.ty = ty;
        self
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

/// A value produced or consumed by a statement, with its checked type.
///
/// The shape is a [`ValueKind`]; the wrapper keeps the type next to it so a
/// caller reads `.ty` and `.kind` without a separate lookup.
#[derive(Clone, Debug)]
pub struct Value {
    /// The checked type of the value.
    pub ty: Ty,
    /// The value's shape.
    pub kind: ValueKind,
}

impl Value {
    /// Builds a value from a type and a shape.
    pub fn new(ty: Ty, kind: ValueKind) -> Self {
        Self { ty, kind }
    }

    /// A boolean value of the given type.
    pub fn bool(ty: Ty, value: bool) -> Self {
        Self::new(ty, ValueKind::Bool(value))
    }

    /// A place read as a value, taking its type from the place.
    pub fn place(place: Place) -> Self {
        let ty = place.ty.clone();
        Self::new(ty, ValueKind::Place(place))
    }
}

/// The value shapes the checker names in the graph.
///
/// The emitter renders each shape straight to a C expression, so every form
/// the source can spell has its own shape here. A form the checker does not
/// name explicitly becomes a temporary, which is sufficient because the
/// ownership decisions have already been made by the time the statement is
/// emitted.
#[derive(Clone, Debug)]
pub enum ValueKind {
    /// A place read as a value.
    Place(Place),
    /// An integer literal.
    Int(i128),
    /// A floating-point literal.
    Float(f64),
    /// A byte literal.
    Char(u8),
    /// A boolean literal.
    Bool(bool),
    /// The `null` literal.
    Null,
    /// A string literal.
    Str(String),
    /// A dotted name that is not a place: an enum member such as `E.V`.
    Path(Vec<String>),
    /// A call, with its arguments lowered in order.
    Call {
        /// Name of the called function or intrinsic.
        callee: String,
        /// The concrete type arguments of a generic call, in order.
        type_args: Vec<Ty>,
        /// Lowered arguments in source order.
        args: Vec<Value>,
        /// The callable value, when the callee is a `func`-typed field rather
        /// than a named function. The emitter calls it directly instead of
        /// prefixing a receiver as the first argument.
        receiver: Option<Box<Value>>,
    },
    /// A field selection, which keeps a run-time base intact.
    Member {
        /// The selected-from value.
        base: Box<Value>,
        /// The field name.
        name: String,
    },
    /// An element selection, which keeps a run-time index intact.
    Index {
        /// The indexed value.
        base: Box<Value>,
        /// The index value.
        index: Box<Value>,
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
    /// A conditional `cond ? then : else`.
    Conditional {
        /// The tested condition.
        condition: Box<Value>,
        /// The value when the condition holds.
        then_value: Box<Value>,
        /// The value when the condition fails.
        else_value: Box<Value>,
    },
    /// A struct literal, with the written fields in source order.
    StructLit {
        /// The struct's simple name.
        name: String,
        /// The written field names and their values.
        fields: Vec<(String, Value)>,
    },
    /// An array literal, with any trailing fill value.
    ArrayLit {
        /// The written elements in order.
        elements: Vec<Value>,
        /// The value every remaining element takes, when a fill is written.
        fill: Option<Box<Value>>,
    },
    /// A template string with its literal and placeholder pieces.
    Template(Vec<ValuePiece>),
    /// A reserved built-in call such as `len`, `sizeof`, `assert`, or `panic`.
    Intrinsic {
        /// The intrinsic name.
        name: String,
        /// The value arguments in order.
        args: Vec<Value>,
        /// The type argument of `sizeof`, when one is written.
        type_arg: Option<Ty>,
    },
    /// A struct or array literal the checker did not need to name explicitly.
    Aggregate,
    /// A reference to a compiler-created temporary.
    Temp(TempId),
}

/// One piece of a template string.
#[derive(Clone, Debug)]
pub enum ValuePiece {
    /// Literal text between placeholders.
    Literal(String),
    /// A `{...}` placeholder and its lowered value.
    Placeholder(Value),
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
    /// Store a value through a pointer-derived lvalue.
    ///
    /// A store target with a run-time index is lowered as the address of the
    /// element plus a store through it. The address carries the index and its
    /// bounds check, so the element selection is preserved instead of being
    /// reduced to a fixed element.
    StorePointer {
        /// The address of the destination.
        pointer: Value,
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
    /// Empty a place whose value moved into a call, without releasing it.
    ///
    /// A `take` argument transfers ownership to the callee, so the caller's
    /// binding must be emptied afterward or its scope drop would release the
    /// value a second time. The statement zeroes the place only; unlike
    /// [`Statement::Move`] it never drops the old value, because the callee
    /// already owns it.
    MoveOut {
        /// The place whose value the callee took.
        place: Place,
    },
    /// Begin a loan over a place. This is a checker statement and has no
    /// runtime code of its own.
    Borrow {
        /// The borrowed place.
        place: Place,
        /// The permission the loan grants.
        kind: LoanKind,
    },
    /// Invoke a function or intrinsic for its effect, discarding any result.
    Call {
        /// Name of the callee.
        callee: String,
        /// The concrete type arguments of a generic call, in order.
        type_args: Vec<Ty>,
        /// Lowered arguments in source order.
        args: Vec<Value>,
        /// The callable value, when the callee is a `func`-typed field rather
        /// than a named function.
        receiver: Option<Box<Value>>,
        /// The place that receives an owned result, when there is one.
        result: Option<Place>,
    },
    /// Release an owned place, null-safely and exactly once.
    Drop {
        /// The place released.
        place: Place,
        /// The scope whose exit emits this drop.
        scope: ScopeId,
        /// Registration order of the dropped binding within the function,
        /// shared with `defer` entries so a scope's cleanup can interleave.
        order: u32,
    },
    /// A registered cleanup entry. The referenced block holds the deferred
    /// statements and joins the scope's ladder in registration order.
    Defer {
        /// The block holding the deferred statements.
        block: BlockId,
        /// The scope whose exit runs the deferred block.
        scope: ScopeId,
        /// Registration order of the `defer` within the function, shared with
        /// drops so a scope's cleanup can interleave.
        order: u32,
    },
    /// The normal fall-through end of a scope, emitted just before the scope's
    /// cleanup.
    ///
    /// An edge that leaves through `return`, `break`, or `continue` carries its
    /// cleanup itself and emits no marker, so the emitter can tell a scope that
    /// ends normally from one that is jumped out of.
    ScopeEnd {
        /// The scope that ends here.
        scope: ScopeId,
    },
    /// A control-flow edge that leaves a scope before its normal end.
    ///
    /// A `return`, `break`, or `continue` inside a scope jumps out before the
    /// scope's closing brace, so it emits one marker per scope it leaves, on
    /// the edge. The marker has no runtime effect of its own; it tells the
    /// emitter which scopes' cleanup the edge runs, so a scope whose only
    /// cleanup is a `defer` is not skipped. The explicit drops that follow
    /// carry the actual work.
    ScopeExit {
        /// The scope the edge leaves.
        scope: ScopeId,
    },
    /// A placeholder with no effect, used where a statement was elided.
    Nop,
}

/// One statement with the source span it was lowered from.
#[derive(Clone, Debug)]
pub struct Instr {
    /// The source span of the statement, used for `#line` emission.
    pub span: Span,
    /// The statement itself.
    pub statement: Statement,
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
        /// The structured shape this branch came from, when it has one.
        shape: BranchShape,
    },
    /// Multi-way branch with an explicit default.
    Switch {
        /// The matched value.
        value: Value,
        /// Case label and target block arms, in source order. The label is a
        /// constant value so an enum member keeps its `E.V` spelling.
        arms: Vec<(Value, BlockId)>,
        /// The block reached when no arm matches.
        default: BlockId,
        /// The block the whole `switch` continues at after it ends.
        join: BlockId,
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
    pub statements: Vec<Instr>,
    /// The block's single exit.
    pub terminator: Terminator,
    /// The source span of the terminator, used for `#line` emission.
    pub terminator_span: Span,
}

/// One function parameter: its name, type, and declared mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    /// The written name.
    pub name: String,
    /// The checked parameter type.
    pub ty: Ty,
    /// The written mode.
    pub mode: Mode,
}

/// One local binding: its name and checked type, in declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Local {
    /// The written name.
    pub name: String,
    /// The checked local type.
    pub ty: Ty,
}

/// How one lexical scope nests and where its cleanup runs.
///
/// A scope owns the bindings declared directly in it and the `defer` entries it
/// registers. Recording the nesting in the IR lets the emitter build the
/// cleanup ladder from the lowered function alone, without walking the source
/// tree again. `in_loop` is set for a scope that opens inside a loop body: its
/// cleanup belongs to each iteration, not only to the end of the function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeInfo {
    /// The enclosing scope, or `None` for the function body scope.
    pub parent: Option<ScopeId>,
    /// True when the scope opens inside a loop body.
    pub in_loop: bool,
    /// Source span of the scope's closing point, used to attribute generated
    /// cleanup to the scope's closing line.
    pub close_span: Span,
}

/// A lowered function: its signature, locals, scopes, and control-flow graph.
#[derive(Clone, Debug)]
pub struct Function {
    /// The function's declared name.
    pub name: String,
    /// The dotted module the function is declared in.
    pub module: String,
    /// Parameters in declaration order.
    pub params: Vec<Param>,
    /// The declared return type.
    pub ret: Ty,
    /// Locals in declaration order.
    pub locals: Vec<Local>,
    /// The type of each compiler-created temporary, by index.
    pub temps: Vec<Ty>,
    /// The lexical scopes of the body, indexed by [`ScopeId`]. Scope `0` is the
    /// function body and has no parent; nested scopes have a higher id.
    pub scopes: Vec<ScopeInfo>,
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
    span: Span,
}

impl Builder {
    /// Starts a function whose entry block is empty.
    pub fn new(name: impl Into<String>, params: Vec<Param>, ret: Ty) -> Self {
        let entry = BasicBlock {
            id: 0,
            statements: Vec::new(),
            terminator: Terminator::Unreachable,
            terminator_span: Span::default(),
        };
        Self {
            function: Function {
                name: name.into(),
                module: String::new(),
                params,
                ret,
                locals: Vec::new(),
                temps: Vec::new(),
                scopes: Vec::new(),
                blocks: vec![entry],
                entry: 0,
            },
            current: 0,
            span: Span::default(),
        }
    }

    /// Sets the span attached to the next statement or terminator.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
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
            let span = self.span;
            self.function.blocks[self.current]
                .statements
                .push(Instr { span, statement });
        }
    }

    /// Creates an empty block and returns its index without switching to it.
    pub fn new_block(&mut self) -> BlockId {
        let id = self.function.blocks.len();
        self.function.blocks.push(BasicBlock {
            id,
            statements: Vec::new(),
            terminator: Terminator::Unreachable,
            terminator_span: Span::default(),
        });
        id
    }

    /// Makes `id` the block further statements go into.
    pub fn switch_to(&mut self, id: BlockId) {
        self.current = id;
    }

    /// Ends the current block with a terminator.
    pub fn terminate(&mut self, terminator: Terminator) {
        let span = self.span;
        self.function.blocks[self.current].terminator = terminator;
        self.function.blocks[self.current].terminator_span = span;
    }

    /// Records a declared local in declaration order.
    pub fn add_local(&mut self, name: impl Into<String>, ty: Ty) {
        self.function.locals.push(Local {
            name: name.into(),
            ty,
        });
    }

    /// Records a compiler-created temporary and returns its index.
    pub fn add_temp(&mut self, ty: Ty) -> TempId {
        let id = self.function.temps.len() as TempId;
        self.function.temps.push(ty);
        id
    }

    /// Records a lexical scope and returns its id.
    ///
    /// `parent` is the enclosing scope, or `None` for the function body scope.
    /// The id is the index into the function's scope list, so it stays valid
    /// for the rest of the build even after the scope closes. `close_span` is
    /// the source range of the scope's closing point, so the emitter can
    /// attribute the scope's generated cleanup to the line that closes it.
    pub fn add_scope(
        &mut self,
        parent: Option<ScopeId>,
        in_loop: bool,
        close_span: Span,
    ) -> ScopeId {
        let id = self.function.scopes.len();
        self.function.scopes.push(ScopeInfo {
            parent,
            in_loop,
            close_span,
        });
        id
    }

    /// Replaces the structured shape of an already-terminated branch block.
    ///
    /// A three-clause `for` record can only be classified after its step block
    /// is lowered, so the checker sets the shape once the step is known. A
    /// block that does not end in a branch is left untouched.
    pub fn set_branch_shape(&mut self, block: BlockId, shape: BranchShape) {
        if let Terminator::Branch {
            shape: current, ..
        } = &mut self.function.blocks[block].terminator
        {
            *current = shape;
        }
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


#[cfg(test)]
mod tests;

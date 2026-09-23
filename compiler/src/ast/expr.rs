//! Expression nodes and operator tables.
//!
//! Split from the declaration nodes to keep each file focused; the names are
//! re-exported from [`crate::ast`].

use super::*;

/// An expression with its source span.
pub struct Expr {
    /// Expression shape.
    pub kind: ExprKind,
    /// Span of the whole expression.
    pub span: Span,
}

/// Expression shapes.
pub enum ExprKind {
    /// Integer literal.
    Int(u64),
    /// Float literal.
    Float(f64),
    /// Character literal byte.
    Char(u8),
    /// String literal bytes.
    Str(Vec<u8>),
    /// `true` or `false`.
    Bool(bool),
    /// `null`.
    Null,
    /// Simple identifier.
    Name(String),
    /// Dotted identifier.
    Qualified(QualifiedName),
    /// Call with optional explicit type arguments.
    Call {
        /// Called expression.
        callee: Box<Expr>,
        /// Explicit type arguments.
        type_args: Vec<Type>,
        /// Arguments in order.
        args: Vec<Arg>,
    },
    /// Indexing.
    Index {
        /// Indexed expression.
        base: Box<Expr>,
        /// Index expression.
        index: Box<Expr>,
    },
    /// Member access.
    Member {
        /// Base expression.
        base: Box<Expr>,
        /// Member name.
        name: String,
    },
    /// Prefix or postfix unary operator.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        operand: Box<Expr>,
    },
    /// Binary operator application.
    Binary {
        /// Operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// Assignment, which is syntactically an expression with a void result.
    Assign {
        /// Operator.
        op: AssignOp,
        /// Assignment target.
        target: Box<Expr>,
        /// Assigned value.
        value: Box<Expr>,
    },
    /// Conditional `? :` expression.
    Conditional {
        /// Condition.
        condition: Box<Expr>,
        /// Value when true.
        then_value: Box<Expr>,
        /// Value when false.
        else_value: Box<Expr>,
    },
    /// Scalar cast `T(expr)`.
    Cast {
        /// Target type.
        ty: Type,
        /// Converted expression.
        value: Box<Expr>,
    },
    /// Raw-value cast `raw T(expr)`.
    RawCast {
        /// Target type.
        ty: Type,
        /// Converted expression.
        value: Box<Expr>,
    },
    /// Struct literal.
    StructLiteral {
        /// Literal type name.
        name: QualifiedName,
        /// Explicit type arguments.
        type_args: Vec<Type>,
        /// Field initializers in source order.
        fields: Vec<FieldInit>,
    },
    /// Array literal, either a list or a fill form.
    ArrayLiteral {
        /// Listed elements.
        elements: Vec<Expr>,
        /// Fill value of the `[elem; fill]` and `[; fill]` forms.
        fill: Option<Box<Expr>>,
    },
    /// Lambda expression.
    Lambda {
        /// Capture list.
        captures: Vec<Capture>,
        /// Typed parameters.
        params: Vec<LambdaParam>,
        /// Body shape.
        body: LambdaBody,
    },
    /// `unsafe` prefix operator.
    Unsafe(Box<Expr>),
    /// Call of a reserved built-in name.
    IntrinsicCall {
        /// Intrinsic name.
        name: String,
        /// Arguments in the order the intrinsic expects them.
        args: Vec<IntrinsicArg>,
    },
    /// Template string with parsed placeholder expressions.
    Template(Vec<TemplatePiece>),
    /// Placeholder for a failed parse; the parser already recorded an error.
    Error,
}

/// One call argument with its optional marker.
pub struct Arg {
    /// Optional `const`/`borrow`/`take` marker.
    pub marker: Mode,
    /// Argument expression.
    pub value: Expr,
    /// Span of the whole argument.
    pub span: Span,
}

/// One struct-literal field initializer.
pub struct FieldInit {
    /// Field name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Field value.
    pub value: Expr,
    /// Span of the whole initializer.
    pub span: Span,
}

/// One lambda capture.
pub struct Capture {
    /// `borrow`/`take` marker, or no marker.
    pub mode: Mode,
    /// Captured name.
    pub name: String,
    /// Span of the capture.
    pub span: Span,
}

/// One lambda parameter, which always has a type.
pub struct LambdaParam {
    /// Parameter name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Parameter type.
    pub ty: Type,
    /// Span of the whole parameter.
    pub span: Span,
}

/// A lambda body is an expression or a block.
pub enum LambdaBody {
    /// Expression body.
    Expr(Box<Expr>),
    /// Block body.
    Block(Block),
}

/// One argument of an intrinsic call.
pub enum IntrinsicArg {
    /// Expression argument.
    Expr(Expr),
    /// Type argument, used by `sizeof` and `offsetof`.
    Type(Type),
}

/// One piece of a parsed template.
pub enum TemplatePiece {
    /// Literal bytes.
    Literal(Vec<u8>),
    /// Placeholder expression.
    Placeholder(Expr),
}

/// Prefix and postfix unary operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// `!`
    Not,
    /// `~`
    BitNot,
    /// `-`
    Neg,
    /// `&` address-of
    AddressOf,
    /// `*` raw dereference
    Deref,
    /// `++` prefix
    PreIncrement,
    /// `--` prefix
    PreDecrement,
    /// `++` postfix
    PostIncrement,
    /// `--` postfix
    PostDecrement,
}

impl UnaryOp {
    /// True for the increment and decrement operators.
    ///
    /// These read and write their operand, so they carry a side effect while
    /// the other unary operators are pure.
    pub fn is_update(self) -> bool {
        matches!(
            self,
            Self::PreIncrement | Self::PreDecrement | Self::PostIncrement | Self::PostDecrement
        )
    }
}

/// Binary operators in precedence order from loosest to tightest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    /// `||`
    LogicalOr,
    /// `&&`
    LogicalAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `&`
    BitAnd,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
}

impl BinaryOp {
    /// Binding power of the operator; higher binds tighter.
    pub fn precedence(self) -> u8 {
        match self {
            Self::LogicalOr => 1,
            Self::LogicalAnd => 2,
            Self::BitOr => 3,
            Self::BitXor => 4,
            Self::BitAnd => 5,
            Self::Eq | Self::Ne => 6,
            Self::Lt | Self::Le | Self::Gt | Self::Ge => 7,
            Self::Shl | Self::Shr => 8,
            Self::Add | Self::Sub => 9,
            Self::Mul | Self::Div | Self::Rem => 10,
        }
    }
}

/// Assignment operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    /// `=`
    Assign,
    /// `+=`
    Add,
    /// `-=`
    Sub,
    /// `*=`
    Mul,
    /// `/=`
    Div,
    /// `%=`
    Rem,
    /// `&=`
    BitAnd,
    /// `|=`
    BitOr,
    /// `^=`
    BitXor,
    /// `<<=`
    Shl,
    /// `>>=`
    Shr,
}

impl AssignOp {
    /// The binary operator a compound assignment applies, or `None` for `=`.
    ///
    /// `x op= y` is equivalent to `x = x op y`, so the operator stage and the
    /// code generator both need the same mapping from the assignment spelling
    /// to the binary operation it performs.
    pub fn binary_op(self) -> Option<BinaryOp> {
        match self {
            Self::Assign => None,
            Self::Add => Some(BinaryOp::Add),
            Self::Sub => Some(BinaryOp::Sub),
            Self::Mul => Some(BinaryOp::Mul),
            Self::Div => Some(BinaryOp::Div),
            Self::Rem => Some(BinaryOp::Rem),
            Self::BitAnd => Some(BinaryOp::BitAnd),
            Self::BitOr => Some(BinaryOp::BitOr),
            Self::BitXor => Some(BinaryOp::BitXor),
            Self::Shl => Some(BinaryOp::Shl),
            Self::Shr => Some(BinaryOp::Shr),
        }
    }
}

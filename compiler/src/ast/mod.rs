//! Abstract syntax tree.
//!
//! The tree mirrors the frozen grammar one-to-one, from the module declaration
//! down to individual expressions. Every node carries the source span it was
//! parsed from; nothing here resolves names or types. Templates keep their
//! placeholder expressions parsed so later stages can type and lower them.

use crate::span::Span;

mod expr;

pub use expr::*;

/// A parsed file: its module declaration, imports, and top-level items.
pub struct Program {
    /// The `module` declaration. An empty name marks a missing declaration,
    /// which the module stage reports.
    pub module: ModuleDecl,
    /// A further `module` declaration found after the first declaration has
    /// started. The parser records it without reporting so the module stage
    /// owns the "must be first" diagnostic.
    pub misplaced_module: Option<ModuleDecl>,
    /// Import statements in source order.
    pub imports: Vec<ImportDecl>,
    /// Declarations in source order.
    pub items: Vec<Item>,
}

/// `module A.B.C;`
pub struct ModuleDecl {
    /// Dotted module name.
    pub name: QualifiedName,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A dotted name such as `std.io` or `Direction.UP`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedName {
    /// Name segments in order.
    pub segments: Vec<String>,
    /// Span of the whole name.
    pub span: Span,
}

impl QualifiedName {
    /// Joins the segments with dots.
    pub fn to_dotted(&self) -> String {
        self.segments.join(".")
    }
}

/// `import { name as alias, ... } from A.B;`
pub struct ImportDecl {
    /// Imported names.
    pub items: Vec<ImportItem>,
    /// Module the names come from.
    pub from: QualifiedName,
    /// Span of the whole statement.
    pub span: Span,
}

/// One imported name with an optional rename.
pub struct ImportItem {
    /// Imported name.
    pub name: String,
    /// Span of the imported name.
    pub name_span: Span,
    /// Local rename from an `as` clause.
    pub alias: Option<String>,
    /// Span of the whole item.
    pub span: Span,
}

/// A top-level declaration.
pub enum Item {
    /// `func` with a body.
    Func(FuncDecl),
    /// `comptime func` with a body.
    Comptime(FuncDecl),
    /// `struct` declaration.
    Struct(StructDecl),
    /// `enum` declaration.
    Enum(EnumDecl),
    /// `handle` declaration.
    Handle(HandleDecl),
    /// `ffi` block.
    Ffi(FfiBlock),
    /// `intrinsic func` declaration.
    Intrinsic(FuncSignature),
    /// Module-level `let`.
    GlobalLet(GlobalLetDecl),
    /// Module-level `const`.
    GlobalConst(GlobalConstDecl),
}

/// A function declaration with its body.
pub struct FuncDecl {
    /// `export` marker.
    pub exported: bool,
    /// Function name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Generic parameters.
    pub type_params: Vec<TypeParam>,
    /// Parameters in order.
    pub params: Vec<Param>,
    /// Return type, `void` or a value type.
    pub returns: Type,
    /// Body block.
    pub body: Block,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A parameter or a call-site marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// No mode written.
    None,
    /// `const` read-only lend.
    Const,
    /// `borrow` writable lend.
    Borrow,
    /// `take` ownership transfer.
    Take,
}

/// One function parameter.
pub struct Param {
    /// Parameter mode.
    pub mode: Mode,
    /// Parameter name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Parameter type.
    pub ty: Type,
    /// Span of the whole parameter.
    pub span: Span,
}

/// A generic parameter with an optional callable bound.
pub struct TypeParam {
    /// Parameter name.
    pub name: String,
    /// Callable bound.
    pub bound: Option<CallableBound>,
    /// Span of the whole parameter.
    pub span: Span,
}

/// `func(T, ...): R` bound of a type parameter.
pub struct CallableBound {
    /// Bound parameter types.
    pub params: Vec<Type>,
    /// Bound return type.
    pub returns: Box<Type>,
    /// Span of the whole bound.
    pub span: Span,
}

/// `struct` declaration.
pub struct StructDecl {
    /// `export` marker.
    pub exported: bool,
    /// Struct name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Generic parameters.
    pub type_params: Vec<TypeParam>,
    /// Optional struct-level `freedBy` clause; the semantic stage rejects it.
    pub freed_by: Option<QualifiedName>,
    /// Fields in declaration order.
    pub fields: Vec<StructField>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// One struct field.
pub struct StructField {
    /// Field name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// `volatile` marker, allowed only on fields.
    pub volatile: bool,
    /// Field type.
    pub ty: Type,
    /// Span of the whole field.
    pub span: Span,
}

/// `enum` declaration.
pub struct EnumDecl {
    /// `export` marker.
    pub exported: bool,
    /// Enum name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Required backing integer type.
    pub backing: Type,
    /// Members in order.
    pub members: Vec<EnumMember>,
    /// Span of the whole declaration.
    pub span: Span,
}

/// One enum member.
pub struct EnumMember {
    /// Member name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Explicit value, when written.
    pub value: Option<Expr>,
    /// Span of the whole member.
    pub span: Span,
}

/// `handle Name freedBy Deleter;`
pub struct HandleDecl {
    /// `export` marker.
    pub exported: bool,
    /// Handle name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Deleter name.
    pub freed_by: QualifiedName,
    /// Span of the whole declaration.
    pub span: Span,
}

/// `ffi Name { ... }`
pub struct FfiBlock {
    /// `export` marker, parsed and rejected later.
    pub exported: bool,
    /// Block name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Declared signatures.
    pub decls: Vec<FuncSignature>,
    /// Span of the whole block.
    pub span: Span,
}

/// A bodyless signature, used by `ffi` blocks and `intrinsic` declarations.
pub struct FuncSignature {
    /// Function name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Generic parameters. Only a compiler-known `intrinsic func` may declare
    /// them; an `ffi` signature always leaves this empty.
    pub type_params: Vec<TypeParam>,
    /// Parameters in order.
    pub params: Vec<Param>,
    /// Return type.
    pub returns: Type,
    /// Span of the whole declaration.
    pub span: Span,
}

/// Module-level `let`.
pub struct GlobalLetDecl {
    /// `export` marker.
    pub exported: bool,
    /// `volatile` marker.
    pub volatile: bool,
    /// Binding name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Declared type.
    pub ty: Type,
    /// Mandatory initializer.
    pub value: Expr,
    /// Span of the whole declaration.
    pub span: Span,
}

/// Module-level `const`.
pub struct GlobalConstDecl {
    /// `export` marker.
    pub exported: bool,
    /// Constant name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Declared type.
    pub ty: Type,
    /// Mandatory constant initializer.
    pub value: Expr,
    /// Span of the whole declaration.
    pub span: Span,
}

/// A type in any type position.
pub struct Type {
    /// Type shape.
    pub kind: TypeKind,
    /// `?` nullability suffix.
    pub nullable: bool,
    /// Span of the whole type.
    pub span: Span,
}

/// Type shapes.
pub enum TypeKind {
    /// `void`.
    Void,
    /// A fixed-width scalar.
    Scalar(ScalarType),
    /// `str` view.
    Str,
    /// `cstring` view.
    CString,
    /// `static str` value.
    StaticStr,
    /// `raw` followed by its target atom.
    Raw(Box<Type>),
    /// A named type with optional explicit type arguments.
    Named {
        /// Dotted type name.
        name: QualifiedName,
        /// Explicit type arguments.
        args: Vec<Type>,
    },
    /// Fixed array or slice view of an element type.
    Array {
        /// Element type.
        element: Box<Type>,
        /// Bracketed extents; `None` is the omitted slice extent.
        extents: Vec<Extent>,
    },
    /// Exact-signature function pointer type.
    Function {
        /// Parameter types with their modes.
        params: Vec<FunctionTypeParam>,
        /// Return type.
        returns: Box<Type>,
    },
    /// Placeholder for a failed parse; the parser already recorded an error.
    Error,
}

/// The fixed-width scalar types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarType {
    /// `i8`
    I8,
    /// `i16`
    I16,
    /// `i32`
    I32,
    /// `u8`
    U8,
    /// `u16`
    U16,
    /// `u32`
    U32,
    /// `f32`
    F32,
    /// `f64`
    F64,
    /// `bool`
    Bool,
    /// `usize`
    Usize,
    /// `isize`
    Isize,
}

/// One bracketed extent; the size is absent for a slice view.
pub struct Extent {
    /// Constant expression between the brackets.
    pub size: Option<Expr>,
    /// Span of the extent.
    pub span: Span,
}

/// One parameter of a function pointer type.
pub struct FunctionTypeParam {
    /// Parameter mode.
    pub mode: Mode,
    /// Parameter type.
    pub ty: Type,
    /// Span of the whole parameter.
    pub span: Span,
}

/// A statement block.
pub struct Block {
    /// Statements in order.
    pub statements: Vec<Stmt>,
    /// Span including the braces.
    pub span: Span,
}

/// One statement.
pub enum Stmt {
    /// `let` binding.
    Let(LetStmt),
    /// `const` binding.
    Const(ConstStmt),
    /// Expression statement.
    Expr(Expr),
    /// `if` statement.
    If(IfStmt),
    /// `while` loop.
    While(WhileStmt),
    /// `do ... while` loop.
    DoWhile(DoWhileStmt),
    /// Three-clause `for` loop.
    ForC(ForCStmt),
    /// Element `for` loop.
    ForIn(ForInStmt),
    /// `return` with optional value.
    Return {
        /// Returned value, when present.
        value: Option<Expr>,
        /// Span of the statement.
        span: Span,
    },
    /// `break;`
    Break(Span),
    /// `continue;`
    Continue(Span),
    /// `switch` statement.
    Switch(SwitchStmt),
    /// `defer { ... }`
    Defer(Block),
    /// `unsafe { ... }`
    Unsafe(Block),
    /// Bare block statement.
    Block(Block),
}

/// `let` binding.
pub struct LetStmt {
    /// Binding name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Optional declared type.
    pub ty: Option<Type>,
    /// Optional initializer.
    pub value: Option<Expr>,
    /// Span of the whole statement.
    pub span: Span,
}

/// `const` binding.
pub struct ConstStmt {
    /// Binding name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Optional declared type.
    pub ty: Option<Type>,
    /// Initializer.
    pub value: Expr,
    /// Span of the whole statement.
    pub span: Span,
}

/// `if` statement.
pub struct IfStmt {
    /// Condition expression.
    pub condition: Expr,
    /// Body of the branch.
    pub then_block: Block,
    /// `else` branch, which is another `if` or a block.
    pub else_branch: Option<ElseBranch>,
    /// Span of the whole statement.
    pub span: Span,
}

/// The two `else` shapes.
pub enum ElseBranch {
    /// `else if`
    If(Box<IfStmt>),
    /// `else { ... }`
    Block(Block),
}

/// `while` loop.
pub struct WhileStmt {
    /// Loop condition.
    pub condition: Expr,
    /// Loop body.
    pub body: Block,
    /// Span of the whole statement.
    pub span: Span,
}

/// `do ... while` loop.
pub struct DoWhileStmt {
    /// Loop body.
    pub body: Block,
    /// Loop condition.
    pub condition: Expr,
    /// Span of the whole statement.
    pub span: Span,
}

/// Three-clause `for` loop.
pub struct ForCStmt {
    /// Initializer clause.
    pub init: Option<ForInit>,
    /// Condition clause.
    pub condition: Option<Expr>,
    /// Step expressions.
    pub step: Vec<Expr>,
    /// Loop body.
    pub body: Block,
    /// Span of the whole statement.
    pub span: Span,
}

/// The three initializer shapes of a three-clause `for`.
pub enum ForInit {
    /// `let` initializer.
    Let(LetStmt),
    /// `const` initializer.
    Const(ConstStmt),
    /// Expression list initializer.
    Expr(Vec<Expr>),
}

/// Element `for` loop.
pub struct ForInStmt {
    /// True for the `borrow` form.
    pub borrow: bool,
    /// Bound element name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// Sequence expression.
    pub iterable: Expr,
    /// Loop body.
    pub body: Block,
    /// Span of the whole statement.
    pub span: Span,
}

/// `switch` statement.
pub struct SwitchStmt {
    /// Value being matched.
    pub scrutinee: Expr,
    /// Clauses in source order.
    pub clauses: Vec<SwitchClause>,
    /// Span of the whole statement.
    pub span: Span,
}

/// One `case` or `default` clause.
pub struct SwitchClause {
    /// Clause label.
    pub label: SwitchLabel,
    /// Statements of the clause body.
    pub statements: Vec<Stmt>,
    /// Span of the whole clause.
    pub span: Span,
}

/// `case` value or `default`.
pub enum SwitchLabel {
    /// `case value:`
    Case(Expr),
    /// `default:`
    Default,
}

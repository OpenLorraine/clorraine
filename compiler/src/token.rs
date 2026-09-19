//! Token vocabulary: keywords, punctuation, literals, and templates.
//!
//! These types are the interface between the lexer and the parser. They carry
//! no scanning logic; see [`crate::lex`] for the scanner that produces them.

use crate::span::Span;

/// The 50 reserved keywords.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyword {
    Module,
    Import,
    Export,
    From,
    Func,
    Comptime,
    Return,
    Struct,
    Enum,
    Handle,
    FreedBy,
    Ffi,
    Intrinsic,
    Unsafe,
    Volatile,
    Raw,
    Let,
    Const,
    Borrow,
    Take,
    Defer,
    If,
    Else,
    While,
    Do,
    For,
    In,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    I8,
    I16,
    I32,
    U8,
    U16,
    U32,
    F32,
    F64,
    Bool,
    Void,
    Usize,
    Isize,
    Str,
    Cstring,
    Static,
    True,
    False,
    Null,
}

impl Keyword {
    /// Maps an identifier spelling to its keyword, or `None` when the
    /// spelling is an ordinary identifier.
    pub fn from_spelling(spelling: &str) -> Option<Self> {
        let keyword = match spelling {
            "module" => Self::Module,
            "import" => Self::Import,
            "export" => Self::Export,
            "from" => Self::From,
            "func" => Self::Func,
            "comptime" => Self::Comptime,
            "return" => Self::Return,
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "handle" => Self::Handle,
            "freedBy" => Self::FreedBy,
            "ffi" => Self::Ffi,
            "intrinsic" => Self::Intrinsic,
            "unsafe" => Self::Unsafe,
            "volatile" => Self::Volatile,
            "raw" => Self::Raw,
            "let" => Self::Let,
            "const" => Self::Const,
            "borrow" => Self::Borrow,
            "take" => Self::Take,
            "defer" => Self::Defer,
            "if" => Self::If,
            "else" => Self::Else,
            "while" => Self::While,
            "do" => Self::Do,
            "for" => Self::For,
            "in" => Self::In,
            "switch" => Self::Switch,
            "case" => Self::Case,
            "default" => Self::Default,
            "break" => Self::Break,
            "continue" => Self::Continue,
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "bool" => Self::Bool,
            "void" => Self::Void,
            "usize" => Self::Usize,
            "isize" => Self::Isize,
            "str" => Self::Str,
            "cstring" => Self::Cstring,
            "static" => Self::Static,
            "true" => Self::True,
            "false" => Self::False,
            "null" => Self::Null,
            _ => return None,
        };
        Some(keyword)
    }
}

/// Punctuation and operator tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Punct {
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Bang,
    Lt,
    Gt,
    Eq,
    Dot,
    Comma,
    Colon,
    Semi,
    Question,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    EqEq,
    BangEq,
    LtEq,
    GtEq,
    Shl,
    Shr,
    AmpAmp,
    PipePipe,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    ShrEq,
    FatArrow,
    PlusPlus,
    MinusMinus,
}

/// Names that only an `intrinsic func` declaration may bind. They are
/// identifier-shaped tokens; the parser recognizes them in call position.
pub const RESERVED_BUILTINS: &[&str] = &[
    "sizeof",
    "offsetof",
    "len",
    "embed",
    "panic",
    "assert",
    "readVolatile8",
    "readVolatile16",
    "readVolatile32",
    "writeVolatile8",
    "writeVolatile16",
    "writeVolatile32",
];

/// True when `name` is a reserved built-in name.
pub fn is_reserved_builtin(name: &str) -> bool {
    RESERVED_BUILTINS.contains(&name)
}

/// One piece of a template string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplatePart {
    /// Decoded literal bytes (escapes and `{{`/`}}` already resolved).
    Literal(Vec<u8>),
    /// Source span of a placeholder expression, between its braces.
    Placeholder(Span),
}

/// Token classification.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    /// An ordinary identifier, including the reserved built-in names.
    Ident(String),
    /// A reserved keyword.
    Keyword(Keyword),
    /// A decimal, hexadecimal, or binary integer literal.
    Int(u64),
    /// A floating-point literal.
    Float(f64),
    /// A character literal; exactly one encoded byte.
    Char(u8),
    /// A string literal holding its decoded bytes.
    Str(Vec<u8>),
    /// A template string and its pieces.
    Template(Vec<TemplatePart>),
    /// Punctuation or an operator.
    Punct(Punct),
    /// A malformed construct; the lexer already recorded the diagnostic.
    Error,
    /// End of the token stream.
    Eof,
}

/// One token with the source span it was scanned from.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    /// Token classification.
    pub kind: TokenKind,
    /// Source range of the token.
    pub span: Span,
}

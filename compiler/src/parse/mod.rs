//! Parser: tokens to the abstract syntax tree.
//!
//! The parser is recursive descent for declarations and statements plus a
//! precedence-climbing chain for expressions. Ambiguous forms follow the
//! grammar's disambiguation notes: generic type arguments are recognized by a
//! bounded speculative parse, `{` after a name in expression position is a
//! struct literal, and the diagnostic-only starts (`union`, a misplaced
//! `volatile`) are recognized to produce their targeted diagnostics.
//!
//! Recovery is token-based: a syntax error records one `E20000` diagnostic and
//! the parser skips ahead to the next plausible boundary, so one mistake does
//! not cascade into a wall of messages.

mod expr;
mod items;
mod stmt;
#[cfg(test)]
mod tests;
mod types;

use crate::ast::*;
use crate::diag::{Diagnostic, Diagnostics};
use crate::lex::{Keyword, Lexer, Punct, Token, TokenKind};
use crate::span::{FileId, SourceMap, Span};

/// The recursive-descent parser.
pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    sources: &'a SourceMap,
    file: FileId,
    diagnostics: &'a mut Diagnostics,
    /// Depth of speculative parses; while above zero diagnostics are
    /// suppressed and any reported problem marks the guess as failed.
    speculation: usize,
    /// Set when a diagnostic is produced inside a speculative parse.
    speculation_failed: bool,
}

impl<'a> Parser<'a> {
    /// Lexes a whole file and prepares it for parsing.
    ///
    /// Error tokens are dropped: their diagnostics are already recorded and
    /// the parser only sees well-formed tokens.
    pub fn new(sources: &'a SourceMap, file: FileId, diagnostics: &'a mut Diagnostics) -> Self {
        let tokens = Lexer::new(sources, file, diagnostics)
            .lex()
            .into_iter()
            .filter(|token| token.kind != TokenKind::Error)
            .collect();
        Self {
            tokens,
            pos: 0,
            sources,
            file,
            diagnostics,
            speculation: 0,
            speculation_failed: false,
        }
    }

    /// Parses the whole token stream into a program.
    pub fn parse_program(mut self) -> Program {
        let module = self.parse_module_decl();
        let mut imports = Vec::new();
        let mut items = Vec::new();
        while !self.at_eof() {
            if self.at_keyword(Keyword::Import) {
                if let Some(import) = self.parse_import() {
                    imports.push(import);
                }
                continue;
            }
            if self.at_union_decl() {
                self.parse_union_decl();
                continue;
            }
            match self.parse_item() {
                Some(item) => items.push(item),
                None => {
                    self.bump();
                }
            }
        }
        Program {
            module,
            imports,
            items,
        }
    }

    /// Token helpers

    /// The token at the cursor; the last token is always `Eof`.
    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    /// Kind of the token at the cursor.
    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    /// Kind of the token `offset` positions ahead, when it exists.
    fn kind_at(&self, offset: usize) -> &TokenKind {
        let index = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[index].kind
    }

    /// True when the cursor is at the end of the file.
    fn at_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    /// True when the cursor is at the given punctuation.
    fn at_punct(&self, punct: Punct) -> bool {
        matches!(self.peek_kind(), TokenKind::Punct(found) if *found == punct)
    }

    /// True when the cursor is at the given keyword.
    fn at_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek_kind(), TokenKind::Keyword(found) if *found == keyword)
    }

    /// True when the cursor is at an identifier with the given spelling.
    fn at_ident(&self, spelling: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(name) if name == spelling)
    }

    /// Consumes and returns the token at the cursor.
    fn bump(&mut self) -> Token {
        let token = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if !self.at_eof() {
            self.pos += 1;
        }
        token
    }

    /// Consumes the given punctuation when present.
    fn eat_punct(&mut self, punct: Punct) -> bool {
        if self.at_punct(punct) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consumes the given keyword when present.
    fn eat_keyword(&mut self, keyword: Keyword) -> bool {
        if self.at_keyword(keyword) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consumes the given punctuation or reports `E20000`.
    fn expect_punct(&mut self, punct: Punct) -> bool {
        if self.eat_punct(punct) {
            true
        } else {
            self.unexpected();
            false
        }
    }

    /// Consumes the given keyword or reports `E20000`.
    fn expect_keyword(&mut self, keyword: Keyword) -> bool {
        if self.eat_keyword(keyword) {
            true
        } else {
            self.unexpected();
            false
        }
    }

    /// Consumes an identifier, or reports `E20000`.
    fn expect_ident(&mut self) -> Option<(String, Span)> {
        match self.peek_kind().clone() {
            TokenKind::Ident(name) => {
                let span = self.bump().span;
                Some((name, span))
            }
            _ => {
                self.unexpected();
                None
            }
        }
    }

    /// Diagnostics

    /// Records a diagnostic unless a speculative parse is running.
    fn report(&mut self, diagnostic: Diagnostic) {
        if self.speculation > 0 {
            self.speculation_failed = true;
        } else {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Reports `E20000` for the token at the cursor.
    fn unexpected(&mut self) {
        let token = self.peek();
        let text = self.token_text(token);
        let span = token.span;
        self.report(Diagnostic::error(
            "E20000",
            format!("syntax error: unexpected {text}"),
            span,
        ));
    }

    /// Source spelling of a token, used in diagnostics.
    fn token_text(&self, token: &Token) -> String {
        match token.kind {
            TokenKind::Eof => "end of file".to_owned(),
            _ => self.sources.snippet(token.span).to_owned(),
        }
    }

    /// Span helpers

    /// Byte offset where the token at the cursor starts.
    fn current_start(&self) -> usize {
        self.peek().span.start as usize
    }

    /// Byte offset just past the last consumed token.
    fn previous_end(&self) -> usize {
        if self.pos == 0 {
            0
        } else {
            self.tokens[self.pos - 1].span.end as usize
        }
    }

    /// Span from `start` to the end of the last consumed token.
    fn span_from(&self, start: usize) -> Span {
        Span::new(self.file, start, self.previous_end())
    }

    /// Speculation

    /// Runs `parse` speculatively.
    ///
    /// Any diagnostic produced inside marks the attempt as failed; the cursor
    /// is restored and `None` is returned. On success the cursor stays where
    /// the attempt left it.
    fn speculate<T>(&mut self, parse: impl FnOnce(&mut Self) -> T) -> Option<T> {
        let checkpoint = self.pos;
        self.speculation += 1;
        let previously_failed = self.speculation_failed;
        self.speculation_failed = false;
        let value = parse(self);
        let failed = self.speculation_failed;
        self.speculation -= 1;
        self.speculation_failed = previously_failed;
        if failed {
            self.pos = checkpoint;
            None
        } else {
            Some(value)
        }
    }

    /// Declared names

    /// Parses a declared name.
    ///
    /// A keyword name reports `E20005` and a reserved built-in name reports
    /// `E20006`, except inside an `intrinsic func` declaration where binding a
    /// reserved name is the documented purpose.
    fn parse_declared_name(&mut self, allow_reserved: bool) -> (String, Span) {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Ident(name) => {
                if !allow_reserved && crate::token::is_reserved_builtin(&name) {
                    self.report(Diagnostic::error(
                        "E20006",
                        format!("'{name}' is a reserved built-in and cannot be redeclared"),
                        token.span,
                    ));
                }
                self.pos += 1;
                (name, token.span)
            }
            TokenKind::Keyword(keyword) => {
                let spelling = keyword_spelling(keyword).to_owned();
                self.report(Diagnostic::error(
                    "E20005",
                    format!("'{spelling}' is a keyword and cannot be used as a name"),
                    token.span,
                ));
                self.pos += 1;
                (spelling, token.span)
            }
            _ => {
                let text = self.token_text(&token);
                self.unexpected();
                self.bump();
                (text, token.span)
            }
        }
    }

    /// Parses a dotted name.
    fn parse_qualified_name(&mut self) -> QualifiedName {
        let start = self.current_start();
        let Some((first, _)) = self.expect_ident() else {
            return QualifiedName {
                segments: Vec::new(),
                span: self.span_from(start),
            };
        };
        let mut segments = vec![first];
        while self.at_punct(Punct::Dot) {
            self.bump();
            match self.expect_ident() {
                Some((segment, _)) => segments.push(segment),
                None => break,
            }
        }
        QualifiedName {
            segments,
            span: self.span_from(start),
        }
    }

    /// Parses an optional `const`/`borrow`/`take` mode.
    fn parse_mode(&mut self) -> Mode {
        if self.eat_keyword(Keyword::Const) {
            Mode::Const
        } else if self.eat_keyword(Keyword::Borrow) {
            Mode::Borrow
        } else if self.eat_keyword(Keyword::Take) {
            Mode::Take
        } else {
            Mode::None
        }
    }

    /// Recovery

    /// Skips a brace-balanced block; the cursor must be at `{`.
    fn skip_braced_block(&mut self) {
        if !self.eat_punct(Punct::LBrace) {
            return;
        }
        let mut depth = 1usize;
        while depth > 0 && !self.at_eof() {
            if self.eat_punct(Punct::LBrace) {
                depth += 1;
            } else if self.eat_punct(Punct::RBrace) {
                depth -= 1;
            } else {
                self.bump();
            }
        }
    }

    /// Skips tokens up to and including the next semicolon.
    fn skip_to_semicolon(&mut self) {
        while !self.at_eof() && !self.eat_punct(Punct::Semi) {
            self.bump();
        }
    }
}

/// Scalar type behind a scalar keyword, if the keyword names one.
fn scalar_type_of(keyword: Keyword) -> Option<ScalarType> {
    let scalar = match keyword {
        Keyword::I8 => ScalarType::I8,
        Keyword::I16 => ScalarType::I16,
        Keyword::I32 => ScalarType::I32,
        Keyword::U8 => ScalarType::U8,
        Keyword::U16 => ScalarType::U16,
        Keyword::U32 => ScalarType::U32,
        Keyword::F32 => ScalarType::F32,
        Keyword::F64 => ScalarType::F64,
        Keyword::Bool => ScalarType::Bool,
        Keyword::Usize => ScalarType::Usize,
        Keyword::Isize => ScalarType::Isize,
        _ => return None,
    };
    Some(scalar)
}

/// Spelling of a keyword, used in diagnostics.
fn keyword_spelling(keyword: Keyword) -> &'static str {
    match keyword {
        Keyword::Module => "module",
        Keyword::Import => "import",
        Keyword::Export => "export",
        Keyword::From => "from",
        Keyword::Func => "func",
        Keyword::Comptime => "comptime",
        Keyword::Return => "return",
        Keyword::Struct => "struct",
        Keyword::Enum => "enum",
        Keyword::Handle => "handle",
        Keyword::FreedBy => "freedBy",
        Keyword::Ffi => "ffi",
        Keyword::Intrinsic => "intrinsic",
        Keyword::Unsafe => "unsafe",
        Keyword::Volatile => "volatile",
        Keyword::Raw => "raw",
        Keyword::Let => "let",
        Keyword::Const => "const",
        Keyword::Borrow => "borrow",
        Keyword::Take => "take",
        Keyword::Defer => "defer",
        Keyword::If => "if",
        Keyword::Else => "else",
        Keyword::While => "while",
        Keyword::Do => "do",
        Keyword::For => "for",
        Keyword::In => "in",
        Keyword::Switch => "switch",
        Keyword::Case => "case",
        Keyword::Default => "default",
        Keyword::Break => "break",
        Keyword::Continue => "continue",
        Keyword::I8 => "i8",
        Keyword::I16 => "i16",
        Keyword::I32 => "i32",
        Keyword::U8 => "u8",
        Keyword::U16 => "u16",
        Keyword::U32 => "u32",
        Keyword::F32 => "f32",
        Keyword::F64 => "f64",
        Keyword::Bool => "bool",
        Keyword::Void => "void",
        Keyword::Usize => "usize",
        Keyword::Isize => "isize",
        Keyword::Str => "str",
        Keyword::Cstring => "cstring",
        Keyword::Static => "static",
        Keyword::True => "true",
        Keyword::False => "false",
        Keyword::Null => "null",
    }
}

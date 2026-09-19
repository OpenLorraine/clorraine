//! Lexer: source bytes to tokens.
//!
//! The scanner implements the lexical chapter: identifiers and the 50
//! keywords, numeric/character/string literals, template strings, comments,
//! and the punctuation set. It never fails hard: a malformed construct records
//! a diagnostic and yields an [`TokenKind::Error`] token so the parser can
//! keep going. Labels that come from other languages (`->`, `::`, `#`, `@`,
//! `?.`, `??`) are diagnosed as foreign syntax.

use crate::diag::{Diagnostic, Diagnostics};
use crate::span::{FileId, SourceMap, Span};

pub use crate::token::{Keyword, Punct, TemplatePart, Token, TokenKind};

mod literals;

/// Byte-to-token scanner for one file.
pub struct Lexer<'a> {
    file: FileId,
    text: &'a str,
    pos: usize,
    limit: usize,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> Lexer<'a> {
    /// Creates a scanner for a whole file.
    pub fn new(sources: &'a SourceMap, file: FileId, diagnostics: &'a mut Diagnostics) -> Self {
        let text = sources.text(file);
        Self {
            file,
            text,
            pos: 0,
            limit: text.len(),
            diagnostics,
        }
    }

    /// Creates a scanner for one byte range of a file, used to re-lex the
    /// text of a template placeholder. Spans stay absolute in the file.
    pub fn range(
        sources: &'a SourceMap,
        file: FileId,
        span: Span,
        diagnostics: &'a mut Diagnostics,
    ) -> Self {
        let text = sources.text(file);
        Self {
            file,
            text,
            pos: span.start as usize,
            limit: span.end as usize,
            diagnostics,
        }
    }

    /// Tokenizes the whole range and ends with an [`TokenKind::Eof`] token.
    pub fn lex(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia();
            let start = self.pos;
            if self.is_eof() {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: self.span(start, start),
                });
                return tokens;
            }
            let kind = self.scan_token(start);
            tokens.push(Token {
                kind,
                span: self.span(start, self.pos),
            });
        }
    }

    /// Position helpers

    /// True when the scanner reached the end of its range.
    fn is_eof(&self) -> bool {
        self.pos >= self.limit
    }

    /// Byte at the current position, or `None` at the end of the range.
    fn peek(&self) -> Option<u8> {
        if self.pos < self.limit {
            self.text.as_bytes().get(self.pos).copied()
        } else {
            None
        }
    }

    /// Byte `offset` positions ahead of the cursor, within the range.
    fn peek_at(&self, offset: usize) -> Option<u8> {
        let index = self.pos + offset;
        if index < self.limit {
            self.text.as_bytes().get(index).copied()
        } else {
            None
        }
    }

    /// Consumes one UTF-8 character and returns it.
    fn next_char(&mut self) -> char {
        let ch = self.text[self.pos..]
            .chars()
            .next()
            .expect("character at the cursor");
        self.pos += ch.len_utf8();
        ch
    }

    /// Builds a span for the current file.
    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.file, start, end)
    }

    /// Records one diagnostic.
    fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Trivia

    /// Skips whitespace and comments.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(byte) if byte.is_ascii_whitespace() => self.pos += 1,
                Some(b'/') if self.peek_at(1) == Some(b'/') => self.skip_line_comment(),
                Some(b'/') if self.peek_at(1) == Some(b'*') => self.skip_block_comment(),
                _ => return,
            }
        }
    }

    /// Skips a `//` comment; documentation comments are trivia for now.
    fn skip_line_comment(&mut self) {
        while let Some(byte) = self.peek() {
            if byte == b'\n' {
                return;
            }
            self.pos += 1;
        }
    }

    /// Skips a block comment and reports `E20001` when it never closes.
    fn skip_block_comment(&mut self) {
        let start = self.pos;
        self.pos += 2;
        loop {
            match self.peek() {
                None => {
                    self.report(Diagnostic::error(
                        "E20001",
                        "unterminated block comment",
                        self.span(start, self.pos),
                    ));
                    return;
                }
                Some(b'*') if self.peek_at(1) == Some(b'/') => {
                    self.pos += 2;
                    return;
                }
                Some(_) => self.pos += 1,
            }
        }
    }

    /// Token scanning

    /// Scans the token that starts at `start`.
    fn scan_token(&mut self, start: usize) -> TokenKind {
        match self.peek().expect("token start") {
            b'"' => self.scan_string(),
            b'\'' => self.scan_char(),
            b'$' if self.peek_at(1) == Some(b'"') => self.scan_template(),
            b'0'..=b'9' => self.scan_number(false),
            b'.' if self.peek_at(1).is_some_and(|byte| byte.is_ascii_digit()) => {
                self.scan_number(true)
            }
            byte if is_ident_start(byte) => self.scan_ident(),
            _ => self.scan_punct(start),
        }
    }

    /// Scans an identifier or keyword.
    fn scan_ident(&mut self) -> TokenKind {
        let start = self.pos;
        while self.peek().is_some_and(is_ident_continue) {
            self.pos += 1;
        }
        let spelling = &self.text[start..self.pos];
        match Keyword::from_spelling(spelling) {
            Some(keyword) => TokenKind::Keyword(keyword),
            None => TokenKind::Ident(spelling.to_owned()),
        }
    }

    /// Scans a punctuation or operator token.
    ///
    /// The foreign spellings `->`, `::`, `?.`, and `??` are reported here with
    /// `E20008` instead of producing a token.
    fn scan_punct(&mut self, start: usize) -> TokenKind {
        let byte = self.peek().expect("punctuation byte");
        self.pos += 1;
        let kind = match byte {
            b'+' => match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    Punct::PlusPlus
                }
                Some(b'=') => {
                    self.pos += 1;
                    Punct::PlusEq
                }
                _ => Punct::Plus,
            },
            b'-' => match self.peek() {
                Some(b'>') => {
                    self.pos += 1;
                    return self.foreign(start);
                }
                Some(b'-') => {
                    self.pos += 1;
                    Punct::MinusMinus
                }
                Some(b'=') => {
                    self.pos += 1;
                    Punct::MinusEq
                }
                _ => Punct::Minus,
            },
            b'*' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    Punct::StarEq
                }
                _ => Punct::Star,
            },
            b'/' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    Punct::SlashEq
                }
                _ => Punct::Slash,
            },
            b'%' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    Punct::PercentEq
                }
                _ => Punct::Percent,
            },
            b'^' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    Punct::CaretEq
                }
                _ => Punct::Caret,
            },
            b'&' => match self.peek() {
                Some(b'&') => {
                    self.pos += 1;
                    Punct::AmpAmp
                }
                Some(b'=') => {
                    self.pos += 1;
                    Punct::AmpEq
                }
                _ => Punct::Amp,
            },
            b'|' => match self.peek() {
                Some(b'|') => {
                    self.pos += 1;
                    Punct::PipePipe
                }
                Some(b'=') => {
                    self.pos += 1;
                    Punct::PipeEq
                }
                _ => Punct::Pipe,
            },
            b'~' => Punct::Tilde,
            b'!' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    Punct::BangEq
                }
                _ => Punct::Bang,
            },
            b'<' => match self.peek() {
                Some(b'<') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'=') => {
                            self.pos += 1;
                            Punct::ShlEq
                        }
                        _ => Punct::Shl,
                    }
                }
                Some(b'=') => {
                    self.pos += 1;
                    Punct::LtEq
                }
                _ => Punct::Lt,
            },
            b'>' => match self.peek() {
                Some(b'>') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'=') => {
                            self.pos += 1;
                            Punct::ShrEq
                        }
                        _ => Punct::Shr,
                    }
                }
                Some(b'=') => {
                    self.pos += 1;
                    Punct::GtEq
                }
                _ => Punct::Gt,
            },
            b'=' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    Punct::EqEq
                }
                Some(b'>') => {
                    self.pos += 1;
                    Punct::FatArrow
                }
                _ => Punct::Eq,
            },
            b'?' => match self.peek() {
                Some(b'.' | b'?') => {
                    self.pos += 1;
                    return self.foreign(start);
                }
                _ => Punct::Question,
            },
            b':' => match self.peek() {
                Some(b':') => {
                    self.pos += 1;
                    return self.foreign(start);
                }
                _ => Punct::Colon,
            },
            b'#' | b'@' => return self.foreign(start),
            b'.' => Punct::Dot,
            b',' => Punct::Comma,
            b';' => Punct::Semi,
            b'(' => Punct::LParen,
            b')' => Punct::RParen,
            b'[' => Punct::LBracket,
            b']' => Punct::RBracket,
            b'{' => Punct::LBrace,
            b'}' => Punct::RBrace,
            _ => {
                self.pos -= 1;
                let ch = self.next_char();
                self.report(Diagnostic::error(
                    "E20000",
                    format!("syntax error: unexpected {ch}"),
                    self.span(start, self.pos),
                ));
                return TokenKind::Error;
            }
        };
        TokenKind::Punct(kind)
    }

    /// Reports one foreign-syntax spelling that spans `start..self.pos`.
    fn foreign(&mut self, start: usize) -> TokenKind {
        let text = self.text[start..self.pos].to_owned();
        self.report(Diagnostic::error(
            "E20008",
            format!("'{text}' is not Clorraine syntax"),
            self.span(start, self.pos),
        ));
        TokenKind::Error
    }

    /// Literals

    /// Scans a numeric literal.
    ///
    /// `leading_dot` marks a literal that starts with `.` (for example `.5`),
    /// which is always malformed because a float needs digits on both sides of
    /// the dot. A suffix (`1u8`) or a trailing dot without digits (`1.`) is
    /// malformed too.
    fn scan_number(&mut self, leading_dot: bool) -> TokenKind {
        let start = self.pos;
        if leading_dot {
            self.pos += 1;
            self.take_digits(10);
            self.take_exponent();
            self.take_suffix();
            return self.malformed_number(start);
        }
        if let Some(radix) = self.take_radix_prefix() {
            let word = self.take_alnum_word();
            if valid_digit_word(word, radix) {
                return match u64::from_str_radix(&word.replace('_', ""), radix) {
                    Ok(value) => TokenKind::Int(value),
                    Err(_) => self.malformed_number(start),
                };
            }
            return self.malformed_number(start);
        }
        let integer = self.take_digits(10);
        if !valid_digit_word(integer, 10) {
            self.take_suffix();
            return self.malformed_number(start);
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            if self.peek_at(1).is_some_and(|byte| byte.is_ascii_digit()) {
                self.pos += 1;
                let fraction = self.take_digits(10);
                if !valid_digit_word(fraction, 10) {
                    self.take_suffix();
                    return self.malformed_number(start);
                }
                if !self.take_exponent() {
                    return self.malformed_number(start);
                }
                is_float = true;
            } else if self.peek_at(1).is_some_and(is_ident_start) {
                // `1.len()` is member access on an integer, not a float.
            } else {
                self.pos += 1;
                return self.malformed_number(start);
            }
        } else if matches!(self.peek(), Some(b'e' | b'E')) {
            if !self.take_exponent() {
                return self.malformed_number(start);
            }
            is_float = true;
        }
        if self.take_suffix() {
            return self.malformed_number(start);
        }
        let text = self.text[start..self.pos].replace('_', "");
        if is_float {
            match text.parse::<f64>() {
                Ok(value) => TokenKind::Float(value),
                Err(_) => self.malformed_number(start),
            }
        } else {
            match text.parse::<u64>() {
                Ok(value) => TokenKind::Int(value),
                Err(_) => self.malformed_number(start),
            }
        }
    }

    /// Consumes a `0x`/`0b` prefix and returns its radix.
    fn take_radix_prefix(&mut self) -> Option<u32> {
        match (self.peek(), self.peek_at(1)) {
            (Some(b'0'), Some(b'x' | b'X')) => {
                self.pos += 2;
                Some(16)
            }
            (Some(b'0'), Some(b'b' | b'B')) => {
                self.pos += 2;
                Some(2)
            }
            _ => None,
        }
    }

    /// Consumes digits of a radix plus `_` separators.
    fn take_digits(&mut self, radix: u32) -> &'a str {
        let start = self.pos;
        while let Some(byte) = self.peek() {
            if is_radix_digit(byte, radix) || byte == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        &self.text[start..self.pos]
    }

    /// Consumes letters, digits, and `_`.
    fn take_alnum_word(&mut self) -> &'a str {
        let start = self.pos;
        while self.peek().is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
            self.pos += 1;
        }
        &self.text[start..self.pos]
    }

    /// Consumes an optional exponent; returns false when `e`/`E` appears
    /// without digits behind it. The consumed marker stays consumed so the
    /// malformed literal keeps its full spelling.
    fn take_exponent(&mut self) -> bool {
        if !matches!(self.peek(), Some(b'e' | b'E')) {
            return true;
        }
        self.pos += 1;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.pos += 1;
        }
        let digits = self.take_digits(10);
        valid_digit_word(digits, 10)
    }

    /// Consumes an identifier-like suffix and reports whether one was present.
    fn take_suffix(&mut self) -> bool {
        if self.peek().is_some_and(is_ident_start) {
            self.take_alnum_word();
            true
        } else {
            false
        }
    }

    /// Reports a malformed numeric literal that spans `start..self.pos`.
    fn malformed_number(&mut self, start: usize) -> TokenKind {
        let text = self.text[start..self.pos].to_owned();
        self.report(Diagnostic::error(
            "E20004",
            format!("invalid numeric literal '{text}'"),
            self.span(start, self.pos),
        ));
        TokenKind::Error
    }

}

/// True for the first byte of an identifier.
fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

/// True for a byte that may continue an identifier.
fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// True for a digit of the given radix.
fn is_radix_digit(byte: u8, radix: u32) -> bool {
    match radix {
        2 => matches!(byte, b'0' | b'1'),
        10 => byte.is_ascii_digit(),
        16 => byte.is_ascii_hexdigit(),
        _ => false,
    }
}

/// True when `word` is a valid digit sequence: non-empty groups of radix
/// digits separated by single `_` characters.
fn valid_digit_word(word: &str, radix: u32) -> bool {
    !word.is_empty()
        && word.split('_').all(|group| {
            !group.is_empty() && group.bytes().all(|byte| is_radix_digit(byte, radix))
        })
}

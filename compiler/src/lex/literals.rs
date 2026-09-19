//! Literal scanning: strings, characters, templates, and escape sequences.
//!
//! The scanners keep malformed input recoverable; every diagnostic is recorded
//! once and the caller receives a token that lets the parser continue.

use super::*;

/// Outcome of scanning one escape sequence.
enum Escape {
    /// The escape decoded to one byte.
    Byte(u8),
    /// The escape was invalid; the diagnostic was recorded.
    Invalid,
    /// The escape ran into the end of the line or file.
    Unfinished,
}

/// Where a template body ended.
enum TemplateEnd {
    /// The closing quote was consumed.
    Closed,
    /// A quote arrived while a placeholder was still open.
    Unbalanced,
    /// A line ending or the file ended inside the template.
    Unterminated,
}

/// Where a placeholder scan stopped.
enum PlaceholderEnd {
    /// Offset of the closing brace.
    Closed(usize),
    /// A quote ended the template while the placeholder was open.
    StoppedAtQuote,
    /// The file ended inside the placeholder.
    StoppedAtEof,
}

impl<'a> Lexer<'a> {
    /// Scans a character literal; it must decode to exactly one byte.
    pub(super) fn scan_char(&mut self) -> TokenKind {
        let start = self.pos;
        self.pos += 1;
        let mut bytes: Vec<u8> = Vec::new();
        let mut invalid_escape = false;
        let mut terminated = false;
        loop {
            match self.peek() {
                None | Some(b'\n') | Some(b'\r') => break,
                Some(b'\'') => {
                    self.pos += 1;
                    terminated = true;
                    break;
                }
                Some(b'\\') => match self.scan_escape() {
                    Escape::Byte(byte) => bytes.push(byte),
                    Escape::Invalid => invalid_escape = true,
                    Escape::Unfinished => break,
                },
                Some(_) => {
                    let ch = self.next_char();
                    let mut buffer = [0u8; 4];
                    bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
                }
            }
        }
        if !terminated {
            self.report(Diagnostic::error(
                "E20002",
                "unterminated character literal",
                self.span(start, self.pos),
            ));
            return TokenKind::Error;
        }
        if invalid_escape {
            return TokenKind::Error;
        }
        if bytes.len() == 1 {
            TokenKind::Char(bytes[0])
        } else {
            self.report(Diagnostic::error(
                "E20007",
                "character literal must encode exactly one byte",
                self.span(start, self.pos),
            ));
            TokenKind::Error
        }
    }

    /// Scans a string literal and returns its decoded bytes.
    pub(super) fn scan_string(&mut self) -> TokenKind {
        let start = self.pos;
        self.pos += 1;
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            match self.peek() {
                None | Some(b'\n') | Some(b'\r') => {
                    self.report(Diagnostic::error(
                        "E20002",
                        "unterminated string literal",
                        self.span(start, self.pos),
                    ));
                    return TokenKind::Error;
                }
                Some(b'"') => {
                    self.pos += 1;
                    return TokenKind::Str(bytes);
                }
                Some(b'\\') => match self.scan_escape() {
                    Escape::Byte(byte) => bytes.push(byte),
                    Escape::Invalid => {}
                    Escape::Unfinished => {
                        self.report(Diagnostic::error(
                            "E20002",
                            "unterminated string literal",
                            self.span(start, self.pos),
                        ));
                        return TokenKind::Error;
                    }
                },
                Some(_) => {
                    let ch = self.next_char();
                    let mut buffer = [0u8; 4];
                    bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
                }
            }
        }
    }

    /// Scans an escape sequence.
    fn scan_escape(&mut self) -> Escape {
        let start = self.pos;
        self.pos += 1;
        match self.peek() {
            None | Some(b'\n') | Some(b'\r') => Escape::Unfinished,
            Some(byte) => {
                let simple = match byte {
                    b'n' => Some(b'\n'),
                    b'r' => Some(b'\r'),
                    b't' => Some(b'\t'),
                    b'0' => Some(0),
                    b'\\' => Some(b'\\'),
                    b'"' => Some(b'"'),
                    b'\'' => Some(b'\''),
                    _ => None,
                };
                if let Some(value) = simple {
                    self.pos += 1;
                    return Escape::Byte(value);
                }
                if byte == b'x' {
                    let high = self.peek_at(1).and_then(hex_value);
                    let low = self.peek_at(2).and_then(hex_value);
                    if let (Some(high), Some(low)) = (high, low) {
                        self.pos += 3;
                        return Escape::Byte(high * 16 + low);
                    }
                    self.pos += 1;
                    self.invalid_escape(start);
                    return Escape::Invalid;
                }
                self.next_char();
                self.invalid_escape(start);
                Escape::Invalid
            }
        }
    }

    /// Reports an invalid escape sequence covering `start..self.pos`.
    fn invalid_escape(&mut self, start: usize) {
        let text = self.text[start..self.pos].to_owned();
        self.report(Diagnostic::error(
            "E20003",
            format!("invalid escape sequence '{text}'"),
            self.span(start, self.pos),
        ));
    }

    /// Scans a template string.
    pub(super) fn scan_template(&mut self) -> TokenKind {
        let start = self.pos;
        self.pos += 2;
        let (parts, end) = self.scan_template_body(true);
        match end {
            TemplateEnd::Closed => TokenKind::Template(parts),
            TemplateEnd::Unbalanced => TokenKind::Error,
            TemplateEnd::Unterminated => {
                self.report(Diagnostic::error(
                    "E20002",
                    "unterminated template literal",
                    self.span(start, self.pos),
                ));
                TokenKind::Error
            }
        }
    }

    /// Scans a template body after its `$"` opener.
    ///
    /// When `report` is cleared the scan only keeps a nested template
    /// balanced; its error was already reported by the outer scan.
    fn scan_template_body(&mut self, report: bool) -> (Vec<TemplatePart>, TemplateEnd) {
        let mut parts: Vec<TemplatePart> = Vec::new();
        let mut literal: Vec<u8> = Vec::new();
        loop {
            match self.peek() {
                None | Some(b'\n') | Some(b'\r') => {
                    return (template_parts(parts, literal), TemplateEnd::Unterminated);
                }
                Some(b'"') => {
                    self.pos += 1;
                    return (template_parts(parts, literal), TemplateEnd::Closed);
                }
                Some(b'\\') => match self.scan_escape() {
                    Escape::Byte(byte) => literal.push(byte),
                    Escape::Invalid => {}
                    Escape::Unfinished => {
                        return (template_parts(parts, literal), TemplateEnd::Unterminated);
                    }
                },
                Some(b'{') if self.peek_at(1) == Some(b'{') => {
                    literal.push(b'{');
                    self.pos += 2;
                }
                Some(b'}') if self.peek_at(1) == Some(b'}') => {
                    literal.push(b'}');
                    self.pos += 2;
                }
                Some(b'}') => {
                    if report {
                        self.report(Diagnostic::error(
                            "E10503",
                            "unbalanced braces in template",
                            self.span(self.pos, self.pos + 1),
                        ));
                    }
                    self.pos += 1;
                }
                Some(b'{') => {
                    if !literal.is_empty() {
                        parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
                    }
                    let inner_start = self.pos + 1;
                    self.pos += 1;
                    match self.scan_placeholder_body(report) {
                        PlaceholderEnd::Closed(close) => {
                            parts.push(TemplatePart::Placeholder(self.span(inner_start, close)));
                            self.pos = close + 1;
                        }
                        PlaceholderEnd::StoppedAtQuote => {
                            parts.push(TemplatePart::Placeholder(self.span(inner_start, self.pos)));
                            if report {
                                self.report(Diagnostic::error(
                                    "E10503",
                                    "unbalanced braces in template",
                                    self.span(self.pos, self.pos + 1),
                                ));
                                self.pos += 1;
                            }
                            return (template_parts(parts, literal), TemplateEnd::Unbalanced);
                        }
                        PlaceholderEnd::StoppedAtEof => {
                            parts.push(TemplatePart::Placeholder(self.span(inner_start, self.pos)));
                            return (template_parts(parts, literal), TemplateEnd::Unterminated);
                        }
                    }
                }
                Some(_) => {
                    let ch = self.next_char();
                    let mut buffer = [0u8; 4];
                    literal.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
                }
            }
        }
    }

    /// Scans a placeholder body after its opening `{`.
    ///
    /// Nested braces, strings, character literals, comments, and nested
    /// templates are tracked so their contents cannot end the placeholder
    /// early. A nested template is diagnosed once and skipped.
    fn scan_placeholder_body(&mut self, report: bool) -> PlaceholderEnd {
        let mut depth = 1usize;
        loop {
            match self.peek() {
                None => return PlaceholderEnd::StoppedAtEof,
                Some(b'{') => {
                    depth += 1;
                    self.pos += 1;
                }
                Some(b'}') => {
                    depth -= 1;
                    if depth == 0 {
                        return PlaceholderEnd::Closed(self.pos);
                    }
                    self.pos += 1;
                }
                Some(b'"') => {
                    if !self.probe_string() {
                        return PlaceholderEnd::StoppedAtQuote;
                    }
                }
                Some(b'\'') => {
                    if !self.probe_char() {
                        self.pos += 1;
                    }
                }
                Some(b'/') if self.peek_at(1) == Some(b'/') => self.skip_line_comment(),
                Some(b'/') if self.peek_at(1) == Some(b'*') => {
                    if !self.probe_block_comment() {
                        return PlaceholderEnd::StoppedAtEof;
                    }
                }
                Some(b'$') if self.peek_at(1) == Some(b'"') => {
                    let nested_start = self.pos;
                    if report {
                        self.report(Diagnostic::error(
                            "E90005",
                            "nested templates are not yet supported",
                            self.span(nested_start, nested_start + 2),
                        ));
                    }
                    self.pos += 2;
                    match self.scan_template_body(false).1 {
                        TemplateEnd::Closed => {}
                        TemplateEnd::Unbalanced => return PlaceholderEnd::StoppedAtQuote,
                        TemplateEnd::Unterminated => return PlaceholderEnd::StoppedAtEof,
                    }
                }
                Some(_) => {
                    self.next_char();
                }
            }
        }
    }

    /// Skips a string inside a placeholder without diagnosing it.
    ///
    /// Returns false (and restores the cursor) when the quote cannot start a
    /// string that closes on the same line.
    fn probe_string(&mut self) -> bool {
        let mark = self.pos;
        self.pos += 1;
        loop {
            match self.peek() {
                None | Some(b'\n') | Some(b'\r') => {
                    self.pos = mark;
                    return false;
                }
                Some(b'"') => {
                    self.pos += 1;
                    return true;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    if matches!(self.peek(), None | Some(b'\n') | Some(b'\r')) {
                        self.pos = mark;
                        return false;
                    }
                    self.next_char();
                }
                Some(_) => {
                    self.next_char();
                }
            }
        }
    }

    /// Skips a character literal inside a placeholder without diagnosing it.
    fn probe_char(&mut self) -> bool {
        let mark = self.pos;
        self.pos += 1;
        loop {
            match self.peek() {
                None | Some(b'\n') | Some(b'\r') => {
                    self.pos = mark;
                    return false;
                }
                Some(b'\'') => {
                    self.pos += 1;
                    return true;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    if matches!(self.peek(), None | Some(b'\n') | Some(b'\r')) {
                        self.pos = mark;
                        return false;
                    }
                    self.next_char();
                }
                Some(_) => {
                    self.next_char();
                }
            }
        }
    }

    /// Skips a block comment inside a placeholder without diagnosing it.
    fn probe_block_comment(&mut self) -> bool {
        let mark = self.pos;
        self.pos += 2;
        loop {
            match self.peek() {
                None => {
                    self.pos = mark;
                    return false;
                }
                Some(b'*') if self.peek_at(1) == Some(b'/') => {
                    self.pos += 2;
                    return true;
                }
                Some(_) => self.pos += 1,
            }
        }
    }
}

/// Pushes the trailing literal of a template body, when it is not empty.
fn template_parts(mut parts: Vec<TemplatePart>, literal: Vec<u8>) -> Vec<TemplatePart> {
    if !literal.is_empty() {
        parts.push(TemplatePart::Literal(literal));
    }
    parts
}

/// Numeric value of one hexadecimal digit.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

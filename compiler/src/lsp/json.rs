//! A small JSON value model for the language server.
//!
//! The workspace is std-only, so the protocol needs its own JSON parser and
//! writer. The parser accepts the JSON subset the protocol uses: objects,
//! arrays, strings, integers, booleans, and `null`. Numbers are integers
//! because every number the server sends or receives is a request id or a
//! position; a fractional number is rejected rather than silently rounded.
//!
//! The writer is deterministic: object keys are emitted in `BTreeMap` order.
//! Protocol code that needs a specific field order builds its objects from a
//! string when the order is part of the contract.

use std::collections::BTreeMap;

/// One JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Number(i64),
    /// A string with escapes already decoded.
    String(String),
    /// An array in source order.
    Array(Vec<Json>),
    /// An object whose keys are unique.
    Object(BTreeMap<String, Json>),
}

/// A parse failure with the byte offset where it was detected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    /// Human-readable reason.
    pub message: String,
    /// Byte offset in the input.
    pub offset: usize,
}

impl Json {
    /// Parses one JSON value.
    ///
    /// Leading and trailing whitespace is allowed; any non-whitespace after
    /// the value is an error.
    pub fn parse(text: &str) -> Result<Self, JsonError> {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            index: 0,
            depth: 0,
        };
        parser.skip_whitespace();
        let value = parser.value()?;
        parser.skip_whitespace();
        if parser.index != parser.bytes.len() {
            return Err(parser.error("trailing characters after the JSON value"));
        }
        Ok(value)
    }

    /// Renders the value as compact JSON.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    /// Looks up an object member.
    pub fn get(&self, key: &str) -> Option<&Json> {
        self.as_object()?.get(key)
    }

    /// Returns the string value, if this is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the integer value, if this is a number.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Number(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the array slice, if this is an array.
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(values) => Some(values),
            _ => None,
        }
    }

    /// Returns the object map, if this is an object.
    pub fn as_object(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Json::Object(values) => Some(values),
            _ => None,
        }
    }

    /// Builds an object from key/value pairs.
    pub fn object<const N: usize>(pairs: [(&str, Json); N]) -> Json {
        let mut values = BTreeMap::new();
        for (key, value) in pairs {
            values.insert(key.to_owned(), value);
        }
        Json::Object(values)
    }

    /// Builds an object from a dynamic list of key/value pairs.
    pub fn from_pairs(pairs: Vec<(&str, Json)>) -> Json {
        let mut values = BTreeMap::new();
        for (key, value) in pairs {
            values.insert(key.to_owned(), value);
        }
        Json::Object(values)
    }

    /// Writes the value into `out`.
    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Number(value) => out.push_str(&value.to_string()),
            Json::String(value) => write_string(value, out),
            Json::Array(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    value.write(out);
                }
                out.push(']');
            }
            Json::Object(values) => {
                out.push('{');
                for (index, (key, value)) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Writes a JSON string with the required escapes.
fn write_string(value: &str, out: &mut String) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
}

/// Recursive-descent parser state.
struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    /// Builds an error at the current offset.
    fn error(&self, message: &str) -> JsonError {
        JsonError {
            message: message.to_owned(),
            offset: self.index,
        }
    }

    /// Skips JSON whitespace.
    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.index += 1;
        }
    }

    /// Parses one value.
    fn value(&mut self) -> Result<Json, JsonError> {
        if self.depth > 128 {
            return Err(self.error("JSON nesting is too deep"));
        }
        let Some(byte) = self.bytes.get(self.index).copied() else {
            return Err(self.error("unexpected end of JSON"));
        };
        match byte {
            b'n' => self.literal("null", Json::Null),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'"' => self.string().map(Json::String),
            b'[' => self.array(),
            b'{' => self.object(),
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(self.error("unexpected character")),
        }
    }

    /// Parses a fixed literal.
    fn literal(&mut self, text: &str, value: Json) -> Result<Json, JsonError> {
        if self.bytes[self.index..].starts_with(text.as_bytes()) {
            self.index += text.len();
            Ok(value)
        } else {
            Err(self.error("invalid literal"))
        }
    }

    /// Parses an integer.
    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.index;
        if self.bytes.get(self.index) == Some(&b'-') {
            self.index += 1;
        }
        let digits = self.index;
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            self.index += 1;
        }
        if digits == self.index {
            return Err(self.error("a number needs at least one digit"));
        }
        let text = &self.bytes[start..self.index];
        let text = std::str::from_utf8(text).map_err(|_| self.error("invalid number"))?;
        text.parse::<i64>()
            .map(Json::Number)
            .map_err(|_| self.error("number is not a 64-bit integer"))
    }

    /// Parses an array.
    fn array(&mut self) -> Result<Json, JsonError> {
        self.index += 1;
        self.depth += 1;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.bytes.get(self.index) == Some(&b']') {
            self.index += 1;
            self.depth -= 1;
            return Ok(Json::Array(values));
        }
        loop {
            self.skip_whitespace();
            values.push(self.value()?);
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    self.depth -= 1;
                    return Ok(Json::Array(values));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    /// Parses an object.
    fn object(&mut self) -> Result<Json, JsonError> {
        self.index += 1;
        self.depth += 1;
        let mut values = BTreeMap::new();
        self.skip_whitespace();
        if self.bytes.get(self.index) == Some(&b'}') {
            self.index += 1;
            self.depth -= 1;
            return Ok(Json::Object(values));
        }
        loop {
            self.skip_whitespace();
            if self.bytes.get(self.index) != Some(&b'"') {
                return Err(self.error("an object key must be a string"));
            }
            let key = self.string()?;
            self.skip_whitespace();
            if self.bytes.get(self.index) != Some(&b':') {
                return Err(self.error("expected ':' after an object key"));
            }
            self.index += 1;
            self.skip_whitespace();
            let value = self.value()?;
            values.insert(key, value);
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b'}') => {
                    self.index += 1;
                    self.depth -= 1;
                    return Ok(Json::Object(values));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    /// Parses a string literal with escapes.
    fn string(&mut self) -> Result<String, JsonError> {
        self.index += 1;
        let mut out = String::new();
        loop {
            let Some(byte) = self.bytes.get(self.index).copied() else {
                return Err(self.error("unterminated string"));
            };
            match byte {
                b'"' => {
                    self.index += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.index += 1;
                    self.escape(&mut out)?;
                }
                byte if byte < 0x20 => {
                    return Err(self.error("a raw control byte is not allowed in a string"));
                }
                _ => {
                    let start = self.index;
                    let character = std::str::from_utf8(&self.bytes[start..])
                        .map_err(|_| self.error("invalid UTF-8"))?
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("invalid UTF-8"))?;
                    out.push(character);
                    self.index += character.len_utf8();
                }
            }
        }
    }

    /// Parses one escape after the backslash.
    fn escape(&mut self, out: &mut String) -> Result<(), JsonError> {
        let Some(byte) = self.bytes.get(self.index).copied() else {
            return Err(self.error("unterminated escape"));
        };
        self.index += 1;
        match byte {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\u{08}'),
            b'f' => out.push('\u{0c}'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => {
                let first = self.hex4()?;
                if (0xD800..=0xDBFF).contains(&first) {
                    if self.bytes.get(self.index) != Some(&b'\\')
                        || self.bytes.get(self.index + 1) != Some(&b'u')
                    {
                        return Err(self.error("a high surrogate must be followed by a low surrogate"));
                    }
                    self.index += 2;
                    let second = self.hex4()?;
                    if !(0xDC00..=0xDFFF).contains(&second) {
                        return Err(self.error("invalid low surrogate"));
                    }
                    let value = 0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00);
                    let character = char::from_u32(value)
                        .ok_or_else(|| self.error("invalid surrogate pair"))?;
                    out.push(character);
                } else if (0xDC00..=0xDFFF).contains(&first) {
                    return Err(self.error("a lone low surrogate is not allowed"));
                } else {
                    let character = char::from_u32(first)
                        .ok_or_else(|| self.error("invalid escape"))?;
                    out.push(character);
                }
            }
            _ => return Err(self.error("unknown string escape")),
        }
        Ok(())
    }

    /// Reads four hex digits.
    fn hex4(&mut self) -> Result<u32, JsonError> {
        let Some(slice) = self.bytes.get(self.index..self.index + 4) else {
            return Err(self.error("incomplete unicode escape"));
        };
        let text = std::str::from_utf8(slice).map_err(|_| self.error("invalid unicode escape"))?;
        let value = u32::from_str_radix(text, 16)
            .map_err(|_| self.error("invalid unicode escape"))?;
        self.index += 4;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_trip_the_protocol_values() {
        let text = r#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{"rootUri":"file:///tmp/a b","n":-3,"ok":true,"items":[1,null,"x"]}}"#;
        let value = Json::parse(text).expect("valid JSON");
        assert_eq!(value.get("id").and_then(Json::as_i64), Some(7));
        assert_eq!(value.get("method").and_then(Json::as_str), Some("initialize"));
        let params = value.get("params").expect("params");
        assert_eq!(params.get("ok"), Some(&Json::Bool(true)));
        assert_eq!(
            params.get("items").and_then(Json::as_array).map(|items| items.len()),
            Some(3)
        );
        let rendered = value.render();
        assert_eq!(Json::parse(&rendered), Ok(value));
    }

    #[test]
    fn should_decode_escapes_and_surrogate_pairs() {
        let value = Json::parse(r#""a\n\u00e9\ud83d\ude00""#).expect("valid");
        assert_eq!(value.as_str(), Some("a\né😀"));
    }

    #[test]
    fn should_reject_lone_surrogates_and_trailing_text() {
        assert!(Json::parse(r#""\ud800""#).is_err());
        assert!(Json::parse(r#""\udc00""#).is_err());
        assert!(Json::parse("{} x").is_err());
        assert!(Json::parse("").is_err());
    }

    #[test]
    fn should_reject_fractional_numbers() {
        assert!(Json::parse("1.5").is_err());
        assert!(Json::parse("1e3").is_err());
    }

    #[test]
    fn should_reject_deep_nesting() {
        let deep = "[".repeat(200) + &"]".repeat(200);
        assert!(Json::parse(&deep).is_err());
    }

    #[test]
    fn should_escape_control_bytes_on_render() {
        let value = Json::String("a\u{1}b\n".to_owned());
        assert_eq!(value.render(), "\"a\\u0001b\\n\"");
    }
}

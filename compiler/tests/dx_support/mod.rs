//! Shared scanners for the DX specification suites.
//!
//! The grammar completeness case reads two normative data files: the EBNF
//! productions and the TextMate JSON. These helpers keep the scanning rules in
//! one place so the specification and implementation suites agree on what
//! "covers the term" means.

/// Returns the quoted terms of one EBNF production.
///
/// A production starts on a line whose first token is `name` followed by `=`,
/// and ends at the first semicolon. Quoted strings on the production lines are
/// its terms.
pub fn production_terms<'a>(grammar: &'a str, name: &str) -> Vec<&'a str> {
    let mut terms = Vec::new();
    let mut active = false;
    for line in grammar.lines() {
        if !active {
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix(name) else {
                continue;
            };
            if !rest.trim_start().starts_with('=') {
                continue;
            }
            active = true;
        }
        let mut rest = line;
        while let Some(open) = rest.find('"') {
            rest = &rest[open + 1..];
            let Some(close) = rest.find('"') else {
                break;
            };
            terms.push(&rest[..close]);
            rest = &rest[close + 1..];
        }
        if line.contains(';') {
            break;
        }
    }
    terms
}

/// Returns the decoded values of `match`, `begin`, and `end` pattern keys.
///
/// The grammar uses only these three keys for token patterns. A later key is
/// ignored, so metadata such as `scopeName` is not mistaken for a pattern.
pub fn pattern_strings(json: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < json.len() {
        if json.as_bytes()[index] != b'"' {
            index += 1;
            continue;
        }
        let (key, next) = read_string(json, index).expect("the grammar contains valid JSON strings");
        index = next;
        let mut cursor = skip_whitespace(json, index);
        if cursor < json.len()
            && json.as_bytes()[cursor] == b':'
            && matches!(key.as_str(), "match" | "begin" | "end")
        {
            cursor = skip_whitespace(json, cursor + 1);
            if cursor < json.len() && json.as_bytes()[cursor] == b'"' {
                let (value, next) =
                    read_string(json, cursor).expect("a pattern key carries a JSON string");
                values.push(value);
                index = next;
            }
        }
    }
    values
}

/// Returns every decoded JSON string, keys and values alike.
///
/// Metadata assertions ("this document names `source.clorraine`") use this
/// view; pattern assertions use [`pattern_strings`] so a scope name cannot
/// satisfy a missing keyword by accident.
pub fn json_strings(json: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < json.len() {
        if json.as_bytes()[index] == b'"' {
            let (value, next) = read_string(json, index).expect("the JSON text is well formed");
            values.push(value);
            index = next;
        } else {
            index += 1;
        }
    }
    values
}

/// True when `term` is a whole identifier token in `pattern`.
///
/// A term must not match inside a longer name: `i8` does not match `i16`, and
/// `as` does not match `case`. This is the word-boundary contract the EBNF
/// completeness test enforces.
pub fn word_bounded(pattern: &str, term: &str) -> bool {
    // A `\b` escape is two source characters; leaving it in place would glue
    // the `b` to the following term and hide a standalone `\bterm\b` match.
    let normalized = pattern.replace("\\b", " ");
    normalized
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == term)
}

/// Reads one JSON string starting at `start`, returning its decoded value and
/// the index just past the closing quote.
fn read_string(text: &str, start: usize) -> Option<(String, usize)> {
    let mut characters = text[start..].char_indices();
    if characters.next()?.1 != '"' {
        return None;
    }
    let mut value = String::new();
    while let Some((offset, character)) = characters.next() {
        match character {
            '"' => return Some((value, start + offset + 1)),
            '\\' => {
                let (_, escaped) = characters.next()?;
                match escaped {
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    '/' => value.push('/'),
                    'b' => value.push('\u{8}'),
                    'f' => value.push('\u{c}'),
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    'u' => {
                        let mut code = 0_u32;
                        for _ in 0..4 {
                            let (_, digit) = characters.next()?;
                            code = code * 16 + digit.to_digit(16)?;
                        }
                        value.push(char::from_u32(code)?);
                    }
                    _ => return None,
                }
            }
            other => value.push(other),
        }
    }
    None
}

/// Skips ASCII whitespace from `start`.
fn skip_whitespace(text: &str, start: usize) -> usize {
    let mut index = start;
    while index < text.len() && text.as_bytes()[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_production_terms_across_lines() {
        let grammar = "keyword = \"module\" | \"import\"\n        | \"func\" ;\nother = \"nope\" ;\n";
        assert_eq!(production_terms(grammar, "keyword"), ["module", "import", "func"]);
    }

    #[test]
    fn decodes_pattern_escapes_independently_of_other_strings() {
        let json = r#"{"name":"Clorraine","match":"\\b(module|i8)\\b","begin":"\\$","end":"\""}"#;
        assert_eq!(pattern_strings(json), ["\\b(module|i8)\\b", "\\$", "\""]);
    }

    #[test]
    fn word_boundaries_do_not_match_prefixes() {
        assert!(word_bounded("\\b(module|import)\\b", "module"));
        assert!(word_bounded("\\b(i8|i16)\\b", "i8"));
        assert!(!word_bounded("\\b(i8|i16)\\b", "i"));
        assert!(!word_bounded("case", "as"));
        assert!(word_bounded("readVolatile8", "readVolatile8"));
        assert!(word_bounded("\\bdeleter\\b", "deleter"));
    }
}

//! In-module tests for the scanner.

use super::*;
use crate::diag::Diagnostics;

/// Lexes a source string; returns the tokens without the trailing `Eof` and
/// the diagnostic codes in report order.
fn tokenize(source: &str) -> (Vec<Token>, Vec<String>) {
    let mut sources = SourceMap::new();
    let id = sources.add_file("test.clo".to_owned(), source.to_owned());
    let mut diagnostics = Diagnostics::new(false, None);
    let tokens = Lexer::new(&sources, id, &mut diagnostics).lex();
    let codes = diagnostics
        .items()
        .iter()
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect();
    let tokens = tokens
        .into_iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .collect();
    (tokens, codes)
}

/// Token kinds of a source string.
fn kinds(source: &str) -> Vec<TokenKind> {
    tokenize(source)
        .0
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

/// Diagnostic codes of a source string.
fn codes(source: &str) -> Vec<String> {
    tokenize(source).1
}

#[test]
fn keywords_identifiers_and_contextual_as() {
    assert_eq!(
        kinds("module Demo.func let true null sizeof as"),
        [
            TokenKind::Keyword(Keyword::Module),
            TokenKind::Ident("Demo".to_owned()),
            TokenKind::Punct(Punct::Dot),
            TokenKind::Keyword(Keyword::Func),
            TokenKind::Keyword(Keyword::Let),
            TokenKind::Keyword(Keyword::True),
            TokenKind::Keyword(Keyword::Null),
            // Reserved built-ins and `as` are ordinary identifiers.
            TokenKind::Ident("sizeof".to_owned()),
            TokenKind::Ident("as".to_owned()),
        ]
    );
}

#[test]
fn punctuation_table_is_complete() {
    let source = "== != <= >= << >> && || += -= *= /= %= &= |= ^= <<= >>= => ++ -- \
                  < > = . , : ; ? ( ) [ ] { } ~ ! & | ^ + - * / %";
    assert_eq!(kinds(source).len(), 45);
    assert!(kinds(source).iter().all(|kind| matches!(kind, TokenKind::Punct(_))));
}

#[test]
fn numeric_forms_decode_their_values() {
    assert_eq!(
        kinds("255 1_000 0xFF 0xFF_FF 0b1010 0B1_0 1.5 1e3 6.02e23"),
        [
            TokenKind::Int(255),
            TokenKind::Int(1000),
            TokenKind::Int(255),
            TokenKind::Int(65535),
            TokenKind::Int(10),
            TokenKind::Int(2),
            TokenKind::Float(1.5),
            TokenKind::Float(1000.0),
            TokenKind::Float(6.02e23),
        ]
    );
}

#[test]
fn malformed_numbers_report_e20004() {
    for source in ["1.", ".5", "0x", "0b1012", "123abc", "1u8", "1e", "1__0", "99999999999999999999"] {
        let (tokens, reported) = tokenize(source);
        assert_eq!(reported, ["E20004"], "source {source:?}");
        assert_eq!(tokens.len(), 1, "source {source:?}");
        assert_eq!(tokens[0].kind, TokenKind::Error, "source {source:?}");
    }
}

#[test]
fn integer_member_access_is_not_a_float() {
    assert_eq!(
        kinds("1.len()"),
        [
            TokenKind::Int(1),
            TokenKind::Punct(Punct::Dot),
            TokenKind::Ident("len".to_owned()),
            TokenKind::Punct(Punct::LParen),
            TokenKind::Punct(Punct::RParen),
        ]
    );
}

#[test]
fn character_literals_decode_one_byte() {
    assert_eq!(
        kinds(r#"'A' '\n' '\x41' '\''"#),
        [
            TokenKind::Char(b'A'),
            TokenKind::Char(b'\n'),
            TokenKind::Char(0x41),
            TokenKind::Char(b'\''),
        ]
    );
    assert_eq!(codes("'ab'"), ["E20007"]);
    assert_eq!(codes("'€'"), ["E20007"]);
    assert_eq!(codes("''"), ["E20007"]);
    assert_eq!(codes(r"'\q'"), ["E20003"]);
    assert_eq!(codes(r"'\x4'"), ["E20003"]);
}

#[test]
fn string_literals_keep_their_bytes() {
    assert_eq!(kinds(r#""A""#), [TokenKind::Str(vec![b'A'])]);
    assert_eq!(kinds(r#""\n""#), [TokenKind::Str(vec![b'\n'])]);
    assert_eq!(kinds(r#""\x41""#), [TokenKind::Str(vec![0x41])]);
    assert_eq!(kinds(r#""€""#), [TokenKind::Str(vec![0xE2, 0x82, 0xAC])]);
    assert_eq!(codes(r#""bad \q escape""#), ["E20003"]);
}

#[test]
fn unterminated_literals_report_e20002() {
    assert_eq!(codes("\"no close\n"), ["E20002"]);
    assert_eq!(codes("'a\n"), ["E20002"]);
    assert_eq!(codes("$\"no close\n"), ["E20002"]);
}

#[test]
fn template_parts_split_literals_and_placeholders() {
    let source = r#"$"a{x + 1}b""#;
    let mut sources = SourceMap::new();
    let id = sources.add_file("t.clo".to_owned(), source.to_owned());
    let mut diagnostics = Diagnostics::new(false, None);
    let tokens = Lexer::new(&sources, id, &mut diagnostics).lex();
    let TokenKind::Template(parts) = &tokens[0].kind else {
        panic!("expected a template token, got {:?}", tokens[0].kind);
    };
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], TemplatePart::Literal(b"a".to_vec()));
    let TemplatePart::Placeholder(span) = parts[1] else {
        panic!("expected a placeholder, got {:?}", parts[1]);
    };
    assert_eq!(sources.snippet(span), "x + 1");
    assert_eq!(parts[2], TemplatePart::Literal(b"b".to_vec()));
    assert!(diagnostics.is_empty());
}

#[test]
fn template_doubled_braces_are_literal() {
    assert_eq!(
        kinds(r#"$"{{x}}""#),
        [TokenKind::Template(vec![TemplatePart::Literal(b"{x}".to_vec())])]
    );
}

#[test]
fn nested_template_reports_e90005() {
    let (tokens, reported) = tokenize(r#"$"outer { $"inner {name}" } end""#);
    assert_eq!(reported, ["E90005"]);
    assert!(matches!(&tokens[0].kind, TokenKind::Template(parts) if parts.len() == 3));
}

#[test]
fn unbalanced_template_reports_e10503() {
    let (tokens, reported) = tokenize(r#"$"n={n";"#);
    assert_eq!(reported, ["E10503"]);
    assert_eq!(tokens[0].kind, TokenKind::Error);
}

#[test]
fn comments_are_trivia() {
    assert_eq!(
        kinds("// line\n/// doc\n/* block */ x"),
        [TokenKind::Ident("x".to_owned())]
    );
    assert_eq!(codes("/* never closed"), ["E20001"]);
}

#[test]
fn foreign_syntax_reports_e20008() {
    for source in ["->", "::", "#", "@", "?.", "??"] {
        let (_, reported) = tokenize(source);
        assert_eq!(reported, ["E20008"], "source {source:?}");
    }
}

#[test]
fn stray_characters_report_e20000() {
    let (tokens, reported) = tokenize("` \\ € $");
    assert_eq!(reported, ["E20000", "E20000", "E20000", "E20000"]);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Error));
}

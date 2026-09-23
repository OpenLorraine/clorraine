//! Output sanitizing for the `clorraine` developer driver.
//!
//! Some driver messages interpolate text the user controls: a source file
//! name, an artifact path derived from it, and the target C compiler's first
//! error line. A control byte in any of those (for example `ESC`) would be
//! written straight to the terminal and could drive terminal escape sequences.
//!
//! [`sanitize`] replaces every control character with the Unicode replacement
//! character, so the printed text is inert. Ordinary text is returned borrowed
//! and unchanged, so the common case allocates nothing.

use std::borrow::Cow;

/// The character written in place of a control character.
const REPLACEMENT: char = '\u{FFFD}';

/// Returns `text` with every control character replaced.
///
/// The caller applies this to a user-controlled field only, never to the fixed
/// message around it: the surrounding text may carry intentional newlines that
/// must survive. A `Cow` borrows `text` when it is already clean.
///
/// # Examples
///
/// ```text
/// sanitize("build/hello.c")   == "build/hello.c"
/// sanitize("a<ESC>[31m")      == "a<REPLACEMENT>[31m"
/// ```
pub fn sanitize(text: &str) -> Cow<'_, str> {
    if text.chars().any(char::is_control) {
        Cow::Owned(
            text.chars()
                .map(|character| {
                    if character.is_control() {
                        REPLACEMENT
                    } else {
                        character
                    }
                })
                .collect(),
        )
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_text_is_returned_unchanged() {
        assert_eq!(sanitize("build/clorraine/x/host.c"), "build/clorraine/x/host.c");
        assert!(matches!(sanitize("plain"), Cow::Borrowed(_)));
    }

    #[test]
    fn control_characters_are_replaced_one_for_one() {
        assert_eq!(sanitize("a\u{1b}b"), "a\u{fffd}b");
        assert_eq!(sanitize("a\nb\tc"), "a\u{fffd}b\u{fffd}c");
    }
}

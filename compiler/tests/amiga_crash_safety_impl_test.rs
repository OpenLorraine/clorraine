//! Implementation tests for the hosted-amiga shim paths that used to crash or
//! hang a real machine.
//!
//! The specification cases run the probes under `vamos` and on emulated
//! Amigas. These cases pin the shim internals that make them pass: where each
//! entry opens the library it needs, and the order of the calls that keep a
//! safe program away from a missing library base.

use std::path::Path;

/// One shim source file, read directly so each contract is pinned where it is
/// implemented.
fn shim(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime").join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// The text of the function whose definition starts with `signature`, up to
/// its closing brace in column 0.
fn body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(&format!("\n{signature}\n{{"))
        .unwrap_or_else(|| panic!("the shim has no definition {signature:?}"))
        + 1;
    let rest = &source[start..];
    let end = rest
        .find("\n}\n")
        .unwrap_or_else(|| panic!("{signature:?} has no closing brace"));
    &rest[..end + 2]
}

/// Asserts that each needle appears, in the given order.
fn assert_order(tag: &str, haystack: &str, needles: &[&str]) {
    let mut from = 0;
    for needle in needles {
        let at = haystack[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("{tag}: missing {needle:?} after offset {from} in:\n{haystack}"));
        from += at + needle.len();
    }
}

/// The Intuition open both entries use: the shared recorded open, so the exit
/// hook closes a base opened here like one opened by a screen or window.
const OPEN_INTUITION: &str = "cl_lib_open(&IntuitionBase, \"intuition.library\", 33)";

/// `showAlert` checks its body first, so a null or over-long body still opens
/// nothing, then opens Intuition before it allocates the alert message and
/// returns with no alert when the library cannot be opened.
#[test]
fn the_alert_opens_intuition_after_the_body_checks_and_before_the_alert() {
    let source = shim("shim_hosted_amiga.c");
    let alert = body(&source, "void cl_show_alert(char *body)");
    assert_order(
        "alert",
        alert,
        &[
            "if (body == NULL)",
            "length > CL_WINDOW_BODY_MAX",
            &format!("if ({OPEN_INTUITION} == NULL) {{\n        return;\n    }}"),
            "AllocMem(CL_ALERT_MESSAGE_MAX, MEMF_CLEAR)",
            "DisplayAlert(1, (STRPTR)message, 20)",
            "FreeMem(message, CL_ALERT_MESSAGE_MAX)",
        ],
    );
}

/// `savePublicScreen` checks its arguments first, then opens Intuition and
/// returns false unless the library is V36 or newer, because public screens
/// are a 2.0 feature; only then does it lock the screen.
#[test]
fn the_public_screen_capture_opens_intuition_v36_before_locking_the_screen() {
    let source = shim("shim_hosted_amiga_20.c");
    let capture = body(&source, "BOOL cl_save_public_screen(char *name, char *path)");
    assert_order(
        "pubscreen",
        capture,
        &[
            "if (name == NULL || path == NULL)",
            &format!("if ({OPEN_INTUITION} == NULL ||"),
            "IntuitionBase->LibNode.lib_Version < 36) {\n        return FALSE;\n    }",
            "LockPubScreen((STRPTR)name)",
            "cl_screen_save_native((void *)screen, path)",
            "UnlockPubScreen(NULL, screen)",
        ],
    );
}

/// Only the two screen and window constructors and the two entries that can
/// run first open Intuition; every other Intuition user receives a screen or
/// window handle, so the library is already open when it runs.
#[test]
fn intuition_is_opened_only_where_it_may_not_be_open_yet() {
    let base = shim("shim_hosted_amiga.c");
    let suffix = shim("shim_hosted_amiga_20.c");
    assert_eq!(base.matches(OPEN_INTUITION).count(), 3, "base shim opens");
    assert_eq!(suffix.matches(OPEN_INTUITION).count(), 1, "2.0 suffix opens");
}

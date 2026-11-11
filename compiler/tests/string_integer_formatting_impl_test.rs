//! Implementation tests for the integer `String` formatter emission.
//!
//! These cases pin the emitted shim text: the shared decimal helpers exist in
//! both profiles, the integer converters no longer call the C library
//! formatter, the float converter still uses `%g`, and a template with no
//! placeholder still lowers to a plain string view.
//!
//! The assertions are emission pins. They deliberately do not duplicate the
//! behavior cases; a change to the intended emission should update these
//! pins, while a behavior change should fail the specification suite.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-string-format-impl-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// The compiler under test. A relative `RDC_CLC` is resolved against the
/// workspace root because Cargo runs integration tests from `compiler/`.
fn clc_path() -> String {
    match std::env::var("RDC_CLC") {
        Ok(value) => {
            let path = PathBuf::from(&value);
            if path.is_absolute() {
                value
            } else {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("the workspace root")
                    .join(path)
                    .display()
                    .to_string()
            }
        }
        Err(_) => env!("CARGO_BIN_EXE_clc").to_owned(),
    }
}

/// Emits one source for a profile and returns the C text.
fn emit(tag: &str, profile: &str, source: &str, name: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join(name);
    let output = Command::new(clc_path())
        .args([
            "--profile",
            profile,
            "-o",
            output_path.to_str().expect("output path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// Asserts `text` contains `needle`.
fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: emitted C does not contain {needle:?}"
    );
}

/// Asserts `text` does not contain `needle`.
fn assert_absent(tag: &str, text: &str, needle: &str) {
    assert!(
        !text.contains(needle),
        "{tag}: emitted C unexpectedly contains {needle:?}"
    );
}

/// A template that exercises the integer, float, string, and literal paths.
const MIXED: &str = r#"module spec.sifi.mixed;

import { println } from std.io;
import { stringOf } from std.text;

func main(): i32 {
    let n: i32 = -7;
    let u: u32 = 9;
    let f: f64 = 1.5;
    let s: String = stringOf("abc");
    let t: str = "x";
    println($"n={n} u={u} f={f} s={s} t={t}");
    return 0;
}
"#;

/// The host emission defines both decimal helpers and uses them in place of
/// the C library integer formatter.
#[test]
fn the_host_shim_has_the_decimal_helpers() {
    let emitted = emit("host-helpers", "host", MIXED, "host.c");
    assert_contains(
        "host-helpers",
        &emitted,
        "static unsigned int cl_format_u32(unsigned char *dst, unsigned int value)",
    );
    assert_contains(
        "host-helpers",
        &emitted,
        "static unsigned int cl_format_i32(unsigned char *dst, int value)",
    );
    assert_contains("host-helpers", &emitted, "cl_string_new(11)");
    assert_contains("host-helpers", &emitted, "cl_string_new(10)");
    assert_contains("host-helpers", &emitted, "string->cap = written;");
}

/// The host integer converters no longer use `snprintf`; the float converter
/// keeps its `%g` formatting.
#[test]
fn the_host_integer_converters_do_not_use_snprintf() {
    let emitted = emit("host-no-snprintf", "host", MIXED, "host.c");
    assert_absent("host-no-snprintf", &emitted, "snprintf(buffer, sizeof buffer, \"%d\"");
    assert_absent("host-no-snprintf", &emitted, "snprintf(buffer, sizeof buffer, \"%u\"");
    assert_contains("host-no-snprintf", &emitted, "snprintf(buffer, sizeof buffer, \"%g\"");
}

/// The Amiga emission carries the same helpers and the direct-write bodies.
#[test]
fn the_amiga_shim_has_the_decimal_helpers() {
    let emitted = emit("amiga-helpers", "hosted-amiga", MIXED, "amiga.c");
    assert_contains(
        "amiga-helpers",
        &emitted,
        "static ULONG cl_format_u32(unsigned char *dst, unsigned int value)",
    );
    assert_contains(
        "amiga-helpers",
        &emitted,
        "static ULONG cl_format_i32(unsigned char *dst, int value)",
    );
    assert_contains("amiga-helpers", &emitted, "cl_string_new(11)");
    assert_contains("amiga-helpers", &emitted, "cl_string_new(10)");
    assert_contains("amiga-helpers", &emitted, "string->cap = written;");
}

/// A template without a placeholder still lowers to a plain string view and
/// never calls the builder.
#[test]
fn a_literal_only_template_is_still_a_plain_view() {
    let source = r#"module spec.sifi.literal;

import { println } from std.io;

func main(): i32 {
    println($"plain text");
    return 0;
}
"#;
    let emitted = emit("literal", "host", source, "host.c");
    assert_contains("literal", &emitted, "cl_str(\"plain text\")");
    assert_absent("literal", &emitted, "cl_string_concat((ClStrPart[])");
}

/// The same source path and profile emit byte-identical C.
///
/// Both runs use one source file because the `#line` mapping records the
/// source path; the output path is not part of the C.
#[test]
fn the_emission_is_deterministic() {
    let scratch = ScratchDir::new("determinism");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, MIXED).expect("write main.clo");
    let mut emitted = Vec::new();
    for name in ["a.c", "b.c"] {
        let output_path = scratch.path().join(name);
        let output = Command::new(clc_path())
            .args([
                "--profile",
                "host",
                "-o",
                output_path.to_str().expect("output path"),
                source_path.to_str().expect("source path"),
            ])
            .output()
            .expect("failed to spawn the compiler binary");
        assert_eq!(
            output.status.code(),
            Some(0),
            "determinism: the front end rejected the program; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        emitted.push(std::fs::read_to_string(&output_path).expect("read the emitted C"));
    }
    assert_eq!(
        emitted[0], emitted[1],
        "the same input emitted different C"
    );
}

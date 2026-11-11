//! Implementation tests for direct template formatting.
//!
//! The cases pin the emitted parts: a template call carries scalar kind codes,
//! literals use a byte part, `String`/`str` use an owned copy, integer
//! placeholders no longer call the converter, the explicit `text` path still
//! uses it, and both profiles emit the same shape. The behavior oracle is the
//! specification suite.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-template-direct-impl-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

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

fn template_line<'a>(tag: &str, emitted: &'a str) -> &'a str {
    emitted
        .lines()
        .find(|line| line.contains("cl_string_concat((ClStrPart[])"))
        .unwrap_or_else(|| panic!("{tag}: the template builder call is missing"))
}

fn assert_contains(tag: &str, text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "{tag}: emitted C does not contain {needle:?}"
    );
}

const MIXED: &str = r#"module spec.tdfi.mixed;
import { println } from std.io;
import { stringOf } from std.text;
func main(): i32 {
    let n: i32 = -7;
    let u: u32 = 9;
    let f: f64 = 1.5;
    let b: bool = true;
    let s: String = stringOf("abc");
    let t: str = "x";
    println($"n={n} u={u} f={f} b={b} s={s} t={t}");
    return 0;
}
"#;

/// The host template call carries scalar kinds, an owned String/view copy, a
/// bool part, and a float part.
#[test]
fn the_host_template_uses_typed_parts() {
    let emitted = emit("host-parts", "host", MIXED, "host.c");
    let line = template_line("host-parts", &emitted);
    assert_contains("host-parts", line, "cl_str_part_i32(");
    assert_contains("host-parts", line, "cl_str_part_u32(");
    assert_contains("host-parts", line, "cl_str_part_owned(cl_string_copy(");
    assert_contains("host-parts", line, "cl_str_part_owned(cl_text_str(");
    assert_contains("host-parts", line, "cl_str_part_bytes(");
    assert_contains("host-parts", line, "cl_str_part_bool(");
    assert_contains("host-parts", line, "cl_str_part_f64(");
    assert_contains("host-parts", &emitted, "#define CL_STR_KIND_I32 2u");
    assert!(
        !line.contains(".owned =") && !line.contains(".i =") && !line.contains(".u ="),
        "host-parts: the program body must not use designated initializers"
    );
}

/// The integer placeholders no longer call the converters; the explicit path
/// still does.
#[test]
fn the_integer_placeholders_do_not_call_the_converters() {
    let emitted = emit("no-converter", "host", MIXED, "host.c");
    let line = template_line("no-converter", &emitted);
    assert!(
        !line.contains("cl_text_i32("),
        "no-converter: an i32 placeholder still calls the converter"
    );
    assert!(
        !line.contains("cl_text_u32("),
        "no-converter: a u32 placeholder still calls the converter"
    );

    let explicit = r#"module spec.tdfi.explicit;
import { println } from std.io;
import { text } from std.fmt;
func main(): i32 {
    println(text(7));
    return 0;
}
"#;
    let emitted = emit("explicit", "host", explicit, "host.c");
    assert_contains("explicit", &emitted, "cl_text_i32((i32)(7))");
}

/// The Amiga emission has the same typed parts and kind macros.
#[test]
fn the_amiga_template_uses_typed_parts() {
    let emitted = emit("amiga-parts", "hosted-amiga", MIXED, "amiga.c");
    let line = template_line("amiga-parts", &emitted);
    assert_contains("amiga-parts", line, "cl_str_part_i32(");
    assert_contains("amiga-parts", line, "cl_str_part_u32(");
    assert_contains("amiga-parts", line, "cl_str_part_owned(cl_string_copy(");
    assert_contains("amiga-parts", &emitted, "#define CL_STR_KIND_I32 2u");
}

/// A literal-only template is still a plain view with no builder call.
#[test]
fn a_literal_only_template_is_still_a_plain_view() {
    let source = r#"module spec.tdfi.literal;
import { println } from std.io;
func main(): i32 {
    println($"plain text");
    return 0;
}
"#;
    let emitted = emit("literal", "host", source, "host.c");
    assert_contains("literal", &emitted, "cl_str(\"plain text\")");
    assert!(
        !emitted.contains("cl_string_concat((ClStrPart[])"),
        "literal: a placeholder-free template must not call the builder"
    );
}

/// The same source path and profile emit byte-identical C.
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
    assert_eq!(emitted[0], emitted[1], "the same input emitted different C");
}

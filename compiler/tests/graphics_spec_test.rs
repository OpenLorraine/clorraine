//! Graphics standard-library specification tests.
//!
//! The graphics surface is hosted-amiga only. These cases pin the contract from
//! the specification before the modules exist: a program that imports the
//! screen and drawing surface must compile with zero diagnostics, and a
//! possibly-null screen value used before a `!= null` check must be rejected by
//! the existing nullable-use rule. The red phase is the unresolved-module
//! compile failure while the modules are still absent.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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
            "clc-graphics-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
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

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Writes `source` to a scratch file and compiles it for `profile`.
///
/// The output path is inside the scratch directory, so nothing touches the
/// repository tree.
fn compile_source(tag: &str, profile: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ])
}

/// A program that imports both graphics modules compiles for hosted-amiga.
///
/// The program exercises the whole exported surface: it opens a screen, sets a
/// color, clears, fills a rectangle, and draws a line. The compiler must accept
/// it with zero diagnostics and exit zero.
#[test]
fn importing_the_graphics_modules_compiles_for_hosted_amiga() {
    let source = r#"module tests.graphics.compile;

import { Screen, openScreen } from std.amiga.intuition;
import { setColor, fill, draw, clear } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let title: String = stringOf("graphics");
    let screen: Screen? = openScreen(320, 256, 4, toCString(title));
    if (screen == null) {
        return 1;
    }
    setColor(screen, 1, 255, 32, 32);
    clear(screen, 0);
    fill(screen, 10, 10, 100, 100, 1);
    draw(screen, 0, 0, 319, 255, 1);
    return 0;
}
"#;
    let output = compile_source("compile", "hosted-amiga", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the graphics program must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the graphics program must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A possibly-null screen used before a check is rejected.
///
/// `openScreen` returns `Screen?`, and every drawing call requires a non-null
/// `Screen`. Passing the unchecked value must be rejected by the existing
/// nullable-use rule with `E10010`.
#[test]
fn using_a_nullable_screen_without_a_check_is_rejected() {
    let source = r#"module tests.graphics.nullable;

import { Screen, openScreen } from std.amiga.intuition;
import { setColor } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let title: String = stringOf("graphics");
    let screen: Screen? = openScreen(320, 256, 4, toCString(title));
    setColor(screen, 1, 255, 32, 32);
    return 0;
}
"#;
    let output = compile_source("nullable", "hosted-amiga", source);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the nullable use must be rejected with exit 1; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let first_line = stderr.lines().next().unwrap_or_default();
    assert!(
        first_line.starts_with("error E10010:"),
        "expected an E10010 first line, got:\n{stderr}"
    );
}

/// The graphics corpus program compiles clean for hosted-amiga.
///
/// The golden corpus is the acceptance target, so its zero-diagnostic contract
/// is asserted here directly rather than inferred from the Amiga build leg,
/// which only enforces the C compiler's warnings.
#[test]
fn the_graphics_corpus_compiles_with_zero_diagnostics() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/tests/corpus/graphics/program.clo");
    let scratch = ScratchDir::new("corpus");
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        corpus.display().to_string(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "the graphics corpus must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        run.stderr.is_empty(),
        "the graphics corpus must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

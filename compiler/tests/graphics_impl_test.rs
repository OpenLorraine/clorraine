//! Graphics standard-library implementation tests.
//!
//! These cases inspect the C the compiler emits for a hosted-amiga program that
//! draws through the two graphics modules. They pin the internals the
//! specification cases leave open: the application surface lowers to the
//! standard-library wrappers rather than raw `ffi`, the wrappers call all six
//! shim functions, the screen drop is null-safe, and the emitted application
//! code contains no `unsafe` construct.
//!
//! The hosted-amiga translation unit cannot be compiled on the host, but its
//! text can be asserted directly.

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
            "clc-graphics-impl-{}-{tag}-{sequence}",
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

/// Compiles `source` for hosted-amiga and returns the emitted C.
fn emit_hosted_amiga(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// A program that opens a screen, draws with every call, and drops the screen.
const DRAWING_PROGRAM: &str = r#"module impl.graphics;

import { Screen, openScreen } from std.amiga.intuition;
import { setColor, fill, draw, clear } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let title: String = stringOf("impl");
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

/// The emitted C calls all six shim functions through the module wrappers.
///
/// The application surface sits above `ffi`: the program names the exported
/// wrappers, and each wrapper body carries the raw shim call. The test asserts
/// both layers are present, so the wrappers are not optimised away and the shim
/// contract is actually used.
#[test]
fn generated_c_uses_the_six_shims_through_the_wrappers() {
    let c = emit_hosted_amiga("shims", DRAWING_PROGRAM);
    for symbol in [
        "cl_open_screen(",
        "cl_close_screen(",
        "cl_set_color(",
        "cl_fill(",
        "cl_draw(",
        "cl_clear(",
    ] {
        assert!(c.contains(symbol), "the emitted C must name `{symbol}`");
    }
    for wrapper in [
        "cl_std_amiga_intuition_openScreen(",
        "cl_std_amiga_graphics_setColor(",
        "cl_std_amiga_graphics_fill(",
        "cl_std_amiga_graphics_draw(",
        "cl_std_amiga_graphics_clear(",
    ] {
        assert!(
            c.contains(wrapper),
            "the emitted C must call the wrapper `{wrapper}`"
        );
    }
}

/// The generated drop closes the screen only behind a null check.
///
/// The owning handle lowers to a cleanup that calls the deleter through an
/// `if (screen)` guard, so a failed open (a null handle) is safe to drop.
#[test]
fn the_screen_drop_is_null_safe() {
    let c = emit_hosted_amiga("drop", DRAWING_PROGRAM);
    let guard = "if (screen) { cl_close_screen(screen); }";
    assert!(
        c.contains(guard),
        "the emitted C must drop the screen behind `{guard}`"
    );
}

/// The emitted application code contains no `unsafe` construct.
///
/// The graphics surface is reachable from safe Clorraine: the raw pointers and
/// the library bases stay inside the shim, so no `unsafe` marker reaches the
/// generated C.
#[test]
fn generated_c_contains_no_unsafe_construct() {
    let c = emit_hosted_amiga("surface", DRAWING_PROGRAM);
    assert!(
        !c.contains("unsafe"),
        "the graphics surface must not emit an `unsafe` construct"
    );
}

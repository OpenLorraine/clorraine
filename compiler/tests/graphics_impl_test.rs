//! Graphics standard-library implementation tests.
//!
//! These cases inspect the C the compiler emits for a hosted-amiga program that
//! draws through the two graphics modules. They pin the internals the
//! specification cases leave open: the application surface lowers to the
//! standard-library wrappers rather than raw `ffi`, each wrapper body carries
//! its raw shim call, and every screen drop is null-guarded.
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

/// The emitted C routes every drawing call through the module wrappers.
///
/// The application surface sits above `ffi`: the program calls the exported
/// `cl_std_amiga_*` wrappers, and each wrapper body carries the raw shim call
/// with the screen argument. The test anchors on the call sites rather than the
/// bare symbol names, so a prototype or an empty wrapper body cannot satisfy it.
#[test]
fn every_graphics_call_sits_above_ffi() {
    let c = emit_hosted_amiga("routing", DRAWING_PROGRAM);
    for wrapper_call in [
        "cl_std_amiga_intuition_openScreen(320,",
        "cl_std_amiga_graphics_setColor(screen,",
        "cl_std_amiga_graphics_fill(screen,",
        "cl_std_amiga_graphics_draw(screen,",
        "cl_std_amiga_graphics_clear(screen,",
    ] {
        assert!(
            c.contains(wrapper_call),
            "the application must call the wrapper `{wrapper_call}`"
        );
    }
    for shim_call in [
        "cl_open_screen(width",
        "cl_set_color(screen,",
        "cl_fill(screen,",
        "cl_draw(screen,",
        "cl_clear(screen,",
    ] {
        assert!(
            c.contains(shim_call),
            "a wrapper body must call the shim `{shim_call}`"
        );
    }
}

/// Every screen drop is null-guarded.
///
/// The owning handle lowers to a cleanup that calls the deleter behind an
/// `if (screen)` guard, so a failed open (a null handle) is safe to drop. The
/// test requires that the guard is present and that no `cl_close_screen(screen)`
/// call appears outside it.
#[test]
fn every_screen_drop_is_null_guarded() {
    let c = emit_hosted_amiga("drop", DRAWING_PROGRAM);
    let call = "cl_close_screen(screen)";
    let guarded = "if (screen) { cl_close_screen(screen); }";
    assert!(
        c.contains(guarded),
        "the emitted C must drop the screen behind `{guarded}`"
    );
    assert_eq!(
        c.matches(call).count(),
        c.matches(guarded).count(),
        "every `{call}` call must sit inside `{guarded}`"
    );
}

/// Importing only the drawing module still injects its screen dependency.
///
/// The graphics module borrows the `Screen` handle from intuition, so a program
/// that imports the drawing module alone must still pull intuition in; the
/// opaque handle typedef proves it. The import is intentionally unused, so the
/// unrelated unused-import warning is not asserted here.
#[test]
fn importing_the_drawing_module_alone_injects_its_screen_dependency() {
    let source = r#"module impl.graphics_solo;

import { setColor } from std.amiga.graphics;

func main(): i32 {
    return 0;
}
"#;
    let c = emit_hosted_amiga("solo", source);
    assert!(
        c.contains("typedef void *ClScreen;"),
        "the screen handle typedef must be emitted when intuition is injected"
    );
}

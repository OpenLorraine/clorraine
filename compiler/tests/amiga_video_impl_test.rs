//! Native-video standard-library implementation tests.
//!
//! These cases inspect the C the compiler emits for a hosted-Amiga program that
//! uses `std.amiga.video`. They pin the internals the specification cases leave
//! open: the application surface lowers to the standard-library wrappers rather
//! than raw `ffi`, each wrapper body carries its raw shim call, and every owned
//! video handle drops behind a null guard.

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
            "clc-video-impl-{}-{tag}-{sequence}",
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

/// A program that uses one call from every part of the surface.
const ROUTING_PROGRAM: &str = r#"module impl.video.routing;

import {
    ChipBuffer, Bitmap, Copper, Display,
    allocChip, freeChip, newBitmap, freeBitmap, newCopper, copperEnd,
    openDisplay, displaySetCopper, displayFlip, displayFailed, waitVBlank
} from std.amiga.video;

func main(): i32 {
    let chip: ChipBuffer? = allocChip(64);
    if (chip == null) {
        return 1;
    }
    freeChip(chip);

    let front: Bitmap? = newBitmap(320, 256, 4);
    if (front == null) {
        return 2;
    }
    let back: Bitmap? = newBitmap(320, 256, 4);
    if (back == null) {
        return 3;
    }
    let copper: Copper? = newCopper();
    if (copper == null) {
        return 4;
    }
    copperEnd(copper);

    let display: Display? = openDisplay(front, back);
    if (display == null) {
        return 5;
    }
    if (!displaySetCopper(display, copper)) {
        return 6;
    }
    displayFlip(display);
    if (displayFailed(display)) {
        return 7;
    }
    waitVBlank();
    return 0;
}
"#;

/// Every video call sits above `ffi` and reaches the shim.
#[test]
fn every_video_call_sits_above_ffi() {
    let c = emit_hosted_amiga("routing", ROUTING_PROGRAM);
    for wrapper_call in [
        "cl_std_amiga_video_allocChip(64u)",
        "cl_std_amiga_video_freeChip(chip)",
        "cl_std_amiga_video_newBitmap(320,",
        "cl_std_amiga_video_newCopper(",
        "cl_std_amiga_video_copperEnd(copper)",
        "cl_std_amiga_video_openDisplay(front,",
        "cl_std_amiga_video_displaySetCopper(display,",
        "cl_std_amiga_video_displayFlip(display)",
        "cl_std_amiga_video_displayFailed(display)",
        "cl_std_amiga_video_waitVBlank(",
    ] {
        assert!(
            c.contains(wrapper_call),
            "the application must call the wrapper `{wrapper_call}`"
        );
    }
    for shim_call in [
        "cl_video_chip_alloc(",
        "cl_video_chip_free(buffer",
        "cl_video_bitmap_new(width",
        "cl_video_copper_new(",
        "cl_video_copper_end(copper",
        "cl_video_display_open(front",
        "cl_video_display_set_copper(display,",
        "cl_video_display_flip(display)",
        "cl_video_wait_vblank(",
    ] {
        assert!(
            c.contains(shim_call),
            "a wrapper body must call the shim `{shim_call}`"
        );
    }
}

/// A program that relies on generated drops for every owned handle.
const DROP_PROGRAM: &str = r#"module impl.video.drop;

import {
    ChipBuffer, Bitmap, Copper, Display,
    allocChip, newBitmap, newCopper, openDisplay, displayFlip
} from std.amiga.video;

func main(): i32 {
    let front: Bitmap? = newBitmap(64, 64, 4);
    if (front == null) {
        return 1;
    }
    let back: Bitmap? = newBitmap(64, 64, 4);
    if (back == null) {
        return 2;
    }
    let display: Display? = openDisplay(front, back);
    if (display == null) {
        return 3;
    }
    let chip: ChipBuffer? = allocChip(16);
    if (chip == null) {
        return 4;
    }
    let copper: Copper? = newCopper();
    if (copper == null) {
        return 5;
    }
    displayFlip(display);
    return 0;
}
"#;

/// Every owned handle drops behind a null guard.
#[test]
fn every_video_drop_is_null_guarded() {
    let c = emit_hosted_amiga("drop", DROP_PROGRAM);
    // The wrapper bodies contain one intentional unguarded deleter call each;
    // the generated drops live in `main`. Slice out the generated main so the
    // count is about drop sites, not wrapper definitions.
    let start = c
        .find("cl_impl_video_drop_main(void)\n{")
        .expect("the emitted C must contain the program entry definition");
    let body = &c[start..];
    let end = body
        .find("\n}\n")
        .map(|offset| offset + 3)
        .unwrap_or(body.len());
    let body = &body[..end];
    for (call, guarded) in [
        (
            "cl_video_copper_free(copper)",
            "if (copper) { cl_video_copper_free(copper); }",
        ),
        (
            "cl_video_chip_free(chip)",
            "if (chip) { cl_video_chip_free(chip); }",
        ),
        (
            "cl_video_display_free(display)",
            "if (display) { cl_video_display_free(display); }",
        ),
        (
            "cl_video_bitmap_free(back)",
            "if (back) { cl_video_bitmap_free(back); }",
        ),
        (
            "cl_video_bitmap_free(front)",
            "if (front) { cl_video_bitmap_free(front); }",
        ),
    ] {
        assert!(
            body.contains(guarded),
            "the emitted main must drop the handle behind `{guarded}`"
        );
        assert_eq!(
            body.matches(call).count(),
            body.matches(guarded).count(),
            "every `{call}` call in main must sit inside `{guarded}`"
        );
    }
    // The display must be released before the bitmaps it borrows. The other
    // handles are independent, but pinning the whole reverse-declaration
    // ladder makes a reordered cleanup fail here.
    // Each handle has an early reset guard before its assignment and the real
    // cleanup ladder at `__cl_c1`. The order check is about the ladder, so use
    // the last occurrence of each guard.
    let position = |needle: &str| {
        body.rfind(needle)
            .unwrap_or_else(|| panic!("missing cleanup `{needle}` in main"))
    };
    let copper = position("if (copper) { cl_video_copper_free(copper); }");
    let chip = position("if (chip) { cl_video_chip_free(chip); }");
    let display = position("if (display) { cl_video_display_free(display); }");
    let back = position("if (back) { cl_video_bitmap_free(back); }");
    let front = position("if (front) { cl_video_bitmap_free(front); }");
    assert!(
        copper < chip && chip < display && display < back && back < front,
        "the generated cleanup must run copper, chip, display, back, front"
    );
}

/// Importing the video module alone does not pull in the Intuition surface.
#[test]
fn the_video_module_does_not_inject_intuition() {
    let source = r#"module impl.video.solo;

import { Bitmap, newBitmap } from std.amiga.video;

func main(): i32 {
    let bitmap: Bitmap? = newBitmap(16, 16, 4);
    if (bitmap == null) {
        return 1;
    }
    return 0;
}
"#;
    let c = emit_hosted_amiga("solo", source);
    assert!(
        !c.contains("typedef void *ClScreen;"),
        "the native-video module must not inject the Intuition screen handle"
    );
}

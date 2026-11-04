//! Text/layers standard-library implementation tests.
//!
//! These cases inspect the C the compiler emits for a hosted-Amiga program
//! that uses `std.amiga.text` and `std.amiga.layers`: the application surface
//! lowers to the standard-library wrappers, each wrapper body carries its raw
//! shim call, and every owned handle drops behind a null guard in reverse
//! declaration order (layer before bitmap/font/screen).

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
            "clc-text-impl-{}-{tag}-{sequence}",
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
const ROUTING_PROGRAM: &str = r#"module impl.text.routing;

import { Screen, openScreen } from std.amiga.intuition;
import {
    Font, openFont, openDiskFont, defaultFont, textWidth, textHeight,
    drawText, freeFont
} from std.amiga.text;
import {
    Layer, newLayer, freeLayer, layerMove, layerSize, layerFill, layerClear,
    layerText, layerBeginUpdate, layerEndUpdate
} from std.amiga.layers;
import { Bitmap, newBitmap, freeBitmap } from std.amiga.video;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let title: String = stringOf("text");
    let screen: Screen? = openScreen(320, 256, 4, toCString(title));
    if (screen == null) {
        return 1;
    }
    let font: Font? = defaultFont();
    if (font == null) {
        return 2;
    }
    let text: u8[4] = [65, 66, 67, 0];
    let rom: Font? = openFont(toCString(stringOf("topaz.font")), 8, 0);
    if (rom != null) {
        let romWidth: i32 = textWidth(rom, slice(text, 0, 3));
        if (romWidth <= 0) {
            freeFont(rom);
            return 6;
        }
        freeFont(rom);
    }
    let disk: Font? = openDiskFont(toCString(stringOf("topaz.font")), 8, 0);
    if (disk != null) {
        freeFont(disk);
    }
    let width: i32 = textWidth(font, slice(text, 0, 3));
    let height: i32 = textHeight(font);
    drawText(screen, font, 10, 20, 1, slice(text, 0, 3));

    let bitmap: Bitmap? = newBitmap(240, 180, 4);
    if (bitmap == null) {
        return 3;
    }
    let layer: Layer? = newLayer(bitmap, 40, 80, 199, 143);
    if (layer == null) {
        return 4;
    }
    let began: bool = layerBeginUpdate(layer);
    layerClear(layer, 0);
    layerFill(layer, 0, 0, 159, 63, 1);
    layerText(layer, font, 4, 16, 2, slice(text, 0, 3));
    layerEndUpdate(layer);
    let moved: bool = layerMove(layer, 4, 2);
    let resized: bool = layerSize(layer, -4, -2);
    if (width < 0 || height < 0 || !began || !moved || !resized) {
        return 5;
    }
    freeLayer(layer);
    freeBitmap(bitmap);
    freeFont(font);
    return 0;
}
"#;

/// Every text/layer call sits above `ffi` and reaches the shim.
#[test]
fn every_text_call_sits_above_ffi() {
    let c = emit_hosted_amiga("routing", ROUTING_PROGRAM);
    for wrapper_call in [
        "cl_std_amiga_text_defaultFont(",
        "cl_std_amiga_text_openFont(",
        "cl_std_amiga_text_openDiskFont(",
        "cl_std_amiga_text_textWidth(font,",
        "cl_std_amiga_text_textHeight(font)",
        "cl_std_amiga_text_drawText(screen,",
        "cl_std_amiga_layers_newLayer(bitmap,",
        "cl_std_amiga_layers_layerFill(layer,",
        "cl_std_amiga_layers_layerClear(layer,",
        "cl_std_amiga_layers_layerText(layer,",
        "cl_std_amiga_layers_layerMove(layer,",
        "cl_std_amiga_layers_layerSize(layer,",
        "cl_std_amiga_layers_layerBeginUpdate(layer)",
        "cl_std_amiga_layers_layerEndUpdate(layer)",
    ] {
        assert!(
            c.contains(wrapper_call),
            "the application must call the wrapper `{wrapper_call}`"
        );
    }
    for shim_call in [
        "cl_font_default(",
        "cl_font_open(",
        "cl_font_open_disk(",
        "cl_text_width(font,",
        "cl_text_height(font)",
        "cl_text_draw(screen,",
        "cl_layer_new(bitmap,",
        "cl_layer_fill(layer,",
        "cl_layer_clear(layer,",
        "cl_layer_text(layer,",
        "cl_layer_move(layer,",
        "cl_layer_size(layer,",
        "cl_layer_begin_update(layer)",
        "cl_layer_end_update(layer)",
    ] {
        assert!(
            c.contains(shim_call),
            "a wrapper body must call the shim `{shim_call}`"
        );
    }
}

/// A program that relies on generated drops for every owned handle.
const DROP_PROGRAM: &str = r#"module impl.text.drop;

import { Screen, openScreen } from std.amiga.intuition;
import { Font, defaultFont } from std.amiga.text;
import { Layer, newLayer } from std.amiga.layers;
import { Bitmap, newBitmap } from std.amiga.video;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let title: String = stringOf("text");
    let screen: Screen? = openScreen(320, 256, 4, toCString(title));
    if (screen == null) {
        return 1;
    }
    let font: Font? = defaultFont();
    if (font == null) {
        return 2;
    }
    let bitmap: Bitmap? = newBitmap(240, 180, 4);
    if (bitmap == null) {
        return 3;
    }
    let layer: Layer? = newLayer(bitmap, 40, 80, 199, 143);
    if (layer == null) {
        return 4;
    }
    return 0;
}
"#;

/// Every owned handle drops behind a null guard in reverse declaration order.
#[test]
fn every_text_drop_is_null_guarded() {
    let c = emit_hosted_amiga("drop", DROP_PROGRAM);
    let start = c
        .find("cl_impl_text_drop_main(void)\n{")
        .expect("the emitted C must contain the program entry definition");
    let body = &c[start..];
    let end = body
        .find("\n}\n")
        .map(|offset| offset + 3)
        .unwrap_or(body.len());
    let body = &body[..end];
    for (call, guarded) in [
        (
            "cl_layer_free(layer)",
            "if (layer) { cl_layer_free(layer); }",
        ),
        (
            "cl_video_bitmap_free(bitmap)",
            "if (bitmap) { cl_video_bitmap_free(bitmap); }",
        ),
        (
            "cl_font_free(font)",
            "if (font) { cl_font_free(font); }",
        ),
        (
            "cl_close_screen(screen)",
            "if (screen) { cl_close_screen(screen); }",
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
    let position = |needle: &str| {
        body.rfind(needle)
            .unwrap_or_else(|| panic!("missing cleanup `{needle}` in main"))
    };
    let layer = position("if (layer) { cl_layer_free(layer); }");
    let bitmap = position("if (bitmap) { cl_video_bitmap_free(bitmap); }");
    let font = position("if (font) { cl_font_free(font); }");
    let screen = position("if (screen) { cl_close_screen(screen); }");
    assert!(
        layer < bitmap && bitmap < font && font < screen,
        "the generated cleanup must run layer, bitmap, font, screen"
    );
}

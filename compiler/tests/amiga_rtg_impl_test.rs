//! Implementation pins for the Picasso96 RTG surface.
//!
//! The suite pins the native P96 calls, the embedded header placement, the
//! ownership/drop shapes, the bounds guards, and the bitmap-fill workaround.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-rtg-impl-{}-{tag}-{sequence}",
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

fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

fn emit(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
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

fn assert_contains(tag: &str, haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "{tag}: emitted C does not contain {needle:?}"
    );
}

fn shim() -> String {
    format!(
        "{}{}",
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/shim_hosted_amiga.c")
        )
        .expect("the shared shim is readable"),
        clc::profile::Profile::HostedAmiga.shim_suffix()
    )
}

const DROP_SOURCE: &str = r#"module tests.rtg.impl;

import { RtgScreen, RtgBitmap, openRtgScreen, allocRtgBitmap }
    from std.amiga.picasso96;

func main(): i32 {
    let screenCreated: RtgScreen? = openRtgScreen(64, 64, 32, "rtg");
    if (screenCreated == null) { return 1; }
    let screen: RtgScreen = screenCreated;
    let bitmapCreated: RtgBitmap? = allocRtgBitmap(8, 8, 32);
    if (bitmapCreated == null) { return 2; }
    let bitmap: RtgBitmap = bitmapCreated;
    return 0;
}
"#;

#[test]
fn the_rtg_shim_pins_the_native_p96_calls() {
    let shim = shim();
    for needle in [
        "OpenLibrary((STRPTR)\"Picasso96API.library\", 2)",
        "CL_P96_RD_NumberOfBoards",
        "p96GetRTGDataTagList",
        "CL_P96_BIDTAG_NominalWidth",
        "CL_P96_BIDTAG_NominalHeight",
        "CL_P96_BIDTAG_Depth",
        "CL_P96_RGBFF_TRUEALPHA",
        "CL_P96_SA_DisplayID",
        "CL_P96_SA_Title",
        "CL_P96_SA_DClip",
        "CL_P96_SA_Quiet",
        "CL_P96_SA_ShowTitle",
        "graphics.library",
        "p96OpenScreenTagList",
        "p96CloseScreen",
        "p96AllocBitMap",
        "CL_P96_BMF_USERPRIVATE",
        "p96FreeBitMap",
        "p96GetBitMapAttr",
        "CL_P96_BMA_BITSPERPIXEL",
        "p96GetModeIDAttr",
        "p96WritePixel",
        "p96RectFill",
        "p96ReadPixelArray",
        "cl_rtg_read_pixel_rp",
        "handle->title",
        "FreeMem(handle->title",
        "if (!p96CloseScreen(handle->screen)) {",
        "CL_P96_BMF_USERPRIVATE",
        "BltBitMapRastPort",
        "0xC0",
    ] {
        assert_contains("rtg-shim", &shim, needle);
    }
}

#[test]
fn the_rtg_header_is_embedded_before_the_shim() {
    let shim = shim();
    assert_contains("rtg-header", &shim, "CLORRAINE_PICASSO96_H");
    assert_contains("rtg-header", &shim, "CL_P96_SA_DisplayID");
    let header = shim
        .find("CLORRAINE_PICASSO96_H")
        .expect("the embedded header");
    let body = shim
        .find("cl_rtg_boards")
        .expect("the RTG shim body");
    assert!(header < body, "the header must precede the RTG shim");
    assert!(
        !shim.contains("#include \"picasso96.h\""),
        "the header must be embedded, not included from a build path"
    );
}

#[test]
fn the_rtg_suffix_is_hosted_amiga_only() {
    let suffix = clc::profile::Profile::HostedAmiga.shim_suffix();
    assert!(suffix.contains("cl_rtg_boards"));
    assert!(clc::profile::Profile::HostedAmiga13.shim_suffix().is_empty());
    assert!(clc::profile::Profile::Host.shim_suffix().is_empty());
    assert!(clc::profile::Profile::Freestanding.shim_suffix().is_empty());
}

#[test]
fn the_rtg_handles_drop_exactly_once() {
    let c = emit("rtg-drop", "hosted-amiga", DROP_SOURCE);
    for cleanup in [
        "if (screen) { cl_rtg_close_screen(screen); }\n    screen = 0;",
        "if (bitmap) { cl_rtg_free_bitmap(bitmap); }\n    bitmap = 0;",
    ] {
        assert_contains("rtg-drop", &c, cleanup);
    }
}

#[test]
fn the_rtg_readback_and_fill_are_bounded() {
    let shim = shim();
    for needle in [
        "if (length < need)",
        "width > 4096",
        "info.BytesPerRow = (WORD)(width * 3);",
        "area > 1048576UL",
        "for (y = y0; y <= y1; y++)",
        "for (x = x0; x <= x1; x++)",
        "cl_rtg_valid_rect",
        "target_width > 4096",
        "target_height > 4096",
    ] {
        assert_contains("rtg-bounds", &shim, needle);
    }
}

#[test]
fn the_rtg_missing_library_guard_returns_zero() {
    let shim = shim();
    for needle in [
        "if (!cl_rtg_open()) {",
        "return 0;",
        "P96Base == NULL",
        "cl_rtg_open_screen",
    ] {
        assert_contains("rtg-guard", &shim, needle);
    }
}

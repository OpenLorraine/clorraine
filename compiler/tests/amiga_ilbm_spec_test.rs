//! IFF/ILBM parser specification tests.
//!
//! Bundle 5 adds the pure `std.iff` module. These cases derive from the
//! requirements, not from the implementation: a host program builds a real IFF
//! byte stream, parses it, decodes the planes, and asserts every pixel. The
//! parser is pure Clorraine, so the same code that runs on the A500 also runs
//! here on the development host.
//!
//! The red phase is the unresolved-module compile failure while `std.iff` is
//! still absent. The host cases run under AddressSanitizer when `RDC_ASAN` is
//! set, so `make asan` exercises the malformed-input paths.

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
            "clc-ilbm-{}-{tag}-{sequence}",
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

/// True when a tool is available on PATH.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// Writes `source` to a scratch file and compiles it for `profile`.
fn compile_source(tag: &str, profile: &str, source: &str) -> (Output, bool) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let emitted = output_path.exists();
    (output, emitted)
}

/// Compiles a host program, builds the emitted C, and runs it.
///
/// When `RDC_ASAN` is set the C build and link add AddressSanitizer, so the
/// same cases cover the parser's bounds paths under the sanitizer.
fn build_and_run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let c_path = scratch.path().join("out.c");
    let compile = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{tag}: the host profile rejected the parser program; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty"
    );

    let program = scratch.path().join("prog");
    let mut cc_args = vec![
        "-std=c99".to_owned(),
        "-O0".to_owned(),
        "-g".to_owned(),
        "-Wall".to_owned(),
        "-Wextra".to_owned(),
        "-Werror".to_owned(),
    ];
    if std::env::var_os("RDC_ASAN").is_some() {
        cc_args.push("-fsanitize=address".to_owned());
        cc_args.push("-fno-omit-frame-pointer".to_owned());
    }
    cc_args.push("-o".to_owned());
    cc_args.push(program.display().to_string());
    cc_args.push(c_path.display().to_string());
    let built = Command::new("cc")
        .args(&cc_args)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let mut command = Command::new(&program);
    if std::env::var_os("RDC_ASAN").is_some() {
        command.env("ASAN_OPTIONS", "detect_leaks=1");
    }
    command
        .output()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the parser program: {err}"))
}

/// Asserts the run wrote exactly `expected` to stdout and exited zero.
fn assert_stdout(tag: &str, output: &Output, expected: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: expected a clean exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "{tag}: unexpected standard output"
    );
}

/// Formats bytes as a Clorraine `u8[N]` initializer.
fn array_literal(bytes: &[u8]) -> String {
    let values: Vec<String> = bytes.iter().map(|b| b.to_string()).collect();
    format!("[{}]", values.join(", "))
}

/// Big-endian 16-bit bytes.
fn be16(value: u16) -> [u8; 2] {
    [(value >> 8) as u8, value as u8]
}

/// Big-endian 32-bit bytes.
fn be32(value: u32) -> [u8; 4] {
    [
        (value >> 24) as u8,
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ]
}

/// One IFF chunk with its even-byte padding.
fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(id);
    out.extend_from_slice(&be32(payload.len() as u32));
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
    out
}

/// One `FORM ILBM` around the given chunks.
fn form_ilbm(chunks: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"FORM");
    out.extend_from_slice(&be32((4 + chunks.len()) as u32));
    out.extend_from_slice(b"ILBM");
    out.extend_from_slice(chunks);
    out
}

/// A 20-byte `BMHD` chunk payload.
fn bmhd(width: u16, height: u16, depth: u8, compression: u8) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&be16(width));
    out.extend_from_slice(&be16(height));
    out.extend_from_slice(&be16(0));
    out.extend_from_slice(&be16(0));
    out.push(depth);
    out.push(0);
    out.push(compression);
    out.push(0);
    out.extend_from_slice(&be16(0));
    out.push(10);
    out.push(11);
    out.extend_from_slice(&be16(width));
    out.extend_from_slice(&be16(height));
    out
}

/// A `CMAP` payload from RGB triplets.
fn cmap(colors: &[[u8; 3]]) -> Vec<u8> {
    let mut out = Vec::new();
    for color in colors {
        out.extend_from_slice(color);
    }
    out
}

/// Row bytes for a word-aligned ILBM row.
fn row_bytes(width: usize) -> usize {
    ((width + 15) / 16) * 2
}

/// Packs one plane row from a pixel pattern.
fn plane_row(pixels: &[u8], width: usize, y: usize, plane: u8) -> Vec<u8> {
    let mut row = vec![0u8; row_bytes(width)];
    let mut x = 0;
    while x < width {
        let pixel = pixels[y * width + x];
        if pixel & (1 << plane) != 0 {
            row[x / 8] |= 0x80 >> (x % 8);
        }
        x += 1;
    }
    row
}

/// Builds an uncompressed, row-interleaved `BODY` for a pixel pattern.
fn body_uncompressed(pixels: &[u8], width: usize, height: usize, depth: u8) -> Vec<u8> {
    let mut out = Vec::new();
    let mut y = 0;
    while y < height {
        let mut plane = 0;
        while plane < depth {
            out.extend_from_slice(&plane_row(pixels, width, y, plane));
            plane += 1;
        }
        y += 1;
    }
    out
}

/// ByteRun1-encodes one row, emitting repeat runs of three or more.
fn rle_row(row: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < row.len() {
        let mut run = 1;
        while at + run < row.len() && row[at + run] == row[at] {
            run += 1;
        }
        if run >= 3 {
            out.push((257 - run) as u8);
            out.push(row[at]);
            at += run;
        } else {
            let start = at;
            at += run;
            while at < row.len() {
                let mut next = 1;
                while at + next < row.len() && row[at + next] == row[at] {
                    next += 1;
                }
                if next >= 3 || at - start >= 128 {
                    break;
                }
                at += next;
            }
            let count = at - start;
            out.push((count - 1) as u8);
            out.extend_from_slice(&row[start..at]);
        }
    }
    out
}

/// Builds a ByteRun1 `BODY` for a pixel pattern.
fn body_rle(pixels: &[u8], width: usize, height: usize, depth: u8) -> Vec<u8> {
    let mut out = Vec::new();
    let mut y = 0;
    while y < height {
        let mut plane = 0;
        while plane < depth {
            out.extend_from_slice(&rle_row(&plane_row(pixels, width, y, plane)));
            plane += 1;
        }
        y += 1;
    }
    out
}

/// FNV-1a over the decoded pixels.
fn pixel_hash(pixels: &[u8]) -> u32 {
    let mut hash: u32 = 2166136261;
    for byte in pixels {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// Builds a full ILBM from a pixel pattern.
fn ilbm(
    width: u16,
    height: u16,
    depth: u8,
    compression: u8,
    colors: &[[u8; 3]],
    pixels: &[u8],
) -> Vec<u8> {
    let body = if compression == 1 {
        body_rle(pixels, width as usize, height as usize, depth)
    } else {
        body_uncompressed(pixels, width as usize, height as usize, depth)
    };
    let mut chunks = Vec::new();
    chunks.extend_from_slice(&chunk(b"BMHD", &bmhd(width, height, depth, compression)));
    chunks.extend_from_slice(&chunk(b"CMAP", &cmap(colors)));
    chunks.extend_from_slice(&chunk(b"BODY", &body));
    form_ilbm(&chunks)
}

/// A program that decodes `data` and checks it against `expected`.
fn decode_program(data: &[u8], expected: &[u8], width: i32, height: i32, depth: u8, colors: u16) -> String {
    format!(
        r#"module tests.ilbm.decode;

import {{ IlbmInfo, ilbmInfo, ilbmDecode, ilbmPalette, ilbmPixelHash }} from std.iff;
import {{ slice }} from std.mem;
import {{ println }} from std.io;

func main(): i32 {{
    let data: u8[{data_len}] = {data};
    let expected: u8[{expected_len}] = {expected};
    let info: IlbmInfo = ilbmInfo(slice(data, 0, {data_len}));
    if (!info.ok) {{
        println("bad header");
        return 1;
    }}
    if (info.width != {width} || info.height != {height} || info.depth != {depth}) {{
        println("bad geometry");
        return 2;
    }}
    let pixels: u8[{expected_len}] = [; 255];
    if (!ilbmDecode(slice(data, 0, {data_len}), info, borrow slice(pixels, 0, {expected_len}))) {{
        println("decode failed");
        return 3;
    }}
    let palette: u8[48] = [; 0];
    let copied: u16 = ilbmPalette(slice(data, 0, {data_len}), info, borrow slice(palette, 0, 48));
    if (copied != {colors}) {{
        println("bad colors");
        return 4;
    }}
    let i: usize = 0;
    while (i < {expected_len}) {{
        if (pixels[i] != expected[i]) {{
            println("pixel mismatch");
            return 5;
        }}
        i = i + 1;
    }}
    println($"ok {width} {height} {depth} {colors} {{ilbmPixelHash(slice(pixels, 0, {expected_len}))}}");
    return 0;
}}
"#,
        data_len = data.len(),
        data = array_literal(data),
        expected_len = expected.len(),
        expected = array_literal(expected),
        width = width,
        height = height,
        depth = depth,
        colors = colors,
    )
}

/// A two-plane 8x4 image with every combination of plane bits.
#[test]
fn an_uncompressed_ilbm_decodes_every_pixel() {
    let width = 8usize;
    let height = 4usize;
    let depth = 2u8;
    let mut pixels = Vec::new();
    let mut y = 0;
    while y < height {
        let mut x = 0;
        while x < width {
            pixels.push(((x + y) & 3) as u8);
            x += 1;
        }
        y += 1;
    }
    let colors = [[0u8, 0, 0], [255, 255, 255], [255, 0, 0], [0, 255, 0]];
    let data = ilbm(width as u16, height as u16, depth, 0, &colors, &pixels);
    let source = decode_program(&data, &pixels, width as i32, height as i32, depth, 4);
    let output = build_and_run_host("uncompressed", &source);
    let expected = format!(
        "ok {} {} {} 4 {}\n",
        width,
        height,
        depth,
        pixel_hash(&pixels)
    );
    assert_stdout("uncompressed", &output, &expected);
}

/// A four-plane 16x8 image compressed with ByteRun1.
#[test]
fn a_byterun1_ilbm_decodes_every_pixel_and_pins_the_hash() {
    let width = 16usize;
    let height = 8usize;
    let depth = 4u8;
    let mut pixels = Vec::new();
    let mut y = 0;
    while y < height {
        let mut x = 0;
        while x < width {
            pixels.push(((x * 3 + y * 5) & 15) as u8);
            x += 1;
        }
        y += 1;
    }
    let colors: Vec<[u8; 3]> = (0..16)
        .map(|i| [(i * 16) as u8, (255 - i * 16) as u8, (i * 8) as u8])
        .collect();
    let data = ilbm(width as u16, height as u16, depth, 1, &colors, &pixels);
    let source = decode_program(&data, &pixels, width as i32, height as i32, depth, 16);
    let output = build_and_run_host("byterun1", &source);
    let expected = format!(
        "ok {} {} {} 16 {}\n",
        width,
        height,
        depth,
        pixel_hash(&pixels)
    );
    assert_stdout("byterun1", &output, &expected);
}

/// Odd widths use word padding; the decoder ignores the pad bits.
#[test]
fn an_odd_width_ilbm_ignores_row_padding() {
    let width = 5usize;
    let height = 3usize;
    let depth = 1u8;
    let mut pixels = Vec::new();
    let mut y = 0;
    while y < height {
        let mut x = 0;
        while x < width {
            pixels.push(((x + y) & 1) as u8);
            x += 1;
        }
        y += 1;
    }
    let colors = [[0u8, 0, 0], [255, 255, 255]];
    let data = ilbm(width as u16, height as u16, depth, 0, &colors, &pixels);
    let source = decode_program(&data, &pixels, width as i32, height as i32, depth, 2);
    let output = build_and_run_host("odd-width", &source);
    let expected = format!(
        "ok {} {} {} 2 {}\n",
        width,
        height,
        depth,
        pixel_hash(&pixels)
    );
    assert_stdout("odd-width", &output, &expected);
}

/// The palette copies whole RGB triplets only.
#[test]
fn the_palette_copies_whole_triplets_only() {
    let colors = [
        [1u8, 2, 3],
        [4, 5, 6],
        [7, 8, 9],
    ];
    let mut chunks = Vec::new();
    chunks.extend_from_slice(&chunk(b"BMHD", &bmhd(1, 1, 1, 0)));
    chunks.extend_from_slice(&chunk(b"CMAP", &cmap(&colors)));
    chunks.extend_from_slice(&chunk(b"BODY", &[0]));
    let data = form_ilbm(&chunks);
    let source = format!(
        r#"module tests.ilbm.palette;

import {{ IlbmInfo, ilbmInfo, ilbmPalette }} from std.iff;
import {{ slice }} from std.mem;
import {{ println }} from std.io;

func main(): i32 {{
    let data: u8[{data_len}] = {data};
    let info: IlbmInfo = ilbmInfo(slice(data, 0, {data_len}));
    if (!info.ok) {{
        println("bad header");
        return 1;
    }}
    let six: u8[6] = [; 0];
    let seven: u8[7] = [; 0];
    let none: u8[6] = [; 0];
    let a: u16 = ilbmPalette(slice(data, 0, {data_len}), info, borrow slice(six, 0, 6));
    let b: u16 = ilbmPalette(slice(data, 0, {data_len}), info, borrow slice(seven, 0, 7));
    let empty: IlbmInfo = IlbmInfo {{ ok: false, width: 0, height: 0, depth: 0, compression: 0, colors: 0, cmap_start: 0, body_start: 0, body_len: 0 }};
    let c: u16 = ilbmPalette(slice(data, 0, {data_len}), empty, borrow slice(none, 0, 6));
    println($"palette {{a}} {{b}} {{c}} {{six[0]}} {{six[1]}} {{six[2]}} {{six[3]}} {{six[4]}} {{six[5]}} {{seven[6]}}");
    return 0;
}}
"#,
        data_len = data.len(),
        data = array_literal(&data),
    );
    let output = build_and_run_host("palette", &source);
    assert_stdout("palette", &output, "palette 2 2 0 1 2 3 4 5 6 0\n");
}

/// Malformed headers and bodies return false instead of trapping.
#[test]
fn malformed_ilbm_is_rejected_without_trapping() {
    let good_pixels = [0u8; 32];
    let colors = [[0u8, 0, 0], [255, 255, 255]];
    let good = ilbm(8, 4, 1, 0, &colors, &good_pixels);

    let mut wrong_form = good.clone();
    wrong_form[0] = b'X';
    wrong_form[1] = b'X';
    wrong_form[2] = b'X';
    wrong_form[3] = b'X';

    let mut wrong_type = good.clone();
    wrong_type[8] = b'I';
    wrong_type[9] = b'L';
    wrong_type[10] = b'B';
    wrong_type[11] = b'X';

    let mut missing_body = Vec::new();
    missing_body.extend_from_slice(&chunk(b"BMHD", &bmhd(1, 1, 1, 0)));
    let missing_body = form_ilbm(&missing_body);

    let mut zero_body_chunks = Vec::new();
    zero_body_chunks.extend_from_slice(&chunk(b"BMHD", &bmhd(1, 1, 1, 0)));
    zero_body_chunks.extend_from_slice(&chunk(b"BODY", &[]));
    let zero_body = form_ilbm(&zero_body_chunks);

    let mut twice_chunks = Vec::new();
    twice_chunks.extend_from_slice(&chunk(b"BMHD", &bmhd(1, 1, 1, 0)));
    twice_chunks.extend_from_slice(&chunk(b"BODY", &[0]));
    twice_chunks.extend_from_slice(&chunk(b"BODY", &[0]));
    let twice = form_ilbm(&twice_chunks);

    let mut wrapped_form = good.clone();
    wrapped_form[4..8].copy_from_slice(&be32(u32::MAX));

    let mut wrapped_size = good.clone();
    wrapped_size[16..20].copy_from_slice(&be32(u32::MAX));

    let compressed_two = ilbm(1, 1, 1, 2, &colors, &good_pixels);
    let depth_zero = ilbm(1, 1, 0, 0, &colors, &good_pixels);

    let cases = [
        ("wrong-form", wrong_form),
        ("wrong-type", wrong_type),
        ("missing-body", missing_body),
        ("zero-body", zero_body),
        ("twice-body", twice),
        ("wrapped-form", wrapped_form),
        ("wrapped-size", wrapped_size),
        ("compression-two", compressed_two),
        ("depth-zero", depth_zero),
    ];

    let mut sources = String::new();
    for (index, (_name, data)) in cases.iter().enumerate() {
        sources.push_str(&format!(
            "    let d{index}: u8[{len}] = {data};\n    let info{index}: IlbmInfo = ilbmInfo(slice(d{index}, 0, {len}));\n    println($\"{index} {{info{index}.ok}}\");\n",
            index = index,
            len = data.len(),
            data = array_literal(data),
        ));
    }

    let source = format!(
        r#"module tests.ilbm.malformed;

import {{ IlbmInfo, ilbmInfo, ilbmDecode }} from std.iff;
import {{ slice }} from std.mem;
import {{ println }} from std.io;

func main(): i32 {{
{sources}    let good: u8[{good_len}] = {good};
    let info: IlbmInfo = ilbmInfo(slice(good, 0, {good_len}));
    let short: u8[1] = [; 0];
    let decoded_short: bool = ilbmDecode(slice(good, 0, {good_len}), info, borrow slice(short, 0, 1));
    println($"short {{decoded_short}}");
    return 0;
}}
"#,
        sources = sources,
        good_len = good.len(),
        good = array_literal(&good),
    );
    let output = build_and_run_host("malformed", &source);
    let expected = "0 false\n1 false\n2 false\n3 false\n4 false\n5 false\n6 false\n7 false\n8 false\nshort false\n";
    assert_stdout("malformed", &output, expected);
}

/// The host profile rejects the Amiga-only DOS and ILBM modules.
#[test]
fn the_host_profile_rejects_the_amiga_only_modules() {
    let cases = [
        (
            "host-dos",
            r#"module tests.host.dos;
import { delayTicks } from std.amiga.dos;
func main(): i32 { delayTicks(u32(1)); return 0; }
"#,
        ),
    ];
    for (tag, source) in cases {
        let scratch = ScratchDir::new(tag);
        let source_path = scratch.path().join("main.clo");
        std::fs::write(&source_path, source).expect("write the snippet");
        let output_path = scratch.path().join("out.c");
        let output = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            source_path.display().to_string(),
        ]);
        assert_eq!(output.status.code(), Some(1), "{tag}: host must reject the module");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("E11200"), "{tag}: expected E11200, got:\n{stderr}");
        assert!(!output_path.exists(), "{tag}: rejected source must not emit C");
    }
}

/// The host compiler is available when the suite runs in the SDK image.
#[test]
fn the_host_c_compiler_is_available() {
    assert!(tool_available("cc"), "cc is required for the parser cases");
}

/// The loader surface used by the viewer.
const LOADER_SURFACE: &str = r#"module tests.ilbm.loader;

import { loadIlbm, IlbmImage } from std.amiga.ilbm;
import { Bitmap } from std.amiga.video;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;
import { println } from std.io;

func useBitmap(borrow bitmap: Bitmap): i32 {
    return 1;
}

func main(): i32 {
    let palette: u8[48] = [; 0];
    let image: IlbmImage = loadIlbm(
        toCString(stringOf("DF0:image.ilbm")), borrow slice(palette, 0, 48));
    if (!image.ok) {
        println("load failed");
        return 1;
    }
    if (image.bitmap == null) {
        println("no bitmap");
        return 2;
    }
    let used: i32 = useBitmap(image.bitmap);
    println($"loader {image.width} {image.height} {image.depth} {image.colors} {image.hash} {used}");
    return 0;
}
"#;

/// Both hosted-Amiga profiles accept the loader surface.
#[test]
fn the_loader_surface_compiles_on_both_hosted_profiles() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let (output, emitted) = compile_source("loader-surface", profile, LOADER_SURFACE);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{profile}: the loader surface must compile; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(emitted, "{profile}: the loader surface must emit C");
    }
}

/// The loader surface cross-builds with the 1.3 runtime.
#[test]
fn the_loader_surface_cross_builds_with_nix13() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    cross_build_loader("loader-nix13", "hosted-amiga-13", "-mcrt=nix13");
}

/// The loader surface cross-builds with the 3.1 runtime.
#[test]
fn the_loader_surface_cross_builds_with_nix20() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    cross_build_loader("loader-nix20", "hosted-amiga", "-mcrt=nix20");
}

/// Emits the loader surface and builds it with the Amiga toolchain.
fn cross_build_loader(tag: &str, profile: &str, runtime: &str) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, LOADER_SURFACE).expect("write the snippet");
    let c_path = scratch.path().join("out.c");
    let emit = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emit.status.code(),
        Some(0),
        "{tag}: the loader surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("loader");
    let build = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            runtime,
            "-m68000",
            "-fwrapv",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
        ])
        .arg(&binary)
        .arg(&c_path)
        .args(["-lamiga", "-lm"])
        .output()
        .expect("run m68k-amigaos-gcc");
    assert!(
        build.status.success(),
        "{tag}: the loader surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The host profile rejects the Amiga-only ILBM module with E11200.
#[test]
fn the_host_profile_rejects_the_ilbm_module() {
    let source = r#"module tests.host.ilbm;
import { loadIlbm } from std.amiga.ilbm;
func main(): i32 { return 0; }
"#;
    let (output, emitted) = compile_source("host-ilbm", "host", source);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E11200"), "expected E11200, got:\n{stderr}");
    assert!(!emitted, "a rejected program must not emit C");
}

/// The committed fixture parses, decodes, and hashes to the generator's value.
#[test]
fn the_committed_fixture_decodes_to_the_pinned_hash() {
    let fixture = std::fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../examples/amiga/assets/clorraine.ilbm"),
    )
    .expect("the committed fixture must exist");
    let path = fixture.display().to_string().replace('\\', "\\\\");
    let source = format!(
        r#"module tests.ilbm.fixture;

import {{ File, openFile, readFile, MODE_OLDFILE }} from std.io;
import {{ Buffer, bufferNew, bufferBytes, slice }} from std.mem;
import {{ IlbmInfo, ilbmInfo, ilbmDecode, ilbmPalette, ilbmPixelHash }} from std.iff;
import {{ println }} from std.io;

func main(): i32 {{
    let file: File? = openFile("{path}", MODE_OLDFILE);
    if (file == null) {{
        println("open failed");
        return 1;
    }}
    let data: Buffer? = bufferNew(131072);
    if (data == null) {{
        return 2;
    }}
    let read: i32 = 0;
    while (true) {{
        let moved: i32 = readFile(
            file, borrow slice(bufferBytes(data), usize(read), 131072));
        if (moved <= 0) {{
            break;
        }}
        read = read + moved;
    }}
    let info: IlbmInfo = ilbmInfo(slice(bufferBytes(data), 0, usize(read)));
    if (!info.ok) {{
        println("bad header");
        return 3;
    }}
    let pixels: u8[64000] = [; 0];
    if (!ilbmDecode(
        slice(bufferBytes(data), 0, usize(read)), info,
        borrow slice(pixels, 0, 64000))) {{
        println("decode failed");
        return 4;
    }}
    let palette: u8[48] = [; 0];
    let colors: u16 = ilbmPalette(
        slice(bufferBytes(data), 0, usize(read)), info,
        borrow slice(palette, 0, 48));
    println($"fixture {{info.width}} {{info.height}} {{info.depth}} {{colors}} {{ilbmPixelHash(slice(pixels, 0, 64000))}}");
    return 0;
}}
"#,
        path = path
    );
    let output = build_and_run_host("fixture", &source);
    assert_stdout("fixture", &output, "fixture 320 200 4 16 603248125\n");
}

/// Decoder-only failures: truncated body, bad literal/repeat counts, and a
/// trailing no-op that never produces a row.
#[test]
fn the_decoder_rejects_truncated_and_bad_rle_streams() {
    let colors = [[0u8, 0, 0], [255, 255, 255]];
    let mut streams: Vec<(&str, Vec<u8>)> = Vec::new();

    let mut truncated_chunks = Vec::new();
    truncated_chunks.extend_from_slice(&chunk(b"BMHD", &bmhd(1, 1, 1, 0)));
    truncated_chunks.extend_from_slice(&chunk(b"CMAP", &cmap(&colors)));
    truncated_chunks.extend_from_slice(&chunk(b"BODY", &[0]));
    streams.push(("truncated", form_ilbm(&truncated_chunks)));

    let mut literal_chunks = Vec::new();
    literal_chunks.extend_from_slice(&chunk(b"BMHD", &bmhd(1, 1, 1, 1)));
    literal_chunks.extend_from_slice(&chunk(b"CMAP", &cmap(&colors)));
    literal_chunks.extend_from_slice(&chunk(b"BODY", &[127, 0]));
    streams.push(("bad-literal", form_ilbm(&literal_chunks)));

    let mut repeat_chunks = Vec::new();
    repeat_chunks.extend_from_slice(&chunk(b"BMHD", &bmhd(1, 1, 1, 1)));
    repeat_chunks.extend_from_slice(&chunk(b"CMAP", &cmap(&colors)));
    repeat_chunks.extend_from_slice(&chunk(b"BODY", &[0x81]));
    streams.push(("bad-repeat", form_ilbm(&repeat_chunks)));

    let mut noop_chunks = Vec::new();
    noop_chunks.extend_from_slice(&chunk(b"BMHD", &bmhd(1, 1, 1, 1)));
    noop_chunks.extend_from_slice(&chunk(b"CMAP", &cmap(&colors)));
    noop_chunks.extend_from_slice(&chunk(b"BODY", &[0x80]));
    streams.push(("noop-end", form_ilbm(&noop_chunks)));

    let mut declarations = String::new();
    let mut calls = String::new();
    for (index, (name, data)) in streams.iter().enumerate() {
        declarations.push_str(&format!(
            "    let d{index}: u8[{}] = {};\n",
            data.len(),
            array_literal(data)
        ));
        calls.push_str(&format!(
            "    let i{index}: IlbmInfo = ilbmInfo(slice(d{index}, 0, {}));\n    let p{index}: u8[1] = [; 0];\n    let ok{index}: bool = ilbmDecode(slice(d{index}, 0, {}), i{index}, borrow slice(p{index}, 0, 1));\n    println($\"{name} {{i{index}.ok}} {{ok{index}}}\");\n",
            data.len(),
            data.len(),
            name = name,
        ));
    }
    let source = format!(
        r#"module tests.ilbm.decoder;

import {{ IlbmInfo, ilbmInfo, ilbmDecode }} from std.iff;
import {{ slice }} from std.mem;
import {{ println }} from std.io;

func main(): i32 {{
{declarations}{calls}    return 0;
}}
"#,
        declarations = declarations,
        calls = calls,
    );
    let output = build_and_run_host("decoder-failures", &source);
    assert_stdout(
        "decoder-failures",
        &output,
        "truncated true false\nbad-literal true false\nbad-repeat true false\nnoop-end true false\n",
    );
}

/// The loader caps the display geometry, depth, and file size before allocating.
#[test]
fn the_loader_pins_the_resource_caps() {
    let source = include_str!("../../stdlib/std.amiga.ilbm.clo");
    for needle in [
        "size > 131072",
        "info.width > 320",
        "info.height > 256",
        "info.depth > 5",
    ] {
        assert!(
            source.contains(needle),
            "the loader must reject {needle}"
        );
    }
}

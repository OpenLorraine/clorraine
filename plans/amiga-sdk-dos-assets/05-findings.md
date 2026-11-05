# Findings: Amiga SDK Bundle 5 — DOS and Assets

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done

## Implementation findings

| # | Finding | Evidence | Disposition |
|---|---|---|---|
| F-1 | Importing the `len` intrinsic from `std.mem` inside a standard-library module makes the compiler loop during module loading. | A minimal `std.iff` with `import { len } from std.mem;` never returned from `clc`; removing the import and using the intrinsic directly compiled immediately. | `std.iff` uses `len` directly. This is recorded as a compiler observation, not a language change; no stdlib module imports an intrinsic. |
| F-2 | Nullable structs are not part of the language, so `IlbmImage?` is rejected with `E10011`. | The first loader surface failed with `nullable type is not allowed on 'IlbmImage'`. | `IlbmImage` carries an `ok` gate and a `Bitmap?` field; callers check both. |
| F-3 | A caller-owned palette borrow plus a local `Buffer` view in one helper triggered `E10030` ("borrow of 'dataBuffer' does not live long enough"). | The loader's direct `ilbmPalette` call failed; staging the palette in a local `u8[768]` and copying to the caller after the data view ended compiled and cross-built. | The loader stages the palette locally, then copies `min(colors, palette.len/3)` triplets to the caller. |
| F-4 | Drawing a caption immediately after `newLayer` and then calling `BeginUpdate`/`EndUpdate` restored the layer's previous content and erased the text. | The A500 held frame showed the black layer box with no caption. | The viewer draws directly after creation, like the Bundle 4 scroll layer; the caption then appears. |
| F-5 | The debug viewer binary is about 280 KB; the ADF was 98% full and the demo did not reach the display within the poll budget. | Debug runs stayed on the Kickstart screen through 90 s; the release image is about 74 KB and reached the display at about 60 s. | `make sdk-a500-assets` builds `--release`; the smoke and appendix record the reason. |
| F-6 | The first report line was one character too long for the KS1.3 shell width and wrapped before the final `1`. | The shell showed `... dos 255 vol` and `1)` on the next line. | The line is `img: ok 320x200x4 hash <N> dos 255 vol 1`, which fits one line and the 620x22 crop. |
| F-7 | The fixture hash is independently reproducible. | The Python generator prints `hash 603248125`; the host parser test pins the same value; the A500 line prints it. | The generator is the independent oracle; the parser test and the target line agree. |
| F-8 | The hello gate's single 35-second capture was too early under load; the line arrives at about 50 seconds. | The captured screen was the Kickstart insert-disk screen at 35 seconds, while manual captures at 50/60 seconds matched the golden exactly. | The hello smoke now polls from 30 seconds in 5-second steps, like the other A500 gates; the golden is unchanged. |

## Red phase

| Gate | Red evidence |
|---|---|
| Parser | `error E11102: 'IlbmInfo' is not exported from module 'std.iff'` (6 failing cases before the module) |
| DOS | `error E11102: 'SEEK_SET'/'seekFile'/... is not exported from module 'std.amiga.dos'` (4 failing cases) |
| Loader | `error E11102: 'loadIlbm' is not exported from module 'std.amiga.ilbm'` and the host rejection case before registration |

## Green phase

| Gate | Result |
|---|---|
| `cargo test --test amiga_ilbm_spec_test` | 14 passed (uncompressed, ByteRun1, odd width, palette, malformed, fixture hash, loader surface, host rejection, `nix13`/`nix20` loader cross-builds) |
| `cargo test --test amiga_dos_spec_test` | 5 passed (full surface on both hosted profiles, short buffers, host rejection, `nix13`/`nix20` cross-builds) |
| `cargo test --test amiga_assets_impl_test` | 9 passed (parser purity/wrapper/untouched, DOS prototypes/drop order/name bound, fixture regeneration, loader primitives/drop) |
| `make sdk-a500-assets` | OK; held frame and report line matched |
| `make asan` | 25 suites passed; `amiga_ilbm_spec_test` ran its host parser cases under AddressSanitizer |
| `make check` | exit 0; spec check OK (27 files, 111 compile-fail tests), registry OK |
| `make verify` | exit 0; `verify: all legs passed` with hello, Boing, sprites, paddle, text, and assets A500 gates |

## Runtime evidence

| Evidence | Value |
|---|---|
| Held image frame | `docs/amiga-sdk/a500-assets.png` (414x410 crop) |
| Report line golden | `docs/amiga-sdk/a500-assets-line.png` (620x22 crop) |
| Report line | `img: ok 320x200x4 hash 603248125 dos 255 vol 1` |
| DOS mask | 255: file (1), path (2), rename (4), delete (8), dir (16), volume (32), lock (64), cwd (128) |
| Volume bit | 1: non-empty volume name, free > 0, used > 0 |
| Fixture | 320x200x4, 16 colors, ByteRun1, 11,532 bytes |
| Pixel hash | 603248125 (FNV-1a, independently generated) |
| Build | `--release`, 74,004 bytes |
| Machine | FS-UAE A500, real Kickstart 1.3 rev 34.5, decrypted licensed ROM |
| ROM SHA-1 | `891e9a547772fe0c6c19b610baf8bc4ea7fcb785` |

## Review ledger

The preflight audit (recorded in `00-preflight-report.md`) found one critical
and five major plan defects before implementation. All were fixed in the
design and re-tested:

| ID | Severity | Finding | Fix |
|---|---|---|---|
| PF-01 | Critical | Decode destination was not cleared. | `ilbmDecode` clears first; tests start at `0xFF`. |
| PF-02 | Major | Volume name fallback used a released lock and a zero volume node. | Lock held through `Info`/`Examine`; zero-node path skipped. |
| PF-03 | Major | Name copies were unbounded. | `min(length, capacity)` in every name copy; short-buffer tests. |
| PF-04 | Major | `fileLength == -1` reached a trapping allocator. | Loader rejects `<= 0` and `> 128 KB`; image capped at 320x256. |
| PF-05 | Major | FORM/chunk window checks could wrap. | Subtraction checks and wrapped-length malformed cases. |
| PF-06 | Major | The A500 hash was self-captured. | `ilbmPixelHash` pinned by the independent generator; the same value is printed on the A500. |

Target-only fixes after the first A500 runs: the caption update protocol
(F-4), the release build (F-5), and the report-line width (F-6).

Post-implementation review added the missing `FORM` check (C1), decoder
failure cases for truncated and bad RLE streams plus the loader resource pins
(T1), whole-triplet palette rounding (C2), consecutive held-frame stability
(T3), exact rename bytes and short-buffer runtime checks (T4/T5), and the
loader depth cap. The review found no critical or memory-safety defect.

## Accepted residuals

- `GfxBase`, `LayersBase`, and `DiskfontBase` remain process-lifetime bases.
- The ILBM subset is the common shape: 1..8 planes, 320x256 loader cap, no HAM,
  EHB, masking, multi-`BODY`, or animation.
- The `len`-import compiler hang (F-1) is recorded for a future compiler
  cleanup; no language or grammar change is made here.
- The A500 smoke is empirical about the poll budget; the release build reaches
  the display in about a minute and the 22x5 s window leaves margin.
- The pure parser host suite is the ASan target; the m68k shim is covered by
  the A500 runtime gates.

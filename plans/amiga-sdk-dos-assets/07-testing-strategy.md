# Testing Strategy: Amiga SDK Bundle 5

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)
> **Covers**: ST-1–ST-16

## Specification tests

Specification tests derive from the requirements, not from the implementation.
They are written first and must fail before the modules exist.

| ID | Case | Input → expected | Requirements |
|---|---|---|---|
| ST-1 | Uncompressed ILBM header and pixels | 8x4x2 `FORM ILBM`, `BMHD` compression 0, 4-color `CMAP`, raw body; destination pre-filled with `0xFF` → `ok`, width 8, height 4, depth 2, colors 4; every pixel overwritten and matches the source pattern | R6, R7, R8 |
| ST-2 | ByteRun1 ILBM | 16x8x4 body with literal and repeat runs; destination pre-filled with `0xFF` → decoded indices match; `colors == 16`; `ilbmPixelHash` equals the host-computed constant | R7, R8 |
| ST-3 | Odd width | 5x3x1 with word-padded rows → decoded 5 pixels per row, padding ignored | R7, AR-5 |
| ST-4 | Palette copy | `CMAP` with three known RGB triplets and a 6-byte buffer → returns 2 and copies two triplets; a 7-byte buffer → returns 2 and copies six bytes; a missing `CMAP` returns 0 | R8 |
| ST-5 | Malformed files | Wrong `FORM` type, missing `BODY`, zero-length `BODY`, second `BODY`, truncated chunk, wrapped FORM length, wrapped chunk size, compression 2, depth 0, short pixel buffer, bad RLE count → `ok == false` or `decode == false`; no trap | R6, R7, AR-6 |
| ST-6 | Host profile | A program importing `std.iff` compiles and runs on `host`; `std.amiga.dos` and `std.amiga.ilbm` are rejected with `E11200` | R6, R13 |
| ST-7 | DOS surface | One program calls every file/lock/path/directory/volume wrapper on `hosted-amiga` and `hosted-amiga-13`; both emit C and the host rejects the module; short name buffers are accepted | R1–R5 |
| ST-8 | Cross-build | The full DOS + ILBM surface builds with `m68k-amigaos-gcc` under `nix13`/`-m68000` and `nix20`, with `-Wall -Wextra -Werror` | R1–R9 |
| ST-9 | Fixture | `clorraine.ilbm` has `FORM ILBM`, `BMHD` 320x200x4 compression 1, a 16-entry `CMAP`, and a `BODY`; the parser reports `ok`, the decoded hash matches the independent reference, and the generator reproduces the bytes | R11 |
| ST-10 | A500 DOS | The viewer sets all eight `dos` bits and the `vol` bit on the real KS1.3 ROM | R1–R5, R10 |
| ST-11 | A500 viewer line | The line crop equals `docs/amiga-sdk/a500-assets-line.png`, including `img: ok 320x200x4 hash <N> dos 255 vol 1` | R9, R10 |
| ST-12 | A500 held frame | The held display crop equals `docs/amiga-sdk/a500-assets.png`; two consecutive held captures compare at zero differing pixels | R10 |
| ST-13 | Earlier gates | `sdk-a500-smoke`, `sdk-a500-boing`, `sdk-a500-sprites`, `sdk-a500-paddle`, and `sdk-a500-text` still pass | R12 |
| ST-14 | Registry | Bundle 5 is 5/7 done, `SDK-DOS-004`/`006` are deferred, and coverage is regenerated | R12 |
| ST-15 | Resources | The loader rejects a file over 128 KB and an image over 320x256 before allocating; the parser rejects a row over 128 bytes | R9, R13 |
| ST-16 | Sanitizer | The host parser suite builds and runs under AddressSanitizer when `RDC_ASAN` is set | R6, R7 |

## Implementation tests

| ID | Case | Expected |
|---|---|---|
| IT-1 | Wrapper routing | The emitted C calls `cl_std_amiga_dos_seekFile`, `cl_std_amiga_dos_pathInfo`, `cl_std_amiga_ilbm_loadIlbm`, and `cl_video_bitmap_indices` from the corresponding wrappers |
| IT-2 | Drop order | `Lock`, `PathInfo`, `VolumeInfo`, `File`, `Buffer`, `Bitmap`, `IlbmImage`, and `Font` drops appear behind null guards in reverse declaration order |
| IT-3 | Parser bounds | The emitted parser uses subtraction window checks and checked slices; malformed cases return before writing |
| IT-4 | Fixture bytes | `python3 tools/amiga-sdk/make_ilbm_fixture.py` into a scratch directory produces bytes identical to the committed fixture; the `FORM` length, chunk sizes, and even padding are consistent |
| IT-5 | Shim types | Generated prototypes for `Lock` use `BPTR`; `PathInfo`/`VolumeInfo` use opaque `Cl*` pointers; the shim builds with the 1.3 flags |
| IT-6 | Short names | `pathInfoName`/`volumeName`/`currentDirName`/`parentDirName` copy at most the caller buffer length and return that count |

## Red/green order

1. **Red:** write the parser, DOS, and viewer spec cases while the modules are
   absent; capture the unresolved-module and host-rejection failures.
2. **Green:** add `std.iff`, the DOS surface, `std.amiga.ilbm`, the video
   primitive, and the fixture; run the host parser cases and both cross-builds.
3. **Impl:** add the routing/drop/fixture/short-name tests in
   `amiga_assets_impl_test.rs`; run them green.
4. **Target:** add the A500 smoke, capture the goldens, and run
   `make sdk-a500-assets`.
5. **Closeout:** `make check`, `make verify`, registry/issue/roadmap.

## Verification commands

| Command | Purpose |
|---|---|
| `cargo test --test amiga_ilbm_spec_test` | Parser and host/rejection cases |
| `cargo test --test amiga_dos_spec_test` | DOS surface compile and cross-build cases |
| `cargo test --test amiga_assets_impl_test` | Routing, drops, fixture, short names, shim types |
| `make asan` | Parser host suite under AddressSanitizer |
| `make sdk-a500-assets` | Real KS1.3 viewer run and goldens |
| `make check` | Spec check, registry test, workspace tests |
| `make verify` | Full acceptance, including the five earlier A500 gates |

## Evidence to keep

| Evidence | Path |
|---|---|
| Held viewer frame | `docs/amiga-sdk/a500-assets.png` |
| Viewer line golden | `docs/amiga-sdk/a500-assets-line.png` |
| Fixture | `examples/amiga/assets/clorraine.ilbm` |
| Parser host output | test stdout (width/height/depth/colors/hash/pixel values) |
| A500 check line | `img: ok 320x200x4 hash <N> dos 255 vol 1` |

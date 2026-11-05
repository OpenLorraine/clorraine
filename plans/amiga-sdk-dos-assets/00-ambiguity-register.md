# Ambiguity Register: Amiga SDK Bundle 5 — DOS and Assets

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Status**: ✅ GATE PASSED
> **Mode**: auto-design (eligible technical decisions delegated and recorded)

## Scope baseline

Bundle 5 (`5-dos-assets`) from `docs/amiga-sdk/registry.tsv`: seek/read/write
(must), locks/directories/delete/rename (must), path/assign/volume info
(should), CLI args via ReadArgs (should, 2.0+), IFF/ILBM parser (must),
datatypes.library (could, 3.0+), and the image viewer demo (must). No language,
grammar, diagnostic, or ABI change.

## Register

| # | Category | Decision / question | Resolution | Owner | Status |
|---|---|---|---|---|---|
| AR-1 | Scope | Which capabilities belong to Bundle 5? | DOS files, directories, path/volume info, IFF/ILBM, and the viewer are in scope. `SDK-DOS-004` (ReadArgs, 2.0+) and `SDK-DOS-006` (datatypes, 3.0+) are deferred because the acceptance target is KS1.3. | User (registry), AI — delegated | ✅ Resolved |
| AR-2 | Module split | One module or several? | Three surfaces with separate ownership: pure `std.iff` for bytes and ILBM decoding, extended `std.amiga.dos` for AmigaDOS resources, and `std.amiga.ilbm` for file-to-Bitmap loading. `std.amiga.video` gains only the pixel-index copy primitive the loader needs. | AI — delegated (auto-design) | ✅ Resolved |
| AR-3 | Parser location | Clorraine or the C shim? | Pure Clorraine in `std.iff`. It is host-runnable, so the specification tests exercise the real parser on the development host instead of only cross-building it. The C shim is limited to OS calls. | AI — delegated (auto-design) | ✅ Resolved |
| AR-4 | ILBM subset | Which IFF/ILBM features? | `FORM ILBM`; `BMHD` (width/height 1..1024, depth 1..8, masking 0); `CMAP` optional up to 256 colors; `BODY` interleaved by row; compression 0 (none) and 1 (ByteRun1). Multi-`BODY`, HAM, EHB, masking, and deep (>8) images are out of scope. | AI — delegated (auto-design) | ✅ Resolved |
| AR-5 | Odd geometry | How are odd widths and padding handled? | Rows are word-aligned: `row_bytes = ((width + 15) / 16) * 2`. The decoder reads the padding and ignores it. | AI — delegated (auto-design) | ✅ Resolved |
| AR-6 | Malformed input | What happens on bad IFF data? | `ilbmInfo` returns `ok = false`; `ilbmDecode` returns `false` and never slices or writes out of bounds. The loader returns an image with `ok = false`. No traps on hostile files. | AI — delegated (auto-design) | ✅ Resolved |
| AR-7 | File handle model | Does `Seek` use AmigaDOS or libc? | The existing `File` contract is backed by the toolchain libc (`fopen`/`fread`/`fwrite`), so seek/position/length use `fseek`/`ftell`. Locks and path/volume info use `dos.library` because they name paths, not open handles. | AI — delegated (auto-design) | ✅ Resolved |
| AR-8 | DOS surface | Which public calls? | `seekFile`, `filePosition`, `fileLength`; `lockPath`/`unlockPath`; `pathInfo` (+ name/is-dir/size); `makeDir`, `deleteFile`, `renameFile`, `setCurrentDir`, `currentDirName`, `parentDirName`; `volumeInfo` (+ name/free/used). This covers the registry rows without exposing raw `FileInfoBlock`/`InfoData`. Assigns resolve through `Lock`/`Open`; 1.3 assign-list enumeration is out of scope because `LockDosList` is 2.0+. | AI — delegated (auto-design) | ✅ Resolved |
| AR-9 | Path info ownership | Who owns `FileInfoBlock` and `InfoData`? | `pathInfo` and `volumeInfo` return opaque handles that own their DOS structure and release it in `freedBy`; the name query copies into a caller buffer. The lock is released before the handle is returned. | AI — delegated (auto-design) | ✅ Resolved |
| AR-10 | Volume name | How is the volume name read on 1.3? | `Info` fills `InfoData`; the name comes from the `DeviceNode` behind `id_VolumeNode` using the BCPL pointer conversion. The lock stays held until the read and the `Examine` fallback finish; a zero `id_VolumeNode` skips the node path. The name is copied into the handle; the DOS list is not retained. | AI — delegated (auto-design) | ✅ Resolved |
| AR-11 | Loader API | How are pixels and palette returned? | `loadIlbm(path, palette)` returns an owned `Bitmap?` and fills the caller's RGB palette buffer (up to its length). The caller can size the buffer from `1 << depth` and applies colors with `displaySetColor`. | AI — delegated (auto-design) | ✅ Resolved |
| AR-12 | Fixture | What image proves the viewer? | A committed 320x200x4, 16-color, ByteRun1-compressed ILBM with large runs and a geometric pattern. A small generator under `tools/amiga-sdk/` documents and reproduces the exact bytes. | AI — delegated (auto-design) | ✅ Resolved |
| AR-13 | Demo | What does the viewer do? | DOS self-test (create/write/seek/read/rename/delete/dir/path/volume), load the ILBM, apply 16 palette entries, show it on a 320x200x4 custom display, draw a caption layer with the ROM font, hold a deterministic frame, and print the check line. | AI — delegated (auto-design) | ✅ Resolved |
| AR-14 | Acceptance | How is the viewer proven on real 1.3? | `make sdk-a500-assets` boots the demo on the real KS1.3 ROM, compares the held display and the `image:` line byte-exact, and the line carries the DOS and volume check bits plus the bitmap checksum. | User (programme acceptance), AI — delegated | ✅ Resolved |
| AR-15 | Compatibility | Does the 3.1 profile change? | The new modules are available on both hosted profiles. `std.iff` is also available on the host. No existing API changes. The 1.3 whitelist adds `std.iff` and `std.amiga.ilbm`; `std.amiga.dos` is already listed. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-16 | Documentation | Which docs move with the code? | `spec/appendix-amiga.md`, `spec/appendix-stdlib.md`, `AGENTS.md`, registry/coverage, issue #6, programme/roadmap, plan. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-17 | Language | Any language/grammar/diagnostic/ABI change? | None. | User (programme rule) | ✅ Resolved |
| AR-18 | Testing | Where do the tests live? | `compiler/tests/amiga_ilbm_spec_test.rs`, `amiga_dos_spec_test.rs`, `amiga_assets_impl_test.rs`, and `docker/scripts/a500-assets-smoke.sh`. The parser host suite is `RDC_ASAN`-aware and is discovered by `make asan`. | AI — delegated (auto-design) | ✅ Resolved |

## Open items

None. Every row is resolved; the two advanced rows are explicitly deferred with
their minimum-OS reason.

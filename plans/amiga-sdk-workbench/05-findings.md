# Findings: Amiga SDK Bundle 8 — Workbench and Services

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done — implementation, review, and closeout findings resolved

## Discovery findings

These are the facts the licensed media and the real 2.0/3.1 machines
established before planning. They drive the scope and the gate design.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-1 | 🟠 Major | The environment has licensed Kickstart ROMs but no Workbench ADF/HDF. `commodities.library`, `locale.library`, `rexxsyslib.library`, `clipboard.device`, `serial.device`, `parallel.device`, `printer.device`, and `nonvolatile.library` are absent from the KS2.05/KS3.1 ROM scans. | **Accepted** — SDK-WBK-003/004/005/006/007 are deferred and nonvolatile is split to SDK-WBK-010 (deferred), with the ROM evidence in 02-current-state; no wrapper is shipped without runtime proof. |
| RV-2 | 🟠 Major | FS-UAE host mouse/keyboard injection reaches the emulator's own F12 menu but not the emulated Workbench screen in the headless setup, so a double-click-driven AppMessage is not reliable as an automated gate. | **Accepted** — the live AppIcon is proven with a native Workbench capture; the AppMessage path is exercised by a test-only fixture that posts three real `struct AppMessage`s (icon/window/menu) to the demo's public port. This limitation is documented. |
| RV-3 | 🟢 Info | The ROM `workbench.library` private `StartWorkbench` vector (LVO -42) returns 1 and opens the `Workbench` public screen on KS2.05 and KS3.1. | **Used by the test harness only** — `workbench-start.c` mirrors `C:LoadWB`; the public SDK never exposes the vector. |
| RV-4 | 🟠 Major | The first keymap probe left `InputEvent.ie_Class` unset, so `MapRawKey` returned 0 for every code and the plan initially treated the keymap as library-call only. | **Fixed** — `cl_map_raw_key` zeroes the event and sets `IECLASS_RAWKEY`; both gates map raw key 0x20 through the ROM default keymap and report `key 1`. The keymap sub-part is runtime-proven. |
| RV-5 | 🟡 Minor | `RemoveAppIcon`/`RemoveAppWindow`/`RemoveAppMenuItem` return pointer residue rather than a usable BOOL on the tested ROMs; a duplicate `RemoveAppIcon` still returned the object's low word. | **Handled** — the wrapper removers are `void`, follow the Workbench example, and free the owned records after the remove call; the object pointer is the ownership token. A removal failure is an exceptional Workbench state. |
| RV-6 | 🟡 Minor | The 1.3 NDK lacks the 2.0 `clib/wb_protos.h`, `clib/keymap_protos.h`, `clib/cardres_protos.h`, and `resources/card.h` headers, so the shared shim cannot include them unconditionally. | **Handled** — the 2.0+ shim is a separate suffix appended only for `hosted-amiga`; the 1.3 build never sees it and never links the 2.0 symbols. |
| RV-7 | 🟢 Info | `FindConfigDev` reports two Commodore boards on both models (manufacturer 2011, products 82 and 4), and `card.resource` is present on both with status 0. | **Used by the gates** — both gates pin `exp 2 man 2011 prod 82 card 1 status 0` in the `svc` line. |
| RV-8 | 🟠 Major | The `clib` stubs for `AddAppIconA`, `GetDefDiskObject`, `FindConfigDev`, `MapRawKey`, and `ReadCardStatus` use the NDK base globals (`WorkbenchBase`, `IconBase`, `ExpansionBase`, `KeymapBase`, `CardResource`), which `libstubs.a` only defines as zero. | **Fixed** — the 2.0 suffix assigns each base from `OpenLibrary`/`OpenResource` before the call; the impl suite pins the assignments. |
| RV-9 | 🟠 Major | `CreatePort` does not copy the port name and `openScreen` does not copy the screen title; passing a temporary `toCString(stringOf(...))` leaves a dangling name. | **Fixed** — the demo declares `portName`/`screenTitle` `String`s before the port/screen; the AppIcon/AppMenuItem titles are copied into their records. |
| RV-10 | 🟠 Major | The original gate design proved the Workbench row only on KS3.1 while the row is min OS 2.0. | **Fixed** — the A600/KS2.05 gate runs the same Workbench and services sequence; the A1200 gate adds the capture comparison. |
| RV-11 | 🟡 Minor | A stale report, capture, or icon file could satisfy a poll after a failed run. | **Fixed** — both gates clear and rebuild the directory HD before every run and assert the start/ready/report/fixture/cleanup handshakes. |

## Closeout review findings

Independent correctness, memory-safety, and emission-integrity reviews after the
full gate. Every critical/major finding is fixed; the remaining notes are
accepted with the recorded rationale.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-C1 | 🟠 Major | `MapRawKey` received an uninitialized `InputEvent` without `IECLASS_RAWKEY`, so the keymap probe returned 0 and the test path was wrong. | **Fixed** — the suffix zeroes the event and sets the class; the gates now map raw key 0x20 and report `key 1`; the impl test pins the initialization. |
| RV-C2 | 🟠 Major | The gate cleanup removed `"${repo_dir}/${rom}"` although `rom` is already absolute, leaving the decrypted ROM behind. | **Fixed** — both gates use `rm -f "${rom}"`; the ROMs are absent after the runs. |
| RV-C3 | 🟡 Minor | The removal frees were gated on the Workbench BOOL, which is not a reliable success signal (pointer residue; a duplicate removal still returned the low word). | **Fixed** — the shim follows the Workbench example: call the remover and free the owned image/title; the object pointer is the ownership token. |
| RV-C4 | 🟡 Minor | No generated-drop tests for AppWindow/AppMenuItem/AppMessage and no AppIcon failure/order/tag pins. | **Fixed** — added labeled caller-cleanup tests and the AppIcon failure/removal/tag pins. |
| RV-C5 | 🟡 Minor | The 2.0 guard test could not fail; the gate test pinned only the `wb` prefix. | **Fixed** — the 2.0 shim is a separate suffix with structural/placement tests; the gates pin the full lines, the crop path/offset, `identify`, and `wb-done`. |
| RV-C6 | 🟡 Minor | The demo wrote its report before the cleanup; a crash during removal could pass. | **Fixed** — the demo writes `CNEXT:wb-done.txt` after the removals/close/delete and both gates require `done`. |
| RV-C7 | 🟡 Minor | The start assertion accepted `screen 0` and the fixture printed the raw BOOL. | **Fixed** — the fixture prints `(started != 0)`; the gates require `screen [1-9a-f]`. |
| RV-C8 | 🟡 Minor | State was ahead of reality (0/26 vs 22/26, issue #9 open, pending closeout). | **Fixed** — the plan is 26/26, the findings/roadmap/programme index are current, and issue #9 is closed. |
| RV-C9 | 🟡 Minor | SDK-WBK-009 was min OS 3.1/AGA while the A600 gate proves it on 2.05/ECS. | **Fixed** — the row is 2.0/ECS/68000/1M; coverage/issue regenerated. |
| RV-C10 | 🟡 Minor | The `WBAPPICONA_PropagatePosition` tag is V44 while R9 said V37-only. | **Fixed** — the docs record the tag as V44 and note that V37/V40 use `do_CurrentX/Y`. |
| RV-C11 | 🟡 Minor | `cl_card_present` did not assign `CardResource` as the design said. | **Fixed** — it assigns and clears the base; `cl_card_status` does the same. |
| RV-C12 | 🟡 Minor | Stale test headers; no freestanding rejection; the icon appendix slice could match §14.2/14.3. | **Fixed** — headers, the freestanding test, and the precise appendix slices. |
| RV-C13 | 🟡 Minor | Documentation drift after the fixes (keymap limitation, `#define`/shim references, `wb-done`). | **Fixed** — the docs use `key 1`, the suffix file, and the `wb-done` handshake. |
| RV-C14 | 🟡 Minor | The `& 0xFFFF` removal guard was semantically the same as the old BOOL test. | **Fixed** — the guard was removed; the canonical removal/free path is used. |
| RV-C15 | 🟡 Minor | The drop tests used a count that a wrapper's own cleanup could satisfy. | **Fixed** — the helper requires a `__cl_cN:` label immediately before the caller block. |
| RV-C16 | 🟡 Minor | The suffix test did not prove the suffix follows the generated program. | **Fixed** — the test asserts `int main(void)` precedes the suffix marker. |
| MS-1 | 🟡 Minor | The removal release decision was driven by a non-boolean Workbench return; a false branch leaked and a truthy failure could free early (host model). | **Accepted** — the tested ROMs return pointer residue, so the BOOL is unusable; the shim follows the Workbench example and frees after the remove call. A removal failure is an exceptional Workbench state. |
| MS-2 | 🔵 Info | The fixture leaked prior messages/port on a partial allocation failure. | **Fixed** — the cleanup frees the non-null messages and deletes the reply port. |
| MS-3 | 🔵 Info | The 2.0 suffix has no sanitizer-reachable execution path (m68k-only shim); a host model covers the record functions. | **Accepted** — static inspection, the host record model, the cross-builds, and the two runtime gates are the available evidence. |
| EI-1 | 🟠 Major | The 2.0 headers were emitted before user code, adding ~1,133 macros that could rewrite user identifiers (`WBTOOL`, `AM_VERSION`, `CL_HOSTED_AMIGA_20`, …). | **Fixed** — the 2.0 shim is a separate suffix appended after the generated program; the collision probes now build on both profiles, and the shared/1.3 emit has no new macros. |
| EI-2 | 🟡 Minor | The keymap `InputEvent` fix was not pinned. | **Fixed** — the impl test pins `memset`, `IECLASS_RAWKEY`, and `ie_SubClass`. |
| EI-3 | 🟠 Major | The new suffix source was untracked while `include_str!` referenced it. | **Fixed** — `compiler/runtime/shim_hosted_amiga_20.c` is staged/committed. |
| EI-4 | 🟡 Minor | The roadmap/programme index still said `key 0`. | **Fixed** — both say `key 1`. |

## Closeout verification

| Gate | Result |
|---|---|
| `cargo test --lib --test amiga_workbench_spec_test --test amiga_workbench_impl_test` | 403 lib + 22 impl + 17 spec green |
| `make sdk-a600-services` | `wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0`; `svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 1` |
| `make sdk-a1200-workbench` | Same lines; `docs/amiga-sdk/a1200-workbench.png` (1280x512) and `a1200-workbench-icon.png` (160x120) match |
| `make check` | 109 test-result suites, 0 failures |
| `make verify` | All legs passed headlessly on the final tree, including `sdk-a600-services` and `sdk-a1200-workbench` |
| `make sdk-registry` | OK (80 capabilities, 10 bundles); Bundle 8 is 4 done / 6 deferred; coverage regenerated |
| GitHub issue #9 | Updated with the checked body, commented, and **CLOSED** |

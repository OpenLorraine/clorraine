# Preflight Report: Amiga SDK Bundle 6 — Intuition UI

> **Document**: 00-preflight-report.md
> **Parent**: [Index](00-index.md)
> **Audit Target**: all ten documents in `plans/amiga-sdk-intuition/`
> **Context**: `AGENTS.md`, `docs/amiga-sdk/registry.tsv`, `bundles.tsv`,
> `coverage.md`, `stdlib/`, `compiler/src/{stdlib,profile,abi}.rs`,
> `compiler/runtime/shim_hosted_amiga.c`, `compiler/tests/`,
> `docker/scripts/a500-*-smoke.sh`, `spec/appendix-amiga.md`,
> `spec/appendix-stdlib.md`, and the pinned 1.3 NDK headers/SFD.
> **Iteration**: 1
> **Codebase Grounded**: 34 source, test, and documentation files; 52
> references verified; 4 claims rejected as false positives.
> **CodeOps Artifact Schema**: 1
> **Last Updated**: 2026-11-05 10:25

⚠️ **SAME-SESSION REVIEW:** the plan was authored in this session. Five
independent read-only audit clusters ran against the artifact; the same-session
bias risk remains for the merge and synthesis. Consider a fresh-session review
before the final acceptance run.

## Verdict

✅ **PREFLIGHT PASSED WITH NOTES** — 0 critical/major open; all 25 findings are
resolved or explicitly accepted; 4 audit claims were dismissed against the
pinned 1.3 headers.

## Codebase Context Summary

**Tech Stack:** Rust workspace (`compiler/`), embedded Clorraine stdlib
(`stdlib/`), C99 runtime shims (`compiler/runtime/`), m68k AmigaOS toolchain in
the pinned Docker SDK, FS-UAE A500 smoke harness.

**Architecture:** the compiler emits one C99 translation unit; the profile shim
(`shim_hosted_amiga.c`) supplies the AmigaOS calls; the stdlib modules are
`include_str!`-embedded and exported through `compiler/src/stdlib.rs`; the 1.3
profile gates `std.amiga.*` through `compiler/src/profile.rs`.

**Key files examined:**

| File | Fact used |
|---|---|
| `stdlib/std.amiga.intuition.clo:14-36` | Only `Screen` + `openScreen`; title is caller-owned |
| `stdlib/std.amiga.graphics.clo:15-51` | `setColor` doc says `0..(1 << depth) - 1` |
| `stdlib/std.amiga.text.clo:17-78` | `drawText` takes `const u8[]`; no window text |
| `compiler/runtime/shim_hosted_amiga.c:1064-1150` | `cl_open_screen`, `SetRGB4`, `CloseScreen` |
| `compiler/runtime/shim_hosted_amiga.c:3309-3458` | Font wrapper, text draw on `screen->RastPort` |
| `compiler/src/stdlib.rs:101-108,395-400` | Intuition/graphics/text registration and export tables |
| `compiler/src/profile.rs:90-112,240` | 1.3 module whitelist and host rejection |
| `compiler/tests/amiga_text_spec_test.rs:222-312` | Cross-build pattern |
| `docker/scripts/a500-assets-smoke.sh:120-171` | Boot, window search, capture/compare |
| `docs/amiga-sdk/registry.tsv:49-57` | Bundle 6 rows, 15-column schema |
| `tools/amiga-sdk/registry.sh:121-123` | Done rows require acceptance/evidence |
| `intuition.h:624` | `StringInfo.MaxChars` includes the NUL |
| `sfd13/intuition_lib.sfd:59,76-79,97-99` | `SetMenuStrip` BOOL; `AutoRequest` 8 args; `RefreshGList`/`BeginRefresh`/`EndRefresh` VOID; `AddGList`/`RemoveGList` UWORD |
| `intuition.h:789-795,1017-1024` | `IntuiMessage.MouseX/Y` are outer-window; `GZZMouseX/Y` are inner |

**Reference verification:** 52 references mapped; 48 verified directly, 4
rejected as false positives (see Dismissed Findings).

## Summary by Dimension

| # | Dimension | Findings | Highest severity |
|---|---|---|---|
| 1 | Ambiguities | 3 | 🟠 MAJOR |
| 2 | Implicit assumptions | 2 | 🟠 MAJOR |
| 3 | Logical contradictions | 3 | 🟠 MAJOR |
| 4 | Completeness gaps | 3 | 🟠 MAJOR |
| 5 | Dependency issues | 2 | 🟠 MAJOR |
| 6 | Feasibility concerns | 3 | 🟠 MAJOR |
| 7 | Testability | 5 | 🟠 MAJOR |
| 8 | Security blind spots | 1 | 🟡 MINOR |
| 9 | Edge cases | 3 | 🟠 MAJOR |
| 10 | Scope creep indicators | 1 | 🟡 MINOR |
| 11 | Ordering & sequencing | 2 | 🟠 MAJOR |
| 12 | Consistency | 5 | 🟠 MAJOR |
| 13 | Codebase alignment | 4 | 🟠 MAJOR |

## Summary by Severity

| Severity | Count | Status |
|---|---|---|
| 🔴 CRITICAL | 0 | — |
| 🟠 MAJOR | 19 | all resolved |
| 🟡 MINOR | 7 | all resolved or accepted |
| 🔵 OBSERVATION | 4 | accepted/applied |

## Findings and Resolutions

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-001 | 🟠 MAJOR | Interactive ST cases were assigned to the host-only spec suite; the A500 cases had no runner. | 07 now tags every case Host/A500/Impl; ST-4/5/7–13/15/16 run in the GUI smoke; 99 phases only require host cases green before Phase 4. |
| PF-002 | 🟠 MAJOR | R3 said `pollEvent` returns `null`, contradicting AR-23 and the non-nullable `WindowEvent`. | R3 now returns `EVENT_NONE`; `EVENT_NONE` is in the kind list and the event contract is single-sourced in 03-01. |
| PF-003 | 🟠 MAJOR | Acceptance said “thirteen tasks”; the execution plan has 17. | 01 AC1 and 99 now say seventeen; the 5+4+4+4 split matches. |
| PF-004 | 🟠 MAJOR | Close order omitted button `Image` records, `StringInfo`, the string `Border`/`XY`, and exact free sizes. | 03-01 step 4–6 and 03-02 now list every allocation and its exact `FreeMem` size; phase 1 implements the title/wrapper part and later phases extend it. |
| PF-005 | 🟠 MAJOR | The plan treated `AddGList` return 0 as failure and listed a `RefreshGList` failure; `AddGList` returns a position and `RefreshGList` is `VOID`. | 03-02 now ignores the `AddGList` return after validation, removes the `RefreshGList` failure path, and pins the 1.3 signatures. |
| PF-006 | 🟠 MAJOR | The menu/item append did not set the previous `NextMenu`/`NextItem`/`FirstItem`, and a `SetMenuStrip` failure freed a still-linked node. | 03-02 now specifies the chain links and unlink-before-free on a false `SetMenuStrip`; IT-5 pins the chain. |
| PF-007 | 🟠 MAJOR | Structs were allocated without clearing; `Image`/`Border` render fields were unspecified. | Every wrapper/Intuition record now uses `MEMF_CLEAR`/`memset`; the button `Image` fields and the string `Border`/`XY` are specified. |
| PF-008 | 🟠 MAJOR | `windowString` used `strlen` on the Intuition-edited buffer; duplicate ids and a null initial were not rejected. | 03-02 uses `NumChars` clamped to `MaxChars-1`; duplicate ids and null initial are rejected; `windowString` writes a bounded trailing NUL. |
| PF-009 | 🟠 MAJOR | Event `mouse_x`/`mouse_y` copied outer-window `IntuiMessage` coordinates while drawing uses inner coordinates. | 03-01 uses `Window.GZZMouseX/GZZMouseY`; IT-6 pins it. |
| PF-010 | 🟠 MAJOR | `x + width` checks could overflow before narrowing into `WORD`. | 03-01/03-02 now cap dimensions and use subtractive checks (`x > limit - width`) after the size check. |
| PF-011 | 🟠 MAJOR | The report line could not prove the alert; a silent no-op still matched. | AR-25 supersedes the alert golden: the interactive About path uses `askYesNo`, `showAlert` is spec-tested, and `req` counts only Yes. |
| PF-012 | 🟠 MAJOR | `pollEvent`, `windowWidth/Height`, `windowFill`, and `windowDraw` were dead surface; the close gadget and key/mouse/active events were not exercised. | The demo now drains with `pollEvent`, uses the inner-size and drawing calls, and exits with the File → Quit command key. The close-gadget, mouse, and raw-key branches are pinned by the impl cases and are not driven by the keyboard-only A500 smoke (AR-25). |
| PF-013 | 🟠 MAJOR | `setColor` documents pens 0–15 on a depth-4 screen, but the pointer uses 17/18. | AR-24 records the 0–31 `ColorMap` range (0–15 playfield, 16–31 sprite); 03-03, 01 R8, 99 1.1.3/4.1.3 and the appendices are updated. No behavior change. |
| PF-014 | 🟠 MAJOR | Usage sketches passed `toCString(stringOf(...))` to `openScreen`, which keeps the pointer; the temporary `String` dangles. | 00-index, 03-01, and 03-04 bind the screen title to a local `String` that outlives the screen; the demo does the same. |
| PF-015 | 🟠 MAJOR | The demo passed string literals to `windowText` (`const u8[]`) and never opened or null-checked the font. | 03-04 uses `bytes(stringOf(...))` for synchronous draws and opens Topaz with a `defaultFont` fallback and null check. |
| PF-016 | 🟠 MAJOR | The A500 menu sequence used left clicks; classic 1.3 menus need the right button held. | AR-25 supersedes the pointer gesture: the smoke is keyboard-only on Xvfb, uses Right Amiga command keys for the menus, and Left Amiga+V for requesters. |
| PF-017 | 🟠 MAJOR | The registry closeout flipped rows to done without acceptance/evidence, which the validator rejects. | 03-04 now lists every row’s acceptance/evidence/spec/notes; 99 4.1.3 names the columns and `make sdk-registry`. |
| PF-018 | 🟠 MAJOR | The string gadget had no render `Border`, and the button `Image` records had no geometry. | 03-02 specifies the string `Border`/`XY` and the button `Image` width/height/depth; IT-4/IT-8 pin both. |
| PF-019 | 🟠 MAJOR | Phase 1’s close path and IT-1 referenced gadget/menu/pointer types that arrive in later phases. | 03-01 stages the close path; 99 1.1.2/1.1.5 scope phase 1 to title/wrapper and extend IT-1 in 2.1.4/3.1.4. |
| PF-020 | 🟡 MINOR | ST-17/ST-18 were assigned to 4.1.2, and ST-2 was described as a red case although host rejection already passes. | 99 4.1.2 covers ST-4/5/7–13/15/16; 4.1.4 covers ST-17/ST-18; 07 marks ST-2 a regression guard. |
| PF-021 | 🟡 MINOR | `WindowEvent` fields used camelCase while the repository’s public structs use snake_case. | AR-10/03-01/07 now use `mouse_x`/`mouse_y`; the accessors are unchanged. |
| PF-022 | 🟡 MINOR | The report line was called 41 characters (it is 38), and `menu` could mean headers or items. | 03-04 records 38 bytes and defines `menu` as menu headers. |
| PF-023 | 🟡 MINOR | A 128-byte requester/alert body cannot fit the 320-pixel screen. | AR-14/01/03-03/07 now cap requester/alert bodies at 28 bytes (240-pixel requester, 320-pixel alert with x=20); the demo uses 9/14 bytes. |
| PF-024 | 🟡 MINOR | ST-2 cited AR-6 (drawing home) instead of the host-gating rule. | 07 ST-2 now cites R12. |
| PF-025 | 🟡 MINOR | The current-state citations for `openScreen` and the `std.amiga.exec` 1.3 gate were off. | 02 cites `stdlib/std.amiga.intuition.clo:14-36` and `compiler/src/profile.rs:90-112`. |
| PF-026 | 🔵 OBSERVATION | “No new test harness” conflicted with the new interaction steps. | 00-index now says the existing FS-UAE boot/capture harness is reused and the smoke adds interaction steps. |
| PF-027 | 🔵 OBSERVATION | `AddGList`/`SetMenuStrip` failure handling was over-specified. | Covered by PF-005/PF-006: only `SetMenuStrip` has a checkable result; `AddGList` is a position. |
| PF-028 | 🔵 OBSERVATION | `windowFill`/`windowDraw`/`pollEvent` had no consumer. | Covered by PF-012; the demo now uses them. |
| PF-029 | 🔵 OBSERVATION | A 64-byte label can be wider than its gadget. | 03-02 documents that Intuition clips an over-wide label; the demo uses short labels. |

## Dismissed Findings

| ID | Claim | Evidence | Disposition |
|---|---|---|---|
| D-1 | `StringInfo.MaxChars` excludes the NUL, so `maxChars+1` overflows. | `intuition.h:624`: “max number of chars in Buffer (including NULL)”. | **Dismissed** — the plan’s `MaxChars = maxChars + 1` for a `maxChars+1` buffer is correct. |
| D-2 | `AutoRequest` takes ten arguments; the plan passes eight. | `sfd13/intuition_lib.sfd:76`: `AutoRequest(window, body, posText, negText, pFlag, nFlag, width, height)` — 8 arguments. | **Dismissed** — the 1.3 call is correct. |
| D-3 | `SetMenuStrip` returns void, so its failure branch is dead. | `sfd13/intuition_lib.sfd:59`: `BOOL SetMenuStrip(...)`. | **Dismissed** — the branch is valid; `ClearMenuStrip`/`RefreshGList` are the void calls. |
| D-4 | `AddGList` return 0 is a valid position, not a failure. | `sfd13/intuition_lib.sfd:98`: `UWORD AddGList(...)`; the return is the insertion position. | **Accepted** — folded into PF-005. |

## Accepted Residuals

| Residual | Reason |
|---|---|
| The A500 smoke is empirical about boot time and click coordinates. | The existing Bundle 0–5 harness is empirical; the smoke calibrates once, checks each expected frame, and keeps the last screenshot on failure. |
| The `openScreen` title remains caller-owned. | Changing the legacy contract is outside this bundle; the demo binds the title and AR-4 documents the rule. |
| The window handle has no lifetime tie to its screen. | The language has no lifetime parameter here; AR-8/03-01 document “screen outlives window”, and the demo uses the reverse drop order. |
| A 64-byte label may clip. | Visual limitation only; Intuition clips to the gadget, and the demo uses short labels. |
| The pure C shim cannot run under host AddressSanitizer. | The m68k Intuition path is proven on the real A500; the host suite is compile/cross-build. |

## Verdict

All critical and major findings are resolved in the plan. The remaining minor
and observation items are fixed or documented as accepted residuals.

**Pass tier:** ✅ **PASSED WITH NOTES** — 0 critical/major open; accepted
residuals listed above.

# Testing Strategy: Amiga SDK Bundle 8 — Workbench and Services

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The suite follows the repository's specification-first split:

- **Specification tests** (`amiga_workbench_spec_test.rs`) derive from R1–R10.
  They are the oracle: a failure means the implementation is wrong.
- **Implementation tests** (`amiga_workbench_impl_test.rs`) pin the emitted
  shim calls, the handle/drop routing, the AppMessage accessors, the icon
  round-trip, the expansion/card/keymap calls, and the demo shape.
- **Runtime acceptance** (`make sdk-a1200-workbench`,
  `make sdk-a600-services`) proves the behavior on the real A1200/KS3.1 and
  A600/KS2.05 machines.

The existing A500 gates and the Bundle 7 exec suite are immutable.

## Specification Test Cases

### Profile and surface

| # | Venue | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|---|
| ST-1 | Host | A program imports every icon, Workbench, and expansion wrapper, `createPublicPort`, and the public-port drop; compiles for `hosted-amiga` | Exit 0, zero diagnostics, emitted C names every shim call | R1–R4 |
| ST-2 | Host | The same program compiles for `hosted-amiga-13` and `host` | Exit 1 with `E11200`; no C emitted | R9 |
| ST-3 | Host | The full surface cross-builds with `m68k-amigaos-gcc -mcrt=nix20 -m68000 -fwrapv -Wall -Wextra -Werror ... -lamiga -lm` | Build passes; the test skips itself when the cross toolchain is absent | R9 |
| ST-4 | Host | `spec/appendix-stdlib.md` §13.2 and §14 are read | Every exec public-port and icon/Workbench/expansion export appears as a whole identifier | R10 |

### Runtime

| # | Venue | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|---|
| ST-5 | A1200/KS3.1 and A600/KS2.05 | The Workbench demo declares the port/screen names, adds an AppIcon, AppWindow, and AppMenuItem, captures the Workbench screen, waits for three AppMessages, and removes the objects | `CNEXT:wb.txt` is `wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0` | R1, R6 |
| ST-6 | A1200/KS3.1 and A600/KS2.05 | The fixture posts three `struct AppMessage`s (icon, window, menu) to `ClorraineWbTest` | `CNEXT:wb-post.txt` is `posted`; the demo replies all three and exits | R1, R5 |
| ST-7 | A1200/KS3.1 | The native Workbench capture is converted and compared | `a1200-workbench.png` matches `docs/amiga-sdk/a1200-workbench.png` byte-exact, is non-blank, and its AppIcon crop matches `docs/amiga-sdk/a1200-workbench-icon.png` | R1, R6 |
| ST-8 | A600/KS2.05 and A1200/KS3.1 | The services demo reads a default icon, writes and reloads it, enumerates boards, queries the card resource, and calls `mapRawKey` | `CNEXT:svc.txt` is `svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 0` | R2, R3, R6 |
| ST-9 | A600/KS2.05 | `mapRawKey(0x20, 0)` on the bare ROM | Returns null/zero length; the report records `key 0`; the library call is reached | R3, AR-8 |
| ST-10 | A1200/KS3.1 and A600/KS2.05 | `createPublicPort("ClorraineWbTest")` returns a port and the fixture finds it with `FindPort`; the name `String` outlives the port | Non-null port; `FindPort` succeeds; the drop deletes it | R4 |

## Implementation Test Cases

| # | Target | Assertion |
|---|---|---|
| IT-1 | `compiler/src/stdlib.rs` unit test | The icon, Workbench, and expansion modules are registered with every export and source |
| IT-2 | `compiler/src/profile.rs` unit test | `hosted-amiga` provides the three modules; `hosted-amiga-13` and `host` do not |
| IT-3 | Emitted C | The Workbench wrappers call `cl_add_app_icon`, `cl_add_app_window`, `cl_add_app_menu_item`, `cl_wait_app_message`, the accessors, `cl_reply_app_message`, and `cl_save_public_screen`; the drops route to `cl_remove_*`/`cl_reply_app_message` |
| IT-4 | Shim | The AppIcon record owns the title copy, the raw icon, and the default `DiskObject`; `do_Type` is cleared to 0; the position is `(400, 140)` with the propagate tag; every failure path frees the object/title/record; `RemoveAppIcon` is followed by `FreeDiskObject` |
| IT-5 | Shim | `cl_add_app_window` reads `cl_window.window`; the AppWindow is removed before the window; `cl_add_app_menu_item` uses the port/title |
| IT-6 | Shim | `cl_wait_app_message` uses `WaitPort`/`GetMsg`; the accessors read `am_Type`, `am_ID`, `am_UserData`, `am_Class`, `am_NumArgs`; `cl_reply_app_message` calls `ReplyMsg`; the fixture sets `mn_Length`, `NT_MESSAGE`, the reply port, `am_Version`, and the three types |
| IT-7 | Shim | `cl_save_public_screen` uses `LockPubScreen`/`cl_screen_save_native`/`UnlockPubScreen` and is null-safe |
| IT-8 | Shim | The icon functions call `GetDiskObject`, `PutDiskObject`, `FreeDiskObject`, `GetDefDiskObject`; the type range check is 1..8; the drop frees |
| IT-9 | Shim | `cl_expansion_board_*` walks `FindConfigDev` and assigns `ExpansionBase`; the card assigns `CardResource`, calls open/read/`RemResource`; `cl_map_raw_key` assigns `KeymapBase` and uses `AskKeyMapDefault`/`MapRawKey` and builds a `String`; indexes are bounds-checked |
| IT-10 | Shim guard | The 2.0-only section is under `CL_HOSTED_AMIGA_20`; the 1.3 build preprocesses it out; the shared public-port function remains; the emitted `hosted-amiga` C defines the macro and the `hosted-amiga-13` C does not |
| IT-11 | Demo shape | `examples/amiga/workbench.clo` has the ten stages, the port/screen name strings, the pinned line, the public port, the capture, and the removes; `examples/amiga/services.clo` has the icon round-trip, board/card/keymap calls, and the pinned line |
| IT-12 | Test integrity | The existing exec/intuition/A500 suites still run; no golden or test is weakened; the new tests fail without the implementation |
| IT-13 | Fixtures | `workbench-start.c` uses the private `-42` vector and checks the `Workbench` screen; `workbench-post.c` sets the AppMessage header/reply/version/three types and links `-lamiga -lm` |

## Test Files

| File | Change |
|---|---|
| `compiler/tests/amiga_workbench_spec_test.rs` | New; ST-1–ST-10 |
| `compiler/tests/amiga_workbench_impl_test.rs` | New; IT-3–IT-13 |
| `compiler/src/stdlib.rs` | Module sources/exports and IT-1 |
| `compiler/src/profile.rs` | Module gating and IT-2 |
| `stdlib/std.amiga.workbench.clo` | New module |
| `stdlib/std.amiga.icon.clo` | New module |
| `stdlib/std.amiga.expansion.clo` | New module |
| `stdlib/std.amiga.exec.clo` | Public port |
| `compiler/runtime/shim_hosted_amiga.c` | 2.0+ shim section and public port |
| `compiler/src/codegen.rs` | Emit `CL_HOSTED_AMIGA_20` for `hosted-amiga`; IT-10 pins it |
| `examples/amiga/workbench.clo`, `services.clo` | New demos |
| `docker/scripts/workbench-start.c`, `workbench-post.c` | Test fixtures |
| `docker/scripts/a1200-workbench-smoke.sh`, `a600-services-smoke.sh` | Gates |

## Red/Green Order

1. Add the spec surface and ST-1/ST-3/ST-4; run red (unresolved exports).
2. Add the modules, shim, base globals, profile gating, export table, and annex
   text; run ST-1–ST-4 green.
3. Add the implementation cases IT-1–IT-13; fix the emitted shapes.
4. Build the demos and fixtures; run the A600 services gate first, then the
   A1200 Workbench gate; capture the golden.
5. Run `make check`, `make verify`, and `make sdk-registry`.
6. Run the independent review (correctness + memory safety + emission
   integrity); fix findings; commit.

## Verification Commands

```text
cargo test --lib --test amiga_workbench_spec_test --test amiga_workbench_impl_test
make sdk-a600-services
make sdk-a1200-workbench
make check
make verify
make sdk-registry
./spec/check.sh
bash -n docker/scripts/a1200-workbench-smoke.sh docker/scripts/a600-services-smoke.sh
```

## Evidence to Keep

| Artifact | Path |
|---|---|
| Workbench report | `CNEXT:wb.txt` asserted on both gates |
| Fixture report | `CNEXT:wb-post.txt` asserted on both gates |
| Workbench capture/golden | `CNEXT:workbench.cscr` -> `docs/amiga-sdk/a1200-workbench.png` and `docs/amiga-sdk/a1200-workbench-icon.png` |
| Services report | `CNEXT:svc.txt` asserted on both gates |
| Emitted shapes | `amiga_workbench_impl_test` assertions |
| Registry/coverage | `docs/amiga-sdk/registry.tsv`, `coverage.md` |
| Issue | GitHub #9 body, comment, and closed state |

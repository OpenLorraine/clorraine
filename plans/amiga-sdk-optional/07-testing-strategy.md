# Testing Strategy: Bundle 9

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Principles

The specification suites derive from the requirements and component docs, not from the
implementation. A failing spec test means the implementation is wrong. Implementation suites pin
emitted C, shim ownership, and gate scripts. Runtime gates are headless and use clean work
directories. [AR-33, AR-34]

## Row-to-Evidence Map

| Row | Module / surface | Gate | Expected line |
|---|---|---|---|
| `SDK-EXE-007` | `utility` tags/date + `pool` | `sdk-a600-addons` + `sdk-a1200-ks30-addons` | `exec: ok ... tag 1 date 1`; `pool: ok 1` |
| `SDK-EXE-010` | `utility` hooks | `sdk-a600-addons` | `exec: ok ... hook 1` |
| `SDK-EXE-011` | `semaphore` | `sdk-a600-addons` | `exec: ok sem 1 ...` |
| `SDK-INT-006` | `gadtools` | `sdk-a600-addons` (intuition mode) | `gt: ok gad 4` |
| `SDK-INT-007` | `asl` | `sdk-a600-addons` (intuition mode) | `asl: ok path asl.txt` |
| `SDK-INT-008` | `boopsi` | `sdk-a1200-ks30-addons` | `boopsi: ok hue 42` |
| `SDK-DOS-004` | `args` | `sdk-a600-addons` | `args: ok n 3 file 1 num 7 sw 1` |
| `SDK-DOS-006` | `datatypes` | `sdk-a1200-addons` | `dt: ok w 320 h 200 depth 4 colors 16` |
| `SDK-TXT-003` | `std.amiga.text.openDiskFont` | `sdk-a500-diskfont` | `diskfont: ok h 12` |
| `SDK-TXT-005` | `regions` | `sdk-a600-addons` | `regions: ok bounds 10 10 20 20 clip 1` |
| `SDK-AUD-006` | `lowlevel` | `sdk-a600-ks31-addons` | `ll: ok v 40 p0 0 p1 0` |
| `SDK-AUD-008` | `eclock` | `sdk-a600-addons` | `eclock: ok 709379` |
| `SDK-WBK-003` | `commodities` | `sdk-a600-addons` | `cmd: ok broker 1 dup 1 reopen 1` |
| `SDK-WBK-004` | `locale` | `sdk-a600-addons` | `locale: ok name 1 cc 1 gmt 1 fmt 1` |
| `SDK-WBK-005` | `arexx` | `sdk-a600-addons` | `rexx: ok action 1` |
| `SDK-WBK-006` | `clipboard` | `sdk-a600-addons` | `clip: ok n 4` |
| `SDK-WBK-007` | `devices` | `sdk-a600-addons` | `ser: ok n 4`; `par: ok n 4`; `prt: ok open 1` |
| `SDK-WBK-010` | `nonvolatile` | `sdk-a600-ks31-addons` | `nv: ok n 23` |
| `SDK-OPT-002` | `bsdsocket` | `sdk-a1200-addons` | `net: ok send 5 recv 5` |
| `SDK-OPT-003` | `amigaguide` | `sdk-a1200-addons` | `guide: ok msg 1 result 0` |

`SDK-OPT-004` RTG remains out-of-scope. Coverage is 20 in-scope done / 21 total. [AR-39]

## Specification Test Cases

| ID | Input | Expected output | Trace |
|---|---|---|---|
| ST-1 | Register each new module | `lookup`/`source`/`exports` succeed for every export | R1–R7; AR-8 |
| ST-2 | Compile each surface for host/1.3/freestanding | Exit 1, `E11200`, no `out.c` | R10; AR-10 |
| ST-3 | Compile each surface for `hosted-amiga`, cross-build `nix20 -Werror` | Exit 0, zero diagnostics, binary | R1–R7; AR-10 |
| ST-4 | Scan each module’s appendix section | Every export appears in its own section | R14; AR-34 |
| ST-5 | Semaphore source | Routes to `cl_*_semaphore`; obtain/attempt/release/free | R2; AR-8 |
| ST-6 | Hook source with `func(i32): i32` | Routes to `cl_*_hook`; no capturing lambda | R2; AR-11 |
| ST-7 | Tag/date source | `tagPresent`/`DateStamp` handle; no `u64` | R2; AR-12, AR-13 |
| ST-8 | Pool source | `cl_pool_*`; `write`/`read`; refcounted pool | R2; AR-12, AR-39 |
| ST-9 | gadtools source | `CreateContext`/`CreateGadgetA`/`RefreshGList`/`GT_*`; no readback | R3; AR-14 |
| ST-10 | ASL source | `cl_asl_*`; split drawer/file; save flag | R3; AR-15 |
| ST-11 | BOOPSI source | `colorwheel.gadget`, `WHEEL_Screen`, `SetGadgetAttrsA`/`GetAttr` | R3; AR-16 |
| ST-12 | ReadArgs source with 3 slots and command text | `cl_args_*`; `readArgsFrom`; template kinds; no overflow | R4; AR-17 |
| ST-13 | Datatypes source + ILBM | `PDTA_BitMapHeader`/`PDTA_NumColors`; `dtDraw` | R4; AR-18, AR-39 |
| ST-14 | Region source | `cl_region_*`; `regionAndRect` void; clip borrow | R5; AR-19 |
| ST-15 | EClock source | `openEClock`/`readEClock` `EClockTicks`/frequency | R5; AR-20 |
| ST-16 | Lowlevel source | `cl_lowlevel_*`; type-aware decode | R5; AR-21 |
| ST-17 | Commodities source | `NewBroker` `NB_VERSION`, duplicate, close | R6; AR-22 |
| ST-18 | Locale source | `cl_locale_*`; all four fields | R6; AR-23 |
| ST-19 | ARexx source | `ClearRexxMsg` + `DeleteRexxMsg`; reply/reclaim | R6; AR-24 |
| ST-20 | Clipboard source | `CBD_CURRENTWRITEID`, `CMD_UPDATE`, current-read ID | R6; AR-25 |
| ST-21 | Devices source | serial/parallel SETPARAMS; printer `union printerIO`/`PRD_RAWWRITE`; guard only clipboard | R6, R10; AR-26, AR-31 |
| ST-22 | Nonvolatile source | `StoreNV` tens length; `SizeNVData`; value/delete | R6; AR-27, AR-39 |
| ST-23 | bsdsocket source | `127.0.0.1`, loop send/recv, close | R7; AR-28 |
| ST-24 | AmigaGuide source | keep `NewAmigaGuide`; message result; Workbench start | R7; AR-29 |
| ST-25 | `openDiskFont` on 1.3 | `diamond.font` size 12, height 12 | R8; AR-30 |
| ST-26 | Registry closeout | 20 in-scope done / 21 total; baselines corrected; RTG out | R13; AR-3, AR-39 |
| ST-27 | Gate scripts | stage/case/S removal/Classes/AddDataTypes; full lines; cleanup | R9, R12; AR-7 |
| ST-28 | Full verify | `make check`, `make verify`, `make sdk-registry` exit 0 | R15; AR-33 |
| ST-29 | Mutation checks | Removing each call breaks at least one spec/impl test | R12; AR-33 |
| ST-30 | Appendix stale text | Each superseded “deferred/not part” statement is corrected in its section | R14; AR-34 |

## Implementation Test Cases

| ID | Pin |
|---|---|
| IT-1 | Module source/export tables and profile rejection lists |
| IT-2 | Optional suffix after the 2.0 suffix; 1.3 emit has neither |
| IT-3 | Semaphore allocation/init/free, no release-at-drop |
| IT-4 | Hook `h_Data` scalar, trampoline, null-safe free |
| IT-5 | Tag `TAG_DONE`, `tagPresent`, date handle, pool refcount and read/write bounds |
| IT-6 | gadtools `CreateContext`/`CreateGadgetA`, label/TextAttr copies, list-before-window |
| IT-7 | ASL drawer/file split, path join, cancel, `FreeAslRequest` |
| IT-8 | BOOPSI `WHEEL_Screen`, geometry, attach/refresh, hue set/get, dispose |
| IT-9 | ReadArgs template count/kinds, `DOS_RDARGS`, `FreeArgs`/array order |
| IT-10 | Datatypes `PDTA_BitMapHeader`/`PDTA_NumColors`, `dtDraw`, fixture path |
| IT-11 | Region bounds/clip borrow, clear-before-free, no double free |
| IT-12 | EClock `UNIT_ECLOCK`, hi/lo, frequency, close |
| IT-13 | Lowlevel type mask, port range, library version |
| IT-14 | Commodities `NB_VERSION`, port lifetime, duplicate |
| IT-15 | Locale fields/copies, default name |
| IT-16 | ARexx `ClearRexxMsg`, created/received/reclaim, reply exactly once |
| IT-17 | Clipboard current IDs, `CMD_UPDATE`, read bound, retry |
| IT-18 | Serial/parallel SETPARAMS, printer union/PRD_RAWWRITE, close/reap |
| IT-19 | Nonvolatile tens length, 23-byte round-trip, free/size |
| IT-20 | Socket fd ownership, loop send/recv, close |
| IT-21 | AmigaGuide struct/copies lifetime, message reply, close |
| IT-22 | Diskfont gate uses `DF1:C/Assign`, `diamond.font` 12, writable ADF |
| IT-23 | Clipboard-only generic guard; shared shim has no 2.0 symbols; the existing 1.3 forbidden-call scan is scoped to the shared shim |
| IT-24 | Gate scripts pin media, ROM SHA-1, model/memory, handshakes, capture, cleanup |
| IT-25 | Registry acceptance/evidence values and coverage count |

## Runtime Gate Matrix

| Gate | Machine | Rows | Lines |
|---|---|---|---|
| `sdk-a500-diskfont` | A500/KS1.3 | TXT-003 | `diskfont: ok h 12` |
| `sdk-a600-addons` | A600/KS2.05 | EXE-007/010/011, INT-006/007, DOS-004, TXT-005, AUD-008, WBK-003/004/005/006/007 | `exec`, `args`, `regions`, `eclock`, `gt`, `asl`, `cmd`, `locale`, `rexx`, `clip`, `ser`, `par`, `prt` |
| `sdk-a600-ks31-addons` | A600/KS3.1 | AUD-006, WBK-010 | `ll: ok v 40 p0 0 p1 0`; `nv: ok n 23` |
| `sdk-a1200-ks30-addons` | A1200/KS3.0 | EXE-007 (pool), INT-008 | `pool: ok 1`; `boopsi: ok hue 42`; golden `a1200-ks30-boopsi.png` |
| `sdk-a1200-addons` | A1200/KS3.1 | DOS-006, OPT-002, OPT-003 | `dt`, `net`, `guide`; optional `a1200-datatypes.png` |

## Commands

| Command | Purpose |
|---|---|
| `make check` | `spec/check.sh`, registry validation, Rust suites |
| `make sdk-a500-diskfont` | A500 diskfont gate |
| `make sdk-a600-addons` | A600 2.05 services/intuition gate |
| `make sdk-a600-ks31-addons` | A600 3.1 lowlevel/nonvolatile gate |
| `make sdk-a1200-ks30-addons` | A1200 3.0 pool/BOOPSI gate |
| `make sdk-a1200-addons` | A1200 3.1 datatypes/network/guide gate |
| `make verify` | Full acceptance, all new gates |
| `make sdk-registry` | Registry validation and coverage |

## Test Integrity Rules

- Each new export has a compile-only spec case; removing the export or wrapper breaks it.
- Each new shim call has a text pin; removing the call breaks it.
- Each runtime gate clears its HD and polls a fresh handshake; a stale file cannot pass.
- A capture golden is accepted only from a `CAPTURE_GOLDEN=1` run after the report line passes.
- The registry validator rejects `done` rows without acceptance/evidence.
- The final review verifies that each assertion fails without the fix (mutation check).
- The m68k shim cannot be linked into host ASan; memory safety is reviewed statically and through
  host record models and m68k runtime gates. [AR-40]

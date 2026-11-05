# Amiga SDK Bundle 5: DOS and Assets

> **Status**: Done (13/13 tasks; make verify green)
> **Created**: 2026-11-04
> **Bundle**: `5-dos-assets` (core; min OS 1.3; OCS/68000/512K)
> **Tracking**: [issue #6](https://github.com/OpenLorraine/clorraine/issues/6)
> **Registry**: [coverage.md](../../docs/amiga-sdk/coverage.md)
> **CodeOps Artifact Schema**: 1

## Purpose

Add the DOS file, directory, path, and volume surface, a pure IFF/ILBM parser,
and an ILBM image viewer that runs on the real A500/Kickstart 1.3 machine.
The viewer loads a committed ILBM from the acceptance ADF, applies its palette,
draws a caption layer, and holds a deterministic frame for capture.

`SDK-DOS-004` (ReadArgs, 2.0+) and `SDK-DOS-006` (datatypes.library, 3.0+) are
outside the A500 baseline and are explicitly deferred.

## Documents

| Document | Contents |
|---|---|
| [00-ambiguity-register.md](00-ambiguity-register.md) | Gate register; all items resolved |
| [00-preflight-report.md](00-preflight-report.md) | Independent audit; all critical/major resolved |
| [01-requirements.md](01-requirements.md) | Scope, requirements, acceptance |
| [02-current-state.md](02-current-state.md) | Existing surface and gaps |
| [03-01-iff-parser.md](03-01-iff-parser.md) | Pure `std.iff` chunk and ILBM design |
| [03-02-dos-surface.md](03-02-dos-surface.md) | `std.amiga.dos` file/lock/path/volume design |
| [03-03-ilbm-loader-viewer.md](03-03-ilbm-loader-viewer.md) | Loader, fixture, viewer, smoke |
| [05-findings.md](05-findings.md) | Runtime, review, and closeout evidence |
| [07-testing-strategy.md](07-testing-strategy.md) | Specification cases and verification |
| [99-execution-plan.md](99-execution-plan.md) | Phases and task checklist |

## Definition of done

1. `std.iff` parses ILBM headers, palettes, uncompressed and ByteRun1 bodies,
   and decodes plane data into chunky pixel indices.
2. `std.amiga.dos` provides seek/position/length, locks and path information,
   directory create/delete, file delete/rename, current/parent directory, and
   volume information.
3. `std.amiga.ilbm.loadIlbm` returns a `Bitmap` and fills the caller's palette.
4. `examples/amiga/imageviewer.clo` runs on FS-UAE A500/KS1.3 and prints
   `img: ok 320x200x4 hash <N> dos 255 vol 1`; the held frame and line
   crops are committed.
5. `make sdk-a500-assets` passes inside `make verify`; the hello, Boing, sprite,
   paddle, and text gates stay green.
6. `make check`, `make verify`, and `make sdk-registry` pass.
7. `SDK-DOS-004` and `SDK-DOS-006` are `deferred` with their min-OS reason.
8. Issue #6 is closed and the roadmap/programme show Bundle 5 done.

## Specialist Agents

None as a project-local specialist. The bundle adds embedded standard-library
sources and C shim code, not compiler semantics, ownership lowering, IR, or
emission. The phase review follows the AGENTS.md packet for a drop/lifetime
surface: an independent correctness/memory-safety pass with a blast-radius step
(every lock/path/volume call site and every error path) and a test-integrity
step (each assertion fails without its fix). The pure parser host suite runs
under AddressSanitizer through `make asan`; the host ASan auditor cannot execute
the m68k shim, so the A500 runtime gates cover the target DOS paths. The
`analyze-agents` check on 2026-11-04 recorded `None`.

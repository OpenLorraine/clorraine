# Amiga SDK Bundle 4: Text, Fonts, Layers

> **Status**: Done (9/9 tasks; make verify green)
> **Created**: 2026-11-04
> **Bundle**: `4-text-layers` (core; min OS 1.3; OCS/68000/512K)
> **Tracking**: [issue #5](https://github.com/OpenLorraine/clorraine/issues/5)
> **Registry**: [coverage.md](../../docs/amiga-sdk/coverage.md)
> **CodeOps Artifact Schema**: 1

## Purpose

Add font open/close, text drawing and measurement, diskfont.library, layers
with clipping, and a scrolling text/layers demo on the real A500/KS1.3 machine.
The demo opens the ROM Topaz font, measures `Clorraine layers` at 128x8, and
draws a clipped title layer and a scrolling text layer over a custom display.
The diskfont and 2.0+ regions rows are explicitly deferred.

## Documents

| Document | Contents |
|---|---|
| [00-ambiguity-register.md](00-ambiguity-register.md) | Gate register; all items resolved |
| [01-requirements.md](01-requirements.md) | Scope, requirements, acceptance |
| [02-current-state.md](02-current-state.md) | Existing surface and gaps |
| [03-01-fonts-text.md](03-01-fonts-text.md) | `std.amiga.text` design |
| [03-02-layers-demo.md](03-02-layers-demo.md) | Layers, scrolling demo, smoke |
| [05-findings.md](05-findings.md) | Review and runtime evidence |
| [07-testing-strategy.md](07-testing-strategy.md) | Specification cases and verification |
| [99-execution-plan.md](99-execution-plan.md) | Phases and task checklist |

## Definition of done

1. Font, text draw/measure, and layers/clipping are done with evidence.
2. Diskfont is done if the A500 run opens a disk font; otherwise it is
   explicitly deferred with the bare-ADF reason.
3. Diskfont and regions/damage are explicitly deferred with evidence.
4. `examples/amiga/scroller.clo` runs on FS-UAE A500/KS1.3 and prints
   `text: ok width 128 height 8 layer 160x64 checks 510`.
5. The held display and line crops are committed.
6. `make check`, `make verify`, `make sdk-registry`, and the text smoke pass.
7. The hello, Boing, sprite, and paddle gates stay green.
8. Issue #5 is closed and the roadmap/programme show Bundle 4 done.

## Specialist Agents

None as a project-local specialist — the bundle adds embedded standard-library
sources and C shim code, not compiler semantics, ownership lowering, IR, or
emission. The new shim has font/layer lifetime paths; the phase review includes
an independent correctness/memory-safety pass. The host ASan auditor cannot
execute the m68k shim; the A500 runtime gates cover the target path. The
`analyze-agents` check on 2026-11-04 recorded `None` with 59 plan folders in
`codeops/specialist-check.json`.

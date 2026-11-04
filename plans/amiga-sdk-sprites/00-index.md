# Amiga SDK Bundle 2: Sprites and Collision

> **Status**: Done (8/8 tasks; make verify green)
> **Created**: 2026-11-04
> **Bundle**: `2-sprites` (core; min OS 1.3; OCS/68000/512K)
> **Tracking**: [issue #3](https://github.com/OpenLorraine/clorraine/issues/3)
> **Registry**: [coverage.md](../../docs/amiga-sdk/coverage.md)
> **CodeOps Artifact Schema**: 1

## Purpose

Add the eight OCS hardware sprite channels, pixel upload, movement,
show/hide, attached 16-color pairs, hardware collision detection, and a small
animation object. The acceptance is a sprite demo on the real A500/KS1.3
machine with a deterministic collision result.

Sprites are global custom-chip DMA channels attached to an Intuition screen
viewport. The module is independent of the Bundle 1 `Display`; the sprite demo
uses the Intuition screen surface.

## Documents

| Document | Contents |
|---|---|
| [00-ambiguity-register.md](00-ambiguity-register.md) | Gate register; all items resolved |
| [01-requirements.md](01-requirements.md) | Scope, requirements, acceptance |
| [02-current-state.md](02-current-state.md) | Existing surface and gaps |
| [03-01-sprite-api.md](03-01-sprite-api.md) | `std.amiga.sprite` design |
| [03-02-demo-collision.md](03-02-demo-collision.md) | Demo, collision, and smoke design |
| [05-findings.md](05-findings.md) | Review and runtime evidence |
| [07-testing-strategy.md](07-testing-strategy.md) | Specification cases and verification |
| [99-execution-plan.md](99-execution-plan.md) | Phases and task checklist |

## Definition of done

1. The six registry rows move to `done` with evidence.
2. `examples/amiga/sprite-demo.clo` runs on FS-UAE A500/KS1.3, shows the
   attached pair and single sprite, and prints `sprite-demo: collision 0x1000`.
3. The live display and collision-line crops are committed.
4. `make check`, `make verify`, `make sdk-registry`, and the sprite smoke pass.
5. The existing hello and Boing A500 gates stay green.
6. Issue #3 is closed and the roadmap/programme show Bundle 2 done.

## Specialist Agents

None as a project-local specialist — the bundle adds an embedded standard
library and C shim code, not compiler semantics, ownership lowering, IR, or
emission. The new shim has chip-memory allocation paths and was reviewed by
the independent correctness/memory-safety pass; the host ASan auditor cannot
execute the m68k shim. The `analyze-agents` check on 2026-11-04 recorded
`None` with 57 plan folders in `codeops/specialist-check.json`; the existing
catalog reviewers cover the area.

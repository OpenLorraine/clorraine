# Amiga SDK Bundle 9: Optional Add-ons and Advanced Closeout

> **Feature**: Close the remaining 20 Amiga SDK capabilities: advanced Exec/utility, Intuition, DOS, graphics, input, Workbench services, networking, and AmigaGuide, using the licensed Amiga Forever Workbench media.
> **Status**: Done — 30/30; 20/21 capabilities; RTG #11 and symbol-collision #12 tracked
> **Created**: 2026-11-06 02:09
> **Last Updated**: 2026-11-06 07:30
> **Implements**: —
> **CodeOps Artifact Schema**: 1

## Overview

Bundle 9 is the closeout bundle for the Amiga SDK programme. It implements the 12 capabilities
whose registry notes already point at Bundle 9, the two `9-optional` rows, and the six Workbench
services deferred from Bundle 8. The licensed Amiga Forever disc supplied the missing Workbench
1.3.5, 2.04, 2.1, 3.0, and 3.1 media; the media is mounted read-only at `/amiga` and is never
committed.

The bundle follows the established stdlib pattern: one module per library/domain, opaque handles
with exactly-once `freedBy` cleanup, typed wrappers over a new optional C shim suffix, and headless
FS-UAE gates. Network uses FS-UAE’s built-in `bsdsocket.library`; real hardware still needs a
third-party stack. RTG remains out of scope and is tracked by issue #11. Runtime/generated C symbol collisions are a documented limitation (issue #12), not a source-name rule.

## Minimum-Sufficient Baseline

**Original goal:** finish every open Amiga SDK capability with the newly available licensed
Workbench media, without changing the language, grammar, diagnostics, or ABI.

**Smallest viable design:** extend the existing stdlib/shim/gate pattern with thin modules and one
optional shim suffix. Reuse the existing A500/A600/A1200 directory-HD gates, `xdftool` media
extraction, `workbench-start.c`, and the A500 GUI input harness. No new framework, service, or
external dependency.

**Excluded machinery:** RTG (issue #11), third-party TCP/IP stacks (FS-UAE emulation for the
gate), ClassAct/custom BOOPSI classes, printer-driver installation, language-level memory pools,
and capturing closures. See the register for the named exclusions.

**Approved complexity:** none beyond the existing project patterns. The new suffix file is the
same emission mechanism Bundle 8 introduced.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00 | [Index](00-index.md) | This document — overview and navigation |
| 01 | [Requirements](01-requirements.md) | Capability, compatibility, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | What exists, what is missing, and the media matrix |
| 03-01 | [Exec and Utility](03-01-exec-utility.md) | Semaphores, hooks, tags, date, and utility pools |
| 03-02 | [Intuition Advanced](03-02-intuition-advanced.md) | gadtools, ASL, and BOOPSI |
| 03-03 | [DOS, Graphics, Input](03-03-dos-graphics-input.md) | ReadArgs, datatypes, regions, EClock, ReadJoyPort |
| 03-04 | [Workbench Services](03-04-workbench-services.md) | Commodities, locale, ARexx, clipboard, devices, nonvolatile |
| 03-05 | [Network and AmigaGuide](03-05-network-guide.md) | bsdsocket and AmigaGuide |
| 03-06 | [Demos, Gates, Media](03-06-demos-gates-media.md) | Media installation, demos, and runtime gates |
| 07 | [Testing Strategy](07-testing-strategy.md) | ST cases and verification matrix |
| 99 | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Usage Examples

```clorraine
import { Semaphore, createSemaphore, obtainSemaphore, releaseSemaphore,
         freeSemaphore } from std.amiga.semaphore;

func guarded(): i32 {
    let sem: Semaphore? = createSemaphore();
    if (sem == null) { return 1; }
    let s: Semaphore = sem;
    obtainSemaphore(s);
    releaseSemaphore(s);
    freeSemaphore(s);
    return 0;
}
```

```clorraine
import { Args, readArgsFrom, argNumber, argText, freeArgs } from std.amiga.args;
import { String, stringOf, bytes } from std.text;

func parsed(): i32 {
    let args: Args? = readArgsFrom("FILE/A,NUM/N,VERBOSE/S", 3,
        bytes(stringOf("FILE=x NUM=7 VERBOSE")));
    if (args == null) { return 1; }
    let a: Args = args;
    let n: i32 = argNumber(a, 1);
    freeArgs(a);
    return n;
}
```

### Key Decisions

| Decision | Outcome | AR |
|---|---|---|
| Bundle scope | All 20 open rows; RTG excluded | AR-1, AR-5 |
| Tracking | Reassign 20 rows to `9-optional`; regenerate issue #10 | AR-2, AR-3 |
| Baseline corrections | EXE-007 3.0, DOS-006 3.1, AUD-006/WBK-010 3.1; floor 1.3/OCS | AR-4, AR-39 |
| Network gate | FS-UAE built-in `bsdsocket.library`, host `127.0.0.1` | AR-6, AR-28 |
| Media | Stage ADFs, copy named trees, `LIBS: ... ADD Classes`, `AddDataTypes` | AR-7, AR-36 |
| Module split | 19 modules, one per library/domain | AR-8 |
| Shim | New optional suffix after the 2.0 suffix | AR-9 |
| Profiles | New modules on `hosted-amiga` only; no chapter-15 edit | AR-10, AR-34 |
| Hooks | Capture-less `func(): void`, `i32` payload, `hookSetResult` | AR-11 |
| Pools | Distinct `std.amiga.pool`, refcounted, safe `u8[]` access | AR-12 |
| Gate matrix | A500 1.3, A600 2.05, A600 3.1, A1200 3.0, A1200 3.1 | AR-33 |
| Coverage | 20 in-scope done / 21 total including RTG out-of-scope | AR-39 |
| Appendices | Additive only; frozen chapters untouched | AR-34, AR-35 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
|---|---|---|---|
| `correctness-reviewer` | reviewer | Phase and closeout review: contract, ownership, test integrity, docs | AR-11, AR-12, AR-19 |
| `memory-safety-auditor` | auditor (read-only) | Static ownership review, host models, and m68k runtime probes; no host ASan for the m68k shim | AR-40 |
| `emission-integrity-auditor` | auditor (read-only) | Base/head corpus diff, strict C build, suffix placement, macro collisions | AR-9, AR-10 |

_Detection evidence: `compiler/runtime/shim_hosted_amiga_optional.c` is a new suffix; the bundle
adds C shim ownership wrappers and gate scripts. No semantics reviewer is needed because no
language rule changes._

## Related Files

| Path | Role |
|---|---|
| `stdlib/std.amiga.*.clo` | New modules and the existing `std.amiga.text` diskfont path |
| `compiler/runtime/shim_hosted_amiga.c` | Shared 1.3-safe changes: clipboard guard and event-id fallback |
| `compiler/runtime/shim_hosted_amiga_20.c` | Existing Workbench/icon/expansion suffix |
| `compiler/runtime/shim_hosted_amiga_optional.c` | New Bundle 9 suffix |
| `compiler/src/profile.rs` | Suffix concatenation and module tests |
| `compiler/src/stdlib.rs` | Module source/export tables |
| `compiler/src/codegen.rs` | Appends the complete suffix after the generated program |
| `compiler/tests/amiga_optional_spec_test.rs` | New specification suite |
| `compiler/tests/amiga_optional_impl_test.rs` | New implementation suite |
| `examples/amiga/*.clo` | Bundle 9 demos |
| `docker/scripts/*-smoke.sh` | A500 diskfont, A600 2.05/3.1, A1200 3.0/3.1 gates |
| `docker/scripts/workbench-media.sh` | Read-only ADF staging helper |
| `docs/amiga-sdk/registry.tsv`, `bundles.tsv`, `coverage.md` | Registry closeout |
| `spec/appendix-stdlib.md`, `spec/appendix-amiga.md` | Additive documentation |

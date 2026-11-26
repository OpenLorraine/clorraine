# Amiga Fault-Exit Cleanup Implementation Plan

> **Feature**: Release every AmigaOS resource on every hosted-amiga exit; handle worker-task faults; Ctrl-C in the shim waits
> **Status**: Planning Complete
> **Created**: 2026-11-26
> **Implements**: none (standalone plan; no RD) — closes DEF-582, DEF-586, DEF-604 and theme issue #40 (findings #54, #58, #76, and ASM-WS5-020 in #92)
> **CodeOps Artifact Schema**: 1

## Overview

A Clorraine program on the hosted-amiga profiles can leave AmigaOS damaged when it ends abnormally. A runtime fault or `panic` calls libnix `exit(20)` and the runtime releases nothing. An interrupt server installed by the program then keeps running in unloaded code; the independent assessment reproduced the resulting machine crash on the A500 and the A1200. Worker tasks, public ports, locks, chip memory, devices, and library opens leak the same way. Even a clean exit never closes the library bases the runtime opened lazily. A fault inside a `createTask` worker hangs the machine. No shim wait responds to Ctrl-C.

This plan adds one runtime mechanism shared by every exit. A registry records the handles the program owns. Fixed exit phases remove interrupt servers and worker tasks, release the registry in reverse order, restore the runtime's own chip writes, and close the libraries the runtime opened. An `atexit` hook runs the phases, so a normal return, a fault, a panic, and a Ctrl-C all release the same way. Workers report their faults through the parent. Ctrl-C ends the breakable waits with `***Break` and rc 20, matching libnix.

The language rule is unchanged: a fault still skips the cleanup ladder and user drops (spec/05 §15). The release is runtime-internal, so `spec/` is untouched (AR-6).

## Minimum-Sufficient Baseline

**Original goal:** Release every OS resource the hosted-amiga runtime acquired, on every exit (normal, fault, panic, Ctrl-C). Handle worker faults without DOS calls or `exit()`. Add Ctrl-C to the blocking waits (issue #40).
**Smallest viable design:** A shim-internal registry for owned handles, reusing each kind's existing release logic through typed thunks. Interrupt servers, workers, chip state, and library bases go through fixed exit phases with the static storage they already have. One `atexit` hook, installed on first use. There is no compiler, API, or spec change ([03-01](03-01-exit-registry.md)).
**Excluded machinery:** These were considered and not taken:
- per-kind static lists with a hand-written release-all (larger, AR-8);
- a generated-`main` change to force rc 20 (AR-10);
- a `breakRequested()` API (AR-5);
- a committed C driver for the A500 gate (AR-17).
**Approved complexity:** AR-8 (the simplified shim resource registry; approval evidence in the register).

## Planning Coverage

> **Coverage**: 4/4 clusters · 12/12 categories · 36 candidates · 6 residual limits · Effort: high/high/high/xhigh→applied as requested · Independence: uniform-effort
> **Freshness**: Coverage as of 2026-11-26; artifacts issue #40, #54, #58, #76, finding texts WS5-shims.md, shim sources at 1462063e

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | worker exit, Ctrl-C scope, chip state, in-flight messages, R5/R9 boundary, docs | shim fault/task/window/display/port paths, stdlib exec/custom/arexx, spec §15, docs | per-deleter exit safety; vamos break delivery | 8 | libnix `__chkabort` inconclusive here (resolved by C4) | high |
| C2 Behavior and edges | re-entry, ordering, unregister points, Ctrl-C mechanics, worker faults, registration failure | shim lifetimes, libnix `atexit.c`/`ncrt0.S` sources | no execution; optional-module deleters one by one | 10 | atexit re-run on nested `exit` read from source | medium-high |
| C3 Technical and data | record storage/key, typed release, worker record, libc-owned resources, size, naming | shim, stdlib `take` consumers, impl pins, perf guards, SDK libnix/vbcc symbols, GCC 16 cast check | vbcc atexit order; prototype size | 9 | cast-function-type error reproduced | high |
| C4 Risk and stakeholders | Exec safety, libnix Ctrl-C, licensed media, size budget, roadmap interplay | shim, libnix disassembly, gate scripts, bench harness, roadmap | no FS-UAE/vamos run; other gate goldens assumed unchanged | 9 | libnix `raise` prints `***Break` and `exit(20)` | high |

**Source note:** merged into AR-8 (challenger + C3-305), AR-9 (C1-001, C2-001..004, C2-008, C3-301..305, C3-307, C4-005..007), AR-10 (C1-001, C2-007), AR-11 (C4-002), AR-12 (C1-002, C2-005, C3-306, C4-001), AR-13 (C2-006, C1-007, C4-003), AR-14 (C1-003, C2-010), AR-15 (C1-004, C2-009, C3-302), AR-16 (C1-006), AR-17 (C1-005, C4-004, C3-309), AR-18 (C3-308, C4-008), AR-19 (C1-008, C3-309). Omitted in strict scope: C4-009 (optional: R5/R12 interface contracts). Residual limits: no execution under vamos/FS-UAE during planning; vbcc exit ordering unverified; libnix `__exitlibraries` close behaviour for a shim-opened `UtilityBase` unverified (mitigated by AR-9's "shim-opened only" rule).

## Document Index

| # | Document | Description |
| --- | --- | --- |
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (21 rows) and the coverage map |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Requirements and scope (owning document; no RD) |
| 02 | [Current State](02-current-state.md) | Fault path, exit order, handles, waits, gates at HEAD |
| 03-01 | [Exit registry](03-01-exit-registry.md) | Registry, hook, phases, re-entry, library bases, chip state, in-flight messages |
| 03-02 | [Registration sites](03-02-registration-sites.md) | Per-kind table: storage and unregister points |
| 03-03 | [Worker faults](03-03-worker-faults.md) | Worker list, fault record, report at join, exit removal |
| 03-04 | [Ctrl-C](03-04-ctrl-c.md) | Break routine and breakable waits |
| 03-05 | [Gates and docs](03-05-gates-and-docs.md) | Probes, `vamos` leg, A500 gate, docs, bench |
| 07 | [Testing Strategy](07-testing-strategy.md) | ST-1..ST-19 |
| 99 | [Execution Plan](99-execution-plan.md) | 6 phases, 55 tasks |

## Quick Reference

### Usage Examples

```clorraine
// Before: a fault after addHandler left the VBlank server in freed code.
// After: the runtime removes it, closes the port, and the program still exits rc 20.
let token: HandlerToken = addHandler(onTick);
let port: MsgPort? = createPublicPort("ClFaultCleanup");
println($"{xs[pick(5)]}");      // error E11211: index out of range — server and port released
```

### Key Decisions

| Decision | Outcome |
| --- | --- |
| Scope | Every shim acquisition on every exit (AR-1, AR-2) |
| Mechanism | Simplified registry plus fixed phases (AR-8, AR-9) |
| Worker faults | Re-raised by the parent at join; unreported ones printed at exit (AR-3, AR-10) |
| Ctrl-C | `***Break`, rc 20; listed waits only; `waitSignals` mask rule (AR-5, AR-11–13) |
| Chip state | Shim-owned writes only (AR-14) |
| In-flight messages | Kept for foreign holders; freed on own ports (AR-15, AR-21) |
| Boundaries | Interrupt-context faults, R5 and R9 items out (AR-4, AR-16) |
| Evidence | `vamos` leg on both profiles plus an A500/KS1.3 gate; bench guard (AR-7, AR-17, AR-18) |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| `amiga-platform-reviewer` (existing) | reviewer | every phase review (Phases 1–6), together with the correctness reviewer (AGENTS.md dispatch for Amiga platform changes) | AR-19 |

**New candidates: None.** The capability-gap check found no gap that a new standing specialist
would close.

_Detection evidence:_
- The Kickstart 1.3–3.1 availability matrix, m68k C crossing rules, and FS-UAE gate mechanics this
  plan needs are already the brief of `amiga-platform-reviewer` (`codeops/specialists/amiga-platform-reviewer.md`;
  `.opencode/agents/`).
- Host AddressSanitizer does not instrument the m68k shim (AGENTS.md Testing), so the
  memory-safety auditor adds no evidence here.
- The emitted C changes only through the prepended shim text; no IR or codegen change.

## Related Files

- Modified:
  - `compiler/runtime/shim_hosted_amiga.c`, `shim_hosted_amiga_20.c`, `shim_hosted_amiga_optional{,_ui,_services,_net,_rtg}.c`
  - `stdlib/std.amiga.{exec,interrupt,dos,timer}.clo` (doc comments)
  - `docker/scripts/run-fault-amiga.sh`, `Makefile`, `AGENTS.md`, `docs/known-limitations.md`, `site/project/verification.md`
  - `docker/scripts/a500-gui-smoke.sh` (waits for the ADF to settle before stopping FS-UAE; AR-24)
  - the impl-test pins in `compiler/tests/amiga_{exec,workbench,intuition}_impl_test.rs`
- Created:
  - `compiler/tests/amiga_fault_cleanup_{spec,impl}_test.rs`
  - `docker/scripts/a500-fault-cleanup-smoke.sh`, `docker/scripts/fault-cleanup-walker.c`
  - `examples/robustness/fault-cleanup/*.clo`

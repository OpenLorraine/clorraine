# Execution Plan: Amiga shim crash-safety

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-27 08:36 (preflight fixes applied; no task started)
> **Progress**: 0/39 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

The plan has five phases:
1. Open Intuition on demand, and extend the A1200 gate with the probe mechanism (DEF-587).
2. Finish the clipboard read (DEF-588).
3. Release borrowed clip regions and guide messages safely (DEF-593).
4. Clip screen text (DEF-585).
5. Close out: docs, bench evidence, `make verify`.

Feature phases 1–4 follow the specification-first order. Phase 5 adds no behaviour.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Intuition on demand; A1200 probe mechanism | 9 |
| 2 | Clipboard read | 6 |
| 3 | Clip regions and guide messages | 10 |
| 4 | Screen text clipping | 9 |
| 5 | Closeout | 5 |

**Total: 39 tasks across 5 phases.**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

Rules for every task:
- Source, tests, and doc comments restate rationale in plain language. They never cite AR/ST/ASM/
  DEF identifiers or `plans/` (AGENTS.md).
- Probe and scratch output goes under `build/tmp`.
- Review dispatch per phase: correctness reviewer plus `amiga-platform-reviewer`
  ([00-index §Specialist Agents](00-index.md#specialist-agents)).
- FS-UAE gates run on private Xvfb displays; check that no interactive FS-UAE window is open on the
  desktop before a desktop-capturing gate (`sdk-a500-exec`).

---

## Phase 1: Intuition on demand; A1200 probe mechanism

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: medium — two small shim fixes plus a gate extension that later phases reuse.

### Step 1.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-1, ST-2, ST-4 · [03-04 §A1200 leg](03-04-gates-and-docs.md#a1200-leg-sdk-a1200-addons) · AR-2, AR-13, AR-14

- [ ] 1.1.1 [spec-author] Write the probes for ST-1, ST-2, ST-4 — `examples/robustness/crash-safety/{alert-first,pubscreen-first,pubscreen-wb}.clo`
- [ ] 1.1.2 [spec-author] Write the `vamos` spec test for ST-1 and ST-2 (both profiles where listed, the Intuition-off option, the leak pattern, SDK skip) — `compiler/tests/amiga_crash_safety_spec_test.rs`
- [ ] 1.1.3 [spec-author] Extend the A1200 gate: build the probes in the `sdk-a1200-addons` recipe, install them, run them after the existing legs, the `done.txt` marker with the bounded settle, and the ST-4 check — `docker/scripts/a1200-addons-smoke.sh`, `Makefile`
- [ ] 1.1.4 Red phase: `cargo test --test amiga_crash_safety_spec_test` and `make sdk-a1200-addons`; record ST-1/ST-2 failing (A6=0 crash) and ST-4 failing, with the existing legs still passing

### Step 1.2: Implementation

**Reference**: [03-01 §Intuition on demand](03-01-intuition-and-clipboard.md#intuition-on-demand) · AR-2

- [ ] 1.2.1 Open Intuition on demand in `cl_show_alert` — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 1.2.2 Open Intuition on demand in `cl_save_public_screen`, with the V36 check — `compiler/runtime/shim_hosted_amiga_20.c`
- [ ] 1.2.3 Green phase: ST-1, ST-2, ST-4 pass; the existing A1200 legs still pass

### Step 1.3: Implementation tests and hardening

**Reference**: [07 §Implementation Tests](07-testing-strategy.md#implementation-tests-edge-cases-internals)

- [ ] 1.3.1 Write the Intuition impl tests; update any pin whose text intentionally changed — `compiler/tests/amiga_crash_safety_impl_test.rs`
- [ ] 1.3.2 Phase gate: `make check`, `make sdk-a1200-addons`, `make sdk-a1200-workbench`

**Deliverables**:
- [ ] ST-1, ST-2, ST-4 green; the A1200 gate runs probes after its legs with a completion marker

**Task verify**: `make check` (4442 tests observed 2026-11-27)
**Verify (gate)**: `make check && make sdk-a1200-addons && make sdk-a1200-workbench`

---

## Phase 2: Clipboard read

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: medium — a device-protocol fix with an autodoc-derived sequence; target-proven.

### Step 2.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-5, ST-6 · AR-10, AR-11, AR-13

- [ ] 2.1.1 [spec-author] Write the ST-5 and ST-6 probes and their A1200 gate checks (they run after `pubscreen-wb`; Phase 3 inserts its probes before them and Phase 4 appends `text-offscreen` last), and the ST-5 run in the A600 gate (build, install, one startup line after the existing legs, the `clipiff.txt` wait and check) — `examples/robustness/crash-safety/clip-{iff,raw}.clo`, `docker/scripts/a1200-addons-smoke.sh`, `docker/scripts/a600-addons-smoke.sh`, `Makefile`
- [ ] 2.1.2 Red phase: `make sdk-a1200-addons` and `make sdk-a600-addons`; record ST-5/ST-6 failing on the A1200 (a write after a read never returns; `done.txt` missing) with the other checks reporting as before, and record ST-5's base outcome on the A600 (a guard, red not required)

### Step 2.2: Implementation

**Reference**: [03-01 §Clipboard read](03-01-intuition-and-clipboard.md#clipboard-read) · AR-10, AR-11

- [ ] 2.2.1 Finish the read after the caller's `CMD_READ` (saved count, skip read, one past-the-end read) — `compiler/runtime/shim_hosted_amiga_optional_services.c`
- [ ] 2.2.2 Green phase: ST-5 (A1200 and A600) and ST-6 pass; if any write still hangs or the A600 reads differ from the A1200 reads, mark `[!]` and follow the AR-10 reopen triggers before any other change

### Step 2.3: Implementation tests and hardening

- [ ] 2.3.1 Write the clipboard impl tests; update any pin whose text intentionally changed — `compiler/tests/amiga_crash_safety_impl_test.rs`
- [ ] 2.3.2 Phase gate: `make check`, `make sdk-a1200-addons`, `make sdk-a600-addons`

**Deliverables**:
- [ ] ST-5, ST-6 green on A1200/KS3.1; ST-5 green on A600/KS2.05

**Verify (gate)**: `make check && make sdk-a1200-addons && make sdk-a600-addons`

---

## Phase 3: Clip regions and guide messages

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — touches the 1.3 layer free path and the exit-time release order.

### Step 3.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-7, ST-8, ST-3 (the `cl_res_each` part) · AR-6..AR-9, AR-13, AR-15

- [ ] 3.1.1 [spec-author] Write the ST-7 and ST-8 probes and their A1200 gate checks — `examples/robustness/crash-safety/{region-free,guide-late}.clo`, `docker/scripts/a1200-addons-smoke.sh`, `Makefile`
- [ ] 3.1.2 [spec-author] Add the `cl_res_each` half of ST-3 (hello and a program with no region or guide link no `cl_res_each`; `region-free` links it) — `compiler/tests/amiga_crash_safety_spec_test.rs`
- [ ] 3.1.3 Red phase: `make sdk-a1200-addons` and the spec test; record ST-7 (checksums differ or crash), ST-8 (stale type or crash), and ST-3's new half failing; ST-7's layer-free half is a guard (07 §Red-phase expectations)

### Step 3.2: Implementation

**Reference**: [03-03](03-03-borrowed-objects.md) · AR-6..AR-9

- [ ] 3.2.1 Add `cl_res_each` next to `cl_own_ports_hold` — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 3.2.2 Add the layer update flag (set on every `layerBeginUpdate`, cleared by `layerEndUpdate`, which still calls `EndUpdate(layer, TRUE)`) and `cl_layer_unclip`; in `cl_layer_free`, end an open update with `EndUpdate(layer, FALSE)`, then unclip, before `DeleteLayer` — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 3.2.3 Unclip every holding layer in `cl_free_region` (no `EndUpdate`); leave set/clear clip unchanged — `compiler/runtime/shim_hosted_amiga_optional.c`
- [ ] 3.2.4 Link each guide message to its guide; reply and orphan held messages in `cl_guide_close` — `compiler/runtime/shim_hosted_amiga_optional_net.c`
- [ ] 3.2.5 Green phase: ST-7, ST-8, ST-3 (this half) pass; the existing legs still pass

### Step 3.3: Implementation tests and hardening

- [ ] 3.3.1 Write the layer, region, and guide impl tests; update any pin whose text intentionally changed — `compiler/tests/amiga_crash_safety_impl_test.rs`
- [ ] 3.3.2 Phase gate: `make check`, `make run-fault-amiga`, `make sdk-a1200-addons`, `make sdk-a500-text`, `make sdk-a500-assets`, `make sdk-a500-fault-cleanup`

**Deliverables**:
- [ ] ST-7, ST-8 green; the 1.3 layer gates unchanged

**Verify (gate)**: `make check && make run-fault-amiga && make sdk-a1200-addons && make sdk-a500-text && make sdk-a500-assets && make sdk-a500-fault-cleanup`

---

## Phase 4: Screen text clipping

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: xhigh — new drawing code on the memory-safety boundary, Kickstart 1.3 metrics, blitter lifetime.

### Step 4.1: Specification tests

**Reference**: [07](07-testing-strategy.md) ST-9, ST-10, ST-3 (the `cl_text_draw_clipped` part) · AR-3..AR-5, AR-13

- [ ] 4.1.1 [spec-author] Write the ST-9 probe and its A1200 gate check (last in the startup-sequence) — `examples/robustness/crash-safety/text-offscreen.clo`, `docker/scripts/a1200-addons-smoke.sh`, `Makefile`
- [ ] 4.1.2 [spec-author] Write the ST-10 probe and its A500 gate case per 03-04 §A500 leg: build for `hosted-amiga-13` from the `crash-safety/` source path, `chip` column, the capture unpack, `cscr2png.py` on the host, the window comparisons, the two-colour reference check, and `python3`/`convert`/`compare` in the host-tool check — `examples/robustness/crash-safety/text-clip.clo`, `docker/scripts/a500-fault-cleanup-smoke.sh`
- [ ] 4.1.3 [spec-author] Add the `cl_text_draw_clipped` half of ST-3 — `compiler/tests/amiga_crash_safety_spec_test.rs`
- [ ] 4.1.4 Red phase: `make sdk-a1200-addons`, `make sdk-a500-fault-cleanup`, the spec test; record ST-9 failing (hang), ST-3's new half failing, and ST-10's base outcome with the AR-13 justification

### Step 4.2: Implementation

**Reference**: [03-02](03-02-text-clipping.md) · AR-3..AR-5

- [ ] 4.2.1 Add, after `SetFont`/`SetAPen`, the range and empty-text checks, the visible rectangle, the superset box walk (03-02 step 3), and the classification to `cl_text_draw`; fully inside keeps the old sequence — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 4.2.2 Add `cl_text_draw_clipped`: the culled run, the two-plane raster, the template blits, `WaitBlit`, the frees — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 4.2.3 Green phase: ST-9, ST-10, ST-3 pass; if ST-10 shows a pixel difference, mark `[!]` and follow the AR-4/AR-5 reopen triggers; a `chip` or `mem` difference follows the 03-04 `chip` column rule

### Step 4.3: Implementation tests and hardening

- [ ] 4.3.1 Write the text impl tests; update any pin whose text intentionally changed — `compiler/tests/amiga_crash_safety_impl_test.rs`
- [ ] 4.3.2 Phase gate: `make check`, `make sdk-a500-fault-cleanup`, `make sdk-a1200-addons`, `make sdk-a500-gui`, `make sdk-a500-text`, `make sdk-a1200-workbench`

**Deliverables**:
- [ ] ST-9, ST-10 green; the GUI and text goldens unchanged

**Verify (gate)**: `make check && make sdk-a500-fault-cleanup && make sdk-a1200-addons && make sdk-a500-gui && make sdk-a500-text && make sdk-a1200-workbench`

---

## Phase 5: Closeout

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: medium — docs, measurement, and the full gate.

### Step 5.1: Docs, evidence, and the plan gate

**Reference**: [03-04 §Docs, §Bench evidence](03-04-gates-and-docs.md#docs) · AR-15, AR-16

- [ ] 5.1.1 Update the doc comments per 03-04 §Docs — `stdlib/std.amiga.{text,intuition,workbench,regions,amigaguide,clipboard}.clo`
- [ ] 5.1.2 Update `docs/known-limitations.md` §4 and §5 (clipboard proof on A1200/KS3.1 and A600/KS2.05) and add the colour-font clip limit — `docs/known-limitations.md`
- [ ] 5.1.3 Update the `sdk-a1200-addons`, `sdk-a600-addons`, and `sdk-a500-fault-cleanup` rows and the Makefile comments — `AGENTS.md`, `Makefile`, `site/project/verification.md` if it names the probes
- [ ] 5.1.4 Bench evidence: `make bench-base COMMIT=<the commit before task 1.1.1>`, `make bench`, `make bench-compare` (empty `REQUIRE`), and the release `.text` of `gui` and `scroller` (`hosted-amiga-13`) at base and head; record them here and in 07
- [ ] 5.1.5 Plan gate: `make verify`

**Deliverables**:
- [ ] `make verify` green; bench comparison within the 1% guard; deltas recorded
- [ ] Docs updated

**Verify (gate)**: `make verify`

---

## Dependencies

```
Phase 1 (Intuition; A1200 probe mechanism)
    ↓
Phase 2 (clipboard)        Phase 3 (regions, guide)
    ↓                          ↓
Phase 4 (text clipping; uses both gates)
    ↓
Phase 5 (closeout)
```

Phases 2 and 3 depend only on Phase 1's gate mechanism; they run in order so the gate's startup
sequence grows one phase at a time.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ `make verify` passing, including the extended `sdk-a1200-addons` and `sdk-a500-fault-cleanup`
3. ✅ No warnings or errors (GCC `-Werror` on both CRTs; vbcc build leg)
4. ✅ No dead code: no unused helpers, fields, or probes
5. ✅ Memory safety: no safe call writes outside the screen bitmap or reaches a freed region, guide context, or held clip (ST-5..ST-10)
6. ✅ Documentation updated (03-04 §Docs)
7. ✅ Phase reviews passed (correctness reviewer + `amiga-platform-reviewer`)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)

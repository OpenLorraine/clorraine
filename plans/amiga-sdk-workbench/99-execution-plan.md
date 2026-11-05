# Execution Plan: Amiga SDK Bundle 8 — Workbench and Services

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-05 21:56
> **Progress**: 0/26 tasks (0%) — Ready
> **CodeOps Artifact Schema**: 1

## Overview

Six phases: the shared 2.0+ shim guard and public port, the icon library, the
Workbench application objects and messages, expansion/card/keymap, the demos
and A1200/A600 gates, then documentation, the full gate, the independent
review, and the bundle closeout. Specification tests come before
implementation in every phase.

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Foundation: 2.0 shim guard and public port | 4 |
| 2 | Icon library | 4 |
| 3 | Workbench application objects and messages | 4 |
| 4 | Expansion, card, and keymap | 4 |
| 5 | Demos, fixtures, and A1200/A600 gates | 6 |
| 6 | Documentation, full gate, review, closeout | 4 |

**Total: 26 tasks across 6 phases.**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of
> truth** for progress. Every task line appears exactly once in this document.
> The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the
>    Last Updated stamp after EVERY task — never batch updates. Only `[x]`
>    counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]`
>    task is resumed first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>`
>    on the same line. The plan lifecycle is `Ready`, `Executing`, `Done`, or
>    `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to
> keep the marks current means progress is invisible after crashes, context
> resets, or session handoffs.

---

## Phase 1: Foundation — 2.0 shim guard and public port

> **Reasoning**: high — a codegen macro gate and a new Exec ownership surface
> affect both hosted profiles.

### Step 1.1: Shared foundation

**Reference**: [03-03](03-03-expansion-card-keymap.md) · AR-10, R4, R9

- [ ] 1.1.1 Add the `CL_HOSTED_AMIGA_20` define to `compiler/src/codegen.rs` and guard the 2.0-only shim section; add a unit test that the define is emitted only for `hosted-amiga`, and verify a 1.3 and a 2.0 emit still cross-build
- [ ] 1.1.2 Add `createPublicPort(name)` to `stdlib/std.amiga.exec.clo`, `cl_create_public_port` to the shared shim, the export table, and `spec/appendix-stdlib.md` §13.2; update the exec module doc; add the specification case (both hosted profiles) and the drop/delete case
- [ ] 1.1.3 Add the Phase 1 implementation cases (public-port shim, delete routing, null/empty-name guards) to `compiler/tests/amiga_workbench_impl_test.rs`; run them green
- [ ] 1.1.4 Add the Phase 1 `amiga_workbench_spec_test.rs` public-port case and the IT-1/IT-2 unit-test scaffolding (module tables and profile gates); run the small suite green

**Verify**: `cargo test --lib --test amiga_exec_spec_test --test amiga_workbench_spec_test --test amiga_workbench_impl_test`

---

## Phase 2: Icon library

> **Reasoning**: medium — a new module and a simple owned handle.

### Step 2.1: `std.amiga.icon`

**Reference**: [03-02](03-02-icon-library.md) · AR-6, R2, R9

- [ ] 2.1.1 [spec-author] Add the icon surface and its host/1.3 rejection, `nix20` cross-build, and appendix checks to `amiga_workbench_spec_test.rs`; run red
- [ ] 2.1.2 Add `stdlib/std.amiga.icon.clo`, the icon shim section, the export table, and the `hosted-amiga`-only profile gate; run the spec green
- [ ] 2.1.3 Add the icon implementation cases (shim calls, default object, type range, drop) to `amiga_workbench_impl_test.rs`
- [ ] 2.1.4 Add the appendix §14.1 icon text and verify the icon spec/impl cases and the `nix20` cross-build

**Verify**: `cargo test --lib --test amiga_workbench_spec_test --test amiga_workbench_impl_test`

---

## Phase 3: Workbench application objects and messages

> **Reasoning**: high — new owned Workbench handles, a public message port, and
> a cross-process AppMessage fixture.

### Step 3.1: `std.amiga.workbench`

**Reference**: [03-01](03-01-workbench-appicon.md) · AR-3–AR-5, AR-11, R1, R5

- [ ] 3.1.1 [spec-author] Add the Workbench surface and its host/1.3 rejection, `nix20` cross-build, and appendix checks to `amiga_workbench_spec_test.rs`; run red
- [ ] 3.1.2 Add `stdlib/std.amiga.workbench.clo`, the Workbench shim section (AppIcon record with the default DiskObject, AppWindow borrow, AppMenuItem, AppMessage accessors/reply, `savePublicScreen`), the export table, and the profile gate; run the spec green
- [ ] 3.1.3 Add the Workbench implementation cases (drop routing, AppIcon object ownership, AppWindow borrow, AppMessage fields, public-screen capture) to `amiga_workbench_impl_test.rs`
- [ ] 3.1.4 Add the appendix §14.2 Workbench text and verify the Workbench spec/impl cases and the `nix20` cross-build

**Verify**: `cargo test --lib --test amiga_workbench_spec_test --test amiga_workbench_impl_test`

---

## Phase 4: Expansion, card, and keymap

> **Reasoning**: medium — a read-only service surface with optional resources.

### Step 4.1: `std.amiga.expansion`

**Reference**: [03-03](03-03-expansion-card-keymap.md) · AR-7, AR-8, R3, R9

- [ ] 4.1.1 [spec-author] Add the expansion/card/keymap surface and its host/1.3 rejection, `nix20` cross-build, and appendix checks to `amiga_workbench_spec_test.rs`; run red
- [ ] 4.1.2 Add `stdlib/std.amiga.expansion.clo`, the expansion/card/keymap shim section, the export table, and the profile gate; run the spec green
- [ ] 4.1.3 Add the expansion/card/keymap implementation cases (FindConfigDev loop, field reads, card open/read/RemResource, MapRawKey String, bounds checks) to `amiga_workbench_impl_test.rs`
- [ ] 4.1.4 Add the appendix §14.3 expansion text and verify the spec/impl cases and the `nix20` cross-build

**Verify**: `cargo test --lib --test amiga_workbench_spec_test --test amiga_workbench_impl_test`

---

## Phase 5: Demos, fixtures, and A1200/A600 gates

> **Reasoning**: high — real Workbench and 2.0/3.1 runtime proof.

### Step 5.1: Acceptance programs and gates

**Reference**: [03-04](03-04-demo-and-gates.md) · AR-9, AR-13, AR-14, R5–R7

- [ ] 5.1.1 Add `examples/amiga/services.clo` and `examples/amiga/workbench.clo` with the pinned report lines; compile both for `hosted-amiga`
- [ ] 5.1.2 Add the test-only `docker/scripts/workbench-start.c` and `workbench-post.c`; compile them and run the start fixture on the A600 and A1200 ROMs
- [ ] 5.1.3 Add `docker/scripts/a600-services-smoke.sh` and the `sdk-a600-services` Makefile target; run the gate on A600/KS2.05
- [ ] 5.1.4 Add `docker/scripts/a1200-workbench-smoke.sh` and the `sdk-a1200-workbench` Makefile target; run the gate on A1200/KS3.1 and capture `docs/amiga-sdk/a1200-workbench.png` and `docs/amiga-sdk/a1200-workbench-icon.png`
- [ ] 5.1.5 Add both targets to `verify` and the demo/fixture implementation cases (IT-11, IT-13); re-run both gates
- [ ] 5.1.6 Run `make sdk-a600-services` and `make sdk-a1200-workbench` together; record the report lines and both goldens (full screen and AppIcon crop)

**Verify**: `make sdk-a600-services && make sdk-a1200-workbench`

---

## Phase 6: Documentation, full gate, review, closeout

> **Reasoning**: high — frozen appendices, registry, issue, full acceptance,
> and independent review.

### Step 6.1: Closeout

**Reference**: [03-04](03-04-demo-and-gates.md) · AR-15, AR-16, AR-18, R10

- [ ] 6.1.1 Update `spec/appendix-stdlib.md` §13.2/§14, `spec/appendix-amiga.md`, `spec/15-profiles.md`, `docs/known-limitations.md`, and `AGENTS.md`
- [ ] 6.1.2 Update the registry (split `SDK-WBK-008` into expansion/keymap/card and the deferred `SDK-WBK-010` nonvolatile; set `SDK-WBK-009` depends to `SDK-WBK-001`; 4 done/6 deferred), `bundles.tsv`, `coverage.md` via `make sdk-registry`, issue #9, the programme index, the roadmap, and `codeops/specialist-check.json`
- [ ] 6.1.3 Run `make check`, `make verify`, and `make sdk-registry`; fix regressions
- [ ] 6.1.4 Run the independent correctness, memory-safety, and emission-integrity review; fix every critical/major finding; re-run the affected gates; commit; mark the plan Done

**Verify**: `make check && make verify && make sdk-registry`

---

## Success Criteria

1. The icon, Workbench, and expansion surfaces compile on `hosted-amiga`; the
   host and 1.3 profiles reject them with `E11200`.
2. `make sdk-a1200-workbench` reports
   `wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0`,
   the fixture reports `posted`, and the native Workbench capture and AppIcon
   crop match their goldens.
3. `make sdk-a600-services` reports the same `wb:`/`posted` lines and
   `svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 0`.
4. The earlier A500 gates and the full `make verify` stay green.
5. Registry Bundle 8 is 4 done / 6 deferred with the media evidence; issue #9
   is closed; the programme index and roadmap show Bundle 8 done and Bundle 9
   next.
6. No language, grammar, diagnostic, or ABI change.

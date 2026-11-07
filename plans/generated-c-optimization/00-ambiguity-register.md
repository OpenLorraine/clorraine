# Ambiguity Register: Generated-C Optimization (PLAN-14)

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Mode**: auto-design active — eligible technical decisions delegated under policy version 1
> **Root invocation ID**: `codeops-plan14-20261107`
> **Policy version**: 1

## Register

| ID | Category | Question | Decision | Status / evidence |
|---|---|---|---|---|
| AR-1 | Scope | What does PLAN-14 deliver? | The benchmark harness and suite first, then only the generated-C changes the baseline justifies. No language, grammar, diagnostic, golden, ABI, runtime-shim, or stdlib change. | ✅ Resolved — AI delegated; R1–R8; baseline study in `02-current-state.md` |
| AR-2 | Metric | Which measurement decides a runtime win? | m68k `-O2` net dynamic instruction count from the deterministic `vamos` trace. Host time is recorded but is advisory, never a hard gate. | ✅ Resolved — AI delegated; exact counts reproduced twice (3,044,035 / 2,165,514 / 2,436,214) |
| AR-3 | Baseline storage | Is the baseline JSON committed? | No. `make bench-base COMMIT=<ref>` builds the base compiler in a detached worktree and writes `build/perf/base.json`; the human-readable record is `02-current-state.md`. | ✅ Resolved — AI delegated; reproducible without committing machine-specific timings |
| AR-4 | Harness technology | How is the harness built? | Python 3 standard library only, at `tools/perf/bench.py`, with `make bench`, `make bench-fast`, `make bench-base`, `make bench-compare`, and `make bench-test`; the unit tests run in the pinned container as part of `make check`. | ✅ Resolved — AI delegated; repository already uses Python tooling and has no Python dependency manager |
| AR-5 | Benchmark suite | Which programs are scored? | `sieve`, `matrix`, `strings`, and `moves`, plus a calibration program. Four programs cover the selected shapes without turning the gate into an hour-long run. | ✅ Resolved — AI delegated; R2; each costs minutes at most |
| AR-6 | Fill form | How is a large fill compacted? | One-extent scalar fills only: a `u8`/`i8` constant fill uses one `memset`; every other scalar type, including `bool`, uses one `for` loop; the threshold is 16 covered slots. Nested-row, struct-literal, prefix-array-copy, and deferred-expression forms keep their existing spelling. | ✅ Resolved — AI delegated; R5; evidence in `02-current-state.md` |
| AR-7 | Const folding | Which constants fold, and does storage change? | Only scalar module `const` reads in function bodies fold to typed literals. The read must resolve to a bare global const; an address-taken const never folds. Imported/expression initializers fold with the declaring module's folder. Storage and run-time initialization stay exactly as they are. Module initializers and non-scalar consts stay places. | ✅ Resolved — AI delegated; R4; the address-taken exclusion preserves accepted `unsafe &N` mutation |
| AR-8 | Bounds guard | Does the plan eliminate the upper index check? | No. It removes only the dead `< 0` half for a non-negative constant index into a slice or `str`. Unsigned and fixed-array constant forms already omit it. The baseline showed that removing the upper checks by hand made the m68k code slower after constant folding. | ✅ Resolved — AI delegated; 3,044,035 → 2,165,514 with checks retained; → 2,436,214 with checks removed |
| AR-9 | Division guard | What happens with a constant divisor? | A constant non-zero divisor other than `-1` omits the guard; `-1` keeps only the `MIN / -1` check; constant `0` keeps the existing trap expression and no separate `if` guard. | ✅ Resolved — AI delegated; R7; preserves the existing fault contract |
| AR-10 | Const storage | Should module `const` become `static const` or read-only data? | No. It is a separate storage/initialization change with no measured need beyond read folding; it would touch the module-init contract and the ABI-adjacent layout. | ✅ Resolved — AI delegated; rejected alternative recorded in `03-02-generated-c-changes.md` |
| AR-11 | Thresholds | What counts as a pass? | `net_instructions` and m68k `.text` may grow by at most 1%; host time is advisory; at least one benchmark must improve by 20% instructions or 10% `.text`. The numbers implement the user-approved measured-need direction, not a new product acceptance criterion. | ✅ Resolved — AI delegated technical implementation of the user's "make it shine"/PLAN-14 direction; R1.5 and AR-11 record the rationale |
| AR-18 | Address-taken scan | How is a const's address exposure detected? | A whole-program pre-pass records `&name`/`&qualified.name` roots in function bodies and module initializers; any recorded const is never folded. The pass is conservative at the root and does not need a full points-to analysis. | ✅ Resolved — AI delegated; R4.3; the `unsafe &N` probe returns 1 at HEAD and must keep returning 1 |
| AR-19 | Module initializers | Does const folding reach module-level initializers? | No. The separate `lower_global` path stays unchanged; only function bodies and local initializers fold. | ✅ Resolved — AI delegated; R4.7; the measured hot loop is in a function body |
| AR-20 | Frozen goldens | May the plan add run-fault cases under `spec/tests`? | No. New trap and behavior cases live in the new spec suite; no file under `spec/` is edited. | ✅ Resolved — AI delegated; AGENTS.md frozen-oracle rule; R8.1 |
| AR-21 | Base/head selection | How does the comparison get a base compiler? | `make bench-base COMMIT=<ref>` resolves and validates a canonical commit, builds `clc` from it in a detached worktree under `build/perf/`, runs the current harness and benchmarks against it, and writes `build/perf/base.json`; `make bench` writes the head with the head commit. | ✅ Resolved — AI delegated; R1.9; the base is the only variable and the toolchain identity must match |
| AR-22 | Struct-size metric | Does the harness need per-struct sizes? | No. The linked `.text`/`.data`/`.bss` sizes and emitted-C size are the accepted code-size metrics; the ABI and struct layout are frozen, so a per-struct size adds no decision signal. | ✅ Resolved — AI delegated; R8.7 records the drop; the roadmap row is updated |
| AR-12 | Benchmark naming | How are benchmark files and modules named? | `benchmarks/<name>/program.clo` with module `bench.<Name>`; expected output beside it; a suite README. | ✅ Resolved — AI delegated; matches the corpus layout without calling the programs spec goldens |
| AR-13 | Test split | Which tests prove the changes? | Behavior derives from R4–R7 in a new spec suite that is green before and after; the emitted shapes are the red-first driver in a new impl suite; harness logic in Python unit tests; existing gates and frozen goldens unchanged. | ✅ Resolved — AI delegated; behavior preservation means only the impl pins can be red |
| AR-14 | Commit mode | How is the work landed? | `--auto-commit` for each verified task, on `main`, including the push defined by the exec-plan skill. | ✅ Resolved — user objective; action permission granted by the invocation |
| AR-15 | Rejected polish | Are the `if (sizeof(...))` warning markers and trailing `continue` in scope? | No. Neither showed a measured runtime or code-size cost at the acceptance threshold; they are recorded as rejected rather than shipped on feel. | ✅ Resolved — AI delegated; baseline inspection |
| AR-16 | FS-UAE counters | Must the harness use FS-UAE cycle counts? | No. FS-UAE 3.1.66 has no stable headless cycle counter; its wall time includes boot and display work. The `vamos` instruction trace is deterministic and already part of the acceptance path. | ✅ Resolved — AI delegated; the roadmap's intent is a deterministic target-side metric |
| AR-17 | Acceptance breadth | Is the full `make verify` required? | Yes. The feature changes codegen and must keep every SDK gate green; a host-only check would miss the m68k and emulator legs. | ✅ Resolved — user objective ("make it shine"); repository verification standard |

## Delegated Decision Records

### AR-1 — Scope

```text
Authority: AI — delegated by --auto-design
Eligibility: performance engineering and compiler mechanism; stays inside the confirmed PLAN-14 roadmap row and changes no product behavior, scope, or acceptance criteria beyond the row
Objective: make generated C measurably better without changing language meaning
Decision: deliver the harness, the suite, and the measured codegen changes R4–R7
Evidence: plans/00-roadmap.md PLAN-14; baseline in plans/generated-c-optimization/02-current-state.md
Rejected alternatives: a general optimizer pass (unmeasured and unbounded); optimizing on inspection alone (the roadmap forbids it); a storage-layout change (separate capability)
Strongest counterargument: four local rules may look ad hoc; the counter is that each is selected by a named measured cost and a recorded reject, exactly as the row asks
Confidence: High — the two headline wins are reproduced and the reject is measured
Hardening: independent performance and emission reviews in Phase 5; the benchmark comparison is the executable guard
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a selected rule no longer shows its baseline cost, or a new benchmark reverses a result
```

### AR-2 — Authoritative runtime metric

```text
Authority: AI — delegated by --auto-design
Eligibility: measurement mechanism; no product behavior
Objective: choose a target-side runtime metric that is deterministic and available headlessly
Decision: m68k -O2 net instruction count from `vamos --instr-trace` is the only hard runtime gate; host time is recorded and displayed as advisory
Evidence: repeated trace counts are byte-identical; host best-of-30 timings vary by 6-10%; FS-UAE has no stable headless cycle counter
Rejected alternatives: FS-UAE wall time (boot/display overhead, non-deterministic); host time alone (not the target); static instruction count (misses loop counts)
Strongest counterargument: `vamos` emulates Amiga library calls in Python, so the trace measures guest CPU work, not library internals; the counter is that every compared program uses the same library path and the delta is the signal
Confidence: High
Hardening: calibration program subtracts the fixed startup count; both raw and net values are recorded
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: the trace becomes non-deterministic, or a target-side library-cost question needs a different metric
```

### AR-3 — Baseline storage

```text
Authority: AI — delegated by --auto-design
Eligibility: artifact organization
Objective: keep the baseline reproducible without committing machine-specific data
Decision: `make bench-base COMMIT=<ref>` builds the base compiler in a detached worktree and writes build/perf/base.json; record the committed base/head values and commands in 05-findings.md, with the exploratory study in 02-current-state.md
Evidence: repository hygiene rules; build/ is ignored; the base compiler is the only variable
Rejected alternatives: commit a baseline JSON (stale and machine-specific); no reproducible base path (the comparison gate would be manual)
Strongest counterargument: a reader cannot rerun the exact numbers from a document alone; the counter is that the commands and compiler commit are recorded and the harness regenerates them
Confidence: High
Hardening: the execution plan records both base and head JSON paths used for the acceptance comparison
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a reproducible baseline becomes a release requirement
```

### AR-4 — Harness technology

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanism and internal tooling
Objective: a maintainable harness that runs in the pinned container and the host
Decision: Python 3 stdlib, tools/perf/bench.py, with make bench / bench-compare / bench-test; bench-test joins make check
Evidence: docker/scripts already use Python 3; no package manager is present; stdlib covers subprocess, JSON, statistics, unittest
Rejected alternatives: a Rust workspace binary (new crate and build surface); a shell script (no portable JSON/timing/table logic); Node (tools/demo is a separate npm package)
Strongest counterargument: a Rust harness would share the compiler's toolchain; the counter is that it adds a crate and a build for tooling that is not shipped
Confidence: High
Hardening: harness unit tests in make check; the script has no third-party imports
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: Python is removed from the pinned image, or the harness needs a capability stdlib cannot provide
```

### AR-5 — Benchmark suite

```text
Authority: AI — delegated by --auto-design
Eligibility: test and benchmark strategy
Objective: cover the selected shapes with a bounded gate
Decision: sieve, matrix, strings, moves, plus calibration
Evidence: sieve exercises fill, const, nested loops, and bounds; matrix adds multi-extent arrays and calls; strings adds views and allocation; moves adds ownership and generics
Rejected alternatives: reuse the spec corpus (too small, no hot loops); one giant benchmark (a regression in one shape hides); a large ported program (license and scope)
Strongest counterargument: four small programs cannot represent every real workload; the counter is that the selection rule is shape-based and the harness can add a program later
Confidence: High
Hardening: every benchmark has a README and a stable checksum; the baseline records its counted instructions
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a selected shape has no benchmark, or a real program exposes a cost the suite misses
```

### AR-6 — Fill compaction

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler emission mechanism
Objective: remove the per-element fill bloat without changing fill semantics
Decision: threshold 16 on the covered slots; one-extent scalar fills only; u8/i8 constant uses memset; every other scalar including bool uses a for loop; nested, struct-literal, prefix-copy, and deferred forms keep their spelling
Evidence: 8,192-slot fill = 8,192 statements, 489,491 C bytes, 101,396 .text at -O2; compacted = 310,413 C bytes, 76,768 .text at -O2 (host time unchanged within noise)
Rejected alternatives: always compact (small fills lose the unrolled fast path); never compact (the measured cost remains); a runtime helper (new surface and call overhead); bool memset (bool is not byte-sized on the target)
Strongest counterargument: a loop can be slower than unrolled stores at -O0; the counter is that .text and C size are the measured cost, and the threshold protects small fills
Confidence: High
Hardening: behavior suite covers prefix, nested, impure, and guarded fills; benchmark compare guards runtime
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: an -O0 benchmark regresses materially, or the C compiler already collapses the unrolled form on the target
```

### AR-7 — Scalar const folding

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler lowering mechanism
Objective: remove the run-time static read that blocks constant propagation and bounds elimination
Decision: fold scalar module const reads in function bodies to typed literals; require a bare global-const place; exclude any address-taken const; use the declaring module's folder; keep storage and initialization
Evidence: sieve net instructions 3,044,035 → 2,165,514 (-28.9%) and container host -O2 median 31.0 → 22.0 ms; `unsafe &N` currently changes the storage and must keep doing so
Rejected alternatives: change storage to static const (separate layout/init change); fold only integers (bools/floats are the same mechanism); fold aggregate consts (different lowering, no measured need); fold module initializers (separate lowering, no measured need)
Strongest counterargument: folding hides the storage from the C compiler, but the address-taken pre-pass keeps every exposed const as a place; the counter is that scalar parameters are by value in every mode
Confidence: High
Hardening: spec cases for imported, qualified, negative, f32, shadowed, address-taken, and expression initializers; impl pin proves the storage declaration and initializer remain
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a borrow form needs the address, or the const's initializer becomes runtime-evaluated
```

### AR-8 — Bounds guard

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler emission mechanism
Objective: remove a provably dead comparison without risking the measured regression
Decision: omit the `< 0` half only for a non-negative constant index into a slice or str; unsigned and fixed-array constant forms already omit it; do not add manual upper-bound elimination
Evidence: manual removal of three upper checks made m68k instructions 2,165,514 → 2,436,214; a slice/str `s[1]` currently emits `(1 < 0)` while a `usize` index does not
Rejected alternatives: loop-range analysis (unbounded, measured regression risk); re-implementing the already-correct unsigned/fixed-array forms (no red test); dropping the change (leaves a dead compare in every slice/str constant access)
Strongest counterargument: the dead compare is cheap and the C compiler may remove it; the counter is that the emitted C is the contract and the simplification is local and provable
Confidence: High
Hardening: signed negative traps and unsigned boundary traps stay in the spec suite
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a benchmark shows the lower check matters, or the semantic layer starts accepting negative literals
```

### AR-9 — Division guard

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler emission mechanism
Objective: remove a constant-false guard without changing a trap
Decision: non-zero literal (not -1) omits the zero check; -1 keeps the MIN check; 0 keeps the trap
Evidence: `x % 2` currently emits `if (2 == 0 || (x == MIN && 2 == -1))`, which is constant false
Rejected alternatives: leave it (dead code in every constant division); fold only zero/non-zero (the MIN case is the only remaining hazard)
Strongest counterargument: the C compiler folds the condition; the counter is that the emitted C is also the debugging and inspection artifact, and the change is local
Confidence: High
Hardening: run-fault suite covers MIN/-1 and zero divisors
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a new divisor type or a non-literal fold changes the guard shape
```

### AR-10 — Const storage (rejected)

```text
Authority: AI — delegated by --auto-design
Eligibility: compiler emission mechanism
Objective: decide whether read-only storage belongs in this row
Decision: reject it
Evidence: the measured win comes from the read, not the storage; the module-init contract and initialization order are ABI-adjacent; the specification already treats const as compile-time but the current storage is a separate concern
Rejected alternatives: static const with initializer (touches init ordering and unused-symbol warnings); removal of the initializer (touches the M5/M6 contract)
Strongest counterargument: read-only storage would shrink .data and prevent mutation bugs; the counter is that it is a separate capability with no measured cost in this suite
Confidence: Med — a future row may take it with its own evidence
Hardening: recorded here and in 03-02-generated-c-changes.md; no partial change ships
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a measured .data/read-only need appears, or a const-mutation defect requires the storage change
```

### AR-11 — Acceptance thresholds

```text
Authority: AI — delegated by --auto-design
Eligibility: performance engineering and test strategy
Objective: distinguish a real win from noise without hiding a regression
Decision: 1% instruction and .text allowance; host time is advisory; 20% instruction or 10% .text required win. The numbers implement the user-approved measured-need direction rather than creating a new product acceptance criterion.
Evidence: repeated instruction counts are exact; host best-of-30 varies by 6-10%; the selected wins are 28.9% and 24.3%
Rejected alternatives: zero tolerance (flaky host time); a hard 2% host gate (contradicts AR-2 and the measured noise); a single aggregate score (hides a per-benchmark regression)
Strongest counterargument: thresholds are arbitrary; the counter is that each is tied to the metric's observed noise or the expected win's margin, and the user's direction is the measured-need rule
Confidence: High
Hardening: the comparison table shows every metric and the failed row names its threshold; host time is displayed but not gated
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: the harness shows a metric's noise exceeds its allowance
```

### AR-12 — Benchmark naming

```text
Authority: AI — delegated by --auto-design
Eligibility: repository organization
Objective: keep benchmark fixtures distinct from frozen spec goldens
Decision: benchmarks/<name>/program.clo, module bench.<Name>, expected-output.txt, README.md
Evidence: spec/tests/corpus uses the same per-program layout; the prefix makes provenance explicit
Rejected alternatives: put them under spec/tests (implies frozen-spec status); one flat file (no README or expected output)
Strongest counterargument: another directory is more surface; the counter is that the spec chasm is important
Confidence: High
Hardening: benchmarks/README.md states they are not spec goldens
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: the spec adopts a benchmark corpus
```

### AR-13 — Test split

```text
Authority: AI — delegated by --auto-design
Eligibility: test strategy
Objective: make each requirement falsifiable without weakening the frozen oracle
Decision: new spec and impl suites; Python unit tests for the harness; existing suites unchanged
Evidence: repository testing standard; the changes are codegen behavior and emission shape
Rejected alternatives: only impl pins (does not prove behavior); only the benchmark (too coarse); add cases to unrelated suites (poor ownership)
Strongest counterargument: two more suites add CI time; the counter is that they are focused and the feature is codegen-wide
Confidence: High
Hardening: every spec case names its R; every impl pin names the emitted line it protects
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a rule's behavior cannot be observed from a program, or an existing suite already owns the exact case
```

### AR-14 — Commit mode

```text
Authority: user — explicit objective
Eligibility: not delegated; action permission from the invocation
Objective: land the completed plan without a per-task prompt
Decision: --auto-commit for each verified task on main, including the push defined by exec-plan
Evidence: the user's objective names --auto-commit and --auto-design
Rejected alternatives: ask per task (contradicts the objective); commit without push (contradicts the named mode)
Strongest counterargument: auto-commit can land a bad intermediate state; the counter is that each task is verified before its commit and the quality gate can stop the run
Confidence: High
Hardening: every commit is preceded by its task's verification command; a failed gate stops the run
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: the user changes the commit mode or the integration branch
```

### AR-15 — Rejected polish

```text
Authority: AI — delegated by --auto-design
Eligibility: performance engineering
Objective: decide whether two visible C-quality issues are in scope
Decision: reject the `if (sizeof(...))` warning markers and trailing `continue` removal
Evidence: the warning marker is a compile-time no-op; the trailing `continue` is removed by the C compiler at -O2 and showed no measured benchmark delta
Rejected alternatives: ship both as "obvious" cleanups (forbidden by the measured-need rule)
Strongest counterargument: they make the C smaller and easier to read; the counter is that the row's acceptance is measured cost, and neither clears the threshold
Confidence: High
Hardening: recorded in 05-findings.md with the inspection evidence
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a benchmark shows a material cost, or a readability requirement is separately approved
```

### AR-16 — FS-UAE counters

```text
Authority: AI — delegated by --auto-design
Eligibility: measurement mechanism
Objective: satisfy the roadmap's target-side measurement intent
Decision: use the deterministic vamos instruction trace; do not add FS-UAE cycle instrumentation
Evidence: FS-UAE 3.1.66 exposes no stable headless cycle counter; wall time includes boot and display; vamos is already the run-fault and corpus runner
Rejected alternatives: FS-UAE wall time (noisy); a custom UAE build (out of scope); no target metric (contradicts the row)
Strongest counterargument: vamos is not cycle-accurate; the counter is that instruction count is a valid optimization signal and the code-size metric covers the other axis
Confidence: High
Hardening: the limitation is stated in the harness doc and known limitations
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: a cycle-accurate headless counter becomes available, or a decision needs cycle timing
```

### AR-17 — Acceptance breadth

```text
Authority: user — explicit objective
Eligibility: not delegated; verification scope follows the user's "make it shine"
Objective: prove the feature does not break the Amiga surface
Decision: run the full make verify, including every SDK gate
Evidence: the repository's acceptance model; the change touches emitted C for every profile
Rejected alternatives: make check only (misses m68k and emulator legs); a subset of SDK gates (not the repository standard)
Strongest counterargument: the full run is long; the counter is that codegen changes are exactly the case the full matrix exists for
Confidence: High
Hardening: the execution plan gates the final commit on the full result
Policy version: 1
Root invocation ID: codeops-plan14-20261107
Reopen triggers: the user narrows the acceptance scope
```

## Reserved Decisions

No product-behavior, scope, priority, product acceptance-criteria,
security-policy, legal, financial, credential, deployment, or publication
decision is delegated. The benchmark pass thresholds are the technical
implementation of the user-approved direction — PLAN-14's measured-need rule
and the objective's "make it shine" — not a new product acceptance criterion.
The codegen mechanisms are technical. The commit mode and the full-verify
breadth come from the user's explicit objective.

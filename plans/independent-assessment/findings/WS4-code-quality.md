# WS4 — Generated-code quality and the optimizer question

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS4 (WS4-01..WS4-10)

## Scope and limits

Components: `codegen/`, `ir/`, `consts/`, the release flags, the emitted C of the
benchmarks. Measurements cite the Phase 0 matrix (`records/WS0-matrix.json`); WS4 adds
attribution traces, patched-C estimates (diffs against the baseline A500 emission), and
flag variants. Optimizer estimates patch emitted C only; no compiler is changed (AR #17).

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (4.1.1) | `probes/WS4/400`–`430` | done |
| Red check (4.1.2) | seeded symbol, seeded mismatch; attribution amended | done |

## Checklist outcomes

_(filled in Step 4.2)_

## Findings

_(filled in Step 4.2)_

## Spec questions

_(filled in Step 4.2)_

## Falsifiability log

Red check 2026-11-25 after `34400fe4` (log `build/tmp/assessment/WS4/red.log`).

| Probe | Form | Red-check result |
|-------|------|------------------|
| `400-attribute.py` | seeded known symbol: `400-seeded.c`, whose `static spin()` runs a 20,000-iteration loop | **first run failed the check**: 96.2% was attributed to `___exitmalloc`, because the Amiga hunk symbol table keeps only global symbols and `spin` is `static`. The script was amended to read function addresses from the link map (`.text.<name>` sections) and committed before re-running (`probes/WS4/400-attribute.py`); the re-run attributes 96.1% to `spin.constprop.0`. Every emitted Clorraine function is `static`, so the amendment is what makes WS4-02 possible |
| `430-patch-measure.py` | comparator with a seeded wrong expected output (`--seeded-mismatch`, patch 414) | reported `FAIL` with both outputs rejected, exit 1 |
| `420-flags.py` | comparator: the same `bench.normalize_output` equality whose rejection of a mutated expectation was shown in Phase 0 | form recorded |
| `410`–`417` diffs | each patch must keep the golden output; the comparator above shows the check fires | observed in task 4.2.2 |

## Escalations

None yet.

## Probe index

| Item | Probes |
|------|--------|
| WS4-01 overhead table | Phase 0 matrix |
| WS4-02 attribution | `400-attribute.py`, `400-seeded.c` |
| WS4-03 guards | `410-sieve-range-guards.diff` |
| WS4-04 drop flags | `415-strings-static-drop.diff` |
| WS4-05 temporaries and copies | `411`, `412`, `413`, `414` diffs |
| WS4-06 static state | `416-lookup-static-init.diff` |
| WS4-07 size | `417-trap-no-printf.diff`, `425-size.sh` |
| WS4-08 flags | `420-flags.py` |
| runner | `430-patch-measure.py` |

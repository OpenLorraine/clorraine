# Execution Plan: Direct Template Formatting

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-11 12:00
> **Progress**: 0/26 tasks (0%)
> **CodeOps Artifact Schema**: 1
> **Commit mode**: `--auto-commit --auto-design`; pushes only after the final
> gate.

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Baseline | 3 |
| 2 | Specification tests and red run | 3 |
| 3 | Implementation and green run | 5 |
| 4 | Measurement, sanitizers, and emission proof | 6 |
| 5 | Reviews and full gate | 5 |
| 6 | Closeout | 4 |

**Total: 26 tasks across 6 phases.**

> **⚠️ EXECUTION RULE:** the checkboxes are the single source of truth. Mark
> `[~]` with a timestamp on implementation, promote to `[x]` only after the
> task's verify passes, and update the Progress header after every task. A
> blocker is `[!]` with a short reason.

---

## Phase 1: Baseline

### Step 1.1: Commit the plan and measure the base

**Reference**: R6, AR-08
**Objective**: fix the shim-only base before changing codegen.

- [ ] 1.1.1 Commit the plan documents with the git-commit skill; no compiler or runtime file may change.
- [ ] 1.1.2 Run `make bench-base COMMIT=00eb6bc` and confirm `build/perf/base.json` names the commit with `dirty=false`; record the compiler SHA-256 in `05-findings.md` §1.
- [ ] 1.1.3 Run `make bench` on the plan commit and `make bench-compare`; record the zero-delta baseline and the allocation count.

**Verify**: `make check` green on the plan commit; base and pre-change documents equal.

---

## Phase 2: Specification tests and red run

### Step 2.1: Write the allocation and behavior tests

**Reference**: R1–R3, AR-07 · `07-testing-strategy.md` ST-1 – ST-15
**Objective**: pin the allocation contract before changing the lowering.

- [ ] 2.1.1 [spec-author] Write `compiler/tests/template_direct_formatting_spec_test.rs` ST-1 – ST-15: the `--wrap=malloc` counter, the allocation deltas, the boundary outputs, the source-reuse case, the call-once case, ASan/UBSan markers, and `RDC_CLC` support.
- [ ] 2.1.2 Run the suite against `00eb6bc` and record the red deltas (ST-1/2/3/4 expected 3/2/2/2) and the no-change passes in `05-findings.md` §3.
- [ ] 2.1.3 Confirm the suite is discovered by `make asan` and `make ubsan`; do not run the full legs yet.

**Verify**: the red run is recorded; no compiler or shim file changed.

---

## Phase 3: Implementation and green run

### Step 3.1: Lower placeholders into typed parts

**Reference**: R1–R5, AR-02 – AR-06 · `03-template-lowering.md`
**Objective**: implement the typed part and the direct builder in both shims and the codegen.

- [ ] 3.1.1 Change `compiler/src/codegen/expr.rs::template` to emit the typed parts (scalars, borrowed `str`/`String`, bool, float) while keeping the literal-only path and `text_call` unchanged.
- [ ] 3.1.2 Replace `ClStrPart`, the helpers, and `cl_string_concat` in `compiler/runtime/shim_host.c`; move the decimal helpers above the builder.
- [ ] 3.1.3 Apply the same builder and helpers to `compiler/runtime/shim_hosted_amiga.c`; keep the freestanding shim untouched.
- [ ] 3.1.4 Run the new spec suite green and the existing `string_integer_formatting` suites green.
- [ ] 3.1.5 Write `compiler/tests/template_direct_formatting_impl_test.rs` after the implementation and run it green; record the head commit and the green result in `05-findings.md` §3.

**Verify**: `cargo test --workspace` green.

---

## Phase 4: Measurement, sanitizers, and emission proof

### Step 4.1: Prove the win and the safety legs

**Reference**: R5, R6, AR-08, AR-09
**Objective**: measure the allocation and instruction win and complete the emission proof.

- [ ] 4.1.1 Run `make bench` and `make bench-compare`; record the final table, outputs, allocation count, and the 1% guard in `05-findings.md` §4.
- [ ] 4.1.2 Run the allocation-count probe on the head; confirm the two-integer template is 1 and `strings` is near 2,002; record the split.
- [ ] 4.1.3 Run `make asan` and `make ubsan`; confirm the new suite is in both legs; record in §5.
- [ ] 4.1.4 Run `make bench-shape`; confirm host, Amiga, nix13, vbcc, and freestanding legs are clean.
- [ ] 4.1.5 Build a boundary program for `hosted-amiga`, run it under `vamos`, and compare it with the host output.
- [ ] 4.1.6 Record the base/head emitted-C diff for the host and Amiga profiles; confirm only the template call and the shim builder changed.

**Verify**: `make bench-compare` passed; sanitizer and shape legs green.

---

## Phase 5: Reviews and full gate

### Step 5.1: Independent review and acceptance

- [ ] 5.1.1 Dispatch the correctness reviewer with a blast-radius packet (every caller of `template`, `text_call`, `cl_string_concat`, both profiles, explicit converters).
- [ ] 5.1.2 Dispatch the memory-safety auditor with the borrow/lifetime, union, capacity, and ASan probes.
- [ ] 5.1.3 Dispatch the read-only emission-integrity auditor with the base/head corpus emission diff and the strict-C build hunt.
- [ ] 5.1.4 Resolve every critical/major finding and rerun the affected legs.
- [ ] 5.1.5 Run `make check`, `make bench-compare`, and `make verify`; record the results in `05-findings.md` §5–§6.

**Verify**: `make verify: all legs passed`; no open critical/major finding.

---

## Phase 6: Closeout

- [ ] 6.1.1 Complete `05-findings.md` with the final table, allocation split, emission proof, and review verdicts.
- [ ] 6.1.2 Update `plans/00-roadmap.md` with the plan row at Done and the final numbers.
- [ ] 6.1.3 Close issues #25 and #19 with the final evidence.
- [ ] 6.1.4 Commit the documentation and roadmap with the git-commit skill; verify the tree is clean and push after the final gate.

**Verify**: tree clean; `main == origin/main`; both issues closed.

---

## Success Criteria

1. ✅ All 26 tasks completed.
2. ✅ `strings` allocations near 2,002; net instructions below 1,489,187.
3. ✅ Every scored benchmark inside the 1% instruction and `.text` guard; outputs equal.
4. ✅ ASan/UBSan clean; all strict-C legs green.
5. ✅ Three reviews recorded with no open critical/major finding.
6. ✅ `make check`, `make bench-shape`, `make bench-compare`, `make verify` green.
7. ✅ #25 and #19 closed; roadmap Done.

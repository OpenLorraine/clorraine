# Execution Plan — Evaluation and Initializer Residue (Batch T)

> **Progress**: 0/23 tasks
> **Rows**: `DEF-529`, `DEF-531`, `DEF-532`
> **Baseline**: `55beee0`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Run-time-index view store (`DEF-529`)

> **Lenses**: memory-safety, correctness
> **Reasoning**: high — a leak plus a caller-visible transfer

### Step 1.1: Red case

- [ ] 1.1.1 Add the spec case: the generic run-time-index store at a `str`
      instance leaves the caller's owner readable and returns 0; the
      constant-index twin stays accepted
- [ ] 1.1.2 Confirm the run-time case fails at the baseline (the caller's
      `len(a.v)` sees a null handle) and commit the red case

### Step 1.2: Implementation

- [ ] 1.2.1 In `emit_indexed_store`, keep a place source for an unknown or
      parameter element type instead of consuming it
- [ ] 1.2.2 Run the focused suite and `make check`; commit the fix

### Step 1.3: Pins and review

- [ ] 1.3.1 Add the implementation pin: no transfer temporary, no source
      `= NULL`, the view widens the caller's place; run the host ASan probe
- [ ] 1.3.2 Dispatch the phase review; apply corrections
- [ ] 1.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 2: Non-call effectful `len` operands (`DEF-531`)

> **Lenses**: correctness, semantics, emission-integrity
> **Reasoning**: high — evaluation order and path sensitivity

### Step 2.1: Red cases

- [ ] 2.1.1 Add the spec cases: a conditional operand runs only the selected
      arm, an array literal runs each element once, a run-time index runs its
      index call once; the pure place and direct-call controls stay green
- [ ] 2.1.2 Confirm each red case drops the effects at the baseline and commit
      the red cases

### Step 2.2: Implementation

- [ ] 2.2.1 Split the non-pure, non-call operand by
      `speculative_lowering`: `lower_discarded` in an unconditional position,
      structural fold in a conditionally evaluated position
- [ ] 2.2.2 Run the focused suites and `make check`; commit the fix

### Step 2.3: Pins and review

- [ ] 2.3.1 Add the implementation pins (the discard shape and the folded
      extent) and the ASan runs
- [ ] 2.3.2 Dispatch the phase review; apply corrections
- [ ] 2.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 3: Module initializer call rule (`DEF-532`)

> **Lenses**: correctness, semantics
> **Reasoning**: medium — one focused read-only walk

### Step 3.1: Red cases

- [ ] 3.1.1 Add the rejecting cases: a direct global borrow and a bare global
      dot receiver in a module initializer; the `const` global control stays
      accepted
- [ ] 3.1.2 Confirm the two rejecting cases pass at the baseline and commit the
      red cases

### Step 3.2: Implementation

- [ ] 3.2.1 Add the initializer call walk to `own/program.rs` and pass the
      collected program from `own::check`
- [ ] 3.2.2 Run the focused suite and `make check`; commit the fix

### Step 3.3: Pins and review

- [ ] 3.3.1 Add the accepted control pin and the one-report assertion
- [ ] 3.3.2 Dispatch the phase review; apply corrections
- [ ] 3.3.3 Record the phase in `05-findings.md` and the roadmap

## Close-out

- [ ] 4.1 Run `make check` and `make verify` at the batch head
- [ ] 4.2 Reconcile the roadmap rows, the findings ledger, and the plan
      progress
- [ ] 4.3 Commit the documentation close-out

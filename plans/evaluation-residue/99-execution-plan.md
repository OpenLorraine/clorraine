# Execution Plan — Evaluation and Initializer Residue (Batch T)

> **Progress**: 30/30 tasks
> **Rows**: `DEF-529`, `DEF-531`, `DEF-532`
> **Baseline**: `55beee0`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Run-time-index view store (`DEF-529`)

> **Lenses**: memory-safety, correctness
> **Reasoning**: high — a leak plus a caller-visible transfer

### Step 1.1: Red case

- [x] 1.1.1 Add the spec case: the generic run-time-index store at a `str`
      instance leaves the caller's owner readable and returns 0; the
      constant-index twin stays accepted
- [x] 1.1.2 Confirm the run-time case fails at the baseline (the caller's
      `len(a.v)` sees a null handle) and commit the red case (`2aa065a`)

### Step 1.2: Implementation

- [x] 1.2.1 In `emit_indexed_store`, keep a place source for an unknown or
      parameter element type instead of consuming it
- [x] 1.2.2 Run the focused suite and `make check`; commit the fix (`5ca8c08`)

### Step 1.3: Pins and review

- [x] 1.3.1 Add the implementation pin: no transfer temporary, no source
      `= NULL`, the view widens the caller's place; run the host ASan probe
- [x] 1.3.2 Dispatch the phase review; apply corrections (the review called the
      fix sound; `DEF-535` filed for the adjacent pre-existing case)
- [x] 1.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 2: Non-call effectful `len` operands (`DEF-531`)

> **Lenses**: correctness, semantics, emission-integrity
> **Reasoning**: high — evaluation order and path sensitivity

### Step 2.1: Red cases

- [x] 2.1.1 Add the spec cases: a conditional operand runs only the selected
      arm, an array literal runs each element once, a run-time index runs its
      index call once; the pure place and direct-call controls stay green
- [x] 2.1.2 Confirm each red case drops the effects at the baseline and commit
      the red cases (`ca0ba15`)

### Step 2.2: Implementation

- [x] 2.2.1 Split the non-pure, non-call operand by `defer_move_outs == 0 &&
      !in_loop_condition`: an effects-only walk in an unconditional position,
      structural fold in a conditionally evaluated position
- [x] 2.2.2 Run the focused suites and `make check`; commit the fix (`0e3b74e`)

### Step 2.3: Pins and review

- [x] 2.3.1 Add the implementation pins (the no-move operand shape and the
      folded extent) and the ASan runs
- [x] 2.3.2 Dispatch the phase review; fix the mixed place-arm conditional UAF
      and extend the walk to the bare and dot spellings; run the ASan probes
- [x] 2.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 3: Module initializer call rule (`DEF-532`)

> **Lenses**: correctness, semantics
> **Reasoning**: medium — one focused read-only walk

### Step 3.1: Red cases

- [x] 3.1.1 Add the rejecting cases: a direct global borrow and a bare global
      dot receiver in a module initializer; the `const` global control stays
      accepted
- [x] 3.1.2 Confirm the two rejecting cases pass at the baseline and commit the
      red cases (`1afd3ab`)

### Step 3.2: Implementation

- [x] 3.2.1 Add the initializer call walk to `own/program.rs` and pass the
      collected program from `own::check`
- [x] 3.2.2 Run the focused suite and `make check`; commit the fix (`88b73b4`)

### Step 3.3: Pins and review

- [x] 3.3.1 Add the accepted control pin and the one-report assertion
- [x] 3.3.2 Dispatch the phase review; fix the false positives and false
      negatives (scope-first callee resolution, declaring module, projected
      receiver, field table, generic dot, index projection)
- [x] 3.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 5: Review corrections

> **Lenses**: memory-safety, correctness, semantics, emission
> **Reasoning**: high — one critical UAF and several initializer judgments

- [x] 5.1.1 Replace the unconditional `len` discard with the effects-only walk;
      a mixed caller-place conditional evaluates the selected fresh arm and
      leaves the place arm in place
- [x] 5.1.2 Add the every-spelling spec test (bare, dot, qualified; owned and
      mixed cases) and the no-move implementation pin; run `RDC_ASAN=1`
- [x] 5.2.1 Rewrite the initializer callee resolution and receiver typing;
      restore the current-module/import/program order and the declaring module
- [x] 5.2.2 Add the accepted and rejected initializer spec pins for the
      projected field, same-named struct, import alias, fallback, generic dot,
      and index-projected receiver
- [x] 5.3.1 Restore the two retargeted implementation pins to the original
      global-slice shapes
- [x] 5.3.2 Record the residuals (`DEF-534`–`DEF-537`), extend `DEF-533`, and
      update the plan, findings, and roadmap

## Close-out

- [x] 4.1 Run `make check` and `make verify` at the batch head
- [x] 4.2 Reconcile the roadmap rows, the findings ledger, and the plan
      progress
- [x] 4.3 Commit the documentation close-out

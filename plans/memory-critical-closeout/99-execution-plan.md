# Execution Plan — Memory and Borrow Critical Closeout (Batch O)

> **Progress**: 24/26 tasks
> **Rows**: `DEF-449`, `DEF-450`, `DEF-451`, `DEF-452`, `DEF-453`, `DEF-459`, `DEF-468`, `DEF-469`
> **Baseline**: `d19a467`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: View escapes through carriers and calls

> **Rows**: `DEF-449`, `DEF-450`, `DEF-451`, `DEF-453` (all critical)
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — the fix changes loan transfer and escape judgment

### Step 1.1: Red cases

- [x] 1.1.1 Write the red cases: the conditional carrier copy (`DEF-449`) in
      `borrow_store_integrity_spec_test.rs`; the aggregate out-slot escape (`DEF-450`) and the
      call-mediated inline/global views (`DEF-451`, `DEF-453`) in
      `eval_emission_integrity_spec_test.rs`. Each case asserts the concrete twin's existing
      diagnostic, one report per site, and the accepted control stays green
- [x] 1.1.2 Confirm the cases fail at the baseline (accepted+UAF or wrong report), then commit the
      red cases

### Step 1.2: Implementation

- [x] 1.2.1 Record copy element loans for a carrier destination in the let and assignment paths
      (`DEF-449`)
- [x] 1.2.2 Extend the cross-frame judgment to aggregate arguments whose parameter can carry a view
      (`DEF-450`)
- [x] 1.2.3 Add the `Call`/`ClosureCall` arm to the lowered root and value-loan walks; route the
      global fallback through the value loans (`DEF-451`, `DEF-453`)
- [x] 1.2.4 Run the focused suites and `make check`; commit the fix

### Step 1.3: Pins and review

- [x] 1.3.1 Add the implementation pins (the destination positions, the emitted owner binding, the
      report count/text, byte-identical accepted controls)
- [x] 1.3.2 Dispatch the four-lens phase review; apply corrections and bounded confirmation
- [x] 1.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 2: Take alias capture

> **Row**: `DEF-459` (critical)
> **Lenses**: correctness, memory-safety, emission-integrity
> **Reasoning**: high — evaluation order and post-call addressing

### Step 2.1: Red case

- [x] 2.1.1 Write the red case in `conditional_projection_residue_spec_test.rs`: the pure-index
      pointer take under a mutating callee; assert ASan-clean, expected exit, and the captured
      pointer shape
- [x] 2.1.2 Confirm the baseline failure and commit the red case

### Step 2.2: Implementation

- [x] 2.2.1 Capture the raw-deref pointer for an owning take, including pure places; keep the
      guarded deferral
- [x] 2.2.2 Run the focused suites and `make check`; commit the fix

### Step 2.3: Pins and review

- [x] 2.3.1 Add the implementation pins (the captured local, the move-out and alias use, the
      unchanged non-mutating twin)
- [x] 2.3.2 Dispatch the three-lens review; apply corrections and bounded confirmation
- [x] 2.3.3 Record the phase

## Phase 3: Generic conditional duties

> **Rows**: `DEF-468`, `DEF-469` (both critical)
> **Lenses**: correctness, semantics, memory-safety
> **Reasoning**: high — per-instance substitution and duty uniqueness

### Step 3.1: Red cases

- [x] 3.1.1 Write the red cases in `borrow_generic_integrity_spec_test.rs`: the generic conditional
      return of a lend arm and the generic field-projection arm; assert the instance report and the
      duplicable-instance acceptance
- [x] 3.1.2 Confirm the baseline failure and commit the red cases

### Step 3.2: Implementation

- [x] 3.2.1 Collect the type-parameter conditional arms for a return and record the per-instance
      move duty (return and projection forms; the take path if missing)
- [x] 3.2.2 Run the focused suites and `make check`; commit the fix

### Step 3.3: Pins and review

- [x] 3.3.1 Add the implementation pins (the duty, one report, the accepted duplicable instance,
      byte-identical concrete twins)
- [x] 3.3.2 Dispatch the three-lens review; apply corrections and bounded confirmation
- [x] 3.3.3 Record the phase

## Phase 4: Ruling, controls, and close-out

> **Row**: `DEF-452` (minor)
> **Lenses**: correctness, semantics

- [x] 4.1.1 Record the generated-owner ruling and add the green same-statement/later-fill control
- [ ] 4.1.2 Run `make check`, `make verify`, the focused ASan legs, and the emission differentials
- [ ] 4.1.3 Update the roadmap rows (`DEF-449`, `450`, `451`, `453`, `459`, `468`, `469` Done;
      `DEF-452` Done with the ruling), the ledger, and the plan status; push

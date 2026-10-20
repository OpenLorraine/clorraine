# Execution Plan — Opaque Store and Call Residue (Batch R)

> **Progress**: 0/49 tasks
> **Rows**: `DEF-525`, `DEF-526`, `DEF-527`, `DEF-528`, `DEF-395`, `DEF-396`, `DEF-401`,
> `DEF-406`, `DEF-407`, `DEF-402`
> **Baseline**: `766e9b9`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Opaque member store

> **Rows**: `DEF-525` (critical)
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — the fix adds a per-instance duty and changes a store's emitted shape

### Step 1.1: Red cases

- [ ] 1.1.1 Write the red cases in `eval_emission_integrity_spec_test.rs`: the local-view
      escape (`E10030` at the return), the caller-backed parameter-view control (accepted and
      clean), and the owned-instance view store (`E10200` at the store); each asserts the twin's
      code once or an exact clean run
- [ ] 1.1.2 Confirm every case fails at the baseline (accepted+UAF, invalid C, or the wrong
      acceptance), then commit the red cases

### Step 1.2: Implementation

- [ ] 1.2.1 Add `ir::RequiresOpaqueStore` and its accessor to `ir/mono.rs`
- [ ] 1.2.2 Record the duty at an unknown/parameter destination store (`own/expr.rs`)
- [ ] 1.2.3 Mark the duty escaped at a return of its root binding (`own/statements.rs`)
- [ ] 1.2.4 Discharge the duty in `own/instance_check.rs`: conversion or escape
- [ ] 1.2.5 Render an unknown-target `Move` as a view store when the instance target is
      view-like (`codegen/function.rs`)
- [ ] 1.2.6 Run the focused suites and `make check`; commit the fix

### Step 1.3: Pins and review

- [ ] 1.3.1 Add the implementation pins (report count and text, widened store text, no source
      move, byte-identical accepted control)
- [ ] 1.3.2 Dispatch the four-lens phase review; apply corrections and bounded confirmation
- [ ] 1.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 2: Type and closure surface

> **Rows**: `DEF-527`, `DEF-528`
> **Lenses**: correctness, emission-integrity; semantics for the closure rule
> **Reasoning**: medium — a pure typedef shape and one conditional rule

### Step 2.1: Red cases

- [ ] 2.1.1 Write the red cases: the slice-of-func typedef and call, and the capturing-closure
      conditional; each asserts a strict build/run or one `E10404`
- [ ] 2.1.2 Confirm both fail at the baseline and commit the red cases

### Step 2.2: Implementation

- [ ] 2.2.1 Teach `declarator`'s `Raw` arm the function-pointer shape and route the slice
      typedef through it (`DEF-527`)
- [ ] 2.2.2 Reject a conditional of distinct capturing closures with `E10404`
      (`DEF-528`)
- [ ] 2.2.3 Run the focused suites and `make check`; commit the fix

### Step 2.3: Pins and review

- [ ] 2.3.1 Add the implementation pins (typedef text, strict build, one-report text)
- [ ] 2.3.2 Dispatch the phase review; apply corrections
- [ ] 2.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 3: Capture write summary

> **Rows**: `DEF-526`
> **Lenses**: correctness, memory-safety
> **Reasoning**: medium — a conservative rule is relaxed only for provably surviving writes

### Step 3.1: Red case

- [ ] 3.1.1 Write the safe parameter-backed write case (accepted and clean) beside the batch-Q
      local-owner escape control; confirm the safe case fails at the baseline
- [ ] 3.1.2 Commit the red case

### Step 3.2: Implementation

- [ ] 3.2.1 Add the lambda-body write summary and apply a surviving provenance in
      `mark_written_captures`
- [ ] 3.2.2 Keep the frame-backed and unknown-write cases conservative
- [ ] 3.2.3 Run the focused suites and `make check`; commit the fix

### Step 3.3: Pins and review

- [ ] 3.3.1 Add the implementation pins (the safe emission and the retained rejection)
- [ ] 3.3.2 Dispatch the phase review; apply corrections
- [ ] 3.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 4: Generic dot call

> **Rows**: `DEF-395`, `DEF-401`, `DEF-406`
> **Lenses**: correctness, semantics, emission-integrity
> **Reasoning**: high — the checked return type and explicit type arguments drive emission

### Step 4.1: Red cases

- [ ] 4.1.1 Write the red cases: the generic dot-call `String` return, the explicit
      type-argument count/type mismatch, and the shared-`T` generic caller; each asserts the
      concrete twin's acceptance or report
- [ ] 4.1.2 Confirm each fails at the baseline and commit the red cases

### Step 4.2: Implementation

- [ ] 4.2.1 Return the substituted type from `check_generic_dot_call` (`DEF-395`, `DEF-406`)
- [ ] 4.2.2 Thread and seed explicit type arguments on the dot-call route (`DEF-401`)
- [ ] 4.2.3 Run the focused suites and `make check`; commit the fix

### Step 4.3: Pins and review

- [ ] 4.3.1 Add the implementation pins (the `String` temporary, the instance symbols, the
      one-report count)
- [ ] 4.3.2 Dispatch the phase review; apply corrections
- [ ] 4.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 5: Qualified call route

> **Rows**: `DEF-396`, `DEF-407`
> **Lenses**: correctness, semantics, emission-integrity
> **Reasoning**: high — a qualified callee must reach the same direct/generic checker

### Step 5.1: Red cases

- [ ] 5.1.1 Write the red cases: the qualified generic return as a loop iterable, the qualified
      arity and type-argument-count reports, and the qualified argument conversion
- [ ] 5.1.2 Confirm each fails at the baseline and commit the red cases

### Step 5.2: Implementation

- [ ] 5.2.1 Route a qualified module function through `check_generic_call` or
      `check_args_direct` with its modes and written type arguments (`DEF-396`)
- [ ] 5.2.2 Confirm the loop iterable resolves to the substituted fixed array (`DEF-407`)
- [ ] 5.2.3 Run the focused suites and `make check`; commit the fix

### Step 5.3: Pins and review

- [ ] 5.3.1 Add the implementation pins (the concrete loop element, the report counts, the
      accepted qualified control)
- [ ] 5.3.2 Dispatch the phase review; apply corrections
- [ ] 5.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 6: Initializer verification

> **Rows**: `DEF-402`
> **Lenses**: correctness, emission-integrity
> **Reasoning**: low — verification only

- [ ] 6.1 Add the self-qualified initializer verification case
- [ ] 6.2 Confirm it is green at the batch head; if so, close `DEF-402` as incidentally fixed,
      otherwise record the remaining gap
- [ ] 6.3 Update the roadmap and findings

## Close-out

- [ ] 7.1 Run `make check` and `make verify` at the batch head
- [ ] 7.2 Reconcile the roadmap rows, the findings ledger, and the plan progress
- [ ] 7.3 Commit the documentation close-out

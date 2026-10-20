# Execution Plan — Qualified Access Residue (Batch S)

> **Progress**: 0/24 tasks
> **Rows**: `DEF-397`, `DEF-398`, `DEF-399`, `DEF-400`
> **Baseline**: `5830521`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Generic dot-call inference and bounds

> **Rows**: `DEF-397`
> **Lenses**: correctness, semantics
> **Reasoning**: medium — mirror four existing direct-call judgments

### Step 1.1: Red cases

- [ ] 1.1.1 Write the red cases: `1.fail()` (`E10301`), `1.apply(5)` (`E10303`),
      and the accepted explicit callable-bound dot call
- [ ] 1.1.2 Confirm the rejecting cases fail at the baseline and commit the red cases

### Step 1.2: Implementation

- [ ] 1.2.1 Add the unbound/recursive/bounds/ephemeral judgments to
      `check_generic_dot_call`
- [ ] 1.2.2 Run the focused suite and `make check`; commit the fix

### Step 1.3: Pins and review

- [ ] 1.3.1 Add the implementation pins (report counts/text, accepted control)
- [ ] 1.3.2 Dispatch the phase review; apply corrections
- [ ] 1.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 2: Qualified function values and intrinsics

> **Rows**: `DEF-398`, `DEF-399`
> **Lenses**: correctness, emission-integrity, semantics
> **Reasoning**: high — a codegen path and a resolver/sema surface

### Step 2.1: Red cases

- [ ] 2.1.1 Write the red cases: the qualified function value, the three
      qualified intrinsics, and the cross-module `ffi` call
- [ ] 2.1.2 Confirm each fails at the baseline and commit the red cases

### Step 2.2: Implementation

- [ ] 2.2.1 Resolve a module-qualified function value in `path`
- [ ] 2.2.2 Make the `std.mem` intrinsics qualified-visible
- [ ] 2.2.3 Resolve the qualified intrinsic/ffi call in sema and lower the
      simple intrinsic name
- [ ] 2.2.4 Run the focused suites and `make check`; commit the fix

### Step 2.3: Pins and review

- [ ] 2.3.1 Add the implementation pins (symbol text, intrinsic C, strict runs)
- [ ] 2.3.2 Dispatch the phase review; apply corrections
- [ ] 2.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 3: Module-path precedence

> **Rows**: `DEF-400`
> **Lenses**: correctness, semantics
> **Reasoning**: medium — one precedence guard in the own call checker

### Step 3.1: Red case

- [ ] 3.1.1 Write the imported-head collision pair and the lexical-shadow control;
      confirm the collision fails at the baseline
- [ ] 3.1.2 Commit the red case

### Step 3.2: Implementation

- [ ] 3.2.1 Skip the field/binding probes for a resolved qualified module function
- [ ] 3.2.2 Run the focused suite and `make check`; commit the fix

### Step 3.3: Pins and review

- [ ] 3.3.1 Add the implementation pins (the module symbol, the shadow control)
- [ ] 3.3.2 Dispatch the phase review; apply corrections
- [ ] 3.3.3 Record the phase in `05-findings.md` and the roadmap

## Close-out

- [ ] 4.1 Run `make check` and `make verify` at the batch head
- [ ] 4.2 Reconcile the roadmap rows, the findings ledger, and the plan progress
- [ ] 4.3 Commit the documentation close-out

# Execution Plan — Qualified Access Residue (Batch S)

> **Progress**: 26/26 tasks (the original header undercounted the list by two)
> **Rows**: `DEF-397`, `DEF-398`, `DEF-399`, `DEF-400`
> **Baseline**: `5830521`
> **Head**: `0fac18e`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Generic dot-call inference and bounds

> **Rows**: `DEF-397`
> **Lenses**: correctness, semantics
> **Reasoning**: medium — mirror four existing direct-call judgments

### Step 1.1: Red cases

- [x] 1.1.1 Write the red cases: `1.fail()` (`E10301`), `1.apply(5)` (`E10303`),
      and the accepted explicit callable-bound dot call
- [x] 1.1.2 Confirm the rejecting cases fail at the baseline and commit the red cases

### Step 1.2: Implementation

- [x] 1.2.1 Add the unbound/recursive/bounds/ephemeral judgments to
      `check_generic_dot_call`
- [x] 1.2.2 Run the focused suite and `make check`; commit the fix

### Step 1.3: Pins and review

- [x] 1.3.1 Add the implementation pins (report counts/text, accepted control)
- [x] 1.3.2 Dispatch the phase review; apply corrections
- [x] 1.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 2: Qualified function values and intrinsics

> **Rows**: `DEF-398`, `DEF-399`
> **Lenses**: correctness, emission-integrity, semantics
> **Reasoning**: high — a codegen path and a resolver/sema surface

### Step 2.1: Red cases

- [x] 2.1.1 Write the red cases: the qualified function value, the three
      qualified intrinsics, and the cross-module `ffi` call
- [x] 2.1.2 Confirm each fails at the baseline and commit the red cases

### Step 2.2: Implementation

- [x] 2.2.1 Resolve a module-qualified function value in `path`
- [x] 2.2.2 Make the `std.mem` intrinsics qualified-visible
- [x] 2.2.3 Resolve the qualified intrinsic/ffi call in sema and lower the
      simple intrinsic name
- [x] 2.2.4 Run the focused suites and `make check`; commit the fix

### Step 2.3: Pins and review

- [x] 2.3.1 Add the implementation pins (symbol text, intrinsic C, strict runs)
- [x] 2.3.2 Dispatch the phase review; apply corrections
- [x] 2.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 3: Module-path precedence

> **Rows**: `DEF-400`
> **Lenses**: correctness, semantics
> **Reasoning**: medium — one precedence guard in the own call checker

### Step 3.1: Red case

- [x] 3.1.1 Write the imported-head collision pair and the lexical-shadow control;
      confirm the collision fails at the baseline
- [x] 3.1.2 Commit the red case

### Step 3.2: Implementation

- [x] 3.2.1 Skip the field/binding probes for a resolved qualified module function
- [x] 3.2.2 Run the focused suite and `make check`; commit the fix

### Step 3.3: Pins and review

- [x] 3.3.1 Add the implementation pins (the module symbol, the shadow control)
- [x] 3.3.2 Dispatch the phase review; apply corrections
- [x] 3.3.3 Record the phase in `05-findings.md` and the roadmap

## Review and corrections

- [x] R.1 First four-lens review at `c14e81b`; close the intrinsic, value-position,
      nullability, module-path, and generic explicit-list findings in `814d04d`
- [x] R.2 Second four-lens confirmation at `814d04d`; close the receiver,
      folded-operand warning, initializer function value, and non-call operand
      findings in `b6b3855` and `f40da32`
- [x] R.3 Third confirmation at `f40da32`; move the non-call effectful operand
      shapes and the module-initializer own-call gap to `DEF-531`/`DEF-532`,
      record the oracle-versus-BR-9 conflict as Deferred `DEF-533`, and keep the
      non-call shapes at the structural fold in `0fac18e`

## Close-out

- [x] 4.1 Run `make check` and `make verify` at the batch head (`0fac18e`; verify:
      all legs passed)
- [x] 4.2 Reconcile the roadmap rows, the findings ledger, and the plan progress
- [x] 4.3 Commit the documentation close-out

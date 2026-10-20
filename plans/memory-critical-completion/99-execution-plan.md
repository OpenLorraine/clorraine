# Execution Plan — Memory and Borrow Critical Completion (Batch P)

> **Progress**: 35/35 tasks
> **Rows**: `DEF-485`, `DEF-486`, `DEF-487`, `DEF-488`, `DEF-489`, `DEF-493`, `DEF-495`,
> `DEF-496`, `DEF-497`, `DEF-498`, `DEF-499`, `DEF-501`, `DEF-502`, `DEF-503`, `DEF-504`
> **Baseline**: `efc68f9`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Loan-carrying copies

> **Rows**: `DEF-485`, `DEF-486`, `DEF-493`, `DEF-497`, `DEF-498`, `DEF-499` (all critical)
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — the fix changes loan transfer and escape judgment

### Step 1.1: Red cases

- [x] 1.1.1 Write the red cases in `borrow_store_integrity_spec_test.rs`: the `const` conditional
      copy (`DEF-485`), the global conditional copy (`DEF-486`), the call-carrier copy (`DEF-493`),
      the mixed aggregate (`DEF-497`), the conditional shadow (`DEF-498`), and the alias element
      return (`DEF-499`). Each asserts the twin's code once, no translation unit, and a green
      control
- [x] 1.1.2 Confirm every case fails at the baseline (accepted+UAF or wrong report), then commit
      the red cases

### Step 1.2: Implementation

- [x] 1.2.1 Share the carrier-copy branch with `check_const` (`DEF-485`)
- [x] 1.2.2 Judge a global conditional carrier's arms at the store (`DEF-486`)
- [x] 1.2.3 Add `call_carrier_source` and use it in the copy collector (`DEF-493`)
- [x] 1.2.4 Fall through to the owned-field judgment for a mixed aggregate (`DEF-497`)
- [x] 1.2.5 Snapshot conditional copy loans before the declaration (`DEF-498`)
- [x] 1.2.6 Resolve an alias root in `carrier_return_owners` (`DEF-499`)
- [x] 1.2.7 Run the focused suites and `make check`; commit the fix

### Step 1.3: Pins and review

- [x] 1.3.1 Add the implementation pins (the destination positions, the report count/text,
      byte-identical accepted controls)
- [x] 1.3.2 Dispatch the four-lens phase review; apply corrections and bounded confirmation
- [x] 1.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 2: Call-mediated view escapes

> **Rows**: `DEF-487`, `DEF-488`, `DEF-489`, `DEF-495`, `DEF-496` (all critical)
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — the fix changes the call boundary and the source-resolution order

### Step 2.1: Red cases

- [x] 2.1.1 Write the red cases in `eval_emission_integrity_spec_test.rs`: the returned
      call-mediated view, the inline carrier, the conditional call arm, the closure/func-typed/
      generic/wrapper calls, and the dot receiver; each asserts one report and a green control
- [x] 2.1.2 Confirm every case fails at the baseline and commit the red cases

### Step 2.2: Implementation

- [x] 2.2.1 Route the view return through the call source and its loans (`DEF-487`)
- [x] 2.2.2 Use `call_carrier_source` in the cross-frame and copy judgments (`DEF-488`, `DEF-489`)
- [x] 2.2.3 Resolve closure, func-typed, generic, and wrapper calls (`DEF-495`)
- [x] 2.2.4 Include the dot receiver at parameter zero (`DEF-496`)
- [x] 2.2.5 Run the focused suites and `make check`; commit the fix

### Step 2.3: Pins and review

- [x] 2.3.1 Add the implementation pins and the byte-identical accepted controls
- [x] 2.3.2 Dispatch the four-lens phase review; apply corrections and bounded confirmation
- [x] 2.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 3: Generic per-instance conditional sources

> **Rows**: `DEF-501`, `DEF-502`, `DEF-503`, `DEF-504` (all critical)
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — the fix changes the per-instance duty set

### Step 3.1: Red cases

- [x] 3.1.1 Write the red cases in `borrow_generic_integrity_spec_test.rs`: the generic take, the
      raw field, the struct-literal field and projected assignment, and the nested projection;
      each asserts the instance report count and a duplicable control
- [x] 3.1.2 Confirm every case fails at the baseline and commit the red cases

### Step 3.2: Implementation

- [x] 3.2.1 Record the take path's type-parameter conditional sources (`DEF-501`)
- [x] 3.2.2 Strip `raw` before the audit's field step (`DEF-502`)
- [x] 3.2.3 Route the struct-literal field and projected assignment arms through the walk
      (`DEF-503`)
- [x] 3.2.4 Supplement the projection return with the maybe-owning walk (`DEF-504`)
- [x] 3.2.5 Run the focused suites and `make check`; commit the fix

### Step 3.3: Pins and review

- [x] 3.3.1 Add the implementation pins and the byte-identical accepted controls
- [x] 3.3.2 Dispatch the four-lens phase review; apply corrections and bounded confirmation
- [x] 3.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 4: Close-out

- [x] 4.1 Run `make check` and `make verify`; record the focused leg counts
- [x] 4.2 Record the close-out (rows, commits, residuals) and sync `plans/00-roadmap.md`
- [x] 4.3 Refresh the specialist check if a new plan landed; leave the tree clean

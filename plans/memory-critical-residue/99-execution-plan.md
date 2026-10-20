# Execution Plan — Memory Critical Residue (Batch Q)

> **Progress**: 23/32 tasks
> **Rows**: `DEF-508`, `DEF-509`, `DEF-510`, `DEF-511`, `DEF-512`, `DEF-514`, `DEF-515`,
> `DEF-518`, `DEF-519`, `DEF-521`, `DEF-523`
> **Baseline**: `6a67880`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Carrier sources and callable expressions

> **Rows**: `DEF-508`, `DEF-509`, `DEF-510`, `DEF-511`, `DEF-512`, `DEF-519` (all critical)
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — the fix changes the call boundary, the source set, and the emission of
> value callees

### Step 1.1: Red cases

- [x] 1.1.1 Write the red cases in `eval_emission_integrity_spec_test.rs`: the projected
      carrier member, the two-source pair, the inline lambda and conditional func callee, the
      closure capture out-slot, the `take` identity and two-argument helper, and the returned
      struct-literal carrier; each asserts the twin's code once or a clean strict run, plus a
      green control
- [x] 1.1.2 Confirm every case fails at the baseline (accepted+UAF, invalid C, or the identity
      false rejection), then commit the red cases

### Step 1.2: Implementation

- [x] 1.2.1 Replace the single carrier source with the plural source set and iterate it in
      every consumer (`DEF-509`, `DEF-512`)
- [x] 1.2.2 Descend a member/index chain rooted at a carrier call in the frame judgment and
      the copy collector (`DEF-508`)
- [x] 1.2.3 Stop the return from treating a returned take parameter as frame-dying; keep the
      caller-side store judgment (`DEF-512`)
- [x] 1.2.4 Descend a struct/array literal's view positions in the frame judgment (`DEF-519`)
- [x] 1.2.5 Resolve a lambda and a conditional callable in `callable_call_signature`, the call
      check, and `lower_call` (`DEF-510`)
- [x] 1.2.6 Treat a by-value capture as frame-dying in the owner and provenance judgments
      (`DEF-511`)
- [x] 1.2.7 Run the focused suites and `make check`; commit the fix

### Step 1.3: Pins and review

- [x] 1.3.1 Add the implementation pins (report count and text, emitted carrier/callee shapes,
      strict-build and byte-identical accepted controls)
- [ ] 1.3.2 Dispatch the four-lens phase review; apply corrections and bounded confirmation
- [ ] 1.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 2: Generic conditional wrappers

> **Rows**: `DEF-514`, `DEF-515`, `DEF-518`, `DEF-521` (all critical)
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — the fix changes per-instance duties and the emitted take move-out

### Step 2.1: Red cases

- [x] 2.1.1 Write the red cases in `borrow_generic_integrity_spec_test.rs`: the conditional take
      of a take parameter's member and its `unsafe` twin (clean runs), the owned array-literal
      element, and the `unsafe` wrapped struct field; each asserts the instance report or a clean
      sanitizer run, plus a duplicable control
- [x] 2.1.2 Confirm every case fails at the baseline and commit the red cases

### Step 2.2: Implementation

- [x] 2.2.1 Accept a conditional take with at least one owning place arm and lower it through
      the shared carrier (`DEF-514`, `DEF-515`)
- [x] 2.2.2 Unwrap `unsafe`/casts in the unresolved conditional arm walk (`DEF-515`)
- [x] 2.2.3 Record the type-parameter conditional duty for array-literal elements and fill
      (`DEF-518`)
- [x] 2.2.4 Unwrap `unsafe`/casts in the type-parameter conditional source walk (`DEF-521`)
- [x] 2.2.5 Run the focused suites and `make check`; commit the fix

### Step 2.3: Pins and review

- [x] 2.3.1 Add the implementation pins and the byte-identical accepted controls
- [ ] 2.3.2 Dispatch the four-lens phase review; apply corrections and bounded confirmation
- [ ] 2.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 3: Moved state

> **Rows**: `DEF-523` (critical)
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — the fix changes the moved-state lattice

### Step 3.1: Red cases

- [x] 3.1.1 Write the red cases in `borrow_generic_integrity_spec_test.rs`: the generic and
      concrete nested write below a moved field report `E10020` at each use site, and the whole-field
      re-initialization control stays accepted and clean
- [x] 3.1.2 Confirm every case fails at the baseline and commit the red cases

### Step 3.2: Implementation

- [x] 3.2.1 Compare moved paths against the written path and keep the moved prefix (`DEF-523`)
- [x] 3.2.2 Run the focused suites and `make check`; commit the fix

### Step 3.3: Pins and review

- [x] 3.3.1 Add the implementation pins and the accepted control
- [ ] 3.3.2 Dispatch the four-lens phase review; apply corrections and bounded confirmation
- [ ] 3.3.3 Record the phase in `05-findings.md` and the roadmap

## Phase 4: Close-out

- [ ] 4.1 Run `make check` and `make verify`; record the focused leg counts
- [ ] 4.2 Record the close-out (rows, commits, residuals) and sync `plans/00-roadmap.md`
- [ ] 4.3 Refresh the specialist check if a new plan landed; leave the tree clean

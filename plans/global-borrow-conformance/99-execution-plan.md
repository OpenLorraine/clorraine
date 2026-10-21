# Execution Plan — Global Borrow and Move Conformance (Batch U)

> **Progress**: 4/12 tasks
> **Rows**: `DEF-533`, `DEF-537`, `DEF-425`
> **Baseline**: `513fd90`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Uniform BR-9 (`DEF-533`)

- [x] 1.1.1 Remove the body receiver carve-out and the initializer
      module-qualified / unmarked scalar-view exemptions
- [x] 1.1.2 Check a view builder's non-`const` global source
- [x] 1.2.1 Retarget the conflicting oracles and emission pins to `const`,
      local, or call-result sources
- [x] 1.2.2 Add the rejection spec cases for the qualified receiver, view
      builder, and unmarked arguments; run `make check` and `make verify`

## Phase 2: Func-typed initializer calls (`DEF-537`)

- [ ] 2.1.1 Resolve a func-typed global's signature and judge its arguments
- [ ] 2.1.2 Resolve a func-typed field's signature and judge its arguments
- [ ] 2.2.1 Add the rejection and accepted-control spec cases; run the
      focused suites and `make check`

## Phase 3: Generic element move (`DEF-425`)

- [ ] 3.1.1 Record the borrowed-element move and judge it per instance
- [ ] 3.1.2 Retarget the pinned batch-M acceptance cases to the concrete/take
      rejection
- [ ] 3.2.1 Add the generic/concrete/take matrix and the duplicable control;
      run `make check`

## Close-out

- [ ] 4.1 Run `make check` and `make verify` at the batch head
- [ ] 4.2 Record the phase findings and update the roadmap rows

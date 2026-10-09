# Requirements: Defer Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Close the five filed `defer` defects (DEF-84 – DEF-88) in the Clorraine compiler. A `defer` is
code motion that joins its scope's cleanup ladder and runs on every exit edge — the normal scope
end, `return`, `break`, and `continue` (frozen rules: `spec/05-statements.md` §13 and §15,
`spec/17-codegen.md` §4 CL-1–CL-7). The five defects are implementation deviations from those
rules: an undefined cleanup label (strict C build failure), a deferred-return self-loop (hang), a
defer skipped on the fall-through edge, an earlier returning defer that never fires, and a
deferred return that reads its own local after the drop (segfault). The fix is one shared
deferred-copy emission context plus a gated return-ordering change, per the AR-4 design; no
language feature, no `spec/` change, no new dependency.

## Functional Requirements

### Must Have

- [ ] **R1 (DEF-84):** A `continue` or `break` inside a deferred block reaches the loop's
  corresponding exit edge; the emitted unit builds under the strict C99 contract
  (`-std=c99 -Wall -Wextra -Werror`); the deferred copy never swallows the enclosing loop or
  emits a `goto` to a never-defined label. (AR-6)
- [ ] **R2 (DEF-85):** A `return` inside a deferred block evaluates its value, runs the remaining
  cleanup of the scope whose cleanup is executing (its later-registered actions excluded, its
  earlier ones included) and then the outer scopes' cleanup, and returns that value exactly once;
  no self-jump and no hang on the function-scope, switch-arm, and outer-droppable shapes. (AR-5)
- [ ] **R3 (DEF-86):** A deferred block runs on **every** exit edge of its scope: the normal end,
  `return`, `break`, and `continue`, including the fall-through path when the scope also contains
  a continue edge and when an early return leaves the scope. (AR-7)
- [ ] **R4 (DEF-87):** Multiple defers in one scope run in reverse registration order on every
  exit path; an earlier returning defer fires on every path that reaches its scope exit,
  regardless of a later defer's guarded early exit. (AR-7)
- [ ] **R5 (DEF-88):** A `return` inside a scope with inline cleanup (a deferred block's own
  scope, or a loop body) evaluates its value before any cleanup that releases storage the value
  reads; the defer-local and the loop-local shapes are host-AddressSanitizer clean. (AR-8)
- [ ] **R6 (regression):** Programs that do not exercise the five shapes keep byte-identical
  emission where the design says so (defer-free programs, byte-identical except the accepted
  removal of lone `#line` directives for empty cleanup scopes; single-site defers without loop
  edges or returns); every existing specification and implementation suite stays green; the
  corpus emission sweep shows only traced, intended diffs. (AR-4, AR-10)

### Should Have

- [ ] The deferred block's effects are checked on every exit edge (the checker's defer entry
  survives later cleanup sites), so a deferred move or loan is visible on each path. (AR-7)

### Won't Have (Out of Scope)

- No `spec/` chapter, registry, or golden change (AR-2).
- No new language feature, diagnostic, dependency, crate, or harness (AR-14).
- The other defect batches (DEF-77–79/81–83 and DEF-76/80/90/92–94) and DEF-95 (AR-1).
- A generalized cleanup framework or per-edge IR re-lowering beyond the AR-4 design (AR-14).
- The rung-hosted `break`/`continue` sibling shape is covered only by the inline-only guard
  (AR-6); the residual is reported, not claimed as fully closed.

## Technical Requirements

### Compatibility

- The frozen specification is untouched; every ST expectation derives from an existing rule or
  the reproduced defect evidence (AR-2).
- The emitted unit stays within the strict C99 subset (`spec/17-codegen.md` §1): no new C
  constructs, no compound literals, no unused labels (`-Werror`).
- Determinism: two emissions of one program are byte-identical; label/copy numbering must be
  stable across the emitter's preparation and emission passes (AR-4 reopen trigger).

### Performance

- No compile-time or code-size target beyond the existing conventions; per-site copies are
  bounded by the number of cleanup sites, mirroring the "copied into the ladder" model already
  accepted by the spec (AR-4).

### Security

- N/A — a compiler-internal change with no user-input path, network surface, secret, or
  authentication concern. The relevant safety property (memory safety of emitted code) is
  verified by the host AddressSanitizer probes and the memory-safety auditor (AR-13).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch scope | A: defer family · B: borrow/generic · C: lambdas/diagnostics/emission · D: DEF-95 | A | User instruction ("the next logical defect batch") + the only fully cohesive batch (five majors, one subsystem, one review) | AR-1 |
| Spec stance | A: implementation-only · B: amend | A | The rules already define the behavior; the defects are deviations | AR-2 |
| Modification set | A: compiler stages + tests + plan folder · B: also `spec/` | A | Minimal surface; no spec change needed | AR-3 |
| Core design | A: shared copy context · B: per-defect patches · C: re-model at lowering | A | Challenger-verified: B is incorrect (leaks, duplicate labels, skipped cleanup), C violates byte-identity | AR-4 |
| Return continuation | A: remaining actions, then outer rungs · B: skip own rung | A | `d85c` proves B leaks the remaining drop | AR-5 |
| Loop edges | A: context + bounded closure + inline guard · B: reject the shapes | A | The programs are accepted by the checker and the spec; rejection contradicts §13 | AR-6 |
| Every-edge copies | A: per-site copies + namespaces · B: one-shot re-anchored | A | The evidence (D86/D87) requires each site to run the defer | AR-7 |
| Return ordering | A: generic value-before-cleanup · B: defer-scoped | A | One root cause (the loop-local shape reproduces without defer); B leaves it | AR-8 |

> **Traceability:** Every scope decision references the Ambiguity Register entry (AR #) that
> resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] All ST cases in [07-testing-strategy.md](07-testing-strategy.md) are red at their phase
   baseline for the recorded reason and green at the phase's end.
2. [ ] The implementation suites pin the new emission shapes and the red/green edge cases.
3. [ ] Host AddressSanitizer is clean on every changed shape and on the probe matrix; the strict
   host build accepts every accepted program.
4. [ ] `make check` is green after every phase; `make verify` reports all legs passed at the end.
5. [ ] No new warnings; no dead code; the corpus emission sweep shows only traced, intended diffs.
6. [ ] The roadmap rows for DEF-84 – DEF-88 move to Done; the AGENTS.md test-naming map records
   the new suites.

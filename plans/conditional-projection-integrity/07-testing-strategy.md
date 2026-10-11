# Testing Strategy: Conditional and Projection Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Suites

| Suite | Purpose |
|-------|---------|
| `compiler/tests/conditional_projection_integrity_spec_test.rs` | **Specification tests** — each row's reproduction derived from the frozen rules: the rejected shapes report exactly the pinned code once; the accepted shapes build (strict flags) and run; the emitted C behaves |
| `compiler/tests/conditional_projection_integrity_impl_test.rs` | **Implementation tests** — the emitted shapes (the per-arm move-out, the guarded fill, the statement temporaries, the unified conditional return, the pinned postfix), the byte-identity pins for the accepted references, and the report count/span/message pins |

Both suites are listed in `AGENTS.md`'s test naming map at close-out.

## Methodology

1. **Spec-first red.** Each phase writes its specification cases first and runs them at the phase
   baseline tree; every case must fail with the filed symptom (the sanitizer abort, the leak, the
   missing report, the strict-build failure). A case that passes at the baseline is a finding
   against the case.
2. **Green, then pins.** The phase's implementation runs until the spec suite is green; the
   implementation pins then capture the emitted shapes and the diagnostic text/counts.
3. **Strict builds everywhere.** The spec suite's run helper builds the emitted C with
   `-Wall -Wextra -Werror`; accepted shapes also emit for `freestanding` and `hosted-amiga` in the
   phase verification.
4. **AddressSanitizer.** Every ownership case (phases 1, 2, 3, 4) runs under `RDC_ASAN=1` (the host
   AddressSanitizer build) in addition to the plain run; the double-free and use-after-free shapes
   are probed directly with leak detection on.
5. **Corpus differential.** Each codegen/IR phase's review includes the emission auditor's
   base/head differential over the tracked corpus and the inline suite programs, classifying every
   delta as intended (the new statement temporary, the unified arm spelling, the pinned update, the
   bound call root, the intrinsic call spelling) or a finding.
6. **Diagnostic hygiene.** Every rejection case pins the code, the count, and the site span; every
   message case pins the rendered text.
7. **Determinism.** Changed shapes re-emit byte-identically on a second run.

## Case Inventory

The per-phase case tables live in the phase documents (`03-01` … `03-05`); this document owns the
conventions above. The suite grows by one case group per phase; the counts are recorded in the
execution plan and the findings ledger.

## Acceptance

- Every case fails at its phase baseline and passes at the phase head.
- `make check` (all suites) and `make verify` (all cross-toolchain legs) pass at the close-out head;
  the ASan suite passes where required.
- No accepted shape in the corpus changes behavior or emission beyond the classified deltas.

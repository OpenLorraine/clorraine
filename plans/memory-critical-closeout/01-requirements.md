# Requirements

The rows are reproduced in `02-current-state.md`; the acceptance criteria below are the oracle.
Every report is the existing diagnostic the concrete twin uses; no new diagnostic is allowed.

| R | Row | Severity | Requirement and acceptance |
|---|-----|----------|----------------------------|
| R1 | `DEF-449` | critical | A conditional copy of a carrier that holds element loans transfers those loans to the destination. After `let h2: Holder = c ? h1 : h3;`, writing through the loaned owner and then reading `h2` must be rejected with the concrete twin's diagnostic (`E10034`/`E10026` per the frozen borrow rule), exactly once. The unconditional-overwrite and mixed place/literal variants obey the same rule. The current accepted run is a heap-use-after-free and must become a front-end report. |
| R2 | `DEF-450` | critical | A call whose borrowed aggregate argument's resolved parameter may carry a view out-slot is judged at the cross-frame boundary. `func store(h: Box, borrow slot: str?) { slot = h.v; }` called with a local `b` that dies before the slot's owner reads it reports the existing escape diagnostic (`E10030`/`E10033`) once. The non-escaping caller-backed case stays accepted. |
| R3 | `DEF-451` | critical | A call-mediated view of an inline temporary view-array (`pick([mk().v; "EE"], 0)`, and the direct argument form) reports the existing escape diagnostic at the call, not a heap-use-after-free. The same call over a caller-backed array stays accepted. |
| R4 | `DEF-453` | critical | A call-mediated view stored in a global or a global-backed destination (`g = pick(xs, 0)` after `xs[0] = a.v`) reports `E10033`, exactly once. The module-level source case stays accepted. |
| R5 | `DEF-459` | critical | A pure pointer place (`*ps[idx]`) evaluated before a call whose callee mutates the index or pointer is captured before the call; the take move-out and the self-alias test use the captured pointee. The program has no double free, no use-after-free, and no SEGV under ASan; the non-mutating twin is unchanged. |
| R6 | `DEF-468` | critical | A generic conditional return of a lend arm (`func f<T>(c, h: T): String? { return c ? h : null; }` at `T = String`) reports the concrete twin's `E10034` per instance (or is rejected once in the generic body if the frozen rule makes the body invalid). No double free. |
| R7 | `DEF-469` | critical | A generic field-projection conditional arm (`let r: String? = c ? h.name : null;` with `h: T`, `T = H { name: String }`) records the per-instance duty and reports the concrete twin's `E10034`; no double free. |
| R8 | `DEF-452` | minor | Recorded ruling: Ch 03 D5 (`spec/03-ownership.md:169`) drops a source temporary at the end of its statement. A generated owner bound for a temporary keeps that statement scope; a next-statement read of a view derived from the temporary is rejected with `E10030`, while a same-statement read stays accepted. The phase-2 pin that codified the body-scope extension is corrected. |

## Constraints

- Every fix is a conformance fix; `spec/` and the diagnostics registry are not edited.
- A spec test is the immutable oracle for the batch; an existing Rust pin that contradicts a frozen
  chapter is corrected only under the R8 ruling and with the correction recorded.
- Reports are deterministic (same site, same count, same text) and the emitted C stays byte-identical
  for the accepted controls.
- The host ASan/LSan legs run for every accepted case that owns storage.

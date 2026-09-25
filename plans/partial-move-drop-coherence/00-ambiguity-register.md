# Ambiguity Register: Partial-Move / Drop Coherence

> **Status**: ✅ GATE PASSED — all 17 items resolved
> **Last Updated**: 2026-09-25 23:38

## Scope

Fix **DEF-11**: moving an owned field out of a value is not reflected in the
emitted C, so a later drop of the source frees a value the destination also
owns (a double free). Reproduced for the struct-literal reassignment
(`b = Box<String> { value: b.value };`) and the temporary-binding form
(`let t = b.value; b = Box<String> { value: t };`), for generic and non-generic
structs. The fix realizes the invariant the frozen spec already states:
a move lowers to a copy plus a zero of the source, and the zeroed source is the
drop flag (`spec/17-codegen.md:182-199`, P3).

## Register

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | Breadth of the fix | (a) general invariant; (b) struct-literal only | (a) general invariant — one rule: a value-consuming read of an owned place moves it and zeroes the source | ✅ Resolved |
| AR-2 | Behavioral | Required result when an owned field is moved out and the source is later dropped or reassigned | (a) free exactly once; partial moves stay legal; source later drops only its remaining fields; whole reassignment reinitializes | (a) | ✅ Resolved |
| AR-3 | Technical | Where the invariant is realized | (a) ownership lowering emits the move + source zero for a value-consuming nested place read, and the assignment lowering releases the old target after the RHS is evaluated; (b) codegen-only structural patching of each site | (a) ownership lowering emits the move + source zero for a value-consuming nested place read; the assignment lowering evaluates the RHS, then releases the old target, then stores | ✅ Resolved |
| AR-4 | Testing | Regression oracle and placement | (a) ASan compile-and-run plus emitted-C assertions; (b) emitted-C only | (a) ASan runtime + emitted-C; spec cases in `compiler/tests/codegen_run_spec_test.rs` and `compiler/tests/stage_spec_test.rs`, plus an ASan compile-and-run helper in `compiler/tests/` | ✅ Resolved |
| AR-5 | Technical | Verify commands | (a) `make check` per task; final `make verify`; (b) `make check` only | (a) `make check` per task; final `make verify` | ✅ Resolved |
| AR-6 | Naming | Plan folder slug | (a) `partial-move-drop-coherence`; (b) `selfassign-double-free` | (a) `plans/partial-move-drop-coherence/` | ✅ Resolved |
| AR-7 | Behavioral | Spec basis: does the frozen spec define the behavior? | It does; no spec edit | Move lowers to copy + zero source; the zeroed source is the drop flag; partial move is `dst = s.field; cl_zero(&s.field);` (`spec/17-codegen.md:182-199`); partial moves and re-initialization (`spec/03-ownership.md:123-134`, `spec/07-structs.md:87-121`). No `spec/` edit | ✅ Resolved |
| AR-8 | Non-functional | Non-move programs must not change | Existing byte-identity and run suites pin the unchanged cases | Programs with no partial move keep byte-identical emitted C (pinned by the determinism and corpus suites) | ✅ Resolved |
| AR-9 | Security & compliance | User-input / injection / data-protection surface | N/A | Compiler internal; the fix closes a memory-safety hole (double free); no external input surface | ✅ Resolved |
| AR-10 | Technical | Does the lowering change trigger the Complexity Escalation Gate? | It is a spec-required correction of existing machinery, not a new layer/dependency/harness | No escalation: the change realizes an existing spec invariant (`spec/17-codegen.md` P3) in the existing ownership and codegen stages; no new abstraction, dependency, or support surface | ✅ Resolved |
| AR-11 | Behavioral | Generic vs non-generic coverage | The root is general | The fix applies to both; DEF-11 was only *surfaced* through a generic struct (`Box<String>`) | ✅ Resolved |
| AR-12 | Edge cases | Borrow RHS must keep working | Existing behavior preserved | `f = Dup(borrow f)` still drops the old value after the RHS and emits `open/dup/close/close` (`codegen_run_spec_test.rs:690-722`) | ✅ Resolved |
| AR-13 | Edge cases | Self-assignment of an owned value | Free once | `s = s` must zero the source through the move, drop nothing, and store back; frees once at scope exit | ✅ Resolved |
| AR-14 | Edge cases | Multi-field and repeated reads | Move each owned field once | `s = S { a: s.a, b: s.b }` moves both fields; a field read twice must not move twice (the first read moves it; the second is a use-after-move, already `E10020`) | ✅ Resolved |
| AR-15 | Scope | Out of scope | v1 partial-move limits stand | Deeper-than-top-level partial moves stay `E10026`; arrays of owners and owner elements are out of scope; global places are out of scope | ✅ Resolved |
| AR-16 | Integration points | Interface affected | Internal only | Internal lowering and codegen only; no CLI or artifact contract change | ✅ Resolved |
| AR-17 | Dependency | Must the paused generic-field-integrity plan's Phase 4 wait? | It waits on DEF-11 | PLAN-10 is `⛔ Blocked (was: Executing)` on DEF-11; Phase 4 resumes after this plan completes | ✅ Resolved |

## Pending confirmation

_(none — the gate passed)_

## Resolution Notes

**AR-1/AR-2:** The user chose the general invariant. Every subsequent decision
must serve free-once semantics for all value-consuming move-out forms, not just
the struct-literal spelling.

**AR-3:** The user confirmed the lowering + assignment approach. A non-duplicable
place read consumed as a value is lowered as a move that zeroes the source; the
assignment lowering evaluates the RHS into a temporary, releases the old target,
then stores, so no stale save-copy can alias a moved-out value.

**AR-4:** The user confirmed ASan runtime + emitted-C, placed in the spec suites
with an ASan helper in `compiler/tests/`.

> **Traceability:** Every design decision, scope decision, and error-handling
> strategy in the plan documents back-references the AR entry that resolved it.

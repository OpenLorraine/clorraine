# Ambiguity Register: Ownership and Drop Model Completion

> **Status**: ✅ GATE PASSED — all 32 items resolved (22 at creation, 7 added by the preflight review, 3 added at runtime)
> **Last Updated**: 2026-09-26 19:59

## Scope

This plan fixes the confirmed version-1 ownership and drop-model defects in
`plans/00-roadmap.md` PLAN-11. The defects are: a fixed array is misclassified
as a view (F1), a slice view field is accepted (F2), owning-element drop glue
for arrays is staged (F3), and a move-only fill literal aliases its value (F4).
The preflight review added four more confirmed gaps: a fixed-array return cannot
be emitted (PF-001), a nested array literal emits invalid C (PF-003), a fixed
array is still nullable (PF-004), and `set` leaks the replaced element (PF-007).
The plan adds no language surface; it makes the implementation match the frozen
specification chapters 03, 08, 11, 16 and the standard-library appendix. The
ledger-deferred items (nested partial moves O4, per-field loan precision O8,
aggregate view returns F2, `Box<T>` X1, and non-lexical lifetimes) stay out of
this plan.

## Register

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | Which work the plan covers | (a) the confirmed v1 defects F1–F3 only, keeping O4/O8/F2-view-returns/X1/NLL deferred; (b) also pull one or more deferred items forward (needs a ledger change) | (a) confirmed 2026-09-26 | ✅ Resolved |
| AR-2 | Scope | Owning-element drop-glue scope | (a) implement fixed-array **and** growable `Array<T>` element drop, and un-stage `E90000`; (b) fix fixed arrays only and keep growable `Array<move-only>` staged | (a) confirmed 2026-09-26 | ✅ Resolved |
| AR-3 | Technical | `E90000` loses its only trigger once `Array<move-only>` is accepted | (a) delete `E90000` from the registry, the exclusion list, and the staged golden; (b) keep `E90000` as the dormant generic staged fallback, delete only the now-passing staged golden, keep it excluded | (b) — the minimal consequence of the authorized un-stage (AR-2); it changes no normative chapter, only the staged test data and the exclusion comment. Amended at execution (review RV-011): `E90000` keeps one live trigger — a fixed-array `Array` element — so it is removed from the exclusion list and gains a live compile-fail golden; its other trigger is gone | ✅ Resolved |
| AR-4 | Behavioral | Which view-typed fields `E10205` rejects | (a) reject `str` and slice (`T[]`) fields, keep `cstring`, `static str`, `raw`, `func`, handles, structs, and fixed arrays; (b) keep rejecting only `str` | (a) confirmed 2026-09-26; `spec/08-arrays-strings.md:289` says "view-typed struct field other than `static str`", and `T4`/`T9` ban slice fields | ✅ Resolved |
| AR-5 | Technical | How the two array shapes are distinguished | (a) one predicate `Ty::is_slice` (an `Array` with any omitted extent), used by every consumer, with the fixed array as the complement; (b) a per-site check | (a) — a single predicate is what stops the stages from drifting apart again | ✅ Resolved |
| AR-6 | Behavioral | How an array-literal element is consumed | (a) consume/move an owned element exactly like a struct-literal field, in both the ownership walk and the lowering; (b) keep copying elements | (a) confirmed 2026-09-26; a move copies then empties the source (`spec/08` section 6, O1/O3) | ✅ Resolved |
| AR-7 | Behavioral | Array drop order | Fixed array: elements first-to-last, no separate storage step. Growable `Array<T>`: elements first-to-last, then the storage. | Grounded in O2 (`spec/decisions.md:80`) and `spec/appendix-stdlib.md:66-67`; no new decision | ✅ Resolved |
| AR-8 | Technical | How growable `Array<T>` element drop is emitted | (a) an inline loop in the drop ladder that walks the buffer through the existing `cl_buffer_ptr` FFI, reusing the existing inline drop ladder; (b) a new generated runtime helper or harness | (a) — ordinary feature code in an existing pattern; no new support surface | ✅ Resolved |
| AR-9 | Behavioral | A fixed-array return is currently rejected as a view return (`E10036`) | The function `func pair(): String[N]` from `spec/08` section 6 must compile and return by value; the return path is verified and fixed if needed | Grounded in `spec/08-arrays-strings.md:158-171`; no new decision | ✅ Resolved |
| AR-10 | Edge cases | A fill literal (`[x; fill]`, `[; fill]`) of a move-only element currently aliases the value and double-frees it | (a) reject the fill form when the element type is not duplicable, extending the existing array-literal code `E10215` and its chapter note; (b) leave it unsound and record an accepted risk | (a) confirmed 2026-09-26 | ✅ Resolved |
| AR-11 | Edge cases | Nested fixed arrays (`T[N][M]`), a growable array with zero elements, a fixed array as a struct field, and duplicable elements that need no element drop | Covered by the classification (AR-5) and the drop design (AR-7, AR-8); each is an explicit test | Design follows from AR-5/AR-7/AR-8 | ✅ Resolved |
| AR-12 | Testing | Test placement | (a) new tests in the Rust suites plus the existing build-and-run oracle, with no new `spec/` goldens (the `E90000` staged golden is removed per AR-3); (b) add new `spec/` goldens | (a) confirmed 2026-09-26 | ✅ Resolved |
| AR-13 | Technical | Verification commands | (a) `make check` per task and `make verify` before completion; (b) `make check` only | (a) confirmed 2026-09-26 | ✅ Resolved |
| AR-14 | Naming | Plan folder slug and database link | (a) `plans/ownership-drop-completion/`, `Implements: PLAN-11`; (b) another name | (a) confirmed 2026-09-26 | ✅ Resolved |
| AR-15 | Integration points | Interfaces the change affects | Internal only: `types`, `borrow`, `own`, `sema`, and `codegen`. No CLI, ABI, profile, or emitted-artifact contract changes | N/A — grounded in the change surface | ✅ Resolved |
| AR-16 | Security & compliance | External input, injection, auth, or data protection | N/A — a compiler-internal change. The change improves memory safety, which is the reverse of a security regression | N/A | ✅ Resolved |
| AR-17 | Non-functional | Performance, scalability, availability | No target. The element-drop loop is bounded by the element count; type checking cost is unchanged in order | N/A | ✅ Resolved |
| AR-18 | Data & state | Data model, ownership, or persisted format | N/A — no persisted format or data model is involved | N/A | ✅ Resolved |
| AR-19 | Stakeholder conflicts | Competing priorities or permission boundaries | N/A — single maintainer | N/A | ✅ Resolved |
| AR-20 | UX & presentation | Diagnostic wording | Reuse the existing `E10205` text; AR-10 chose (a), so extend the existing `E10215` text in `spec/14-diagnostics.md` and the `AL` rules in `spec/08-arrays-strings.md`; no new code is introduced | AR-10(a) applies — extend `E10215`, no new code | ✅ Resolved |
| AR-21 | Technical (complexity escalation) | Is the change a material support surface? | (a) ordinary feature code in existing patterns, no escalation; (b) full escalation packet | (a) — the work is edits to existing type, checker, and emitter code plus the existing build-and-run oracle; no new layer, dependency, harness, or infrastructure | ✅ Resolved |
| AR-22 | Behavioral | A slice's duplicability | A slice copies freely (it is a view); the current `duplicable_at` wrongly recurses into the element. The classification predicate (AR-5) also fixes duplicability | Grounded in `spec/03-ownership.md:21` and T9; no new decision | ✅ Resolved |
| AR-23 | Technical | A fixed array must not be nullable now that it is owned | (a) route `is_nullable_capable` through `is_slice`; (b) leave it | (a) — preflight PF-004; a fixed array is inline storage, not a pointer-like view (T8) | ✅ Resolved |
| AR-24 | Behavioral | Whether a fixed array stays a lifetime-elision candidate | (a) keep it (borrowed storage a returned view may borrow); (b) remove it | (a) — preflight PF-006; removing it would break a valid view return | ✅ Resolved |
| AR-25 | Behavioral | `set<T>` dropping the replaced element | (a) drop the old element; (b) leave the overwrite | (a) — preflight PF-007; `spec/appendix-stdlib.md:62` requires the drop. Verified at execution: the emitter already drops an owned assignment target, so the behavior holds with no standard-library change (see AR-31) | ✅ Resolved |
| AR-26 | Technical | Lowering a by-value fixed-array return | (a) hidden trailing out-parameter; (b) generated struct wrapper | (a) — preflight PF-001; reuses the existing pointer-lowered parameter shape | ✅ Resolved |
| AR-27 | Technical | Emitting a nested array literal | (a) recurse one dimension per level; (b) compound-literal cast | (a) — preflight PF-003; per-dimension indexing matches the C declaration | ✅ Resolved |
| AR-28 | Technical | Whether `emits_drop` gains a fixed-array arm | (a) no arm; whole fixed-array assignment stays out of scope; (b) add the arm and route assignment | (a) — preflight PF-009; the scratch local is a C array and cannot be assigned | ✅ Resolved |
| AR-29 | UX | `E10215` message for the move-only fill | (a) reuse the existing message text with a specific help; (b) add a second message | (a) — preflight PF-008; keeps one registry template and the message matcher | ✅ Resolved |
| AR-30 | Testing (runtime) | A staged specification test also pins the old `E90000` trigger: `compiler/tests/stage_spec_test.rs:2368` asserts `Array<Buffer>` reports `E90000`, which AR-2/AR-3 remove but the plan did not list. | (a) convert it to an acceptance test for `Array<Buffer>`, renamed; (b) delete it, since the new ST-8 spec test covers acceptance; (c) keep the trigger (contradicts AR-2) | (a) confirmed 2026-09-26 — the case stays in the staged suite as the un-staged regression | ✅ Resolved |
| AR-31 | Testing (runtime) | PF-007 claimed `set<T>` leaks the replaced element. Against the built emitter, `one[0] = v` already drops the owned assignment target: the emitted `cl_set__String` body frees the old element, and an AddressSanitizer run shows no leak. A standard-library change (`let old = one[0];`) double-drops. | (a) skip the `std.mem` change and verify the existing drop; (b) force an explicit drop | (a) confirmed 2026-09-26 — PF-007 is a false positive; 3.2.3 is re-scoped to verification | ✅ Resolved |
| AR-32 | Behavioral (runtime) | ST-12 is red for a path the phase does not own. `is_uncontracted_param` (`compiler/src/ffi.rs:92`) matches only `Handle`/`Struct`, so a fixed-array FFI parameter such as `u8[4]` is treated as contracted: the call needs no `unsafe` and `E10101` is unreachable (the call-site guard at `own/call_check.rs:424` is never entered). CN-11 (`spec/12-ffi-contracts.md:131`) says an uncontracted parameter accepts only scalars, `raw`, `cstring`, views, and `null`; a fixed array is owned inline storage, so passing one is `E10101`. Phase 1 assumed the existing rule already reached this case (`00-preflight-report.md:228`), but the predicate gate is the declared type, not the argument. | (a) add a Phase 4 task: treat a fixed-array parameter as uncontracted in `is_uncontracted_param`, with a unit test; (b) defer ST-12 and record the gap as a deferred item | (a) confirmed 2026-09-26 — one predicate (`|| ty.is_fixed_array()`), no stdlib `ffi` declaration uses a fixed-array parameter, so no first-party breakage; task 4.2.4 added and `compiler/src/ffi.rs` joins the phase's expected modification set | ✅ Resolved |

## Resolution Notes

**AR-3:** `E90000` is the generic staged fallback; its one live trigger is
`Array<move-only>` (`spec/tests/staged/README.md:11`). Once AR-2 lands that
trigger is gone, so the staged golden can no longer assert the code. The
smallest change keeps the fallback defined for future staged features and only
removes the stale test data. No chapter text changes.

**AR-5:** This mirrors the lesson from the generic-field-integrity plan: the
defects exist because the same fact (instances substitute, arrays are views) was
duplicated across stages. One predicate, used everywhere, prevents the drift.

**AR-10:** Reproduced against the emitted C: a `String[2]` initialized with
`[; a]` emits `arr[0] = a; arr[1] = a;` and then `FreeString(a)`, leaving two
aliases of one freed handle. This is the same soundness family as F1 and the
plan's own array-literal change (AR-6) intersects it, so it is surfaced here
rather than silently left. Option (a) is a small normative addition to the
array-fill rules.

**AR-22:** A slice is a pointer-and-length view (`spec/08-arrays-strings.md:66`),
so a copy is a cheap view copy and the type is duplicable regardless of its
element. The current recursion into the element is the dual of the fixed-array
misclassification and is fixed by the same predicate.

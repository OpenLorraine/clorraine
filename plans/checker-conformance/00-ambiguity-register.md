# Ambiguity Register: Checker Conformance

> **Status**: ✅ GATE PASSED — all items resolved (AR-2 by user ruling: add `E10308` and fix
> both call forms)
> **Last Updated**: 2026-10-11 15:20
> **Batch**: G of the defect mandate (the checker-conformance cluster)
> **Auto-design**: active — policy version 1; reserved decisions (spec text, new user-visible
> behavior, acceptance criteria) remain user-owned and AR-2 is the one open item.

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which defects batch G closes | The 15 checker-conformance rows / a different partition | **User (standing):** the E → F → G split; G = the 15 rows listed in the index; D stays parked until no defects are left | ✅ Resolved |
| 2 | Error handling | DEF-147 (and its direct-call twin, the new DEF-213): the frozen registry has **no code for a call arity mismatch** (E10300 covers type arguments only). A fix needs a registry row, which is a reserved spec edit | A) a new `E10308` row with the E10300-mirrored message, fixing both the dot-call and the direct-call forms in Phase 3 / B) the same row, dot-call only / C) file both rows, fix in a later spec batch | **User (ruling):** option A — add `E10308 | Wrong number of arguments for '{name}': expected {expected}, found {found} | pass one argument per parameter` to `spec/14-diagnostics.md`, and fix DEF-147 and DEF-213 in Phase 3. **Executed in Phase 3:** the row lands after E10307, with the compile-fail golden `E10308_arity_mismatch.clo` (the chapter's coverage rule) and the harness inventory line; the generic and `ffi` direct-call twins join the same rule | ✅ Resolved (executed) |
| 3 | Behavioral | DEF-144: a `str`/slice view argument at a `cstring` parameter is accepted and emits the view struct where `char *` is declared | Reject with `E10200` (the conversion the language does not allow) / cross as the view's pointer member | **AI — delegated by --auto-design (PD-1):** reject. `implicit_conversion` (`types/mod.rs:595`) defines no view→`cstring` arm; the implicit String→`str` and literal rules stay; the explicit route is `toCString`, widened by DEF-154 (the frozen appendix documents it for `String` or `str`) | ✅ Resolved |
| 4 | Behavioral | DEF-145: a `T` iterable is judged only after substitution; a struct-element array, a scalar, or an enum instance emits invalid C | Fix the element binding per instance / reject at the instance | **AI — delegated (PD-1), amended in the Phase 2 fix round:** the semantics review's measurement showed the emitter mis-binds non-i32 elements and multi-extent rows, and that the concrete twins for struct/borrow elements build, so the plan's own decision rule (the twin decides) selected the **typed element binding**: an opaque element carries a placeholder resolved from the substituted argument's one-step element, so struct, row, and float elements bind correctly; a non-array substitution reports `E10043`, and a non-borrow non-duplicable element reports `E10040`, once at the first offending instance. The null-shaped iterable reports `E10012` at the declaration. The scoped re-reviews measured the mechanism's limit: the placeholder reaches the binding, not the element's **uses** (member reads, call arguments, index bounds, captures), which the type stage still types as unresolved; those gaps are filed as DEF-229–DEF-232 with the hoist boundary | ✅ Resolved (amended) |
| 5 | Behavioral | DEF-146: a value-form member read on a bare type parameter is accepted (only the call form reports `E10212`) | Report `E10212` per instance / at the declaration | **AI — delegated (PD-2), amended twice:** report **per instance** (the declaration keeps the designed late resolution; AR-5's first amendment). The Phase 2 fix round extended the audit's resolution to mirror the value-form checker's kind dispatch: a struct field (field-wins, including a field named `len`), an enum member, a member through a `raw` pointer to a struct, and the structural `.len` of a fixed array, slice, `str`, or `static str`; a `cstring`/`String` `.len` reports `E10212` (the concrete value-form gap is filed as DEF-224). The scoped re-reviews noted the enum arm accepts a declared member while the value-form emission is broken — the concrete spelling is identically broken, and the class is filed as DEF-233 | ✅ Resolved (amended twice) |
| 6 | Behavioral | DEF-148: an unknown member callee on a raw or plain-struct receiver is accepted; the emitted C calls an undeclared symbol | Report `E10212` for every receiver whose surface resolves nothing / keep the acceptance | **AI — delegated (PD-2):** report — Ch 04 §8's zero-match rule names `E10212`; the field-wins rule (PF-016) is unchanged. A field that exists but is not callable (a separate, filed class: DEF-214) keeps its current behavior in this batch | ✅ Resolved |
| 7 | Behavioral | DEF-149/DEF-151: a null-shaped local (inferred from `null`) accepts a `str` store and is returned as a concrete value; both emit invalid C | `E10012` (the frozen null rule) / a new code | **AI — delegated (PD-3):** `E10012` — spec Ch 03 §6: "Using `null` where a non-nullable type is required is E10012"; the registry row exists. Applies to the store target, the concrete use positions (return, argument, iterable), while comparisons and nullable targets stay accepted | ✅ Resolved |
| 8 | Behavioral | DEF-153: an update of a moved or uninitialized place reports `E10020` up to 3× / `E10021` 2× at one site | Dedupe per site (keep the first) / leave the repeats | **AI — delegated (PD-2):** dedupe — the one-report-per-site convention is established across the earlier batches and the repeat is an artifact of two walks reading the same place | ✅ Resolved |
| 9 | Behavioral | DEF-160/DEF-161: a by-value capture move outside a `return`/`take` reports `E10034` (the lend duty) where the frozen capture rule assigns `E10401`; a block-bodied composite return reports both | Extend the capture-move recorder to every move site and retire the lend duty for capture roots / keep the split | **AI — delegated (PD-3):** extend — the frozen Ch 06 L-3 rule assigns a move of a captured binding to `E10401` regardless of the destination form | ✅ Resolved |
| 10 | Behavioral | DEF-162: the `E10030` message can name a compiler-generated owner (`__cl_owned0`) | Substitute a display phrase for a generated owner / name the view binding | **AI — delegated (PD-2):** a display phrase — the binding's `generated` flag exists (`own/flow.rs:74`); the registry template `borrow of '{name}' does not live long enough` is unchanged and the `{name}` placeholder renders the phrase, exactly as the temporary case does | ✅ Resolved |
| 11 | Technical | DEF-136: the initializer path re-reads an impure dividend and an impure index base (the body path already stores both once) | Materialize in the initializer path only, mirroring the body placement / rework both paths | **AI — delegated (PD-3):** the initializer path only, shape-gated. The body twin is verified single-evaluation at head (`d() / 1` stores `__cl_tmp0` once); the initializer materialization set (`program.rs:1428`) lacks the dividend and the base | ✅ Resolved |
| 12 | Technical | DEF-142/DEF-215/DEF-216: slice typedefs go missing when the slice type reaches the body only through a value's own result type, a lifted-lambda specialization, or a generic struct-instance element | Collect the missing sources in the existing collector / restrict the spellings | **AI — delegated (PD-2):** collect; the additions only grow the typedef set, so unaffected units emit byte-identically | ✅ Resolved |
| 13 | Behavioral | DEF-150: `len(flag ? b.s : b.s)` in a generic body folds to `0`; the conditional arm defeats the substituted projection resolution | Resolve the chosen arm's projection through the substituted base at the fold / lower through the runtime `len` | **AI — delegated (PD-2):** resolve for the fold. The structural fold keeps its rule: it evaluates nothing (the batch F contract), so the condition and the arms' stores never emit | ✅ Resolved |
| 14 | Behavioral | DEF-154: the embedded `toCString` declares `String` only, while the frozen appendix documents `String` or `str`; `toCString(view)` reports `E10200` | Widen the embedded signature to `str` (the appendix) / narrow the documented surface | **AI — delegated (PD-1):** widen the embedded signature — the appendix is frozen and the stdlib conforms to it; `String` callers keep working through the frozen implicit String→`str` widening (PF-022), and the body reads the view's pointer | ✅ Resolved |
| 15 | Error handling | Diagnostic codes for the fixes | Reuse existing codes / add new ones | **AI — delegated:** existing codes only — `E10200` (DEF-144), `E10043`/`E10212` (DEF-145/146/148), `E10012` (DEF-149/151), `E10401` (DEF-160/161); the arity row is AR-2 and remains user-owned. **Amended in Phase 3:** DEF-144 needed no code — the frozen ffi crossing rules (Ch 12 CN-11 and §10) accept a view at an uncontracted parameter and cross it as its pointer, so the emitter spells the `char *` cast (the design's `E10200` rejection was amended) | ✅ Resolved (AR-2 excluded; AR-15 amended) |
| 16 | Non-functional | Byte-identity of unaffected emissions | Shape-gated changes / blanket re-pinning | **AI — delegated:** shape-gated; the emission auditor re-diffs the corpus per phase and classifies every delta | ✅ Resolved |
| 17 | Non-functional | Verify commands | `make check` + `make verify` + the scoped suites | **User (standing):** confirmed in the prior registers; unchanged | ✅ Resolved |
| 18 | Process | Phase review dispatch | AGENTS.md matrix by change surface | **Convention (project policy):** recorded per phase in the execution plan | ✅ Resolved |
| 19 | Technical | Complexity: does any fix add a layer, dependency, harness, or cross-cutting refactor? | None — the fixes extend existing checks and walks; the only new artifact is one recorded sema duty list (`iterables`) mirroring `capture_moves` | **AI — delegated (PD-1):** none; the arity registry row (AR-2) is the only spec-touching change and is user-owned | ✅ Resolved |

### Resolution Notes

**AR-2 (user ruling):** the frozen registry (`spec/14-diagnostics.md`) has no row for a
value-argument count mismatch; `E10300` covers type arguments. The user ruled option (A): add
`E10308 | Wrong number of arguments for '{name}': expected {expected}, found {found} | pass one
argument per parameter`, and fix DEF-147 plus the direct-call twin (filed as DEF-213) in
Phase 3. The registry edit is authorized by this ruling.

**AR-3 (delegated):** probed at head — `RawC(s)` with `s: str` emits the `ClStr` struct where
`char *` is declared (`incompatible type for argument 1`); a `u8[]` slice is identical; a
`String` handle build and a literal build. The conversion table confirms the rejection is the
conformance fix.

**AR-7 (delegated):** the null shape is `TyKind::Unknown` with `nullable: true`
(`sema/expr.rs:51`); `implicit_conversion` accepts it everywhere (its unknown escape hatch),
which is why the store and the return pass today. The fix tests the shape explicitly and
reports `E10012`, keeping `null` itself accepted at nullable targets and in comparisons.

**AR-9 (delegated):** the recorder (`sema/expr.rs:307`) currently runs at `return` and `take`
sites only; `own/place.rs:491` skips the lend duty for by-value captures **that escape** and
documents the gap for non-escape moves ("A move into an owned local stays with the lend duty,
which the capture-move recorder does not cover"). Both halves extend to the `let`/store and
composite-store sites.

**AR-14 (delegated):** the current body (`stdlib/std.text.clo:45`) returns the shim's pointer
for a `String`; the widened version reads the view's own pointer, matching how the standard
library's `print(s: str)` reads `s.ptr` (`stdlib/std.io.clo:64`). The exact body is pinned by
the in-phase probe so `String` callers emit byte-identically.

### Delegated Decision Provenance

#### PD-1 — Reject over cross; audit over typed element views

```text
Second-guessed:
- DEF-144 could cross a view as its pointer member (the uncontracted-parameter rule already
  does that at `codegen/calls.rs:144-160`). Rejected: a `str` view is not a NUL-terminated
  `cstring` in general and a slice has no NUL guarantee at all; the frozen crossing model
  (PF-018) accepts scalars, `raw`, and `cstring` — not views — at uncontracted parameters,
  and DEF-154 provides the explicit conversion route.
- DEF-145 could emit a typed element view at the instance. Deferred: it duplicates the body's
  element machinery for a shape whose concrete twin must first be probed; the reject route
  reuses E10043 and the existing audit pattern.
```

#### PD-2 — Reuse the frozen codes and the existing predicates

```text
Grounded in: Ch 04 §8 (dot-call candidate matching; E10212 for zero matches), Ch 03 §6
(E10012 for null where non-nullable), Ch 06 L-3 (capture moves are E10401),
`own/flow.rs:74` (the generated-owner flag), `codegen/expr.rs:280` (`resolved_value_ty`'s
member/index descent).
```

#### PD-3 — Conformance-first on the emission and ownership halves

```text
DEF-136 mirrors the body's `materialize_guard_operand` placement into the initializer path
without touching the verified body path; DEF-149/151 and DEF-160/161 reuse the frozen
diagnostics their rules name rather than inventing new shapes; the ownership boundary is
unchanged (both capture codes are errors, so acceptance does not move).
```

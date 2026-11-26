# Verification packet 9.2.2 — WS2

## Scope

- Whole components at the baseline `e24a8752b08827c3a5d7c77b6e2827dac708b629`; pre-existing defects are in scope.
- Finding IDs use the `ASM-WS<n>-<NNN>` scheme.
- Read-only: do not edit any tracked file; write scratch output only under `build/tmp/assessment/verify/`.
- Before each reproduction run the drift check: `bash plans/independent-assessment/probes/WS0/drift-check.sh plans/independent-assessment e24a8752b08827c3a5d7c77b6e2827dac708b629` (it must pass). Builds and runs go through the SDK container: `make -C docker exec CMD="…"` (no nested quotes in CMD). The pinned compiler is `build/tmp/assessment/baseline/clc` (release; `clc-debug` alongside).
- Judge each finding from its text, the committed probes, the code, and `spec/`. Do not rely on the workstream files' other sections; they are the author's reasoning.

## Checklist briefs

- `.opencode/agents/memory-safety-auditor.md`

## Protocol

For every finding, the verifier:

1. Re-runs the reproduction or measurement from the committed sources with the drift check
   passing; for every `probable` Major or Critical it attempts a reproduction.
2. Tries to refute it: checks `spec/`, `docs/known-limitations.md`, earlier plans, and
   whether the observed behavior is intended.
3. Checks the severity against §Severity and the confidence against §Confidence.
4. Checks for duplicates across workstreams and merges them under the lowest ID.
5. Records one verdict: `confirmed`, `refuted`, `downgraded <from→to>`, `upgraded <from→to>`,
   or `merged into <ID>`, with a one-line reason.
6. Escalates any verdict that results in a Critical, per §Escalation.

The verifier also re-checks at least two `clean` outcomes per workstream, preferring items on
the A500 lens and memory-safety checklist items (WS2, WS5-03, WS6-05), and records a verdict
for each. Findings raised by a verifier are
added to the workstream file and receive their verdict from a different verification session.
Refuted findings stay in `05-findings.md` with their verdict; they never reach the report's
ranked list.

## Severity

| Severity | Definition | Ref |
|----------|------------|-----|
| 🔴 Critical | An accepted program miscompiles, is memory-unsafe, or emits invalid C; a crash or hang on a real Kickstart; data loss | AR #15 |
| 🟠 Major | Latent undefined behavior in emitted C; a shim leak or missing failure path; a wrongly rejected valid program; ≥10% net m68k instructions or `.text` against the equivalent C reference on a shipped 68000 build (§Measurement rules) | AR #15, PF-009 |
| 🟡 Minor | Edge-case divergence with low impact; maintainability debt that has already caused a recorded DEF | AR #15, PF-020 |
| 🟢 Opportunity | A measured improvement with no defect behind it below the Major threshold, or any gap measured only on a 68020 probe build | AR #15, PF-009 |

A `hypothesis` is never above Minor. A `clc` crash or hang on any input is Critical; a compile
time or peak RSS above 10× the Phase 0 `bench-compile` value for the same input size is Major
(PF-020). For emitted C (WS3-11), a C-compiler crash on valid emitted C, or a C compile time
above 10× the Phase 0 C compile time of the nearest-size emitted program, is Major (PB-005).

### Confidence

`verified` is only this confidence label; it never means "passed Phase 9" (PF-002).

| Label | Meaning |
|-------|---------|
| `verified` | Reproduced from committed probe sources at the baseline (drift check passing), or measured |
| `probable` | Strong code-reading evidence, no reproduction yet |
| `hypothesis` | Plausible from reading; never ranked above Minor (AR #15) |

A finding is **surviving** when its Phase 9 verdict is `confirmed`, `downgraded`, or
`upgraded`. Only surviving findings reach the report and the DEF ledger (AR #13 as clarified
by PF-002).



## Output

Write `build/tmp/assessment/verify/9.2.2.md` and return its content: one row per finding `| ID | verdict | reason (one line) | what you re-ran and observed |`, one row per sampled clean outcome with a verdict (`confirmed` or `refuted` + reason), any duplicate you would merge, and any new finding you raise in the finding template (it will be verified by a different session).

## Findings to verify

### ASM-WS2-001 — Owned temporaries drop at the end of the enclosing scope, not at the end of their statement

- **Severity**: Major
- **Confidence**: verified
- **Lens**: all (host verified; the lowering is profile-independent)
- **Location**: `compiler/src/own/lower.rs:5174-5184` (a call result is bound to a scope owner), `compiler/src/own/lower.rs:199-205`, `materialize_owned_argument`; emitted as function-level `__cl_owned<N>` slots released by the scope cleanup ladder, at e24a8752
- **Checklist item**: WS2-05
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — probes `probes/WS2/226-call-result-temp.clo`, `227`, `228`, `229`, `246`, `247`, `219`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS2 --clc build/tmp/assessment/baseline/clc --sanitize --only 219 226 227 228 229 246 247"`. Expected (spec/04-expressions.md §3: "Temporaries created anywhere in a statement … live until the end of that statement and drop there in reverse creation order"; spec/17-codegen.md §"temporary … dropped at statement end in reverse creation order"; spec/decisions.md D5, PF-040): `println($"{idOf(tok(7))}"); println("next");` prints `7`, `drop 7`, `next`. Observed in the debug, release, ASan, and UBSan builds: `7`, `next`, `end`, `drop 7` — the temporary is released after every later statement of the scope. Same for argument temporaries (227: `3`, `end`, `drop 2`, `drop 1`), short-circuit and `?:` temporaries (228, 229), and projections of call results (246, 247). In a loop body (219) the condition temporary of iteration *n* drops after the iteration's earlier-declared local, reversing creation order (`drop 0`, `drop 50` instead of `drop 50`, `drop 0`). The emitted C for 248 shows the mechanism: `ClPair __cl_owned0 = {0};` at function scope, released at the `__cl_c1` cleanup label. Memory safety is intact (no sanitizer report).
- **Impact**: every owned temporary (a `String` from a template or call, a handle returned by a call and only lent) lives until its scope ends instead of its statement. Deleter side effects (file close and flush, unlock, port deletion, chip-memory release) run later and in a different order than the spec defines; in a long function on the A500, temporaries' chip or heap memory stays allocated until the function returns.
- **Root cause**: to fix leaks of materialized temporaries (DEF-23, DEF-45), a call result that owns storage is bound to a scope owner whose release the scope ladder emits; there is no per-statement cleanup point, although the code comments state "owns its storage only until the statement ends" (`own/lower.rs:5176`). The user ruling `plans/eval-emission-integrity/00-ambiguity-register.md` AR-B2 extended a temporary to scope end only when a **view binding** keeps it alive; the implementation extends every owned temporary.
- **Recommendation**: emit a statement-end release for statement-scoped owners (reverse creation order) and keep scope adoption only for temporaries whose view is bound past the statement (the AR-B2 case); add drop-order run goldens for call-argument, template, short-circuit, and projection temporaries.
- **Effort**: L
- **Related**: DEF-23, DEF-45 (leak fixes that introduced scope owners); AR-B2 (`eval-emission-integrity`); SQ-WS2-1

### ASM-WS2-002 — A field move out of a call result is accepted

- **Severity**: Minor
- **Confidence**: verified
- **Lens**: all
- **Location**: `compiler/src/own/lower.rs:5174-5184` and the partial-move check in `compiler/src/own/place.rs` at e24a8752
- **Checklist item**: WS2-10
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Reproduction — `probes/WS2/248-projection-move-from-temporary.clo` (`let t: Tok = makePair(1, 2).a;`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS2 --clc build/tmp/assessment/baseline/clc --sanitize --only 248"`. Expected (spec/03-ownership.md §4 and spec/07-structs.md STR-11: partial moves are limited to top-level fields of owned locals and `take` parameters, E10026): reject E10026. Observed: accepted (`rc=0`). Built by hand under ASan (`build/tmp/assessment/WS2/run248.sh`): prints `use 1`, `end`, `drop 1`, `drop 2`, exit 0, no report — the remainder is released once.
- **Impact**: a program the spec rejects compiles and runs memory-safely; portability of sources to a conforming compiler is the only cost.
- **Root cause**: the call result is bound to a scope owner (ASM-WS2-001), which makes it an owned local in the lowering, so the partial-move rule sees an owned local.
- **Recommendation**: report E10026 for a field move whose root is a call result or other temporary, or ask for a spec ruling that admits it (the semantics are already sound).
- **Effort**: S
- **Related**: ASM-WS2-001

### ASM-WS2-003 — Move, copy, and drop decisions are re-derived at about 240 sites in two stages with different predicates

- **Severity**: Minor
- **Confidence**: probable
- **Lens**: all
- **Location**: `compiler/src/own/` (`.duplicable(` 79 sites, `maybe_owns(` 70 sites, 12 files); `compiler/src/codegen/` (`needs_drop` 84 sites in `function.rs` 38, `guards.rs` 17, `program.rs` 11, `drop.rs` 9, `cleanup.rs` 9); predicates `compiler/src/own.rs:675` (`maybe_owns_ty`, which counts generic-parameter mentions) and `compiler/src/codegen/drop.rs:283` (`needs_drop`, which excludes parameters) at e24a8752
- **Checklist item**: WS2-12
- **Author session**: ses_dec1f5b4dffeG1pq3Qx6rPrRPn
- **Evidence**: Code reading (site counts by `grep -rn` over `compiler/src/own` and `compiler/src/codegen`). The ownership stage emits drops in the IR, but the C emitter separately decides, at each store, field write, guard, and cleanup, whether a value "needs a drop" (for example the old-value release before an assignment), using `needs_drop` = not duplicable, not a view, not unknown, not a parameter, not void — a different predicate from the ownership stage's `maybe_owns`. The DEF ledger holds 65 rows mentioning a double free, 74 a leak, and 47 a use after free or dangling access, many naming a single shape where one site decided differently from another (for example DEF-13, DEF-34, DEF-53, DEF-60, DEF-65).
- **Impact**: each new shape needs the same decision made consistently in many places; the residue series in the roadmap (`conditional-projection-residue` i–ii, `memory-critical-*`, `def-zero-*`) are consistent with sites disagreeing. No probe in this workstream found a live disagreement.
- **Root cause**: ownership facts (moved, owned, needs release) are not recorded once on the IR and consumed by the emitter; both stages recompute them from types.
- **Recommendation**: make the ownership stage the single authority — annotate every IR store and place with its release obligation — and have codegen consume the annotation instead of `needs_drop`; WS8-08 weighs this against the DEF classes.
- **Effort**: XL
- **Related**: WS8-07, WS8-08

## Clean outcomes to sample

From `findings/WS2-ownership.md` §Checklist outcomes:

| ID | Outcome | Evidence |
|----|---------|----------|
| WS2-01 | `clean` | probes 201–209 (move by `let`, `take` argument, return, struct and array literal, `?:` arms, partial move, re-initialization, whole use after partial move E10025): output and drop order as specified, four builds, sanitizer-clean. The field move out of a call result is ASM-WS2-002 (WS2-10) |
| WS2-04 | `clean` | probes 221 (reverse registration with drops), 222 (early return through nested blocks), 223 (defer reading a moved value E10020), 224 (defer in a loop on continue/break), 225 (nested defers) |


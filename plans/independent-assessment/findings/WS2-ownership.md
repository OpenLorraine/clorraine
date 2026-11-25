# WS2 — Ownership, borrowing, and drops

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS2 (WS2a = WS2-01..06, WS2b = WS2-07..12)

## Scope and limits

Components: `own/`, `borrow.rs`, `init_graph.rs`. Lens: an accepted program that leaks,
double-frees, uses after free or move, or reads uninitialized memory is Critical. Probes
run on the host profile in four builds: the debug line, the shipped release line, and the
AddressSanitizer and UndefinedBehaviorSanitizer lines of 03-01 §Ground rules.

**Instrument.** `probes/WS2/tok.clo` declares a move-only handle `Tok` whose deleter prints
`drop <id>` and frees a one-byte block from the host C library's `strdup`. Drop order and
count are visible on stdout; leaks and double frees are visible to the sanitizers. The
instrument is host-only: libnix declares `strdup`, so the user `ffi` prototype conflicts on
the Amiga profiles (and the compiler, correctly, refuses user `ffi` bindings to the shim's
own symbols, E11101). The m68k lenses are therefore not exercised by WS2 probes; the
ownership lowering is profile-independent apart from the embedded shim.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | xhigh | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (2.1.1, 2.1.2) | `probes/WS2/200`–`254`, `tok.clo`, `san-control.*` | done |
| Red check (2.1.3) | `san-control`, all contrasts, corrections 255–259 | done |
| WS2a analysis (2.2.1, 2.2.2) | probes 200–234 run in four builds; `own/lower.rs` temporary binding read | done |
| WS2b analysis (2.2.3, 2.2.4) | probes 235–259 run; structural site count over `own/` and `codegen/` | done |
| Self-refutation (2.3.1) | against `spec/`, `docs/known-limitations.md`, the DEF ledger, earlier rulings, and the spec tests that describe the behavior | done |

## Checklist outcomes

Main run 2026-11-25 (`make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS2 --clc build/tmp/assessment/baseline/clc --sanitize --only …"` over every probe; log
`build/tmp/assessment/WS2/main.log`, results `records/WS2-results.json`): 183 cells, 148
pass. **No sanitizer report in any of the 55 probes × 4 builds** (`grep -c SANITIZER` = 0):
no leak, double free, use after free, or undefined behavior was observed on any probed shape.

| ID | Outcome | Evidence |
|----|---------|----------|
| WS2-01 | `clean` | probes 201–209 (move by `let`, `take` argument, return, struct and array literal, `?:` arms, partial move, re-initialization, whole use after partial move E10025): output and drop order as specified, four builds, sanitizer-clean. The field move out of a call result is ASM-WS2-002 (WS2-10) |
| WS2-02 | `clean` | probes 210 (body locals then parameters), 211 (conditional move drop flag), 212 (early return), 213 (break/continue), 214 (assignment drops the old value first), 215 (a runtime fault runs no ladder) |
| WS2-03 | `clean` | probes 213, 216 (move in loop E10020), 217 (loop-carried re-initialization), 218 (`for (borrow e in xs)`; by-value loop E10040), 220 (nested loop break). Probe 219 differs only in temporary drop timing (ASM-WS2-001) |
| WS2-04 | `clean` | probes 221 (reverse registration with drops), 222 (early return through nested blocks), 223 (defer reading a moved value E10020), 224 (defer in a loop on continue/break), 225 (nested defers) |
| WS2-05 | ASM-WS2-001 | probes 219, 226–229 (temporary drop point and order); probe 230 (String temporaries in templates, loops, and reassignment under ASan) is clean. See SQ-WS2-1 for condition temporaries |
| WS2-06 | `clean` | probes 231 (loan ends at last use), 232 (loan on one branch), 233 (move while loan live E10022), 234 (field lends); the conflicting mutation is E10032, which spec/appendix-stdlib.md §3 allows ("E10031 or E10032"). Two-phase borrows are not specified (SQ-WS2-3) and were not probed |
| WS2-07 | `clean` | probes 237 (storing another lend in a `borrow T?` slot E10034), 238 (slot replacement drops the incoming value first), 258 (view of a local `Array` returned E10035), 259 (global borrowed E10038). Probes 235/236 were design errors (fixed arrays never become views, spec/08 AR-2), replaced by 258/259 |
| WS2-08 | `clean` | probes 239 (generic move vs copy instances), 240 (`Array<Tok>` push/set/drop order, 20 elements, ASan-clean), 241 (generic struct copy vs move instances), 242 (`Box<T>`, `boxSet`, `Box<Pair>`, `Box<T>?`) |
| WS2-09 | `clean` | probes 255 (`[borrow total, t]` capture: write-through and read-only borrow, single drop), 256 (moving a capture E10401). Returning a capturing closure is rejected, with E10400 rather than E10404 (probe 257; SQ-WS2-2). Probes 243–245 omitted capture lists (E10405) and were replaced |
| WS2-10 | ASM-WS2-001, ASM-WS2-002 | probes 246, 247 (projection of a call result / of `?:`: correct values and only the chosen arm built, but the temporary drops at scope end); 248 (field move out of a call result accepted); 249 (nested-path move E10026) clean |
| WS2-11 | `clean` | probes 250 (one-branch assignment E10021), 251 (every `switch` path assigns), 252 (loop-only assignment E10021), 253 (read after a move on one path E10020), 254 (fall-through assignment) |
| WS2-12 | ASM-WS2-003 | site counts and the two drop predicates below |

## Findings

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
- **Self-refutation (2.3.1)**: `docs/known-limitations.md` has no entry on temporary drop timing; no DEF row records it; spec/04 §3, spec/17, and D5/PF-040 agree on the statement end. The existing spec tests that describe a release "when the statement ends" (for example `compiler/tests/conditional_projection_residue_spec_test.rs:4600-4660`) assert only leak-freedom and the exit code, never the drop point, so they cannot catch it (input to WS8-02/WS8-03). Not a regression of a closed DEF: the scope-owner binding was the fix for DEF-23/DEF-45. Severity stays Major: values and memory safety are correct, but deleter timing and order are observable and differ from the spec.
- **Verdict**: _(Phase 9)_

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
- **Self-refutation (2.3.1)**: spec/03 §4 and STR-11 are explicit; no ruling admits call-result field moves; no limitation lists it. Minor, because the accepted program is memory-safe.
- **Verdict**: _(Phase 9)_

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
- **Self-refutation (2.3.1)**: a site count is not proof of disagreement, and no WS2 probe exposed a live disagreement, so the confidence stays `probable` and the severity Minor (maintainability debt with recorded DEFs, 03-01 §Severity); WS8-08 decides whether it is the structural cause of the recurring classes.
- **Verdict**: _(Phase 9)_

## Spec questions

- **SQ-WS2-1 — When does a temporary created in an `if`/`while` condition drop?** spec/04 §3
  says temporaries drop "at the end of that statement", but an `if` or `while` statement
  contains its body; the text does not say whether a condition temporary drops after the
  condition or after the whole statement (and, for `while`, per iteration). Probes 219 and
  228 avoid depending on the answer.
- **SQ-WS2-2 — Is returning a capturing closure E10404 or E10400?** spec/06 §6.1 says
  "Storing, returning, or capturing a closure is E10404 (L-6)" and "a capturing lambda used
  where a plain `func(...)` value is required is E10400" (L-7). A return of type
  `func(): i32` matches both; `clc` reports E10400 (probe 257).
- **SQ-WS2-3 — Are two-phase borrows part of the model?** The WS2-06 checklist names them,
  but spec/16 does not define them (for example `xs.push(xs.len())`, an exclusive receiver
  lend with a read in the argument). Not probed.

## Falsifiability log

Red check 2026-11-25 after `ad1c7faa` (probes) and the corrections commit (255–259, the 228
contrast amendment), all committed before running. Commands: `san-control.sh` and
`probe.py plans/independent-assessment/probes/WS2 … --contrast --sanitize` (logs
`build/tmp/assessment/WS2/contrast.log`, results `build/tmp/assessment/WS2/probes/contrast.json`,
`probes-fix/contrast.json`). Every contrast ran on host in four builds (debug, release, ASan, UBSan).

| Probe | Form | Red-check result |
|-------|------|------------------|
| `san-control` | seeded defect: leak, double free, use after free (ASan, `detect_leaks=1`), signed overflow (UBSan); clean control | every seeded defect reported, clean control passed both lines |
| 200–208, 210–214, 217, 220–222, 224, 225, 230, 234, 238–242, 251, 254, 255 | contrasting input: a changed id, operand, branch, or literal whose expected output differs | all matched their contrast expectations in all four builds |
| 201, 202, 207, 209, 215, 216, 218, 223, 233, 237, 244→256, 245→257, 248–250, 252, 253, 258 | contrasting input: the reject probe's offending construct is replaced by a valid one, which must be accepted (and print where stated) | all accepted with the stated output |
| 231, 232 | contrasting input: the element view is used after the mutation, which must be rejected | rejected with **E10032**; the contrast expected E10031, but spec/appendix-stdlib.md §3 allows "E10031 or E10032" for this case, so the contrast discriminates and the code is within the spec |
| 259 | contrasting input: lend a local copy instead of the global | accepted; printed `1`, not the `2` the contrast expected — spec/03 §2 V9 ("A lend of a duplicable value binds a copy", W10001) makes `1` correct; the contrast expectation was wrong, the discrimination (reject vs accept) holds |
| 219, 226, 227, 228, 229, 246, 247 | contrasting input: a changed id or branch | **did not match**: in every build the owned temporaries drop at the end of the enclosing scope, not at the end of their statement, and in 219 after a local declared before them. The contrasts discriminate (ids and branch outputs changed as expected); the drop timing itself is evidence for WS2-05, checked by the main run in task 2.2.2 |
| 235, 236, 243, 244, 245 | — | **probe design errors**, kept with their outcome: 235/236 lent a fixed array as a view (E10200; spec/08 AR-2), 243–245 omitted captures from the capture list (E10405; spec/14). Replaced by 258, 259, 255, 256, 257 before running |

## Measurement and run records

Committed copies (ST-12): [`records/WS2-results.json`](records/WS2-results.json) (main run,
183 cells), [`records/WS2-contrast.json`](records/WS2-contrast.json) (red check, 210 cells).
All runs: pinned `clc` sha256 `0c2fd66e…`, baseline `e24a8752`, host x86-64 inside the SDK
container, builds per `probe.py` `BUILDS` and `SANITIZE`. Supplementary (not committed):
`build/tmp/assessment/WS2/run248.sh`, `build/tmp/assessment/WS2/diag/`.

## Phase 2 gate (2.3.2)

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | three findings, every template field present, IDs `ASM-WS2-001..003` |
| ST-2 | `verified` findings cite committed probes and the command; reproduced at HEAD |
| ST-3 | the `probable` finding is Minor; no `hypothesis` |
| ST-4 | no performance finding — n/a |
| ST-5 | probes committed in `ad1c7faa`, corrections 255–259 and the 228 amendment in their own commit, each before running; falsifiability log complete; the sanitizer self-test proves the detectors fire |
| ST-6 | three spec questions; findings cite spec sections; no `spec/` change |
| ST-7 | drift check PASS |
| ST-8 | WS2-01..WS2-12 each have one outcome; coverage table complete |
| ST-11 | no Critical — n/a |
| ST-12 | records above |
| ST-14, ST-15 | n/a |
| ST-16 | dispatch record `xhigh` |

## Escalations

None (no Critical in WS2).

## Probe index

Runner: `probes/WS1/probe.py --sanitize` (adds the ASan and UBSan host lines).

| Item | Probes |
|------|--------|
| instrument | 200 |
| WS2-01 moves | 201–209 |
| WS2-02 drop elaboration | 210–215 |
| WS2-03 loops | 213, 216–220 |
| WS2-04 `defer` | 221–225 |
| WS2-05 temporaries | 219, 226–230 |
| WS2-06 NLL | 231–234 |
| WS2-07 views and escapes | 235–238 |
| WS2-08 generics | 239–242 |
| WS2-09 closures | 243–245 |
| WS2-10 projections | 246–249 |
| WS2-11 initialization | 250–254 |
| WS2-12 structural review | code reading (task 2.2.4) |
| sanitizer self-test | `san-control.{c,sh,expected}` |

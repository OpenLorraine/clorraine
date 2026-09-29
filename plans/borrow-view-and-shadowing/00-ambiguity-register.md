# Ambiguity Register: Borrow-View Moves and Shadowed Narrowing

> **Status**: ✅ GATE PASSED — all 14 items resolved
> **Last Updated**: 2026-09-29 08:06

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Which defects this plan implements | DEF-18 + DEF-20 together (the second plan announced by the completed array-value plan) / DEF-18 only / a different cluster | The AR-10 second plan: `DEF-18` and `DEF-20` together | ✅ Resolved |
| 2 | Naming & terminology | Plan folder name | `borrow-view-and-shadowing` / `checker-semantics-integrity` / `ownership-checker-fixes` | `borrow-view-and-shadowing` | ✅ Resolved |
| 3 | Behavioral | What moving through a `for (borrow e in xs)` element view does | reject with `E10026` / reject with `E10034` / support the move | Reject with `E10026` in every move form; reads, lends, and writes through the view stay accepted | ✅ Resolved |
| 4 | Edge cases | Which move forms are rejected | take argument, owned-local binding, field path, aggregate-literal field, return / a subset | Every move form funnels through the shared move decision and is rejected | ✅ Resolved |
| 5 | Behavioral | `DEF-20`: the shadowing initializer under a narrowed nullable | resolve the initializer against the outer binding / other | The initializer resolves before the shadowing declaration, so the narrowed outer value flows in; genuine nullable uses keep `E10010` | ✅ Resolved |
| 6 | Scope | The `const` shadowing variant | include it in `DEF-20`'s fix / file it separately | Include `const` (same root cause and order as `let`) with its spec cases | ✅ Resolved |
| 7 | Naming & terminology | `DEF-18`'s diagnostic text | reuse the `E10026` registry row verbatim / a site-specific help | Reuse the registry row verbatim (`partial moves are limited to top-level fields in v1`; `move the whole value instead`) | ✅ Resolved |
| 8 | Integration points | Verification commands | `make check` per task, `make verify` at close / other | `make check` per task, `make verify` at close (project standard) | ✅ Resolved |
| 9 | Technical | Where the `DEF-18` rejection lives | one check in `move_place` (the shared move decision every form reaches) / a check repeated at each move site | User accepted the recommendation: the shared `move_place` check; a borrow element binding is `BindingKind::Local` with `Mode::Borrow` and a `DerefTemp` base, so the existing parameter rule misses it | ✅ Resolved |
| 10 | Scope | `spec/`, the grammar, and the diagnostics registry | change them / keep the frozen oracle | User accepted the recommendation: keep frozen; both defects are implementation conformance gaps, and the registry rows exist | ✅ Resolved |
| 11 | Security & compliance | Security surface of this change | compiler-internal only / a new input or data path | User accepted the recommendation: compiler-internal — no user input, secrets, persistent data, or network surface | ✅ Resolved |
| 12 | Scope | The other filed defects (`DEF-23` – `DEF-29`) and the `PLAN-05`/`PLAN-13` programs | include them / leave them filed | User accepted the recommendation: leave them filed; this plan stays the two-defect cluster | ✅ Resolved |
| 13 | Edge cases | Guard behavior around the rejection | unchanged / changed | User accepted the recommendation: unchanged, pinned by spec cases — reads, lends, and writes through the view; the copy-form loop binding (which owns its copy); borrow-parameter moves (already `E10034`) | ✅ Resolved |
| 14 | Testability | Oracle sources for the specification cases | `spec/03` §3–4, `spec/08` §5, `spec/14`, `spec/16` / other | User accepted the recommendation: `spec/03` §3–4 (element moves are `E10026`), `spec/08` §5 (the view contract), `spec/14` (`E10010`/`E10026` rows), and `spec/16` (borrow rules) | ✅ Resolved |

### Resolution Notes

**AR-1:** `DEF-18` and `DEF-20` were left filed by the completed `array-value-integrity` plan's AR-10 for a focused second plan; both are checker-semantics fixes on the ownership stage.

**AR-2:** The name states the two defects: a borrow-view move and shadowed narrowing.

**AR-3:** `spec/03` §3 states "moving an element out of an array is an element move, which v1 limits", and §4 states "nested paths and element moves are `E10026` in v1". The probe shows `eat(e)`, `let s: String = e;`, a field move, and a return through the view are accepted today.

**AR-4:** All move forms reach `FunctionChecker::move_place` (`own/place.rs:182`), whose borrowed-parameter rule checks `BindingKind::Param` only; the loop element binds as `BindingKind::Local` with `Mode::Borrow` (`own/loops.rs:311-338`), so the rule misses it.

**AR-5:** `check_let` declares and initializes the shadowing binding before `check_value_into` runs, so the initializer's name resolves to the new (not narrowed) binding and reports a spurious `E10010`; the same order exists in `check_const`. The initializer shares the file's own rule that it must resolve to the outer binding.

**AR-6:** The const probe reports the identical spurious `E10010`.

**AR-7:** The registry row is canonical and the completed plan reused it for the ownerless-element rule; the help is generic but consistent.

**AR-8:** From `AGENTS.md`, as the completed plan used.

**AR-9:** `move_place` receives every ownership transfer (`check_argument_place` take mode, `prepare_store`/`bind_value` moves, `check_return`, and the aggregate consume point). A view binding is uniquely `BindingKind::Local` with `Mode::Borrow` (and a `DerefTemp` base); borrow parameters keep their existing `E10034`.

**AR-10:** No specification, grammar, or registry change; `E10026` and `E10010` already own the rows.

**AR-11:** The change alters checker acceptance only; no input parsing, storage, or network path.

**AR-12:** `DEF-23` – `DEF-29` and the backlog programs stay outside this plan's documents.

**AR-13:** The guard cases pin that only ownership transfers through the view are rejected; the view's reads, lends, and writes, the copy-form loop binding, and the borrow-parameter rule keep their behavior.

**AR-14:** The specification cases are derived from those chapters and rows; the red-phase states were probed on 2026-09-29.

# Ambiguity Register: Residual Major Closeout

> **Status**: ✅ GATE PASSED — the planning decisions are recorded (auto-design delegated); no
> batch row is parked; the six older rows and the batch-J follow-ons stay out (AR-2); the two
> roadmap-open behavior decisions are resolved inside the frozen rules (AR-6, AR-7)
> **Last Updated**: 2026-10-16 (planning)
> **Batch**: K of the defect mandate (the 20 open 🟠 majors; 19 red, `DEF-209` verified fixed)
> **Auto-design**: active — policy version 1 (the standing user mandate: all defects first,
> batches continue without further go-aheads; criticals first, then majors). Reserved decisions
> (spec text, new user-visible behavior, acceptance criteria) are **not** touched: every fix
> below is conformance to the frozen specification. Rows whose fix needs a spec or registry
> ruling stay parked and are collected in AR-2.

## Why this batch

The batch-J critical (`DEF-303`) and majors are fixed; the remaining open backlog is led by
exactly these 20 🟠 majors. They come from six machineries — the module-initializer call/guard
emission (batch F Phase 1/2/4 reviews and the batch G memory audit), the batch G Phase 2 element
surface, the batch G Phase 3/4 member and conversion surfaces, the batch G Phase 4/5 null/capture
destinations, and the batch E/batch F/close-out standalone emissions — so one batch with six
machinery-coherent phases keeps the per-phase review surfaces coherent and reuses the origin
suites as the regression oracle. Every row already carries a reproduction, a base/head
observation, and a fix direction; all 20 were re-probed at the head (AR-3).

| # | Category | Ambiguity / Gap | Options Presented | Resolution | Status |
|---|----------|-----------------|-------------------|------------|--------|
| 1 | Scope | Which rows batch K closes | A) the 20 open 🟠 majors, severity-first; B) the six older ruling-parked rows; C) the batch-J follow-ons | **A** — pre-decided by the dispatch and recorded, not re-litigated: these 20 lead the backlog by count, share the origin batches' machinery, and carry fresh evidence with fix directions. B stays parked (rulings due); C forms the batch after this one | ✅ Resolved (pre-decided) |
| 2 | Scope | Rows excluded on purpose | — | **The six older ruling-parked rows** (`DEF-167`, `DEF-169`, `DEF-171`, `DEF-190`, `DEF-191`, `DEF-263` — including the two majors `DEF-167`/`DEF-171`) **and the batch-J follow-on rows** (`DEF-311`–`DEF-334` minus the closed `DEF-331`/`332`/`333`), which are the next batch. None of the 20 needs a spec or registry edit | ✅ Resolved (pre-decided) |
| 3 | Verification | The current status of the 20 rows at the head | probe the actual compiler (`80433a6`) vs the roadmap notes | **The AR-3 probe table below** — `DEF-209` is verified fixed (green control); `DEF-229` is partial (the direct member read green, the type-sensitive use red); the other 18 confirmed red. One reclassification: `DEF-209` → already fixed | ✅ Resolved |
| 4 | Design | Phase grouping and sequencing | A) six phases by origin machinery, executed 1 → 2 → 3 → 4 → 5 → 6; B) one row per phase; C) origin-batch phases (batch F/G rows together) | **A** — P1 initializer call/guard/view rows; P2 element placeholder/inference/bounds; P3 member typing; P4 call conversions/`ffi`; P5 null destinations/capture stores; P6 standalone emissions. Option C mixes unrelated surfaces (the batch G rows alone span four machineries); one row per phase multiplies the review overhead. The order follows the dependency direction (AR-4) | ✅ Resolved (delegated) |
| 5 | Design | Per-row handling mode | red → green spec case, or a control where the probe shows the row fixed | **Red → green for 19 rows**; `DEF-209` is a green control (its shapes stay build-clean and ASan-clean), and `DEF-229`'s probe-verified green direct-read spelling is a control beside its red type-sensitive case. Every case must fail without the fix (or pin the control's shape) | ✅ Resolved (delegated) |
| 6 | Behavior | `DEF-233`'s value-form enum member rule | A) reject with the frozen unknown-member code `E10212` on both the concrete and the instance paths; B) emit the member constant | **A** — Ch 04 §8 spells the enum member form `Type.MEMBER`; a value receiver has no member surface, so the read reports the registered unknown-member family at both paths. B would add acceptance the frozen member table does not define (a new language surface) and would invent the base's evaluation; it is rejected | ✅ Resolved (delegated) |
| 7 | Behavior | `DEF-211`'s fixed-array `sizeof` value | A) the size of the whole type (extent × element), the natural reading of `sizeof(type)`; B) park for a spec ruling | **A** — Ch 04 §11 types `sizeof(type)` as the size of the type, and Ch 11 §7 supplies the layout; rendering the *element* type's size for an operand typed `i32[2]` is wrong under every reading of the spec, so the fix is conformance and needs no edit. The emitted value becomes `sizeof(element) * extent` (or an equivalent whole-type spelling) | ✅ Resolved (delegated) |
| 8 | Design | `DEF-242`'s deferred conversion shape | A) a per-instance conversion duty mirroring the iterable/storable/out-slot duties; B) reject a parameter-typed argument at the declaration | **A** — the duty keeps legitimate generic spellings accepted (`T = cstring`) and reports the frozen `E10200` only at instances where the concrete destination cannot receive the argument; B over-rejects every abstract body. The duty covers the argument, return, and declaration positions the row names | ✅ Resolved (delegated) |
| 9 | Naming | New suite pair vs origin suites | — | **Origin suites, additions only**: P1/P6(`DEF-211`) append to `module_initializer_integrity_{spec,impl}_test.rs`; P2/P3/P4/P5 append to `checker_conformance_{spec,impl}_test.rs`; `DEF-170` to `callable_release_integrity_{spec,impl}_test.rs`; `DEF-189` to `freestanding_spec_test.rs`; `DEF-255` to `def_backlog_closeout_{spec,impl}_test.rs`. A new `residual_major_closeout_{spec,impl}_test.rs` pair is created only if a phase cannot host a case naturally (AR-9) | ✅ Resolved |
| 10 | Lenses | Per-phase reviewer dispatch | — | P1: correctness + emission-integrity + memory-safety; P2: correctness + semantics + emission-integrity + memory-safety; P3/P4: correctness + semantics + emission-integrity; P5: correctness + memory-safety + semantics; P6: correctness + emission-integrity + semantics. The read-only auditors stay read-only | ✅ Resolved |
| 11 | Verification | `DEF-189`'s probe and case legs | — | The freestanding host no-libc run (`-ffreestanding -fno-builtin -nostdlib`) plus the m68k link; `DEF-245`'s handle spelling uses the hosted-amiga driver build (`m68k-amigaos-gcc -mcrt=nix20 ... -lamiga -lm`). Both legs are already in the repository's suites and toolchain scripts | ✅ Resolved |
| 12 | Design | No spec, registry, or golden edit | — | Every in-scope fix is conformance: no new diagnostic code, no registry row, no spec chapter edit, no golden change; the rejections reuse `E10012`, `E10034`/`E10035`, `E10200`, `E10212`, `E10216`, `E10401`, and the runtime traps `E11211`/`E11213` | ✅ Resolved |
| 13 | Design | `DEF-245`'s null-shaped destination fix | A) lower the null shape to the destination's null value (the zero view / `NULL`), per Ch 17's `T?` lowering rule; B) reject the destination positions with `E10012` | **A** — Ch 17 requires `T?` to lower to the same C type as `T` with null as zero/NULL, so the lowering is the conformance fix and keeps `let s: str? = x;` accepted; the comparison/copy/literal acceptance stays. B remains the fallback only if a phase cannot render the typed null (the stop condition is AR-13) | ✅ Resolved (delegated, stop-gated) |
| 14 | Complexity | Escalation triggers | — | None — every fix extends existing machinery (the init lowering/release pass, the monomorphization walk and its inference helpers, the element binding/substitution/key mapping, the member dispatch and the per-instance audit, the duty pattern, the null lowering, the lend/capture store judgment, the intrinsic/shim/lambda emission sites); no new architecture, dependency, or cross-cutting refactor | ✅ Resolved |
| 15 | Deviation (runtime) | A phase's fix surface proves wider than the row | A) stop and re-plan; B) extend the phase's task files to the adjacent modules with the evidence and record the deviation here | **B** — the batch I/J precedent: a conformance fix may touch the checker or codegen module that owns the judgment; the deviation is recorded and the phase review covers the extra files | ✅ Resolved (runtime, delegated) |
| 16 | Specialist gap | Missing specialist types for this surface | — | None — `correctness-reviewer`, `semantics-reviewer`, the read-only `memory-safety-auditor`, and the read-only `emission-integrity-auditor` cover the batch; recorded in the index's Specialist Agents section | ✅ Resolved |
| 17 | Design | `DEF-207`'s nullable conditional shape | — | The null arm carries the null view: each arm constructs its own view (`cl_zero_ClStr()` for the null arm, the handle's view for the owned arm) instead of one `cl_string_view(handle)` after the conditional; a conditional at a nullable view destination keeps the arms' nullness. A body and an initializer spelling share the fix | ✅ Resolved (delegated) |
| 18 | Verification | The command set | — | `make check` per phase; the focused origin suites; `RDC_ASAN=1` memory runs on P1/P2/P5; strict builds on the pinned container (host where a leg needs it); `make verify` at close-out; the freestanding host no-libc and m68k legs for `DEF-189`; the hosted-amiga driver build for `DEF-245`'s `File?` spelling | ✅ Resolved |

## Verified Current Status (AR-3 probes, 2026-10-16, head `80433a6`)

Probe method: `make -C docker exec CMD="cargo build --workspace"`, then
`target/debug/clc --profile host -o out.c prog.clo` (plus `--profile freestanding` for
`DEF-189`/`DEF-234` and `--profile hosted-amiga` for `DEF-245`'s handle spelling), with the
stem built and run in the pinned SDK container (`cc -std=c99 -O0 -g -Wall -Wextra -Werror`,
GCC 12.2; AddressSanitizer `-fsanitize=address -no-pie` with `ASAN_OPTIONS=detect_leaks=1` for
the memory rows; the m68k link and the hosted-amiga build in the same container). Probe sources
under `/tmp/opencode/batch-k-probes/` (scratch, removed at planning close).

| Row | Probe | Verified current behavior |
|-----|-------|---------------------------|
| `DEF-170` | `import { g, h } from lib; g(5); h.call(6)` and the qualified `lib.g(5)` | ❌ Red: strict build fails — `implicit declaration of function 'g'` and `'h' undeclared` (both spellings emit the bare written name) |
| `DEF-189` | freestanding `let a: str[2] = [; "ab"];` | ❌ Red: front end accepts, m68k link OK, host no-libc build OK, run segfault 139 (the misaligned `_start` call into `main`'s 16-byte store); `DEF-234`'s `str[2]`/`len` unit segfaults identically |
| `DEF-196` | `let h: i32 = mk()(5);` and `(c ? addOne : addOne)(5)` in an initializer | ❌ Red: accepted, strict build passes, run exit 255 — the emitted initializer stores `h = (5);` (the call collapses to its argument; silently wrong) |
| `DEF-197` | `let g: i32 = pick(1, 2);` (inferred `pick<T>`) | ❌ Red: strict build fails — `implicit declaration of function 'cl_..._pick'` (no instance declared) |
| `DEF-207` | `let s: str? = c ? stringOf("abc") : null;` with the null test and `len(s)` (body and module-level) | ❌ Red: accepted, strict build passes, run segfault 139 — the null arm reaches `cl_string_view(NULL)` and the view constructor dereferences the null handle |
| `DEF-209` | `mkBox().count` in the let/return/argument and module-initializer spellings | ✅ **Fixed at the head**: the projection root binds (`__cl_owned0 = mkBox()`) and releases the owned member; strict builds, run exit 0, ASan-clean — pinned as the green control |
| `DEF-211` | `let n: usize = sizeof(i32[2]);` | ❌ Red: accepted, strict build passes, run exit 252 — the emitted `n = sizeof(i32)` measures 4 where the array type is 8 |
| `DEF-219` | `let hit: bool = yes && (firstByte(stringOf("ab")) == 97);` in an initializer | ❌ Red: strict build fails — `'__cl_itmp1' undeclared` (the owned bind is declared inside the `if` block, released after it) |
| `DEF-220` | `let e: i32 = viewOf(slice(xs, 0, 4))[1];` (inferred `viewOf<T>`) | ❌ Red: strict build fails — `implicit declaration of 'cl_..._viewOf'` and an invalid initializer |
| `DEF-229` | `id(v.y)` inside `each<S[2]>` (a member read of the loop element passed to a generic callee) | ❌ Red (residual): strict build fails — the instance keys `cl_id__unknown`; the direct member read (`total + i32(v.y)`) is a verified green control |
| `DEF-230` | `[v] () => v.y` inside `each<S[2]>` | ❌ Red: strict build fails — the environment declares `i32 v` for a `ClS` element (`incompatible types`, `request for member 'y'`) |
| `DEF-231` | `inner(v)` inside `outer<i32[2]>`; `each(x)` with `x = null` | ❌ Red: strict build fails — `cl_inner__unknown` and `cl_each__unknownQ` implicit declarations |
| `DEF-232` | `row[5]` inside `rowsum<i32[2][3]>`; the concrete `row[5]` twin | ❌ Red: the instance is accepted, emits no guard, and the out-of-bounds read runs to completion (exit 1); the concrete twin reports `E10216` (probe-verified control) |
| `DEF-233` | `let e: E = E.A; let n = e.A;` and `read<T>(x) { x.A }` at `T=E` | ❌ Red: accepted, the strict build fails with `request for member 'A' in something not a structure or union` (both paths; no diagnostic) |
| `DEF-238` | `func f(s: str): usize { return s.ptr; }`; the `.len` twin | ❌ Red: accepted, the strict build fails `-Werror=int-conversion` (a view pointer into `usize`); the `.len` twin builds and runs (green control) |
| `DEF-242` | `call<str>(v) { unsafe { Open(x, 0); } }`; the ordinary `cstring` twin; `make<i32>()` with `return x`; `let y: T = x` at `f<i32>()` | ❌ Red: each spelling is accepted and fails the strict build — `ClStr` where `char *` is declared, `returning 'void *'` where `i32` is declared, `assignment to 'i32' from 'void *'` |
| `DEF-243` | `Uses(Count())` with `ffi { func Count(): i32; func Uses(s: cstring): i32; }` | ❌ Red: accepted, the strict build fails — `passing argument 1 of 'Uses' makes pointer from integer` |
| `DEF-245` | `let s: str? = x;` and `show(x)` at `str?` (host); `let fl: File? = x;` (hosted-amiga) | ❌ Red: accepted; host fails `incompatible types ... from 'void *'` and `incompatible type for argument 1`; the hosted-amiga build fails `assignment to 'BPTR' from 'void *'` |
| `DEF-251` | `wrap<String?>(borrow out, s)` with `([borrow x, borrow out] () => { out = x; })()` | ❌ Red: accepted; the plain run aborts 134 (`free(): double free detected`), ASan reports `attempting double-free` |
| `DEF-255` | `let f: func(): void = () => assert(false, "nope");` (and the `panic` twin) | ❌ Red: accepted, the strict build fails — `'return' with a value, in function returning void`; with the warning suppressed the run would not abort |

**The one reclassification**: `DEF-209` is **already fixed** at the head (the batch H/I/J
projection-root work closed it incidentally). Its row keeps its "Backlog" text until close-out;
the phase pins its shapes as a control so the fix cannot regress.

Rows not probed beyond the table above: none — all 20 were probed.

## Resolution Notes

**AR-1:** The dispatch fixed the scope as the 20 open 🟠 majors; the register records it without
re-litigating. The batch includes `DEF-209` as the probe-verified control so all 20 roadmap rows
are accounted for by the close-out.

**AR-2:** Parked rows and the decision each needs:
`DEF-167` — the diagnostic for a call of a non-callable callee (shared with `DEF-214`);
`DEF-169` — whether to add an escaping scheme for generated type keys (an ABI decision);
`DEF-171` — whether a staged `embed` outside a constant context stays rejected;
`DEF-190` — the diagnostic for a nameless nullable source;
`DEF-191` — whether the `unsafe` exemption for the null-use rule matches the conversion rule;
`DEF-263` — whether a folded fixed-array `.len` receiver evaluates its operand.
The batch-J follow-on rows (`DEF-311`–`DEF-334`, minus the closed ones) are the next batch's
scope. None of the 20 in-scope rows needs a spec or registry edit: every fix reuses the frozen
rules and codes (`E10012`, `E10034`, `E10035`, `E10212`, `E10216`, `E10401`, `E11211`,
`E11213`, `E10200`) and the frozen layout/ABI chapters. A row that turns out to need a ruling is
parked mid-phase with the evidence (the stop conditions in AR-8/AR-9).

**AR-3:** Probe evidence is the table above; the probe sources and logs live under
`/tmp/opencode/batch-k-probes/` during planning (removed at planning close: they are scratch,
not artifacts). The verified statuses keep every roadmap row's "Backlog" text except `DEF-209`,
whose fix is recorded here and pinned at close-out; `DEF-229`'s partial status keeps its green
direct-read spelling as a control and its red type-sensitive use as the case.

**AR-4:** The phase grouping and dependency order:
P1 initializer call/guard/view conditionals (the init lowering, the release pass, the
conditional view arms) → P2 element placeholder/inference/bounds (the type-stage binding feeds
the member reads and inference keys P3/P4 extend) → P3 member typing (the value-form enum rule
and the view members) → P4 call conversions and `ffi` result typing (the duty pattern P2's
instance work exercises) → P5 null destinations and capture slot stores (independent
destinations; the duty/store judgments P4's machinery settled) → P6 standalone emission shapes
(the intrinsic, the shim, the callee symbol, the void lambda). Executed strictly 1 → 2 → 3 → 4
→ 5 → 6; `make check` gates every phase.

**AR-5:** Handling modes: 19 red → green rows. `DEF-209` is a green control (the exact probed
shapes: the let, return, argument, and module-initializer spellings, ASan-clean). `DEF-229`'s
direct member read is a pinned control beside its red case. `DEF-232`'s constant-index case has
a probe-verified concrete control (`E10216`). `DEF-238`'s `.len` twin is the acceptance control.
Every red case must be reproduced at the phase's red ref before implementation and flip green
after it.

**AR-6:** `DEF-233`'s rule: the value-form enum member read reports `E10212` on both paths. The
concrete fix is the enum arm of the member dispatch (`sema/expr.rs:569-579`), which currently
checks membership and returns the enum type; the instance fix is the same judgment in
`own/instance_check.rs::projection_resolves` (`:1003`). The `Type.MEMBER` scoped form (handled
before the member dispatch in `check_qualified`, `:485-498`) stays accepted. The design records
that the registered message names the enum type and the help line teaches the scoped spelling;
no message text changes (Ch 14 owns the text).

**AR-7:** `DEF-211`'s value: a fixed-array `sizeof` operand is the size of the whole type under
the layout rules (Ch 11 §7), emitted as the element size times the extents (or an equivalent
whole-type spelling). `offsetof` and the scalar/struct spellings are unchanged; the emitted-C
pin checks byte-identical output for non-array operands.

**AR-8:** `DEF-242`'s duty: mirroring `check_iterable_requirements`/the storable-slot duty, the
call check records a conversion duty at each site whose argument (or return/declaration value)
has a source that may resolve through a parameter and whose destination is concrete; the
instance audit substitutes and reports the frozen `E10200` once per site where the concrete
conversion is not implicit. The legitimate generic spellings (`T = cstring`, `T = str`,
`T = String`) stay accepted. Stop condition: if the duty cannot be expressed without rejecting
the instantiated spellings that convert legally, the phase stops and parks the row with the
evidence (a recorded limit changes the accepted surface).

**AR-9:** Test placement: additions only to the origin suites (AR-12). If a phase finds a case
with no natural home in an origin suite, it creates
`compiler/tests/residual_major_closeout_{spec,impl}_test.rs` with only that case (recorded at
the phase); the `AGENTS.md` test map gains the pair only at close-out if it is created.

**AR-10:** Lens justification: P1 changes init emission, ownership release scoping, and view
arm shapes (correctness + emission + memory); P2 changes the element type binding (a language
rule), the instance key derivation, and the bounds guard (correctness + semantics + emission +
memory); P3 changes the member surface and the pointer-member conversion (correctness +
semantics + emission); P4 changes conversion duties and ffi result typing (correctness +
semantics + emission); P5 changes a null lowering and a lend/capture store judgment with a
double free (correctness + memory + semantics); P6 changes a shim entry, an intrinsic value, a
callee symbol, and a void judgment (correctness + emission + semantics).

**AR-11:** `DEF-189`'s probe and case use the freestanding host no-libc leg
(`-ffreestanding -fno-builtin -nostdlib`) plus the m68k link; `DEF-245`'s handle spelling uses
the hosted-amiga driver build (`m68k-amigaos-gcc -mcrt=nix20 ... -lamiga -lm`). Both legs are
already in the repository's suites and toolchain scripts.

**AR-12:** Every in-scope fix is conformance: no new diagnostic code, no registry row, no
spec chapter edit, no golden change. The rows' rejections reuse `E10012`, `E10034`/`E10035`,
`E10200`, `E10212`, `E10216`, `E10401`, and the runtime traps `E11211`/`E11213`; the
`DEF-233`/`DEF-211` decisions stay inside the frozen member table and the layout rules.

**AR-13 (runtime):** The `DEF-245` stop condition: if the typed-null lowering cannot be
expressed at every destination without breaking the pinned null-shape acceptances (the
comparisons, the copies, the literal), the phase stops and parks `DEF-245` with the probes.
`DEF-242`'s stop condition is AR-8. Any other row whose fix needs a spec or registry edit is
parked mid-phase with the evidence and recorded in the findings ledger.

**AR-14:** Complexity: every fix rides an existing hook — the init lowering and owner pass
(`codegen/program.rs`), the monomorphization walk and the body inference helpers
(`own/generics.rs`, `own/call.rs`), the element binding/substitution (`sema/stmt.rs`,
`own/loops.rs`, `codegen.rs`, `types/keys.rs`), the member dispatch and the per-instance audit
(`sema/expr.rs`, `own/expr.rs`, `own/instance_check.rs`), the duty pattern, the null lowering
(`codegen/expr.rs`), the lend/capture store (`own/expr.rs`), and the standalone sites
(`codegen/calls.rs`, `own/lower.rs`, `compiler/runtime/shim_freestanding.c`).

**AR-15:** The runtime deviation protocol follows batch I/J: a phase's fix may touch the module
that owns the judgment beyond the task's file list; the deviation is recorded in this register
and reviewed by the phase's lenses.

## Open Questions (collected for the user — none blocks execution)

- The six parked rows' rulings (AR-2) are still due; they stay out of this batch.
- `DEF-245`'s and `DEF-242`'s stop conditions may return a parked row with fresh evidence; the
  findings ledger records either outcome.
- `DEF-209`'s incidental fix: batch K pins it; if a close-out probe shows a regression, it
  becomes a red case in P1 with the new evidence.

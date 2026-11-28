## Ambiguity Register: c-name-integrity

> **Status**: ✅ GATE PASSED — all 26 items resolved (AR-1..AR-4 from the user's invocation; AR-10 by the user; AR-5..AR-9, AR-11..AR-26 delegated by `--auto-design`, provenance below; no complexity escalation)
> **Last Updated**: 2026-11-28 (preflight amendments: AR-7, AR-14, AR-16..AR-21 refined or reopened and re-resolved; see §Preflight amendments)

Planning target: theme issue #42 (remediation roadmap R3) — findings ASM-WS3-001 (#61, DEF-589),
ASM-WS3-002 (#64, DEF-592), ASM-WS3-005 and ASM-WS3-006 (tracking issue #91).
Context artifacts: `plans/independent-assessment/findings/WS3-codegen.md` and its probes 340, 341,
342 and 360 under `plans/independent-assessment/probes/WS3/`, `spec/01-lexical.md` §2–§4,
`spec/13-data-inclusion.md` (project root), `spec/17-codegen.md` §3, §7, §11, §13,
`docs/known-limitations.md` §7, `THIRD_PARTY_NOTICES.md`, the emitter (`compiler/src/codegen.rs`,
`compiler/src/codegen/`), `compiler/src/profile.rs`, the runtime shims, the driver
(`compiler/src/clorraine/`), the VS Code task builder (`tools/vscode/src/actions.ts`), and the spec
suites that pin `#line` paths (`stage_spec_test.rs`, `windows_cli_spec_test.rs`).
Modification set: this plan folder and `plans/00-roadmap.md` only. No `spec/` change and no
spec-test change.

Mode: auto-design (policy version 1, root invocation `ad-3c91e07a5d24`). Delegated categories:
algorithms, internal interfaces, compiler mechanisms, testing strategy, implementation
sequencing. Reserved: product scope, spec or spec-test changes, user-visible behaviour changes,
legal risk acceptance, complexity escalation, outward actions.

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Which defects this plan closes | the four findings of issue #42 / a wider naming sweep | Fixed by the user's invocation of `make-plan c-name-integrity` for issue #42: ASM-WS3-001, -002, -005, -006 | ✅ Resolved |
| 2 | Scope | Which C name positions "every C99 keyword and header/NDK macro name" covers | locals and parameters only / every position a source name reaches C | Fixed by the issue #42 goal ("every"): locals, parameters (function, lambda, prototype, function-type), struct fields, closure-capture fields, and enum constants (AR-15) | ✅ Resolved |
| 3 | Scope | Which generated symbols "generic-instance symbols" covers | `cl_<f>__<key>` only / also the per-type generated helpers | Fixed by the issue #42 goal: instances plus the per-type helpers that collide the same way (Box, zero, hook support; AR-20) | ✅ Resolved |
| 4 | Scope | Same-root defects the goal does not name | include / leave out and report | Strict scope: left out of this plan and reported in the plan summary (notes) | ✅ Resolved |
| 5 | Technical | Which names a source name may not keep | keywords only / plus reserved identifiers / plus every name the unit's prelude defines or declares | AI — delegated: three classes (notes; class 3 refined by AR-19) | ✅ Resolved |
| 6 | Technical | How an escaped name is spelled | keep `cl_` prefix / stable `_<n>` suffix / `u<len>_` prefix | AI — delegated: a stable suffix from one name-only function (refined by AR-18) | ✅ Resolved |
| 7 | Technical | Where the header names come from | hand-written list / shape rule / committed list extracted from the toolchains, guarded by a completeness test | AI — delegated: committed list plus completeness test (refined by AR-19; extraction method defined by PF-008) | ✅ Resolved |
| 8 | Technical | How instance and helper symbols stay apart from user symbols | new instance prefix / escape `_`-led names / reserve generated symbols so the existing `__user` rename applies | AI — delegated: reservation plus the `__user` rename (refined by AR-20) | ✅ Resolved |
| 9 | Technical | Where `#line` goes for code that has no statement mapping | reset to the C file / map to a source or logical runtime line | AI — delegated: per-definition mapping (refined by AR-21) | ✅ Resolved |
| 10 | Scope (reserved) | The path spelling in `#line` (ASM-WS3-006): two spec suites pin the command-line spelling, including an absolute one | fix in the driver / change clc and four spec tests / document only / leave out | User: "Fix it in the driver (Recommended)" — clc keeps the pinned command-line spelling; the `clorraine` driver passes each source relative to its working folder when the file is inside it; a direct `clc` call with an absolute path still embeds it, documented | ✅ Resolved |
| 11 | Non-functional | Host builds against C libraries other than the extracted ones | extract more / residual limit | AI — delegated: glibc and MinGW-w64 are extracted (AR-19); any other C library is a residual limit in `docs/known-limitations.md` | ✅ Resolved |
| 12 | Technical | Verify commands | per AGENTS.md and the issue | AI — delegated: phase gate `make check`; final gate adds `make asan`, `make ubsan`, `make corpus-amiga`, `make bench-shape`, `make differential`, `make bench-compile`, `make toolchain-check TOOLCHAIN=<rc24>` | ✅ Resolved |
| 13 | Naming | Test and data file names | — | AI — delegated: `compiler/tests/c_name_integrity_{spec,impl}_test.rs`; data `compiler/src/codegen/c_names.txt` | ✅ Resolved |
| 14 | Feature gaps | Which docs change | none / known limitations / plus notices | AI — delegated (refined by PF-007): `docs/known-limitations.md` §7, the §11 helper rows, and a residual list, edited in the phase that causes each; `site/guide/known-limitations.md` re-evaluated every phase; `THIRD_PARTY_NOTICES.md` provenance (AR-23) | ✅ Resolved |
| 15 | Scope | Enum constants (`#define <Enum>_<Member>`) as a name position (coverage hunt) | include / leave out | Fixed by the issue #42 goal: included (evidence in the notes) | ✅ Resolved |
| 16 | Technical | How enum constants are escaped, and locals that spell an enum constant | prefix every constant / suffix the colliding ones and treat the program's constants as class 3 / emit each use as its value with no macro | AI — delegated (reopened by preflight PF-001): emit each constant use as its parenthesized value; no `#define`; the program's constants do not join class 3 | ✅ Resolved |
| 17 | Behavioral | Generated `__cl_*` names pass through `unique_local`; fallback paths spell names without it | escape in `unique_local` / a separate source-name entry point used by every source-name site | AI — delegated (refined by PF-002, PF-003): one source-name entry point; generated locals told apart by a flag on `ir::Local`; the not-in-table fallbacks keep plain `sanitize` | ✅ Resolved |
| 18 | Technical | Field and capture spellings at untyped dotted-path sites | per-struct table plus typed walks / one injective name-only function | AI — delegated (refined by PF-004, PF-009, PF-021): one injective name-only function through one member helper at every site; counter skips class members; `""` → `_0` | ✅ Resolved |
| 19 | Data & state | Extent of the name list: prelude or whole unit, per profile or one list, which toolchains | per-profile SDK prelude / one union over every supported leg and toolchain | AI — delegated: one list, the union of whole-unit names over every leg (notes) | ✅ Resolved |
| 20 | Integration | Generated symbols colliding with each other, and the families the reservation misses | exact instance set only / prefix families plus exact rename loops | AI — delegated (refined by PF-005, PF-011, PF-012): trigger families, fixed prelude helpers, a precomputed instance map that also separates instances, exact-set loops | ✅ Resolved |
| 21 | Behavioral | `#line` for lifted lambdas, hook support, and the single-string suffix | per notes | AI — delegated (refined by PF-006, PF-014, PF-019): lambda and hook lines from new site spans; a pieces accessor beside the unchanged `shim_suffix()`; "top-level item" defined | ✅ Resolved |
| 22 | Security & compliance | `-g` builds keep the C compiler's absolute working directory in DWARF | `-fdebug-prefix-map` (changes the spec/17 §13 command lines: a spec ruling) / residual limit | AI — delegated: residual limit; the plan claims the emitted C only | ✅ Resolved |
| 23 | Security & compliance | Provenance of a name list taken from third-party headers | provenance note / record that none is needed | AI — delegated: provenance note in `THIRD_PARTY_NOTICES.md`, the project's practice; no risk acceptance | ✅ Resolved |
| 24 | Integration | Driver relativization details | per notes | AI — delegated: containment by canonical paths, `/` separators, diagnostics follow, outside files unchanged | ✅ Resolved |
| 25 | Testing | Pinned spellings that change | per notes | AI — delegated: impl and unit pins updated as intended; a spec-test pin conflict stops execution | ✅ Resolved |
| 26 | Non-functional | Lookup cost and binary size of the list | per notes | AI — delegated: sorted static slice, binary search; `make bench-compile` in the final gate | ✅ Resolved |

## Coverage Map

> **Coverage**: 4/4 clusters · 12/12 categories · 22 candidates · 9 residual limits · Effort: high/xhigh→unavailable · Independence: uniform-effort
> **Freshness**: Coverage as of 2026-11-28 01:55; artifacts this register (AR-1..AR-14 at dispatch), issue #42, the WS3 findings and probes, the emitter, profile, shims, driver

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | name positions, `#line` regions, user text | every `Item` kind, emission order, 14 `#line` writers, driver ICE text, docs and site; probes for enum, lambda, `libnix` locals | vbcc predefines; a running hook program | 3 (CV-001 enum → AR-15/16; CV-002 lambda/hook → AR-21; CV-003 ICE first-error line: optional, omitted under strict scope) | general agent with the preflight-auditor brief | high |
| C2 Behavior and edges | field, capture, instance, helper, fallback sites | 10 probes on host and hosted-amiga; pins in `compiler/tests` | vbcc and freestanding extraction; list blast radius on stdlib pins | 7 (CV-001 enum → AR-15/16; CV-002 helper collisions → AR-20; CV-003 generated names → AR-17; CV-004 shim and emitter names → AR-19; CV-005 field types → AR-18; CV-006 fallbacks → AR-17; CV-007 hook/suffix → AR-21) | general agent with the preflight-auditor brief | high |
| C3 Technical and data | prelude identity, name counts, vbcc, suffix layout | measured preludes and name counts per leg; vbcc `-dM` absence; field sites | Windows headers; exact typedef counts | 6 (CV-001 prelude vs unit → AR-19; CV-002 enum → AR-15/16; CV-003 field spelling → AR-18; CV-004 vbcc → AR-19; CV-005 shim names → AR-19; CV-006 suffix pieces → AR-21) | general agent with the preflight-auditor brief | high |
| C4 Risk and stakeholders | determinism, size, debug info, rc24, licensing, AR-10 | SDK vs rc24 header trees; DWARF comp_dir probe; bench guards; VS Code tasks | legal merit of notices; vbcc in rc24; gdb stepping; lookup cost measurement | 4 (CV-001 rc24 → AR-19; CV-002 DWARF → AR-22; CV-003 AR-10 evidence → AR-10/24; CV-004 provenance → AR-23) | general agent with the preflight-auditor brief | high |

**Source note:** AR-15 ← C1-CV-001, C2-CV-001, C3-CV-002; AR-16 ← the same; AR-17 ← C2-CV-003, C2-CV-006; AR-18 ← C2-CV-005, C3-CV-003; AR-19 ← C2-CV-004, C3-CV-001, C3-CV-004, C3-CV-005, C4-CV-001; AR-20 ← C2-CV-002 and the challenger; AR-21 ← C1-CV-002, C2-CV-007, C3-CV-006; AR-22 ← C4-CV-002; AR-23 ← C4-CV-004; AR-24 ← C4-CV-003; AR-25, AR-26 ← C2 and C4 assumptions. No Discovery keys. Omitted as optional (strict scope): C1-CV-003.

Residual limits named: vbcc predefined macros beyond `__VBCC__`/`__M68K__` (vbcc has no `-dM`); C libraries other than glibc and MinGW-w64 (AR-11); toolchains other than the pinned SDK and rc24; DWARF comp_dir (AR-22); `clc` given an absolute path (AR-10); a file outside the driver's working folder (AR-24); `compiler/runtime/` files are not on a user's disk (AR-21); gdb stepping not exercised; legal sufficiency of the provenance note (AR-23).

### Resolution Notes

Root invocation `ad-3c91e07a5d24`, policy version 1 for every delegated row. Reproductions ran on
`main` at `7f63bb3c` (the probes' baseline was `e24a8752`); scratch under `build/tmp/cni/`.

**Hardening:** a blind `design-challenger` (general agent with the brief; packet without the
planner's choices) returned: name escaping **Justified** with changes (extract only what the C
sees, add vbcc and MinGW sources, one list, retire `cl_`), generated symbols **Simplify** (prefix
families; the exact instance set misses the zero helper), `#line` **Justified** with hook support
added and no directive for the prefix shim. Every change is taken in AR-18..AR-21. Its
whole-prefix-only advice (extract the prelude, not the suffix) is not taken: enum constants are
defined before the suffix and can rewrite it (AR-16), so the whole unit counts.

**AR-1:** Authority: the user's invocation. Probes 340 and 341 still fail on host, hosted-amiga,
and hosted-amiga-13 at `7f63bb3c`; probe 342 needs both files on the `clc` command line (the driver
reports E11102 for the folder layout) and still emits `cl_get__Point` twice.

**AR-2:** Authority: the issue #42 goal. Evidence at `7f63bb3c`: `struct Gps { lat: i32; long: i32; }`
fails the host and hosted-amiga builds (fields are emitted raw, `program.rs:954`, read as `s.int`);
a capture of a local named `int` fails because the environment field takes the source name
(`program.rs:746`); `func f(EOF: i32, restrict: i32, errno: i32)` and a lambda parameter `TRUE`
fail through the parameter path (`codegen.rs:941`, `function.rs:351`).

**AR-3:** Authority: the issue #42 goal. Evidence: `module box; export func alloc__i32` beside
`boxNew<i32>` emits two `cl_box_alloc__i32` definitions; `module zero; func ClRec` beside a moved
`struct Rec` gives "conflicting types for 'cl_zero_ClRec'" (challenger).

**AR-4:** Authority: strict scope. Left out and named in the summary: (a) a local or parameter
spelled like a mangled or generated *function or object* symbol (`let cl_Main_helper` before a
call to `helper`; `let ClPoint` beside `struct Point`), which shadows it; (b) two modules each with
a private generic `pick` are rejected with E11101; (c) the driver's "first error" line can show a
GCC context note instead of the error (C1-CV-003); (d) user aggregates spelled `Cl<Type>` where the
spec/17 §7 table says `Cl<Module>_<Type>` (challenger observation).

**AR-5:** Authority: AI — delegated by --auto-design. Eligibility: compiler mechanism inside the
AR-2 scope. Objective: every accepted program whose names reach C builds on every leg.
Decision: a source name is escaped when it is (1) a C99 keyword (`auto` … `while`, `inline`,
`restrict`, `_Bool`, `_Complex`, `_Imaginary`) or `bool`; (2) a reserved identifier, beginning with
`__` or with `_` and an upper-case letter (C99 §7.1.3 reserves them for the implementation; they
cover `_Pragma`, `__func__`, `__LINE__`, GCC's `__attribute__`, vbcc's `__chip`); or (3) a name the
translation unit defines or declares at file scope outside the program's own function and object
symbols (AR-19). Every other name keeps its source spelling. A name class 3 holds is not a
"unique C identifier" of the unit in spec/17 §7's sense, so escaping it, including one that would
only shadow (`free`, `div`), is the spec's own rule. Evidence: the probes; `is_c_keyword`
(`codegen.rs:1237-1246`) lists only the C89 keywords plus `inline` and `bool`; a local `memcpy`
fails because the array copy calls `memcpy`; locals `Close` and `FreeString` fail because the drop
glue calls them; the hosted-amiga prelude defines lower-case macros a camelCase local can take
(`textPen`, `detailPen`, `major`, `minor`). Rejected: keywords only (probe 341 keeps failing); a
shape rule for upper-case names (renames valid unique names, still needs the lower-case list).
Strongest counterargument: class 3 is large and toolchain-specific. Confidence: High — would change
if the prelude varied per program (measured identical for two programs per profile). Hardening:
challenger Justified. Reopen triggers: a toolchain bump, a new shim include, a new leg.

**AR-6:** Authority: AI — delegated by --auto-design. Decision: an escaped name takes a stable
suffix (spec/17 §7). The `cl_` prefix for keywords is retired: it moves a user name into the
compiler's namespace and is not a suffix. The exact function is AR-18. Strongest counterargument:
today's escaped keywords change spelling (`cl_int` → `int_0`), with one implementation pin
(`codegen_ownership_impl_test.rs:381`) and the unit asserts at `codegen.rs:1300-1307`; no spec test
pins `cl_int` (C4). Confidence: High. Reopen triggers: a spec ruling on local naming.

**AR-7:** Authority: AI — delegated by --auto-design. Decision: the class-3 names are data built
into `clc`, extracted once inside the SDK image and from the rc24 toolchain by a one-off command
recorded in `03-01`, and a completeness test re-extracts them and fails naming every missing
entry. Evidence: emission must not depend on the build machine, so headers cannot be read at
compile time. Complexity classification: not an escalation — the list is the finding's own
recommended deny-list, the extraction is a recorded command rather than committed tooling, and the
guard is an ordinary test using the suites' skip-without-toolchain pattern. Confidence: High.
Reopen triggers: the list needs committed generator tooling to stay maintainable.

**AR-8:** Authority: AI — delegated by --auto-design. Decision: generated symbols are reserved so
the existing `__user` rename (`codegen.rs:835-845`) moves a colliding user function or global; one
helper spells an instance symbol for the four call sites (`program.rs:1019`, `program.rs:4896`,
`expr.rs:1279`, `function.rs:213`). Rejected: a `clg_` prefix (changes the spec/17 §7 row; needs a
ruling); escaping `_`-led names (misses `module get_; func Point`, `alloc__i32`, `ClRec`).
Confidence: High. Details in AR-20.

**AR-9:** Authority: AI — delegated by --auto-design. Decision: every top-level definition after
the first `#line` starts with its own `#line`: a function or instance at its `func` line (covering
its header and hoisted declarations), a module initializer at its `module` line, the entry stub at
the root `main` declaration. No directive is added for the prefix shim, whose real C lines are
already right and which `windows_cli_spec_test.rs:156-197` requires to carry no foreign `#line`
name on host. Rejected: `#line <n> "<unit>.c"` (embeds the output path, which can be absolute, and
breaks that spec test). Evidence: in hello on hosted-amiga the last `#line`
(`stdlib/std.io.clo:73`) covers the stub and about 5,000 appended shim lines. Confidence: High.
Details in AR-21.

**AR-10:** Authority: the user (direct choice, 2026-11-28). "Fix it in the driver (Recommended)".
Context shown: `stage_spec_test.rs::line_directives_name_the_logical_cli_path` pins `sub/main.clo`;
three `windows_cli_spec_test.rs` cases pin the absolute directory; the extension passes absolute
paths with the workspace root as working folder. Consequences stated: driver diagnostics for
editor builds show workspace-relative paths; a direct `clc` call with an absolute path still embeds
it, documented. Details in AR-24.

**AR-11:** Authority: AI — delegated by --auto-design. The list covers glibc and MinGW-w64 (both in
the SDK image, AR-19); a host build against another C library can still meet an unlisted macro.
Recorded in `docs/known-limitations.md`.

**AR-12:** Authority: AI — delegated by --auto-design. AGENTS.md gates plus the issue's list, plus
`make differential` (host vs hosted-amiga stdout), `make bench-compile` (AR-26), and
`make toolchain-check TOOLCHAIN=<rc24 root>` because the shipped toolchain's headers differ
(AR-19). All gates run with `IMAGE=clorraine-sdk:windows-cli`.

**AR-13, AR-14:** Authority: AI — delegated by --auto-design; AGENTS.md naming and the
docs-follow-the-code rule. The site's known-limitations page has no symbol-naming rows
(searched), so no site change.

**AR-15:** Authority: the issue #42 goal. Evidence: `program.rs:999-1001` writes
`#define {Enum}_{member} {value}` after the prefix shim, used raw at `expr.rs:585`;
`enum INT : i32 { MIN, MAX }` fails host and hosted-amiga (`"INT_MAX" redefined [-Werror]`),
`enum MEMF : u32 { PUBLIC, CHIP }` fails hosted-amiga; release lines carry no `-Werror`
(`toolchain.rs:116-120`) and the suffix uses `MEMF_CHIP` after the user's `#define`, so a release
build can compile with the runtime reading the user's value; `let Color_Red` beside
`enum Color { Red }` fails host.

**AR-16:** _(Superseded by §Preflight amendments; original text kept for the audit trail.)_ Authority: AI — delegated by --auto-design. Decision: an enum constant whose spelling
`<Enum>_<Member>` is in class 1–3 is spelled with the AR-18 function at the `#define` and at every
use, through one helper; the program's final enum-constant spellings join class 3 for every other
source name, so `let Color_Red` is escaped. Rejected: prefixing every constant (`Cl<Enum>_<Member>`
churns every enum and can meet a user typedef `ClColor_Red`). Confidence: High. Reopen triggers: a
constant emitted outside the helper.

**AR-17:** _(Partly superseded by §Preflight amendments: the fallbacks are not escaped.)_ Authority: AI — delegated by --auto-design. Decision: one source-name entry point
(`source_c_name`-style, exact name chosen by the executor) applies classes 1–3 and the AR-18
spelling; every source-name site uses it: locals, parameters, lambda parameters, the
`local_names` fallbacks (`expr.rs:241,247,631`, `function.rs:608,663`) and the func-typed callee
fallback (`expr.rs:1370-1375`); `unique_local` keeps serving generated names (`__cl_*`, `__env`,
`__cl_ret`), whose spelling does not change. Evidence: about 60 generated callers of
`unique_local` and 270 pinned `__cl_*` spellings (C2). Confidence: High. Reopen triggers: a pinned
generated spelling changes.

**AR-18:** _(Refined by §Preflight amendments.)_ Authority: AI — delegated by --auto-design. Decision: a name `n` is *marked* when `n`,
or `n` with one or more trailing `_<digits>` groups removed, is in class 1–3. `escape(n)` is
`n + "_0"` when `n` is marked, else `n`. It is injective: appending a fixed suffix is injective,
and an escaped result is itself marked (its ancestor `n` is), so it never equals a kept name. The
`_0` digit keeps it apart from the uniqueness counter, which starts at `_1`. The function needs no
type, so every field site, capture site, and dotted path (`expr.rs:583-635,711-735`,
`drop.rs:95,236`, `guards.rs:1564,1639`, `program.rs:1468,1551,2781`, `function.rs:3148`) spells
a member as `escape(name)`; locals and parameters pass the result through the existing counter for
shadowing. A class-2 result stays reserved-shaped (`__LINE___0`) and is accepted because it is not
in class 1 or 3. Data invariant, asserted by the completeness test: no class-1 or class-3 entry is
`x + "_0"` for a marked `x`. Examples: `int` → `int_0`; a literal `int_0` → `int_0_0`; `EOF_2` →
`EOF_2_0`; `count` → `count`. Rejected: a per-struct table (needs the struct type at untyped
sites, C3-CV-003); the shortest-`_1`-run rule first drafted here (not injective when `EOF_1` is
itself listed). Strongest counterargument: a literal `EOF_2` is renamed too; harmless and stable.
Confidence: High. Reopen triggers: the invariant fails for a new list.

**AR-19:** Authority: AI — delegated by --auto-design. Decision: one sorted list,
`compiler/src/codegen/c_names.txt`, the union over every supported leg of the non-reserved names
the whole emitted unit defines or declares outside the program's own function and object symbols:
object- and function-like macros (`-dM` with the leg's exact flags, so `libnix` from `-mcrt=nix13`
is caught), typedefs, functions, and objects, taken from a fixed sample program's unit so the
emitter's fixed typedefs (host `BPTR`, `ClStr`) and the shim's own names (`Close`, `FreeString`,
`CL_*`, `cl_zero`) are included. Legs: host GCC/glibc; host MinGW-w64; m68k GCC SDK for
hosted-amiga (whole unit with the suffix), hosted-amiga-13 (`-mcrt=nix13`), freestanding (m68k and
host); vbcc for hosted-amiga through GCC's preprocessor with vbcc's include order
(`-nostdinc -undef -D__VBCC__ -D__M68K__`, vbcc then NDK includes) plus every `#define` name in
vbcc's own include directory; the rc24 toolchain's two hosted profiles. One list keeps names
stable across profiles; an extra escape only renames. Measured sizes: about 7,250 names for the
hosted-amiga unit, 2,850 hosted-amiga-13, 300 host, 2 freestanding, plus 38+ vbcc and the rc24 delta
(`FOPEN_MAX`, `TMP_MAX`). The completeness test runs the same extraction for every leg available in
the SDK image during `make check`, and for rc24 when `CLORRAINE_RC24_ROOT` names a root; a shim or
header change that adds a name fails it with the missing list. Confidence: Med — vbcc's own
predefines are not enumerable (residual). Reopen triggers: a new leg or toolchain pin.

**AR-20:** _(Partly superseded by §Preflight amendments: zero helpers per emitted struct unit; one instance map.)_ Authority: AI — delegated by --auto-design. Decision: the rename *trigger* for a user
function or global (`is_reserved_symbol`) adds the families `cl_<g>__` for every declared generic
function `g` (from the AST), `cl_box_alloc__`, `cl_box_drop__`, `cl_zero_Cl`, and `cl_hook_`, next
to the existing ones; the instance helper applies the same trigger against the helper families
(`cl_box_*__`, `cl_zero_Cl`, `cl_lambda_`, `cl_hook_`) and the shim set, so an instance of a user
generic named `box_alloc` or `lambda_0` takes the `__user` suffix instead of meeting the helper.
The rename *loops* check exact sets only: shim symbols, user symbols, every emitted instance
symbol, and every emitted helper (zero helpers per moved struct type, Box helpers per key, lambda
specializations, hook support), all computed before emission; so a family never loops and a
`__user` candidate equal to a real helper (a struct named `Rec__user`) still moves on. Rejected:
exact instance set only (misses the zero helper). Confidence: High. Reopen triggers: a generated
symbol emitted that the exact set does not hold.

**AR-21:** _(Partly superseded by §Preflight amendments: separators before the next named piece; `shim_suffix()` unchanged.)_ Authority: AI — delegated by --auto-design. Decision: refines AR-9. A lifted lambda maps
to its lambda expression's line; hook trampolines and wrappers to their hook lambda's line; the
hook env-stack helpers to the first hook lambda's line. `Profile` exposes the suffix as an ordered
list of named pieces (the six files, `picasso96.h`, and the two literal separators placed outside
any file's mapping); each file starts with `#line 1 "compiler/runtime/<file>"`, matching how
`stdlib/<module>.clo` follows the repository path. Confidence: Med — `compiler/runtime/` files are
not on a user's disk, so gdb shows no source for runtime frames (residual). Reopen triggers: a
debugger test needing another attribution.

**AR-22:** Authority: AI — delegated by --auto-design. Evidence: a unit whose only `#line` is
relative still puts the absolute working folder into the host and m68k objects (DW_AT_comp_dir,
probe C4); the fix is `-fdebug-prefix-map`, which changes the spec/17 §13 command lines and needs a
ruling. The plan claims the emitted C only and records the debug-info residual.

**AR-23:** Authority: AI — delegated by --auto-design. Decision: a provenance note in
`THIRD_PARTY_NOTICES.md` names the headers the identifier list was read from (glibc, MinGW-w64,
NDK 3.2, libnix, vbcc target headers, the rc24 toolchain) and states that only identifier names are
kept, no declarations or text. This follows the project's practice for header-derived material
(`THIRD_PARTY_NOTICES.md:69-82`); it accepts no risk. Whether a name list needs a notice at all is
a legal question the plan does not answer (summary).

**AR-24:** Authority: AI — delegated by --auto-design, inside the user's AR-10 choice. Decision: the
`clorraine` driver, before compiling, rewrites each source path that lies inside its working
folder — compared after canonicalizing both, with the Windows verbatim prefix stripped by the
existing `platform.rs` helper — to the path relative to that folder with `/` separators; a path
outside it is passed unchanged. The rewritten path feeds the source map, so `#line` and the
driver's diagnostics both show it. `clc` is unchanged. Evidence: VS Code passes absolute
`realPath` names with the workspace root as cwd (`actions.ts:63-80,290-295`); `stage_spec_test`
and `windows_cli_spec_test` drive `clc` and stay as they are; no driver test pins an absolute
diagnostic path (searched). Confidence: High. Reopen triggers: a driver spec test pins an absolute
spelling.

**AR-25:** Authority: AI — delegated by --auto-design. Pins in `*_impl_test.rs` and unit tests whose
text changes because of AR-6, AR-16, AR-18, AR-20, or AR-21 are updated as intended emission
changes, each listed in the phase notes. A `*_spec_test.rs` whose pinned spelling would change
stops execution for a user ruling (AGENTS.md).

**AR-26:** Authority: AI — delegated by --auto-design. The list is embedded with `include_str!` and
parsed once into a sorted `&'static [&'static str]` (or checked sorted at test time) and probed by
binary search; about 100 KB of text against a 6.45 MB release `clc`. `make bench-compile` runs in the
final gate; a regression over its existing guard is a finding.

## Preflight amendments

Preflight (`00-preflight-report.md`, `--auto-design`, policy version 1, root invocation
`ad-pf-7b2e91c4`) found new codebase evidence that reopens AR-16 and refines AR-7, AR-14,
AR-17..AR-21. **Where an original note above disagrees with this section or with the 03-*
design documents, this section and 03-* win** — in particular the AR-17 fallbacks keep plain
`sanitize`, AR-20's zero helpers are per emitted struct unit, and AR-21's literal separators are
written before the next named piece's `#line`. Each amended decision keeps its original category; the full provenance, rejected
alternatives, and challenger verdicts are in the report's findings.

**AR-16 (reopened, PF-001):** Authority: AI — delegated by --auto-design. Eligibility: compiler
mechanism; spec/17 §3 asks only for "an emitted constant" and spec/17 §7 says C macros are not
used. New evidence: an escaped local `INT_MAX` and the escaped constant of `enum INT { MAX }` both
spell `INT_MAX_0`; any further suffix patch fails (`enum Color { Red, Red_0 }` against a local
`Color_Red`); the macros also rewrite suffix tokens no file-scope list holds (`io_Command`,
`font_size`), user symbols, and each other (`enum A_B { C }` and `enum A { X, B_C }` both define
`A_B_C`; a release build prints a wrong value). Decision: the one use site (`expr.rs:584-585`)
emits the constant's folded value as `(<value>)`; the `#define` loop (`program.rs:999-1001`) is
removed; the program's constants no longer join class 3. Rejected: suffixing colliding constants
(collides with escaped locals); `#undef` before the suffix (prefix, user-code, and
constant-vs-constant collisions remain). Strongest counterargument: emitted C loses readable
constant names (cosmetic; macro names are not in `-g` debug info). Confidence: High. Hardening:
challenger chose the same option. Reopen triggers: a site other than `path()` reads the
`<Enum>_<Member>` spelling.

**AR-17 (refined, PF-002, PF-003):** generated owners `__cl_owned<N>` share the locals table; a
`generated` flag on `ir::Local`, copied from the binding, keeps them on plain `unique_local` (a
spelling pattern would miss a source local named `__cl_owned0`). The fallbacks reached only by
names outside the table (`ffi` function values, generated paths) keep plain `sanitize`.

**AR-18 (refined, PF-004, PF-009, PF-021):** the member spelling is one helper and the contract is
"every member site calls it", proved by keyword-named fields through every access form under the
strict builds; the counter skips class members for source names; an empty name is `_0`.

**AR-20 (refined, PF-005, PF-011, PF-012):** instance symbols are assigned once, before emission,
in monomorphization order, and a later instance meeting an earlier one takes `__user`; zero
helpers are per emitted struct unit plus `cl_zero_ClStr`; `cl_str`, `cl_string_view`, and
`cl_zero_ClStr` join the reserved and exact sets.

**AR-21 (refined, PF-006, PF-014, PF-019):** `shim_suffix()` stays as it is (a spec test reads it)
and a pieces accessor feeds emission; lambda and hook spans are new fields on their sites; a
"top-level item" is a function, file-scope object, or struct definition outside the suffix.

**AR-7, AR-19 (refined, PF-008):** typedefs, objects, and file-scope enumerators come from a
std-only depth-0 scan of the `-E` output shared by the one-off command and the test, with a
recall check against `-aux-info`; a scanner inside a test is not committed generator tooling.

**AR-14 (refined, PF-007):** the AGENTS.md prime directive (after this register was written)
moves each limitation edit into the phase that causes it and re-evaluates the site summary every
phase.

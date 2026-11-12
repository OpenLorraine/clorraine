# Ambiguity Register: Template Placeholder Residuals II

> **Status**: ✅ GATE PASSED — all 21 items resolved
> **Last Updated**: 2026-11-12 02:00
> **Authority mode**: auto-design (root invocation
> `codeops-20261112-0200-template-placeholder-residuals-ii`, policy version 1)

## Register

| # | Category | Ambiguity / Gap | Options Presented | Decision | Status |
|---|----------|-----------------|-------------------|----------|--------|
| 1 | Behavioral | `text(null)` is accepted and emits `cl_text_unknown(NULL)`, failing the C build | reject E10500 at the argument / reject E10012 / accept as empty text | Reject E10500 (`type 'null' is not formattable`) at the argument; no C is written | ✅ Resolved |
| 2 | Behavioral | A concrete non-formattable argument to explicit `text` (`text(someStruct)`) has no diagnostic | E10500 / E10200 / leave | Reject E10500 with the frozen formattable rule, matching a template placeholder | ✅ Resolved |
| 3 | Behavioral | A generic argument to explicit `text` still emits a nonexistent converter at the instance | defer to the instance / reject at the declaration / leave | Record a deferred formattability/nullability duty and report at the first offending instance; narrowed and formattable instances stay accepted | ✅ Resolved |
| 4 | Behavioral | `$"{null + 1}"` is accepted and emits invalid pointer arithmetic C | E10012 at the null / E10200 / leave | E10012 at the null operand: a bare null has no operator surface outside `==`/`!=` with a null-capable sibling | ✅ Resolved |
| 5 | Behavioral | A generic `flag ? null : x` at `T = str` reports E10012+E10010, while the concrete twin is an accepted empty `str` view | accept / E10010 / E10012 | Accept at `T = str` (and every bare-null-capable result): the null is the empty view state | ✅ Resolved |
| 6 | Behavioral | The same generic conditional at `T = String` reports E10012+E10010, while the concrete twin reports only E10010 | E10010 only / E10012 only / two reports | E10010 only at the null arm: the settled result is `String?`, and the placeholder requires a non-null value | ✅ Resolved |
| 7 | Behavioral | The same generic conditional at a scalar or struct `T` carries a spurious E10010 beside the E10012 null report | E10012 only / E10011 / leave | E10012 only at the null arm, matching the concrete scalar settlement | ✅ Resolved |
| 8 | Behavioral | `let x = flag ? null : 5;` is accepted and emits `(flag ? NULL : 5)` | E10012 at the null / E10011 / leave | E10012 at the null arm when the settled result is concrete, non-nullable, and not a bare-null-capable type | ✅ Resolved |
| 9 | Technical | How the concrete conditional and the generic instance share the null-arm rule | duplicate the rule / shared helper / only fix the instance | One shared `types` helper that classifies a null arm against its settled sibling; both callers use it | ✅ Resolved |
| 10 | Behavioral | A for-step owning assignment (`s = $"..."`) keeps the compact C `for` header and leaks the old value | force the flat form / pre-drop in the header / leave | `for_step_is_structured` rejects an `Assign` whose target may own, so the flat block runs the ordinary pre-drop store | ✅ Resolved |
| 11 | Behavioral | A template in a for-init expression list is checked but never lowered, so its effect does not run | lower through `check_expr_statement` / evaluate eagerly / leave | Lower each init expression through `check_expr_statement` with a release floor and drain, exactly as an ordinary statement | ✅ Resolved |
| 12 | Behavioral | A non-`needs_stores` aggregate's field order relies on the C initializer's declaration order, which C99 leaves unspecified | fieldwise store every multi-field aggregate / document the supported-toolchain assumption | Document the assumption in the codegen contract and pin the GCC host behavior with a same-order and a differing-order test; vbcc is compiled and run on `vamos` by the SDK smoke/corpus leg; no emission-wide change (the issue authorizes this option) | ✅ Resolved |
| 13 | Behavioral | `cl_panic` passes a NULL message with length 0 to `fwrite`, which UBSan reports | skip the zero-length write / substitute an empty string / leave | Skip the `fwrite` when `len == 0` in both shims, keeping each shim's existing stream (host stderr, Amiga stdout); the host runtime is tested and the Amiga body will be pinned by a shim-parity test and built by `make verify` | ✅ Resolved |
| 14 | Scope | Whether #24 (`text` arity) is part of this plan | bundle / separate mini-plan | Separate lightweight plan `plans/text-arity/`; this plan does not change arity | ✅ Resolved |
| 15 | Scope | Whether any spec, stdlib, or ABI artifact changes | edit spec / no edit | No `spec/` or `stdlib/` edit; no ABI change. The behavior follows the frozen formattability, null, conditional, source-order, and zero-state rules | ✅ Resolved |
| 16 | Naming | Plan and suite names | — | `template-placeholder-residuals-ii`; `template_placeholder_residuals_ii_{spec,impl}_test.rs` | ✅ Resolved |
| 17 | Non-functional | Verification and regression budget | full gate + 1% guard / correctness only | Full gate (`check`, `asan`, `ubsan`, `verify`, `bench-shape`, `bench` + `bench-compare`) against `bb3956c`; every scored output equal and no scored metric above 1% | ✅ Resolved |
| 18 | Edge cases | A null literal compared with a null-capable sibling (`x != null`) must stay legal | keep / reject | Keep; the null rule allows `==`/`!=` when the other side is null-capable (or null-shaped) | ✅ Resolved |
| 19 | Security | New input, parser, FFI, or unsafe surface | — | None: diagnostics, a deferred duty, two internal lowering/cleanup paths, and a runtime length guard | ✅ Resolved |
| 20 | Behavioral | `text(s)` with an unnarrowed `String?` argument is accepted, while a template placeholder reports E10010 | reject E10010 / accept (the runtime copy is null-safe) / defer | Reject E10010 at the argument unless the flow narrowed it; the explicit converter is a concrete use, exactly like a placeholder. The runtime null safety stays, but it does not waive the language rule | ✅ Resolved |
| 21 | Behavioral | `text(x.n)` with an opaque parameter projection is typed `Unknown` and emits `cl_text_unknown` | reject E10500 at the opaque projection / record a deferred duty / leave | Reject E10500 at the opaque projection, mirroring the template placeholder's opaque-member gate; a generic whole-parameter argument keeps the deferred duty | ✅ Resolved |

## Delegated Resolution Records

All decisions are eligible technical consequences of the frozen specification and the
issue #28 fix directions. The shared objective is issue #28: close the seven
adjacent residuals without changing a frozen rule or adding a language surface.
Root invocation `codeops-20261112-0200-template-placeholder-residuals-ii`.
Reopen triggers are listed once at the end.

**AR-1, AR-2 — explicit `text` formattability.** The frozen placeholder
formattability rule (`spec/04-expressions.md` §12, E10500) already defines which
types are formattable. `text(v)` is the explicit spelling of the same
conversion, so a null-shaped or concrete non-formattable argument reports
E10500 at the argument with the frozen registry message and help
(`convert with `text`, `toHex`, or `toDecimal``). Rejected: accepting null as empty text (the explicit
converter has no expected type, exactly like a bare placeholder) and E10012
(formattability owns the report). Evidence: `let t: String = text(null);`
emits `cl_text_unknown(NULL)`; `text(Box { n: 1 })` declares no converter.

**AR-3 — generic `text` argument.** The declaration cannot judge a parameter
type, so the argument records the same deferred duty the template placeholder
uses: at each instance the substituted type reports E10500 when not
formattable, and E10010 when nullable and unnarrowed. Rejected: rejecting the
generic body at the declaration (a formattable instance must stay legal).
Evidence: `show<Box>` emits `cl_text_Box`, which fails the strict C build.

**AR-20 — nullable `text` argument.** The explicit converter is a concrete
use, so an unnarrowed nullable argument reports E10010 exactly as a template
placeholder does; a narrowed argument stays accepted. Rejected: accepting it
because `cl_string_copy`/`cl_string_view` are null-safe (runtime safety does
not make the use typed) and rejecting every nullable argument (a narrowed one
is valid). Evidence: `let s: String? = stringOf("x"); text(s)` is accepted on
the base; a template placeholder over the same value reports E10010.

**AR-21 — opaque `text` projection.** `text(x.n)` over a parameter is typed
`Unknown` and emits `cl_text_unknown`. The template placeholder rejects the
same projection with E10500 at the placeholder; the explicit converter uses
the same opaque-projection gate. A whole-parameter argument (`text(x)`) is not
`Unknown`; it records the deferred duty from AR-3. Rejected: emitting a
fallback converter and deferring the failure to the C build.

**AR-4 — bare null operand in a binary expression.** A literal null is the
`T?` context type (`spec/04-expressions.md` §6); with no expected type it has
no operator surface. The existing rule excludes a literal null from the
null-operand check, so `null + 1` and `null + null` reach codegen. The rule now
treats a literal null like any null-shaped operand: E10012 unless the operator
is `==`/`!=` and the sibling is null-capable or null-shaped. `&&`/`||` keep the
frozen E10218 condition report (the spec explicitly names `null && true`).
Rejected: special-casing arithmetic operators (the null rule owns the report),
replacing the frozen E10218 with E10012, and accepting null as an integer zero
(the language has no such conversion).

**AR-5, AR-6, AR-7 — generic null-arm parity.** The concrete rule settles a
null arm with its sibling: a bare-null-capable view/`raw`/`cstring`/slice keeps
the empty state, a handle/`Box` settles to its nullable form, and a scalar or
struct rejects the null. In a placeholder, a non-formattable settlement (a
`raw`/`cstring`/slice arm) still reports E10500, so the acceptance claim applies
to the settlement, not to every formatting context. The base's scalar/struct
generic conditional reports E10012+E10010; the fix leaves the single E10012. The instance duty must apply the same settlement after
substitution. The placeholder context adds the E10010 possible-null rule when
the settled result is nullable. Rejected: keeping the destination-only
`null_arm_needs_report` for an inferred placeholder (it cannot see the settled
sibling) and dropping the null-arm report entirely (unsound for scalars).

**AR-8, AR-9 — concrete scalar null arm.** The declaration has no expected
destination, so `let x = flag ? null : 5;` currently accepts. A shared
`types` helper classifies the null arm against the settled sibling; the
concrete conditional reports E10012 for the non-bare-null, non-nullable
result. The generic instance calls the same helper. Rejected: E10011 (that
code is for an explicit scalar nullable type), and reporting only in a
placeholder (the plain store is equally unsound).

A written expected destination keeps `report_null_conditional_arm`, exactly as
before; a result that still names a parameter defers to the instance duty.

**AR-10 — for-step owning assignment.** The compact C `for` header can carry
only a plain assignment expression; the ordinary store's pre-drop is a
statement. A step assignment whose target owns (or may own after substitution)
therefore forces the flat label/`goto` form, where the existing store emits the
save/drop. Rejected: a comma-expression drop (the drop is not an expression)
and leaving the leak (O3 requires releasing the old value).

**AR-11 — for-init expression effects.** `ForInit::Expr` currently calls
`eval`, which checks but does not lower a template, call, conditional, or
increment into its run-time effect. Lowering each expression through the
ordinary `check_expr_statement` path runs every effect and binds a discarded
owner to a statement temporary; a release floor/drain frees it once before the
loop. An assignment's own store already ran and keeps its shape. Rejected:
lowering the whole init list as one expression (the grammar is a
comma-separated list, and each expression is a separate statement) and
special-casing templates (an increment has the same defect).

**AR-12 — aggregate initializer order.** The C99 standard leaves initializer
list evaluation order unspecified. The compiler already forces fieldwise
stores when the written order differs from the declaration order; when the
orders agree, the emitted initializer lists fields in declaration order.
GCC (the host compiler used by the tests and benchmarks) evaluates that list
in order, and the corpus and benchmark outputs confirm it. vbcc is compiled and
run on `vamos` by the SDK smoke/corpus leg (`spec/17-codegen.md` §13), so both
supported compilers execute the contract. The codegen contract documents this
assumption and the new regression test pins both the same-order (`12`) and
differing-order (`21`) results on the host. Rejected: converting every multi-field literal to fieldwise stores
(an emission-wide change with no observable benefit on a supported toolchain,
and a scored-regression risk).

**AR-13 — `cl_panic` zero length.** A zero-length panic message is a null
pointer with length zero; `fwrite` with a null buffer is undefined even for
zero bytes. The panic keeps the prefix and newline and skips only the message
write, keeping the host stream (`stderr`) and the Amiga stream (`stdout`) as
they are. The host runtime is tested; the Amiga body will be pinned by a
shim-parity assertion and compiled by `make verify`. Rejected: substituting
`""` (equivalent output, more work), changing the Amiga stream, and leaving
the UBSan report.

**AR-14 to AR-19 — scope, naming, verification, edge, security.** No `spec/`,
`stdlib/`, or ABI edit; #24 stays a separate mini-plan. The full gate runs on
the pinned SDK image; the FS-UAE legs use the same private-Xvfb procedure as
PLAN-31. `x != null` and `x == null` stay legal. No new input or unsafe
surface.

## Reopen Triggers

- The generic `text` duty rejects a formattable or narrowed valid instance
  (AR-3).
- The shared null-arm helper changes a written-expected destination report or
  #26 ST-37 (AR-5 to AR-9).
- The flat for-step form changes a scored benchmark or a non-owning step's
  emitted shape (AR-10).
- The for-init statement path double-runs or double-releases an effect (AR-11).
- `cl_panic` still reaches a zero-length `fwrite` on either shim (AR-13).
- `make bench-compare` shows a scored regression above 1% (AR-17).

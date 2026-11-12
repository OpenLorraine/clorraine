# Current State: Template Placeholder Residuals III

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## 1. Baseline

All reproductions were run on `541ba27` (clean `main == origin/main`) with
`target/debug/clc` and, for runtime cases, host C builds under strict flags,
ASan, and UBSan.

| Case | Source shape | Observed on `541ba27` | Expected |
|------|--------------|-----------------------|----------|
| aliased `text` | `import { text as t } from std.fmt; t(7)` | accepted; emits `t(7)`, undeclared | `cl_text_i32(7)` |
| aliased `text(null)` | `t(null)` | accepted; emits `t(NULL)` | E10500 |
| qualified `text` | `std.fmt.text(7)` | accepted; emits `std.fmt.text(7)`, undeclared | `cl_text_i32(7)` |
| user `std.fmt.text` | a holder module `std.fmt` with `export func text` | accepted; calls the user function | unchanged (user declaration wins) |
| generic condition | `show<i32>(c, x)` with `if (c ? null : x)` | E10012 at the null | E10218 only |
| generic `while`/`assert` | `while (c ? null : x)` / `assert(c ? null : x)`, `T = i32` | E10012 at the null | E10218 only |
| both-null inferred | `let z = c ? null : null;` | accepted as `void *`; settlement named `Deferred` | accepted as the null value |
| both-null concrete | `let z: i32 = c ? null : null;` | E10012 at each arm | unchanged |
| both-null placeholder | `$"{c ? null : null}"` | E10500 | unchanged |
| null vs param | `f<i32>` with `null == x` | accepted; emits `(NULL == x)` | E10012 at the null |
| null vs param (`x == null`) | `f<i32>` with `x == null` | accepted; emits `(x == NULL)` | E10012 at the null |
| null vs nullable param | `f<String>` with `x == null` | accepted; valid C | accepted |
| null cstring panic | `let p: cstring = null; panic(p)` | segfault; UBSan null argument to `strlen` | clean panic |
| step array call | `for (...; a = mk3(), ...)` | accepted; compact header; C rejects the missing destination | flat store |
| step array member | `for (...; a = s.vals, ...)` | accepted; compact header; C rejects the array assignment | flat store |

## 2. Root Cause per Residual

### 2.1 Aliased and qualified `text`

`compiler/src/sema/call.rs:276` recognizes the intrinsic only when the written
callee name is exactly `text`. An aliased import is typed under its local name
(`compiler/src/sema/call.rs:1546`, `text_name_is_shadowed`), so the branch is
skipped and the ordinary opaque call path accepts it. `std.fmt.text` is an
`ExprKind::Qualified` path: the embedded `std.fmt` does not declare `text`, so
`qualified_decl_kind` returns `None`, the dot/module path checks fail, and the
call is accepted as opaque (`compiler/src/sema/call.rs:150-233`). The lowered
callee keeps the written spelling (`compiler/src/own/call.rs:336`), so codegen
emits the undeclared symbol instead of the `cl_text_*` selector
(`compiler/src/codegen/expr.rs:1280`).

The ownership stage has a parallel recognition in
`compiler/src/own/expr.rs:1613` (`is_builtin_text_call`), used for the
nullable-use argument check and the deferred placeholder duty. It also matches
only the bare name.

### 2.2 Generic condition context

`compiler/src/sema/ops.rs:465` (`check_conditional`) receives the one-shot
`condition_root` flag and skips the null-arm settlement for a non-bool
conditional in a condition, leaving E10218 to `require_bool`. The site records
`ConditionalArms` (`compiler/src/sema/mod.rs:375`) without the flag, and
`check_module` copies it to `OpaqueConditional` (`compiler/src/sema/mod.rs:332`)
without it. The instance audit
(`compiler/src/own/instance_check.rs:2661`) therefore runs the null-arm
settlement first and reports E10012 for a substituted scalar, where the
concrete twin reports E10218 only.

### 2.3 Both-null conditional

`compiler/src/types/mod.rs:1112` (`settle_null_arm`) joins the two null arms
through `conditional_result`; the result is the unresolved null shape, so it
returns `Deferred`. A no-destination both-null site stays non-opaque: `ConditionalArms::opaque`
needs an open destination or a parameter (`compiler/src/sema/ops.rs:539-543`),
so `check_module` never turns this shape into an `OpaqueConditional`. The
`Deferred` return therefore has no consumer and creates no instance duty; the
value is accepted and every concrete use already reports through the ordinary
null rules. The defect is the undocumented, misleading classification, not the
behavior.

### 2.4 Null compared with an open parameter

`compiler/src/sema/ops.rs:121` (`check_binary`) keeps `==`/`!=` legal when the
null side is the literal and the other side is still a parameter
(`null_capable` at `:150`). Nothing records the site for the instance, so
`T = i32` accepts and the emitter writes `NULL == x`. The binding form
(`let z = null; z == x`) is already rejected because the null side is not the
literal.

### 2.5 Null cstring panic

`compiler/src/codegen/function.rs:1369` (`emit_panic`) emits
`cl_cstring_len((const char *)(local))` for a `cstring` message.
`compiler/runtime/shim_host.c:48`, `shim_hosted_amiga.c:552`, and
`shim_freestanding.c:230` call `strlen` unconditionally, so a null cstring
crashes before `cl_panic`. A language-level `len(cstring)` already reports
E10010 for a possibly-null operand, so the panic path is the observable defect.

### 2.6 For-step fixed array

`compiler/src/own/loops.rs:1005` (`for_step_is_structured`) accepts an
`Assign` whose target does not own, whose value needs no guards/stores, and is
not an `ArrayLit`. A fixed-array target from a call (`a = mk3()`) needs the
hidden destination argument, and one from a member (`a = s.vals`) has no C
array assignment. Both keep the compact header and fail the strict C build.

## 3. Existing Contracts Reused

| Contract | Location | Use in this plan |
|----------|----------|------------------|
| Explicit `text` judgments | `compiler/src/sema/call.rs:276` | Alias/qualified spelling |
| Intrinsic ownership/nulling | `compiler/src/own/expr.rs:1613` | Alias/qualified recognition |
| `$text` sentinel | `compiler/src/codegen/expr.rs:1170` | Safe lowered spelling |
| Deferred conditional duty | `compiler/src/own/instance_check.rs:2661` | Condition flag |
| Null-arm settlement | `compiler/src/types/mod.rs:1112` | Both-null decision |
| Binary null rule | `compiler/src/sema/ops.rs:121` | Deferred null comparison |
| Deferred duty plumbing | `compiler/src/sema/mod.rs:478-486` | New comparison duty |
| For-step shape | `compiler/src/own/loops.rs:1005` | Fixed-array target |
| Panic/runtime shims | `compiler/runtime/shim_*.c` | Null-safe cstring length |

## 4. Constraints

- `spec/` is the immutable oracle. No chapter or golden changes.
- The emitted C must stay valid C99 on host and m68k; the three shims stay in
  agreement on the cstring length contract.
- The freestanding profile keeps its own byte-copying panic implementation.
- The #28/#24 emission and diagnostic contracts stay green.
- The both-null decision matches the existing bare-null binding rule; it does
  not make `let z = null;` illegal.

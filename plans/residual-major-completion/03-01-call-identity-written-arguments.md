# Call Identity and Written-Argument Checks: Residual Major Completion

> **Document**: 03-01-call-identity-written-arguments.md
> **Parent**: [Index](00-index.md)

## Overview

Three red rows share the call-name surface: a dot call's written argument escapes the conversion
check, a declaration/import name collision is not reported, and a qualified call is typed as the
module's function but emitted as the caller's. The fourth row (`DEF-361`) is the direct-name
shadow family, verified green at the head and pinned as a control. The fixes are narrowly scoped:
the dot-call argument check, the import binding check, and the callee-identity threading.

## Design

### 1. The dot-call written argument (`DEF-360`)

`pick<T>(take v: T, take w: T)` with `b: Box` and `b.pick(7)` is accepted: the receiver fixes
`T = Box`, so `7` can never match `w: Box`, but the dot-call route checks the written argument
against the declared `T` (a type parameter accepts anything). The direct `pick<Box>(b, 7)` and the
inferred `pick(b, 7)` both report one `E10200` at the `7`. The frozen rule is Ch 04 §8
(`spec/04-expressions.md:189-192`): a dot-call candidate "must accept the receiver as its first
parameter and match the argument list under the usual implicit widenings (section 5, Ch 02
section 9); no other conversions apply"; Ch 04 §14 (`:327`) makes the illegal implicit conversion
`E10200`.

The fix runs the same written-argument conversion check as a plain call **after** the receiver
fixes the first parameter: either infer the instance's type arguments at the dot-call site and
substitute the remaining parameter types, or record a deferred conversion duty for an argument
whose parameter still mentions a type parameter and judge it in the per-instance audit (the
`DEF-345`/`DEF-351` machinery). The phase prefers the deferred duty when the argument's type is
concrete and the parameter is opaque at the declaration (the same rule the emitter's inference
uses); the check reports at the call's written argument, once, and never at the generic
declaration. Both the `take`-receiver and `borrow`-receiver spellings are covered; the receiver's
own deferred conversion record stays unchanged. The `W10001` warning on a `borrow` receiver of a
duplicable type is orthogonal and stays.

### 2. The declaration/import collision (`DEF-363`)

`import { b as g } from holder;` beside a same-module `let g: i32` is accepted; `load/mod.rs`'s
`resolve_imports` (:386) reports `E11103` only when an **earlier import** bound the same local
name (:413-423). Ch 10 §4 MOD-12 (`spec/10-modules.md:111`) requires: "Binding one name twice in
a module is an error; rename with `as` | E11103". The fix extends the duplicate test to the
module's declarations (`declarations_of`, every declaration kind — `func`, `struct`, `enum`,
`handle`, `let`, `const`, `comptime`) while preserving the existing prior-import check. One
`E11103` is reported per colliding import, at the import item's name span (`ImportItem::name_span`,
the existing duplicate-import convention; the message names the local binding), in either
declaration order; the module declaration is not an `E11101` duplicate (it is bound once by the
declaration and once by the import). The frozen message is the registry's "duplicate import of
'{name}'". The renamed-import and unrelated-declaration shapes are the controls.

### 3. The qualified call's identity (`DEF-390`)

`probe.m390.lib.f()` is typed with module `lib`'s `f` but emitted as the caller's `f`: the
lowerer reduces a qualified callee to its last segment (`callee_name`,
`own/call_check.rs:291`) and `resolve_function` (`codegen.rs:701`) tries the caller's own module
first. Ch 10 §5 MOD-13 (`:112`): "Qualified access `A.B.f()` is always available, with or without
an import"; Ch 04 §8 (`:178`) spells `A.B.f` as a "module-qualified name or function". The fix
threads the resolved `(module, name)` identity through the lowered call so the emitter calls the
qualified module's function; the module path wins over a same-named current-module function, and
a lexical local/parameter named like the head still shadows the module path (batch-L AR-25). The
checker's module-signature typing is already correct and stays; a signature mismatch keeps its
`E10200`, and the matching qualified call builds and runs the module's function. The fix must
also keep the import/alias and func-global identity paths of batch L intact (their suites
re-run).

### 4. The control (`DEF-361`)

No red reproduction exists at the head. The direct analog matrix — a local or parameter named
like a direct imported binding used as a dot receiver, in a body, an expression lambda, a block
lambda, a module initializer, and a generic body — compiles strictly and runs 0
(`build/batchM-plan/matrix/d_*.clo`, `probes/m361_local.clo`, `probes/m361_param.clo`). The class
is closed by the batch-L phase-2 corrections: function bodies and both lambda body forms walk
with an empty scope map (`own/generics.rs:34-52`, `:400-410`), so a body/parameter name cannot
resolve to an import. Frozen rule: Ch 10 §5 MOD-15/16 (`:169-170`, the innermost scope wins; a
local may shadow a module name). The phase pins the shapes as controls and plans no code change
(AR-31).

## Integration Points

- `compiler/src/sema/call.rs:465` (`check_dot_call`), `:671` (`check_args_impl`),
  `compiler/src/sema/expr.rs:491`/`:560` (the deferred conversion record and the conversion
  check), `compiler/src/own/instance_check.rs` (the per-instance audit).
- `compiler/src/load/mod.rs:386` (`resolve_imports`), the declaration collector
  (`collect_declarations`), the `E11103` report.
- `compiler/src/own/call.rs:279` (`lower_call`), the qualified callee arm (:255),
  `compiler/src/own/call_check.rs:291` (`callee_name`), `compiler/src/codegen.rs:701`
  (`resolve_function`), the `Callee`/`FuncInfo` identity.
- Controls: `probes/m360_control.clo`, `m361_*`, `m363_control.clo`, `m390_control.clo`; the
  batch-L aliased/qualified identity cases in `callable_release_integrity_*` and
  `checker_conformance_*`.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A dot-call written argument that cannot convert to the resolved parameter | One `E10200` at the argument (the direct twin's rule); no TU | AR-8 |
| A dot call whose parameter stays unresolved (no inference) | No report here; the emitter's inference/audit owns the instance (the existing unknown rule) | AR-8 |
| A declaration/import collision | One `E11103` at the import item, one per colliding import, either order | AR-9 |
| A qualified call whose module is not loaded | The existing `E10219`/`E10212` name error (unchanged) | — |
| A lexical local shadowing the qualified head | The value-headed call; the module path does not apply (batch-L AR-25) | AR-24 |
| A mismatched qualified argument | One `E10200` from the checker (already correct); the emission is not reached | AR-24 |

## Testing Requirements

- **Spec cases** (`checker_conformance_spec_test.rs`; `callable_release_integrity_spec_test.rs`
  for `DEF-390`):
  - `should_report_a_dot_call_written_argument_at_its_resolved_parameter` (`DEF-360`): the
    `take` and `borrow` receivers report one `E10200` at the argument; the matching dot calls
    and the direct twins are controls.
  - `should_report_a_declaration_import_collision` (`DEF-363`): the import-first, declaration-
    first, and direct spellings report one `E11103`; the renamed control runs.
  - `should_emit_the_qualified_function_identity` (`DEF-390`): the qualified value call, the
    signature shape, and the mismatch control.
  - `should_keep_a_direct_name_shadow_at_a_dot_receiver` (`DEF-361` control): the local,
    parameter, lambda, and module-own shapes run 0.
- **Impl pins** (`checker_conformance_impl_test.rs`; `callable_release_integrity_impl_test.rs`):
  the report code/count/span per spelling, the emitted callee symbol per qualified shape, the
  absence of a dead instance for the shadow controls, and a re-emission determinism pin.
- **Verification**: `make check`; the focused suites; the strict builds of the accepted shapes;
  the batch-L `k18`-equivalent identity matrix and the origin suites stay green.
- **Risks/lenses**: correctness + semantics + emission-integrity (AR-27). The correctness lens
  checks the blast radius (every dot-call route, every import binding, every qualified callee
  path); the emission lens checks the symbol set and the byte-identical controls.

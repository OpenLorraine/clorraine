# Findings

## Phase 1 — Generic dot-call inference and bounds

**Rows**: `DEF-397`.
**Status**: Done at `6f6ea4a`.

The dot-call route now runs the direct call's judgments:

- an uninferable parameter reports `E10301` once;
- a recursive inferred instantiation reports `E10305` once;
- a callable-bound violation reports `E10303` once;
- an ephemeral closure argument reports `E10306` once.

An explicit type-argument list fixes the instance itself. It no longer infers
the missing parameters from the receiver or the written arguments, so a short
list reports only its count (`E10300`), exactly as the direct twin does. The
accepted bounds control uses inference; the parser does not spell a function
type inside a dot-call explicit type-argument list.

Cases:

- `should_judge_a_generic_dot_calls_inference_and_bounds`;
- `should_judge_a_generic_dot_calls_recursion_explicit_list_and_ephemeral_argument`;
- `should_report_a_generic_dot_calls_inference_and_bound` (implementation pin).

## Phase 2 — Qualified function values and intrinsics

**Rows**: `DEF-398`, `DEF-399`.
**Status**: Done at `c14e81b`, with the review corrections below.

- A module-qualified function value names the holder's emitted symbol. A
  lexical binding of the head still shadows the path.
- `std.mem.len`, `std.mem.rawSlice`, and `std.mem.rawStr` are visible through
  qualified access using the declaration kind from the loaded standard module.
  The temporary export-table additions were reverted: an `import { rawSlice }`
  stays `E11102`, and an alias cannot bypass the intrinsic judgment.
- A cross-module `ffi` call resolves the target module's written signature and
  uses the ffi argument and result rules.
- The standard-module intrinsic is call-only: a value position reports
  `E10219` with the call-it-directly help. Qualified `ffi` values report the
  same code.
- Qualified intrinsic arguments keep the bare spelling's judgments:
  `E10010` for a nullable `len`/`rawSlice`/`rawStr` source, `E10038` for a
  view-builder source borrowed from a non-`const` global, `E10308` for a wrong
  argument count, and `E10300` for a wrong type-argument count.
- A free `len` folds a pure fixed-array place before lowering. A call operand
  is evaluated through the guard materialization, so a conditional,
  short-circuit, or loop-condition call runs only on the reached path and an
  owned result is released there. A module initializer reaches the same free
  spelling.
- A fixed-array result returned by a discarded call now writes through a
  statement-local hidden destination, so the C call is valid.
- An `ffi` declaration may not take a compiler intrinsic's reserved name; the
  loader reports `E20006`, exactly as it does for a user function.

Cases:

- `should_emit_a_qualified_function_value`;
- `should_resolve_a_qualified_intrinsic_and_ffi_declaration`;
- `should_keep_a_module_path_binding_dot_call_and_free_intrinsic_apart`;
- `should_evaluate_a_folded_len_operand_on_the_reached_path`;
- `should_judge_a_qualified_intrinsics_arguments_and_value_position`;
- `should_reject_an_ffi_declaration_of_a_reserved_intrinsic`;
- `should_emit_qualified_function_intrinsic_and_ffi_identities`;
- `should_emit_module_path_values_and_free_intrinsics_by_identity`;
- `should_keep_a_folded_len_operand_inside_its_conditional_arm`.

## Phase 3 — Module-path precedence

**Rows**: `DEF-400`.
**Status**: Done at `c14e81b`, with the review corrections below.

A qualified name is classified by its longest leading module path:

- when the path ends at the final segment, the name is a module declaration (a
  function, intrinsic, `ffi` function, or binding), not a dot call;
- when the path ends earlier, the next segment is a module binding and the
  remaining segments are its members, so the call is a dot call on that
  binding;
- a lexical binding of the head always shadows the path;
- an imported or module-level value head never displaces the module path.

The same rule applies in the type checker, the own call checker, the place
lowering, and the module-initializer emitter. A single-segment module path
counts as a module path. A module-qualified function value wins over an
imported value head in both a function body and a module initializer. A
module-qualified func-typed global still calls through its binding.

A bare global dot receiver keeps the global-borrow rule (`E10038`), exactly as
the free twin does. The module-qualified receiver shape accepted by the
existing `should_reach_a_module_path_bindings_member_call` oracle stays
accepted; the oracle-versus-BR-9 conflict is filed as `DEF-533`.

Cases:

- `should_prefer_a_module_path_over_an_imported_value_head`;
- `should_keep_a_module_path_binding_dot_call_and_free_intrinsic_apart`;
- `should_reject_a_bare_global_dot_call_receiver`;
- `should_emit_a_module_function_value_over_an_imported_value_head`.

## Review rounds

| Round | Lenses | Result |
|-------|--------|--------|
| 1 (`5830521..c14e81b`) | correctness, semantics, emission-integrity, memory-safety | 4 critical/major intrinsic and classification gaps found; all closed in `814d04d`. |
| 2 (`c14e81b..814d04d`) | the same four | The original findings closed. New: `SR-005` (bare global receiver exemption), `SR-006` (spurious unused warning), `EI-010` (row index), `EI-011` (initializer function value), `EI-012`, `EI-013` (call operand dropped); corrected in `b6b3855`/`f40da32`. |
| 3 (`b6b3855..f40da32`) | correctness, emission-integrity, memory-safety | `EI-014` (unconditional operand in a conditional/short-circuit) corrected by the guard materialization; `NEW-4` (reserved `ffi` name poisoning the intrinsic identity) corrected by the loader `E20006`. The non-call effectful operand shapes and the module-initializer own-call gap are filed as `DEF-531`/`DEF-532`; the oracle conflict is `DEF-533`. The final code head `0fac18e` keeps the non-call shapes at the structural fold. |

All four lenses confirmed the final call-operand, receiver, initializer, and
intrinsic-identity behavior. Memory probes were ASan/LSan clean, including the
conditional, short-circuit, loop, and discarded forms of an owned `String[2]`
call result.

## New rows filed

| Row | Severity | Finding |
|-----|----------|---------|
| `DEF-531` | major, pre-existing | A non-call effectful fixed-array `len` operand (a conditional, an array literal, or a run-time index) folds to its extent without evaluating the operand. The bare and dot call spellings also drop a fixed-array call result. The free/qualified **call** spelling is evaluated in this batch. |
| `DEF-532` | major, pre-existing | Module initializers run sema but not the own call check, so a bare global receiver or a written global borrow is accepted in an initializer while the function-body twin reports `E10038`. |
| `DEF-533` | major, Deferred | `spec/16-borrow-rules.md:195` BR-9 forbids borrowing a global, but the existing `should_reach_a_module_path_bindings_member_call` oracle accepts a module-qualified global receiver passed to a `borrow` parameter. Batch S preserves the oracle shape and rejects the bare shape. A user ruling is needed to retarget the oracle or amend BR-9. |

## Rulings

| Rule | Ruling | Rationale |
|------|--------|-----------|
| Qualified classification | The longest leading module path decides. A path ending at the final segment is a module declaration; an earlier end is a dot call on the binding. A lexical head shadows; an imported or module-level value head does not. | The module path is the written identity; an imported value is not a lexical local. A module binding followed by members is a dot call on that binding. |
| Qualified intrinsic | Call-only; a value position reports `E10219`. Arguments mirror the bare spelling's null, loan, arity, and type-argument judgments. | An intrinsic has no value symbol; the call-site judgments must not depend on the spelling. |
| Reserved `ffi` names | An `ffi` declaration named `len`, `slice`, `rawSlice`, or `rawStr` reports `E20006`. | The declaration would emit the written C symbol, collide with the standard-library shim, and make the intrinsic identity ambiguous. |
| Explicit generic dot type arguments | An explicit list fixes the instance; missing parameters are not inferred from the receiver or arguments. A short list reports only `E10300`; inferred recursion keeps `E10305`. | Matches the direct call's explicit-type-argument route. |
| Fixed-array `len` operand | A pure place folds; a call operand materializes through the guard so its evaluation follows the reached path. Non-call effectful shapes are filed as `DEF-531`. | The extent is structural, but an effectful operand must not run on a path that does not reach the expression. |
| Global receiver | A bare global dot receiver keeps `E10038`; the module-qualified receiver shape required by the existing oracle stays accepted. The conflict is `DEF-533`. | Contain the oracle exception to the exact accepted shape; do not widen it to every global. |

## Close-out

`make check` and `make verify` pass at `0fac18e` (verify: all legs passed).
No `spec/` file, diagnostic registry entry, or generated C was edited. The
roadmap records `DEF-397`, `DEF-398`, `DEF-399`, and `DEF-400` as Done, and
files `DEF-531`, `DEF-532`, and `DEF-533` as the follow-up rows.

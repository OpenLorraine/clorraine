# Nested Loop Step: Element Loop Integrity

> **Document**: 03-02-nested-loop-step.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-17: the emitter must print a `for` loop's own step block
in the C header even when the loop is nested. The ownership stage already knows the
step when it builds the loop; the fix carries it in the existing IR shape and
removes the emitter's search heuristic.

> **Decision per AR #7 / AR #8:** store the step block on `BranchShape::Loop` as
> `step: Option<BlockId>`; `None` for `while`/`do-while`.

## Architecture

### Current Architecture

- `BranchShape::Loop { kind, body, exit }` (`compiler/src/ir/control.rs:26-34`)
  records the printed form of a structured loop.
- `check_for_c` (`own/loops.rs:161`) creates `step_block` (`:178`) and, when the
  step can print in a C header (`for_step_is_structured`, `:557`), records the
  shape at `:268`.
- `check_for_in` (`own/loops.rs:378`) creates `index_temp`/`address_temp` and
  `step_block` (`:421`) and records the shape at `:423` when the head is empty.
- `emit_loop` (`codegen/control.rs:314`) matches the shape; for `LoopKind::For` it
  calls `for_step_block(function, body, head)` (`:407`), which returns the first
  block in `closure_from(function, body)` whose terminator is `Goto(head)`.
- For a nested loop the search returns the loop's own pre-header, because the
  enclosing loop's back edge re-enters it (02-current-state.md → Gap 2).

### Proposed Changes

```rust
/// A loop whose body is `body` and whose exit is `exit`.
Loop {
    /// Which loop form to print.
    kind: LoopKind,
    /// The first block of the loop body.
    body: BlockId,
    /// The block reached when the loop ends.
    exit: BlockId,
    /// The step block of a `for` loop; `None` for `while`/`do-while`.
    step: Option<BlockId>,
},
```

- `loop_shape` (`own/loops.rs:537`, used by `while` and `do-while`) sets
  `step: None`.
- `check_for_c` (`own/loops.rs:268`) sets `step: Some(step_block)`.
- `check_for_in` (`own/loops.rs:423`) sets `step: Some(step_block)`.
- `emit_loop` (`codegen/control.rs:314`) takes `step` from the shape. For
  `LoopKind::For` it uses the stored block directly; `continue_target` becomes
  `step.unwrap_or(branch)`. The `for_step_block` function and its
  `closure_from`-based search are deleted.
- The `Plain` path is untouched: an unstructured loop still prints its step block
  as an ordinary labelled block (the emitter's post-pass,
  `codegen/control.rs:122-126`).

No block is created or renumbered, so block ids and the emitted C of non-nested
programs are unchanged (R5, R6).

## Implementation Details

### Step Text and Shape Selection (unchanged)

- `for_step_text` (`codegen/control.rs:423`) still renders the step block's
  `Assign` statements as a comma expression.
- For a three-clause `for`, the structured shape is still chosen only when
  `for_step_is_structured(step_block)` holds; otherwise the flat `Plain` form is
  used and the stored step is not consulted.
- For `for ... in`, the step block always holds the single index increment
  (`emit_index_step`, `own/loops.rs:507`).

### Emitted C Before and After

```c
/* before: nested element loop — the pre-header is the step and the real step is dead */
for (; (__cl_tmp0 < 2); __cl_tmp0 = (__cl_tmp0 + 1u)) {
    ...
    for (; (__cl_tmp2 < 3); __cl_tmp1 = &(xs[__cl_tmp0]), __cl_tmp2 = 0u) {
        ... goto __cl_b7;
    }
}
__cl_b7:
    __cl_tmp2 = (__cl_tmp2 + 1u);
    for (; (__cl_tmp2 < 3); __cl_tmp1 = &(xs[__cl_tmp0]), __cl_tmp2 = 0u) { }
    return 0;

/* after: the inner header carries its own step */
for (; (__cl_tmp0 < 2); __cl_tmp0 = (__cl_tmp0 + 1u)) {
    __cl_tmp1 = &(xs[__cl_tmp0]);
    __cl_tmp2 = 0u;
    for (; (__cl_tmp2 < 3); __cl_tmp2 = (__cl_tmp2 + 1u)) {
        ...
    }
}
return s;
```

### Integration Points

- `codegen/control.rs:98` (`prepare_regions`) matches `BranchShape::Loop { kind, body, .. }` — the `..` absorbs the new field; no change.
- `codegen/control.rs:211` matches `BranchShape::Loop { .. }` — no change.
- `codegen/control.rs:322` (`emit_loop`) destructures the new field.
- `own/loops.rs` constructors at `:268`, `:423`, and `:547` supply the field.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Unstructured loop (`Plain` shape) | Unchanged flat label/`goto` path; the stored step is unused | AR #8 |
| Unreachable step (body always terminates) | The stored step is still printed in the header only when the C header can carry it; C semantics are unchanged because the step never runs | AR #8 |
| A `for` whose step cannot print in a C header | `for_step_is_structured` still forces `Plain`; unchanged | AR #8 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases ST-14 – ST-21 in `07-testing-strategy.md`.
- Implementation cases in `element_loop_integrity_impl_test.rs`: the emitted inner
  header carries the increment, no dead step label or duplicate loop remains, and
  two emissions are byte-identical.
- Existing suites (`codegen_run_spec_test.rs`, `corpus_spec_test.rs`, the NLL and
  ownership suites) guard non-nested behavior; `make verify` covers every profile.

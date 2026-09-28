# Requirements: Codegen Ownership Safety

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

This plan removes two critical code-generation defects that affect accepted
programs. Both were confirmed with AddressSanitizer on the host against the
emitted C; the container runs the behavioral tests, and the host probes remain
manual evidence (AR #5).

The specification is the oracle for both fixes and needs no change:

- A `borrow` element view writes through to the owner
  (`spec/16-borrow-rules.md` section 2, `spec/08-arrays-strings.md` section 5
  LOOP-2).
- Assignment drops the live old value of the target first, then stores the new
  value (`spec/03-ownership.md` O3).
- A local may shadow an outer local or a module name, and the innermost
  enclosing scope wins (`spec/10-modules.md` section 5, MOD-15/MOD-16).

## Functional Requirements

### Must Have

- [ ] R1 — A `for (borrow e in xs)` element is a true alias of the element: a
  write through the binding reaches the iterable immediately, and an assignment
  of an owned value follows O3 on the element (drop the old value, then store),
  exactly once.
- [ ] R2 — Every exit from a borrow-element loop observes the write:
  fallthrough, `break`, `continue`, `return`, and deferred cleanup paths, with
  no write-back step and no dangling array element.
- [ ] R3 — Shadowing works for every MOD-16 form: local over local (nested and
  sibling scopes), local over parameter, `const` locals, defer-machinery
  locals, generic instantiations, and lifted lambda bodies. The innermost
  binding wins; the outer binding keeps its value and its storage.
- [ ] R4 — Emitted C stays warning-free under the project's `-Wall -Wextra
  -Werror` conventions on every profile.
- [ ] R5 — IR names and emitted C stay deterministic: speculative loop passes,
  generic instantiation, and repeated compilation produce identical output.

### Should Have

- [ ] R6 — The now-dead copy-in/write-back machinery is removed (no dead code):
  `ForInWrite`, `set_for_in_write`, `store_for_in_element`,
  `run_for_in_write`, and the `break`/`continue` hooks that ran it.
- [ ] R7 — A base-overridden binding (a lambda capture or a loop alias) never
  declares a spurious C local.

### Won't Have (Out of Scope)

- The defer-temporary runtime leak (minor, filed outside this plan).
- FM-008 (the implementation-test oracle does not run ASan).
- Multi-extent element loops binding the base element (filed as `DEF-15`).
- A binding-id-keyed IR redesign.
- Any change to `spec/` (the specification already defines the required
  behavior).

## Technical Requirements

### Performance

No run-time target is required. The element-alias change removes a write-back
per iteration and adds one address assignment per iteration, so emitted-code
cost is neutral.

### Compatibility

All three profiles (`host`, `hosted-amiga`, `freestanding`) and both C
toolchains (`gcc`, `vbcc`) must keep working; `make check` and `make verify`
cover the existing corpus, and the implementation suite adds a build-only
freestanding compile of a borrow-element program because no corpus program
uses the construct.

### Security

Not applicable: the change is compiler-internal with no user input, no secrets,
and no persistent data or network surface (AR #15).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Defects fixed | both criticals / also the leak / also FM-008 | DEF-13 + DEF-14 | Smallest correct scope; the other findings are filed separately | AR #1 |
| Element alias design | `raw` temp alias / patch the cache / C pointer local | `raw` temp alias | Removes the duplicate-ownership cache instead of patching it; reuses `Binding.base` and compiler temps | AR #2 |
| Local identity design | unique IR spelling / binding-id IR | unique IR spelling | Localized to the lowering; codegen maps keep working | AR #3 |
| Replacement semantics | drop-then-store / overwrite | drop-then-store | `spec/16` section 2 plus `spec/03` O3 | AR #6 |
| Exit behavior | alias is the element / write-back on every exit | alias is the element | One source of truth; `return` becomes correct with no extra machinery | AR #7 |
| Non-borrow loops | unchanged / alias too | unchanged | LOOP-1 keeps the per-iteration copy | AR #8 |
| Shadowing coverage | all MOD-16 forms / reproduced case only | all MOD-16 forms | The specification allows every form | AR #9 |
| Multi-extent defect | exclude + file / exclude only / include | exclude and file `DEF-15` | Different root cause | AR #10 |
| Internal names | proposed / different | `PlaceBase::DerefTemp`, `name#<index>` | Deterministic and readable | AR #11 |
| Test placement | new suites / extend existing | new per-plan suites | Matches the per-feature convention | AR #12 |

> **Traceability:** Every scope decision must reference the Ambiguity Register
> entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] The DEF-13 reproducers (plain loop, `return` inside the loop) run
   clean on the host under AddressSanitizer, and the equivalent spec cases
   (ST-1 – ST-7) pass.
2. [ ] The DEF-14 reproducers (MOD-16 example, nested shadowing, defer-local
   shadowing) build with `-Wall -Wextra -Werror` and run correctly; the spec
   cases (ST-8 – ST-15) pass.
3. [ ] No dead copy-in/write-back code remains (R6).
4. [ ] Emitted C is deterministic (R5).
5. [ ] `make check` is green after every GREEN task; RED tasks verify by
   failing for the defect reason; `make verify` reports all legs passed
   before completion.
6. [ ] Documentation is updated: plan documents complete, `AGENTS.md` test
   naming map gains the new suites, and the roadmap rows for `DEF-13` and
   `DEF-14` track the plan.

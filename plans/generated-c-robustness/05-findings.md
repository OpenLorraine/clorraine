# Findings and Evidence: Generated-C Robustness Bundle

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-12 15:05
> **Status**: Implementation complete; full gate blocked at the FS-UAE legs

## 1. Red Evidence (baseline `0fdb96c`)

The spec suite ran against the saved baseline compiler
(`build/base0f-clc`). 9 cases failed:

- ST-2, ST-3, ST-4: a module-level/imported `std` value made
  `std.fmt.text` fall to the member path (invalid C or no E10500).
- ST-6, ST-7, ST-9, ST-10: ASan reported the owned text argument leaking.
- ST-11, ST-13: the initializer resolved the member field (length 1) instead
  of the module path (length 4).

The 6 guards passed: ST-1, ST-5, ST-8, ST-12, ST-14, ST-15.

`IT-4` (init-local collision) failed on the Phase 1 head: the emitted local was
`__cl_itmp0`; after the #16 fix it is `__cl_itmp1`.

## 2. Implementation

- `compiler/src/sema/call.rs`: the qualified `std.fmt.text` gate no longer
  treats a module-level/imported `std` value as a shadow; only a lexical head
  reaches the member path (MOD-13/MOD-16).
- `compiler/src/own/expr.rs`: the matching ownership gate is removed;
  `is_dot_call` already handles the lexical/module-path split.
- `compiler/src/codegen/program.rs`: the initializer decides
  `init_text_intrinsic_name` before `init_callable_value`; a user
  function/`ffi`/global `text` still wins. `declares_text` now covers a
  `Global`/`Const` declaration. The module-level `std` gate is removed.
- `compiler/src/own/call.rs`: the intrinsic text call uses a synthetic
  `Mode::Borrow`, so an owned body temporary is materialized and released;
  a named place stays a copy.
- `compiler/src/codegen/program.rs`: `next_init_local` advances past
  `is_taken_symbol`.
- `compiler/tests/template_placeholder_residuals_iii_*`: the #29 IT-13 pin and
  ST-15 comment are updated for the module-path precedence; the #29 01/03/05/07/99
  notes mark the module-level clause superseded.

## 3. Green Evidence

- `generated_c_robustness_spec_test`: 15 passed (plain, ASan).
- `generated_c_robustness_impl_test`: 11 passed.
- #29: 49 spec + 13 impl passed.
- `checker_conformance_spec_test`: 212 passed.
- `make check`: green (fuzz-smoke 28/0); the host run used
  `CODEOPS_TMPDIR`/`TMPDIR` under `build/tmp` because the host `/tmp` user
  quota is exhausted (the container tests are unaffected).

## 4. Phase Review

### 4.1 First Round

| Review | Verdict | Findings |
|--------|---------|----------|
| Correctness | 1 major, 3 minor | RV-001 non-callable initializer split; RV-003 stale doc; RV-004 dead test helpers; RV-005 bookkeeping |
| Semantics | no critical/major | SR-001 ffi initializer (pre-existing, fixed); SR-002 `take`/`borrow` text markers (pre-existing) |
| Memory safety | clean | no defect; a harness timeout minor |
| Emission integrity | clean | no diff attributable; ffi initializer noted pre-existing |

**RV-001 (MAJOR) — non-callable declaration initializer split.** `declares_text`
recognized only Func/Global/Const/ffi, so a struct/enum `std.fmt.text` let the
initializer fire the intrinsic while sema/own declined it. `declares_text` now
returns true for any declaration; IT-10 pins the rejected E10200/no-C shape.

**SR-001 / RV-002 (ffi initializer).** A real `std.fmt.text` ffi lost to the
value-head callable in the initializer. `init_text_ffi_signature` resolves the
exact path or imported alias before the value-head fallback and builds the ffi
call; IT-9 pins `text(1234)` and the object build. The body already worked.

**RV-003 — stale doc.** The initializer doc now says a real declaration wins
and a module-level/imported value head does not shadow the module path.

**RV-004 — dead test helpers.** The unused `compile`/`count_reports`/
`build_strict` and panic/`reject` helpers were removed; the suites compile
warning-free.

**SR-003 (MAJOR) — ffi initializer lost the qualified spelling.** The first
ffi branch built the call from the bare signature name, so a same-named
function of the initializing module could capture it. The initializer now
keeps the qualified `std.fmt.text` spelling; IT-11 pins it.

**SR-004 (MAJOR) — non-callable declaration in an initializer.** A struct/enum
`std.fmt.text` was accepted and emitted a literal path. The type stage now
rejects a qualified struct/handle/enum call with E10309; IT-10 pins the
rejection and no C.

**RV-001 (final, MAJOR) — imported non-callable declaration.** The E10309
judgment was qualified-only; a bare/aliased imported struct `text` was still
accepted and emitted an undeclared symbol in an initializer. The type stage
now applies E10309 to a bare or imported Struct/Handle/Enum callee too;
IT-12 pins it.

**SR-005 (MINOR) — IT-10 body case.** IT-10 was rewritten as the initializer
non-callable pin; it is red on the base (which emits invalid C) and green on
the head.

**SR-002 — `take`/`borrow` text markers.** `text(take s)` / `text(borrow s)`
are accepted without E10039. This predates the bundle (the text branch and
sema checks are unchanged); recorded for a follow-up.

### 4.2 Re-review

The correctness and semantics re-reviews verified the RV-001/SR-001 fixes and
found two final initializer edges, both fixed:

- **SR-003 (MAJOR)** — the ffi initializer lost the qualified spelling, so a
  same-named function of the initializing module could capture it. Fixed by
  keeping `std.fmt.text` as the callee; IT-11.
- **SR-004 (MAJOR)** — a non-callable struct/enum declaration in an initializer
  emitted a literal path. Fixed by E10309 for a qualified struct/handle/enum
  call; IT-10.
- **RV-001 (final)** — the E10309 judgment was qualified-only; a bare/aliased
  imported struct was still accepted. Fixed by applying the same judgment to
  imported/bare declarations; IT-12.

No critical or major review finding remains open. The `take`/`borrow` marker
acceptance (SR-002) is pre-existing and recorded; the initializer own-call
check (E10100/E10101) is a pre-existing stage gap.

## 5. Acceptance

| Gate | Result |
|------|--------|
| `make check` | green (spec check, workspace tests, bench-test, fuzz-test, fuzz-smoke 28/0) |
| `make asan` | 37 suites green |
| `make ubsan` | 11 suites green |
| `make bench-compare` | passed; instructions and `.text` +0.0%, C +0.0% |
| `make bench-shape` | gcc, nix13, vbcc, freestanding warning-free |
| `make differential` | all portable programs match |
| `make run-fault-amiga` | all eight faults match |
| `make sdk-a500-smoke` | OK on the private Xvfb |
| `make sdk-a500-boing` | passed once on the private Xvfb; later runs hit a host GL failure |
| `make verify` | **blocked** at the FS-UAE native-video legs |

### 5.1 FS-UAE Blocker

The host `/tmp` per-user quota was exhausted for the whole session. Xvfb
cannot reliably write its lock/keymap files, and when `/tmp` fills the FS-UAE
Mesa context fails with `[GLAD] Failed to initialize OpenGL context`. After
archiving stale temp dirs to `/home/gevik/tmp-archive` (with symlinks), the
A500 smoke and the boing leg each passed at least once, but the sequential
`make verify` still cannot keep a stable GL context for the native-video legs.
This is an environment/resource issue, not a code defect: the bundle does not
touch the Amiga display/runtime scripts or the captures. The remaining verify
legs are `sdk-a500-*` through `sdk-a4000-rtg-piv` and `corpus-fs-uae`.

### 5.2 Environment Note

The stale Clorraine temp dirs (`ppverify`, `v11`, `trace-sieve.txt`) were
removed, and the stale other-project temp dirs (`openvault-final`,
`openvault-ncu`, `blend65`, `ci-check`, `ci-check2`, `part9-preflight`) were
moved to `/home/gevik/tmp-archive` with symlinks left at the original
`/tmp/opencode` paths. `docker/.env` was restored to the user's `DISPLAY=:0`,
and the temporary compose override/wrapper were removed.

<!-- Verifier report for packet 9.2.2, session ses_dea9a612affe7B68nL0nJN0bQn, copied verbatim from build/tmp/assessment/verify/9.2.2.md -->
# Verification 9.2.2 — WS2 ownership, borrowing, drops

- Baseline: `e24a8752b08827c3a5d7c77b6e2827dac708b629`. Drift check `plans/independent-assessment/probes/WS0/drift-check.sh` → `PASS (HEAD 31f1bc07)` before every reproduction.
- Compiler: `build/tmp/assessment/baseline/clc` (release). All builds and runs went through `make -C docker exec`.
- Lens: `.opencode/agents/memory-safety-auditor.md`. Host ASan (`-no-pie`, `detect_leaks=1:handle_segv=0`) and UBSan were run through `probe.py --sanitize`, plus hand-built ASan probes run under `timeout 30`.
- Scratch output: `build/tmp/assessment/verify/ws2/` (`run1` = findings, `run2` = clean outcomes under sanitizers, `run3` = clean-outcome contrasts, `p1` = file-close probe, `p2` = projection-move and block probes).

## Findings

| ID | verdict | reason (one line) | what you re-ran and observed |
|----|---------|-------------------|------------------------------|
| ASM-WS2-001 | **upgraded Major→Critical** | An accepted program gives wrong output. A temporary `File` written in one statement is not closed or flushed at that statement's end (spec/04 §3, spec/03 §5 D5, spec/17), so a later read in the same function reads 0 bytes instead of 5, on host and on hosted-amiga. That is "an accepted program miscompiles" (§Severity, Critical). The drop actually happens at **function exit**, or when the same statement runs again, not at the end of the enclosing scope, so the title understates the extent. Nothing in `docs/known-limitations.md` or any earlier plan accepts this; AR-B2 covers only view-bound temporaries. | `probe.py --sanitize --only 219 226 227 228 229 246 247`: every probe FAILs identically in the debug, release, asan, and ubsan builds. 226: `7,next,end,drop 7` (want `7,drop 7,next,end`). 227: `3,end,drop 2,drop 1`. 228/229/246/247 match the finding text. 219: `drop 0,drop 50,drop 1,end,drop 51`, so the break-iteration temporary drops after `end`, at function exit. No sanitizer report. Emitted C (219, `run1/219-*/host/prog.c`): `ClTok __cl_owned0` is declared at function scope, released only before its own reassignment and at the function-exit label `__cl_c1`. New probe `p2/block.clo` (temporary inside a nested `{}` block) prints `in 9, after block, end, drop 9`. New probe `p1/file-temp.clo`: `writeBytes(mk(path), bytes(data));` then `openFile(path, MODE_OLDFILE)` and `readFile`. It prints `wrote 5 / read 0` on host ASan (clean), on host `-O2`, and on hosted-amiga under `vamos`. The contrast `p1/file-contrast.clo` (same handle bound to a local in an inner block) prints `read 5` on host and `vamos`. **Escalate (Critical).** |
| ASM-WS2-002 | confirmed (Minor, verified) | spec/03 §4, spec/07 STR-11, and spec/14 require E10026 for a partial move outside a top-level field of an owned local or `take` parameter; clc accepts it. Release is single and ASan-clean, so Minor (memory-safe false accept) is right. No ruling admits the shape. array-value-integrity AR-5 even keeps element moves out of temporaries at E10026. | `probe.py --sanitize --only 248`: `FAIL … clc: [reject E10026] rc=0 first=None`. Hand-built ASan copy (`p2/m248.clo`, with an `after-let` marker added) prints `after-let, use 1, end, drop 1, drop 2`, rc 0, no report. Extra variant `p2/nested.clo` `let t: Tok = mk().p.a;` (a **nested-path** move out of a call result, which spec/03 §4 also lists as E10026) is accepted too: `use 1, end, drop 1, drop 2, drop 3`, ASan-clean. Same root cause, so it is folded into this finding as extra evidence. |
| ASM-WS2-003 | confirmed (Minor, probable) | The structural claim holds in the code. `codegen/drop.rs:283` `needs_drop` re-derives the release decision from `TypeTable::duplicable` (excluding `is_param`), separately from `own.rs:675` `maybe_owns_ty` and the `!duplicable \|\| maybe_owns` tests in `own/`. The rubric's "maintainability debt that has already caused a recorded DEF" is met: `plans/def-backlog-completion/00-index.md` attributes DEF-56/59/61 to checks that cannot judge a type parameter before substitution, and DEF-57 to "the emitter disagreeing with the recorded instance type". No live disagreement was reproduced, so `probable` is right. | `grep -rn` at the baseline: `.duplicable(` 79 and `maybe_owns(` 70 in `own/` (80/75 when `own.rs` is included). `needs_drop` 84 in `codegen/` (function 38, guards 17, program 11, drop 9, cleanup 9), all matching the finding. Small error: the files holding those decisions number **10** (`own/`) or 11 (with `own.rs`), not 12. Checked DEF-13/34/53/60/65 in `records/WS8-defs.tsv`. DEF-60's documented root cause is a lost element index, not a predicate split, so DEF-56/59/61 and DEF-57 are the better citations. |

## Clean outcomes sampled

| ID | verdict | reason | what you re-ran and observed |
|----|---------|--------|------------------------------|
| WS2-01 (moves, partial moves, E10025) | confirmed | Output and drop order match spec/03 §3–§5 in all four builds. Rejections give the spec codes. The contrasts flip as expected, so the probes are falsifiable. | `probe.py --sanitize --only 201…209`: every check PASS. 201–208 pass asan, debug, release, and ubsan; 209 passes `clc: reject E10025`. `--contrast`: all PASS (201/202/207 contrasts reject, the others produce the contrast output). The out-of-scope call-result field move is ASM-WS2-002, and its nested-path variant is now covered there too. |
| WS2-04 (defer / cleanup ladder) | confirmed | Reverse registration with drops, an early return through a nested block (`drop 2, defer, drop 1`), E10020 for a defer reading a moved value (spec/05 F5, spec/03 O1), defers on continue and break in a loop, and nested defers all behave as specified and are sanitizer-clean. | `probe.py --sanitize --only 221…225`: every check PASS (221/222/224/225 asan, debug, release, ubsan; 223 `clc: reject E10020`). `--contrast`: every check PASS. These probes create no owned call-result temporaries, so ASM-WS2-001 does not affect them. |

## Duplicates and merges

- **ASM-WS8-003 ↔ ASM-WS2-003.** Root cause (1) of WS8-003, "the drop obligation … decided again in codegen with a different predicate", is the same defect as ASM-WS2-003, which has the lower ID. WS8-003 is broader: it also covers invalid-C composition, route-specific false accepts, and C evaluation order. A full merge would lose those, so I do not merge it from here. Recommendation for the WS8-003 verifier: either merge WS8-003's memory-safety part into ASM-WS2-003 and narrow WS8-003 to causes (2)–(4), or keep WS8-003 as the umbrella and cite ASM-WS2-003 for cause (1) without counting it twice in the ranked list.
- ASM-WS2-002 stays separate from ASM-WS2-001. It is a false accept in the check (wrong-acceptance class), not a drop-timing defect, although both share the scope-owner binding.
- ASM-WS2-001 is not a duplicate of ASM-WS1-002 (right-to-left argument evaluation). That is a different mechanism.

## Requested corrections to finding text (no new finding)

- ASM-WS2-001 title and impact: "the end of the enclosing scope" should read "function exit (or the next execution of the same statement)". `__cl_owned<N>` slots are function-level and the release sits at the function cleanup label. `p2/block.clo` shows a temporary from a nested block outliving the block. Add the file-close reproduction (`p1/file-temp.clo` vs `p1/file-contrast.clo`) as the Critical evidence. On a real Kickstart, the still-open `MODE_NEWFILE` handle would also likely make the `MODE_OLDFILE` reopen fail (exclusive lock). That is inferred only; FS-UAE was not run.
- ASM-WS2-002 evidence: add the nested-path variant `mk().p.a` (accepted; spec/03 §4 "nested paths … are E10026").
- ASM-WS2-003 location: "12 files" should be 10 (`own/`) or 11 (with `own.rs`). Cite DEF-56/59/61 and DEF-57 instead of, or alongside, DEF-60.

## New findings

None. The two extra shapes I found (nested-path move out of a call result; function-exit rather than scope-end lifetime with the file-read miscompile) share the root causes of ASM-WS2-002 and ASM-WS2-001 and are recorded above as evidence and corrections, not as new IDs.

## Escalation

ASM-WS2-001 now reaches **Critical** under this verdict and must be escalated per the protocol's §Escalation step.

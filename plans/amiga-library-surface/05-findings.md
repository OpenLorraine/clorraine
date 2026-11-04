# Findings: Amiga Library Surface

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done

## Red phases

### Phase 1 — wrapper

`cargo test --test amiga_exec_spec_test` on 2026-11-03: **0 passed, 4 failed**. The module did
not exist, so the host case reported `E11102` instead of `E11200`, the hosted-amiga emission case
reported `E11102` for every import, and both program cases failed with missing program files.
After the wrapper landed, ST-1 and ST-2 passed; the program cases stayed red until Phase 2.

### Phase 2 — programs

`exec-self-test` first failed on the multi-line import syntax, then on a shim prototype type
mismatch (`const void *` versus the emitted `void *`), then ran with the expected transcript.
Hamurabi first failed on `f64`/`f32` literals and a shared-borrow reader; the original input-driven
policy desynchronized on invalid choices, so the port moved to a deterministic built-in steward.
The final transcript matches.

## Reviews

### Emission integrity

No findings. Base/head corpus diff is shim-only for non-importing programs; host and freestanding
emitted C are byte-identical; all 18 hosted-amiga units and 34 variant probes build under
`-Wall -Wextra -Werror`; the ten new shim symbols agree with the emitted prototypes. The
`cl_release_message` delta is clean.

### Memory safety

The first review found a **major**: `Message`'s drop replied an unsent message. Fixed by splitting
`NewMessage` (created/unsent, frees on drop) from `Message` (fetched, replies on drop), adding
`releaseMsg`, and using `NewMessage` in the acceptance program. A minor drain gap in the
self-test's inbox-empty branch was fixed and re-reviewed clean. Host ASan/LSan probes cover the
success and failure paths; 10 repeated `vamos` runs are byte-identical.

### Correctness and semantics

The first review found two majors: the Hamurabi MIT notice was missing, and the unsent-message drop
queued the message. Both are fixed: the upstream notice is in `THIRD_PARTY_NOTICES.md`, referenced
by the port and README, and the handle split makes the drop safe. The review also found the missing
ST-4/ST-7 coverage, the stale appendix §2 sketch, and a weak land-trade path; ST-4 repeats the
self-test, ST-7 checks export/appendix parity, ST-9 covers all four drop routes, appendix §2 uses
the delivered names, and Hamurabi buys 10 acres in year 1 and sells 10 in year 2. Re-reviews are
clean except for a documentation-only wording issue (RV-010), fixed in the appendix.

## Accepted residuals

| Residual | Reason |
|---|---|
| A fetched `Message` must outlive its reply port; deleting the port first is a contract violation | The compiler has no lifetime carrier for the reply-port pointer; the module documents the rule, and `NewMessage` drops are safe because they only free |
| An owned call result used directly in a comparison can be emitted without a temporary (base-identical, e.g. `File(0)` and `getMsg(...) != null`) | Pre-existing expression-temporaries class (DEF-67); the acceptance programs bind the result first and do not exercise it |
| An imported handle name used in call syntax (`Message(n)`) is emitted as a C function call instead of a cast | Pre-existing imported-handle cast defect; base reproduces with `File(0)`/`Buffer(s)`; no new instance is introduced by the split |

## Verification

`make check` green (92 test-result groups); `./spec/check.sh` OK (27 files, 111 compile-fail
tests); `amiga_exec_spec_test` 6/6; `make verify` exit 0 with `verify: all legs passed`; both
programs and `message-drop` match their transcripts under `vamos`.

# Phase Review: Clorraine build/run driver

> **Document**: 08-review.md
> **Parent**: [Index](00-index.md)
> **Feature diff**: baseline `873155cbfb04aa75ba2cb7e1b6d4b05c53abb25d` → `3dd4ac1` (15 files)
> **Scope mode**: strict
> **Reviewers**: correctness (`RV-8xx`) and security/reliability auditor (`SA-8xx`), in parallel
> **Coverage note**: Phases 1–3 were reviewed together (the feature is small and tightly coupled); no separate per-phase reviews were run.

## Result

Both reviewers confirmed the driver meets its spec tests and the plan's decisions
(default host profile, `run`/`build`/`emit`, vamos default + FS-UAE handoff,
artifact layout, ICE report, exit codes, multi-file input). No critical defect
was found. The audit surfaced one major security finding in the FS-UAE handoff
and several minors. A pre-existing compiler defect (`for (let i = …; i++)`
drops the step) was also observed and is tracked separately.

## Findings

| ID | Severity | Title | Location | Evidence |
|---|---|---|---|---|
| SA-801 | 🟠 MAJOR | The FS-UAE handoff prints a paste-able shell command with a source-derived path unquoted | `compiler/src/clorraine/toolchain.rs:110` | A source named `$(touch PWNED).clo` makes the printed line `PROG=build/clorraine/$(touch PWNED)/hosted-amiga/$(touch PWNED)`; pasting it (as the driver instructs) executes the substitution. A `weird ; name.clo` file splits the line. CWE-78. |
| RV-801 | 🟡 MINOR | `--release --profile hosted-amiga` appends `-Wall -Wextra -Werror`, which the §13 release line omits | `compiler/src/clorraine/toolchain.rs:77` | Deviates from the frozen matrix (`spec/17-codegen.md:359`) that AR-16 grounds the commands in; the plan's `03-02` says "every build appends warnings", so plan and spec conflict. |
| RV-802 / SA-803 | 🟡 MINOR | Driver messages echo user paths and compiler stderr without a trailing newline / without stripping control bytes | `compiler/src/clorraine/mod.rs:127,144,131,164`, `build.rs:36-39` | `BuildFailure::Error` prints with no `\n`; a file with `ESC` injects terminal escapes into `built …` and the ICE lines (CWE-150). |
| RV-803 / SA-804 | 🟡 MINOR | `emit` writes to a predictable `/tmp` path and follows symlinks | `compiler/src/clorraine/mod.rs:180-184` | `/tmp/clorraine-<pid>-0.c` pre-planted as a symlink is written through (CWE-377). |
| RV-804 | 🟡 MINOR | A program exiting with code 2 is indistinguishable from a usage/environment error | `compiler/src/clorraine/mod.rs:29,56,139-167` | AR-11 mandates propagation; the collision is undocumented. |
| RV-805 | 🟡 MINOR | The hosted-amiga release command is not unit-asserted | `compiler/src/clorraine/toolchain.rs:132-139` | ST-DRV-16 asks for debug and release; only debug is asserted. |
| RV-806 | 🟡 MINOR | The SDK/missing-tool caller exit codes (2 and 1) are not covered by a test | `compiler/src/clorraine/mod.rs:53-62` | Only the pure predicates are unit-tested; the exit-2 path is not injectable. |
| RV-807 | 🟡 MINOR | `--capture` still prints `captured <path>` when the write failed | `compiler/src/clorraine/mod.rs:163-165` | `run_program` returns 1, but the "captured" line is printed anyway. |
| SA-802 | 🟡 MINOR | `make run FILE=…` interpolates `$(FILE)` into a `bash -lc` string | `Makefile:35,39` | Self-supplied, but a forwarded untrusted value would inject. |
| SA-805 | 🟡 MINOR | No timeout; `--capture`/`vamos` buffer all stdout in memory | `build.rs:85`, `run.rs:44,53` | A runaway or unbounded-output program can hang or OOM the driver. |
| SA-806 | 🟡 MINOR | `tool_on_path` checks `is_file`, not the execute bit | `environment.rs:52` | A non-executable `cc` passes preflight and fails later with a misleading ICE. |
| SA-807 | 🟡 MINOR | `sdk_present` trusts any regular file (note only) | `environment.rs:25-27` | Low impact; gates guidance only. |

**Confirmed not exploitable:** no shell anywhere (two `Command::new(...).args(...)` sites); leading-`-` option injection blocked; path traversal/unsafe deletion blocked (`program_stem` yields a single component; `..`/`.`/absolute names are rejected); `-o` is emit-only; no network, credentials, or Docker socket.

**Out-of-scope observation:** `for (let i = 0; i < n; i++)` emits `for (; (i < n); )` and drops the step (`clc` codegen bug). Pre-existing, tracked separately.

## Rulings

User ruling (2026-09-23): **fix SA-801 and the cheap robustness minors** —
quote the handoff path (SA-801); align the hosted-amiga/host release command
with the §13 line by dropping the warning flags on release (RV-801); add the
missing newline and sanitize control bytes in driver messages (RV-802/SA-803);
make the `emit` temp file secure (RV-803/SA-804); fix the capture message
(RV-807); assert the release command (RV-805); cover the SDK/missing-tool caller
exit codes (RV-806); check the execute bit (SA-806). Document the exit-2
collision (RV-804) and the Make passthrough's trusted-input assumption (SA-802);
record SA-805/SA-807 as accepted. Then run the one permitted scoped re-review of
the fix diff.

## Accepted findings (not fixed)

- **SA-805** — the driver has no timeout and `--capture`/`vamos` buffer all
  stdout in memory. Accepted for an interactive developer tool; a runaway or
  unbounded-output program is the developer's own program. Revisit if the
  driver is used in automation.
- **SA-807** — `sdk_present` trusts any regular file at the marker path. Low
  impact: it gates guidance only, not a privilege, and the path is root-owned
  in the image.
- **RV-902** — the tool preflight uses the file mode bits rather than the
  caller's identity (`access(X_OK)`); PATH tools are normally world-executable.

## Re-review of the fix diff

The single permitted scoped re-review (`3dd4ac1..b21c2e3`) confirmed **every
accepted finding resolved** and found **no new critical or major** issue:

- SA-801 — the handoff path is single-quoted and escaped; an end-to-end probe
  with a name containing `$(touch PWNED)`, `;`, a space, and `'` produced a
  safe literal argument.
- RV-801 — debug keeps `-Wall -Wextra -Werror`, release matches the §13 line.
- RV-802/SA-803 — messages end with a newline and control bytes are sanitized;
  normal paths are unchanged.
- RV-803/SA-804 — the `emit` scratch C lives in a private `build/` directory
  created with `create_new`; no `/tmp` path remains.
- RV-805, RV-806, RV-807, SA-806 — resolved with tests.
- RV-804, SA-802 — documented; SA-805, SA-807 — accepted.

It raised one residual minor, RV-901 (the `usage error:` message was not
sanitized), which was then fixed; and the low RV-902, accepted above.

**Compliance note:** Phases 1–3 were reviewed together rather than per-phase
(the feature is small and tightly coupled), and the phase's single scoped
re-review was spent on the fix batch, so the RV-901 follow-up was verified by
`make check` rather than a further independent review.

### Accepted findings (not fixed)

| ID | Severity | Title | Reason accepted |
|---|---|---|---|
| SA-805 | 🟡 MINOR | No timeout; `--capture`/`vamos` buffer all stdout in memory | The driver is a developer tool that runs trusted local programs; a timeout and streaming capture are deferred. |
| SA-807 | 🟡 MINOR | `sdk_present` trusts any regular file | The marker only gates guidance; a wrong positive fails later at the tool check. |

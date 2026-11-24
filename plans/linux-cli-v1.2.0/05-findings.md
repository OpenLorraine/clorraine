# Findings ledger

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-001 | 1 | MAJOR | Native emulator | Fixed: native `--emulator` is explicitly unavailable in Phase 1; the old native staging path is retired, the marked SDK container keeps its handoff, and Phase 2 owns the verified runner. |
| RV-002 | 1 | MAJOR | Real toolchain gate | Fixed: `make toolchain-check` rebuilds the driver and runs the full `setup` validator (glibc, rc24 version, both probes) in a private config before building. |
| RV-003 | 1 | MAJOR | Test seam | Fixed: `resolve_from_stored` has marker/no-selection unit coverage; the container-skip integration case is no longer the only test. |
| RV-004 | 1 | MAJOR | Setup paths | Fixed: canonical roots and compilers are checked for UTF-8/control characters; compiler lines are sanitised; error messages carry the `clorraine:` prefix. |
| RV-005 | 1 | MAJOR | Test integrity | Fixed: added glibc 2.35, probe-failure-after-good-config, atomic write, no-fallback, empty-config, and config-log coverage. |
| RV-006 | 1 | MINOR | glibc parsing | Fixed: the minor component is parsed from dotted versions such as `2.37.1`. |
| RV-007 | 1 | MINOR | Probe pipe handling | Fixed: stdout/stderr are drained concurrently while a bounded watchdog waits. |
| RV-008 | 1 | MINOR | Error prefix | Fixed: setup errors are prefixed with `clorraine:`. |
| RV-009 | 1 | MINOR | Missing-tool hint | Fixed: `cc` and `vamos` get tool-specific guidance instead of setup guidance. |
| RV-010 | 1 | MINOR | Empty config | Fixed: an empty/unknown-only config is `None`, so the marked container can fall back; an empty value is still malformed. |
| RV-011 | 1 | MINOR | Directory containment | Fixed: required subdirectories are canonicalised and checked inside the root. |
| RV-012 | 1 | MINOR | Config permissions | Fixed: the Clorraine config directory is 0700 and the file is 0644 on Unix. |
| RV-013 | 1 | MINOR | Setup flags | Fixed: explicit `--profile` and `--diagnostics=json` are rejected by `setup`. |
| RV-014 | 1 | OBS | Exit-code docs | Fixed: the module doc names invalid/missing toolchain selection. |
| RV-015 | 1 | OBS | Review scope | Fixed: the expected set and refreshed diff include `run.rs`, retired files, the checker script, and `site/guide/known-limitations.md`. |
| RV-016 | 1 | OBS | Evidence | Fixed: Phase 1 red/green output, real setup, build sizes, and gate logs are recorded in the plan. |
| RV-017 | 1 | MINOR | Help/error wording | Fixed: help and the native-emulator error now describe the build limitation without phase language. |
| RV-018 | 1 | OBS | Scratch placement | Fixed: `toolchain-check` exports `TMPDIR`/`CODEOPS_TMPDIR` to `build/tmp`, and the script uses the same root. |
| RV-019 | 1 | OBS | Regression red evidence | Accepted with note: the initial public-path red run is recorded; the review-added failure tests (probe timeout, config-rename cleanup) are green and target the corrected failure paths. |

Re-review result: no CRITICAL or MAJOR findings. RV-017–RV-019 are fixed or accepted as notes above; RV-004R/RV-007R/RV-010R are resolved in the final diff.

## Phase 2 review (2026-11-24)

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-020 | 2 | MINOR | Capture failure output | Fixed: a capture-write failure prints the readable guest result before the retained diagnostic; the spec asserts stdout. |
| RV-021 | 2 | MINOR | Staging coverage | Fixed: the staging test asserts the A1200 config, literal argv options, isolated environment, 0700 mode, and that no ROM enters the guest drive. |
| RV-022 | 2 | MINOR | Wrapper build failure | Fixed: `FAKE_COMPILER_FAIL` seam and a test assert exit 1, no emulator start, and no retained private run. |
| RV-023 | 2 | MINOR | Selection coverage | Fixed: environment selection, both no-fallback directions, the 3.2.0 boundary (`3.1.99`/`3.2.0`/`3.2.35-2`), and pure timeout bounds (1/90/3600/0/3601/abc) are tested. |
| RV-024 | 2 | OBS | Site selection wording | Fixed after the re-review: the Amiga emulator page requires a native toolchain in the prose and example. |
| SR-001 | 2 | MAJOR | Gate process leak | Fixed: `pgrep` is required, the check starts from a clean process set, and it asserts no FS-UAE/Xvfb survivor after every invocation, including failures and the negative. A dirty-start run exits 2; the real gate is green. |
| SR-002 | 2 | MINOR | Guest exit codes | Fixed: help and the CLI reference state that a verified guest failure exits 1 for every nonzero guest code; host/`vamos`/freestanding keep their own code. |
| SR-003 | 2 | MINOR | Capture wording | Fixed: help and docs say capture is success-only; a failed guest prints its output and retains the run. |

Open-question rulings:

- The staging spec now asserts the effective A1200 config and ROM option; the
  real A1200/KS3.1 boot remains the platform evidence.
- `make fs-uae-check` stays a local product check, like `make toolchain-check`;
  it needs a licensed ROM and a native toolchain, so it is not added to
  `make verify`.
- Failure evidence retention is intentional and documented: a failed check
  keeps its temporary run and prints the path; a dirty-start preflight removes
  the empty temporary directory; a successful check removes it.

Fix-diff re-review (2026-11-24): no CRITICAL or MAJOR. SR-001 and RV-020–RV-023 are verified fixed. Three residuals were reported and fixed without a third review pass:

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-025 | 2 | OBS | Native-selection wording | Fixed: `site/amiga/emulator.md` now states the native-toolchain requirement and the example passes `--toolchain`. |
| RV-026 | 2 | MINOR | Capture scope | Fixed: the success-only rule is scoped to verified guests; host, `vamos`, and freestanding capture stdout regardless of exit code. |
| RV-027 | 2 | OBS | Dirty-start evidence | Re-recorded with the committed script: exit 2, clean-start message, no retained temporary directory (`build/tmp/fs-uae-gate-dirty4.log`). |
| SR-004 | 2 | MINOR | Gate retention wording | Fixed: the script header, design, and ledger now distinguish a failed check (evidence kept), a dirty-start preflight (empty temporary directory removed), and a successful check (removed). |

## Phase 3 review (2026-11-24)

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-028 | 3 | MAJOR | Purge coverage | Fixed: the installer spec now covers uninstall keeping data, purge with a sibling file, purge with `XDG_CONFIG_HOME` unset, empty config-directory removal, and `--purge` without `--uninstall`. |
| RV-029 | 3 | MINOR | Lock PID reuse/concurrency | Fixed: an alive PID with a different start time is stale; an uninitialized owner is live; a second installer is refused. |
| RV-030 | 3 | MINOR | Lock initialization race | Fixed: a missing/partial owner is not stale, and the loser waits briefly for the owner before failing. |
| RV-031 | 3 | MINOR | Hostile member types | Fixed: validation lists with `-P` and rejects absolute names; hardlink and absolute-member tests pass. |
| RV-032 | 3 | MINOR | Curl flags | Fixed: the fake curl records its argv; the network test pins `--disable`, HTTPS protos, timeouts, and the size cap. |
| RV-033 | 3 | MINOR | Rollback links | Fixed: rollback removes only links that were absent before the invocation. |
| RV-034 | 3 | MINOR | Temp cleanup/TOCTOU | Fixed: local archives are verified in a private copy; download/verification temp trees are removed on every exit. |
| RV-035 | 3 | MINOR | New-key precedence | Fixed: a both-keys editor test asserts `toolchain.path` wins with no warning. |
| RV-036 | 3 | MINOR | Legacy discovery | Fixed: a `~/.clorraine/bin` fixture is ignored in favor of `PATH`. |
| RV-037 | 3 | MINOR | Success line ordering | Fixed: the success line follows the PATH update. |
| RV-038 | 3 | MINOR | Check documentation | Fixed: `AGENTS.md` lists `dist-check` and splits the SDK/installer test commands. |
| RV-039 | 3 | OBS | PATH hint quoting | Fixed: the non-`--modify-path` hint uses the same single-quote escaping. |
| RV-040 | 3 | OBS | Dead code | Fixed: unused `PROGRAM`, parameters, and locals are gone. |
| RV-041 | 3 | OBS | Test root cleanup | Fixed: a suite trap removes every fixture root; leaked roots were deleted. |
| RV-042 | 3 | OBS | Asset identity | Fixed: the plan records one final candidate identity. |
| SA-001 | 3 | MINOR | Lock initialization | Fixed with RV-030. |
| SA-002 | 3 | MINOR | Link ownership | Fixed: `readlink -f` prefix containment; a foreign symlink is refused and left in place on uninstall. |
| SA-003 | 3 | MINOR | Local archive TOCTOU | Fixed with RV-034's private verified copy. |
| SA-004 | 3 | MINOR | Rollback word splitting | Fixed with RV-033's fixed link names. |
| SR-005 | 3 | MINOR | Asset identity | Fixed with RV-042. |
| SR-006 | 3 | MINOR | Survivor evidence | Fixed: the final E2E logs an explicit no-survivor line after every guest and prints `processes after: none`. |
| SR-007 | 3 | MINOR | Stale docs | Fixed: limitations, changelog, release notes, and quick start describe the local candidate and remaining acceptance/removal work. |

Fix-diff re-review (2026-11-24): no CRITICAL or MAJOR. RV-029–RV-042 and SA-001–SA-004 are verified fixed. Four residuals were reported and fixed without a third review pass:

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-043 | 3 | MINOR | Purge empty-dir assertion | Fixed: the test purges again after removing the sibling and asserts the directory; the HOME purge also asserts its config directory. |
| RV-044 | 3 | MINOR | Test-root registry | Fixed: roots are recorded in a shared file read by the EXIT trap; the post-fix runs leave no roots. |
| RV-045 | 3 | MINOR | EXIT-trap cleanup | Fixed: `rollback` returns 0 so the trap always releases the lock and removes private storage; the foreign-symlink test asserts no verified copy remains. |
| RV-046 | 3 | OBS | Design command/lock text | Fixed: `sleep` and `tr` are in the required-command list; the lock section documents the bounded missing-owner wait. |
| SR-008 | 3 | OBS | Plan evidence counts | Fixed: the final 83/21/36 and 174 editor counts are recorded. |

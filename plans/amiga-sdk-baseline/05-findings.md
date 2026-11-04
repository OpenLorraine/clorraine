# Findings: Amiga SDK Baseline (Bundle 0)

> **Document**: 05-findings.md
> **Parent**: [00-index.md](00-index.md)
> **Status**: Done (9/9; issue #1 closed)

## Red phases

### Profile tests

`cargo test --test amiga_a500_spec_test` before the profile existed: **1 passed, 3 failed**.
The 3.1 control passed; the 1.3 graphics case, the 1.3 exec-rejection case, and the
`nix13` cross-build failed with `E11201: unknown profile 'hosted-amiga-13'`. The
spec test was run red before the implementation; the test and profile landed
together in `6fddbdc`, and all four cases pass.

### Registry tests

`tools/amiga-sdk/registry_test.sh` before `registry.sh` existed failed with
`missing executable`. After the validator and seed existed, the test exposed a
test-side backtick mismatch in the issue-body grep; the generator was correct.
The fixed test passes.

## Phase review (correctness, memory safety, emission integrity)

### Correctness

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| RV-001 | 🟠 Major | The automated 1.3 "runtime proof" was a V47 API-emulation smoke, not a Kickstart 1.3 kernel. | **Fixed:** `make sdk-a500-smoke` decrypts the real KS1.3 ROM, boots the ADF in FS-UAE as an A500, and compares the captured `Hello, Amiga, from Clorraine!` line to `docs/amiga-sdk/a500-hello.png`; the full screen is `docs/amiga-sdk/a500-screen.png`. |
| RV-002 | 🟠 Major | `--emulator` for `hosted-amiga-13` booted the default A1200/3.1 model. | `fs_uae_handoff` now prints `AMIGA_MODEL=A500`, the KS1.3 ROM, and `RUNNER=adf`; the Makefile `EMULATOR=1` branch uses `$(PROFILE)` and guards non-Amiga profiles. |
| RV-003 | 🟠 Major | Docs said `std.amiga.exec` was 2.0+ with a missing fallback, but the shared shim is already 1.3-safe. | Reworded: the module stays gated on 1.3 until Bundle 7 validates it; the shim already uses 1.3-safe calls. |
| RV-004 | 🟠 Major | Core/1.3 registry rows named 2.0+ APIs (`ReadArgs`, hooks, EClock, regions, ReadJoyPort, semaphores). | Split into 1.3 rows and 2.0+/3.1 rows; corrected `min_os`/`min_hw`; validator passes. |
| RV-005 | 🟠 Major | `E11201` help disagreed with the driver, and several profile docs omitted `hosted-amiga-13`. | Updated `spec/14`, `spec/00`, `spec/15`, `spec/17`, `spec/appendix-stdlib`; added a registry-help consistency test. |
| RV-006 | 🟡 Minor | Registry tests were not gated; validator rules untested; deferred/out-of-scope rendered as open. | `make check` runs `registry_test.sh`; `make verify` runs the A500 smoke; added unknown-bundle, must/out-of-scope, and one-missing-evidence cases; issue bodies mark deferred/out-of-scope. |
| RV-007 | 🟡 Minor | Bundle 0 close-out was not reflected in statuses/coverage/issue #1; `SDK-BASE-010` belonged to Bundle 1. | Marked 001–009 `done`; moved 010 to `1-native-video`; regenerated coverage and all issue bodies; issue #1 closed. |
| RV-008 | 🟡 Minor | Plan/roadmap counts and the "committed red" claim were stale. | Swept counts to 0–9 / ten issues; corrected Bundle 7 and the red-phase wording. |
| RV-009 | 🟡 Minor | Tier definitions contradicted rows; duplicate icon/BOOPSI rows. | Advanced is now "2.0+/ECS and up"; removed the duplicate icon and BOOPSI rows. |
| RV-015 | 🟠 Major | The A500 Makefile prefix was assigned in a shell variable, so the shell treated `AMIGA_MODEL=A500` as a command word. | Moved to make-level `FS_UAE_ENV`; `make -n` and a `DOCKER_MAKE=true` parse probe show the assignments before the command. |

### Memory safety

**No findings in the Bundle 0 diff.** `AllocMem`/`FreeMem` sizes match, the port
create/delete pair is symmetric, drop routing is unchanged, and host ASan probes
plus target loops found no leak or double free. Two pre-existing issues are
recorded below.

### Emission integrity

**Delta clean.** Host and freestanding emission is byte-identical; the 3.1 delta
is the intended shim replacement; the 1.3 units build with `-mcrt=nix13` and
strict flags; declarations agree. Two documentation nits (EI-001/EI-002) were
fixed.

## Runtime resolution

The earlier attempts failed because the configured `.rom` files are Cloanto
`AMIROMTYPE1` images; FS-UAE silently fell back to AROS (V47), and the ADF
startup command also needed the real 1.3 shell. The working path:

1. Decrypt the licensed `amiga-os-130.rom` with `amitools` and `rom.key` to
   gitignored `build/`; verify SHA-1 `891e9a547772fe0c6c19b610baf8bc4ea7fcb785`.
2. Build a clean ADF with the root `startup-sequence` `DF0:hello` and the
   `boot1x` bootblock.
3. Boot FS-UAE as an A500 with the decrypted ROM.
4. Capture the inner window with X11 `import` after 35 seconds and compare the
   hello-line crop to `docs/amiga-sdk/a500-hello.png`.

The captured screen shows AmigaDOS Release 1.3 and
`Hello, Amiga, from Clorraine!` (see `docs/amiga-sdk/a500-screen.png`). `vamos` is no longer the acceptance proof.

## Accepted residuals

| Residual | Reason |
|---|---|
| PRE-001: `examples/amiga/message-drop.clo` failure paths delete a port while a sent/replied message is still queued (56-byte leak). | Pre-existing in both base and head; outside Bundle 0. The normal and allocation-failure paths are clean. |
| PRE-002: a fetched `Message` must outlive its reply port; deleting the port first is a use-after-free. | Pre-existing, already recorded as the PLAN-13 accepted residual; the compiler has no reply-port lifetime carrier. |

## Verification

- `make check` exit 0; `spec check: OK (27 files, 111 compile-fail tests)`; 93 test groups.
- `make verify` exit 0; `verify: all legs passed` (differential, FS-UAE corpus, six runtime faults, A500 smoke).
- `cargo test --test amiga_a500_spec_test` 4/4; `amiga_exec_spec_test` 6/6.
- `make sdk-registry` OK (79 capabilities, 10 bundles); `registry_test.sh` OK.
- `make sdk-a500-smoke` OK: nix13 build + `vamos` load/run.
- Ten tracking issues created (#1–#10); Bundle 0 issue #1 closed.
- `make sdk-a500-smoke` passes with the real KS1.3 screenshot comparison.

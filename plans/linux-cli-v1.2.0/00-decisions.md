# Decisions

> **Status**: All resolved for this plan
> **Rule**: A new ambiguity stops execution and is added here with a proposed
> decision before code changes.

| ID | Area | Question | Decision | Evidence |
|---|---|---|---|---|
| AR-1 | Scope | Which platforms ship in 1.2.0? | Linux x86_64 only. macOS and Windows are a separate follow-up. WSL is unsupported and not heuristically rejected. | Spike host is Linux; no macOS/Windows evidence. |
| AR-2 | Toolchain | Who provides the Amiga compiler? | The user downloads AmigaPorts `v16.2-rc24`; Clorraine validates it and never mirrors or redistributes it. | Spike pinned asset, size, and SHA-256. |
| AR-3 | Toolchain | How is the toolchain selected? | `--toolchain PATH` is a global selector for `setup`, `build`, and `run`; `setup` persists it. Precedence is flag > `CLORRAINE_TOOLCHAIN` > stored config > marker-gated legacy `PATH` compiler for the Docker SDK. The verified FS-UAE runner needs the explicit/config selection. `--sdk-root` and `CLORRAINE_SDK` are removed. | The old SDK layout cannot represent the upstream root; the existing Docker/handoff tests must stay valid. |
| AR-4 | Host | What is the Linux minimum? | glibc 2.38 or newer; Ubuntu 24.04+ is the minimum and was toolchain-probe tested. The full spike host was Ubuntu 26.04; acceptance records the actual host. The installer checks before running the CLI. | rc24 requires `GLIBC_2.38`; Ubuntu 22.04 fails. |
| AR-5 | Emulator | Which machine and ROM does `run --emulator` use? | FS-UAE >= 3.2.0, A1200, a 512 KB Kickstart 3.1 A1200 ROM. Both Amiga profiles may run there; this is not a KS1.3 proof. KS3.0 is not tested and is not an acceptance requirement. | Spike ran hello and corpus on A1200/KS3.1; hello13 also passed. |
| AR-6 | AROS | Is AROS a verified fallback? | No. The CLI requires a ROM for `run --emulator`. FS-UAE's built-in AROS was observed to boot and run hello, but it is not a supported or acceptance path and is not launched by the CLI. | Spike observed AROS output; the policy still requires a real ROM for a verified run. |
| AR-7 | Guest result | How is success proven? | A compiled guest wrapper runs the program with `SystemTagList`, writes `clorraine-rc.txt` always and `clorraine-status.txt` only for rc 0. The host requires rc 0 plus status `ok` and reads both before cleanup. | A two-line echo marker let a return-1 program pass; `FailAt` is unavailable on the bare drive. The wrapper was tested with rc 0/1/5/9. |
| AR-8 | Timeout | How long may a guest run take? | 90 seconds by default, overridable with `CLORRAINE_EMULATOR_TIMEOUT_SECONDS` (1–3600). Invalid values are usage errors. After exit, kill/wait is bounded; an unreaped child retains the directory. | Boot-to-result took about 6 seconds; a blocking wait could exceed the timeout. |
| AR-9 | Capture | Is `--capture` supported with FS-UAE? | Yes. It writes guest output to the normal `out.txt` and prints the path. | The old rejection was a limitation, not a requirement. |
| AR-10 | Installer | What does `install.sh` install? | A versioned CLI archive only. No toolchain, ROM, FS-UAE, Python helper, or Docker. Network mode verifies the pinned size and SHA-256; local `--archive` mode verifies the supplied checksum and records the size as evidence, because draft bytes may differ from the release pin. | The old helper path is superseded; the earlier size wording contradicted the draft-bytes mode. |
| AR-11 | Activation | How are updates and PATH handled? | Install into a unique `versions/<version>-<archive-sha12>` directory, verify `--version`, then atomically switch `current`. The old tree is never moved, so a crash leaves the old `current` usable. `--modify-path` replaces one marked block with a shell-quoted prefix; uninstall removes it. | A move-before-switch sequence is not crash-atomic. |
| AR-12 | Publication | Does this plan publish? | No. It builds a local candidate and records acceptance evidence. Tagging, release creation, and upload need separate approval. | Prior instruction: no publication without separate approval. |
| AR-13 | Editor | Which settings remain? | `clorraine.toolchain.path` replaces `clorraine.sdk.path`; the old key is read once with a warning. `clorraine.fsUae.path` and `clorraine.kickstart.path` remain pass-through flags. | Editor already passes those flags; only the SDK flag changes. |
| AR-14 | Removal | When is the old pipeline deleted? | After the local candidate passes acceptance. Docker SDK setup (`install_sdk.py`) and `tools/distribution/inputs/` stay. | Removal before proof was a prior blocker. |
| AR-15 | CI | What runs in CI? | Ubuntu 24.04 host tests plus hermetic `dist-check`; no network, FS-UAE, ROM, or acceptance evidence in CI. | CI has no licensed media. |
| AR-16 | Spec supersession | What happens to the old native-driver spec/impl suites? | The Linux toolchain contract replaces the native `sdk/amiga` spec cases. Rewrite `native_driver_spec_test.rs` and `native_driver_impl_test.rs` for the new contract, keep the legacy handoff cases, and record the supersession here. Plan approval is the required ruling. | The old cases pin removed flags and the old process-status return. |
| AR-17 | Locking | How is the installer locked? | Atomic `mkdir` lock containing the PID and Linux process start time; a stale lock is removed and the `mkdir` retried, with a post-retry recheck. Concurrent/live locks fail with the owner. | PID-only liveness is racy under PID reuse. |
| AR-18 | Editor artifact | Where is the VSIX delivered? | A separate local candidate asset (`build/dist/clorraine-1.2.0.vsix`), not inside the CLI archive. `make dist` builds it, `SHA256SUMS` covers it, and acceptance records its hash. Publication is out of scope. | The editor is a Must requirement but the CLI archive must stay CLI-only. |

## Superseded decisions

The blocked `cross-platform-cli-fs-uae` plan and its 33-item register are
superseded. Its unresolved installer, release, and acceptance contradictions
are replaced by the smaller contract above. Git history retains the old record.

## Runtime decisions (auto-design)

> **Authority**: AI — delegated by `--auto-design` (policy version 1; root
> invocation ID `linux-cli-v1.2.0-phase2-20261124`).
> **Eligibility**: implementation mechanisms inside the approved Phase 2
> runner design. No product behavior, scope, acceptance criterion, or reserved
> decision changes.
> **Objective**: make the verified runner work from the installed binary-only
> archive with bounded, inspectable failure handling.
> **Hardening**: the Phase 2 post-phase review checks these choices against the
> staging, timeout, termination, cleanup, and capture tests. The mechanisms are
> local and reversible, so no separate design challenger was required.

| ID | Decision | Evidence | Rejected alternatives | Strongest counterargument | Confidence | Reopen triggers |
|---|---|---|---|---|---|---|
| AR-19 | Embed `compiler/runtime/emulator_runner.c` with `include_str!`, write it into the private run, and compile it there. | The release archive contains only `bin/` binaries and release docs, so a run-time source lookup fails after install. | Look up the source relative to the driver (fails outside a checkout); ship the source in the archive (adds a release file and a path contract). | Embedding duplicates the wrapper bytes in the binary. Accepted: the wrapper is small and the staging test inspects the compiled source. | High | The archive starts shipping runtime sources, or the wrapper moves. |
| AR-20 | Terminate FS-UAE with `Child::kill` (SIGKILL) and poll `try_wait` for up to five seconds; retain the run when reaping is unconfirmed. | The crate forbids unsafe code and the standard library exposes no SIGTERM binding; the guest markers are complete before termination. | Declare the libc `kill` symbol (blocked by `#![forbid(unsafe_code)]`); call an external `kill` tool (adds a runtime dependency and a PATH seam). | SIGKILL denies FS-UAE a graceful shutdown. Accepted: the result is already read and the private directory is host-owned. | High | The crate starts allowing a signal crate or a safe SIGTERM API. |
| AR-21 | Re-read both markers once after `try_wait` reports exit; if still incomplete, fail with "did not report success" instead of waiting for the timeout. | An exited process cannot write more markers; the wrapper writes rc after the program returns. | Wait for the full timeout (slow and adds no evidence). | A marker could be written by a surviving grandchild. Accepted: the runner owns the direct child; a grandchild is outside the verified contract. | High | FS-UAE starts leaving a marker-writing helper after the main process exits. |
| AR-22 | Strip carriage returns from guest output and write the remaining bytes unchanged to stdout or `out.txt`. | The fake-emulator tests write CRLF; the existing `vamos` path already strips CR. | Preserve raw bytes (terminal noise); decode as UTF-8 (guest output need not be text). | A program that intentionally prints CR loses it. Accepted: CR is a line-ending convention, and the CLI already normalizes it for Amiga runs. | High | The product documents raw-byte output. |
| AR-23 | Parse the leading `major.minor[.patch]` numbers of the first stdout token and ignore a package suffix. | `3.2.35` is the tested output; packaged builds spell versions such as `3.2.35-2`. | Require an exact `3.2.35` string (rejects equivalent packaged builds); parse the whole line as a semver (no semver crate). | A malformed suffix could hide an older build. Accepted: the numeric prefix is the ordering fact, and the minimum is 3.2.0. | High | FS-UAE changes its version-line format. |
| AR-24 | A deadline with no marker is a timeout; an incomplete marker or an exited child is "did not report success"; a partial marker is re-read for at most one second. | Distinguishes "never started" from "started but incomplete" and bounds the empty/partial marker race the spike observed. | Treat every deadline as a timeout (loses the missing-marker distinction); wait indefinitely for a partial marker (unbounded). | A slow filesystem could exceed the one-second stabilization window. Accepted: the markers are tiny and the wrapper writes rc before status. | High | Real runs show a marker taking longer than one second to become complete. |
| AR-25 | Write `--capture` only for a successful guest; failure output goes to stdout and the run is retained. | `out.txt` is the success artifact; a failed run keeps its evidence in the retained directory. | Capture failures too (confuses the success artifact with retained evidence). | Scripts may expect capture on failure. Accepted: the exit code and retained path carry the failure. | Med | A requirement asks for failed-run capture. |
| AR-26 | `make fs-uae-check` accepts an optional `TOOLCHAIN` (otherwise the stored config), uses `xvfb-run -a` when `DISPLAY` is unset, and fails when a new FS-UAE or Xvfb process survives a run. | The plan's command names only `KICKSTART`; the local check must work with a stored setup and without a desktop. | Require both arguments (breaks the documented command); require a display (fails headless). | The stored config may point at a stale toolchain. Accepted: the driver validates it before the first build. | High | The check becomes a CI gate or the stored-config path is removed. |

**Reopen rule:** new evidence that breaks one of these assumptions reopens the
owning decision, updates this table and `03-02-fs-uae.md`, and re-runs the
Phase 2 gates.

### AR-27 — Installer/bootstrap supersession ordering

- **Authority**: AI — delegated by `--auto-design` (policy version 1; root
  invocation ID `linux-cli-v1.2.0-phase3-20261124`).
- **Eligibility**: delivery sequencing inside the approved plan; no product
  behavior, scope, or acceptance change.
- **Objective**: keep `make check` green while the new Linux installer replaces
  the old bootstrap template.
- **Decision**: `setup-check` runs only the retained Docker SDK tests
  (`test_cli`, `test_detect`, `test_fetch`); `make check` stops calling
  `distribution-check`; the old target and native/bootstrap files are removed
  after acceptance in Phase 4.
- **Evidence**: replacing `tools/setup/install.sh` broke 58 old tests that
  pinned its helper slots; the retained Docker SDK suite passes 29 tests.
- **Rejected alternatives**: keep the old template at a second path (confusing
  and still broken); remove the old pipeline before acceptance (out of order
  and against the plan's safety rule).
- **Strongest counterargument**: the old native tests no longer run before
  removal, so a latent defect could escape. Accepted: the old pipeline is
  superseded and not a product path; the new installer has its own 100 checks.
- **Confidence**: High — the retained suite is the documented Docker SDK suite.
- **Hardening**: the Phase 4 removal test checks retained inputs and notices.
- **Policy version**: 1 · **Root invocation ID**:
  `linux-cli-v1.2.0-phase3-20261124`
- **Reopen triggers**: the old pipeline is reinstated, or a retained Docker SDK
  test starts depending on the release template.

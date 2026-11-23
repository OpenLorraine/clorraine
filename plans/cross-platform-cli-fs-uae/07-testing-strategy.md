# Testing Strategy: Cross-platform CLI and FS-UAE delivery

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The primary behavior is an installed CLI that builds an Amiga program and runs
it in FS-UAE. Unit fixtures prove rejection and argument handling; the Linux
clean-install gate and follow-up real-machine checks are the acceptance proof.

## Test Slices & Groups

N/A — no test map. Task verification uses the command named on each task; phase
gates use `make check`, `make verify`, `make vscode-check`, `make site-check`.

## 🚨 Specification Test Cases

### Setup and CLI

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | Linux x86_64, glibc 2.35, curl/tar/hash tool present | Accepted; setup proceeds | R15/AR-9 |
| ST-2 | Linux i686, Linux ARM, or an unsupported OS in v1.2.0 | Rejected with the supported list; exit 1 | R12/AR-2/27 |
| ST-3 | Linux glibc 2.34 | Rejected with required 2.35; no config change | R15/AR-9 |
| ST-4 | `curl`, `tar` or the platform hash tool missing | Exit 1; prints the platform install command; no sudo | R15/R16/AR-9/31 |
| ST-5 | Host `cc` missing, Amiga build requested | Setup succeeds; host profile later reports the missing compiler | R7/AR-10 |
| ST-6 | `--toolchain PATH` with a valid upstream compiler | Validated; hello builds for both Amiga profiles; config written | R3/AR-26 |
| ST-7 | `--toolchain PATH` missing/invalid compiler | Exit 1 naming the path; no fallback/config | R3/AR-26 |
| ST-8 | Existing valid config/toolchain, rerun setup | Reused; no download; no config churn | R17/AR-19 |
| ST-9 | Toolchain smoke fails | Exit 1; previous config/bin remains; no partial config | R17/AR-19 |
| ST-10 | Config relative/unknown/duplicate/non-UTF-8 path | Exit 1; no executable selected | R5/AR-19 |
| ST-11 | `--fs-uae`/`--kickstart` on setup and run --emulator | Valid paths recorded/used; invalid paths exit 1 with remedy | R4/AR-11/12 |
| ST-12 | `--fs-uae`/`--kickstart` on build/emit/plain run | Usage error; no fallback | R4/AR-11 |
| ST-13 | Installer hash mismatch/HTTP failure | No activation; exit 1; old install remains | R2/AR-28/31 |
| ST-14 | Fresh install with empty PATH | Installer invokes `<home>/bin/clorraine` by absolute path; setup runs | R2/AR-28 |
| ST-15 | Update to a new version, then failed setup | Previous bin/config remains; new version not active | R17/AR-30 |

### Build and FS-UAE

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-16 | Configured toolchain, build hosted-amiga | Exit 0; Amiga hunk magic `0x000003F3` | R7/AR-26 |
| ST-17 | build hosted-amiga-13 | Exit 0; same hunk output; KS1.3 flags unchanged | R7/AR-1 |
| ST-18 | build host without `cc` | Exit 1 with host compiler remedy; Amiga builds unaffected | R7/R16/AR-10 |
| ST-19 | run --emulator with FS-UAE and user ROM | Staged drive/config; startup writes `result.txt`; host reads expected content; returns emulator status only | R8/AR-11/12/29 |
| ST-20 | No FS-UAE configured | Build succeeds; run --emulator exits 1 with remedy | R8/R16/AR-11 |
| ST-21 | No ROM configured | Warning; FS-UAE launched without ROM override; AROS is launch-only | R8/AR-12/29 |
| ST-22 | Configured ROM missing/unreadable | Exit 1 before launch; no AROS fallback | R8/AR-12 |
| ST-23 | Plain run with `vamos` on PATH | Uses it; program output visible | R9/AR-18 |
| ST-24 | Plain run without `vamos` | Exit 1 naming amitools and `--emulator` | R9/AR-18 |
| ST-25 | Spaces, comma, trailing-space ROM, control chars | Literal preservation where valid; comma/control/trailing-space rules retained | R5/R8/AR-23 |

### Editor

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-26 | Installed CLI/config in `<home>`, no PATH | Discovery finds it; Build/Run invoke it | R10/AR-13/21 |
| ST-27 | No CLI installed | Actionable install command; no task/bundled fallback | R10/AR-13 |
| ST-28 | Build and Run in FS-UAE | Exact CLI argv/profile; visible output/status; no shell strings | R10/AR-13 |
| ST-29 | WSL or Windows extension host in v1.2.0 | Unsupported/untested; no false success; Windows follow-up is separate | R10/AR-32 |

### Release and acceptance

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-30 | Release workflow definition | Manual dispatch; hosted Ubuntu; SHA-pinned actions; read-only build, one write upload; draft only; no self-hosted/Docker | R11/AR-22 |
| ST-31 | Linux archive | Three binaries + licence files; no checkout paths | R1/R11/AR-13 |
| ST-32 | SHA256SUMS and scripts | Archive, install scripts and VSIX hashes present | R11/AR-28/31 |
| ST-33 | Removed-path check | Old installer/support/workflows absent; Makefile/CI no dangling references | R13/AR-16 |
| ST-34 | Linux clean Ubuntu 22.04/24.04 and Debian 12 | Install draft assets, setup with upstream toolchain, host/Amiga builds, FS-UAE with user ROM and guest result; evidence hashes recorded | R12/R15/AR-27 |
| ST-35 | macOS/Windows follow-up | Out of this plan; separate plan and evidence | AR-27 |
| ST-36 | Intel Mac/WSL | Unsupported/untested; no false claim; WSL may pass Linux checks but is not supported | R12/AR-32 |
| ST-37 | Missing macOS/Windows evidence | v1.2.0 Linux-only release remains valid; no unverified archives attached | R12/AR-27 |
| ST-38 | Legacy extension SDK/FS-UAE/ROM settings present | One warning; settings ignored; CLI config is authoritative; no duplicate executable selection | R10/AR-21 |

## Acceptance evidence record

Each real-machine run writes one JSON record with exactly:

```json
{
  "schema": 1,
  "platform": "linux|macos|windows",
  "os_version": "...",
  "architecture": "x86_64|arm64",
  "date": "YYYY-MM-DD",
  "candidate": {
    "archive": {"name": "name", "sha256": "..."},
    "install_sh": {"sha256": "..."},
    "checksums": {"sha256": "..."},
    "vsix": {"sha256": "..."}
  },
  "toolchain": {"tag": "...", "sha256": "..."},
  "fs_uae": {"version": "...", "rom": "user|aros"},
  "commands": [{"argv": ["..."], "exit": 0, "result": "..."}],
  "guest_success": true,
  "evidence_files": {"log": "path", "sha256": "..."}
}
```

`guest_success` is true only with a user ROM and an observed program result; AROS
runs set it false and are launch-only. The acceptance test validates the record,
candidate hash and guest result.

## Test Categories

### Specification Tests

| Test file | ST cases | Component |
|---|---|---|
| `compiler/tests/setup_spec_test.rs` | ST-1–ST-12 | Setup/CLI |
| `compiler/tests/platform_port_spec_test.rs` | ST-16–ST-25 | Build/FS-UAE |
| `tools/vscode/test/discovery.spec.test.ts` | ST-26, ST-27, ST-29, ST-38 | Discovery/migration |
| `tools/vscode/test/actions.spec.test.ts` | ST-28 | Actions |
| `tools/release/tests/test_release_spec.py` | ST-30–ST-33 | Release/removal |
| `tools/setup/tests/test_bootstrap_spec.py` | ST-13–ST-15 | Bootstrap |
| `tools/release/tests/test_acceptance_spec.py` | ST-34–ST-37 | Linux acceptance and deferral |

### Implementation Tests

| Test file | Description |
|---|---|
| `compiler/tests/setup_impl_test.rs` | config/resolver/rollback edge cases |
| `compiler/tests/platform_port_impl_test.rs` | path/FS-UAE/vamos edge cases |
| `tools/vscode/test/actions.impl.test.ts` | argv/config/missing CLI |
| `tools/release/tests/test_release_impl.py` | archives/workflow/removal controls |
| `tools/release/tests/test_acceptance_impl.py` | evidence/hash mismatch controls |

### E2E / Manual

| Scenario | Steps | Expected |
|---|---|---|
| Linux clean install | Draft assets → install.sh → setup --toolchain → build → FS-UAE user ROM | ST-34 |
| macOS/Windows follow-up | Separate plan | Deferred |

## Verification Checklist

- [ ] All ST cases have concrete input/output and named test files
- [ ] Spec tests precede implementation and are red first
- [ ] Implementation tests added after green
- [ ] `make check`, `make verify`, `make vscode-check`, `make site-check` pass
- [ ] Linux evidence recorded; macOS/Windows follow-up evidence before claims

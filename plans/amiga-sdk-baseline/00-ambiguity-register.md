# Ambiguity Register: Amiga SDK Baseline (Bundle 0)

> **Plan**: [00-index.md](00-index.md)
> **Status**: ✅ GATE PASSED
> **Created**: 2026-11-04 14:33
> **Mode**: auto-design (`--auto-design` intent; root invocation `bundle0-2026-11-04-1433`)

Every material decision below is resolved or explicitly deferred. `AI — delegated`
rows use the auto-design policy (version 1); `User` rows are explicit decisions
from the conversation. No row is silently deferred.

| ID | Category | Question | Resolution | Authority | Status |
|----|----------|----------|------------|-----------|--------|
| AR-1 | Scope | What is Bundle 0? | Only the A500 baseline, the SDK registry, the tracking issues, and their verification. No graphics/audio/Intuition implementation. | User | ✅ Resolved |
| AR-2 | Scope | Which hardware/OS baseline? | Core tier: A500, OCS, 68000, 512 KB chip, Kickstart 1.3. Advanced tier stays A1200/AGA/68020/2 MB/Kickstart 3.1. | User | ✅ Resolved |
| AR-3 | Naming | What is the new profile called? | `hosted-amiga-13`. The existing `hosted-amiga` remains the 3.1 profile. | AI — delegated | ✅ Resolved |
| AR-4 | Technical | Which modules may the 1.3 profile import? | `std.io`, `std.mem`, `std.text`, `std.fmt`, `std.amiga.custom`, `std.amiga.dos`, `std.amiga.graphics`, `std.amiga.intuition`, `std.amiga.interrupt`. `std.amiga.exec` stays 2.0+ until Bundle 7a. | AI — delegated | ✅ Resolved |
| AR-5 | Technical | Separate 1.3 shim or shared shim? | One shared hosted-amiga shim. Replace `CreateMsgPort`/`DeleteMsgPort` with `CreatePort`/`DeletePort`, `AllocVec`/`FreeVec` with `AllocMem`/`FreeMem`, and open graphics/intuition at version 33. This keeps one source of truth and works on 1.3+. | AI — delegated | ✅ Resolved |
| AR-6 | Technical | Registry format and tooling | `docs/amiga-sdk/registry.tsv` is canonical; `coverage.md` is generated; a POSIX shell/awk validator and generator (`tools/amiga-sdk/registry.sh`) checks schema and emits the coverage view. No new runtime dependency. | AI — delegated | ✅ Resolved |
| AR-7 | Scope | GitHub issue granularity | One generated tracking issue per bundle (0–8), with a checklist from the registry; individual issues only for blocked/high-value items. The user asked for issues; the granularity is an implementation choice. | User (issues), AI — delegated (granularity) | ✅ Resolved |
| AR-8 | Naming | Registry fields and status vocabulary | Fields: id, bundle, area, capability, priority, tier, min_os, min_hw, status, depends, acceptance, evidence, spec, notes. Status: missing/planned/in-progress/blocked/done/deferred/out-of-scope. Priority: must/should/could/wont. Tier: core/advanced/optional. | AI — delegated | ✅ Resolved |
| AR-9 | Technical | How is the A500 runtime proven? | FS-UAE with `AMIGA_MODEL=A500` and the real Kickstart 1.3 ROM; directory drive first, bootable ADF fallback. The run must print the hello line; skip with a recorded note only when the ROM/display is unavailable. | AI — delegated | ✅ Resolved |
| AR-10 | Technical | Build/verify command | A500 cross build uses `-mcrt=nix13 -m68000 -fwrapv -lamiga -lm`; the project gates remain `make check` and `make verify`, with `./spec/check.sh` inside `make check`. | Project fact (AGENTS.md) | ✅ Resolved |
| AR-11 | Documentation | Which docs change? | `spec/15-profiles.md` (profile table), `spec/appendix-amiga.md` (1.3 note), `AGENTS.md` (profiles/commands), `README.md` (profile list). No grammar, chapter, diagnostic, or ABI change. | User (baseline), AI — delegated (locations) | ✅ Resolved |
| AR-12 | Scope | Release/version impact | Bundle 0 is infrastructure; no release, tag, or version bump. The next feature bundle owns the v1.1 release. | AI — delegated | ✅ Resolved |
| AR-13 | Scope | Registry seed depth | Seed every bundle at capability level (~6–10 rows each); each bundle plan expands its own rows. Not a 1:1 NDK symbol list. | AI — delegated | ✅ Resolved |
| AR-14 | Behavioral | What happens when a 1.3 program imports `std.amiga.exec`? | Reject with `E11200` naming the profile and the module; no translation unit. | AI — delegated | ✅ Resolved |
| AR-15 | Behavioral | Default runner for `hosted-amiga-13` | `vamos` for quick tests; `--emulator` selects FS-UAE and is allowed for both Amiga profiles. `required_tools` adds `vamos` only when `--emulator` is absent. | AI — delegated | ✅ Resolved |
| AR-16 | Edge case | Does a missing `std.amiga.exec` shim symbol break the 1.3 build? | No: the shared shim still defines every symbol; the module is gated out of the 1.3 profile, so the program cannot call it. The shared shim is compiled and linked on 1.3. | AI — delegated | ✅ Resolved |

## Delegated provenance

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanics within the confirmed Bundle 0 scope:
  profile name, module whitelist, shim strategy, registry format, issue granularity,
  runtime proof, documentation locations, seed depth, error behavior.
Objective: establish a trustworthy A500/1.3 baseline and a durable SDK registry.
Evidence: spec/15-profiles.md, compiler/src/profile.rs, compiler/src/clorraine/toolchain.rs,
  compiler/runtime/shim_hosted_amiga.c, docker/scripts/run.sh, docker/.env (KS 1.3 ROM present).
Rejected alternatives:
  - Separate 1.3 shim: duplicates ~800 lines of C and risks drift; rejected.
  - One profile with runtime version checks: cannot fix the `nix20`/`nix13` C runtime;
    rejected.
  - JSON registry: needs a parser/tooling; TSV + awk is dependency-free; rejected for
    Bundle 0.
  - Per-capability issues: noisy and hard to keep in sync; rejected for generated
    per-bundle tracking issues.
Strongest counterargument: a shared shim still contains 2.0 code paths; a 1.3 program
  could link them. Mitigation: module gating + the 1.3 smoke leg proves the supported
  surface; unsupported modules are compile-time errors.
Confidence: High — the NDK 3.2 headers provide 1.3-compatible calls; the KS 1.3 ROM is
  available; the profile/toolchain table is small and testable.
Hardening: The decision is reversible; the 3.1 profile is unchanged. No independent
  challenger is required for these bounded implementation choices.
Policy version: 1
Root invocation ID: bundle0-2026-11-04-1433
Reopen triggers: A500 smoke fails on KS 1.3; `nix13` lacks a required libc function;
  the registry cannot represent a capability; a user changes the baseline or scope.
```

## User decisions carried from the conversation

- Do the full SDK as smaller bundles; start with the most visual parts.
- Use the A500 as the lowest baseline.
- Create a registry and/or GitHub issues for the missing parts.
- Execute Bundle 0 with auto-commit and auto-design.

## Deferred

- `std.amiga.exec` on 1.3 is deferred to Bundle 7a (AR-4).
- Native video, sprites, audio/input, text/layers, DOS/assets, Intuition, Exec, and
  Workbench bundles are deferred to their own plans.

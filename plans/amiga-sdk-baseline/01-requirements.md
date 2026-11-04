# Requirements: Amiga SDK Baseline (Bundle 0)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Make the A500/Kickstart 1.3 target a first-class compiler baseline and create the
durable registry for the remaining Amiga SDK surface. Bundle 0 delivers no native
graphics/audio API; it makes the foundation trustworthy and the backlog visible.

## Functional Requirements

### Must Have

- **R1 — 1.3 profile.** Add `hosted-amiga-13` for A500/OCS/68000/512 KB/Kickstart 1.3.
  It uses `-mcrt=nix13`, `-m68000`, `-fwrapv`, `-lamiga -lm`.
- **R2 — Module gating.** On 1.3, allow `std.io`, `std.mem`, `std.text`, `std.fmt`,
  `std.amiga.custom`, `std.amiga.dos`, `std.amiga.graphics`, `std.amiga.intuition`,
  and `std.amiga.interrupt`. Reject `std.amiga.exec` and future modules with `E11200`.
- **R3 — Shared shim compatibility.** The hosted-amiga shim must compile with
  `-mcrt=nix13`: use `CreatePort`/`DeletePort`, `AllocMem`/`FreeMem`, and open
  graphics/intuition at version 33. The 3.1 profile must keep working.
- **R4 — A500/KS1.3 runtime proof.** Build `examples/hello/hello.clo` with
  `--profile hosted-amiga-13` and run it under FS-UAE A500 with the Kickstart 1.3
  ROM; capture the visible `Hello, Amiga, from Clorraine!` line. The `nix13` build
  plus `vamos` load/run is a pre-check, not the acceptance.
- **R5 — Registry.** Add a TSV registry with stable capability IDs, schema validation,
  a generated `coverage.md`, and a programme map covering Bundles 0–9.
- **R6 — Tracking issues.** Generate and create one GitHub tracking issue per bundle,
  with a checklist from the registry; individual issues only for blocked/high-value items.
- **R7 — Documentation.** Update `spec/15-profiles.md`, `spec/appendix-amiga.md`,
  `AGENTS.md`, and `README.md` with the two-tier baseline and the new profile.
- **R8 — Verification.** `make check`, `make verify`, `./spec/check.sh`, `make sdk-registry`,
  and the A500 smoke leg pass.

### Should Have

- The registry validator rejects duplicate IDs, unknown bundles, invalid statuses,
  missing dependencies, and `done` rows without acceptance evidence.

### Won't Have (Out of Scope)

- Native video, sprites, audio/input, text/layers, DOS/assets, Intuition, Exec, or
  Workbench implementation.
- `std.amiga.exec` on 1.3 (deferred to Bundle 7).
- A release, tag, or version bump.
- A JSON/YAML registry parser, a two-way issue sync, or per-capability issues.
- Any language, grammar, diagnostic, or ABI change.

## Technical Requirements

### Compatibility

- `hosted-amiga` (3.1) behavior and artifacts stay valid.
- The 1.3 profile uses the same emitted-C shape and shim; only the runtime flags and
  module surface differ.
- The 3.1 profile keeps `std.amiga.exec`; the 1.3 profile rejects it at compile time.

### Performance

- No runtime performance change for 3.1 programs.
- The 1.3 profile must build the hello program without a new dependency.

### Security

- No new network, credential, or user-input surface. The registry validator treats
  repository text only. Issue creation uses the existing `gh` authentication and only
  the repository's own issue tracker.

## Scope Decisions

| Decision | Options | Chosen | Rationale | AR Ref |
|---|---|---|---|---|
| Lowest baseline | A500/1.3, A500+/2.0, A1200/3.1 | A500/1.3 | User's hardware/emulator baseline | AR-2 |
| Profile model | New profile, runtime checks | New profile | `nix13` vs `nix20` is a build-time runtime choice | AR-3 |
| Shim | Separate, shared | Shared with 1.3 fallbacks | One source of truth; no 800-line duplication | AR-5 |
| Registry | TSV, JSON, issues-only | TSV + generated Markdown | Dependency-free, reviewable, machine-readable | AR-6 |
| Issues | Per bundle, per capability | Per bundle | Visibility without issue noise | AR-7 |

## Acceptance Criteria

1. `clc --profile hosted-amiga-13` emits C; a 1.3-only module import is accepted.
2. `std.amiga.exec` on 1.3 reports `E11200`; the 3.1 profile still accepts it.
3. The A500 hello builds with `-mcrt=nix13` and prints the hello line under
   FS-UAE A500/KS1.3; the captured line matches `docs/amiga-sdk/a500-hello.png`.
4. The registry validator passes; `coverage.md` is generated; ten tracking issues exist.
5. `make check`, `make verify`, `./spec/check.sh`, and the A500 smoke leg pass.
6. No language, grammar, diagnostic, or ABI change.

# Current State: Amiga SDK Baseline (Bundle 0)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing implementation

- `Profile` has three variants: `Host`, `HostedAmiga`, `Freestanding`
  (`compiler/src/profile.rs:20-28`). `parse` accepts only those names
  (`:35-42`); `supports_module` gives `hosted-amiga` every module (`:73-79`).
- The hosted-amiga toolchain uses `-mcrt=nix20` and `-lamiga -lm`
  (`compiler/src/clorraine/toolchain.rs:39-54`).
- The runtime shim is shared by every hosted-amiga program
  (`compiler/src/profile.rs:13`). It opens graphics/intuition at version 37
  (`compiler/runtime/shim_hosted_amiga.c:596,600`), creates message ports with
  `CreateMsgPort` (`:690`), uses `AllocVec` (`:707`), and frees with `FreeVec`
  (`:764`). Those are 2.0 APIs; a `-mcrt=nix13` build fails at compile time.
- The A500 Kickstart 1.3 ROM is present at `~/amiga/roms/amiga-os-130.rom`
  (mounted as `/amiga/roms/...`). The FS-UAE runner sources `docker/.env` and
  hardcodes `KICKSTART`/`AMIGA_MODEL` from it (`docker/scripts/run.sh:20-35`),
  so there is no per-run A500 override yet.
- `docker/scripts/corpus-fs-uae.sh` is the existing pattern for a bounded FS-UAE
  run: start in the background, poll the captured output for a sentinel, stop the
  emulator, and diff the capture.
- The standard-library table lists ten modules
  (`compiler/src/stdlib.rs:103-190`); `std.amiga.exec` is 2.0+ because of
  `CreateMsgPort`/`AllocVec`.
- The project's gates are `make check` (`./spec/check.sh` + `cargo test
  --workspace`) and `make verify` (`AGENTS.md` Commands).

## Gaps

| Gap | Required change |
|---|---|
| No 1.3 profile | Add `HostedAmiga13` with `-mcrt=nix13` and its name/parse/runner/ABI mappings |
| No 1.3 module gate | Whitelist the 1.3-safe modules; reject `std.amiga.exec` with `E11200` |
| Shim uses 2.0 APIs | `CreatePort`/`DeletePort`, `AllocMem`/`FreeMem`, `OpenLibrary(...,33)` |
| No A500 run | Add explicit `KICKSTART`/`AMIGA_MODEL` overrides and a bounded A500 smoke script |
| No registry | Add `docs/amiga-sdk/registry.tsv`, `bundles.tsv`, schema, validator, coverage |
| No issue tracking | Generate and create one tracking issue per bundle |
| Docs still say `nix20` only | Update `spec/15-profiles.md`, `spec/appendix-amiga.md`, `AGENTS.md`, `README.md` |

## Risks

| Risk | Mitigation |
|---|---|
| `nix13` lacks a libc function the shim uses | Build the 1.3 hello early; add a fallback in the shim if needed |
| Shared shim accidentally exposes a 2.0 path | Module gate + compile-time A500 smoke; `std.amiga.exec` remains 3.1-only |
| A500 directory drive does not mount on KS1.3 | Use the existing bootable-ADF path as fallback |
| Registry and coverage drift | Generated `coverage.md` is committed; `make sdk-registry` regenerates and validates |
| Issue creation needs `gh` auth | The repo already uses `gh`; if auth fails, record the blocker and leave the registry as the source |

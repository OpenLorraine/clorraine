# Changelog

## [1.0.0] - 2026-11-04

### Added

- v1.0 specification freeze: chapters 00-17 and appendices A/B are `stable (v1.0)`.
- `std.amiga.exec`: private message ports, `NewMessage`/`Message`, signals, put/get/reply,
  and release operations.
- Hosted-amiga acceptance programs: `exec-self-test`, `message-drop`, and the MIT Hamurabi port.
- Developer tooling: `--explain CODE`, `--diagnostics=json`, and the canonical TextMate grammar
  under `editors/`.
- MIT licence, release checklist, known-limitations document, and third-party notices.
- Host-only CI workflow, `make asan`, and `make differential`.

### Fixed

- Release-blocking ownership, drop, nullability, and emission residuals from the
  production-readiness hardening phases.
- Driver behavior on a bare host: host-profile builds and `emit` no longer require the SDK marker.
- `make differential` argument forwarding: `DIFF_ONLY`/`OUTDIR` are passed through the container
  environment, never interpolated into the shell command.

### Verification

`make check`, `make verify`, `make asan`, and `make differential` pass for v1.0.0.

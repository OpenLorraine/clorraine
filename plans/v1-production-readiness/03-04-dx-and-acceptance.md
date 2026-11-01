# DX and Acceptance: Clorraine v1.0 Production Readiness

> **Document**: 03-04-dx-and-acceptance.md
> **Parent**: [Index](00-index.md)

## Overview

Two existing surfaces must be complete before the production gate: the DX slice already planned
under `plans/clorraine-dx/`, and the bounded PLAN-13 Amiga acceptance slice. This programme does
not restate either plan. It sequences them, records the gate, and requires every implementation,
porting, and FFI gap they expose to be fixed before the final production check. An accepted-program
gap blocks PLAN-13 Done; a gap that needs a new language feature stops for a user ruling and is
outside the slice.

## Architecture

### Current Architecture

- `plans/clorraine-dx/99-execution-plan.md` is 0/28. It delivers `--explain` on both CLIs,
  `--diagnostics=json`, and the canonical TextMate grammar. Its plan, ST cases, and component
  docs are complete; only execution is missing.
- PLAN-13 is a roadmap backlog row (`plans/00-roadmap.md:576`), not a plan. The repository has
  `std.amiga.dos`, `std.amiga.interrupt`, `std.amiga.intuition`, `std.amiga.graphics`, and
  `std.amiga.custom`, but no `std.amiga.exec` module and no external console-program acceptance.
- The hosted-amiga corpus already runs four portable programs; the FS-UAE legs cover the
  hardware-specific programs.

### Proposed Changes

1. **DX execution.** Run the existing `plans/clorraine-dx/` plan to 28/28 with its own
   specification-first ordering and review dispatch. Do not edit its tasks from this programme.
   Because DX task 1.2.1 adds twelve help cells to `spec/14-diagnostics.md`, re-review that file
   after the DX plan completes, run `./spec/check.sh`, and re-record its `stable (v1.0)` status.
   The production gate requires its `99-execution-plan.md` progress to read 28/28 and its
   verification (`make check`, `make verify`, `make demo-check`) to be green.
2. **PLAN-13 plan.** After the user records the two programme names and licence audit at the
   Phase 5 gate, create a separate `plans/amiga-library-surface/` plan via `make-plan` for the
   bounded slice: `std.amiga.exec` plus those two allowlisted console programs. The plan owns the
   wrapper API, FFI contracts, and porting details.
3. **PLAN-13 execution.** Execute that plan to Done. The two programs build and run on
   hosted-amiga against an independent expected-output oracle. An exposed implementation,
   porting, or FFI gap is fixed in the same plan or a bounded follow-up. A gap that requires a
   language-surface change stops the phase for a user ruling; it does not silently expand v1.0.
4. **Production gate.** The final phase checks both plans' progress and the reproduced outputs.
   Phase 5 starts only after the user records the two programme names and the licence audit under
   AR-13.

## Implementation Details

### DX Plan Boundary

| Item | Owner |
| ---- | ----- |
| `--explain`, `--diagnostics=json`, TextMate grammar | `plans/clorraine-dx/` |
| DX ST-1 – ST-19 | `plans/clorraine-dx/07-testing-strategy.md` |
| DX component designs | `plans/clorraine-dx/03-01` – `03-03` |
| DX execution and review | `plans/clorraine-dx/99-execution-plan.md` |
| Production gate on DX completion | This plan, Phase 4 |

### PLAN-13 Boundary

| Item | Owner |
| ---- | ----- |
| `std.amiga.exec` wrapper surface | New `plans/amiga-library-surface/` |
| Two allowlisted console programs | User selects at the Phase 5 gate; the new plan records the names |
| Porting and FFI gaps | New plan; new DEF rows; fix in the plan or a bounded follow-up |
| Accepted-program compiler gaps | New plan; they block PLAN-13 Done |
| Language-surface gaps | Out of scope; stop for a user ruling |
| Production gate on the slice | This plan, Phase 5 |

The two programs are selected from the roadmap's allowlist: public domain, BSD, MIT, APL, or
GPL-compatible sources. The user selects them at the Phase 5 gate (AR-13); the new plan records
the two names and the licence audit, and the production gate cannot close Phase 5 until they are
recorded.

### New Types/Interfaces

None in this programme. The DX plan owns its CLI flags and grammar file; the PLAN-13 plan owns
the `std.amiga.exec` module surface and FFI declarations.

### New Functions/Methods

None in this programme. The DX and PLAN-13 plans own their code.

### Integration Points

- The DX plan's final verification includes `make check`, `make verify`, and `make demo-check`;
  the Phase 4 review also re-reviews `spec/14-diagnostics.md` after its help-cell edit.
- The PLAN-13 plan's final verification includes `make verify`, the two program runs, and an
  independent expected-output oracle.
- Before any `make verify` in Phases 4, 5, or 7, check the FS-UAE environment: the SDK X display,
  `docker/.env`, and the local Kickstart ROM path. A missing dependency is a named blocker, not a
  code failure.
- This programme records the gate in Phase 4 and Phase 5 of `99-execution-plan.md`.

## Code Examples

```sh
# Execute the DX plan through the normal CodeOps flow.
# exec-plan clorraine-dx --auto-commit --auto-design

# Create the PLAN-13 plan when Phase 5 starts.
# make-plan amiga-library-surface

# Production gate checks.
grep -n 'Progress' plans/clorraine-dx/99-execution-plan.md
make check
make verify
make demo-check
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| DX plan execution finds a new ambiguity | Stop and resolve it through the DX plan's register | AR-1 |
| DX verification fails | Do not close Phase 4; fix the DX plan's implementation | AR-1 |
| DX edits the diagnostics registry | Re-review `spec/14-diagnostics.md`, run `./spec/check.sh`, and re-record its stable status | AR-6 |
| PLAN-13 scope grows beyond the slice | Keep the wider surface in PLAN-13 backlog; the gate uses only the bounded slice | AR-8 |
| A port exposes a compiler defect in an accepted v1.0 construct | Fix it; file a DEF row and re-run the program; it blocks PLAN-13 Done until closed | AR-8 |
| A gap needs a language-surface change | Stop; do not expand v1.0; ask for a user ruling; it is outside the slice | AR-8 |
| The two programs are not yet selected | Phase 5 stays blocked; record the AR-13 deferral and do not count it as Done | AR-13 |
| A program needs an unsupported profile feature | Reject it for the slice or add a bounded wrapper; no language feature | AR-8 |
| FS-UAE, X, or ROMs are missing | Stop with a named environment blocker; do not report a code failure | AR-9 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- DX: its own ST-1 – ST-19, plus `make check`, `make verify`, and `make demo-check`; ST-15.
- DX post-review: `spec/14-diagnostics.md` re-reviewed after the help-cell edit; `./spec/check.sh`
  green; status re-recorded.
- PLAN-13: the two programs' output on hosted-amiga against an independent, hand-checked oracle;
  `make verify`; the new wrapper's unit or integration tests in the PLAN-13 plan; ST-16.
- Production gate: `grep` the DX plan progress is 28/28; PLAN-13 is Done; both outputs match.
- Environment preflight: FS-UAE, the SDK X display, `docker/.env`, and the ROM path are present
  before any `make verify` in this component.

# Task: Honour a caller-supplied display in the A500 gate scripts

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 5/5 tasks (100%) — full `make verify` green
> **Reasoning**: low — five shell scripts preserve two environment variables
> they already use; no compiler, runtime, ABI, stdlib, capture geometry, or
> golden change.

## Objective

Deliver issue #36: let the five A500 screenshot gates run on a private X
server. The scripts currently source `docker/.env` with `set -a`, which
overwrites the caller's `DISPLAY` and `XAUTHORITY`, so a private Xvfb run
requires a temporary edit of the local `.env`. Capture the caller's values
before sourcing and restore them when they were set; otherwise the local SDK
configuration still supplies the desktop display.

**Smallest viable design:** add two caller captures before the existing
`set -a` / source / `set +a` block and two fallbacks after it in
`a500-smoke.sh`, `a500-boing-smoke.sh`, `a500-sprite-smoke.sh`,
`a500-paddle-smoke.sh`, and `a500-text-smoke.sh`, document the option in each
header, and add one static specification test that requires the pattern.

## Tasks

- [x] T-01 Add `the_a500_screenshot_gates_honour_a_caller_display` to
  `compiler/tests/amiga_a500_spec_test.rs`: for each of the five gates, require
  the caller `DISPLAY`/`XAUTHORITY` captures and the post-source fallbacks.
  Record the red run before the scripts change.
- [x] T-02 Add the override to the five scripts and document it in their
  headers. With no caller value the behavior is unchanged.
- [x] T-03 Prove the override at runtime: run two gates on a private `:99`
  Xvfb without editing `docker/.env`, run one gate with an unreachable display
  as the negative probe, and run one gate on the desktop through the default
  `.env` path.
- [x] T-04 Run the independent correctness and Amiga platform reviews; fix
  every critical/major finding. Both reviews are clean on critical/major; the
  three minor platform findings are fixed and recorded in the Findings section.
- [x] T-05 Run `make check` and the full `make verify` gate; update the roadmap
  and close #36. The final verify ran the five display-aware gates on a private
  `:99` Xvfb and reports `verify: all legs passed`.

**Verify**: `make check`; `DISPLAY=:99 docker/scripts/a500-boing-smoke.sh …`;
`DISPLAY=:99 docker/scripts/a500-text-smoke.sh …`; the negative and desktop
probes; `make verify`.

## Design constraints

| Constraint | Reason |
|---|---|
| Only `DISPLAY` and `XAUTHORITY` are overridable | They are the two variables the X capture path already reads |
| Caller values win only when set | A headless shell with no `DISPLAY` keeps the local `.env` behavior |
| No capture, crop, golden, or Makefile change | The issue is display selection, not geometry |
| The private server must share `/tmp/.X11-unix` | The FS-UAE container already mounts that socket directory |
| `XAUTHORITY` remains mounted | A private server with `-ac` ignores it; an authenticated server needs it |

## Known risks

| Risk | Mitigation |
|---|---|
| A stale caller `DISPLAY` selects the wrong server | The gate fails with its existing no-window or size assertion; the caller can unset `DISPLAY` to fall back to `.env` |
| A private Xvfb uses authentication | Set `XAUTHORITY` for that server; the override preserves it |
| The override changes the desktop path | With no caller value the variables come from `.env` exactly as before; T-03 runs a desktop gate |
| Five scripts drift | The static test requires the same pattern in all five |

## Key decisions

| Decision | Outcome |
|---|---|
| Override mechanism | The standard `DISPLAY`/`XAUTHORITY` environment variables, not a new gate-specific variable |
| Default | `docker/.env`, so existing `make sdk-a500-*` behavior is unchanged |
| Private-server recipe | Start Xvfb in the SDK image with `/tmp/.X11-unix` shared, then run the gate with `DISPLAY=:NN` |
| Guard | One static specification test over the five gates |
| Specialist reviews | Correctness plus Amiga platform, because the change is FS-UAE gate mechanics |

## Findings

### Red baseline

`cargo test --test amiga_a500_spec_test
the_a500_screenshot_gates_honour_a_caller_display` failed before the change:

```
docker/scripts/a500-smoke.sh must honour a caller-supplied display with "caller_display=\"${DISPLAY:-}\""
```

The test passes with the five scripts changed.

### Override block

Each script now captures `caller_display` and `caller_xauthority` before the
`set -a` / source / `set +a` block and restores them afterwards. With no caller
value the `docker/.env` values are used exactly as before.

### Runtime proof

| Probe | Result |
|---|---|
| `DISPLAY=:99` Boing gate on a private Xvfb, `docker/.env` untouched | OK |
| `DISPLAY=:99` text gate on the same Xvfb | OK |
| `DISPLAY=:97` sprite gate, no server on `:97` | gate exits 1 with `a500-sprite-smoke: no FS-UAE window`; FS-UAE logs `FATAL: [GLAD] Failed to initialize OpenGL context`, so it used `:97` rather than the desktop `:0` |
| `env -u DISPLAY -u XAUTHORITY make sdk-a500-boing` | OK through the `.env` path; `DISPLAY=:0` unchanged |

The ordering guard was probed by moving the caller capture below the
`docker/.env` source in the Boing script; the test fails with
`the caller display and authority must be captured before sourcing docker/.env`.

### Limits

- Only the five gates listed here honour the caller display. `a500-assets` and
  `a500-exec` still overwrite it from `docker/.env`; they are follow-up #35.
- `docker/.env` is no longer authoritative for the display. Only
  `/tmp/.X11-unix` is mounted into the FS-UAE container, so a caller
  `DISPLAY=localhost:10.0` (an ssh-forwarded display) cannot work; use a
  shared Unix socket such as an Xvfb display.
- A reused private display can match an FS-UAE window left by a killed run.
  That is pre-existing on `:0` too; start a fresh display per run.
- No fail-fast display probe is added. An unreachable display still fails with
  the gate's existing no-window error after the poll.

### Review findings

The correctness review reported no findings. It ran `make check` exit 0 on the
final scripts and test, verified the base-red and the order mutation, exercised
the shell matrix (caller wins only when non-empty, values stay exported), and
confirmed the change touches no geometry, golden, Makefile, or `.env`.

The Amiga platform review reported three minor evidence gaps:

| ID | Finding | Resolution |
|---|---|---|
| AP-001 | The static guard was order-blind: a caller capture moved below the `.env` source still passed | Added capture-before-source and restore-after-source assertions; a mutation probe fails with `the caller display and authority must be captured before sourcing docker/.env` |
| AP-002 | The negative-probe record stopped before the gate's own error | Re-ran to completion: exit 1 with `a500-sprite-smoke: no FS-UAE window` and the FS-UAE `FATAL: [GLAD] Failed to initialize OpenGL context` line |
| AP-003 | The "no caller display" desktop row used the caller `:0` branch, not the `.env` branch | Re-ran with `env -u DISPLAY -u XAUTHORITY`; the gate passes through `.env` |

The Amiga platform reviewer also ran the hello gate on `:99` with AE=0 against
`a500-hello.png`, proving the 640x256 aspect-corrected path works on a WM-less
Xvfb, and confirmed that a reused display can match a leaked FS-UAE window
(recorded as a limit, not changed here).

### Verify observation

The first full `make verify` stopped at `sdk-a500-smoke` with `AE=4`: a single
2x2 dark block at the top of the first glyph, consistent with the host pointer
being drawn over the emulated line. The rerun then stopped at
`sdk-a500-paddle` with `never saw the paddle screen`; the last capture was the
shell, and the held court frame never matched. Both gates pass alone and on a
private Xvfb, and both captures are deterministic, so the common suspect is
the host pointer overlay rather than the display override. Filed as #37 and
not changed here.

The final acceptance run uses the feature under test: the five display-aware
A500 gates run with `DISPLAY=:99` on a private Xvfb, while the legacy gates
(assets, exec, A600/A1200) keep the `.env` `:0` path. This removes the pointer
from the five captures and proves the override on the full gate. The run
reports `verify: all legs passed`.

### Gate evidence

| Gate | Result |
|---|---|
| Correctness review `make check` | exit 0 on the final scripts and test |
| Final `make check` (inside verify) | exit 0; workspace tests, 103 bench tests, fuzz smoke `cases=28 host=28 sanitizer=16 target=16 failures=0` |
| Final `make verify` | `verify: all legs passed`, exit 0; `a500-smoke`, `a500-boing`, `a500-sprites`, `a500-paddle`, and `a500-text` all report OK on the private `:99` display |
| Runtime probes | `:99` Boing/text/hello pass; `:97` fails through the no-window path; `env -u DISPLAY -u XAUTHORITY` passes through `.env` |





<!-- title: [bug] Release interrupt servers and other OS registrations when a program faults -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R1 -->

## Summary
Every program that faults (index out of range, division by zero, out of memory, `assert`, `panic`) after installing an OS registration leaves it behind. For an interrupt server (`addHandler`, `addPortHandler`, and the stdlib users of the dispatcher: input, timer, video) the next program loaded into that memory turns the server into a jump into arbitrary code at 50 Hz — a crash of the whole machine. The same exit skips: worker tasks (left on the ready list in unloaded code), the custom `View` and DMACON (display not restored), windows and screens (IDCMP ports signalling a dead task), the Workbench public-screen lock, public ports (in the system list with a name in freed memory), devices with pending I/O, sprite and audio channels, DOS locks, and every `AllocMem` block (AmigaOS does not reclaim them).

| Field | Value |
|---|---|
| Finding | ASM-WS5-001 · DEF-582 |
| Severity / Priority / Rank | Critical / P0 / 1 of 58 |
| Domain / Lens | shims / A500, A1200 shipped (shared code) |
| Confidence / Verdict | verified / confirmed (Critical) (independent verification) |
| Effort | L |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probe `probes/WS5/541-fault-handlers.clo` (registers a VBlank handler, creates a public port, then indexes out of range), runner `probes/WS5/54x-fs-uae.sh` (build: `make -C docker exec CMD="bash plans/independent-assessment/probes/WS5/54x-fs-uae.sh build build/tmp/assessment/baseline/clc build/tmp/assessment/WS5/fsuae"`; run on the host: `bash plans/independent-assessment/probes/WS5/54x-fs-uae.sh run build/tmp/assessment/WS5/fsuae 541 541c`). Expected (`541-fault-handlers.expected`: exec `AddIntServer` autodoc — a server must be removed before its code and `Interrupt` structure go away; spec/05 §15 — a fault aborts without the cleanup ladder): the walker's after-line equals its before-line. Observed on A1200/KS3.1: `before vertb 4 …`, `error E11211: index out of range`, `rc 20`, `after vertb 5 …`; the no-fault contrast 541c ends `after vertb 4`. The walker's server count fired on the seeded program (`vertb 4 → 5`). Under `vamos`, probe 503 (`lockPath` then a fault) orphans a 20-byte block, the size of a DOS `FileLock` (`records/WS5-runs.log`), on both profiles and both build lines; its no-fault contrast is clean. Two filler runs (`54x-filler.c`, 541f) did not crash the machine: the freed program memory was not overwritten in those runs, so the crash itself is `probable`, the dangling registration `verified`.

Probe `plans/independent-assessment/probes/WS5/541-fault-handlers.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;
import { HandlerToken, addHandler, removeHandler } from std.amiga.interrupt;
import { MsgPort, createPublicPort, deleteMsgPort } from std.amiga.exec;

volatile let ticks: u32 = 0;

func onTick(): void {
    ticks = ticks + 1;
}

func pick(n: usize): usize {
    return n;
}

func main(): i32 {
    let token: HandlerToken = addHandler(onTick);
    let port_opt: MsgPort? = createPublicPort("ClProbe541");
    if (port_opt == null) {
        println("no port");
        removeHandler(token);
        return 1;
    }
    let port: MsgPort = port_opt;
    let xs: u8[2] = [1, 2];
    println("armed");
    println($"{xs[pick(5)]}");
    removeHandler(token);
    deleteMsgPort(port);
    return 0;
}
```

## Root cause
the language rule (a fault aborts without drops) is implemented on AmigaOS by libnix `exit()`, which releases only libnix's own stdio and malloc state; the shim keeps no record of the OS resources it acquired, so nothing can release them on the fault path.

Location: `compiler/runtime/shim_hosted_amiga.c:544-549` (`cl_trap`: `printf`, `fflush`, `exit(20)`), `:562-574` (`cl_panic`), `:640-668` (`AddIntServer` for VERTB/PORTS), at e24a8752

## Recommended fix
a shim-level resource registry: each acquire registers a release record (interrupt server, task, view, window, screen, port, device, lock, sprite, audio, library); `cl_trap`/`cl_panic` (and a libnix `atexit` hook) walk it in reverse before `exit`. The ordinary drop removes the record. Alternatively, as a minimum, remove the two interrupt servers and worker tasks in `cl_trap` — those are the crash class.

## Acceptance criteria
- The behavior the specification requires: the walker's after-line equals its before-line (source: `541-fault-handlers.expected`: exec `AddIntServer` autodoc — a server must be removed before its code and `Interrupt` structure go away; spec/05 §15 — a fault aborts without the cleanup ladder)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; the `make verify` FS-UAE legs for the touched modules (`sdk-a500-*`, `sdk-a600-*`, `sdk-a1200-*`)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, amiga-platform-reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS5-005 (fault inside a worker task); spec/05 §15 (E4) and spec question SQ-WS5-1; ASM-WS4-003 (the same fault writer). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.

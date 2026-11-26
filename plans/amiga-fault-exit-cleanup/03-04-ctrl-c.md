# Ctrl-C: Amiga fault-exit cleanup

> **Document**: 03-04-ctrl-c.md
> **Parent**: [Index](00-index.md)

## Overview

A program blocked in a breakable shim wait leaves when the user presses Ctrl-C. It prints
`***Break` and exits rc 20 through the exit hook, so OS resources are released and user drops
are skipped, as for a fault (AR-5, AR-11). The exit matches libnix's own handling of a break
during stdio. libnix `__chkabort` clears `SIGBREAKF_CTRL_C` with `SetSignal` and raises
SIGINT, whose default handler calls `Write(Output(), "***Break\n", 9)` and then `exit(20)`
(`nix/signal/raise.c:31`, `__chkabort`, verified during planning).

## Implementation Details

### The break exit

A shim-internal break routine:

1. clears `SIGBREAKF_CTRL_C` with `SetSignal(0, SIGBREAKF_CTRL_C)`, so libnix's stdio check does
   not fire a second time;
2. flushes stdout;
3. writes `***Break` followed by a newline;
4. flushes again;
5. calls `exit(20)` (AR-11).

A wait that has a device request outstanding aborts and reaps that request first (`AbortIO`,
`WaitIO`, `pending = 0`) (AR-12). A break arriving through libnix's stdio path takes libnix's
route and reaches the same hook (AR-11).

### Breakable waits

| Site | Today | New wait | AR |
|---|---|---|---|
| `cl_window_next_event` (`shim_hosted_amiga.c:2922`) | `WaitPort` | loop: check break, `GetMsg`, `Wait(port bit \| CTRL_C)` | AR-12 |
| `cl_wait_signals` (`:3061`, `waitSignals`) | `Wait(mask)` | the caller's mask includes the break bit: unchanged (`Wait(mask)`, the bit is returned). Otherwise: `Wait(mask \| CTRL_C)`, and a break exits | AR-13a |
| Task join (`:3148`) | `Wait(join bit)` | `Wait(join bit \| CTRL_C)`. A recorded worker fault wins ([03-03 §Report at join](03-03-worker-faults.md#report-at-join)). Otherwise a break exits | AR-12, AR-13c |
| `cl_device_do_io` (`:3518`, `deviceDoIO`) | `DoIO` | `SendIO`, `pending = 1`, `Wait(reply port bit \| CTRL_C)` until `CheckIO`, then `WaitIO`, `pending = 0`. On a break: abort, reap, break exit | AR-12 |
| `cl_device_wait_io` (`:3572`, `deviceWaitIO`) | `WaitIO` | `Wait(reply port bit \| CTRL_C)` until `CheckIO`, then `WaitIO`. On a break: abort, reap, break exit | AR-12 |
| `cl_timer_wait` (`:5636`) | `DoIO` | as `cl_device_do_io`. `cl_timer` gains `pending`, and `cl_timer_free` aborts and reaps a pending request before `CloseDevice` | AR-12 |
| AppMessage wait (`_20.c:255`) | `WaitPort` | loop as for window events | AR-12 |
| GadTools wait (`_optional_ui.c:303`) | `WaitPort` | loop as for window events | AR-12 |
| AmigaGuide wait (`_optional_net.c:252`) | `Wait(mask)` | `Wait(mask \| CTRL_C)` | AR-12 |

**Not breakable (documented in [03-05 §Docs](03-05-gates-and-docs.md#docs)):** these stay
unbreakable (AR-12):
- audio (`cl_audio_*` `DoIO`/`WaitIO`);
- the clipboard, serial, parallel, and printer devices (`_optional_services.c` `DoIO`);
- DOS `Delay` (`cl_delay_ticks`, `:85-87`).

### Precedence and edge rules

- A break and a pending event can arrive together. The break wins. The event stays queued and is
  released with its port at exit (AR-13c).
- A break pending at program start is not cleared; the first breakable wait exits. This matches
  libnix's stdio check (AR-13b).
- Every breakable wait in the table checks the break bit before it touches the port, so a
  queued message never hides a break (AR-13c).

### During the exit

While `cl_exiting` is set, the break routine is never entered. Breakable waits leave the break bit
out of their mask, and the release functions use the existing non-breakable `WaitIO`
([03-01 §Re-entry](03-01-exit-registry.md#re-entry)). libnix's own stdio check can still raise
SIGINT if the hook prints a worker's line during a break. That nested `exit(20)` re-enters the
idempotent hook, which drains the remaining records (AR-9 rule 1).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Break during `deviceDoIO`/`deviceWaitIO`/timer wait | abort, reap, break exit | AR-12 |
| Break while a worker fault is recorded | fault line, rc 20 | AR-13c |
| Break during the exit hook | ignored by shim waits; libnix path re-enters the idempotent hook | AR-9 rule 1 |

## Testing Requirements

- Spec: ST-15, ST-16 (FS-UAE A500, `xdotool` Ctrl-C) in [07](07-testing-strategy.md).
- Impl: each listed wait includes `SIGBREAKF_CTRL_C` except under `cl_exiting`; the
  `waitSignals` mask rule; `cl_timer_free` aborts a pending request; the unbreakable sites are
  unchanged.

# Current State: Amiga fault-exit cleanup

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

All line numbers are at HEAD `1462063e` (the assessment baseline `e24a8752` has the same shim
text for these regions). The full acquisition inventory was produced during planning; the
per-kind registration table derived from it is owned by
[03-02](03-02-registration-sites.md).

## Existing Implementation

### What Exists

- **Fault exits.** `cl_trap` (`compiler/runtime/shim_hosted_amiga.c:544-549`) prints
  `error CODE: message` with `printf`, flushes, and calls `exit(20)`; `cl_panic` (`:562-574`)
  writes `panic: …` with `fwrite` and calls `exit(20)`. The compiler emits calls from
  `codegen/guards.rs` (division, bounds, float→int), `codegen/calls.rs:25`, `codegen/expr.rs:985`,
  `codegen/function.rs:1324` (assert) and `:1407-1450` (panic), and `codegen/program.rs:4697`
  (E11215). The shim traps E1 internally at `:700`, `:816`, `:828`, `:845`, `:884`, `:899`,
  `:1057`, `:1163`. Host (`shim_host.c:39,57`) and freestanding (`shim_freestanding.c:222,241`)
  are unaffected by this plan.
- **No exit hook.** No `atexit`, destructor, or `ADD2EXIT` exists in the repository. The generated
  `int main(void)` runs the `cl_init_*` functions and returns the user `main`'s value
  (`compiler/src/codegen/program.rs:4849-4879`). Module globals have no teardown.
- **libnix exit order** (verified in the SDK during planning): exit list priorities atexit
  `0x80`, stdio `0x62`, malloc `0x4e`, libraries `0x31`, so an `atexit` hook runs while stdio, the
  malloc pool, and libnix-opened bases are alive. `__exitatexit` walks its list without unlinking,
  and a nested `exit()`/`_exit()` re-runs the exit list. libnix `__chkabort` runs on every stdio
  read/write; on `SIGBREAKF_CTRL_C` it raises SIGINT, whose default handler prints `***Break` and
  calls `exit(20)`. vbcc's `vc.lib` provides `atexit` in both the 2.0 and kick13 trees.
- **Owned handles.** 53 `freedBy` handle kinds across `stdlib/std.amiga.*.clo` and `std.io.clo`;
  the generated drop is `if (x) { deleter(x); } x = NULL;` (`compiler/src/codegen/drop.rs:34-38`).
  Some kinds are shim wrapper structs (`cl_window` `:1456`, `cl_message` `:2989`, `cl_task`
  `:3109`, `cl_device` `:3270`, video, sprite, audio, timer, font, layer, and the optional
  wrappers); others are raw OS pointers (`struct MsgPort *` from `CreatePort` `:3000`/`:3011`,
  `BPTR` from `Lock` `:229`, `struct Library *` `:3247`, `struct Screen *`).
- **Interrupt servers.** Two static `struct Interrupt` slots; `cl_add_handler` `:632`,
  `cl_add_port_handler` `:653`, `cl_remove_handler` `:675`. `HandlerToken`
  (`std.amiga.interrupt.clo:26`) has no drop, so a server the program never removes stays
  installed on every exit.
- **Worker tasks.** `cl_create_task` `:3163` (record `AllocMem`, `AllocSignal`, `CreateTask` at
  priority -1); `cl_task_entry` `:3122-3135` runs the entry, signals the parent, and parks in
  `Wait(0)`; `cl_task_join_record` `:3140-3159` waits on the join signal, then
  `Forbid`/`RemTask`/`Permit`, `FreeSignal`, `FreeMem`. `cl_task_free` `:3225` (the drop) joins.
  `cl_task_result` `:3208` dereferences `tc_UserData`. The worker contract
  (`std.amiga.exec.clo:22-26,213-242`) forbids DOS, allocation, drops, and blocking.
- **Library bases.** About 20 lazily opened bases, never closed: `shim_hosted_amiga.c:1294-1297`
  (opens `:1307`, `:1311`, `:1600-1604`, `:3682`, `:5742`, `:5859`); `_20.c:18,88,324,423`;
  `optional.c:28,853,911,914,1076` (`UtilityBase` is libnix's own symbol under GCC,
  `optional.c:17-23`); `optional_ui.c:38,46`; `optional_services.c:76,145,253,985`;
  `optional_net.c:48,162`; `optional_rtg.c:35,38`.
- **Chip-state writes.** DMACON raster-on `:4586-4587`; CLXCON `:4993`;
  `ViewPort.Modes |= SPRITES` `:4849`. None is restored.
- **Messages in flight.** `cl_put_msg` `:3046` hands a message to a port without any record;
  `cl_delete_msg_port` `:3015` deletes a port assuming it is drained. ARexx packets live in the
  static `cl_rexx_lifetimes` list (`optional_services.c:237-248`) until their reply is reclaimed.
- **Blocking waits.** `WaitPort` `:2922` (window events), `Wait` `:3061` (`waitSignals`), `:3148`
  (join), `DoIO` `:3518` (generic device), `WaitIO` `:3572`, timer `DoIO` `:5636`, `_20.c:255`
  (AppMessage), `optional_ui.c:303` (GadTools), `optional_net.c:252` (AmigaGuide); none
  includes `SIGBREAKF_CTRL_C`. `cl_device_do_io` never sets `pending` (`:3510-3523`);
  `cl_timer_free` (`:5642-5655`) has no abort path.
- **Profiles.** Both hosted profiles prepend `shim_hosted_amiga.c` (`compiler/src/profile.rs:131-137`,
  `codegen.rs:661`); only `hosted-amiga` appends the suffix files (`profile.rs:18-28,144-149`).
  Builds use `-Wall -Wextra -Werror` (`compiler/src/clorraine/toolchain.rs:120`); release builds
  use `--gc-sections` (`:93-99`).
- **Tests and gates.** `docker/scripts/run-fault-amiga.sh` runs 8 fault programs under `vamos`
  on `hosted-amiga` only and asserts the first line and rc 20 (its header says eight; `Makefile:16`
  and `AGENTS.md` say six). The assessment's `vamos` leak regex and the FS-UAE walker live under
  `plans/independent-assessment/probes/` and cannot be referenced from source. A500 gates follow
  `docker/scripts/a500-diskfont-smoke.sh` (licensed Workbench 1.3.5 HDF copy) and
  `a500-gui-smoke.sh` (`xdotool`). Impl tests pin shim text that this plan changes:
  `amiga_exec_impl_test.rs:176-237,405-451`, `amiga_workbench_impl_test.rs:427-439`,
  `amiga_intuition_impl_test.rs:619-647`.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/runtime/shim_hosted_amiga.c` | shared hosted shim | registry, hook, phases, fault/break paths, worker faults, base-shim registrations, breakable waits |
| `compiler/runtime/shim_hosted_amiga_20.c` | 2.0+ suffix | registrations, library bases, AppMessage break |
| `compiler/runtime/shim_hosted_amiga_optional*.c` | optional suffixes | registrations, library bases, GadTools/AmigaGuide break, ARexx in-flight rule |
| `stdlib/std.amiga.exec.clo`, `std.amiga.interrupt.clo`, `std.amiga.dos.clo` | doc comments | exit, fault, and break behavior |
| `docker/scripts/run-fault-amiga.sh` | `vamos` fault leg | both profiles, new cases, leak check |
| `docker/scripts/a500-fault-cleanup-smoke.sh`, `docker/scripts/fault-cleanup-walker.c` | new A500 gate | create |
| `examples/robustness/fault-cleanup/*.clo` | probe programs | create |
| `compiler/tests/amiga_fault_cleanup_spec_test.rs`, `…_impl_test.rs` | tests | create |
| `Makefile`, `AGENTS.md`, `docs/known-limitations.md`, `site/project/verification.md` | gate wiring and docs | update |

## Gaps Identified

### Gap 1: No release on the fault path
**Current Behavior:** a fault after `addHandler` leaves the VBlank server installed; the A1200
shows Software Failure 8000 0004 and the A500 Guru 00000004 when the memory is reused.
**Required Behavior:** FR-1, FR-2. **Fix Required:** [03-01](03-01-exit-registry.md).

### Gap 2: Library bases leak on every exit
**Current Behavior:** no `CloseLibrary` for any shim-opened base. **Required Behavior:** FR-1.
**Fix Required:** [03-01 §Library bases](03-01-exit-registry.md#library-bases).

### Gap 3: Worker faults run DOS and `exit()` in a plain task
**Current Behavior:** a division by zero in a worker prints and hangs the A1200.
**Required Behavior:** FR-3. **Fix Required:** [03-03](03-03-worker-faults.md).

### Gap 4: No Ctrl-C in shim waits
**Current Behavior:** a program blocked in a shim wait ignores Ctrl-C. **Required Behavior:**
FR-4. **Fix Required:** [03-04](03-04-ctrl-c.md).

## Dependencies

### Internal Dependencies
- The FFI `freedBy` mapping and generated drops (unchanged); the shim concatenation order in
  `profile.rs` (unchanged).

### External Dependencies
- libnix `atexit`/`_exit` semantics (nix13, nix20) and vbcc `vc.lib` `atexit` (build-only leg).
- FS-UAE, the licensed Kickstart 1.3 ROM, and the Workbench 1.3.5 HDF under `~/amiga` (local).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| A missed unregister point causes a double release at exit | Med | High | unregister at the real free or hand-off point (AR-9 rule 3); impl tests per kind; amiga-platform-reviewer |
| Nested `exit` during the walk re-runs libnix's exit list | Med | High | pop-before-release, walking flag, idempotent hook (AR-9 rule 1) |
| Size growth breaks the 1% bench guard | Low | Med | zero-cost rule and bench gates (AR-18) |
| Debug (`-O0`, no section GC) images grow by the registry code on the 512 KB A500 | Med | Low | measured in the closing phase; release gates unaffected |
| Changing waits shifts an existing FS-UAE gate | Low | Med | `make verify` runs every gate (AR-19) |
| libnix closes a base the shim also closes | Low | High | `cl_lib_open` records only bases this call opened (AR-9 rule 9) |

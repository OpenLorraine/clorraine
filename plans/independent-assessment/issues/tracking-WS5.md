<!-- title: [bug] Minor findings from the independent assessment: runtime shims -->
<!-- labels: assessment,kind:tracking,severity:minor,priority:P3,domain:shims,tier:advanced,tier:core,amiga-sdk -->
<!-- parent: - -->

## Summary
The independent assessment of baseline `e24a8752` left 7 surviving Minor finding(s) in the runtime shims workstream. Each line names the remediation theme that closes it; details are in `plans/independent-assessment/findings/WS5-shims.md`.

- [ ] **ASM-WS5-003** — `setCurrentDir` unlocks the launcher's current-directory lock and never restores it (effort S, verified, verdict downgraded Critical→Minor; theme {THEME:R9})
  - `setCurrentDir` unlocks the old current-directory lock. Phase 9 showed the locks stay balanced (the CLI's `CD` does the same, and libnix's startup duplicates the Workbench lock), so there is no double release; what remains is that the launching shell's directory changes after the program exits, and `vamos` reports an error at exit.
  - Location: `compiler/runtime/shim_hosted_amiga.c:448-465` (`cl_dos_set_current_dir`), at e24a8752
  - Fix: keep the original lock on the first change; never unlock it; on later changes unlock only locks the program created; at exit (normal and fault, ASM-WS5-001) `CurrentDir(original)` and unlock the program's lock.
- [ ] **ASM-WS5-011** — `FileInfoBlock`s on the stack are not guaranteed longword-aligned (effort S, probable, verdict downgraded Major→Minor; theme {THEME:R9})
  - Three DOS helpers keep a `FileInfoBlock` on the stack, which the `Examine` autodoc requires to be longword-aligned. GCC keeps the frames at multiples of four and the block fills the frame, so the misalignment is not shown; the rule is followed by accident, not by construction.
  - Location: `compiler/runtime/shim_hosted_amiga.c:320`/`:355` (`cl_dos_volumeinfo_new`), `:471`/`:481` (`cl_dos_current_dir_name`), `:497`/`:508` (`cl_dos_parent_dir_name`), at e24a8752
  - Fix: allocate the FIB with `AllocMem` (1.3) or `AllocDosObject(DOS_FIB)` (2.0+), as `cl_dos_pathinfo_new` does.
- [ ] **ASM-WS5-014** — Device buffers are not in chip memory on the 1.3 profile, which pre-V36 trackdisk requires (effort S, probable, verdict confirmed (Minor, probable); theme {THEME:R9})
  - On an A500 with trapdoor or fast RAM, disk reads land in the wrong memory and writes put garbage on the disk (data loss). The stock A500 lens is unaffected, so the severity comes from the shipped lens (Minor); for real 1.3 machines with memory expansion it is a data-loss defect.
  - Location: `compiler/runtime/shim_hosted_amiga.c:3459` (`AllocMem(length, MEMF_PUBLIC | MEMF_CLEAR)`), `:3516-3518` (a caller slice passed straight to `DoIO`), at e24a8752
  - Fix: `MEMF_CHIP` for device buffers on the 1.3 profile; bounce non-chip caller buffers when `SysBase->LibNode.lib_Version < 36`, or reject them.
- [ ] **ASM-WS5-015** — Commodity brokers are never activated (effort S, probable, verdict downgraded Major→Minor; theme {THEME:R9})
  - Commodity brokers are created inactive and never activated, and queued messages are not drained at close. The module does not expose the input-event network, so no hotkey depends on it; the broker shows as inactive in Exchange.
  - Location: `compiler/runtime/shim_hosted_amiga_optional_services.c:100-112` (`CxBroker` with `nb_Flags = 0`; no `ActivateCxObj` anywhere in `compiler/runtime/`), `:125-127` (`DeleteCxObjAll` without draining the port), at e24a8752
  - Fix: `ActivateCxObj(broker, TRUE)` after building the broker; drain and reply the port after `DeleteCxObjAll`.
- [ ] **ASM-WS5-019** — Lower-impact shim defects (grouped) (effort M, probable, verdict confirmed (Minor); theme {THEME:R9})
  - Individually limited (wrong layout or colours, a stuck layer on low memory, an ignored tag, a reply without a result code); none is shown to corrupt memory from the safe surface except *h* (DMA reads freed chip memory until the next `MrgCop`) and *o* (hypothesis).
  - Location: as listed, at e24a8752
  - Fix: per item, as in the source column; each is a local fix.
- [ ] **ASM-WS5-020** — No Ctrl-C handling in any blocking shim wait (effort M, probable, verdict confirmed (Minor, probable); theme {THEME:R1})
  - A Clorraine program started from a shell cannot be interrupted with Ctrl-C while it waits for events, a device, a timer, or a worker; the user must close its window or reboot. Owned resources are released only if the program reaches its normal exit.
  - Location: `compiler/runtime/shim_hosted_amiga.c:2922` (`WaitPort` on the IDCMP port), `:3148` (task join), `:3518`, `:3572` (`DoIO`/`WaitIO`), `:5637` (timer), `shim_hosted_amiga_20.c:255` (AppMessage wait), at e24a8752
  - Fix: OR `SIGBREAKF_CTRL_C` into the blocking waits and surface it (a `breakRequested()` query, or an event kind), so a program can exit through its normal drops.
- [ ] **ASM-WS5-021** — Host and freestanding shim edges: NULL `memcpy` with zero length, negative lengths, no `memmove` (effort S, probable, verdict confirmed (Minor); theme {THEME:R9})
  - Latent; no current program is affected.
  - Location: `compiler/runtime/shim_host.c:106-114`, `:212`, `:229` (`cl_memcpy`, `cl_buffer_append`, `cl_buffer_write`; the same in `shim_hosted_amiga.c:728-736`, `:834`, `:851`), `shim_host.c:645-654` (`Read`/`Write` cast an `int` length to `size_t`), `shim_freestanding.c:40-81` (no `memmove`), `:171-187`, `:325-328` (generic-fallback `_start`), at e24a8752
  - Fix: guard `n != 0` before `memcpy`; reject negative lengths; add `memmove` to the freestanding shim; drop `_start` from the generic fallback.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).

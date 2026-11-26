# Requirements: Amiga fault-exit cleanup

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

A Clorraine program on the hosted-amiga profiles must leave AmigaOS as it found it, however it
ends: a normal return from `main`, a runtime fault (E11210–E11213, E11215, the shim's E1), a
`panic`, or Ctrl-C. Today a fault calls libnix `exit(20)` and the shim releases nothing, so an
interrupt server stays installed in unloaded code and crashes the machine, worker tasks stay on
the ready list, public ports stay listed, and locks, chip memory, devices, and library opens
leak. A fault inside a `createTask` worker runs DOS and `exit()` from a plain Exec task and hangs
the machine. Lazily opened library bases are never closed even on a normal exit, and no shim
wait responds to Ctrl-C.

Source: theme issue #40 (remediation roadmap R1). Findings: ASM-WS5-001 (#54, DEF-582),
ASM-WS5-005 (#58, DEF-586), ASM-WS5-013 (#76, DEF-604), ASM-WS5-020 (tracking #92). No RD
upstream; this document is the owning requirements document.

## Functional Requirements

### Must Have

- [ ] **FR-1** Every OS resource the hosted-amiga shims acquire is released on every exit (AR-1,
      AR-2): owned handles through a registry walked in reverse; the two interrupt servers, the
      worker tasks, and the library bases the shim opened through fixed exit phases (AR-8); the
      shim's own chip-state writes per AR-14. Governing spec: [03-01](03-01-exit-registry.md),
      [03-02](03-02-registration-sites.md).
- [ ] **FR-2** The release runs from one `atexit` hook, so a normal return, a fault, a panic, and
      a break share it; the fault and panic lines keep their text, order, and rc 20 (AR-2,
      AR-9 rule 10).
- [ ] **FR-3** A fault or panic inside a `createTask` worker never runs DOS or `exit()` in the
      worker; the parent's `joinTask` or generated drop prints the same line and exits rc 20; a
      worker fault discovered only at exit is printed by the hook with the exit code unchanged
      (AR-3, AR-10). Governing spec: [03-03](03-03-worker-faults.md).
- [ ] **FR-4** Ctrl-C in a breakable shim wait prints `***Break` and exits rc 20 through the same
      hook; the breakable waits, `waitSignals` rule, startup rule, and precedence follow AR-5,
      AR-11, AR-12, AR-13. Governing spec: [03-04](03-04-ctrl-c.md).
- [ ] **FR-5** A message the program handed to a foreign port and that is not yet replied stays
      allocated at exit together with its reply port; messages still queued on the program's own
      ports are freed (AR-15, AR-21).
- [ ] **FR-6** Acceptance evidence: `run-fault-amiga` gains fault-after-acquire cases and the
      `vamos` leak check on both hosted profiles; a new FS-UAE gate `sdk-a500-fault-cleanup`
      (A500/KS1.3) joins `make verify` (AR-7, AR-17). Governing spec:
      [03-05](03-05-gates-and-docs.md).

### Should Have

- [ ] **FR-7** The registry's register/unregister contract is documented for the later plans
      that add records (R5 `amiga-shim-crash-safety`, R9 `amiga-shim-protocol-fixes`) (AR-16).

### Won't Have (Out of Scope)

- Faults reached in interrupt context (AR-4); tracked separately in #95.
- `setCurrentDir` restoring the launcher's directory (ASM-WS5-003, R9) and installed clip regions
  or AmigaGuide messages (ASM-WS5-006, R5) (AR-16).
- Restoring user-level `std.amiga.custom` writes (`setDma`, `setColor`) and the DMACON
  raster-on write (AR-14).
- Ctrl-C in audio, clipboard/serial/parallel/printer I/O, and DOS `Delay` (AR-12).
- A `printf`-free fault writer (ASM-WS4-003, R12) and the interrupt-server condition codes
  (ASM-WS5-009, R9).
- Any change to `spec/` (AR-6), to the host or freestanding shims, or to the Clorraine-visible
  stdlib API (AR-3, AR-5).

## Technical Requirements

### Performance

- A program that acquires no OS resource links no registry or hook code in a release build; the
  fault path adds at most one load and compare (AR-18). The closing phase records
  `make bench-compare` (empty `REQUIRE`, 1% guard) and `make bench-flags` (AR-18).

### Compatibility

- Both hosted profiles: `hosted-amiga-13` (Kickstart 1.3, libnix `nix13`, no V36 calls in the
  shared shim) and `hosted-amiga` (2.0+, suffix files). The vbcc build-only leg stays
  warning-free (AR-9 rule 2).
- The existing FS-UAE gate report lines, captures, and corpus goldens stay unchanged; the hook
  prints nothing on a clean exit (AR-9 rule 10).

### Security

- Exec safety is the security boundary here: no use of freed memory by interrupt servers,
  workers, or foreign tasks after exit (AR-8, AR-9, AR-15). Licensed Kickstart and Workbench
  media stay local; the gate decrypts the ROM under `build/` and removes it on exit (AR-17). No
  ROM names or home paths in committed files.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Release breadth | every acquisition / crash class / minimum | every acquisition | reuses the existing release functions | AR #1 |
| Exits covered | every exit / fault only | every exit via one hook | closes the library-base leak too | AR #2 |
| Mechanism | per-kind lists / simplified registry / full registry | simplified registry | smaller than per-kind; ordering-safe | AR #8 |
| Worker faults | re-raise at join / status from join | re-raise at join | no API change | AR #3, AR #10 |
| Interrupt-context faults | out / in | out | separate design | AR #4 |
| Ctrl-C | abort with cleanup / query API / defer | abort with cleanup, rc 20 | matches libnix | AR #5, AR #11–13 |
| Spec | no change / amendment | no change | runtime OS release is below the language rule | AR #6 |
| Adjacent findings | registry only / absorb R9 / absorb R5 | registry only | keeps issue #40 scope | AR #16 |

## Acceptance Criteria

1. [ ] Every ST-case in [07](07-testing-strategy.md) passes on its stated leg.
2. [ ] `make verify` passes, including the new `sdk-a500-fault-cleanup` and the extended
       `run-fault-amiga` (AR-19).
3. [ ] `make bench-compare` (empty `REQUIRE`) and `make bench-flags` pass against the plan base
       (AR-18).
4. [ ] Docs updated per AR-19; issues #54, #58, #76 and the ASM-WS5-020 line of #92 can be
       closed with the evidence.

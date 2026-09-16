# Appendix A - Amiga Platform Notes

> **Status:** planned
> **Depends on:** Ch 12 (contracts), Ch 15 (profiles)

## Scope (planned)

* API surface for v1: `exec.library` (memory, libraries, ports, signals) and
  `dos.library` (files, locks, output); later intuition/graphics behind the
  same contract discipline.
* Amiga-specific types at the boundary: `STRPTR`/`cstring`, `BPTR` file
  handles, `APTR`, `struct Library *`; all opaque in Clorraine until a
  header-importer produces typed views.
* BSTR/BCPL path conversion helpers live in the runtime shim, not in the
  language.
* Error reporting: `IoErr()`-style return codes surface as enum results in the
  wrapper layer; no exceptions.
* Hardware registers (custom chips) require `volatile` access and are written
  only inside escape blocks; the appendix lists the accepted register base
  addresses and the volatile rules.
* Interrupts and callbacks: deferred with Ch 12 `stores` contracts; the
  appendix will define the raw wrappers and the audit checklist.
* Kickstart ROM policy: the SDK image never contains ROMs; tests use the
  built-in AROS ROM, real ROMs are mounted from the host (Ch 15, SDK README).

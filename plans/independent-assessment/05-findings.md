# Findings Ledger: Independent Assessment of Clorraine

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629
> **Built**: Phase 9 (task 9.3.1) from the workstream files under `findings/`
> **Surviving findings**: 58 — Critical 12, Major 19, Minor 21, Opportunity 6
> **Verdicts**: confirmed 53, downgraded 3, upgraded 2

The detailed text of each finding stays in its workstream file; the ledger links to it.
Severity is the final severity after the Phase 9 verdict (the authored severity is shown
when it changed). DEF IDs and GitHub issue numbers are filled in Phase 10.

## Ledger

| ID | Title | Severity | Confidence | Lens | Author session | Verdict | Verifier session | Effort | DEF | Issue |
|----|-------|----------|------------|------|----------------|---------|------------------|--------|-----|-------|
| [ASM-WS1-002](findings/WS1-semantic-core.md) | Call arguments with side effects are evaluated right to left | Critical | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | M | — | — |
| [ASM-WS1-004](findings/WS1-semantic-core.md) | `clc` aborts with a stack overflow on moderately deep nesting | Critical | verified | host (the compiler process; every profile is affected) | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | M | — | — |
| [ASM-WS1-005](findings/WS1-semantic-core.md) | Polymorphic recursion with explicit type arguments hangs `clc` instead of E10305 | Critical | verified | host (the compiler process; every profile is affected) | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | S | — | — |
| [ASM-WS2-001](findings/WS2-ownership.md) | Owned temporaries drop at the end of the enclosing scope, not at the end of their statement | Critical (was Major) | verified | all (host verified; the lowering is profile-independent) | ses_dec1f5b4dffe | upgraded Major→Critical | ses_dea9a612affe | L | — | — |
| [ASM-WS3-001](findings/WS3-codegen.md) | Locals named like C keywords or C/NDK macros make the emitted C invalid | Critical | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | S | — | — |
| [ASM-WS3-002](findings/WS3-codegen.md) | A module function can collide with a generic instance symbol | Critical | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | S | — | — |
| [ASM-WS5-001](findings/WS5-shims.md) | A runtime fault exits with OS registrations still installed: a VBlank server is left pointing into the unloaded program | Critical | verified | A500, A1200 shipped (shared code) | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | L | — | — |
| [ASM-WS5-002](findings/WS5-shims.md) | `savePublicScreen` and `showAlert` call Intuition through a NULL `IntuitionBase` and crash the machine | Critical | verified | A1200 shipped (`savePublicScreen`); A500 and A1200 (`showAle | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-004](findings/WS5-shims.md) | `drawText` writes unclipped text through a screen `RastPort` that has no layer | Critical | verified | A500, A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-005](findings/WS5-shims.md) | A runtime fault inside a worker task runs libnix `printf` and `exit()` from a plain Exec task | Critical | verified | A500, A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | M | — | — |
| [ASM-WS5-006](findings/WS5-shims.md) | Safe handles let a borrowed OS object outlive its owner: clip regions and AmigaGuide messages | Critical | probable | A1200 shipped (`hosted-amiga` modules) | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | M | — | — |
| [ASM-WS5-016](findings/WS5-shims.md) | The clipboard read never finishes the read protocol | Critical (was Major) | verified | A1200 shipped | ses_dec1f5b4dffe | upgraded Major→Critical | ses_dea9a6107ffe | S | — | — |
| [ASM-WS1-001](findings/WS1-semantic-core.md) | An inferred-type decimal literal above `i32` range is accepted and silently wraps | Major | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | S | — | — |
| [ASM-WS3-003](findings/WS3-codegen.md) | A value-position `++`/`--` emits signed C increment, undefined on the host and vbcc legs at the limit | Major | verified | host; vbcc leg (both build without `-fwrapv`); the m68k GCC  | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | S | — | — |
| [ASM-WS3-007](findings/WS3-codegen.md) | vbcc compile time grows about cubically with `switch` size | Major | verified | vbcc leg of hosted-amiga (build-only validity leg) | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | M | — | — |
| [ASM-WS3-008](findings/WS3-codegen.md) | An `else if` chain of 127 or more arms makes vbcc crash or reject the emitted C | Major | verified | vbcc leg of hosted-amiga (`vc +aos68k -c99 -O2 -cpu=68000`); | ses_dea9a6112ffe | confirmed | ses_dea685c4bffe | S | — | — |
| [ASM-WS4-001](findings/WS4-code-quality.md) | Templates cost 33% more instructions than equivalent C on the A500 | Major | verified | A500 shipped -O2 (also A500 -Os +19.4%, A1200 shipped +51.3% | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | M | — | — |
| [ASM-WS4-002](findings/WS4-code-quality.md) | `Array<T>` push and get cost 39% more instructions than equivalent C on the A500 | Major | verified | A500 shipped -O2 (also A500 -Os +57.8%, A1200 shipped +38.8% | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | M | — | — |
| [ASM-WS5-007](findings/WS5-shims.md) | `cardPresent()` and `cardStatus()` remove `card.resource` from the system | Major | verified | A1200 shipped (A600/A1200 PCMCIA) | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-008](findings/WS5-shims.md) | The screen capture misreads the 2.0+ colour table, and the A1200 Workbench golden pins the wrong colours | Major | verified | A1200 shipped (every 2.0+ capture); A500 unaffected (`Type = | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-009](findings/WS5-shims.md) | The interrupt servers return with the user handler's condition codes and can end the VERTB chain | Major | verified | A500, A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-010](findings/WS5-shims.md) | The busy-pointer image has no sprite header or terminator, so sprite DMA reads past the 64-byte allocation | Major | probable | A500, A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-012](findings/WS5-shims.md) | GadTools: the last gadget is never attached, rendered, or freed | Major | probable | A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-013](findings/WS5-shims.md) | The shim's own library bases are never closed | Major | probable | A500, A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-017](findings/WS5-shims.md) | Layer drawing returns without `WaitBlit`, so CPU access to the same bitmap races the blitter | Major | probable | A500, A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-018](findings/WS5-shims.md) | Classic menus are never laid out: every menu header and item sits at the same position | Major | probable | A500, A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-023](findings/WS5-shims.md) | `clipboardWrite` reports success for raw data that the 3.1 clipboard device discards, so `clipboardRead` never sees it | Major | verified | A1200 shipped (`hosted-amiga`, `std.amiga.clipboard`); other | ses_dea9a6107ffe | confirmed | ses_dea685c4bffe | S | — | — |
| [ASM-WS6-001](findings/WS6-stdlib.md) | `readFile` reports a read error exactly like end of file | Major | verified | host (reproduced); A500, A1200 shipped (the same `fread` cod | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | S | — | — |
| [ASM-WS6-002](findings/WS6-stdlib.md) | `slice` of a `str` is rejected, although the appendix gives it the same view type | Major | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | M | — | — |
| [ASM-WS7-001](findings/WS7-performance.md) | A module-level fill emits one assignment per element: a 64 KB table needs 548 KB of code and 163 s of GCC | Major | verified | A500 (measured); A1200 shipped and host (the same emission) | ses_dec1f5b4dffe | confirmed | ses_dea685c4bffe | S | — | — |
| [ASM-WS7-002](findings/WS7-performance.md) | Compile time grows faster than quadratically for call-heavy and generic code | Major | verified | host (`clc` itself) | ses_dec1f5b4dffe | confirmed | ses_dea685c4bffe | M | — | — |
| [ASM-WS0-001](findings/WS0-baseline.md) | The flag audit's host binary floor fails on `lookup` at the baseline | Minor | verified | host | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | S | — | — |
| [ASM-WS1-003](findings/WS1-semantic-core.md) | Data inclusion (`embed`) is staged out, but chapter 13 reads stable and the limitation is undocumented | Minor | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | M | — | — |
| [ASM-WS2-002](findings/WS2-ownership.md) | A field move out of a call result is accepted | Minor | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a612affe | S | — | — |
| [ASM-WS2-003](findings/WS2-ownership.md) | Move, copy, and drop decisions are re-derived at about 240 sites in two stages with different predicates | Minor | probable | all | ses_dec1f5b4dffe | confirmed | ses_dea9a612affe | XL | — | — |
| [ASM-WS3-004](findings/WS3-codegen.md) | On the host profile `usize` is 32 bits, so a pointer round trip through `usize` crashes | Minor | verified | host only (the Amiga profiles are 32-bit and pass) | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | M | — | — |
| [ASM-WS3-005](findings/WS3-codegen.md) | Appended runtime code inherits the last `#line`, so its diagnostics and debug lines point into `.clo` files | Minor | verified | all (visible on the Amiga profiles, whose shim suffix follow | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | S | — | — |
| [ASM-WS3-006](findings/WS3-codegen.md) | The emitted C depends on how the source path is spelled and embeds absolute paths | Minor | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | S | — | — |
| [ASM-WS5-003](findings/WS5-shims.md) | `setCurrentDir` unlocks the launcher's current-directory lock and never restores it | Minor (was Critical) | verified | A500, A1200 shipped | ses_dec1f5b4dffe | downgraded Critical→Minor | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-011](findings/WS5-shims.md) | `FileInfoBlock`s on the stack are not guaranteed longword-aligned | Minor (was Major) | probable | A500, A1200 shipped | ses_dec1f5b4dffe | downgraded Major→Minor | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-014](findings/WS5-shims.md) | Device buffers are not in chip memory on the 1.3 profile, which pre-V36 trackdisk requires | Minor | probable | A500 with expansion memory (not the stock 512 KB A500 lens,  | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-015](findings/WS5-shims.md) | Commodity brokers are never activated | Minor (was Major) | probable | A1200 shipped | ses_dec1f5b4dffe | downgraded Major→Minor | ses_dea9a6107ffe | S | — | — |
| [ASM-WS5-019](findings/WS5-shims.md) | Lower-impact shim defects (grouped) | Minor | probable | as listed | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | M (all items) | — | — |
| [ASM-WS5-020](findings/WS5-shims.md) | No Ctrl-C handling in any blocking shim wait | Minor | probable | A500, A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | M | — | — |
| [ASM-WS5-021](findings/WS5-shims.md) | Host and freestanding shim edges: NULL `memcpy` with zero length, negative lengths, no `memmove` | Minor | probable | host; freestanding (provisional profile) | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS6-003](findings/WS6-stdlib.md) | NaN prints as `-nan` on the host and `NaN` on the Amiga profiles | Minor | verified | host vs. A500/A1200 shipped | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | S | — | — |
| [ASM-WS6-004](findings/WS6-stdlib.md) | 25 public functions, including `std.io`'s writers, have no specification entry | Minor | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | M | — | — |
| [ASM-WS6-005](findings/WS6-stdlib.md) | The appendix's Kickstart minimums and Box signatures drift from the code | Minor | verified | A1200 shipped (the 2.0 modules); all (spelling) | ses_dec1f5b4dffe | confirmed | ses_dea9a6131ffe | S | — | — |
| [ASM-WS8-001](findings/WS8-test-integrity.md) | The fuzzer cannot generate the constructs where the defects are | Minor | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea685c4bffe | M | — | — |
| [ASM-WS8-002](findings/WS8-test-integrity.md) | Tests and gates assert weaker properties than the behavior they cover | Minor | verified | all; A500/A1200 gates | ses_dec1f5b4dffe | confirmed | ses_dea685c4bffe | M | — | — |
| [ASM-WS8-003](findings/WS8-test-integrity.md) | Memory-safety and invalid-C defects recur because drop and emission decisions are made per expression shape, in two stages | Minor | verified | all | ses_dec1f5b4dffe | confirmed | ses_dea685c4bffe | XL | — | — |
| [ASM-WS8-004](findings/WS8-test-integrity.md) | The Amiga shims never run under a sanitizer, and UBSan covers a third of the ASan suites | Minor | verified | host (sanitizer legs); A500/A1200 (the unsanitized shims) | ses_dec1f5b4dffe | confirmed | ses_dea685c4bffe | M | — | — |
| [ASM-WS4-003](findings/WS4-code-quality.md) | The runtime fault writer pulls 16.6 KB of `printf` and soft-float code into every checked program | Opportunity | verified | A500 shipped -O2 (every Amiga lens links the same code) | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | S | — | — |
| [ASM-WS4-004](findings/WS4-code-quality.md) | A module-level table with a constant initializer occupies RAM twice | Opportunity | verified | A500 shipped -O2 | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | S | — | — |
| [ASM-WS4-005](findings/WS4-code-quality.md) | Every Amiga program links libamiga DOS stubs and `CreatePort` it does not use | Opportunity | verified | A500 shipped -O2 | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | M | — | — |
| [ASM-WS4-006](findings/WS4-code-quality.md) | `-O3` and `-flto` beat the shipped release flags on the A500 | Opportunity | verified | A500 (hosted-amiga-13, nix13, -m68000) and the 68020 probe b | ses_dec1f5b4dffe | confirmed | ses_dea9a6112ffe | S | — | — |
| [ASM-WS5-022](findings/WS5-shims.md) | Integer formatting divides with libgcc's 32-bit routine on the 68000 | Opportunity | verified | A500 | ses_dec1f5b4dffe | confirmed | ses_dea9a6107ffe | S | — | — |
| [ASM-WS7-003](findings/WS7-performance.md) | The 68020 probe build of `container` runs 21% more instructions than the 68000 build | Opportunity | verified | A1200 68020 probe build (`-m68020`, `vamos -C 68020`) — a 68 | ses_dec1f5b4dffe | confirmed | ses_dea685c4bffe | S | — | — |

## Clean sample

Fifteen `clean` outcomes were re-checked, at least two per workstream that has two (WS0 has
none; WS8 has one). Reports: `verification/verdicts-9.2.*.md`.

| Clean outcome | Workstream file | Verdict | Verifier session | Note |
|---------------|-----------------|---------|------------------|------|
| WS1-05 | WS1-semantic-core.md | confirmed | ses_dea9a6131ffesU3qOxzrQQoINm | probes 039–047, 45/45 |
| WS1-07 | WS1-semantic-core.md | confirmed | ses_dea9a6131ffesU3qOxzrQQoINm | probes 050–052 and their seeded contrasts |
| WS2-01 | WS2-ownership.md | confirmed | ses_dea9a612affe7B68nL0nJN0bQn | probes 201–209 under ASan/UBSan |
| WS2-04 | WS2-ownership.md | confirmed | ses_dea9a612affe7B68nL0nJN0bQn | probes 221–225 |
| WS3-06 | WS3-codegen.md | confirmed (evidence record defective, repaired) | ses_dea9a6112ffeMR984F2rpPTplg | `records/WS3-mmio.log` was empty; regenerated with `main` |
| WS3-08 | WS3-codegen.md | confirmed | ses_dea9a6112ffeMR984F2rpPTplg | probes 350–358, 72/72 |
| WS4-03 | WS4-code-quality.md | confirmed | ses_dea9a6112ffeMR984F2rpPTplg | patch 410 +13.0% |
| WS4-04 | WS4-code-quality.md | confirmed | ses_dea9a6112ffeMR984F2rpPTplg | patch 415 |
| WS5-14 | WS5-shims.md | confirmed | ses_dea9a6107ffefWXd6gFWNJQszZ | host vs. Amiga runtime core diff |
| WS5-03 (size pairing only) | WS5-shims.md | confirmed | ses_dea9a6107ffefWXd6gFWNJQszZ | every `AllocMem`/`FreeMem` size pair |
| WS6-03 | WS6-stdlib.md | confirmed | ses_dea9a6131ffesU3qOxzrQQoINm | 620 ratios 1.98–2.03; seeded 4.00 |
| WS6-07 | WS6-stdlib.md | confirmed | ses_dea9a6131ffesU3qOxzrQQoINm | 670 and its seed |
| WS7-04 | WS7-performance.md | confirmed | ses_dea685c4bffegV12VLXpl04mC6 | every `MEMF_CHIP` site |
| WS7-07 | WS7-performance.md | confirmed | ses_dea685c4bffegV12VLXpl04mC6 | RSS reproduces to the KB |
| WS8-06 | WS8-test-integrity.md | confirmed (count note) | ses_dea685c4bffegV12VLXpl04mC6 | 773 tests carry some text pin |


## Merges

No full merge. One partial: ASM-WS8-003 is narrowed to its causes (2)–(4) (invalid C from
string-composed emission, route-specific checks, unsequenced effects); its cause (1), the
second drop decision in codegen, is ASM-WS2-003 and is counted there only (verifier
ses_dea685c4bffegV12VLXpl04mC6, 9.2.5).

## Refuted

None.

## Spec questions

From `findings/WS1-semantic-core.md`:

- **SQ-WS1-1 — withdrawn in task 1.3.1.** The module-level `let embed` contradiction between
  spec/13 §1 and spec/04 §16 was already ruled by the user on 2026-10-24
  (`plans/def-critical-major-closeout/00-ambiguity-register.md` AR-23: follow spec/04 §16)
  and recorded there as a frozen inconsistency; it is not re-raised.
- **SQ-WS1-2 — May a hex or binary literal use the full unsigned width in a signed context?**
  spec/01 §7 and spec/02 §9 say a hex or binary literal "may use the full unsigned width of
  the target" (`0xFFFFFFFF` is a valid `u32`). `let a: i32 = 0x80000000;` is rejected with
  E10202 (`build/tmp/assessment/WS1/diag/v.clo`). The text does not say whether the
  full-width allowance is a bit-pattern rule for signed contexts too.
- **SQ-WS1-3 — Is the opt-in `-m68020` / `-m68881` leg a profile requirement?**
  spec/15-profiles.md §2 lists `hosted-amiga` "Default CPU `-m68000` (opt-in `-m68020`)" and
  an "FPU build leg: opt-in `-m68881`". No compiler or driver option offers either (`grep
  -rn "68020\|68881" compiler/src` finds nothing at e24a8752; the flag tables in
  `compiler/src/clorraine/toolchain.rs:58-63` fix `-m68000`). Either the opt-in is a
  missing feature or the table describes flags a user adds by hand. Input to WS7-03.

From `findings/WS2-ownership.md`:

- **SQ-WS2-1 — When does a temporary created in an `if`/`while` condition drop?** spec/04 §3
  says temporaries drop "at the end of that statement", but an `if` or `while` statement
  contains its body; the text does not say whether a condition temporary drops after the
  condition or after the whole statement (and, for `while`, per iteration). Probes 219 and
  228 avoid depending on the answer.
- **SQ-WS2-2 — Is returning a capturing closure E10404 or E10400?** spec/06 §6.1 says
  "Storing, returning, or capturing a closure is E10404 (L-6)" and "a capturing lambda used
  where a plain `func(...)` value is required is E10400" (L-7). A return of type
  `func(): i32` matches both; `clc` reports E10400 (probe 257).
- **SQ-WS2-3 — Are two-phase borrows part of the model?** The WS2-06 checklist names them,
  but spec/16 does not define them (for example `xs.push(xs.len())`, an exclusive receiver
  lend with a read in the argument). Not probed.

From `findings/WS5-shims.md`:

- **SQ-WS5-1** — spec/05 §15 says runtime faults abort without the cleanup ladder (E4).
  On AmigaOS, aborting without releasing OS registrations crashes the machine
  (ASM-WS5-001). Should the spec require the hosted profiles' fault exit to release
  runtime-owned OS resources (interrupt servers, tasks, display, ports, locks), while
  still skipping user drops? ASM-WS5-001 stands as a platform defect either way.
- **SQ-WS5-2** — `std.amiga.regions` documents a lifetime rule ("clear the clip before
  freeing the region") that the safe surface does not enforce (ASM-WS5-006). Does
  spec/17 §10 ("memory safety is strong above the hardware boundary") allow a safe
  stdlib export to rely on a documented ordering rule?

From `findings/WS6-stdlib.md`:

- **SQ-WS6-1** — The text of NaN and infinity in templates and `text()` is unspecified
  (S2a fixes only the precision). Which spelling, and is a NaN's sign shown? (ASM-WS6-003)
- **SQ-WS6-2** — spec/appendix-stdlib.md §4 says library wrappers return `Status` and
  partial IO returns a count plus a `Status`, but no wrapper does: the appendix's own
  tables give `bool`, `T?`, and −1 counts. Which is normative — the §4 convention or the
  tables? (ASM-WS6-001 depends on the answer for its fix shape.)
- **SQ-WS6-3** — If `slice` of a `str` becomes legal (ASM-WS6-002), `toCString(s)` of a
  sub-view must not return the view's pointer unterminated. Should `toCString` require a
  NUL-terminated origin (and reject sub-views), or copy?

## Escalations

### Escalation outcomes

| ID | Escalated as | Phase 9 verdict |
|----|--------------|-----------------|
| ASM-WS1-002 | Critical (verified) | confirmed |
| ASM-WS1-004 | Critical (verified) | confirmed |
| ASM-WS1-005 | Critical (verified) | confirmed |
| ASM-WS3-001 | Critical (verified; evidence corrected in task 8.2.1) | confirmed |
| ASM-WS3-002 | Critical (verified) | confirmed |
| ASM-WS5-001 | Critical (verified) | confirmed; the crash reproduced on A1200/KS3.1 and A500/KS1.3 |
| ASM-WS5-002 | Critical (verified) | confirmed; also reproduced on A500/KS1.3 (`showAlert`) |
| ASM-WS5-003 | Critical (verified) | **downgraded Critical→Minor**; escalation retracted and the user told |
| ASM-WS5-004 | Critical (probable) | confirmed; now verified |
| ASM-WS5-005 | Critical (probable) | confirmed; now verified |
| ASM-WS5-006 | Critical (probable) | confirmed (probable) |
| ASM-WS2-001 | — (Major) | **upgraded Major→Critical** in Phase 9; escalated |
| ASM-WS5-016 | — (Major) | **upgraded Major→Critical** in Phase 9; escalated |

### From `findings/WS1-semantic-core.md`

| Time | ID | Message sent to the user |
|------|----|--------------------------|
| 2026-11-25 20:22 | ASM-WS1-002 | "Critical (verified): call arguments with side effects are evaluated right to left on every profile — `pair(next(), next())` prints `2 1`, spec/04 §3 requires `1 2`. Reproduce: `make -C docker exec CMD=\"python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS1 --clc build/tmp/assessment/baseline/clc --only 021 066\"`." |
| 2026-11-25 20:22 | ASM-WS1-004 | "Critical (verified): `clc` aborts with a stack overflow on 758 nested calls, a 1,172-term `+` chain, 1,378 nested blocks, or 4,310 `else if` arms. Reproduce: `make -C docker exec CMD=\"python3 plans/independent-assessment/probes/WS1/065-robustness.py build/tmp/assessment/baseline/clc plans/independent-assessment/findings/records/assessment-baseline-compile.json build/tmp/assessment/WS1/robustness\"`." |
| 2026-11-25 20:22 | ASM-WS1-005 | "Critical (verified): a 288-byte generic program with explicit polymorphic recursion (`g<W<T>>(…)`) hangs `clc` (no exit in 600 s) instead of E10305. Reproduce: the same `065-robustness.py` command (case `generic-recursion`)." |

No fix is made (AR #12).

### From `findings/WS2-ownership.md`

| Time | ID | Message sent to the user |
|------|----|--------------------------|
| 2026-11-26 02:16 | ASM-WS2-001 (Phase 9 upgrade) | "Critical (Phase 9 verdict, verifier ses_dea9a612affe7B68nL0nJN0bQn): an owned temporary is dropped at function exit, not at the end of its statement; a `File` written through a temporary is not yet flushed or closed when the same function reopens it, so the read returns 0 bytes on host and hosted-amiga. Reproduce: `make -C docker exec CMD=\"python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS2 --clc build/tmp/assessment/baseline/clc --sanitize --only 260\"`." |

### From `findings/WS3-codegen.md`

| Time | ID | Message sent to the user |
|------|----|--------------------------|
| 2026-11-25 21:35 | ASM-WS3-001 | "Critical (verified): a local named like a C keyword or C/NDK macro (`char`, `int`, `long`, `NULL`, `EOF`, `errno`, `stdout`, …) makes the emitted C invalid on every profile. Reproduce: `make -C docker exec CMD=\"python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --only 340 341\"`." |
| 2026-11-26 02:03 | ASM-WS3-001 (correction) | "Correction to the 21:35 message: `char`, `int`, and `long` are escaped (`cl_char`, `cl_int`, `cl_long`); the names that make the C invalid are the C99 keywords `restrict` and `_Bool` and header/NDK macros (`NULL`, `EOF`, `errno`, `stdout`, `TRUE`, …). Severity unchanged (Critical, verified)." |
| 2026-11-25 21:35 | ASM-WS3-002 | "Critical (verified): module `get`'s function `_Point` and the instance `get<Point>` both mangle to `cl_get__Point`, so the emitted C fails with conflicting types. Reproduce: the same command with `--only 342`." |

No fix is made (AR #12).

### From `findings/WS5-shims.md`

- **2026-11-25 22:50** — "ASM-WS5-001 (Critical, verified): a runtime fault exits with
  OS registrations installed; on A1200/KS3.1 the VBlank server chain keeps a server whose
  code and node are in the unloaded program (vertb 4 → 5; no-fault contrast stays 4). Any
  later program loaded there turns it into a machine crash. Reproduce:
  `make -C docker exec CMD="bash plans/independent-assessment/probes/WS5/54x-fs-uae.sh build build/tmp/assessment/baseline/clc build/tmp/assessment/WS5/fsuae"`
  then `bash plans/independent-assessment/probes/WS5/54x-fs-uae.sh run build/tmp/assessment/WS5/fsuae 541 541c`."
- **2026-11-25 22:50** — "ASM-WS5-002 (Critical, verified): `savePublicScreen` (and
  `showAlert`) before any screen or window calls Intuition through a NULL base; on
  A1200/KS3.1 the machine crashes (no END; with a screen opened first the call returns
  false). Reproduce: the build command above, then `… run build/tmp/assessment/WS5/fsuae 542 542c`."
- **2026-11-25 22:50** — "ASM-WS5-003 (Critical, verified): `setCurrentDir` unlocks the
  launcher's current-directory lock — a double release of a filesystem lock. Reproduce:
  `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS5 --clc build/tmp/assessment/baseline/clc --vamos-leaks --only 501"`."
- **2026-11-25 22:50** — "ASM-WS5-004, -005, -006 (Critical, probable): unclipped
  `drawText` on a screen writes outside the bitmap; a fault in a worker task runs DOS
  and `exit()` from a plain task; a freed clip region or a closed AmigaGuide context can
  be used after free from safe code. Code reading; locations in the findings."
- **2026-11-26 02:43** — Phase 9 verdict ASM-WS5-016 upgraded Major→Critical (verifier ses_dea9a6107ffefWXd6gFWNJQszZ): "On A1200/KS3.1 a `clipboardWrite` after `clipboardRead` on the same handle never returns — the machine's clipboard user hangs. Reproduce: `probes/WS5/verify-9.2.4/clipm.clo` with `probes/WS5/verify-9.2.4/run-a1200.sh` (Workbench 3.1 clipboard.device staged by `stage-wb.sh`)."
- **2026-11-26 02:43** — Phase 9 verdict ASM-WS5-003 downgraded Critical→Minor (verifier ses_dea9a6107ffefWXd6gFWNJQszZ); the 2026-11-25 22:50 escalation is retracted: "the double release is refuted — libnix and the CLI balance the current-directory locks; what remains is that the launching shell's directory changes."
- **2026-11-26 02:43** — Phase 9: ASM-WS5-004 and ASM-WS5-005 confirmed Critical and now `verified` (hangs on A1200/KS3.1); ASM-WS5-001's crash reproduced on A1200 and A500.

## Phase 9 gate (9.3.2)

Session IDs in the ledger are shortened to 16 characters; the full IDs are in each
finding's Verdict line and in `verification/verdicts-9.2.*.md`.

| Case | Result |
|------|--------|
| Task verify | drift check PASS |
| ST-1 | 58 findings, every template field present in the workstream files |
| ST-2 | every `verified` finding names a committed probe or record and a command; verifier reproductions that raised a confidence are committed (`probes/WS2/260`, `probes/WS5/verify-9.2.4/`, `probes/WS3/verify-9.2.3/`) |
| ST-3 | `hypothesis` appears only inside Minor findings (ASM-WS5-019, -021) |
| ST-4 | performance Majors state both sides on the shipped 68000 lens (ASM-WS4-001, -002, ASM-WS7-001) or the compile-time 10x rule (ASM-WS3-007, ASM-WS7-002) |
| ST-5 | every probe's expectation predates its run; the verifier-derived probe 260 was committed before the author's own run and is marked as derived |
| ST-6 | no file under `spec/` differs from the baseline; spec questions listed above |
| ST-7 | drift check PASS; every commit touched only `plans/independent-assessment/` and the roadmap row |
| ST-8 | every workstream's checklist IDs have one outcome (re-checked; WS3 gained its missing Spec questions section) |
| ST-9 | 58 findings and 15 sampled cleans each have one verdict from a verifier session different from the author (the two Phase 9 findings were verified by 9.2.5, not by the sessions that raised them) |
| ST-11 | every Critical escalated; the Phase 9 upgrades (ASM-WS2-001, ASM-WS5-016) and the downgrade (ASM-WS5-003) recorded and told to the user |
| ST-16 | dispatch and reasoning lines are medium, high, or xhigh |

## Report review

_(filled in Phase 10, task 10.3.1)_

# Execution Plan: Amiga SDK Bundle 9 — Optional Add-ons and Advanced Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Progress**: 6/30 tasks (20%) — Executing
> **Created**: 2026-11-06 02:09
> **Last Updated**: 2026-11-06 02:51
> **CodeOps Artifact Schema**: 1

## How to Execute

Work phase by phase in the mandatory order: specification tests → implementation → implementation
tests and hardening. Every task updates this checklist immediately. Each phase adds its appendix
section before its appendix check, then runs the targeted suites and `make check`; runtime gates
run in Phase 6. Commit with the git-commit skill after each verified phase. Do not skip a failing
spec test or weaken a gate pin.

## Phase 0: Foundation — optional suffix, profile, safety guard

> **Reasoning**: high — the suffix placement and the clipboard guard affect every hosted-amiga
> translation unit and a latent memory-safety path.

- [x] 0.1.1 [spec-author] Add the optional-suffix/profile rejection and clipboard extended-device cases to `compiler/tests/amiga_optional_spec_test.rs`; run red ✅ (completed: 2026-11-06 02:51; the suffix, order, and clipboard-guard cases are green)
- [x] 0.1.2 Add `compiler/runtime/shim_hosted_amiga_optional.c`, concatenate it after the 2.0 suffix in `Profile::shim_suffix`, and add `clipboard.device` to `cl_device_is_extended`; run the spec green ✅ (completed: 2026-11-06 02:51; 404 lib + 7 optional spec green)
- [x] 0.1.3 Add impl pins for suffix order, 1.3/2.0 isolation, and the clipboard guard; scope the existing intuition 1.3 forbidden-call test to the shared shim; run `make check` ✅ (completed: 2026-11-06 02:51; optional impl and intuition suites green)

## Phase 1: Exec and utility

> **Reasoning**: high — semaphores, stored callbacks, and refcounted OS pools introduce new
> ownership and lifetime rules.

- [x] 1.1.1 [spec-author] Add the semaphore/utility/pool surfaces, host/1.3/freestanding rejection, `nix20` cross-build, and the `spec/appendix-stdlib.md` sections; run red ✅ (completed: 2026-11-06 02:51; §14.4–§14.6 added)
- [x] 1.1.2 Add `stdlib/std.amiga.semaphore.clo`, `utility.clo`, and `pool.clo` with the optional-suffix C, module/export tables, and profile tests; run the spec green ✅ (completed: 2026-11-06 02:51; semaphore, tags/date/hooks, refcounted pool)
- [x] 1.1.3 Add ownership/drop/hook/trampoline/refcount/read-write impl pins and `examples/amiga/optional-exec20.clo`/`optional-pool.clo`; run targeted tests ✅ (completed: 2026-11-06 02:51; 5 impl tests + both demos build)

## Phase 2: DOS, graphics, and input

> **Reasoning**: high — ReadArgs, datatypes, regions, EClock, and lowlevel add five new FFI
> surfaces, including template parsing and a dedicated EClock unit.

- [ ] 2.1.1 [spec-author] Add the args/datatypes/regions/eclock/lowlevel surfaces, rejection cases, `nix20` cross-builds, and appendix sections; run red
- [ ] 2.1.2 Add the five modules with their optional-suffix C and module/export tables; run the spec green
- [ ] 2.1.3 Add impl pins for template kinds, `PDTA_*`/`dtDraw`, region borrow/bounds, EClock hi/lo/frequency, lowlevel type decode, and `optional-args.clo`/`optional-lowlevel.clo`; run targeted tests

## Phase 3: Intuition advanced

> **Reasoning**: high — gadtools/ASL/BOOPSI add object trees, modal input, and native class
> attributes, plus a shared event-id fallback.

- [ ] 3.1.1 [spec-author] Add the gadtools/asl/boopsi surfaces, rejection cases, `nix20` cross-builds, and appendix sections; run red
- [ ] 3.1.2 Add the three modules with their optional-suffix C; add the shared `FirstGadget`/`GadgetID` fallback; run the spec green
- [ ] 3.1.3 Add impl pins for `CreateContext`/`CreateGadgetA`, ASL drawer/file split, colorwheel screen/hue, and `optional-intuition.clo`/`optional-boopsi.clo`; run targeted tests

## Phase 4: Workbench services

> **Reasoning**: high — six disk-based services add library/device bases, message ownership, and
> extended device requests.

- [ ] 4.1.1 [spec-author] Add the commodities/locale/arexx/clipboard/devices/nonvolatile surfaces, rejection cases, `nix20` cross-builds, and appendix sections; run red
- [ ] 4.1.2 Add the six modules with their optional-suffix C, base globals, and module/export tables; run the spec green
- [ ] 4.1.3 Add impl pins for `NB_VERSION`/port lifetime, locale fields, ARexx `ClearRexxMsg`/reclaim, clipboard update/read bound, SETPARAMS/PRD_RAWWRITE, nonvolatile tens length, and `optional-services.clo`/`optional-nonvolatile.clo`; run targeted tests

## Phase 5: Network and AmigaGuide

> **Reasoning**: high — socket fd ownership and async AmigaGuide messages add external and
> callback-style lifetimes.

- [ ] 5.1.1 [spec-author] Add the bsdsocket/amigaguide surfaces, rejection cases, `nix20` cross-builds, and appendix sections; run red
- [ ] 5.1.2 Add the two modules with their optional-suffix C and module/export tables; run the spec green
- [ ] 5.1.3 Add impl pins for socket close/`127.0.0.1`, guide struct/copies/message result, the committed `.guide`, and `optional-network.clo`/`optional-guide.clo`; run targeted tests

## Phase 6: Demos, media, and gates

> **Reasoning**: high — this is the runtime proof for every row and the first use of the licensed
> Workbench media in the gates.

- [ ] 6.1.1 Add `docker/scripts/workbench-media.sh` and `sdk-a500-diskfont` with the 1.3.4 ADF DF1 mount, `DF1:C/Assign`, `diamond.font` 12, writable boot ADF, and post-stop read; run the gate
- [ ] 6.1.2 Add `sdk-a600-addons` with the 2.04/2.1 media, exec/args/services demos, serial/parallel peers, printer open/close, and full-line assertions; run the gate
- [ ] 6.1.3 Add the intuition mode to `sdk-a600-addons` with Xvfb/xdotool, gadtools/ASL handshakes, and the `a600-intuition.png` golden; run the gate
- [ ] 6.1.4 Add `sdk-a600-ks31-addons` with the 3.1 media, lowlevel/nonvolatile demos, and `ENV:`/`ENVARC:`; run the gate
- [ ] 6.1.5 Add `sdk-a1200-ks30-addons` with the 3.0 media/classes, pool/BOOPSI demos, and the `a1200-ks30-boopsi.png` golden; run the gate
- [ ] 6.1.6 Add `sdk-a1200-addons` with the 3.1 media, `AddDataTypes`, Workbench start, datatypes/network/guide demos, and the host echo server; run the gate
- [ ] 6.1.7 Add all five targets to `Makefile` and `verify`; run every new gate and `make check`; fix regressions

## Phase 7: Documentation, registry, review, closeout

> **Reasoning**: high — registry truth, frozen appendix additions, independent review, and the
> final full acceptance gate.

- [ ] 7.1.1 Update `spec/appendix-stdlib.md`, `spec/appendix-amiga.md`, `docs/known-limitations.md`, and `AGENTS.md`; leave `spec/15-profiles.md` untouched; run the appendix section checks
- [ ] 7.1.2 Reassign the 20 open rows to `9-optional`; correct `SDK-EXE-007` 3.0, `SDK-DOS-006` 3.1, `SDK-AUD-006`/`SDK-WBK-010` 3.1, and `SDK-WBK-003` depends; set Bundle 9 floor 1.3/OCS; update `bundles.tsv`/issue #10/coverage/programme index/roadmap; run `make sdk-registry`
- [ ] 7.1.3 Run `make check`, `make verify`, and `make sdk-registry`; fix every regression
- [ ] 7.1.4 Run the independent correctness, memory-safety (static + host models + m68k gates), and emission-integrity reviews; fix every critical/major finding; re-run the affected gates
- [ ] 7.1.5 Clean temporary artifacts and decrypted ROMs, commit with the git-commit skill, mark the plan Done, and close issue #10

## Verification

`make check`, `make verify`, and `make sdk-registry` from the repository root; all new gates must
pass headlessly. No code is done while any test fails.

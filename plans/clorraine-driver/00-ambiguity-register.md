# Ambiguity Register: Clorraine build/run driver (`clorraine`)

> **Status**: ✅ GATE PASSED — all 27 items resolved
> **Last Updated**: 2026-09-23 00:11
> **CodeOps Artifact Schema**: 1

Every item below was raised during the Phase 1 interview and resolved by the
user on 2026-09-23. The driver is a new, user-facing surface that fronts the
emit-only `clc`; it does not change the compiler's CLI or the frozen
specification.

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | Planning form and tracking | Full feature plan + roadmap row / lightweight task mini-plan / plan folder only | Full feature plan at `plans/clorraine-driver/`, plus a new `PLAN-02` roadmap row | ✅ Resolved |
| AR-2 | Naming | Where the driver binary lives | New crate / separate `clorraine` binary in the `compiler/` crate / `clc` subcommands | Separate `clorraine` binary in the `compiler/` crate (`[[bin]]`), keeping `clc` unchanged | ✅ Resolved |
| AR-3 | Behavioral | Command surface | `run`+`build`+`emit` / `run` only / single implicit command | `run FILE…`, `build FILE…`, `emit FILE…` | ✅ Resolved |
| AR-4 | Scope | Profiles in v1 | host only / host + hosted-amiga / all | `host` and `hosted-amiga` | ✅ Resolved |
| AR-5 | Behavioral | Runners and inclusion | host+vamos with FS-UAE opt-in / host only / all runners in v1 | All runners in v1: host native exec; `hosted-amiga` `vamos` (default) and FS-UAE (opt-in) | ✅ Resolved |
| AR-6 | Technical | FS-UAE cannot be launched from the SDK `dev` container (no Docker socket; the emulator is host-side via `docker compose`) | Hand off to the host wrapper / give the container the Docker socket / run the driver on the host | Hand off: print the exact host command and exit 0; no Docker socket; no new infrastructure | ✅ Resolved |
| AR-7 | Technical | Where the driver runs | SDK-container tool / host tool that shells into the SDK / host-only | SDK-container tool; outside the container it prints guidance and exits 2 | ✅ Resolved |
| AR-8 | Behavioral | Default profile | host / hosted-amiga / require `--profile` | `host` default; `--profile host\|hosted-amiga` | ✅ Resolved |
| AR-9 | UX | Emitted-C visibility and location | hide + `build/clorraine/<stem>/<profile>/` / print C always / keep next to source | Hidden by default; C and artifacts under `build/clorraine/<stem>/<profile>/` (gitignored) | ✅ Resolved |
| AR-10 | Behavioral | What happens when the emitted C fails to compile under the target C compiler | Internal compiler error (message + C path + first error) / show full C / ask the user | Internal compiler error: concise message, C path, target compiler's first error, exit 1 | ✅ Resolved |
| AR-11 | Behavioral | `run` output and exit status | Propagate the program's exit code / exit 0 on launch | Stream program stdout (Amiga CR stripped); exit with the program's own code | ✅ Resolved |
| AR-12 | UX | `emit` destination | stdout with `-o` to file / always write a file | Stdout by default; `-o OUT.c` writes a file | ✅ Resolved |
| AR-13 | Scope | Project manifest in v1 | no manifest / `clorraine.toml` now | No manifest in v1; flags and built-in defaults only | ✅ Resolved |
| AR-14 | Scope | Convenience Make passthrough | add passthrough / none | Add a root `make run FILE=…` / `make build FILE=…` that calls `clorraine` in the container | ✅ Resolved |
| AR-15 | Technical | Where the driver logic lives | lib module + thin bin / all in the bin / separate crate | Testable lib module `clorraine` + a thin `clorraine` binary, mirroring `clc`'s `main.rs`/lib split | ✅ Resolved |
| AR-16 | Technical | Where the profile→toolchain/runner commands live | driver-owned toolchain module / extend `clc::profile` / a config file | Driver-owned toolchain module; the compiler stays invocation-free; the commands come from `spec/17-codegen.md` §13 in one place | ✅ Resolved |
| AR-17 | Technical | How the driver obtains the emitted C | in-process `clc::driver::run` with `-o` / spawn the `clc` binary | In-process call to `clc::driver::run` with `-o <artifact>.c`, then read the file | ✅ Resolved |
| AR-18 | Behavioral | Input files per invocation | one file / one or more files | One or more `.clo` files, all passed together to `clc` | ✅ Resolved |
| AR-19 | Behavioral | Optimization level | debug default with `--release` / release default | Debug (`-O0 -g`) default; `--release` uses `-O2` | ✅ Resolved |
| AR-20 | Non-functional | Testing strategy | in-container cargo tests with a deterministic FS-UAE handoff check / live emulator tests / manual only | Host and `hosted-amiga`/`vamos` legs asserted by `cargo test` in the SDK container; the FS-UAE handoff asserted as the exact printed command; `make check` remains the fast gate | ✅ Resolved |
| AR-21 | Scope | Authorized modification set | the listed files / more / less | `plans/clorraine-driver/**`; `compiler/Cargo.toml`; `compiler/src/lib.rs` + new `compiler/src/clorraine*`; `compiler/tests/clorraine_driver_spec_test.rs`; `Makefile`; `AGENTS.md`; `plans/00-roadmap.md` | ✅ Resolved |
| AR-22 | Scope | Compiler/spec contract | leave unchanged / extend `clc` | Leave `clc`'s CLI (AR-21 of the MVP) and its emit-only role (AR-6) unchanged; `spec/` untouched | ✅ Resolved |
| AR-23 | Behavioral | Invoked outside the SDK container | guidance + exit 2 / best-effort / hard error with no help | Print a short "run via `make -C docker …`" guidance and exit 2 | ✅ Resolved |
| AR-24 | Behavioral | A required tool is missing (`cc`, `m68k-amigaos-gcc`, `vamos`) | named error + SDK guidance, exit 1 / generic failure | Clear error naming the missing tool and the SDK remedy, exit 1 | ✅ Resolved |
| AR-25 | Behavioral | `--capture` semantics | write `out.txt` + print path / print only / both | Write CR-stripped program stdout to `<artifact-dir>/out.txt` and print the path; without it, stream to the terminal; the FS-UAE handoff command includes `RUN_CAPTURE=1` | ✅ Resolved |
| AR-26 | Naming | Flag surface | the listed set / more / fewer | `-p/--profile`, `--release`, `--emit-c`, `--emulator`, `--capture`, `-o` (emit only), `-h/--help`, `-V/--version`; an unknown flag is a usage error (exit 2) | ✅ Resolved |
| AR-27 | Behavioral | `--keep-c` | keep it / drop it | Dropped: the C always remains in the gitignored artifact directory and `--emit-c` prints it | ✅ Resolved |

## Resolution Notes

**AR-2:** The `compiler/` crate already exports `profile` and `driver::run` from
its lib (`compiler/src/lib.rs`), so a second `[[bin]]` shares them with no
compiler change. The binary name `clorraine` avoids overloading `clc`.

**AR-6:** `docker/scripts/run.sh` is a **host** script that calls
`docker compose … run emulator` with X11; the `dev` container's compose service
mounts only `..:/work` and `/amiga` (`docker/compose.yaml`), so it has no way to
launch a sibling container. Handing off keeps the change small and honest.

**AR-16:** `clc` never spawns a process and writes only `-o`
(`plans/clorraine-compiler-mvp/01-requirements.md:86-91`); the toolchain table
therefore belongs to the driver, not to `clc`.

**AR-20:** `cargo test` runs inside the SDK container via `make check`, where
`m68k-amigaos-gcc` and `vamos` exist, so the `hosted-amiga`/`vamos` leg is
testable in the fast gate. FS-UAE is not headless, so its behavior is asserted
as text (the exact handoff command), matching how the corpus FS-UAE leg is kept
out of `make check`.

## Gate Record

- Systematic scan across all 12 categories completed.
- All 27 rows are user-resolved; none deferred; none open.
- No item introduces a material support surface beyond the driver binary and
  its toolchain table (ordinary feature code invoking existing tools), so the
  Complexity Escalation Gate is not triggered.
- Header reads `✅ GATE PASSED`.

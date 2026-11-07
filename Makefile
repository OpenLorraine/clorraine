# Clorraine project entry points.
#
# `make check` is the fast verification used during development: it validates
# the frozen specification suite on the host and runs the Rust tests in the
# pinned SDK container so every contributor uses the same toolchain.
#
# `make corpus-amiga` builds and runs the portable validation corpus on the
# hosted-amiga profile with the m68k GCC toolchain and the `vamos` emulator,
# diffing each program's output against its recorded output, and builds the
# FS-UAE-only programs (interrupt-counter and graphics) for the emulator leg.
#
# `make corpus-fs-uae` boots the FS-UAE-only programs in FS-UAE and diffs their
# captured output; it depends on `corpus-amiga` for the binaries and runs on the
# host, because FS-UAE needs the X display and the local SDK settings.
#
# `make run-fault-amiga` runs the six runtime-fault programs under `vamos` and
# asserts the exact fault line and a nonzero exit.
#
# `make verify` is the full acceptance run: the fast check plus the
# host/hosted-amiga differential, every hosted-amiga cross-toolchain leg, and
# the runtime faults. It requires the SDK X display for the FS-UAE leg.
#
# `make asan` runs the memory-safety suites under AddressSanitizer.

DOCKER_MAKE ?= $(MAKE) -C docker

# Driver passthrough options.
#   PROFILE   host (default), hosted-amiga, hosted-amiga-13, or freestanding
#   EMULATOR  1 to boot the built program in FS-UAE on the host
#   CAPTURE   1 to write the program output to build/run/out.txt instead of streaming
PROFILE ?= host
EMULATOR ?= 0
CAPTURE ?= 0

# Driver flags derived from the options above.
PROFILE_ARG = $(if $(filter-out host,$(PROFILE)),--profile $(PROFILE),)
CAPTURE_ARG = $(if $(filter 1,$(CAPTURE)),--capture,)
# The artifact name the driver derives: the first source file's stem.
STEM = $(basename $(notdir $(FILE)))
# Prefix for the host FS-UAE wrapper when capturing.
CAPTURE_ENV = $(if $(filter 1,$(CAPTURE)),RUN_CAPTURE=1 ,)
# The A500 profile must boot the KS1.3 ROM through the ADF path; the assignments
# are expanded at make time so the shell sees real NAME=value words.
FS_UAE_ENV = $(if $(filter hosted-amiga-13,$(PROFILE)),AMIGA_MODEL=A500 KICKSTART=/amiga/roms/amiga-os-130.rom RUNNER=adf ,)

# Differential and sanitizer overrides reach the container through the
# environment, so a path with spaces stays one value and never becomes shell
# text. `make differential OUTDIR=...` and `make asan ASAN_SUITE_TIMEOUT=...`
# export these automatically.
export DIFF_ONLY
export OUTDIR
export ASAN_SUITE_TIMEOUT

.PHONY: check bench bench-fast bench-base bench-compare bench-test bench-shape asan differential corpus-amiga corpus-fs-uae run-fault-amiga verify run build sdk-registry sdk-issues sdk-a500-smoke sdk-a500-boing sdk-a500-sprites sdk-a500-paddle sdk-a500-text sdk-a500-assets sdk-a500-gui sdk-a500-exec sdk-advanced-build sdk-a600-services sdk-a1200-workbench sdk-bundle9-build sdk-a500-diskfont sdk-a600-addons sdk-a600-ks31-addons sdk-a1200-ks30-addons sdk-a1200-addons sdk-rtg-build sdk-a1200-rtg sdk-a4000-rtg-piv demo demo-check channel-assets demo-youtube

check:
	./spec/check.sh
	tools/amiga-sdk/registry_test.sh
	$(DOCKER_MAKE) exec CMD="cargo test --workspace"
	$(MAKE) bench-test

# Benchmark harness (generated C: emitted size, section size, host time, m68k
# instruction count). `bench` traces; `bench-fast` skips the trace for
# iteration; `bench-base` builds the recorded base compiler; `bench-compare`
# enforces the plan thresholds.
bench:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 tools/perf/bench.py run --compiler-git $$(git rev-parse HEAD) --label head --out build/perf/head.json"

bench-fast:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && python3 tools/perf/bench.py run --compiler-git $$(git rev-parse HEAD) --label head-fast --out build/perf/head-fast.json --no-trace"

bench-base:
	@test -n "$(COMMIT)" || { echo 'usage: make bench-base COMMIT=<ref>'; exit 2; }
	@sha=$$(git rev-parse --verify --quiet "$(COMMIT)^{commit}") || { echo "bench-base: unknown commit: $(COMMIT)"; exit 2; }; \
	BASE_COMMIT=$$sha $(DOCKER_MAKE) exec PASSTHROUGH="BASE_COMMIT" CMD="docker/scripts/bench-base.sh"

bench-compare:
	$(DOCKER_MAKE) exec CMD="python3 tools/perf/bench.py compare build/perf/base.json build/perf/head.json"

bench-test:
	$(DOCKER_MAKE) exec CMD="python3 -m unittest discover -s tools/perf/tests -t tools/perf -v"

bench-shape:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && docker/scripts/bench-shape.sh"

# Run every suite that references RDC_ASAN under AddressSanitizer. The runner
# installs a temporary `cc` wrapper that links ASan children with `-no-pie`,
# because the SDK image's GCC 12 ASan runtime is unstable for PIE executables.
# Each suite runs serially in its own bounded process.
asan:
	$(DOCKER_MAKE) exec PASSTHROUGH="ASAN_SUITE_TIMEOUT" CMD="docker/scripts/asan-suite.sh"

# Compare host and hosted-amiga stdout for three portable programs. This is a
# convenience gate over the same outputs the corpus pins, not an independent
# signal. `DIFF_ONLY=1` re-diffs an existing OUTDIR without rebuilding, which
# is the negative-control path.
differential:
	$(DOCKER_MAKE) exec PASSTHROUGH="DIFF_ONLY OUTDIR" CMD="docker/scripts/differential-seed.sh"

# Developer convenience: build the driver, then run one program.
#   make run FILE=hello.clo                                  host, streamed
#   make run FILE=hello.clo PROFILE=hosted-amiga             Amiga, under vamos
#   make run FILE=hello.clo PROFILE=hosted-amiga-13          A500/1.3, under vamos
#   make run FILE=hello.clo PROFILE=hosted-amiga EMULATOR=1   Amiga, boot FS-UAE
#   make run FILE=hello.clo PROFILE=hosted-amiga EMULATOR=1 CAPTURE=1  Amiga, capture FS-UAE output
#
# FS-UAE runs on the host (the SDK container cannot launch it), so EMULATOR=1
# builds the program in the container and then boots the emulator here.
#
# `FILE` is trusted developer input, never a forwarded untrusted value: it is
# interpolated into the container command, so only pass a path you typed
# yourself. (The driver itself spawns the compiler without a shell; this
# passthrough is the one place where the value reaches one.)
ifeq ($(EMULATOR),1)
run:
	@test -n "$(FILE)" || { echo 'usage: make run FILE=<source.clo> [PROFILE=hosted-amiga|hosted-amiga-13] [EMULATOR=1] [CAPTURE=1]'; exit 2; }
	@case "$(PROFILE)" in hosted-amiga|hosted-amiga-13) ;; *) echo 'EMULATOR=1 requires PROFILE=hosted-amiga or hosted-amiga-13'; exit 2;; esac
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --profile $(PROFILE) $(FILE)"
	$(FS_UAE_ENV)$(CAPTURE_ENV)$(DOCKER_MAKE) run PROG=build/clorraine/$(STEM)/$(PROFILE)/$(STEM)
else
run:
	@test -n "$(FILE)" || { echo 'usage: make run FILE=<source.clo> [PROFILE=hosted-amiga|hosted-amiga-13] [CAPTURE=1]'; exit 2; }
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine run $(PROFILE_ARG) $(CAPTURE_ARG) $(FILE)"
endif

# Like `run`, but only builds the program (no execution).
build:
	@test -n "$(FILE)" || { echo 'usage: make build FILE=<source.clo> [PROFILE=hosted-amiga]'; exit 2; }
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build $(PROFILE_ARG) $(FILE)"

corpus-amiga:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet"
	$(DOCKER_MAKE) exec CMD="docker/scripts/corpus-amiga.sh"

corpus-fs-uae: corpus-amiga
	docker/scripts/corpus-fs-uae.sh

run-fault-amiga:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet"
	$(DOCKER_MAKE) exec CMD="docker/scripts/run-fault-amiga.sh"

verify:
	$(MAKE) check
	$(MAKE) bench-shape
	$(MAKE) sdk-a500-smoke
	$(MAKE) sdk-a500-boing
	$(MAKE) sdk-a500-sprites
	$(MAKE) sdk-a500-paddle
	$(MAKE) sdk-a500-text
	$(MAKE) sdk-a500-assets
	$(MAKE) sdk-a500-gui
	$(MAKE) sdk-a500-exec
	$(MAKE) sdk-a600-services
	$(MAKE) sdk-a1200-workbench
	$(MAKE) sdk-a500-diskfont
	$(MAKE) sdk-a600-addons
	$(MAKE) sdk-a600-ks31-addons
	$(MAKE) sdk-a1200-ks30-addons
	$(MAKE) sdk-a1200-addons
	$(MAKE) sdk-a1200-rtg
	$(MAKE) sdk-a4000-rtg-piv
	$(MAKE) DIFF_ONLY=0 differential
	$(MAKE) corpus-fs-uae
	$(MAKE) run-fault-amiga
	@echo "verify: all legs passed"

# Amiga SDK registry: validate the capability map and regenerate the coverage
# view. The registry is the source of truth; the generated view is committed.
sdk-registry:
	tools/amiga-sdk/registry.sh check
	tools/amiga-sdk/registry.sh coverage > docs/amiga-sdk/coverage.md
	@echo "sdk-registry: coverage written"

# Generate one tracking-issue body per bundle under build/amiga-sdk/issues/.
sdk-issues:
	tools/amiga-sdk/registry.sh issues
	@echo "sdk-issues: bodies in build/amiga-sdk/issues"

# A500/Kickstart 1.3 runtime smoke: build the hello program under the 1.3
# profile, decrypt the licensed Kickstart 1.3 ROM locally, boot a clean ADF in
# FS-UAE as an A500, and compare the captured screen line to the golden image.
sdk-a500-smoke:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --profile hosted-amiga-13 examples/hello/hello.clo"
	docker/scripts/a500-smoke.sh build/clorraine/hello/hosted-amiga-13/hello

# A500/Kickstart 1.3 native-video smoke: build Boing under the 1.3 profile,
# boot it on the real Kickstart 1.3 ROM, see the live custom display, and match
# the final frame checksum line to the committed golden crop.
sdk-a500-boing:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --profile hosted-amiga-13 examples/amiga/boing.clo"
	docker/scripts/a500-boing-smoke.sh build/clorraine/boing/hosted-amiga-13/boing

# A500/Kickstart 1.3 sprite smoke: build the sprite demo under the 1.3 profile,
# boot it on the real Kickstart 1.3 ROM, compare the held sprite screen, and
# match the hardware-collision line.
sdk-a500-sprites:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --profile hosted-amiga-13 examples/amiga/sprite-demo.clo"
	docker/scripts/a500-sprite-smoke.sh build/clorraine/sprite-demo/hosted-amiga-13/sprite-demo

# A500/Kickstart 1.3 paddle game smoke: build the audio/input/timer demo under
# the 1.3 profile, boot it on the real Kickstart 1.3 ROM, compare the held court,
# and match the hit/audio/input line.
sdk-a500-paddle:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --profile hosted-amiga-13 examples/amiga/paddle.clo"
	docker/scripts/a500-paddle-smoke.sh build/clorraine/paddle/hosted-amiga-13/paddle

# A500/Kickstart 1.3 text/layers smoke: build the scrolling demo under the 1.3
# profile, boot it on the real Kickstart 1.3 ROM, compare the held layer frame,
# and match the measured text line.
sdk-a500-text:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --profile hosted-amiga-13 examples/amiga/scroller.clo"
	docker/scripts/a500-text-smoke.sh build/clorraine/scroller/hosted-amiga-13/scroller

# A500/Kickstart 1.3 DOS/asset smoke: build the ILBM viewer in release mode,
# boot it on the real Kickstart 1.3 ROM, compare the held image frame, and match
# the report line. Release keeps the executable small enough for the ADF and
# chip-RAM budget.
sdk-a500-assets:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --release --profile hosted-amiga-13 examples/amiga/imageviewer.clo"
	docker/scripts/a500-assets-smoke.sh build/clorraine/imageviewer/hosted-amiga-13/imageviewer

sdk-a500-gui:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --release --profile hosted-amiga-13 examples/amiga/gui.clo"
	docker/scripts/a500-gui-smoke.sh build/clorraine/gui/hosted-amiga-13/gui

sdk-a500-exec:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --release --profile hosted-amiga-13 examples/amiga/exec-demo.clo"
	docker/scripts/a500-exec-smoke.sh build/clorraine/exec-demo/hosted-amiga-13/exec-demo

# Advanced 2.0+ build: the Workbench and services demos plus the two test-only
# fixtures. The gates below boot A600/KS2.05 and A1200/KS3.1 directory hard
# drives; the A1200 gate also compares the native Workbench capture.
sdk-advanced-build:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --profile hosted-amiga examples/amiga/services.clo && ./target/debug/clorraine build --profile hosted-amiga examples/amiga/workbench.clo"
	$(DOCKER_MAKE) exec CMD="m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -Wall -Wextra -Werror -o build/amiga-sdk/workbench-start docker/scripts/workbench-start.c -lamiga -lm"
	$(DOCKER_MAKE) exec CMD="m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -Wall -Wextra -Werror -o build/amiga-sdk/workbench-post docker/scripts/workbench-post.c -lamiga -lm"

sdk-a600-services: sdk-advanced-build
	docker/scripts/a600-services-smoke.sh build/clorraine/workbench/hosted-amiga/workbench build/clorraine/services/hosted-amiga/services

sdk-a1200-workbench: sdk-advanced-build
	docker/scripts/a1200-workbench-smoke.sh build/clorraine/workbench/hosted-amiga/workbench build/clorraine/services/hosted-amiga/services

# Bundle 9 closeout demos and gates. The demos are release builds because the
# A600/A1200 gates boot 1 MB / 2 MB machines and the debug images do not fit.
sdk-bundle9-build:
	$(DOCKER_MAKE) exec CMD="docker/scripts/build-bundle9-demos.sh"

sdk-a500-diskfont:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --release --profile hosted-amiga-13 examples/amiga/diskfont.clo"
	docker/scripts/a500-diskfont-smoke.sh build/clorraine/diskfont/hosted-amiga-13/diskfont

sdk-a600-addons: sdk-bundle9-build
	docker/scripts/a600-addons-smoke.sh

sdk-a600-ks31-addons: sdk-bundle9-build
	docker/scripts/a600-ks31-addons-smoke.sh

sdk-a1200-ks30-addons: sdk-bundle9-build
	docker/scripts/a1200-ks30-addons-smoke.sh

sdk-a1200-addons: sdk-bundle9-build sdk-advanced-build
	docker/scripts/a1200-addons-smoke.sh

sdk-rtg-build:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --release --profile hosted-amiga examples/amiga/optional-rtg.clo"

sdk-a1200-rtg: sdk-rtg-build
	docker/scripts/a1200-rtg-smoke.sh build/clorraine/optional-rtg/hosted-amiga/optional-rtg

sdk-a4000-rtg-piv: sdk-rtg-build
	docker/scripts/a4000-rtg-piv-smoke.sh build/clorraine/optional-rtg/hosted-amiga/optional-rtg

# Demo tool checks (host-only; not part of `check`/`verify`). The tool needs Node >= 22,
# which runs TypeScript directly. `make demo-check` type-checks and runs the tool tests;
# rendering the story (`make demo`) additionally needs Chrome, VHS, and the emulator.
demo-check:
	@test -d tools/demo/node_modules || (cd tools/demo && npm ci --silent)
	cd tools/demo && npx tsc --noEmit && node --test --experimental-test-coverage test/*.test.ts

# Render the story video, the README gif, and the poster, then re-render the
# string-parser emulator chapter on its own as `demo/emulator.mp4`. Host-only: it needs
# Chrome, VHS, ffmpeg, and the emulator (X display + Kickstart in docker/.env), so it is
# kept out of `check`/`verify`.
demo:
	@test -d tools/demo/node_modules || (cd tools/demo && npm ci --silent)
	node tools/demo/src/cli.ts render demo/story.ts
	node tools/demo/src/cli.ts render demo/story.ts --only emulator --out demo/emulator.mp4

# Render the YouTube channel branding images into build/channel/. Host-only: it needs
# Chrome (headless), so it is kept out of `check`/`verify`.
channel-assets:
	@test -d tools/demo/node_modules || (cd tools/demo && npm ci --silent)
	node tools/demo/src/cli.ts channel

# Render the story at 1920x1080 for upload (the 1100x700 design pillarboxed). Host-only:
# it needs Chrome, VHS, ffmpeg, and the emulator, so it is kept out of `check`/`verify`.
demo-youtube:
	@test -d tools/demo/node_modules || (cd tools/demo && npm ci --silent)
	node tools/demo/src/cli.ts render demo/story.ts --size 1080 --out demo/demo-full-1080.mp4
	node tools/demo/src/cli.ts render demo/story.ts --size 1080 --only emulator --out demo/emulator-1080.mp4

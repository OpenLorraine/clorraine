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
# `make verify` is the full acceptance run: the fast check plus every
# hosted-amiga cross-toolchain leg. It requires the SDK X display for the
# FS-UAE leg.

DOCKER_MAKE ?= $(MAKE) -C docker

# Driver passthrough options.
#   PROFILE   host (default), hosted-amiga, or freestanding
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

.PHONY: check corpus-amiga corpus-fs-uae run-fault-amiga verify run build demo demo-check channel-assets demo-youtube

check:
	./spec/check.sh
	$(DOCKER_MAKE) exec CMD="cargo test --workspace"

# Developer convenience: build the driver, then run one program.
#   make run FILE=hello.clo                                  host, streamed
#   make run FILE=hello.clo PROFILE=hosted-amiga             Amiga, under vamos
#   make run FILE=hello.clo EMULATOR=1                       Amiga, boot FS-UAE
#   make run FILE=hello.clo EMULATOR=1 CAPTURE=1             Amiga, capture FS-UAE output
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
	@test -n "$(FILE)" || { echo 'usage: make run FILE=<source.clo> [PROFILE=hosted-amiga] [EMULATOR=1] [CAPTURE=1]'; exit 2; }
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --profile hosted-amiga $(FILE)"
	$(CAPTURE_ENV)$(DOCKER_MAKE) run PROG=build/clorraine/$(STEM)/hosted-amiga/$(STEM)
else
run:
	@test -n "$(FILE)" || { echo 'usage: make run FILE=<source.clo> [PROFILE=hosted-amiga] [CAPTURE=1]'; exit 2; }
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
	$(MAKE) corpus-fs-uae
	$(MAKE) run-fault-amiga
	@echo "verify: all legs passed"

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

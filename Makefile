# Clorraine project entry points.
#
# `make check` is the fast verification used during development: it validates
# the frozen specification suite on the host and runs the Rust tests in the
# pinned SDK container so every contributor uses the same toolchain.
#
# `make corpus-amiga` builds and runs the portable validation corpus on the
# hosted-amiga profile with the m68k GCC toolchain and the `vamos` emulator,
# diffing each program's output against its recorded output, and builds the
# interrupt-counter for the emulator leg.
#
# `make corpus-fs-uae` boots the interrupt-counter in FS-UAE and diffs its
# captured output; it depends on `corpus-amiga` for the binary and runs on the
# host, because FS-UAE needs the X display and the local SDK settings.
#
# `make run-fault-amiga` runs the six runtime-fault programs under `vamos` and
# asserts the exact fault line and a nonzero exit.
#
# `make verify` is the full acceptance run: the fast check plus every
# hosted-amiga cross-toolchain leg. It requires the SDK X display for the
# FS-UAE leg.

DOCKER_MAKE ?= $(MAKE) -C docker

.PHONY: check corpus-amiga corpus-fs-uae run-fault-amiga verify run build

check:
	./spec/check.sh
	$(DOCKER_MAKE) exec CMD="cargo test --workspace"

# Developer convenience: build the driver, then run one program in the
# container. `FILE` is the Clorraine source path, relative to the repository
# root; the emitted C and the program land under `build/clorraine/`.
#
# `FILE` is trusted developer input, never a forwarded untrusted value: it is
# interpolated into the container command, so only pass a path you typed
# yourself. (The driver itself spawns the compiler without a shell; this
# passthrough is the one place where the value reaches one.)
run:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine run $(FILE)"

# Like `run`, but only builds the program (no execution).
build:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build $(FILE)"

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

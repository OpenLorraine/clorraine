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

DOCKER_MAKE ?= $(MAKE) -C docker

.PHONY: check corpus-amiga corpus-fs-uae run-fault-amiga

check:
	./spec/check.sh
	$(DOCKER_MAKE) exec CMD="cargo test --workspace"

corpus-amiga:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet"
	$(DOCKER_MAKE) exec CMD="docker/scripts/corpus-amiga.sh"

corpus-fs-uae: corpus-amiga
	docker/scripts/corpus-fs-uae.sh

run-fault-amiga:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet"
	$(DOCKER_MAKE) exec CMD="docker/scripts/run-fault-amiga.sh"

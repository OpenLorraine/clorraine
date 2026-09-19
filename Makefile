# Clorraine project entry points.
#
# `make check` is the fast verification used during development: it validates
# the frozen specification suite on the host and runs the Rust tests in the
# pinned SDK container so every contributor uses the same toolchain.

DOCKER_MAKE ?= $(MAKE) -C docker

.PHONY: check

check:
	./spec/check.sh
	$(DOCKER_MAKE) exec CMD="cargo test --workspace"

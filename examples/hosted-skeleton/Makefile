# Clorraine Phase 0 hosted skeleton - build and run through the SDK container.
#
#   make build       compile the skeleton with GCC (68000/68020) and vbcc
#   make run-vamos   run the 68000 build under vamos (fast, no emulator)
#   make run         run it in FS-UAE via a directory hard drive, capture output
#   make run-adf     run it in FS-UAE via a bootable ADF, extract output
#   make verify      check that vamos and FS-UAE produce identical output
#   make clean       remove build artifacts

SHELL := /bin/bash

REPO := $(realpath $(CURDIR)/../..)
DK   := $(REPO)/docker
COMPOSE := docker compose -p clorraine --env-file $(DK)/.env -f $(DK)/compose.yaml
DEV := $(COMPOSE) run --rm -T dev

GCC_CFLAGS  := -mcrt=nix20 -O2 -Wall -Wextra -Werror
# vbcc needs the NDK include tree on its search path; the system header
# warning 53 comes from devices/timer.h and is not ours to fix.
VBCC_CFLAGS := -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include

TIMEOUT ?= 60
BIN := build/phase0-68000

.PHONY: build run-vamos run run-adf verify clean

build:
	@mkdir -p build
	$(DEV) bash -lc 'cd /work/examples/hosted-skeleton && \
		m68k-amigaos-gcc $(GCC_CFLAGS) -m68000 -o build/phase0-68000 skeleton.c -lamiga && \
		m68k-amigaos-gcc $(GCC_CFLAGS) -m68020 -o build/phase0-68020 skeleton.c -lamiga && \
		vc +aos68k $(VBCC_CFLAGS) -o build/phase0-vbcc skeleton.c -lamiga'

run-vamos:
	$(DEV) vamos /work/examples/hosted-skeleton/$(BIN) | tr -d '\r'

run:
	cd $(DK) && RUN_CAPTURE=1 timeout $(TIMEOUT) ./scripts/run.sh ../examples/hosted-skeleton/$(BIN) || true
	@echo "--- captured output ---"
	@cat $(REPO)/build/run/out.txt

run-adf:
	cd $(DK) && RUNNER=adf RUN_CAPTURE=1 timeout $(TIMEOUT) ./scripts/run.sh ../examples/hosted-skeleton/$(BIN) || true
	$(DEV) xdftool /work/build/run.adf read out.txt /work/build/run >/dev/null 2>&1 || true
	@echo "--- captured output ---"
	@cat $(REPO)/build/run/out.txt

verify: build
	$(DEV) vamos /work/examples/hosted-skeleton/$(BIN) | tr -d '\r' > build/vamos.txt
	cd $(DK) && RUN_CAPTURE=1 timeout $(TIMEOUT) ./scripts/run.sh ../examples/hosted-skeleton/$(BIN) || true
	tr -d '\r' < $(REPO)/build/run/out.txt > build/fsuae.txt
	diff -u build/vamos.txt build/fsuae.txt
	@echo "verify: vamos and FS-UAE outputs match"

clean:
	rm -rf build

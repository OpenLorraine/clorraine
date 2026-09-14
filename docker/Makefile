# Clorraine Amiga SDK - convenience wrapper around the compose environment.
#
# Build (once, slow; from pinned sources):
#   make image
#
# First-time local setup:
#   make init
#
# Daily use:
#   make shell                        one-shot interactive shell (/work = repo)
#   make up / make down               persistent dev container
#   make exec CMD="cargo test"        run any command in the dev container
#   make gui                          shell with X11 (emulator display access)
#   make emulator                     FS-UAE with settings from docker/.env
#   make run PROG=build/hello         boot a compiled program and show it
#   make run-vamos PROG=build/hello   run a CLI program without the emulator
#   make smoke                        verify the toolchain
#   make provenance                   print pinned build revisions
#
# All pinned versions live in versions.env; all local settings live in .env
# (created by `make init`, gitignored).

SHELL := /bin/bash

include versions.env

IMAGE ?= clorraine-sdk:dev
DOCKER ?= docker
COMPOSE = $(DOCKER) compose -p clorraine --env-file .env -f compose.yaml
COMPOSE_RUN = $(COMPOSE) run --rm

BUILD_ARGS = \
	--build-arg DEBIAN_VERSION=$(DEBIAN_VERSION) \
	--build-arg AMIGA_GCC_REPO=$(AMIGA_GCC_REPO) \
	--build-arg AMIGA_GCC_COMMIT=$(AMIGA_GCC_COMMIT) \
	--build-arg BUILD_JOBS=$(BUILD_JOBS) \
	--build-arg TOOLCHAIN_TARGETS='$(TOOLCHAIN_TARGETS)'

.PHONY: image check-image init ensure-env up down shell exec gui emulator \
        run run-vamos smoke provenance

image:
	$(DOCKER) build $(BUILD_ARGS) -t $(IMAGE) .

# Fail early with an actionable message instead of trying to pull a local tag
# from a registry (which produces a confusing "pull access denied" error).
check-image:
	@$(DOCKER) image inspect $(IMAGE) >/dev/null 2>&1 || { \
		echo "ERROR: image '$(IMAGE)' not found. Run 'make image' first."; \
		exit 1; \
	}

ensure-env:
	@test -f .env || scripts/init-env.sh

init:
	@scripts/init-env.sh $(if $(FORCE),--force,)

up: ensure-env check-image
	$(COMPOSE) up -d dev
	@echo "dev container is running; attach with: $(COMPOSE) exec dev bash"

down:
	$(COMPOSE) down

shell: ensure-env check-image
	$(COMPOSE_RUN) shell

exec: ensure-env check-image
	@test -n "$(CMD)" || { echo 'usage: make exec CMD="command"'; exit 2; }
	$(COMPOSE_RUN) -T dev bash -lc '$(CMD)'

gui: ensure-env check-image
	$(COMPOSE_RUN) gui

emulator: ensure-env check-image
	$(COMPOSE_RUN) emulator

run: ensure-env check-image
	@test -n "$(PROG)" || { echo 'usage: make run PROG=build/hello'; exit 2; }
	scripts/run.sh $(PROG) $(ARGS)

run-vamos: ensure-env check-image
	@test -n "$(PROG)" || { echo 'usage: make run-vamos PROG=build/hello'; exit 2; }
	$(COMPOSE_RUN) -T dev vamos /work/$(PROG)

smoke: check-image
	$(COMPOSE_RUN) -T smoke

provenance: check-image
	$(COMPOSE_RUN) -T provenance

# Convenience wrapper around the c-next Amiga SDK image.
#
# Usage:
#   make image       build the SDK image
#   make shell       start an interactive shell with the repo mounted at /work
#   make smoke       run the toolchain smoke test inside the image
#   make provenance  print the pinned revisions recorded in the image
#
# All pinned versions live in versions.env.

include versions.env

IMAGE ?= cnext-sdk:dev
DOCKER ?= docker

BUILD_ARGS = \
	--build-arg DEBIAN_VERSION=$(DEBIAN_VERSION) \
	--build-arg AMIGA_GCC_REPO=$(AMIGA_GCC_REPO) \
	--build-arg AMIGA_GCC_COMMIT=$(AMIGA_GCC_COMMIT) \
	--build-arg BUILD_JOBS=$(BUILD_JOBS) \
	--build-arg TOOLCHAIN_TARGETS='$(TOOLCHAIN_TARGETS)'

.PHONY: image check-image shell smoke provenance

image:
	$(DOCKER) build $(BUILD_ARGS) -t $(IMAGE) .

# Fail early with an actionable message instead of trying to pull a local tag
# from a registry (which produces a confusing "pull access denied" error).
check-image:
	@$(DOCKER) image inspect $(IMAGE) >/dev/null 2>&1 || { \
		echo "ERROR: image '$(IMAGE)' not found. Run 'make image' first."; \
		exit 1; \
	}

shell: check-image
	$(DOCKER) run --rm -it -v "$(CURDIR)/..:/work" $(IMAGE) /bin/bash

smoke: check-image
	$(DOCKER) run --rm -v "$(CURDIR)/..:/work" $(IMAGE) cnext-smoke

provenance: check-image
	$(DOCKER) run --rm $(IMAGE) cat /opt/amiga/BUILD-PROVENANCE.txt

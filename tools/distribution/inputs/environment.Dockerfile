# Maintainer build environment. The context contains only declared, verified
# archives, the mirrored Ubuntu package tree, and these input recipes.
# Build with --network=none; acquisition must already be complete.
ARG BASE_IMAGE=docker.io/library/ubuntu@sha256:45e9ddc6895604674618a409ca9f73d301da8fdb75fe330e8e9a27b3b219e4ad
FROM ${BASE_IMAGE}

ARG BASE_IMAGE
ARG ENVIRONMENT_RECIPE_SHA256
LABEL org.clorraine.base-image="${BASE_IMAGE}" \
      org.clorraine.environment-recipe-sha256="${ENVIRONMENT_RECIPE_SHA256}"

COPY inputs/ubuntu-packages.sha256 /sources/ubuntu-packages.sha256
COPY ubuntu/ /sources/ubuntu/
COPY archives/ /sources/archives/
COPY inputs/build-environment.sh /sources/build-environment.sh

# Package installation is the only privileged preparation step. Both checksum
# inputs are bound here before they can authorise package or source execution.
# The complete 126-file set and absence of links also exclude unlisted packages.
RUN set -eu; \
    test "$(dpkg --print-architecture)" = amd64; \
    test "${#ENVIRONMENT_RECIPE_SHA256}" -eq 64; \
    case "$ENVIRONMENT_RECIPE_SHA256" in *[!0-9a-f]*) exit 1 ;; esac; \
    printf '%s\n' \
      'e066847cd7c084c0151f60bd2788c63b2d1c576374701722fb73c58bf2751e1a  /sources/ubuntu-packages.sha256' \
      '0a987058047d157675728317d252c81f269bc9338f601283879089f4b14124e7  /sources/build-environment.sh' \
      | sha256sum --check -; \
    cd /sources/ubuntu; \
    test -z "$(find pool -type l -print -quit)"; \
    test "$(find pool -type f | wc -l)" -eq 126; \
    sha256sum --check /sources/ubuntu-packages.sha256; \
    # APT's no-download path needs its canonical cache names even when every \
    # local archive is passed explicitly. Epochs omitted from upstream pool \
    # filenames occur as percent-encoded colons in APT's cache filenames. \
    find /sources/ubuntu/pool -type f -name '*.deb' -exec sh -eu -c ' \
        for archive do \
            name=$(dpkg-deb --field "$archive" Package); \
            version=$(dpkg-deb --field "$archive" Version); \
            architecture=$(dpkg-deb --field "$archive" Architecture); \
            case "$name" in ""|*[!a-z0-9+.-]*) exit 1 ;; esac; \
            case "$version" in ""|*[!A-Za-z0-9.+~:-]*) exit 1 ;; esac; \
            case "$architecture" in amd64|all) ;; *) exit 1 ;; esac; \
            version=$(printf "%s" "$version" | sed "s/:/%3a/g"); \
            cp -- "$archive" \
                "/var/cache/apt/archives/${name}_${version}_${architecture}.deb"; \
        done' sh {} +; \
    find /sources/ubuntu/pool -type f -name '*.deb' -print0 \
      | xargs -0 -r env DEBIAN_FRONTEND=noninteractive \
          apt-get --yes --no-download --no-install-recommends install; \
    groupadd --gid 1000 builder; \
    useradd --uid 1000 --gid 1000 --create-home --shell /bin/sh builder; \
    mkdir /build /opt/clorraine-build; \
    chown builder:builder /build /opt/clorraine-build

USER builder
ENV HOME=/home/builder
WORKDIR /build
RUN /bin/sh /sources/build-environment.sh

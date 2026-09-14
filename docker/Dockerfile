# syntax=docker/dockerfile:1
#
# c-next Amiga SDK
# ================
# Builds the complete Amiga cross toolchain from pinned upstream sources and
# packages it into a distributable development image.
#
# Design rules:
#   * We own the chain. The only inputs are pinned revisions recorded in
#     versions.env; no third-party prebuilt toolchain images are consumed.
#   * The toolchain is built in a throwaway builder stage; the published
#     image contains only the installed toolchain and development tooling.
#   * Kickstart ROMs are copyrighted and are never baked into the image.
#     For emulator tests, mount your own ROMs at runtime, or use vamos
#     (bundled) which emulates the AmigaOS API without requiring ROMs.
#
# Stages:
#   toolchain  builds binutils/gcc/gdb/libnix/NDK/vbcc from source
#   sdk        the development image users and CI actually run

ARG DEBIAN_VERSION=bookworm

# ============================================================
# Stage 1: toolchain builder
# ============================================================
FROM debian:${DEBIAN_VERSION} AS toolchain

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential \
      autoconf \
      automake \
      bison \
      ca-certificates \
      flex \
      gettext \
      git \
      libgmp-dev \
      libmpc-dev \
      libmpfr-dev \
      libreadline-dev \
      libtool \
      patch \
      rsync \
      wget \
      xz-utils \
 && rm -rf /var/lib/apt/lists/*

# Pinned toolchain sources. Keep in sync with docker/versions.env.
ARG AMIGA_GCC_REPO=https://github.com/AmigaPorts/m68k-amigaos-gcc
ARG AMIGA_GCC_COMMIT=e7ed7a735c4dd9d2c47e23a50e51b2b7ddd4688c
ARG BUILD_JOBS=4
ARG TOOLCHAIN_TARGETS="min gdb ndk vasm vbcc vlink fd2sfd fd2pragma"

WORKDIR /src
RUN git clone "${AMIGA_GCC_REPO}" amiga-gcc \
 && cd amiga-gcc \
 && git checkout --detach "${AMIGA_GCC_COMMIT}" \
 && git rev-parse HEAD > /src/.amiga-gcc-commit

# Build the toolchain and install it to /opt/amiga. The upstream Makefile
# fetches its own submodules (GCC, binutils, libnix, NDK archives) at the
# revisions defined by the pinned commit, and verifies SHA-256 checksums for
# the downloaded archives where upstream provides them.
RUN cd amiga-gcc \
 && make ${TOOLCHAIN_TARGETS} PREFIX=/opt/amiga -j"${BUILD_JOBS}" \
 && { \
      echo "# c-next Amiga SDK build provenance"; \
      echo "# Generated at image build time."; \
      echo "amiga-gcc repo:   ${AMIGA_GCC_REPO}"; \
      echo "amiga-gcc commit: $(cat /src/.amiga-gcc-commit)"; \
      echo; \
      echo "## Built module revisions"; \
      make l 2>/dev/null || true; \
      echo; \
      echo "## Tool versions"; \
      /opt/amiga/bin/m68k-amigaos-gcc --version | head -1 || true; \
      /opt/amiga/bin/m68k-amigaos-gdb --version | head -1 || true; \
      /opt/amiga/bin/vc -v 2>&1 | head -2 || true; \
    } > /opt/amiga/BUILD-PROVENANCE.txt

# ============================================================
# Stage 2: SDK image
# ============================================================
FROM debian:${DEBIAN_VERSION}-slim AS sdk

ENV DEBIAN_FRONTEND=noninteractive

# Development tooling:
#   build-essential/clang/cmake  host builds and header tooling (later phases)
#   python3/venv                 amitools and project scripts
#   git/curl/make/file           build integration and downloads
#   libreadline8/libgmp10/libmpfr6/zlib1g  runtime libraries used by gdb
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      bash \
      build-essential \
      ca-certificates \
      clang \
      cmake \
      curl \
      file \
      fs-uae \
      gdb \
      git \
      libgmp10 \
      libmpfr6 \
      libreadline8 \
      make \
      pkg-config \
      python3 \
      python3-dev \
      python3-pip \
      python3-venv \
      xauth \
      x11-utils \
      xvfb \
      zlib1g \
 && rm -rf /var/lib/apt/lists/*

# The Amiga toolchain (GCC, gdb, libnix, NDK, vasm/vbcc/vlink).
COPY --from=toolchain /opt/amiga /opt/amiga
ENV AMIGA=/opt/amiga
ENV PATH=/opt/amiga/bin:$PATH

# amitools provides vamos, which runs AmigaOS CLI programs directly on the
# host without a Kickstart ROM. This is the fast path for automated tests.
RUN python3 -m venv /opt/amitools \
 && /opt/amitools/bin/pip install --no-cache-dir --upgrade pip \
 && /opt/amitools/bin/pip install --no-cache-dir \
      "amitools[vamos] @ git+https://github.com/AmigaPorts/amitools.git"
ENV PATH=/opt/amitools/bin:$PATH

# Rust is the implementation language for the c-next compiler itself.
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable
ENV PATH=/opt/cargo/bin:$PATH

# Login shells (docker exec/run with -l) re-read /etc/profile, which would
# otherwise drop the SDK tool paths. Keep them visible for interactive use.
RUN printf '%s\n' \
      'export PATH=/opt/amiga/bin:/opt/amitools/bin:/opt/cargo/bin:$PATH' \
      > /etc/profile.d/cnext-path.sh

# Unprivileged user matching typical host UID/GID (1000) for bind mounts.
RUN useradd --create-home --uid 1000 --shell /bin/bash dev \
 && chown -R dev:dev /opt/cargo /opt/rustup

COPY scripts/smoke-test.sh /usr/local/bin/cnext-smoke
RUN chmod +x /usr/local/bin/cnext-smoke

WORKDIR /work
USER dev
CMD ["/bin/bash"]

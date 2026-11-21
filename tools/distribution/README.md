# Native support distribution

This directory owns the native support build, packaging and acceptance work.
Native release installation is not ready yet. Real support archive proof now
passes on both Ubuntu baselines, including source delivery, native loading,
offline wheels, preparation and spaced-root relocation. Phase hardening/full
review and the compiler driver, installer, editor and release work remain.

**Current proof state:** Greenlet's runtime-only selection is verified; its
optional test subtree is omitted with remaining bytes/source/licences preserved.
The resulting Ubuntu runs pass native imports/closure and original-location
corpus execution, but the relocated SDK exposes GCC specs splitting paths at
spaces. The approved new correction quotes only the find-file specs result
in a private GCC build copy. Original staged source pins and every established
artifact acceptance assertion remain unchanged.

Current lock:
`aca1a0ec2dc4c3fa8e98a3b5bdb17ba0ec6d750fd11e082e4ca2b88e63588566`.
It retains 28 source pins and adds the exact GCC patch/instructions for 104
locked inputs. Before/after hashes constrain patching to the private build
copy; full original GCC source plus patch/notices/build instructions accompany
the modified compiler. Five added real specs controls ran red before the fix;
the unchanged six artifact methods still require complete relocation success.

The current exact-lock public producers, source/native assembly audits,
deterministic archives and shipped-patch reconstruction pass. All eleven
real methods pass on both Ubuntu releases. Earlier successful
packaging and failed acceptance evidence is preserved under
`build/distribution/reference-20261117/pre-greenlet-selection/` and
`pre-gcc-quoting/`. Last-completed candidate counts/results below are historical,
not acceptance of the current patched revision.

Resolved reference inputs and their evidence boundaries are recorded in
[`inputs/README.md`](inputs/README.md). That inventory is not an eligible
support lock or accepted native distribution.

The [source/licence review](inputs/licence-review.md) is incomplete. The
maintainer explicitly directed inclusion of the exact ordinary `libamiga.a`
and retained responsibility for its legalities. Its bytes and upstream identity
are pinned; its original source/terms chain is not asserted as resolved.
Local implementation can continue. Other reviews and technical proof remain
required, and public promotion still needs separate approval.

The [build-component findings](inputs/build-licences.md) record arithmetic,
CPython, LHA and inherited target-code terms. The
[origin trace](inputs/payload-origins.md) distinguishes NDK-derived content
from independent helper/networking headers. The source-form reconstruction
probe and delivered-source assembly now pass. Actual native/file-origin audits
and interpreter/module closure are recorded; `sfdc` is a Perl script, not an
independent native executable. These technical checks do not imply legal
clearance or publication approval.

The private reference now has measured file/link inventories, native ELF and
target-archive inspections, and successful strict-C99 links for both target
runtime profiles with the builder prefix unavailable. Its required standalone
Amiga startup objects are now covered by the user-approved audit-format
amendment, with producer scripts ordered before final lock completion.
The package-name amendment is also approved and red-verified. Package fields
accept actual unqualified Debian names such as `libstdc++6`; source/helper/tool
IDs keep their separate strict grammar. `support.py` now implements read-only
lock validation and staged-source verification. Its local contract cases pass.
The user selected canonical stored Git blob bytes for source staging.
The proven fd2pragma `$Id$` expansion has been canonicalised in owned staging;
the original checkout/reference is preserved. Rebuilding the affected helper
in the same offline image produces identical binary bytes, mode and behaviour.
`support-lock.json` now records the complete measured source/input and Ubuntu
package cohorts. Public locked producer execution now passes; structural
validity and producer success are not archive acceptance or publication approval.
The generated private reference is retained under
`build/distribution/reference-20261117`, including licensed NDK content that
must not be packaged wholesale.

## Lock and source verification

```sh
python3 tools/distribution/support.py validate-lock --lock PATH
python3 tools/distribution/support.py verify-sources --lock PATH --sources DIR
```

These commands do not acquire sources, execute a recipe or create an output
archive. The source check requires a non-root caller and the `checkouts/`,
`archives/` and `inputs/` staging layout. Lock metadata is UTF-8 JSON with a
16 MiB limit; duplicate keys and unresolved references fail.

Git checks cover the exact origin, object format, HEAD, object integrity,
index and actual file bytes/modes. Ignored extras and escaping links fail.
External Git storage and execution-bearing configuration are rejected.
No source filter runs. Even built-in Git checkout conversions must be
canonicalised to the stored blobs during acquisition, before verification.
The verifier never repairs or rewrites source inputs.

## Isolated support build

```sh
tools/distribution/build-support.sh --lock PATH --sources DIR \
    --image-id IMAGE_ID --output OUTPUT
```

`IMAGE_ID` must be the full immutable `sha256:` identity of the locally
prepared image, not a mutable tag. The builder checks source inputs first,
then the image identity, Linux/amd64 platform, pinned base layers and both
preparation labels. It clears the image entry point and executes the literal
locked recipe with networking disabled, no pulls, the caller's UID/GID, and
only read-only sources plus private writable work mounted.

The output must be absent or empty, must not be a symlink, and must be disjoint
from source staging. Successful inspected output becomes `payload/` and
`provenance.json`; failed recipes or unsafe trees leave neither artifact.
The container control file stays outside the recipe's writable mount.

`inputs/support-payload.sh` invokes the fixed reference recipe, then selects
the C compiler/linking closure and preparation helpers. It excludes prepared
NDK trees, C++/Objective-C front ends, debuggers, plugin-development tooling
and the unapproved base-relative `libamiga.a`. This remains a candidate:
complete file-origin/native audits and notice/source assembly are separate
mandatory packaging work.

Real Docker authoring probes have verified synthetic successful output and a
network-blocked failed fetch. The selector now produces a private 422-file,
11-link candidate from the measured reference. Its compiler links strict C99
for both runtime profiles, ordinarily and with link-time optimisation.
Those authoring probes were not a complete public support build or archive
acceptance. The later real locked producer run now passes, as recorded below.

The preparation helper closure is LHA, `sfdc` and `fd2sfd`. fd2pragma is
needed during the private reference build for its independent header inputs,
but its executable is not needed during installation. The refined candidate
omits that unused binary; all other selected bytes and links are unchanged.
[`inputs/ndk-inputs.md`](inputs/ndk-inputs.md) records the fixed transformation
data and exact-patch derivation.

## Standalone NDK preparation

The standalone entry is `prepare_ndk.py --archive PATH --destination DIR`.
Run it from an assembled preparation bundle: `preparation.json`, authenticated
helpers and `ndk-inputs/` are resolved beside the script, not from the caller's
working directory. Real packaged metadata covers the complete assembled SDK
baseline and passes the actual isolated preparation tests.

The destination must be the exact inactive, unprepared SDK baseline. Archive
length/hash and every helper/input hash are checked before helper execution.
The complete archive listing is validated before selected data is extracted
privately. The pipeline uses only prebuilt LHA, `sfdc` and `fd2sfd`; no compiler,
build system, package manager, patch utility or network client is needed.
The prepared file inventory must match exactly before the marker is written.

Manual fixed-input proof reproduces all 1,372 reference files and preserves
the baseline. A copied standalone bundle also succeeds with spaces in both
bundle and SDK paths and refuses repeat preparation without changing its
marker. That command proof uses an explicitly synthetic test lock, not an
eligible distribution lock. The later real locked command now passes;
actual-archive acceptance remains pending.

## Offline vamos wheels

```sh
tools/distribution/build-wheels.sh --lock PATH --sources DIR \
    --image-id IMAGE_ID --abi cp310 --output OUTPUT
```

Use `cp312` for Python 3.12. This command shares the support builder's source,
image, mount and output checks. It appends only `--wheels <abi>` to the locked
recipe. Wheel metadata must match the exact lock, actual ZIP tags and bytes,
package source/version and selected interpreter before output is published.

The fixed recipe builds amitools, machine68k, greenlet, PySDL2 and bootstrap
pip from the pinned sources, without dependency-index access or build
isolation downloads. Build backends are already installed in the prepared
image. Source and notice files accompany these private outputs, including
machine68k's inline wrapper/Musashi terms.

Both ABI builds now have manual fixed-input proof: fresh wheel-only installs,
dependency checks, native imports, and actual vamos hello/exit-7 execution for
both Amiga runtime profiles. Native wheel files are x86_64, require at most
GLIBC 2.14, and have no RPATH/RUNPATH. These probes use the prepared Ubuntu
22.04 image, not the later clean 22.04/24.04 acceptance environments.
The retained `wheel-authoring/<abi>/manual-*-receipt.json` files explicitly
record manual proof without a release lock. Separate real locked producer
execution now passes; compliance assembly and archive acceptance remain pending.

## Real locked producer results

The initial lock `8529d78db6b2eea225dd5575c8d3f458c597e8d7c2cf5402b3474d9f6977a72e`
passed both public validation/source commands. The public C-support and both
wheel producers passed in the inspected offline builder. Actual payload bytes
match provenance: support has 422 files and 11 links; each wheel payload has
99 files, including five wheels with real lock-bound metadata.

The copied standalone preparer also passes on both actual Ubuntu releases,
using newly built helpers and that real lock. All 1,372 prepared files and
baseline bytes match; marker/repeat/path-with-spaces checks pass without
compiler, build, package or network-tool execution.

Those private producer outputs and exact proof receipts are retained beneath
`build/distribution/reference-20261117/pre-greenlet-selection/locked-*`. The later assembled archives
are recorded below. Clean-archive acceptance and separate publication approval
still apply.

## Audited support packaging

```sh
python3 tools/distribution/package_support.py --lock PATH \
    --payload DIR --audit PATH --output FILE
```

The command checks complete actual file/link coverage, locked origins and
review bytes, original notice/source obligations and actual native records.
It also revalidates wheel metadata against the exact lock. It inspects host
ELF and wheel members plus target archives/standalone Amiga objects. Missing
or incorrect dependency providers, newer GLIBC requirements, unsafe links and
executable builder-prefix paths fail without a successful archive.

The maintainer API `assemble_payload(lock, sources, producer, destination)`
creates a fresh `payload/` plus `audit.json` from verified real producer bytes.
For a preparation bundle, pass `sdk_base=` pointing to the complete assembled
SDK. Its metadata must include the source/notice/review files in the installed
baseline, not only compiler files. This API is not an installer or an approval
to publish.

Actual outputs are retained under
`build/distribution/reference-20261117/packaging/`:

| Asset | Audited file/link records | Native records |
|---|---:|---:|
| `sdk` | 704 | 172 |
| `cp310`, `cp312` | 130 each | 2 each |
| `preparation` | 130 | 2 |

Each `archives/<asset>.tar` was built twice through the public command with
identical bytes. Normalised metadata, exact extracted inventories and refusal
to overwrite were checked independently. Each `<asset>-archive-proof.json`
records its bytes/hash, command and audit identity. Generated file-origin
evidence and selected-source snapshots were checked against actual locked
sources; original canonical notices match source bytes. Libnix's reviewed
original-plus-patch source representation is retained separately.

The extracted preparation bundle also passes on actual Ubuntu 22.04/24.04
against the full 704-entry assembled baseline. Its overlay, markers, repeat
refusal, corrupt-input and missing-helper checks pass without compiler/build/
package/network-tool use. Local protocol tests pass under both prepared
builder Python versions; test-only Git is not a user-runtime prerequisite.

The final combined real proof also passes native loading and the complete
fixed corpus before/after moving the SDK/venv root into a path with spaces.
Fresh preparation succeeds after relocation with the old assembled root
unavailable. The five added GCC specs controls pass without changing the
original six artifact assertions.

`acceptance/ubuntu-<release>-proof.json` retains actual environment, commands,
outputs, candidate hashes and copied-test identities.
`support-acceptance-summary.json` cross-checks 28 strict corpus compile/runs
and ten focused GCC queries per cohort (three deliberate invalid queries must
fail). This is support proof, not phase full-gate, installer/editor/WSL or
publication acceptance. No licence clearance is inferred, including for the
explicitly directed ordinary libamiga archive.

## Local tests and mandatory archive acceptance

`make distribution-check` runs the 93 local support specification methods,
17 implementation methods and three Makefile wiring controls. `make check`
always invokes it. Like setup tests, it runs on the host using Python unittest.
It requires Git for synthetic source fixtures and the existing wheel builder's
Python `packaging` dependency for wheel-selection tests. It installs nothing.
Missing development tools fail the check.

This target explicitly reports **local tests only**. It does not run, skip or
claim success for the licensed/isolated archive methods. Those remain mandatory
for the support phase and release acceptance on both Ubuntu/Python cohorts.
After staging copied auditor/tests and verified inputs in the isolated
environment described above, run the existing unittest groups there:

```sh
python3 -m unittest -v \
    support_artifact_spec.SupportArtifactAcceptance \
    support_gcc_relocation_spec.SupportGccRelocationTests
```

Set `CLORRAINE_SUPPORT_ACCEPTANCE` to that environment's absolute acceptance
JSON path. Without it both groups fail setup; there is no skip or success
fallback. The manifest and tests still reject a repository mount, exposed
builder prefix, non-loopback network, wrong platform or unverified archive.
Each cohort must pass all six artifact and five GCC methods. A green local
check or `make verify` alone is not this real-archive acceptance.

## Measured support-proof prerequisites

The isolated support environments use these explicit binary packages and their
APT dependency closures from Ubuntu snapshot `20261117T000000Z`. The full
installed cohorts (161 packages on 22.04; 157 on 24.04), versions and measured
native providers are bound in the support lock. This table documents the
support-proof environment, not permission to install packages automatically.

| Package | Ubuntu 22.04 | Ubuntu 24.04 |
|---|---|---|
| `ca-certificates` | `20260816ubuntu0.22.04.2` | `20260816ubuntu0.24.04.2` |
| `gcc` | `4:11.2.0-1ubuntu1` | `4:13.2.0-7ubuntu1` |
| `libc6-dev` | `2.35-0ubuntu3.18` | `2.39-0ubuntu8.12` |
| `python3`, `python3-venv` | `3.10.6-1~22.04.1` | `3.12.3-0ubuntu2.1` |
| `perl` | `5.34.0-3ubuntu1.10` | `5.38.2-3.2ubuntu0.7` |
| `libgmp10` | `2:6.2.1+dfsg-3ubuntu1` | `2:6.3.0+dfsg-2ubuntu6.1` |
| `libmpfr6` | `4.1.0-3build3` | `4.2.1-1build1.1` |
| `libmpc3` | `1.2.1-2build1` | `1.3.1-1build1.1` |
| `libstdc++6` | `12.3.0-1ubuntu1~22.04.3` | `14.2.0-4ubuntu2~24.04.1` |
| `binutils` | `2.38-4ubuntu2.12` | `2.42-4ubuntu2.10` |
| `file` | `1:5.41-3ubuntu0.1` | `1:5.45-3build1` |

GCC/C headers build user programs; CPython/venv runs the prebuilt wheels; Perl
runs sfdc. The arithmetic/C++ libraries supply measured compiler/runtime
dependencies. Binutils/file support acceptance inspection. Git, build backends
and the custom builder Python are not user-runtime prerequisites. No installer
compiles a support dependency to repair a missing system package.

Ubuntu's initial venv bootstrap wheels are recorded separately: pip 22.0.2 and
setuptools 59.6.0 on 22.04, pip 24.0 and setuptools 68.1.2 on 24.04. They are not
the shipped pip 26.2.1 wheel. Acceptance replaces pip from the verified offline
wheel set before checking runtime dependencies. Download/bootstrap transport,
WSL2/Remote-WSL and FS-UAE are separate later acceptance surfaces.

## Private-display verification

`docker/scripts/run.sh` and `docker/scripts/emulator.sh` preserve nonempty
caller `DISPLAY` and `XAUTHORITY` values across loading `docker/.env`. Without
caller values, the configured defaults remain in effect. No configuration file
is rewritten.

All emulator gates must use a private display. Do not launch them against the
desktop to check whether precedence works. The behavioral regression suite uses
copied wrappers, synthetic configuration and a logging Docker process:

```sh
make -C docker exec CMD="cargo test --workspace --test native_gate_spec_test"
```

Those eight cases check direct and wrapped invocations with both overrides,
each override alone, and both values unset. They do not prove actual X
authentication or container mount resolution.

### Start an authenticated private X server

Run this Bash example from the repository root. It needs the existing SDK,
licensed local media and the host tools used by the normal emulator gates,
including `Xvfb`, `xauth`, `xdpyinfo` and `xdotool`. These are maintainer gate
dependencies, not a new native-installer prerequisite list.

Use an owned scratch directory. Keep the private cookie readable only by its
owner; the SDK container user must be able to read the mounted file. A wildcard
Xauthority family permits the host and container's different hostnames to use
the same secret. It does **not** disable authentication. TCP listening stays off.

```bash
(
set -euo pipefail
mkdir -p "${CODEOPS_TMPDIR:-$PWD/build/tmp}"
work=$(mktemp -d "${CODEOPS_TMPDIR:-$PWD/build/tmp}/private-display.XXXXXX")
xvfb_pid=
cleanup() {
    if [ -n "$xvfb_pid" ]; then
        kill "$xvfb_pid" 2>/dev/null || true
        wait "$xvfb_pid" 2>/dev/null || true
    fi
    # This exact directory was created above; never remove a caller's temp root.
    rm -rf -- "$work"
}
trap cleanup EXIT

export DISPLAY=
for number in $(seq 120 219); do
    if [ ! -e "/tmp/.X11-unix/X$number" ] && [ ! -e "/tmp/.X$number-lock" ]; then
        export DISPLAY=":$number"
        break
    fi
done
test -n "$DISPLAY"
export XAUTHORITY="$work/private.xauth"
install -m 600 /dev/null "$XAUTHORITY"
xauth -f "$XAUTHORITY" add "$DISPLAY" . "$(openssl rand -hex 16)"
# Complete the reader before starting the writer: both lock the same file.
entries=$(xauth -f "$XAUTHORITY" nlist "$DISPLAY")
printf '%s\n' "$entries" | sed 's/^..../ffff/' | xauth -f "$XAUTHORITY" nmerge -
unset entries

Xvfb "$DISPLAY" -screen 0 1280x1024x24 -auth "$XAUTHORITY" -nolisten tcp \
    >"$work/xvfb.log" 2>&1 &
xvfb_pid=$!
for attempt in $(seq 1 100); do
    if timeout 5 xdpyinfo >/dev/null 2>&1; then
        break
    fi
    kill -0 "$xvfb_pid"
    sleep 0.1
done
timeout 5 xdpyinfo >/dev/null

make verify
)
```

A competing server can win the display-number race. In that case readiness
must fail with this private cookie; stop rather than attaching to that server.
Do not edit the user's `.env`, use `xhost +`, or disable X authentication to
make the test pass.

### Check the real handoff before the first full gate

Use a staged hello executable and locally prepared ROM, retaining existing
profile/ROM requirements. Test direct `emulator.sh` and the complete
`run.sh` → `emulator.sh` chain. Keep all observation commands on the private
display. For each invocation:

1. Identify only the container started by that invocation. Refuse to overwrite
   shared `build/run` staging while another SDK emulator is running.
2. Inspect the actual FS-UAE process environment: `DISPLAY` must be private and
   container `XAUTHORITY` must name `/tmp/.Xauthority`.
3. Inspect the container mount: `/tmp/.Xauthority` must bind the exact selected
   host cookie file read-only. Compare mounted bytes without printing the secret.
4. Observe an FS-UAE window on the private X server and the expected hello guest
   output. Successful container creation alone is not sufficient.
5. Stop only the owned container/process. Remove temporary decrypted media and
   the cookie. Never publish those files or private paths in release evidence.

These process/mount/guest observations precede the first `make verify`. Repeat
the full gate on the same private setup. If a gate fails, retain its evidence
only while diagnosing it and clean owned artifacts afterwards; do not stop
unrelated containers or prune the Docker installation.

## Recorded wrapper verification

On 2026-11-17, Ubuntu 26.04.1 LTS x86_64 with host Xvfb package
`2:21.1.22-1ubuntu1.2` and the pinned SDK's FS-UAE 3.1.66 passed:

| Check | Result |
|---|---|
| Eight unchanged behavioral cases | All passed; before the fix, six overrides failed and two defaults passed |
| Shell syntax and ShellCheck 0.11.0 | Both wrappers passed |
| Direct emulator and complete program-wrapper handoffs | Actual process environment, read-only cookie bind, private X connection and hello output verified |
| Full `make verify` on private displays | All legs passed, including the existing licensed-media gates |
| Initial `make check` within that gate | 4,240 Rust tests; Python suites 29 + 103 + 28; fuzz smoke 28 cases, zero failures |

The first probe used a hostname-specific cookie and failed authentication.
Adding the wildcard-family record corrected the test setup; no product or user
configuration change was needed. Owned containers, X server, decrypted ROM,
cookie, probe code and temporary tools/logs were cleaned after verification.

This evidence covers the legacy wrapper change. It does not establish native
Ubuntu 22.04/24.04 support-archive, wheel, relocation or NDK-preparation
acceptance. Those proofs require actual locked candidate archives.

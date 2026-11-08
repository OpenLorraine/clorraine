#!/usr/bin/env python3
"""Detect and prepare the Clorraine Amiga SDK.

The installer reports one row per component: Docker, Compose, the pinned SDK
image, the environment file, the m68k toolchain, the NDK, the host C compiler,
Kickstart ROMs, and Workbench/floppy media. It never copies or downloads
licensed media.

`--fetch` delegates the only public download to the pinned Docker image build.
The installer itself downloads nothing; after a build it re-checks the image
provenance, the NDK tree, and every tool version and fails closed on a
mismatch.

Run `make setup` for detection, or:

    python3 tools/setup/install_sdk.py --fetch --yes

All file-system roots are overridable so tests run in a temporary directory.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

#: Image name used when the environment file does not set one.
DEFAULT_IMAGE = "clorraine-sdk:dev"

#: Probe timeout for the quick checks. The image build has no timeout.
PROBE_TIMEOUT = 10.0

#: The shell program run inside the image to report tools, NDK, and provenance.
IMAGE_PROBE = r"""
for t in m68k-amigaos-gcc m68k-amigaos-gdb vc vamos; do
  case "$t" in
    m68k-amigaos-gcc) v="$(m68k-amigaos-gcc --version 2>&1 | head -1)" ;;
    m68k-amigaos-gdb) v="$(m68k-amigaos-gdb --version 2>&1 | head -1)" ;;
    vc)               v="$(vc -v 2>&1 | head -1)" ;;
    vamos)            v="$(vamos --help 2>&1 | head -1)" ;;
  esac
  if command -v "$t" >/dev/null 2>&1 && [ -n "$v" ]; then
    echo "tool:$t=ok"
    echo "version:$t=$v"
  else
    echo "tool:$t=missing"
  fi
done
if [ -d /opt/amiga/m68k-amigaos/ndk-include ] \
   && [ -n "$(ls -A /opt/amiga/m68k-amigaos/ndk-include 2>/dev/null)" ]; then
  echo "ndk=ok"
else
  echo "ndk=missing"
fi
if [ -f /opt/amiga/BUILD-PROVENANCE.txt ]; then
  sed -n 's/^amiga-gcc commit: //p' /opt/amiga/BUILD-PROVENANCE.txt \
    | head -1 | sed 's/^/provenance=/'
else
  echo "provenance="
fi
"""

#: Tools the pinned image must provide.
IMAGE_TOOLS = ("m68k-amigaos-gcc", "m68k-amigaos-gdb", "vc", "vamos")

#: The fixed component order in every report.
COMPONENT_ORDER = (
    "docker",
    "compose",
    "image",
    "env",
    "toolchain",
    "ndk",
    "host-cc",
    "kickstart",
    "workbench",
)


@dataclass
class CommandResult:
    """The observable result of one external command."""

    returncode: int
    stdout: str
    stderr: str


@dataclass
class Probe:
    """Every fact the report is built from."""

    docker_client: str | None
    docker_daemon: str | None
    compose: str | None
    image: str
    image_id: str | None
    env_file: Path
    env_present: bool
    media_root: Path
    roms: int
    adfs: int
    hdfs: int
    host_cc: str | None
    env_readable: bool = False
    tools: dict[str, bool] = field(default_factory=dict)
    versions: dict[str, str] = field(default_factory=dict)
    ndk: bool = False
    provenance: str | None = None
    pinned_commit: str | None = None


@dataclass
class Component:
    """One report row."""

    name: str
    state: str
    detail: str
    hint: str = ""


@dataclass
class Report:
    """The complete detection result."""

    root: Path
    image: str
    media_root: Path
    components: list[Component]
    plan: list[list[str]]
    ready: bool


@dataclass
class Arguments:
    """Parsed command line."""

    root: Path
    amiga_data: Path | None
    fetch: bool
    dry_run: bool
    yes: bool
    require_media: bool
    as_json: bool


def run_command(argv: list[str], timeout: float | None = None) -> CommandResult:
    """Runs one external command and captures its text output.

    Every external call goes through this function so the unit tests can
    replace it with a fake. A missing executable is reported as return code
    127 instead of raising.
    """
    try:
        completed = subprocess.run(
            argv,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except FileNotFoundError:
        return CommandResult(127, "", f"{argv[0]} not found")
    except subprocess.TimeoutExpired:
        return CommandResult(124, "", f"{argv[0]} timed out")
    return CommandResult(completed.returncode, completed.stdout, completed.stderr)


def which(name: str) -> str | None:
    """Finds an executable on PATH; a seam for the tests."""
    return shutil.which(name)


def is_interactive() -> bool:
    """True when standard input is a terminal; a seam for the tests."""
    return sys.stdin.isatty()


def parse_arguments(argv: list[str]) -> Arguments | str:
    """Parses the command line, returning a message on a usage error."""
    root = Path(__file__).resolve().parents[2]
    amiga_data: Path | None = None
    fetch = dry_run = yes = require_media = as_json = False
    index = 0
    while index < len(argv):
        argument = argv[index]
        if argument == "--root":
            index += 1
            if index >= len(argv):
                return "--root needs a value"
            root = Path(argv[index])
        elif argument.startswith("--root="):
            root = Path(argument[len("--root=") :])
        elif argument == "--amiga-data":
            index += 1
            if index >= len(argv):
                return "--amiga-data needs a value"
            amiga_data = Path(argv[index])
        elif argument.startswith("--amiga-data="):
            amiga_data = Path(argument[len("--amiga-data=") :])
        elif argument == "--fetch":
            fetch = True
        elif argument == "--dry-run":
            dry_run = True
        elif argument == "--yes":
            yes = True
        elif argument == "--require-media":
            require_media = True
        elif argument == "--json":
            as_json = True
        elif argument in ("-h", "--help"):
            return "help"
        else:
            return f"unknown option '{argument}'"
        index += 1
    if not root.is_dir():
        return f"repository root does not exist: {root}"
    return Arguments(root.resolve(), amiga_data, fetch, dry_run, yes, require_media, as_json)


def usage() -> str:
    """The one-line usage text."""
    return (
        "usage: install_sdk.py [--root DIR] [--amiga-data DIR] [--fetch] "
        "[--dry-run] [--yes] [--require-media] [--json]"
    )


def read_env(path: Path) -> dict[str, str]:
    """Reads `KEY=value` lines without evaluating them as shell text."""
    values: dict[str, str] = {}
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return values
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, value = stripped.split("=", 1)
        key = key.strip()
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
            value = value[1:-1]
        if key:
            values[key] = value
    return values


def env_readable(path: Path) -> bool:
    """True when the environment file exists and decodes as UTF-8."""
    try:
        path.read_text(encoding="utf-8")
        return True
    except (OSError, UnicodeDecodeError):
        return False


def count_files(directory: Path, suffix: str) -> int:
    """Counts regular files with a suffix, without opening them."""
    try:
        entries = list(directory.iterdir())
    except OSError:
        return 0
    return sum(1 for entry in entries if entry.is_file() and entry.suffix.lower() == suffix)


def probe_image(
    image: str,
) -> tuple[dict[str, bool], dict[str, str], bool, str | None]:
    """Runs the in-image probe and returns tools, versions, NDK, and provenance."""
    result = run_command(
        ["docker", "run", "--rm", "--entrypoint", "sh", image, "-c", IMAGE_PROBE],
        timeout=PROBE_TIMEOUT,
    )
    tools = {name: False for name in IMAGE_TOOLS}
    versions: dict[str, str] = {}
    ndk = False
    provenance = None
    if result.returncode != 0:
        return tools, versions, ndk, provenance
    for line in result.stdout.splitlines():
        if line.startswith("tool:") and "=" in line:
            name, state = line[len("tool:") :].split("=", 1)
            if name in tools:
                tools[name] = state == "ok"
        elif line.startswith("version:") and "=" in line:
            name, value = line[len("version:") :].split("=", 1)
            if name in tools and value.strip():
                versions[name] = value.strip()
        elif line == "ndk=ok":
            ndk = True
        elif line.startswith("provenance="):
            value = line[len("provenance=") :].strip()
            provenance = value or None
    for name in IMAGE_TOOLS:
        if name not in versions:
            tools[name] = False
    return tools, versions, ndk, provenance


def detect(root: Path, amiga_data: Path | None, require_media: bool) -> Report:
    """Builds the complete report from observed host and image state."""
    env_file = root / "docker" / ".env"
    env = read_env(env_file)
    versions = read_env(root / "docker" / "versions.env")
    image = env.get("IMAGE") or DEFAULT_IMAGE
    pinned_commit = versions.get("AMIGA_GCC_COMMIT") or None

    client = run_command(
        ["docker", "version", "--format", "{{.Client.Version}}"],
        timeout=PROBE_TIMEOUT,
    )
    daemon = run_command(
        ["docker", "version", "--format", "{{.Server.Version}}"],
        timeout=PROBE_TIMEOUT,
    )
    compose = run_command(
        ["docker", "compose", "version", "--short"], timeout=PROBE_TIMEOUT
    )
    image_result = run_command(
        ["docker", "image", "inspect", image, "--format", "{{.Id}}"],
        timeout=PROBE_TIMEOUT,
    )

    if amiga_data is not None:
        media_root = amiga_data
    elif env.get("AMIGA_DATA"):
        media_root = Path(env["AMIGA_DATA"])
    elif os.environ.get("AMIGA_DATA"):
        media_root = Path(os.environ["AMIGA_DATA"])
    else:
        media_root = Path.home() / "amiga"

    image_id = image_result.stdout.strip() if image_result.returncode == 0 else None
    if image_id:
        tools, versions, ndk, provenance = probe_image(image)
    else:
        tools = {name: False for name in IMAGE_TOOLS}
        versions, ndk, provenance = {}, False, None

    host_cc = which("cc") or which("gcc")
    named_media = env.get("AMIGA_DATA")
    media_named_ok = not named_media or Path(named_media).is_dir()
    probe = Probe(
        docker_client=client.stdout.strip() if client.returncode == 0 else None,
        docker_daemon=daemon.stdout.strip() if daemon.returncode == 0 else None,
        compose=compose.stdout.strip() if compose.returncode == 0 else None,
        image=image,
        image_id=image_id,
        env_file=env_file,
        env_present=env_file.is_file() and env_readable(env_file) and media_named_ok,
        env_readable=env_readable(env_file),
        media_root=media_root,
        roms=count_files(media_root / "roms", ".rom"),
        adfs=count_files(media_root / "adf", ".adf"),
        hdfs=count_files(media_root / "hdf", ".hdf"),
        host_cc=host_cc,
        tools=tools,
        versions=versions,
        ndk=ndk,
        provenance=provenance,
        pinned_commit=pinned_commit,
    )
    return build_report(probe, root, require_media)


def build_report(probe: Probe, root: Path, require_media: bool) -> Report:
    """Turns a probe into ordered components, a plan, and a readiness flag."""
    tool_names = ", ".join(
        name
        for name in IMAGE_TOOLS
        if not (probe.tools.get(name) and probe.versions.get(name))
    )
    tool_detail = (
        "all present with versions"
        if not tool_names
        else f"missing or without a version: {tool_names}"
    )

    if probe.provenance is None:
        provenance_detail = "not recorded"
    elif probe.pinned_commit is None:
        provenance_detail = f"{probe.provenance} (no pinned commit)"
    elif probe.provenance == probe.pinned_commit:
        provenance_detail = f"{probe.provenance[:12]} (matches docker/versions.env)"
    else:
        provenance_detail = (
            f"{probe.provenance[:12]} but docker/versions.env pins {probe.pinned_commit[:12]}"
        )

    provenance_ok = (
        probe.pinned_commit is not None and probe.provenance == probe.pinned_commit
    )
    media_state = "ok" if probe.roms else ("missing" if require_media else "optional")
    workbench_count = probe.adfs + probe.hdfs
    workbench_state = (
        "ok" if workbench_count else ("missing" if require_media else "optional")
    )

    components = [
        Component(
            "docker",
            "ok" if probe.docker_client and probe.docker_daemon else "missing",
            probe.docker_client or "client or daemon unavailable",
            "install Docker Engine and start the daemon",
        ),
        Component(
            "compose",
            "ok" if probe.compose else "missing",
            probe.compose or "docker compose unavailable",
            "install the Docker Compose plugin",
        ),
        Component(
            "image",
            "ok" if probe.image_id else "missing",
            (
                f"{probe.image} ({probe.image_id[:19]}); provenance {provenance_detail}"
                if probe.image_id
                else probe.image
            ),
            f"make -C {root / 'docker'} image",
        ),
        Component(
            "env",
            "ok" if probe.env_present else "missing",
            str(probe.env_file)
            if probe.env_present
            else f"{probe.env_file} is missing or unreadable",
            f"make -C {root / 'docker'} init",
        ),
        Component(
            "toolchain",
            "ok"
            if all(
                probe.tools.get(name) and probe.versions.get(name)
                for name in IMAGE_TOOLS
            )
            else "missing",
            tool_detail,
            f"make -C {root / 'docker'} image",
        ),
        Component(
            "ndk",
            "ok" if probe.ndk else "missing",
            "NDK headers present" if probe.ndk else "NDK headers absent",
            f"make -C {root / 'docker'} image",
        ),
        Component(
            "host-cc",
            "ok" if probe.host_cc else "optional",
            probe.host_cc or "no cc or gcc on PATH",
            "install a host C compiler for the host profile",
        ),
        Component(
            "kickstart",
            media_state,
            f"{probe.roms} ROM(s) in {probe.media_root / 'roms'}",
            "place a licensed Kickstart ROM in the roms folder; it is never downloaded",
        ),
        Component(
            "workbench",
            workbench_state,
            f"{workbench_count} ADF/HDF image(s) in {probe.media_root}",
            "place licensed Workbench media in adf/ or hdf/; it is never downloaded",
        ),
    ]
    components.sort(key=lambda row: COMPONENT_ORDER.index(row.name))

    plan: list[list[str]] = []
    if not probe.docker_client or not probe.docker_daemon:
        pass
    else:
        if not probe.image_id:
            plan.append(["make", "-C", str(root / "docker"), "image"])
        if not probe.env_present:
            plan.append(["make", "-C", str(root / "docker"), "init"])

    required = ("docker", "compose", "image", "env", "toolchain", "ndk")
    states = {row.name: row.state for row in components}
    ready = all(states[name] == "ok" for name in required)
    if require_media:
        ready = ready and states["kickstart"] == "ok" and states["workbench"] == "ok"
    if probe.image_id and not provenance_ok:
        ready = False
    return Report(root, probe.image, probe.media_root, components, plan, ready)


def format_command(argv: list[str]) -> str:
    """Renders a command for humans, quoting only whitespace-bearing words."""
    return " ".join(f'"{word}"' if " " in word else word for word in argv)


def render_text(report: Report) -> str:
    """Renders the human report."""
    lines = ["Clorraine SDK"]
    for row in report.components:
        lines.append(f"  [{row.state}]".ljust(13) + f"{row.name}".ljust(13) + row.detail)
    lines.append("")
    lines.append(
        "Downloads: the pinned `make -C docker image` build only; upstream "
        "archives are checksum-verified where upstream publishes checksums."
    )
    lines.append("Media: never downloaded, copied, or modified by this installer.")
    if report.plan:
        lines.append("")
        lines.append("Planned changes:")
        lines.extend(f"  {format_command(argv)}" for argv in report.plan)
    missing = [row.name for row in report.components if row.state == "missing"]
    if missing:
        lines.append("")
        lines.append("Missing: " + ", ".join(missing))
        for row in report.components:
            if row.state == "missing" and row.hint:
                lines.append(f"  {row.name}: {row.hint}")
        lines.append(
            "Media is never downloaded or copied by this installer; supply licensed files."
        )
    return "\n".join(lines) + "\n"


def render_json(report: Report) -> str:
    """Renders one machine-readable report with stable keys."""
    payload = {
        "ready": report.ready,
        "image": report.image,
        "media_root": str(report.media_root),
        "components": [
            {
                "name": row.name,
                "state": row.state,
                "detail": row.detail,
                "hint": row.hint,
            }
            for row in report.components
        ],
        "actions": [format_command(argv) for argv in report.plan],
        "plan": [format_command(argv) for argv in report.plan],
        "downloads": (
            "the pinned docker image build; upstream archives are checksum-verified "
            "where upstream publishes checksums; media is never downloaded"
        ),
    }
    return json.dumps(payload, indent=2) + "\n"


def print_report(report: Report, as_json: bool) -> None:
    """Writes the report to standard output."""
    sys.stdout.write(render_json(report) if as_json else render_text(report))


def report_missing(report: Report) -> None:
    """Writes each missing component and its recovery hint to standard error."""
    missing = [row for row in report.components if row.state == "missing"]
    for row in missing:
        sys.stderr.write(
            f"install_sdk: missing: {row.name}: {row.detail}\n"
            f"install_sdk: recovery: {row.hint or 'inspect the report and retry'}\n"
        )


def run_plan(plan: list[list[str]], root: Path) -> bool:
    """Runs the fetch plan; returns false on the first failure."""
    for argv in plan:
        sys.stderr.write(f"running: {format_command(argv)}\n")
        sys.stderr.flush()
        result = run_command(argv)
        if result.returncode != 0:
            detail = result.stderr.strip() or result.stdout.strip() or "no output"
            sys.stderr.write(
                f"install_sdk: command failed ({result.returncode}): "
                f"{format_command(argv)}\n{detail}\n"
                f"install_sdk: recovery: fix the reported build problem, then rerun "
                f"`make -C {root / 'docker'} image` or `make setup --fetch --yes`.\n"
            )
            return False
    return True


def verify(report: Report) -> bool:
    """Reports a post-fetch verification failure with its recovery command."""
    states = {row.name: row.state for row in report.components}
    failed = [name for name in COMPONENT_ORDER if states.get(name) == "missing"]
    if not failed and report.ready:
        return True
    if failed:
        report_missing(report)
    else:
        image = next(item for item in report.components if item.name == "image")
        sys.stderr.write(
            f"install_sdk: verification failed: provenance: {image.detail}\n"
            f"install_sdk: recovery: rebuild with `make -C {report.root / 'docker'} image` "
            "from the pinned docker/versions.env\n"
        )
    return False


def main(argv: list[str] | None = None) -> int:
    """Runs the installer and returns its process exit code."""
    arguments = parse_arguments(list(sys.argv[1:] if argv is None else argv))
    if arguments == "help":
        sys.stdout.write(usage() + "\n")
        return 0
    if isinstance(arguments, str):
        sys.stderr.write(f"install_sdk: {arguments}\n{usage()}\n")
        return 2

    report = detect(arguments.root, arguments.amiga_data, arguments.require_media)

    if not arguments.fetch:
        print_report(report, arguments.as_json)
        if not report.ready and not arguments.dry_run:
            verify(report)
            sys.stderr.write("install_sdk: the SDK is not ready; see the missing rows above.\n")
            return 1
        return 0

    if arguments.dry_run:
        print_report(report, arguments.as_json)
        sys.stderr.write("Dry run: no command was run.\n")
        return 0

    if not report.plan:
        print_report(report, arguments.as_json)
        if report.ready:
            return 0
        verify(report)
        sys.stderr.write(
            "install_sdk: no automatic action can fix the missing components; "
            "follow the recovery hints above.\n"
        )
        return 1

    if not arguments.as_json:
        print_report(report, False)
    if not arguments.yes:
        if not is_interactive():
            sys.stderr.write(
                "install_sdk: a fetch needs confirmation; rerun with --yes "
                "(or use --dry-run to inspect the plan)\n"
            )
            return 2
        sys.stderr.write("Proceed? [y/N] ")
        sys.stderr.flush()
        answer = input().strip().lower()
        if answer not in ("y", "yes"):
            sys.stderr.write("install_sdk: cancelled\n")
            return 1

    if not run_plan(report.plan, arguments.root):
        if arguments.as_json:
            print_report(report, True)
        return 1
    verified = detect(arguments.root, arguments.amiga_data, arguments.require_media)
    print_report(verified, arguments.as_json)
    if not verify(verified):
        return 1
    sys.stderr.write("Clorraine SDK is ready.\n")
    return 0

if __name__ == "__main__":
    sys.exit(main())

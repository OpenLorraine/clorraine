"""Fail-closed input records for explicitly selected real installer proofs."""

from __future__ import annotations

import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from native_manifest_data import file_digest
from native_transport_fixture import limit_files


CORE_SHA256 = "eed3a17570ebd6fd312d808f28f724d9f3f4a10f1a2d1e42aa35a4e8f8738fb0"
"""Accepted real core bytes; miniature observation executables are not acceptance."""


def ordinary_path(value: object, *, directory: bool = False) -> Path:
    """Require an absolute ordinary file/directory and canonicalize its parents."""
    if not isinstance(value, str):
        raise ValueError("actual input path must be a string")
    path = Path(value)
    if not path.is_absolute() or path.is_symlink():
        raise ValueError("absolute ordinary input required: " + value)
    mode = path.stat().st_mode
    if not (stat.S_ISDIR(mode) if directory else stat.S_ISREG(mode)):
        raise ValueError("actual input has the wrong file type: " + value)
    return path.resolve(strict=True)


def _unique(pairs: list[tuple[str, object]]) -> dict:
    """Reject duplicate JSON keys at every record depth."""
    result = {}
    for name, value in pairs:
        if name in result:
            raise ValueError("duplicate actual context/candidate key: " + name)
        result[name] = value
    return result


def _nonfinite(value: str) -> None:
    """Reject nonfinite extensions to JSON numbers."""
    raise ValueError("nonfinite actual context value: " + value)


def read_json(path: Path) -> dict:
    """Decode bounded ordinary UTF-8 JSON with strict duplicate-key handling."""
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError("bounded ordinary JSON required: " + str(path))
    value = json.loads(path.read_bytes().decode("utf-8"), object_pairs_hook=_unique,
                       parse_constant=_nonfinite)
    if not isinstance(value, dict):
        raise ValueError("actual context/candidate must be a JSON object")
    return value


def overlap(first: Path, second: Path) -> bool:
    """Detect either ancestor direction using canonical filesystem paths."""
    return first == second or first in second.parents or second in first.parents


@dataclass(frozen=True)
class ActualCandidate:
    """Verified candidate paths; creating a record never installs or changes its inputs."""

    manifest: dict
    """Unchanged parsed producer manifest; its exact original bytes remain at bootstrap."""
    bootstrap: Path
    """Read-only directory containing the exact three producer outputs."""
    payloads: dict[str, Path]
    """All twelve identity-to-file bindings after exact hash/length validation."""
    work_root: Path
    """Initially empty ordinary-user-owned disposable work directory."""
    ubuntu: str
    """Actual supported Ubuntu cohort, not a simulated probe value."""
    abi: str
    """Actual matching CPython ABI."""

    def asset(self, kind: str) -> dict:
        """Select an independently identified payload by unique public kind."""
        assets = [item for item in self.manifest["assets"] if item["kind"] == kind]
        if len(assets) != 1:
            raise ValueError("actual candidate must have exactly one " + kind)
        return assets[0]


def load_candidate(variable: str, release: str) -> ActualCandidate:
    """Validate one required actual-context record without mocking its environment."""
    selected = os.environ.get(variable)
    if not selected:
        raise RuntimeError(variable + " is required for actual installer proof")
    context_path = ordinary_path(str(Path(selected).absolute()))
    context = read_json(context_path)
    fields = {"schema", "ubuntu", "abi", "bootstrap", "bootstrap_sha256", "payloads", "work_root"}
    if set(context) != fields or type(context["schema"]) is not int or context["schema"] != 1:
        raise ValueError("actual installer context has invalid schema or unknown/missing fields")
    cohorts = {"22.04": "cp310", "24.04": "cp312"}
    ubuntu, abi = context["ubuntu"], context["abi"]
    actual_os = platform.freedesktop_os_release()
    actual_abi = "cp" + str(sys.version_info.major) + str(sys.version_info.minor)
    if (not isinstance(ubuntu, str) or cohorts.get(ubuntu) != abi or
            actual_os.get("ID") != "ubuntu" or actual_os.get("VERSION_ID") != ubuntu or
            actual_abi != abi or platform.machine() != "x86_64" or
            sys.implementation.name != "cpython" or os.geteuid() == 0):
        raise RuntimeError("actual supported Ubuntu/CPython cohort and ordinary user required")
    bootstrap = ordinary_path(context["bootstrap"], directory=True)
    names = {"install.sh", "install.pyz", "manifest.json"}
    hashes = context["bootstrap_sha256"]
    if not isinstance(hashes, dict) or set(hashes) != names:
        raise ValueError("bootstrap_sha256 must name the exact three producer files")
    if {path.name for path in bootstrap.iterdir()} != names:
        raise ValueError("actual bootstrap directory must contain only three producer files")
    for name in names:
        path = ordinary_path(str(bootstrap / name))
        expected = hashes[name]
        if not isinstance(expected, str) or re.fullmatch("[0-9a-f]{64}", expected) is None:
            raise ValueError("invalid bootstrap hash: " + name)
        if file_digest(path) != expected:
            raise ValueError("actual bootstrap hash mismatch: " + name)
    manifest = read_json(bootstrap / "manifest.json")
    if manifest.get("release") != release:
        raise ValueError(variable + " must name the separately assembled " + release)
    assets = manifest.get("assets")
    if not isinstance(assets, list) or len(assets) != 12:
        raise ValueError("actual manifest must bind twelve assets")
    identities = [item["identity"] for item in assets]
    supplied = context["payloads"]
    if not isinstance(supplied, dict) or len(set(identities)) != 12 or set(supplied) != set(identities):
        raise ValueError("actual payload paths must cover every unique manifest identity")
    payloads = {}
    for asset in assets:
        path = ordinary_path(supplied[asset["identity"]])
        if (type(asset["bytes"]) is not int or path.stat().st_size != asset["bytes"] or
                file_digest(path) != asset["sha256"]):
            raise ValueError("actual payload length/hash mismatch: " + asset["identity"])
        payloads[asset["identity"]] = path
    core = [item for item in assets if item["kind"] == "core"]
    if len(core) != 1 or core[0]["sha256"] != CORE_SHA256:
        raise ValueError("actual acceptance requires the accepted real current core")
    work = ordinary_path(context["work_root"], directory=True)
    if work.stat().st_uid != os.geteuid() or any(work.iterdir()):
        raise ValueError("actual work_root must be empty and owned by this ordinary user")
    if any(overlap(work, path) for path in (context_path, bootstrap, *payloads.values())):
        raise ValueError("actual work_root must be disjoint from context and candidate inputs")
    if any(os.access(path, os.W_OK) for path in (bootstrap, *payloads.values())):
        raise RuntimeError("actual candidate inputs must be bound read-only")
    packages = manifest["prerequisites"][ubuntu]
    if not isinstance(packages, list) or not packages or any(
            not isinstance(name, str) or re.fullmatch("[a-z0-9][a-z0-9+.-]+", name) is None
            for name in packages):
        raise ValueError("invalid actual prerequisite package names")
    for name in ("cc", "gcc", "python3", "curl", "sha256sum", "dpkg-query"):
        if shutil.which(name, path="/usr/bin:/bin") is None:
            raise RuntimeError("actual prerequisite unavailable: " + name)
    result = subprocess.run(
        ["/usr/bin/dpkg-query", "-W", "-f=${Package} ${db:Status-Status}\n", *packages],
        env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"}, capture_output=True, text=True,
        timeout=30, check=False, preexec_fn=limit_files)
    installed = {line.split()[0] for line in result.stdout.splitlines() if line.endswith(" installed")}
    if result.returncode or set(packages) - installed:
        raise RuntimeError("actual prerequisite packages unavailable: " +
                           repr(sorted(set(packages) - installed)))
    return ActualCandidate(manifest, bootstrap, payloads, work, ubuntu, abi)


def require_update_pair(initial: ActualCandidate, update: ActualCandidate) -> None:
    """Require independent release inputs with identical core and revised support identity."""
    first, second = initial.asset("core"), update.asset("core")
    if (initial.ubuntu != update.ubuntu or initial.abi != update.abi or
            first["identity"] != second["identity"] or first["sha256"] != second["sha256"] or
            first["bytes"] != second["bytes"] or
            initial.manifest["core_version"] != update.manifest["core_version"] or
            initial.manifest["support"]["identity"] == update.manifest["support"]["identity"]):
        raise ValueError("actual update must retain core identity/bytes and change support identity")
    if initial.work_root.stat().st_dev != update.work_root.stat().st_dev:
        raise ValueError("both actual work roots must be on the installation filesystem")
    originals = {item["identity"]: item for item in initial.manifest["assets"]}
    for asset in update.manifest["assets"]:
        previous = originals.get(asset["identity"])
        if previous is not None and (asset["bytes"], asset["sha256"]) != (
                previous["bytes"], previous["sha256"]):
            raise ValueError("a reused actual component identity has different bytes")

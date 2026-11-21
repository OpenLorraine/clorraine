#!/usr/bin/env python3
"""Validate support metadata and verify staged inputs without executing source code."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
from typing import NoReturn
from urllib.parse import urlsplit


class SupportError(ValueError):
    """A named metadata or source-integrity failure suitable for CLI diagnostics."""


def _fail(where: str, message: str) -> NoReturn:
    """Keep rejected input diagnostics tied to their field or component."""
    raise SupportError(f"{where}: {message}")


def _text(value: object, where: str, *, empty: bool = False) -> str:
    """Require UTF-8 text without control characters or ambiguous surrogate bytes."""
    if not isinstance(value, str) or (not empty and not value):
        _fail(where, "expected a nonempty string")
    try:
        value.encode("utf-8")
    except UnicodeError as error:
        raise SupportError(f"{where}: invalid UTF-8 text") from error
    if any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value):
        _fail(where, "control characters are not permitted")
    return value


def _object(value: object, where: str, fields: set[str] | None = None) -> dict:
    """Reject wrong shapes and unknown fields instead of silently ignoring them."""
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        _fail(where, "expected an object with string keys")
    if fields is not None:
        missing, extra = fields - value.keys(), value.keys() - fields
        if missing:
            _fail(where, f"missing fields: {', '.join(sorted(missing))}")
        if extra:
            _fail(where, f"unknown fields: {sorted(extra)!r}")
    return value


def _token(value: object, where: str, pattern: str) -> str:
    """Apply the specific field's allowlist, not a broader neighbouring grammar."""
    result = _text(value, where)
    if re.fullmatch(pattern, result) is None:
        _fail(where, f"invalid value {result!r}")
    return result


def _identifier(value: object, where: str) -> str:
    """Validate source/helper/tool identifiers independently of package names."""
    return _token(value, where, r"[a-z0-9][a-z0-9._-]*")


def _package_name(value: object, where: str) -> str:
    """Accept actual unqualified Debian names, including valid plus signs."""
    return _token(value, where, r"[a-z0-9][a-z0-9+.-]+")


def _hash(value: object, where: str) -> str:
    """Require a complete lowercase SHA-256 digest."""
    return _token(value, where, r"[0-9a-f]{64}")


def _relative(value: object, where: str) -> str:
    """Keep metadata paths canonical and inside their declared POSIX root."""
    result = _text(value, where)
    if ("\\" in result or "!" in result or result.startswith("/")
            or any(part in ("", ".", "..") for part in result.split("/"))):
        _fail(where, f"unsafe relative path {result!r}")
    return result


def _absolute(value: object, where: str) -> str:
    """Require a literal canonical absolute POSIX path, without parent traversal."""
    result = _text(value, where)
    if (not result.startswith("/") or result.startswith("//") or "\\" in result
            or str(PurePosixPath(result)) != result
            or ".." in result.split("/")):
        _fail(where, f"unsafe absolute path {result!r}")
    return result


def _version(value: object, where: str) -> str:
    """Require a literal version, not a common branch, wildcard or constraint."""
    result = _token(value, where, r"[A-Za-z0-9.!+_~:-]+")
    if result.lower() in {"latest", "main", "master", "head", "stable", "develop", "trunk"}:
        _fail(where, f"mutable version {result!r}")
    return result


def _url(value: object, where: str) -> str:
    """Allow HTTPS acquisition records without credentials, fragments or whitespace."""
    result = _text(value, where)
    try:
        parsed = urlsplit(result)
        valid = (parsed.scheme == "https" and parsed.hostname and parsed.port != 0
                 and parsed.username is None and parsed.password is None and not parsed.fragment)
    except ValueError:
        valid = False
    if not valid or any(char.isspace() for char in result):
        _fail(where, "expected an absolute HTTPS URL without credentials or fragment")
    return result


def _strings(value: object, where: str, check=_text, *, nonempty: bool = False) -> list[str]:
    """Validate array elements and reject duplicates before interpreting references."""
    if not isinstance(value, list) or (nonempty and not value):
        _fail(where, "expected an array" + (" with at least one entry" if nonempty else ""))
    result = [check(item, f"{where}[{index}]") for index, item in enumerate(value)]
    if len(set(result)) != len(result):
        _fail(where, "duplicate array entries")
    return result


def _mapping(value: object, where: str, key_check, value_check, *, nonempty=False) -> dict:
    """Check typed map keys and values without rewriting their identities."""
    result = _object(value, where)
    if nonempty and not result:
        _fail(where, "must not be empty")
    for key, item in result.items():
        key_check(key, f"{where}[{key!r}]")
        value_check(item, f"{where}.{key}")
    return result


def validate_lock(value: object) -> dict:
    """Validate the complete lock schema and references, without executing tools.

    Returns the input dictionary unchanged. Raises SupportError for malformed
    fields or unresolved references. Structural validity is not release clearance.
    """
    root = _object(value, "lock", {"schema", "sources", "inputs", "build", "preparation", "output"})
    if type(root["schema"]) is not int or root["schema"] != 1:
        _fail("schema", "unsupported schema")
    inputs = _mapping(root["inputs"], "inputs", _relative, _hash)
    sources = _object(root["sources"], "sources")
    if not sources or "lha" not in sources:
        _fail("sources.lha", "the lha source is required")
    for name, source in sources.items():
        where = f"sources[{name!r}]"
        _identifier(name, where)
        source = _object(source, where)
        kind = source.get("kind")
        common = {"kind", "url", "licence"}
        if kind == "git":
            _object(source, where, common | {"commit"})
            _token(source["commit"], where + ".commit", r"(?:[0-9a-f]{40}|[0-9a-f]{64})")
        elif kind == "archive":
            _object(source, where, common | {"version", "bytes", "sha256"})
            _version(source["version"], where + ".version")
            if type(source["bytes"]) is not int or source["bytes"] <= 0:
                _fail(where + ".bytes", "expected a positive integer")
            _hash(source["sha256"], where + ".sha256")
        else:
            _fail(where + ".kind", "expected git or archive")
        _url(source["url"], where + ".url")
        terms = _object(source["licence"], where + ".licence",
                        {"id", "notice", "source_required", "review"})
        _text(terms["id"], where + ".licence.id")
        _relative(terms["notice"], where + ".licence.notice")
        if type(terms["source_required"]) is not bool:
            _fail(where + ".licence.source_required", "expected a boolean")
        review = _relative(terms["review"], where + ".licence.review")
        if review not in inputs:
            _fail(where + ".licence.review", f"missing input hash for {review!r}")

    build = _object(root["build"], "build",
                    {"image", "environment_recipe", "ubuntu", "packages", "tools", "argv"})
    _token(build["image"], "build.image", r"[a-z0-9][a-z0-9._/:-]*@sha256:[0-9a-f]{64}")
    if build["ubuntu"] != "22.04":
        _fail("build.ubuntu", "expected 22.04")
    environment = _relative(build["environment_recipe"], "build.environment_recipe")
    if environment not in inputs:
        _fail("build.environment_recipe", f"missing input hash for {environment!r}")
    _mapping(build["packages"], "build.packages", _package_name, _version)
    _mapping(build["tools"], "build.tools", _identifier, _version, nonempty=True)
    argv = _strings(build["argv"], "build.argv",
                    lambda item, where: _text(item, where, empty=True), nonempty=True)
    prefix = "/sources/inputs/"
    if len(argv) < 2 or argv[0] != "/bin/sh" or not argv[1].startswith(prefix):
        _fail("build.argv", "expected /bin/sh followed by a script under /sources/inputs/")
    recipe = _relative(argv[1][len(prefix):], "build.argv[1]")
    if recipe not in inputs:
        _fail("build.argv[1]", f"missing input hash for {recipe!r}")

    preparation = _object(root["preparation"], "preparation",
                          {"archive", "script", "patches", "helpers", "inventory"})
    archive = _identifier(preparation["archive"], "preparation.archive")
    if archive not in sources or sources[archive]["kind"] != "archive":
        _fail("preparation.archive", f"unknown archive source {archive!r}")
    paths = [_relative(preparation["script"], "preparation.script")]
    paths.extend(_strings(preparation["patches"], "preparation.patches", _relative))
    for path in paths:
        if path not in inputs:
            _fail("preparation", f"missing input hash for {path!r}")
    helpers = _object(preparation["helpers"], "preparation.helpers")
    if not helpers or "lha" not in helpers:
        _fail("preparation.helpers.lha", "the lha helper is required")
    for name, helper in helpers.items():
        where = f"preparation.helpers[{name!r}]"
        _identifier(name, where)
        helper = _object(helper, where, {"source", "path"})
        source = _identifier(helper["source"], where + ".source")
        if source not in sources:
            _fail(where + ".source", f"unknown source {source!r}")
        _relative(helper["path"], where + ".path")
    if helpers["lha"]["source"] != "lha":
        _fail("preparation.helpers.lha", "must reference source lha")
    _mapping(preparation["inventory"], "preparation.inventory", _relative, _hash, nonempty=True)

    output = _object(root["output"], "output",
                     {"architecture", "python_abis", "runtimes", "runtime_packages",
                      "builder_prefixes", "forbidden_sha256"})
    if output["architecture"] != "x86_64":
        _fail("output.architecture", "expected x86_64")
    for name, expected in (("python_abis", ["cp310", "cp312"]), ("runtimes", ["nix13", "nix20"])):
        if _strings(output[name], "output." + name, _identifier) != expected:
            _fail("output." + name, f"expected {expected!r}")
    cohorts = _object(output["runtime_packages"], "output.runtime_packages", {"22.04", "24.04"})
    for cohort, packages in cohorts.items():
        packages = _object(packages, "output.runtime_packages." + cohort)
        for name, package in packages.items():
            where = f"output.runtime_packages.{cohort}[{name!r}]"
            _package_name(name, where)
            package = _object(package, where, {"version", "sonames", "paths"})
            _version(package["version"], where + ".version")
            _strings(package["sonames"], where + ".sonames")
            _strings(package["paths"], where + ".paths", _absolute)
    _strings(output["builder_prefixes"], "output.builder_prefixes", _absolute)
    _strings(output["forbidden_sha256"], "output.forbidden_sha256", _hash)
    return root


def load_lock(path: Path) -> tuple[dict, str]:
    """Read one UTF-8 lock and return validated metadata plus its exact-byte SHA-256.

    Duplicate keys fail at any depth. A 16 MiB read bound prevents unbounded
    metadata allocation. The returned digest covers the same bytes that were
    parsed, so callers need not re-read a possibly changing file for provenance.
    """
    def unique(pairs: list[tuple[str, object]]) -> dict:
        """Reject ambiguous JSON keys rather than choosing the last occurrence."""
        result = {}
        for key, value in pairs:
            if key in result:
                _fail(str(path), f"duplicate JSON key {key!r}")
            result[key] = value
        return result

    try:
        # Nonblocking open lets us reject FIFOs without waiting for a writer.
        # Inspect the opened descriptor so a replaced path cannot bypass the
        # ordinary-file requirement between a separate stat and open.
        descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
        with os.fdopen(descriptor, "rb") as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                _fail(str(path), "lock must be an ordinary file")
            content = stream.read(16 * 1024 * 1024 + 1)
        if len(content) > 16 * 1024 * 1024:
            _fail(str(path), "lock exceeds the 16 MiB metadata limit")
        parsed = json.loads(content.decode("utf-8"), object_pairs_hook=unique)
    except SupportError:
        raise
    except (OSError, UnicodeError, ValueError, RecursionError) as error:
        raise SupportError(f"{str(path)!r}: cannot read UTF-8 JSON lock") from error
    return validate_lock(parsed), hashlib.sha256(content).hexdigest()


def _file_hash(path: Path, algorithm: str = "sha256", *, git_blob: bool = False) -> str:
    """Stream file bytes; Git blob hashes include their typed length header."""
    digest = hashlib.new(algorithm)
    with path.open("rb") as stream:
        if git_blob:
            digest.update(f"blob {os.fstat(stream.fileno()).st_size}\0".encode("ascii"))
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _walk(root: Path, *, skip_git: bool = False):
    """Yield entries without following directory links or hiding ignored inputs."""
    def failed(error: OSError) -> NoReturn:
        """An unreadable subtree cannot count as an empty, verified subtree."""
        raise error

    for directory, folders, files in os.walk(root, followlinks=False, onerror=failed):
        if skip_git and Path(directory) == root and ".git" in folders:
            folders.remove(".git")
        for name in sorted(folders + files):
            path = Path(directory) / name
            yield path.relative_to(root).as_posix(), path, path.lstat()


def _internal_link(root: Path, path: Path, where: str) -> None:
    """Reject dangling, cyclic or escaping links before reading linked data."""
    try:
        target = path.resolve(strict=True)
    except (OSError, RuntimeError) as error:
        raise SupportError(f"{where}: invalid source link {path.name!r}") from error
    if target != root and root not in target.parents:
        _fail(where, f"escaping source link {path.name!r}")
    metadata = root / ".git"
    if target == metadata or metadata in target.parents:
        _fail(where, f"source link reads unlocked Git metadata: {path.name!r}")


def _git_environment() -> dict[str, str]:
    """Remove caller Git overrides and disable global configuration and replacement objects."""
    result = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    result.update({
        "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
        "GIT_ATTR_NOSYSTEM": "1", "GIT_NO_REPLACE_OBJECTS": "1",
        "GIT_NO_LAZY_FETCH": "1",
        "GIT_OPTIONAL_LOCKS": "0", "GIT_TERMINAL_PROMPT": "0", "LC_ALL": "C"})
    return result


def _git(checkout: Path | None, arguments: list[str], where: str) -> bytes:
    """Run only explicit Git builtins; errors never expose arbitrary repository config values."""
    command = ["git", "--no-pager", "-c", "core.fsmonitor=false",
               "-c", "core.hooksPath=/dev/null"]
    if checkout is not None:
        command += ["-c", f"safe.directory={checkout}", "--git-dir", str(checkout / ".git"),
                    "--work-tree", str(checkout)]
    try:
        result = subprocess.run(command + arguments, cwd="/", env=_git_environment(),
                                capture_output=True, timeout=180)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise SupportError(f"{where}: Git could not complete {arguments[0]}") from error
    if result.returncode:
        _fail(where, f"Git {arguments[0]} rejected metadata or objects (exit {result.returncode})")
    return result.stdout


def _git_metadata(checkout: Path, where: str) -> dict[bytes, bytes]:
    """Require ordinary self-contained Git storage without execution-bearing local configuration."""
    metadata = checkout / ".git"
    if metadata.is_symlink() or not metadata.is_dir():
        _fail(where, "expected an ordinary .git directory")
    for relative, path, info in _walk(metadata):
        if not (stat.S_ISDIR(info.st_mode) or stat.S_ISREG(info.st_mode)):
            _fail(where, f"unsupported Git metadata entry {relative!r}")
    for relative in ("commondir", "config.worktree", "index.lock", "info/grafts", "info/attributes",
                     "objects/info/alternates", "objects/info/http-alternates", "refs/replace"):
        if (metadata / relative).exists():
            _fail(where, f"undeclared Git storage or overrides at {relative!r}")
    # --file outside a repository plus --no-includes lets us inspect this
    # configuration before any repository command can consult external files.
    raw = _git(None, ["config", "--file", str(metadata / "config"),
                      "--no-includes", "--null", "--list"], where)
    values = {}
    for entry in raw.split(b"\0"):
        if not entry:
            continue
        key, _, value = entry.partition(b"\n")
        lowered = key.lower()
        if (lowered.startswith((b"include.", b"includeif.", b"filter.", b"credential.", b"fsck."))
                or lowered == b"extensions.partialclone"
                or (lowered.startswith(b"remote.")
                    and lowered.endswith((b".promisor", b".partialclonefilter")))
                or lowered in {b"core.fsmonitor", b"core.hookspath", b"core.attributesfile",
                               b"core.worktree", b"core.sshcommand", b"core.gitproxy"}
                or (lowered.startswith(b"diff.") and lowered.endswith((b".command", b".textconv")))):
            _fail(where, f"executable or external Git configuration {key!r}")
        if lowered in values and lowered in {b"remote.origin.url", b"core.bare"}:
            _fail(where, f"ambiguous Git configuration {key!r}")
        values[lowered] = value
    if values.get(b"core.bare", b"false").lower() not in {b"false", b"no", b"off", b"0"}:
        _fail(where, "a bare repository is not a staged checkout")
    return values


def _git_entries(content: bytes, where: str, *, index: bool) -> dict[str, tuple[str, str]]:
    """Decode tree/index records without invoking worktree filters or trusting file extensions."""
    result = {}
    for record in content.split(b"\0"):
        if not record:
            continue
        header, separator, raw_path = record.partition(b"\t")
        fields = header.split()
        if not separator or len(fields) != 3:
            _fail(where, "invalid Git tree/index record")
        mode, middle, last = (item.decode("ascii") for item in fields)
        if mode not in {"100644", "100755", "120000"} or (index and last != "0"):
            _fail(where, "unsupported or unmerged Git entry")
        if not index and middle != "blob":
            _fail(where, "non-blob source entry")
        path = os.fsdecode(raw_path)
        if (path.startswith("/") or any(part in ("", ".", "..", ".git") for part in path.split("/"))
                or path in result):
            _fail(where, f"unsafe or duplicate Git path {path!r}")
        result[path] = (mode, middle if index else last)
    return result


def _verify_git(checkout: Path, source: dict, where: str) -> None:
    """Verify object integrity, index and actual bytes rather than trusting HEAD or Git status alone."""
    if checkout.is_symlink() or not checkout.is_dir():
        _fail(where, "missing ordinary source checkout")
    metadata = _git_metadata(checkout, where)
    if metadata.get(b"remote.origin.url") != source["url"].encode("utf-8"):
        _fail(where, "origin URL does not match the locked source")
    algorithm = _git(checkout, ["rev-parse", "--show-object-format"], where).strip().decode("ascii")
    expected_algorithm = "sha1" if len(source["commit"]) == 40 else "sha256"
    if algorithm != expected_algorithm:
        _fail(where, "Git object format does not match the locked commit")
    if _git(checkout, ["rev-parse", "HEAD"], where).strip().decode("ascii") != source["commit"]:
        _fail(where, "HEAD does not match the locked commit")
    _git(checkout, ["fsck", "--full", "--no-reflogs", "--no-dangling", "--no-progress"], where)
    expected = _git_entries(
        _git(checkout, ["ls-tree", "-r", "-z", "--full-tree", "HEAD"], where), where, index=False)
    index = _git_entries(_git(checkout, ["ls-files", "--stage", "-z"], where), where, index=True)
    if index != expected:
        _fail(where, "tracked index changes do not match HEAD")
    directories = {parent.as_posix() for name in expected
                   for parent in PurePosixPath(name).parents if parent.as_posix() != "."}
    seen = set()
    for relative, path, info in _walk(checkout, skip_git=True):
        if stat.S_ISDIR(info.st_mode):
            if relative not in directories:
                _fail(where, f"extra source directory {relative!r}")
            continue
        if relative not in expected:
            _fail(where, f"extra untracked or ignored source input {relative!r}")
        mode, object_id = expected[relative]
        if stat.S_ISLNK(info.st_mode):
            _internal_link(checkout, path, where)
            content = os.fsencode(os.readlink(path))
            actual = hashlib.new(algorithm, f"blob {len(content)}\0".encode("ascii") + content).hexdigest()
            valid_mode = mode == "120000"
        elif stat.S_ISREG(info.st_mode):
            actual = _file_hash(path, algorithm, git_blob=True)
            valid_mode = mode in {"100644", "100755"} and (
                bool(info.st_mode & stat.S_IXUSR) == (mode == "100755"))
        else:
            _fail(where, f"unsupported source entry {relative!r}")
        if not valid_mode or actual != object_id:
            _fail(where, f"tracked source bytes or mode differ: {relative!r}")
        seen.add(relative)
    if seen != expected.keys():
        missing = sorted(expected.keys() - seen)
        _fail(where, f"missing source file {missing[0]!r} ({len(missing)} missing)")
    if source["licence"]["notice"] not in expected:
        _fail(where, f"missing tracked notice {source['licence']['notice']!r}")


def _verify_files(root: Path, expected: dict[str, tuple[str, int | None]], where: str) -> None:
    """Check complete ordinary-file coverage, lengths and hashes, including extra empty directories."""
    if root.is_symlink() or (expected and not root.is_dir()):
        _fail(where, "missing ordinary staging directory")
    directories = {parent.as_posix() for name in expected
                   for parent in PurePosixPath(name).parents if parent.as_posix() != "."}
    seen = set()
    for relative, path, info in _walk(root):
        if stat.S_ISDIR(info.st_mode):
            if relative not in directories:
                _fail(where, f"extra input directory {relative!r}")
            continue
        if relative not in expected or not stat.S_ISREG(info.st_mode):
            _fail(where, f"extra or non-regular input {relative!r}")
        digest, length = expected[relative]
        if (length is not None and info.st_size != length) or _file_hash(path) != digest:
            _fail(where, f"input bytes differ: {relative!r}")
        seen.add(relative)
    if seen != expected.keys():
        missing = sorted(expected.keys() - seen)
        _fail(where, f"missing input {missing[0]!r} ({len(missing)} missing)")


def verify_sources(lock: dict, sources: Path) -> None:
    """Verify all declared staged inputs without fetching, transforming or executing source files.

    The source root contains only checkouts, archives and inputs. Git storage
    must be self-contained; ignored extras, staged edits and external filters
    cannot hide data or execute code during identity verification.
    """
    validate_lock(lock)
    root = Path(sources).resolve(strict=True)
    if not root.is_dir() or any(path.name not in {"checkouts", "archives", "inputs"}
                                for path in root.iterdir()):
        _fail("sources", "expected only checkouts, archives and inputs")
    checkouts = root / "checkouts"
    git_sources = {name: source for name, source in lock["sources"].items() if source["kind"] == "git"}
    if checkouts.is_symlink() or (git_sources and not checkouts.is_dir()):
        _fail("sources.checkouts", "missing ordinary checkout directory")
    if checkouts.exists() and {path.name for path in checkouts.iterdir()} != git_sources.keys():
        _fail("sources.checkouts", "checkout identities do not match the lock")
    for name, source in git_sources.items():
        _verify_git(checkouts / name, source, f"sources.{name}")
    archives = {name: (source["sha256"], source["bytes"])
                for name, source in lock["sources"].items() if source["kind"] == "archive"}
    _verify_files(root / "archives", archives, "sources.archives")
    _verify_files(root / "inputs", {name: (digest, None) for name, digest in lock["inputs"].items()},
                  "sources.inputs")


def main(arguments: list[str] | None = None) -> int:
    """Run the two public validation commands; report named failures without tracebacks or outputs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("validate-lock", "verify-sources"))
    parser.add_argument("--lock", required=True, type=Path)
    parser.add_argument("--sources", type=Path)
    args = parser.parse_args(arguments)
    if args.operation == "verify-sources" and args.sources is None:
        parser.error("verify-sources requires --sources")
    if args.operation == "validate-lock" and args.sources is not None:
        parser.error("--sources is only valid with verify-sources")
    try:
        lock, _ = load_lock(args.lock)
        if args.operation == "verify-sources":
            if os.getuid() == 0:
                _fail("verify-sources", "source verification requires a non-root caller")
            verify_sources(lock, args.sources)
    except (SupportError, OSError, UnicodeError) as error:
        print(f"support: {error}", file=sys.stderr)
        return 1
    print("support: lock valid" if args.operation == "validate-lock" else "support: sources verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

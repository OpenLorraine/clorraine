"""Independent core-archive data and bounded external version-query fixtures."""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path


BINARIES = ("clc", "clorraine", "clorraine-lsp")
"""Executable names supplied directly to the archive producer."""
NOTICES = ("LICENSE", "LICENSE-EXCEPTION.md", "NOTICE", "TRADEMARKS.md", "THIRD_PARTY_NOTICES.md")
"""Notice files read from the checkout owning the producer."""
PAYLOAD = tuple(f"bin/{name}" for name in BINARIES) + NOTICES
"""The eight indexed payload files; core.json is not indexed by itself."""


def repository() -> Path:
    """Locate the producer checkout without reading any implementation file."""
    return Path(__file__).resolve().parents[3]


def current_version() -> str:
    """Read the current package version without adding a TOML dependency."""
    text = (repository() / "compiler/Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"$', text, re.MULTILINE)
    if match is None:
        raise AssertionError("compiler/Cargo.toml must have a three-part package version")
    return match[1]


def scratch_parent() -> Path:
    """Keep local generated fixtures in the approved scratch tree, not small /tmp."""
    path = Path(os.environ.get("CODEOPS_TMPDIR", str(repository() / "build/tmp")))
    path.mkdir(parents=True, exist_ok=True)
    return path.resolve()


def digest_file(path: Path) -> str:
    """Hash arbitrary-size actual binaries and archives with bounded memory."""
    with path.open("rb") as source:
        return digest_stream(source)


def digest_stream(source) -> str:
    """Hash one file-like byte stream without loading an archive member at once."""
    result = hashlib.sha256()
    for block in iter(lambda: source.read(1024 * 1024), b""):
        result.update(block)
    return result.hexdigest()


def json_bytes(value: object) -> bytes:
    """Encode the compact sorted JSON used by preparation-inventory identities."""
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def read_json_bytes(data: bytes) -> dict:
    """Reject duplicate keys rather than silently choosing their final value."""
    def unique(pairs):
        """Build exactly one value per object key, including nested records."""
        result = {}
        for key, value in pairs:
            if key in result:
                raise AssertionError(f"duplicate JSON key: {key}")
            result[key] = value
        return result
    result = json.loads(data.decode("utf-8"), object_pairs_hook=unique)
    if not isinstance(result, dict):
        raise AssertionError("metadata must be a JSON object")
    return result


def read_json(path: Path) -> dict:
    """Read one ordinary metadata file independently of production validators."""
    if path.is_symlink() or not path.is_file():
        raise AssertionError(f"metadata must be an ordinary file: {path}")
    return read_json_bytes(path.read_bytes())


def elf_x64(path: Path) -> None:
    """Check the actual ELF class, byte order, machine and executable/PIE type."""
    if path.is_symlink() or not path.is_file() or not os.access(path, os.X_OK):
        raise AssertionError(f"ordinary executable required: {path}")
    with path.open("rb") as source:
        header = source.read(64)
    if (len(header) != 64 or header[:6] != b"\x7fELF\x02\x01"
            or int.from_bytes(header[16:18], "little") not in (2, 3)
            or int.from_bytes(header[18:20], "little") != 62):
        raise AssertionError(f"Linux x86_64 executable ELF required: {path}")


def inventory(binaries: Path, notices: Path) -> dict:
    """Describe the exact approved input bytes, independently of core.json."""
    result = {}
    for name in PAYLOAD:
        path = binaries / name[4:] if name.startswith("bin/") else notices / name
        result[name] = {
            "bytes": path.stat().st_size,
            "sha256": digest_file(path),
            "executable": name.startswith("bin/"),
        }
    return result


def inspect_core(path: Path, expected: dict | None = None, version: str | None = None) -> dict:
    """Check exact members, metadata, bytes and normalized PAX-compatible tar headers.

    Short ordinary names use the common POSIX header representation; the check
    does not infer which writer API produced otherwise identical valid bytes.
    """
    with path.open("rb") as source:
        if source.read(512)[257:265] != b"ustar\x0000":
            raise AssertionError("core archive needs an uncompressed POSIX/PAX-compatible header")
    with tarfile.open(path, "r:") as archive:
        members = archive.getmembers()
        names = [member.name for member in members]
        if len(names) != len(set(names)) or set(names) != set(PAYLOAD) | {"core.json"}:
            raise AssertionError(f"core archive must contain only its nine ordinary files: {names}")
        for member in members:
            executable = member.name.startswith("bin/")
            if (not member.isfile() or member.mode != (0o755 if executable else 0o644)
                    or (member.uid, member.gid, member.mtime) != (0, 0, 0)
                    or member.uname != "" or member.gname != ""):
                raise AssertionError(f"invalid core member type/mode/ownership/time: {member.name}")
        core = read_json_bytes(archive.extractfile("core.json").read())
        if (set(core) != {"schema", "core_version", "files"}
                or type(core["schema"]) is not int or core["schema"] != 1
                or not isinstance(core["core_version"], str)
                or re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", core["core_version"]) is None
                or not isinstance(core["files"], dict) or set(core["files"]) != set(PAYLOAD)):
            raise AssertionError("invalid core.json shape/version/file coverage")
        if version is not None and core["core_version"] != version:
            raise AssertionError("core.json must record the current input version")
        for name in PAYLOAD:
            record = core["files"][name]
            if (not isinstance(record, dict) or set(record) != {"bytes", "sha256", "executable"}
                    or type(record["bytes"]) is not int or record["bytes"] <= 0
                    or not isinstance(record["sha256"], str)
                    or re.fullmatch(r"[0-9a-f]{64}", record["sha256"]) is None
                    or type(record["executable"]) is not bool
                    or record["executable"] != name.startswith("bin/")):
                raise AssertionError(f"invalid core file record: {name}")
            member = archive.getmember(name)
            if member.size != record["bytes"]:
                raise AssertionError(f"core member length mismatch: {name}")
            if digest_stream(archive.extractfile(member)) != record["sha256"]:
                raise AssertionError(f"core member hash mismatch: {name}")
            if expected is not None and record != expected[name]:
                raise AssertionError(f"input bytes were not preserved: {name}")
    return core


def invoke(test, operation: str, arguments: list, cwd: Path, environment: dict,
           timeout: int = 30) -> subprocess.CompletedProcess[str]:
    """Require the public command before checking its behavior, never its source."""
    entry = repository() / "tools/distribution/package_core.py"
    test.assertTrue(entry.is_file(), f"Missing public command: {entry}")
    return subprocess.run(
        [sys.executable, str(entry), operation, *(str(value) for value in arguments)],
        cwd=cwd, env=environment, stdin=subprocess.DEVNULL,
        capture_output=True, text=True, timeout=timeout)


_VERSION_SOURCE = r'''
/* Tiny executable test data, not a compiler or language server. */
#define _POSIX_C_SOURCE 200809L
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/** Preserve the running executable's inode bytes before private cleanup. */
static int record_executable(const char *root) {
    char path[4096];
    unsigned char block[16384];
    size_t count;
    int failed = 0;
    int length = snprintf(path, sizeof(path), "%s/%s.elf", root, CORE_NAME);
    if (length < 0 || length >= (int)sizeof(path)) return 84;
    FILE *input = fopen("/proc/self/exe", "rb");
    if (input == NULL) return 85;
    FILE *output = fopen(path, "wb");
    if (output == NULL) {
        fclose(input);
        return 86;
    }
    while ((count = fread(block, 1, sizeof(block), input)) != 0) {
        if (fwrite(block, 1, count, output) != count) {
            failed = 1;
            break;
        }
    }
    if (ferror(input)) failed = 1;
    if (fclose(input) != 0) failed = 1;
    if (fclose(output) != 0) failed = 1;
    return failed ? 87 : 0;
}

/** Report a controlled version and log literal argv without shell expansion. */
int main(int argc, char **argv) {
    char key[128], path[4096], actual[4096];
    const char *root = getenv("CORE_QUERY_LOG");
    int exit_code = 0, signal_exit = 0;
    if (argc != 2 || strcmp(argv[1], "--version") != 0) return 81;
    if (root != NULL) {
        FILE *log;
        snprintf(path, sizeof(path), "%s/%s.argv", root, CORE_NAME);
        log = fopen(path, "wb");
        if (log == NULL) return 82;
        ssize_t size = readlink("/proc/self/exe", actual, sizeof(actual) - 1);
        if (size < 0) return 83;
        actual[size] = '\0';
        fwrite(actual, 1, (size_t)size + 1, log);
        fwrite(argv[0], 1, strlen(argv[0]) + 1, log);
        fwrite(argv[1], 1, strlen(argv[1]) + 1, log);
        fclose(log);
        int recorded = record_executable(root);
        if (recorded != 0) return recorded;
    }
    snprintf(key, sizeof(key), "CORE_SIGNAL_%s", CORE_NAME);
    signal_exit = getenv(key) != NULL;
    snprintf(key, sizeof(key), "CORE_STATUS_%s", CORE_NAME);
    if (getenv(key) != NULL) exit_code = atoi(getenv(key));
    snprintf(key, sizeof(key), "CORE_EMPTY_%s", CORE_NAME);
    if (getenv(key) != NULL) return 0;
    snprintf(key, sizeof(key), "CORE_NAME_%s", CORE_NAME);
    const char *name = getenv(key);
    snprintf(key, sizeof(key), "CORE_VERSION_%s", CORE_NAME);
    const char *version = getenv(key);
    printf("%s %s\n", name == NULL ? CORE_NAME : name,
           version == NULL ? CORE_VERSION : version);
    fflush(stdout);
    if (signal_exit) raise(SIGTERM);
    return exit_code;
}
'''
"""C fixture source compiled before the producer is invoked."""

_GUARD_SOURCE = r'''
"""Block build/network/publication tools in the local packaging fixture."""
import json
import os
from pathlib import Path
import sys
with open(os.environ["CORE_GUARD_LOG"], "a", encoding="utf-8") as stream:
    stream.write(json.dumps({"tool": Path(sys.argv[0]).name}) + "\n")
raise SystemExit(93)
'''
"""Safety guard for external operations the archive command never needs."""


class CoreFixture:
    """Own tiny ELF inputs, output/log directories and a child-only environment.

    ``binaries`` contains three compiled version helpers. ``expected`` records
    their bytes and the real current notice inputs. ``output`` starts absent.
    ``queries`` and ``guard_log`` expose actual external calls. No fake Python
    interpreter or undocumented producer subprocess protocol is introduced.
    """

    def __init__(self, root: Path) -> None:
        """Compile test data using the existing C compiler before installing guards."""
        self.root = root
        self.version = current_version()
        self.manifest_bytes = (repository() / "compiler/Cargo.toml").read_bytes()
        self.binaries = root / "binary inputs ; literal"
        self.queries = root / "version queries"
        self.published = root / "published"
        self.temporary = root / "private work"
        self.guard_log = root / "forbidden-processes.jsonl"
        for path in (self.binaries, self.queries, self.published, self.temporary):
            path.mkdir()
        self.output = self.published / "core ; literal.tar"
        source = root / "version-fixture.c"
        source.write_text(_VERSION_SOURCE, encoding="utf-8")
        compiler = shutil.which("cc")
        if compiler is None:
            raise AssertionError("existing cc is required to compile ELF test data")
        for name in BINARIES:
            subprocess.run(
                [compiler, "-std=c99", "-Wall", "-Wextra", "-Werror",
                 f"-DCORE_NAME={json.dumps(name)}", f"-DCORE_VERSION={json.dumps(self.version)}",
                 "-o", str(self.binaries / name), str(source)],
                check=True, capture_output=True, text=True, timeout=30)
            elf_x64(self.binaries / name)
        self.expected = inventory(self.binaries, repository())
        self.environment = {
            key: value for key, value in os.environ.items()
            if not key.startswith(("PYTHON", "CORE_", "GIT_", "GH_", "GITHUB_", "SSH_", "LD_"))
        }
        guards = root / "external guards"
        guards.mkdir()
        for name in (*BINARIES, "cc", "gcc", "clang", "cargo", "rustc", "make", "cmake",
                     "curl", "wget", "git", "gh", "ssh", "docker", "sudo", "apt", "apt-get"):
            path = guards / name
            path.write_text("#!" + sys.executable + "\n" + _GUARD_SOURCE, encoding="utf-8")
            path.chmod(0o755)
        home = root / "private home"
        home.mkdir()
        self.environment.update(
            PATH=str(guards) + os.pathsep + os.environ.get("PATH", ""),
            HOME=str(home), PYTHONDONTWRITEBYTECODE="1",
            TMPDIR=str(self.temporary), CODEOPS_TMPDIR=str(self.temporary),
            CORE_QUERY_LOG=str(self.queries), CORE_GUARD_LOG=str(self.guard_log))

    def archive(self, test, output: Path | None = None):
        """Invoke only the public archive command with literal input/output paths."""
        return invoke(test, "archive", [
            "--binaries", self.binaries, "--output", output or self.output],
            self.root, self.environment)

    def assert_no_side_effects(self, test) -> None:
        """Reject compilation/network/publication and abandoned private staging."""
        test.assertFalse(self.guard_log.exists(), "producer invoked an unrelated external tool")
        test.assertEqual(list(self.temporary.iterdir()), [], "producer abandoned private work")
        for name in NOTICES:
            path = repository() / name
            test.assertEqual(digest_file(path), self.expected[name]["sha256"],
                             f"producer changed input {name}")
        test.assertEqual((repository() / "compiler/Cargo.toml").read_bytes(), self.manifest_bytes)

    def binary_state(self) -> dict:
        """Snapshot caller inputs after intentional mutations, without following links."""
        result = {}
        for path in self.binaries.rglob("*"):
            relative = path.relative_to(self.binaries).as_posix()
            if path.is_symlink():
                result[relative] = ("symlink", os.readlink(path))
            elif path.is_file():
                result[relative] = ("file", digest_file(path), path.stat().st_mode & 0o7777)
            elif path.is_dir():
                result[relative] = ("directory",)
            else:
                result[relative] = ("special", path.lstat().st_mode)
        return result

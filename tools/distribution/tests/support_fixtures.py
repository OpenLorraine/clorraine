"""Synthetic support inputs and a failing external Docker process.

These fixtures exercise metadata and failure handling. They do not establish
toolchain completeness, redistribution permission or actual network isolation.
"""

from __future__ import annotations

import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path


def digest(data: bytes) -> str:
    """Return the SHA-256 identity of exact fixture bytes."""
    return hashlib.sha256(data).hexdigest()


def json_bytes(value: object) -> bytes:
    """Encode deterministic UTF-8 JSON, including its final newline."""
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


# Only Docker is replaced. Its run operation reports a failed acquisition and,
# when mounted, leaves partial private work for the build command to clean up.
_DOCKER_BODY = r'''
"""Report a failed external container operation without contacting Docker."""
import json
import os
import re
import shutil
import sys
from pathlib import Path

args = sys.argv[1:]
with open(os.environ["SUPPORT_DOCKER_LOG"], "a", encoding="utf-8") as log:
    log.write(json.dumps(args) + "\n")

def options(*names):
    """Read ordinary Docker long/short option spellings."""
    values = []
    for index, argument in enumerate(args):
        for name in names:
            if argument == name and index + 1 < len(args):
                values.append(args[index + 1])
            elif argument.startswith(name + "="):
                values.append(argument[len(name) + 1:])
    return values

if args[:2] == ["image", "inspect"] or args[:1] == ["inspect"]:
    base_reference = os.environ["SUPPORT_DOCKER_IMAGE"]
    base_id = "sha256:" + "2" * 64
    prepared_id = os.environ["SUPPORT_DOCKER_PREPARED_ID"]
    references = [value for value in args if value in (base_reference, base_id, prepared_id)]
    if len(references) != 1:
        sys.exit("Docker fixture requires one known image identity: " + repr(args))
    prepared = references[0] == prepared_id
    layers = ["sha256:" + "3" * 64]
    labels = {}
    if prepared:
        layers.append("sha256:" + "4" * 64)
        labels = {
            "org.clorraine.base-image": os.environ.get("SUPPORT_DOCKER_BASE_LABEL", base_reference),
            "org.clorraine.environment-recipe-sha256": os.environ.get(
                "SUPPORT_DOCKER_RECIPE_LABEL", os.environ["SUPPORT_DOCKER_RECIPE_SHA256"]),
        }
        # Named fixture overrides corrupt one observation without faking an image build.
        if os.environ.get("SUPPORT_DOCKER_WRONG_BASE_PREFIX") == "1":
            layers[0] = "sha256:" + "f" * 64
    image = {
        "Id": os.environ.get("SUPPORT_DOCKER_INSPECTED_ID", prepared_id) if prepared else base_id,
        "Os": "linux", "Architecture": "amd64",
        "RepoDigests": [] if prepared else [base_reference],
        "RootFS": {"Type": "layers", "Layers": layers},
        "Config": {"Labels": labels, "Env": []},
    }
    if not prepared and os.environ.get("SUPPORT_DOCKER_SHORT_BASE_NAME") == "1":
        image["RepoDigests"] = [base_reference.removeprefix("docker.io/library/")]
    formats = options("--format", "-f")
    if not formats:
        print(json.dumps([image]))
    else:
        def field(match):
            """Render image fields using Docker's ordinary inspect notation."""
            expression = match.group(1).strip()
            encoded = expression.startswith("json ")
            expression = expression[5:].strip() if encoded else expression
            value = image
            for key in expression.lstrip(".").split(".") if expression != "." else []:
                value = value[key]
            return json.dumps(value) if encoded else str(value)
        print(re.sub(r"{{(.*?)}}", field, formats[-1]))
    sys.exit(0)

if args[:1] == ["run"]:
    mounts = []
    for mount in options("--mount"):
        fields = dict(part.split("=", 1) for part in mount.split(",") if "=" in part)
        mounts.append((fields.get("src", fields.get("source", "")),
                       fields.get("dst", fields.get("target", fields.get("destination", "")))))
    for mount in options("--volume", "-v"):
        parts = mount.split(":")
        if len(parts) >= 2:
            mounts.append((parts[0], parts[1]))
    for source, target in mounts:
        if target == "/work":
            work = Path(source).resolve()
            output = Path(os.environ["SUPPORT_DOCKER_OUTPUT"]).resolve()
            if work != output and output not in work.parents:
                sys.exit("Docker fixture refuses work outside its owned output")
            if os.environ.get("SUPPORT_DOCKER_SUCCESS") == "1":
                shutil.copytree(os.environ["SUPPORT_DOCKER_PAYLOAD"],
                                work / "payload", symlinks=True)
                if os.environ.get("SUPPORT_DOCKER_PAYLOAD_FIFO") == "1":
                    os.mkfifo(work / "payload/bin/channel")
            else:
                (work / "payload").mkdir(parents=True, exist_ok=True)
                (work / "payload" / "partial").write_bytes(b"incomplete build\n")
    if os.environ.get("SUPPORT_DOCKER_SUCCESS") == "1":
        sys.exit(0)
    sys.stderr.write("recipe.sh: undeclared fetch https://example.invalid/undeclared.tar blocked\n")
    sys.exit(73)

sys.exit("unexpected Docker operation: " + repr(args))
'''


class SupportFixture:
    """Own a miniature source stage, payload and their independent metadata.

    ``root`` owns every generated file. ``sources`` contains ``checkout`` and
    hashed archives/inputs; ``payload`` is a separate candidate copy. ``lock``
    and ``audit`` are mutable JSON data, persisted at ``lock_path`` and
    ``audit_path`` by :meth:`save`. ``build_output`` and ``archive_output`` are
    initially absent. ``image_id`` identifies the synthetic prepared image.
    ``docker_log`` records external calls; ``environment`` redirects Docker and
    temporary files into this fixture. Neither image represents installed tools.
    """

    def __init__(self, root: Path) -> None:
        """Create real Git/archive inputs and the complete synthetic metadata."""
        self.root = root
        self.sources = root / "staged sources"
        self.checkout = self.sources / "checkouts" / "lha"
        self.payload = root / "candidate payload"
        self.lock_path = root / "support-lock.json"
        self.audit_path = root / "audit.json"
        self.build_output = root / "build output"
        self.archive_output = root / "artifacts" / "support.tar"
        self.docker_log = root / "docker.jsonl"
        self.image_id = "sha256:" + "1" * 64
        helper = b"#!/bin/sh\nexit 0\n"
        licence = b"Synthetic fixture licence.\n"
        review = b"Synthetic review; not publication approval.\n"
        excluded = b"Synthetic excluded NDK-derived header.\n"
        recipe = b"#!/bin/sh\nset -eu\ncp -R /sources/checkouts/lha/payload /work/payload\n"
        environment_recipe = b"ARG BASE_IMAGE\nFROM ${BASE_IMAGE}\n"
        files = [("bin/lha", helper, "program"), ("notices/lha.txt", licence, "notice"),
                 ("reviews/lha.txt", review, "review"), ("sources/lha.sh", helper, "source")]
        self.checkout.mkdir(parents=True)
        (self.checkout / "LICENSE").write_bytes(licence)
        for name, content, _ in files:
            path = self.checkout / "payload" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        (self.checkout / "payload/bin/lha").chmod(0o755)
        self.git("init", "--object-format=sha1", "--initial-branch=fixture")
        self.git("remote", "add", "origin", "https://example.invalid/lha.git")
        self.git("add", ".")
        self.git("commit", "-m", "Synthetic source")
        commit = self.git("rev-parse", "HEAD").strip()
        shutil.copytree(self.checkout / "payload", self.payload)
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode="w", format=tarfile.USTAR_FORMAT) as archive:
            member = tarfile.TarInfo("LICENSE")
            member.size, member.mode = len(licence), 0o644
            archive.addfile(member, io.BytesIO(licence))
        archive_bytes = buffer.getvalue()
        (self.sources / "archives").mkdir()
        (self.sources / "archives/seed").write_bytes(archive_bytes)
        inputs = {"recipe.sh": recipe, "prepare.sh": helper, "review.txt": review,
                  "environment.Dockerfile": environment_recipe}
        (self.sources / "inputs").mkdir()
        for name, content in inputs.items():
            (self.sources / "inputs" / name).write_bytes(content)
        terms = {"id": "LicenseRef-Fixture", "notice": "LICENSE",
                 "source_required": True, "review": "review.txt"}
        package = {"version": "1", "sonames": [], "paths": ["/bin/sh"]}
        self.lock = {
            "schema": 1,
            "sources": {
                "lha": {"kind": "git", "url": "https://example.invalid/lha.git",
                        "commit": commit, "licence": terms.copy()},
                "seed": {"kind": "archive", "url": "https://example.invalid/seed.tar",
                         "version": "1", "bytes": len(archive_bytes),
                         "sha256": digest(archive_bytes), "licence": terms.copy()}},
            "inputs": {name: digest(content) for name, content in inputs.items()},
            "build": {"image": "docker.io/library/ubuntu@sha256:" + "0" * 64,
                      "environment_recipe": "environment.Dockerfile",
                      "ubuntu": "22.04", "packages": {"dash": "1"},
                      "tools": {"python3": "3.10.0"},
                      "argv": ["/bin/sh", "/sources/inputs/recipe.sh"]},
            "preparation": {
                "archive": "seed", "script": "prepare.sh", "patches": [],
                "helpers": {"lha": {"source": "lha", "path": "bin/lha"}},
                "inventory": {"include/fixture-ndk.h": digest(excluded)}},
            "output": {
                "architecture": "x86_64", "python_abis": ["cp310", "cp312"],
                "runtimes": ["nix13", "nix20"],
                "runtime_packages": {"22.04": {"dash": package}, "24.04": {"dash": package}},
                "builder_prefixes": ["/sources", "/work"],
                "forbidden_sha256": [digest(excluded)]}}
        self.audit = {
            "schema": 1, "lock_sha256": "",
            "components": {"lha": {"review": "reviews/lha.txt",
                                  "notices": ["notices/lha.txt"], "source": ["sources/lha.sh"]}},
            "files": {name: {"type": "file", "sha256": digest(content),
                             "origins": ["lha"], "role": role} for name, content, role in files},
            "native": {}}
        self.archive_output.parent.mkdir()
        fake_bin = root / "external tools"
        fake_bin.mkdir()
        docker = fake_bin / "docker"
        docker.write_text("#!" + sys.executable + "\n" + _DOCKER_BODY, encoding="utf-8")
        docker.chmod(0o755)
        self.environment = os.environ.copy()
        for name in list(self.environment):
            if name in ("PYTHONPATH", "PYTHONHOME") or name.startswith(("GIT_", "SUPPORT_DOCKER_")):
                del self.environment[name]
        self.environment.update({
            "PATH": str(fake_bin) + os.pathsep + os.environ.get("PATH", ""),
            "PYTHONDONTWRITEBYTECODE": "1", "TMPDIR": str(root), "CODEOPS_TMPDIR": str(root),
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
            "SUPPORT_DOCKER_LOG": str(self.docker_log),
            "SUPPORT_DOCKER_OUTPUT": str(self.build_output),
            "SUPPORT_DOCKER_PAYLOAD": str(self.payload),
            "SUPPORT_DOCKER_IMAGE": self.lock["build"]["image"],
            "SUPPORT_DOCKER_PREPARED_ID": self.image_id,
            "SUPPORT_DOCKER_RECIPE_SHA256": digest(environment_recipe)})
        self.save()

    def git(self, *arguments: str) -> str:
        """Run real Git without inherited hooks, signing or identity settings."""
        environment = {name: value for name, value in os.environ.items()
                       if not name.startswith("GIT_")}
        environment.update({
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
            "GIT_ATTR_NOSYSTEM": "1", "GIT_TERMINAL_PROMPT": "0",
            "GIT_AUTHOR_NAME": "Fixture", "GIT_COMMITTER_NAME": "Fixture",
            "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
            "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
            "GIT_AUTHOR_DATE": "2000-01-01T00:00:00+0000",
            "GIT_COMMITTER_DATE": "2000-01-01T00:00:00+0000"})
        result = subprocess.run(
            ["git", "-C", str(self.checkout), "-c", "core.hooksPath=/dev/null",
             "-c", "commit.gpgSign=false", "-c", "core.autocrlf=false", *arguments],
            env=environment, capture_output=True, text=True, timeout=30, check=True)
        return result.stdout

    def save(self) -> None:
        """Persist metadata and bind the audit to the newly written lock bytes."""
        content = json_bytes(self.lock)
        self.lock_path.write_bytes(content)
        self.audit["lock_sha256"] = digest(content)
        self.audit_path.write_bytes(json_bytes(self.audit))

    def add_file(self, name: str, content: bytes, role: str, origin: str = "lha") -> None:
        """Add or replace a payload file and its matching origin/hash record."""
        path = self.payload / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        self.audit["files"][name] = {
            "type": "file", "sha256": digest(content), "origins": [origin], "role": role}

    def remove_file(self, name: str) -> None:
        """Remove bytes and inventory entry while retaining obligation references."""
        path = self.payload / name
        path.unlink()
        del self.audit["files"][name]
        parent = path.parent
        while parent != self.payload and not any(parent.iterdir()):
            parent.rmdir()
            parent = parent.parent

    def add_component(self, source: str) -> None:
        """Supply complete notice/review/source records for a second locked origin."""
        for directory, role, content in (
                ("reviews", "review", (self.payload / "reviews/lha.txt").read_bytes()),
                ("notices", "notice", (self.payload / "notices/lha.txt").read_bytes()),
                ("sources", "source", b"Synthetic corresponding source.\n")):
            self.add_file(f"{directory}/{source}.txt", content, role, source)
        self.audit["components"][source] = {
            "review": f"reviews/{source}.txt", "notices": [f"notices/{source}.txt"],
            "source": [f"sources/{source}.txt"]}

    def docker_calls(self) -> list[list[str]]:
        """Return calls to the external fixture; an absent log means no invocation."""
        if not self.docker_log.exists():
            return []
        return [json.loads(line) for line in self.docker_log.read_text().splitlines()]

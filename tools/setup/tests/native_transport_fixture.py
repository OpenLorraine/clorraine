"""Small process fixture binding exact producer bytes to the shipped bootstrap."""

from __future__ import annotations

import json
import os
import resource
import signal
import shutil
import subprocess
import sys
from pathlib import Path

from native_manifest_data import API_ROOT, DOWNLOAD_ROOT, digest, encode, file_digest, release_response


def limit_files() -> None:
    """Cap each child output file at one GiB, including grandchildren."""
    resource.setrlimit(resource.RLIMIT_FSIZE, (1024 ** 3, 1024 ** 3))


def run_process(argv: list[str], cwd: Path, environment: dict[str, str],
                timeout: int = 45) -> subprocess.CompletedProcess[str]:
    """Run literal arguments with bounded elapsed time and bounded log files."""
    stdout = cwd / "process.stdout"
    stderr = cwd / "process.stderr"
    with stdout.open("w") as out, stderr.open("w") as err:
        process = subprocess.Popen(argv, cwd=cwd, env=environment, stdin=subprocess.DEVNULL,
                                   stdout=out, stderr=err, start_new_session=True,
                                   preexec_fn=limit_files)
        try:
            status = process.wait(timeout=timeout)
        except BaseException:
            # A timed-out shell may have a Python child. Kill this invocation's
            # whole session before cleanup so no orphan can keep writing files.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=5)
            raise
    return subprocess.CompletedProcess(argv, status,
                                       stdout.read_text(errors="replace"),
                                       stderr.read_text(errors="replace"))


class ShellFixture:
    """Own copied shell/helper bytes, private HTTP routes and command observations."""

    def __init__(self, root: Path, bootstrap: Path, manifest: dict,
                 payloads: dict[str, Path], *, probes: dict | None = None) -> None:
        """Keep inputs untouched and copy only the shipped entry assets into isolation."""
        self.root = root
        self.manifest = manifest
        self.log = root / "transport.jsonl"
        self.config_path = root / "transport.json"
        self.home = root / "home ; private"
        self.install_root = root / "selected root ; with spaces"
        self.temporary = root / "temporary"
        self.shipped = root / "shipped"
        self.shims = root / "shims"
        for directory in (self.home, self.temporary, self.shipped, self.shims):
            directory.mkdir()
        for name in ("install.sh", "install.pyz", "manifest.json"):
            shutil.copyfile(bootstrap / name, self.shipped / name)
        self.original_hashes = {name: digest((self.shipped / name).read_bytes())
                                for name in ("install.sh", "install.pyz", "manifest.json")}
        self.payloads = payloads
        self.payload_hashes = {identity: file_digest(path) for identity, path in payloads.items()}
        runner = root / "runner.py"
        shutil.copyfile(Path(__file__).with_name("native_transport_runner.py"), runner)
        shim_source = Path(__file__).with_name("native_transport_shims.py").read_bytes()
        for name in ("curl", "python3", "code", "sudo", "apt", "apt-get", "wget",
                     "cargo", "rustc", "make", "cmake", "docker", "gh"):
            path = self.shims / name
            path.write_bytes(("#!" + sys.executable + "\n").encode() + shim_source)
            path.chmod(0o755)
        self.configuration = {
            "python": sys.executable, "runner": str(runner), "log": str(self.log),
            "helper": str(self.shipped / "install.pyz"),
            "helper_url": DOWNLOAD_ROOT + manifest["release"] + "/install.pyz",
            "routes": {}, "probes": probes or {},
            "original_hashes": self.original_hashes,
            "original_payload_hashes": self.payload_hashes,
        }
        self.route(API_ROOT + "/releases/tags/" + manifest["release"],
                   encode(release_response(manifest["release"])))
        self.route(API_ROOT + "/releases?per_page=100",
                   encode([release_response(manifest["release"])]))
        self.route(DOWNLOAD_ROOT + manifest["release"] + "/manifest.json",
                   path=self.shipped / "manifest.json")
        for asset in manifest["assets"]:
            self.route(asset["url"], path=payloads[asset["identity"]])
        self.environment = {key: value for key, value in os.environ.items()
                            if not key.startswith(("PYTHON", "CLORRAINE", "RDC_", "LD_",
                                                   "GIT_", "GH_", "GITHUB_", "SSH_"))}
        self.environment.update(HOME=str(self.home), PATH=str(self.shims) + ":/usr/bin:/bin",
                                TMPDIR=str(self.temporary), CODEOPS_TMPDIR=str(self.temporary),
                                PYTHONDONTWRITEBYTECODE="1")

    def route(self, url: str, data: bytes | None = None, *, path: Path | None = None,
              status: int = 200, headers: dict[str, str] | None = None,
              **fault: object) -> None:
        """Supply exact untrusted response bytes, headers or an external fault."""
        route = {"status": status, "headers": headers or {}, **fault}
        if data is not None:
            path = self.root / ("body-" + str(len(list(self.root.glob("body-*")))))
            path.write_bytes(data)
        if path is not None:
            route["body"] = str(path)
        self.configuration["routes"][url] = [route]

    def invoke(self, arguments: list[str], *, root: Path | None = None,
               timeout: int = 45) -> subprocess.CompletedProcess[str]:
        """Execute the unchanged shell, never a native-installer private function."""
        target = root or self.install_root
        self.configuration["bootstrap_arguments"] = ["--root", str(target), *arguments]
        self.config_path.write_bytes(encode(self.configuration))
        return run_process(["/bin/sh", str(self.shipped / "install.sh"),
                            "--root", str(target), *arguments], self.root, self.environment, timeout)

    def events(self, kind: str | None = None) -> list[dict]:
        """Read only fixture-owned command and HTTP observations."""
        records = [json.loads(line) for line in self.log.read_text().splitlines()
                   ] if self.log.exists() else []
        return [item for item in records if kind is None or item["kind"] == kind]

    def assert_failed_safely(self, test, result: subprocess.CompletedProcess[str]) -> None:
        """Require named failure, no activation and no automatic host/editor changes."""
        test.assertNotEqual(result.returncode, 0, result.stdout)
        test.assertTrue(result.stderr.strip(), "failure must name its step and remedy")
        test.assertFalse((self.install_root / "current").is_symlink(), "failed run activated")
        test.assertFalse((self.install_root / "current").exists(), "failed run selected a candidate")
        forbidden = {"sudo", "apt", "apt-get", "code", "make", "cargo", "rustc", "docker", "gh"}
        test.assertFalse([item for item in self.events("command") if item["name"] in forbidden],
                         "installer launched a forbidden host/editor command")
        for name, expected in self.original_hashes.items():
            test.assertEqual(digest((self.shipped / name).read_bytes()), expected,
                             "installer altered the shipped candidate")
        for identity, path in self.payloads.items():
            test.assertEqual(file_digest(path), self.payload_hashes[identity],
                             "installer altered a caller's payload input")

    def requested(self, url: str) -> bool:
        """Prove a fault reached its intended transport boundary before rejection."""
        return any(item["url"] == url for item in self.events("request"))

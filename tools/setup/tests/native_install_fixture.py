"""Owned process sessions and explicit external-boundary state-test controls."""

from __future__ import annotations

import os
import shutil
import signal
import subprocess
import time
import uuid
from pathlib import Path

from native_manifest_data import encode, file_digest
from native_transport_fixture import ShellFixture, limit_files


class OwnedInstaller:
    """Own one installer session and its bounded stdout/stderr file handles."""

    def __init__(self, shell: StateShellFixture, arguments: list[str], root: Path) -> None:
        """Start only this fixture's unchanged shell with literal public arguments."""
        self.shell = shell
        shell.prepare(arguments, root)
        self.invocation = shell.configuration["state_controls"]["invocation"]
        self.stdout_path = shell.root / (self.invocation + ".stdout")
        self.stderr_path = shell.root / (self.invocation + ".stderr")
        self.stdout = self.stdout_path.open("w")
        self.stderr = self.stderr_path.open("w")
        self.closed = False
        try:
            self.process = subprocess.Popen(
                ["/bin/sh", str(shell.shipped / "install.sh"), "--root", str(root), *arguments],
                cwd=shell.root, env=shell.environment, stdin=subprocess.DEVNULL,
                stdout=self.stdout, stderr=self.stderr, start_new_session=True,
                preexec_fn=limit_files)
        except BaseException:
            self.stdout.close()
            self.stderr.close()
            raise

    def close(self) -> None:
        """Kill and join only this owned process group, including lingering children."""
        if self.closed:
            return
        self.closed = True
        try:
            os.killpg(self.process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        self.process.wait(timeout=5)
        self.stdout.close()
        self.stderr.close()

    def wait(self, timeout: int = 600) -> subprocess.CompletedProcess[str]:
        """Wait finitely and return the external status and private log contents."""
        try:
            status = self.process.wait(timeout=timeout)
        except BaseException:
            self.close()
            raise
        self.stdout.close()
        self.stderr.close()
        return subprocess.CompletedProcess(self.process.args, status,
                                           self.stdout_path.read_text(errors="replace"),
                                           self.stderr_path.read_text(errors="replace"))

    def wait_ready(self, event: str, timeout: int = 120) -> Path:
        """Require an invocation-specific recorded boundary, not a guessed file/pointer."""
        ready = self.shell.signals / (self.invocation + "." + event)
        deadline = time.monotonic() + timeout
        while not ready.exists():
            if self.process.poll() is not None:
                result = self.wait(timeout=5)
                raise AssertionError("installer exited before " + event + ": " + result.stderr)
            if time.monotonic() >= deadline:
                raise AssertionError("installer never reached " + event)
            time.sleep(0.01)
        records = self.shell.events(event)
        if not any(item.get("invocation") == self.invocation for item in records):
            raise AssertionError("ready signal has no corresponding recorded syscall boundary")
        return ready

    def release(self, event: str) -> None:
        """Release only this session's selected fixture barrier."""
        path = self.shell.signals / (self.invocation + "." + event + ".release")
        path.write_bytes(b"release\n")


class StateShellFixture(ShellFixture):
    """Extend copied transport fixtures without changing any frozen source file.

    ``fault``, ``barrier`` and ``http_url`` are private test controls. Leaving
    them unset uses the ordinary transport runner without process/rename
    overrides. Actual acceptance also leaves compatibility probes unset.
    """

    def __init__(self, root: Path, bootstrap: Path, manifest: dict,
                 payloads: dict[str, Path], *, probes: dict | None = None) -> None:
        """Copy standalone test runners into owned storage, never into the pyz."""
        super().__init__(root, bootstrap, manifest, payloads, probes=probes)
        self.signals = root / "signals"
        self.signals.mkdir()
        shutil.copyfile(root / "runner.py", root / "transport_base.py")
        shutil.copyfile(Path(__file__).with_name("native_install_runner.py"), root / "state_runner.py")
        self.fault: dict | None = None
        self.barrier: str | None = None
        self.http_url: str | None = None

    def prepare(self, arguments: list[str], root: Path) -> None:
        """Write one private invocation record; no control is passed to the installer."""
        active = self.fault is not None or self.barrier is not None or self.http_url is not None
        if active:
            self.configuration["runner"] = str(self.root / "state_runner.py")
            self.configuration["state_controls"] = {
                "invocation": uuid.uuid4().hex, "root": str(root.resolve()),
                "signals": str(self.signals), "fault": self.fault,
                "barrier": self.barrier, "http_url": self.http_url,
            }
        else:
            self.configuration["runner"] = str(self.root / "runner.py")
            self.configuration.pop("state_controls", None)
        self.configuration["bootstrap_arguments"] = ["--root", str(root), *arguments]
        self.config_path.write_bytes(encode(self.configuration))

    def invoke(self, arguments: list[str], *, root: Path | None = None,
               timeout: int = 600) -> subprocess.CompletedProcess[str]:
        """Execute a synchronous call with optional negative-only external observers."""
        target = root or self.install_root
        self.prepare(arguments, target)
        return super().invoke(arguments, root=target, timeout=timeout)

    def start(self, arguments: list[str], *, root: Path | None = None) -> OwnedInstaller:
        """Start an observed session that the test must register for cleanup immediately."""
        return OwnedInstaller(self, arguments, root or self.install_root)

    def assert_fault_reached(self, test, operation: str) -> None:
        """Require the intended failure child, failed lookup or external launch/load error."""
        controls = self.configuration.get("state_controls", {})
        invocation = controls.get("invocation")
        records = [item for item in self.events() if item.get("invocation") == invocation]
        matches = [item for item in records if item["kind"] == "fault-match" and
                   item.get("operation") == operation]
        test.assertEqual(len(matches), 1, "did not reach the selected external failure operation")
        observed = [item for item in records if item["kind"] in
                    ("fault-child", "launch-error", "load-error") and
                    item.get("operation") == operation]
        test.assertEqual(len(observed), 1, "selected failure was not actually observed")
        if observed[0]["kind"] == "fault-child":
            test.assertEqual(observed[0]["exit"], 83)


def tree_state(root: Path) -> dict[str, tuple]:
    """Snapshot bytes, modes and links without following aliases into other trees."""
    result = {}
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        if path.is_symlink():
            result[relative] = ("link", os.readlink(path))
        elif path.is_file():
            result[relative] = ("file", path.stat().st_mode & 0o7777, file_digest(path))
        elif path.is_dir():
            result[relative] = ("directory", path.stat().st_mode & 0o7777)
        else:
            result[relative] = ("special", path.lstat().st_mode)
    return result

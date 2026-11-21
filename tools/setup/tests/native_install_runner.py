"""Copied negative-test observers around real external process and rename calls.

The exact zip application still owns validation, extraction, completion and
activation. A selected operation can fail or wait at an OS boundary; this
runner cannot supply a successful compiler, preparation, wheel or smoke result.
"""

from __future__ import annotations

import ctypes
import errno
import functools
import json
import os
import re
import runpy
import subprocess
import sys
import time
from pathlib import Path


_FAILURE_CHILD = r'''
import json
import os
import sys
log, invocation, operation, diagnostic = sys.argv[1:]
with open(log, "a", encoding="utf-8") as output:
    output.write(json.dumps({"kind": "fault-child", "invocation": invocation,
                             "operation": operation, "diagnostic": diagnostic,
                             "pid": os.getpid(), "exit": 83}) + "\n")
print(diagnostic, file=sys.stderr, flush=True)
raise SystemExit(83)
'''
"""Real, always-failing subprocess code; its arguments are fixture-owned data."""


def semantic_operation(argv: list[str], smoke_seen: bool) -> str:
    """Identify documented public command roles without depending on staging names."""
    if not argv:
        return ""
    name = Path(argv[0]).name
    if name == "dpkg-query" and "-S" not in argv and "--search" not in argv:
        return "packages"
    if "amiga" not in name and not smoke_seen and re.search(r"(^|-)(cc|gcc)(-\d+)?$", name):
        return "compiler"
    if any(Path(value).name == "prepare_ndk.py" for value in argv):
        if "--archive" in argv and "--destination" in argv:
            return "preparation"
    if "-m" in argv:
        index = argv.index("-m")
        module = argv[index + 1] if index + 1 < len(argv) else ""
        if module == "venv":
            return "venv"
        if module == "pip" and "install" in argv and "-I" in argv:
            spelling = Path(argv[0])
            if spelling.parent.name == "bin" and spelling.parent.parent.name == "vamos":
                return "wheels"
    if "-c" in argv:
        index = argv.index("-c")
        code = argv[index + 1] if index + 1 < len(argv) else ""
        if re.search(r"\b(import\s+venv|from\s+venv\s+import)\b", code):
            return "venv"
    if name == "clorraine" and len(argv) > 1 and argv[1] == "run":
        profile = "host"
        for index, value in enumerate(argv[2:], 2):
            if value in ("-p", "--profile") and index + 1 < len(argv):
                profile = argv[index + 1]
            elif value.startswith("--profile="):
                profile = value.partition("=")[2]
        return "smoke-" + profile
    return ""


class BoundaryObservers:
    """Own one invocation's log, selected fault and bounded synchronization files."""

    def __init__(self, configuration: dict, record) -> None:
        """Retain only test-owned controls, never an installer metadata record."""
        self.configuration = configuration
        self.controls = configuration["state_controls"]
        self.record_transport = record
        self.used = False
        self.smoke_seen = False

    def record(self, kind: str, **details: object) -> None:
        """Attach a unique invocation and PID to each externally observed event."""
        self.record_transport(self.configuration, {
            "kind": kind, "invocation": self.controls["invocation"],
            "pid": os.getpid(), **details,
        })

    def signal(self, event: str, **details: object) -> None:
        """Record the exact boundary before making its ready signal visible."""
        self.record(event, **details)
        path = Path(self.controls["signals"]) / (self.controls["invocation"] + "." + event)
        path.write_text(json.dumps({"event": event, "pid": os.getpid()}), encoding="utf-8")
        selected = self.controls.get("barrier") == event
        selected = selected or (event == "http-ready" and self.controls.get("http_url"))
        if selected:
            release = path.with_suffix(path.suffix + ".release")
            deadline = time.monotonic() + 30
            while not release.exists():
                if time.monotonic() >= deadline:
                    raise TimeoutError("bounded fixture barrier expired: " + event)
                time.sleep(0.01)

    def install_process_observers(self) -> None:
        """Fail only the selected semantic operation; forward every other subprocess."""
        original = subprocess.Popen
        fault = self.controls.get("fault") or {}

        @functools.wraps(original)
        def popen(args, *positional, **keywords):
            """Observe literal argv/CWD and return a real process or launch error."""
            if keywords.get("shell") or (len(positional) > 7 and positional[7]):
                raise AssertionError("installer subprocess must not use shell=True")
            if isinstance(args, (str, bytes, os.PathLike)):
                # A single executable spelling is one literal argument, not a
                # shell command to split or evaluate.
                argv = [os.fsdecode(args)]
            elif isinstance(args, (list, tuple)):
                argv = [os.fsdecode(value) for value in args]
            else:
                raise AssertionError("installer subprocess must use literal argument elements")
            operation = semantic_operation(argv, self.smoke_seen)
            self.smoke_seen = self.smoke_seen or operation.startswith("smoke-")
            cwd = os.fsdecode(keywords.get("cwd") or os.getcwd())
            candidate = Path(argv[0]).parent.parent if argv else Path(".")
            adjacent = (bool(argv) and Path(argv[0]).is_absolute() and
                        (candidate / "sdk/amiga").is_dir())
            self.record("subprocess", argv=argv, cwd=cwd, operation=operation,
                        candidate_sdk=adjacent)
            c_input = any(value == "-" or value.endswith(".c") or
                          value in ("-x", "-xc", "-E", "-fsyntax-only", "-c", "-S")
                          for value in argv[1:])
            selected_probe = not fault.get("header_probe") or c_input
            if not self.used and operation and operation == fault.get("operation") and selected_probe:
                self.used = True
                diagnostic = fault["diagnostic"]
                self.record("fault-match", argv=argv, cwd=cwd, operation=operation,
                            diagnostic=diagnostic, mode=fault.get("mode", "child"))
                if fault.get("mode") == "missing":
                    self.record("launch-error", operation=operation, diagnostic=diagnostic)
                    raise FileNotFoundError(errno.ENOENT, diagnostic, argv[0])
                replacement = [sys.executable, "-I", "-c", _FAILURE_CHILD,
                               self.configuration["log"], self.controls["invocation"],
                               operation, diagnostic]
                forwarded = dict(keywords)
                if "executable" in forwarded:
                    forwarded["executable"] = sys.executable
                return original(replacement, *positional, **forwarded)
            return original(args, *positional, **keywords)

        subprocess.Popen = popen
        original_load = ctypes.CDLL

        @functools.wraps(original_load)
        def load(name, *positional, **keywords):
            """Model only a failed load of the one declared system SONAME."""
            spelling = os.fsdecode(name) if isinstance(name, (str, bytes)) else ""
            if (not self.used and fault.get("operation") == "runtime" and
                    Path(spelling).name == fault.get("soname")):
                self.used = True
                self.record("fault-match", operation="runtime", soname=spelling,
                            diagnostic=fault["diagnostic"])
                self.record("load-error", operation="runtime", soname=spelling)
                raise OSError(fault["diagnostic"])
            return original_load(name, *positional, **keywords)

        ctypes.CDLL = load
    def install_rename_observers(self) -> None:
        """Observe only the selected root's final current directory entry."""
        root = Path(self.controls["root"]).resolve()

        def targets_current(destination, directory_fd) -> bool:
            """Resolve the parent via its fd/CWD without following current itself."""
            try:
                path = Path(os.fsdecode(destination))
                if path.is_absolute():
                    parent = path.parent
                else:
                    base = (Path.cwd() if directory_fd in (None, -100) else
                            Path(os.readlink("/proc/self/fd/" + str(directory_fd))))
                    parent = base / path.parent
                return path.name == "current" and parent.resolve() == root
            except (OSError, TypeError, ValueError):
                # Invalid arguments must retain the real rename's own error.
                return False

        def wrap(original):
            """Preserve arguments, directory fds, result and exception semantics."""
            @functools.wraps(original)
            def rename(source, destination, *positional, **keywords):
                """Signal immediately around the actual atomic replacement call."""
                if not targets_current(destination, keywords.get("dst_dir_fd")):
                    return original(source, destination, *positional, **keywords)
                self.signal("before-current", source=os.fsdecode(source),
                            destination=os.fsdecode(destination))
                result = original(source, destination, *positional, **keywords)
                self.signal("after-current", destination=os.fsdecode(destination))
                return result
            return rename

        os.rename = wrap(os.rename)
        os.replace = wrap(os.replace)


def main() -> None:
    """Compose a private copy of the frozen HTTP fixture and enter the exact pyz."""
    configuration = json.loads(Path(__file__).with_name("transport.json").read_bytes())
    transport = runpy.run_path(str(Path(__file__).with_name("transport_base.py")),
                               run_name="copied_transport_fixture")
    controls = configuration.get("state_controls")
    if controls:
        observers = BoundaryObservers(configuration, transport["record"])
        observers.record("state-start")
        observers.install_process_observers()
        observers.install_rename_observers()
        original_response = transport["FixtureResponse"]

        class GatedResponse(original_response):
            """Pause one real response read while the installer retains its OS lock."""

            def read(self, size: int = -1) -> bytes:
                """Gate only the explicitly selected URL, then stream unchanged bytes."""
                if (self.url == controls.get("http_url") and
                        not getattr(self, "_state_gate_reached", False)):
                    self._state_gate_reached = True
                    observers.signal("http-ready", url=self.url)
                return super().read(size)

        transport["main"].__globals__["FixtureResponse"] = GatedResponse
    transport["main"]()


if __name__ == "__main__":
    main()

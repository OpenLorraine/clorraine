"""External curl/Python/command observation shims copied into private test PATH."""

from __future__ import annotations

import hashlib
import json
import os
import sys
import zipfile
from pathlib import Path


def _record(configuration: dict, value: dict) -> None:
    """Record an external command without reaching installer implementation state."""
    with Path(configuration["log"]).open("a", encoding="utf-8") as output:
        output.write(json.dumps(value, sort_keys=True) + "\n")


def _curl(configuration: dict, arguments: list[str]) -> int:
    """Deliver helper bytes while leaving all shell length/hash checks intact."""
    urls = [value.partition("=")[2] if value.startswith("--url=") else value
            for value in arguments
            if value.startswith(("http://", "https://", "--url="))]
    if urls != [configuration["helper_url"]]:
        raise AssertionError("unexpected bootstrap helper URL: " + repr(urls))
    mode = configuration.get("curl_fault")
    if mode == "timeout":
        return 28
    data = Path(configuration["helper"]).read_bytes()
    if mode == "corrupt":
        index = len(data) // 2
        data = data[:index] + bytes([data[index] ^ 1]) + data[index + 1:]
    elif mode == "truncate":
        data = data[:-1]
    _record(configuration, {"kind": "helper-delivery", "bytes": len(data),
                             "sha256": hashlib.sha256(data).hexdigest()})
    destination = None
    for index, value in enumerate(arguments):
        if value in ("-o", "--output"):
            destination = arguments[index + 1]
        elif value.startswith("--output="):
            destination = value.partition("=")[2]
        elif value.startswith("-o") and not value.startswith("--") and len(value) > 2:
            destination = value[2:]
    if destination is None or destination == "-":
        sys.stdout.buffer.write(data)
    else:
        Path(destination).write_bytes(data)
    return 0


def main() -> None:
    """Dispatch by the copied external command name, never by manifest content."""
    configuration = json.loads(
        (Path(__file__).resolve().parent.parent / "transport.json").read_bytes())
    name = Path(sys.argv[0]).name
    arguments = sys.argv[1:]
    _record(configuration, {"kind": "command", "name": name, "argv": arguments})
    if name == "curl":
        raise SystemExit(_curl(configuration, arguments))
    if name == "python3":
        events = [json.loads(line) for line in Path(configuration["log"]).read_text().splitlines()]
        is_handoff = (len(arguments) >= 2 and arguments[0] == "-I" and
                      Path(arguments[1]).is_file() and zipfile.is_zipfile(arguments[1]))
        if not is_handoff and any(item["kind"] == "runner" for item in events):
            # Later ordinary interpreter probes/venv subprocesses are real
            # processes, not bootstrap handoffs and not simulated successes.
            os.execv(configuration["python"], [configuration["python"], *arguments])
        if not is_handoff:
            raise AssertionError("bootstrap must hand off as python3 -I PYZ ARGS")
        if arguments[2:] != configuration["bootstrap_arguments"]:
            raise AssertionError("bootstrap changed the literal user arguments")
        pyz = Path(arguments[1])
        if not pyz.is_file():
            raise AssertionError("bootstrap did not supply a downloaded helper file")
        _record(configuration, {"kind": "helper-execution", "argv": arguments,
                                 "sha256": hashlib.sha256(pyz.read_bytes()).hexdigest()})
        os.execv(configuration["python"], [
            configuration["python"], "-I", configuration["runner"],
            str(pyz), *arguments[2:]])
    if name == "code":
        raise SystemExit(0)
    raise SystemExit("forbidden external fixture command: " + name)


if __name__ == "__main__":
    main()

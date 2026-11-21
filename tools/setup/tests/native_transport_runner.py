"""Copied, isolated HTTP-open fixture for the shipped Python zip application.

Only the external HTTP boundary and explicitly recorded local compatibility
probes are replaced. The real zip application's validation, extraction, state,
subprocess and activation paths remain in charge.
"""

from __future__ import annotations

import collections
import hashlib
import io
import json
import math
import platform
import runpy
import socket
import sys
import urllib.request
from email.message import Message
from pathlib import Path


def record(configuration: dict, value: dict) -> None:
    """Append bounded observations in private fixture storage."""
    with Path(configuration["log"]).open("a", encoding="utf-8") as output:
        output.write(json.dumps(value, sort_keys=True) + "\n")


class FixtureResponse:
    """Stream one untrusted response without interpreting its payload.

    This object implements only the usual HTTP response protocol. A response
    fault can raise a real timeout, but cannot approve any installer operation.
    """

    def __init__(self, url: str, route: dict, configuration: dict) -> None:
        """Open exact fixture bytes and retain the supplied status and headers."""
        self.url = url
        self.status = route.get("status", 200)
        self.headers = Message()
        for name, value in route.get("headers", {}).items():
            self.headers[name] = value
        self.route = route
        self.configuration = configuration
        self.body = (Path(route["body"]).open("rb") if "body" in route else io.BytesIO())

    def getcode(self) -> int:
        """Return the unmodified HTTP status."""
        return self.status

    def geturl(self) -> str:
        """Return the URL of this response, not its Location destination."""
        return self.url

    def info(self) -> Message:
        """Expose normal case-insensitive response headers."""
        return self.headers

    def read(self, size: int = -1) -> bytes:
        """Read actual body bytes or inject an external connection timeout."""
        if self.route.get("read_error") == "timeout":
            record(self.configuration, {"kind": "read-timeout", "url": self.url})
            raise socket.timeout("fixture stalled connection")
        return self.body.read(size)

    def close(self) -> None:
        """Release the response's private file handle."""
        self.body.close()

    def __enter__(self) -> FixtureResponse:
        """Support the HTTP response context-manager protocol."""
        return self

    def __exit__(self, *_exception: object) -> None:
        """Close the stream even if production validation rejects it."""
        self.close()


def main() -> None:
    """Run exact helper bytes in isolated mode using only fixture HTTP responses."""
    configuration = json.loads(Path(__file__).with_name("transport.json").read_bytes())
    routes = configuration["routes"]
    counts: dict[str, int] = {}

    def open_response(_opener: object, request: object, data: bytes | None = None,
                      timeout: object = socket._GLOBAL_DEFAULT_TIMEOUT) -> FixtureResponse:
        """Replace only outbound HTTP opening and reject every unexpected URL."""
        url = request.full_url if isinstance(request, urllib.request.Request) else str(request)
        finite = isinstance(timeout, (int, float)) and not isinstance(timeout, bool)
        finite = finite and math.isfinite(timeout) and timeout > 0
        method = request.get_method() if isinstance(request, urllib.request.Request) else "GET"
        record(configuration, {"kind": "request", "url": url, "finite_timeout": finite,
                               "method": method})
        if method != "GET" or data is not None:
            raise AssertionError("installer attempted a non-download HTTP operation")
        if url not in routes:
            raise AssertionError("unexpected external request: " + url)
        sequence = routes[url]
        index = counts.get(url, 0)
        counts[url] = index + 1
        route = sequence[min(index, len(sequence) - 1)]
        if route.get("error") == "timeout":
            raise socket.timeout("fixture connection timeout")
        body = route.get("body")
        if body:
            result = hashlib.sha256()
            with Path(body).open("rb") as source:
                for block in iter(lambda: source.read(1024 * 1024), b""):
                    result.update(block)
            record(configuration, {"kind": "response-bytes", "url": url,
                                   "sha256": result.hexdigest()})
        return FixtureResponse(url, route, configuration)

    urllib.request.OpenerDirector.open = open_response
    probes = configuration.get("probes", {})
    if probes:
        # These identities model a rejected local invocation or failed download,
        # never a supported runtime. Actual acceptance supplies no overrides.
        if "ubuntu" in probes:
            platform.freedesktop_os_release = lambda: {
                "ID": "ubuntu", "VERSION_ID": probes["ubuntu"]}
        if "architecture" in probes:
            platform.machine = lambda: probes["architecture"]
        if "python" in probes:
            version = collections.namedtuple(
                "FixtureVersion", "major minor micro releaselevel serial")
            sys.version_info = version(*probes["python"], "final", 0)
    record(configuration, {"kind": "runner", "sys_path": list(sys.path),
                           "probes": probes, "original_hashes": configuration["original_hashes"],
                           "original_payload_hashes": configuration["original_payload_hashes"]})
    pyz, *arguments = sys.argv[1:]
    sys.argv = [pyz, *arguments]
    runpy.run_path(pyz, run_name="__main__")


if __name__ == "__main__":
    main()

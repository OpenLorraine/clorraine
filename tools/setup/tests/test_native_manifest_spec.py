"""Independent manifest, release selection, transport and extraction contracts.

Positive installation and usable printed commands have a separate mandatory
real-context entry. Local responses exercise failures, not supported-runtime
acceptance, and every process case first requires valid production assembly.
"""

from __future__ import annotations

import copy
import json
import math
import unittest

from native_manifest_data import (
    API_ROOT, DOWNLOAD_ROOT, RELEASE, digest, encode, release_response,
)
from native_manifest_archive_spec import NativeArchiveBoundaryTests
from native_manifest_producer_fixture import NativeProcessCase
from native_manifest_structure_spec import ManifestStructureTests


def _https_only(specification: str) -> bool:
    """Evaluate curl's additive/removal protocol notation without pinning flag spelling."""
    enabled = {"all"}
    for item in specification.lower().split(","):
        if item.startswith("="):
            enabled = {item[1:]}
        elif item == "-all":
            enabled.clear()
        elif item.startswith("-"):
            enabled.discard(item[1:])
        else:
            enabled.add(item.lstrip("+"))
    return enabled == {"https"}


class NativeReleaseTransportTests(NativeProcessCase):
    """Exercise unchanged shipped entry assets through external process/HTTP fixtures."""

    def _payload_failure(self, mode: str) -> None:
        """Reach a named core download fault without ever selecting an incomplete root."""
        fixture, shell = self._new()
        asset = next(item for item in fixture.manifest["assets"] if item["kind"] == "core")
        data = fixture.raw["core.tar"]
        if mode == "corrupt":
            data = bytes([data[0] ^ 1]) + data[1:]
        elif mode == "truncate":
            data = data[:-1]
        elif mode == "length":
            data += b"extra"
        shell.route(asset["url"], data, **({"read_error": "timeout"} if mode == "timeout" else {}))
        result = shell.invoke(["--release", RELEASE])
        shell.assert_failed_safely(self, result)
        self.assertTrue(shell.requested(asset["url"]), "faulted payload was never requested")
        self.assertIn("core", result.stderr.lower())

    def test_should_preserve_spaced_root_and_exact_revision_as_literal_shell_arguments(self) -> None:
        """Local handoff checks do not stand in for positive installed-command proof."""
        _, shell = self._new()
        before = {}
        for name in (".profile", ".bashrc", ".vscode/extensions/sentinel"):
            path = shell.home / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"user settings must survive\n")
            before[path] = path.read_bytes()
        shell.route(DOWNLOAD_ROOT + RELEASE + "/manifest.json", b"invalid JSON")
        result = shell.invoke(["--release", RELEASE])
        shell.assert_failed_safely(self, result)
        executions = shell.events("helper-execution")
        self.assertEqual(len(executions), 1)
        self.assertEqual(executions[0]["argv"][2:],
                         ["--root", str(shell.install_root), "--release", RELEASE])
        for path, data in before.items():
            self.assertEqual(path.read_bytes(), data)

    def test_should_select_latest_published_distribution_across_lookup_pages(self) -> None:
        """Newer support/component/draft/prerelease records do not win lookup."""
        _, shell = self._new()
        next_url = API_ROOT + "/releases?per_page=100&page=2"
        first = [release_response("support-fixture-support", published="2027-01-01T00:00:00Z"),
                 release_response("components-fixture-core", published="2027-01-02T00:00:00Z"),
                 release_response("v1.9.0", draft=True, published="2027-01-03T00:00:00Z"),
                 release_response("v1.8.0", prerelease=True, published="2027-01-04T00:00:00Z")]
        shell.route(API_ROOT + "/releases?per_page=100", encode(first),
                    headers={"Link": '<' + next_url + '>; rel="next"'})
        shell.route(next_url, encode([release_response(RELEASE),
                                     release_response("v1.7.0", published="2025-01-01T00:00:00Z")]))
        selected = DOWNLOAD_ROOT + RELEASE + "/manifest.json"
        shell.route(selected, b"invalid JSON after selection")
        result = shell.invoke([])
        shell.assert_failed_safely(self, result)
        self.assertTrue(shell.requested(next_url))
        self.assertTrue(shell.requested(selected))
        self.assertFalse(any("/releases/latest" in item["url"] for item in shell.events("request")))
        self.assertFalse(any(item["url"].endswith("/install.pyz")
                             for item in shell.events("request")), "helper identity changed during lookup")

    def test_should_reject_exact_component_or_support_tags_without_payload_use(self) -> None:
        """Only published distribution tags are selectable through the public CLI."""
        for tag in ("components-fixture-core", "support-fixture-support"):
            with self.subTest(tag=tag):
                _, shell = self._new()
                result = shell.invoke(["--release", tag])
                shell.assert_failed_safely(self, result)
                self.assertTrue(shell.events("helper-execution"), "did not reach real helper")
                self.assertIn(tag, result.stderr)
                self.assertFalse(any(item["url"].endswith("/manifest.json")
                                     for item in shell.events("request")))

    def test_should_reject_unknown_missing_and_path_like_arguments(self) -> None:
        """Malformed public arguments cannot activate or trigger a source fallback."""
        for arguments in (["--unknown"], ["--root"], ["--release"],
                          ["--release", "../v1.2.0"], ["--release", "v1.2.0/next"]):
            with self.subTest(arguments=arguments):
                _, shell = self._new()
                result = shell.invoke(arguments)
                shell.assert_failed_safely(self, result)
                self.assertTrue(shell.events("helper-execution"), "did not reach real helper")
                self.assertFalse(shell.events("request"), "invalid invocation fetched metadata")

    def test_should_reject_unsupported_architecture_and_python_without_abi_fallback(self) -> None:
        """Recorded external identity probes still face the real compatibility checks."""
        for probes in ({"architecture": "aarch64"}, {"python": [3, 11, 0]}):
            with self.subTest(probes=probes):
                _, shell = self._new(**probes)
                result = shell.invoke(["--release", RELEASE])
                shell.assert_failed_safely(self, result)
                self.assertTrue(shell.events("runner"), "helper did not reach recorded probes")
                self.assertTrue(any(word in result.stderr.lower() for word in
                                    ("architecture", "aarch64", "python", "3.11", "supported")))

    def test_should_reject_corrupt_or_truncated_helpers_before_python_execution(self) -> None:
        """The unchanged shell's own length/hash checks protect its executable handoff."""
        for fault in ("corrupt", "truncate", "timeout"):
            with self.subTest(fault=fault):
                _, shell = self._new()
                shell.configuration["curl_fault"] = fault
                result = shell.invoke(["--release", RELEASE])
                shell.assert_failed_safely(self, result)
                self.assertEqual(len([item for item in shell.events("command")
                                      if item["name"] == "curl"]), 1)
                if fault != "timeout":
                    self.assertEqual(len(shell.events("helper-delivery")), 1)
                self.assertFalse([item for item in shell.events("command")
                                  if item["name"] == "python3"], "unverified helper reached Python")
                self.assertFalse(shell.events("helper-execution"), "unverified helper executed")
                self.assertFalse(shell.events("runner"))

    def test_should_configure_bootstrap_certificate_verification_and_https_timeouts(self) -> None:
        """External curl receives finite deadlines and cannot disable TLS verification."""
        _, shell = self._new()
        shell.route(DOWNLOAD_ROOT + RELEASE + "/manifest.json", b"invalid JSON")
        result = shell.invoke(["--release", RELEASE])
        shell.assert_failed_safely(self, result)
        curl = [item for item in shell.events("command") if item["name"] == "curl"]
        self.assertEqual(len(curl), 1)
        arguments = curl[0]["argv"]
        self.assertFalse(any(value in ("-k", "--insecure") for value in arguments))
        values = {}
        for index, value in enumerate(arguments):
            if value.startswith("--"):
                key, separator, inline = value.partition("=")
                values[key] = inline if separator else (
                    arguments[index + 1] if index + 1 < len(arguments) else "")
            elif value == "-m" and index + 1 < len(arguments):
                values["--max-time"] = arguments[index + 1]
            elif value.startswith("-m") and len(value) > 2:
                values["--max-time"] = value[2:]
        for key in ("--proto", "--proto-redir"):
            self.assertTrue(_https_only(values.get(key, "")), key + " permits non-HTTPS transport")
        for key in ("--connect-timeout", "--max-time"):
            value = float(values.get(key, "0"))
            self.assertTrue(math.isfinite(value) and value > 0)
        self.assertTrue(any(value in ("--fail", "--fail-with-body") or
                            (value.startswith("-") and not value.startswith("--") and "f" in value)
                            for value in arguments), "curl must fail on HTTP errors")
        self.assertFalse(any(value.startswith("-") and not value.startswith("--") and "k" in value
                             for value in arguments), "curl must not disable certificate checks")
        self.assertTrue(shell.events("helper-execution"), "did not exercise a valid shell handoff")

    def test_should_reject_a_changed_payload_byte(self) -> None:
        """A matching length does not excuse an incorrect asset digest."""
        self._payload_failure("corrupt")

    def test_should_reject_a_truncated_payload(self) -> None:
        """An early EOF cannot produce a complete component."""
        self._payload_failure("truncate")

    def test_should_stop_a_download_exceeding_its_declared_length(self) -> None:
        """A response longer than the pinned asset is rejected."""
        self._payload_failure("length")

    def test_should_bound_stalled_connections_with_finite_transport_timeouts(self) -> None:
        """A timed-out body read must fail and each open must carry a finite deadline."""
        self._payload_failure("timeout")
        # The assertions above prove that the intended body was requested;
        # inspect each scenario's copied runner log rather than a private constant.
        for log in self.root.glob("scenario-*/shell/transport.jsonl"):
            requests = [json.loads(line) for line in log.read_text().splitlines()]
            self.assertTrue(all(item["finite_timeout"] for item in requests
                                if item["kind"] == "request"))

    def test_should_reject_https_downgrade_before_requesting_the_location(self) -> None:
        """The production redirect loop, not the fixture, must reject insecure URLs."""
        fixture, shell = self._new()
        asset = next(item for item in fixture.manifest["assets"] if item["kind"] == "core")
        outside = "http://example.invalid/unverified-core.tar"
        shell.route(asset["url"], status=302, headers={"Location": outside})
        shell.route(outside, fixture.raw["core.tar"])
        result = shell.invoke(["--release", RELEASE])
        shell.assert_failed_safely(self, result)
        self.assertTrue(shell.requested(asset["url"]))
        self.assertFalse(shell.requested(outside), "downgrade was followed before validation")

    def test_should_follow_safe_https_redirects_before_verifying_downloaded_bytes(self) -> None:
        """A downgrade rejection cannot pass merely by rejecting every redirect."""
        fixture, shell = self._new()
        asset = next(item for item in fixture.manifest["assets"] if item["kind"] == "core")
        destination = "https://example.invalid/redirected-core.tar"
        shell.route(asset["url"], status=302, headers={"Location": destination})
        shell.route(destination, b"changed verified component bytes")
        result = shell.invoke(["--release", RELEASE])
        shell.assert_failed_safely(self, result)
        self.assertTrue(shell.requested(asset["url"]))
        self.assertTrue(shell.requested(destination), "safe redirect was rejected before download")
        self.assertIn("core", result.stderr.lower())

    def test_should_bind_core_vsix_and_prepared_inventory_to_downloaded_metadata(self) -> None:
        """Well-formed metadata still has to agree with actual component contents."""
        for key, kind in (("core_version", "core"), ("extension_version", "vsix"),
                          ("prepared_inventory_sha256", "support-lock")):
            with self.subTest(key=key):
                fixture, shell = self._new()
                manifest = copy.deepcopy(fixture.manifest)
                if key == "prepared_inventory_sha256":
                    manifest["support"][key] = digest(b"different inventory")
                else:
                    manifest[key] = "7.8.9"
                shell.route(DOWNLOAD_ROOT + RELEASE + "/manifest.json", encode(manifest))
                result = shell.invoke(["--release", RELEASE])
                shell.assert_failed_safely(self, result)
                asset = next(item for item in manifest["assets"] if item["kind"] == kind)
                self.assertTrue(shell.requested(asset["url"]), "did not compare actual component")


if __name__ == "__main__":
    unittest.main()

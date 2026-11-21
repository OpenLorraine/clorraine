"""Delivered support must retain every locked notice, source and review obligation."""

from __future__ import annotations

from native_manifest_data import RELEASE, digest
from native_manifest_producer_fixture import NativeProcessCase
from native_review_payload_fixture import DeliveredSupport, observed_shell


class NativeReviewAuditTests(NativeProcessCase):
    """Reject coherent but incomplete support at the shipped shell/helper boundary."""

    def scenario(self):
        """Require valid producer output before mutating only subsequently delivered bytes."""
        fixture, _shell = self._new()
        self.assertIs(fixture.lock["sources"]["lha"]["licence"]["source_required"], True)
        return fixture, observed_shell(fixture), DeliveredSupport(fixture)

    def reject_before_preparation(self, shell, delivered: DeliveredSupport) -> None:
        """Prove exact hostile payload delivery and reject before any preparation child."""
        manifest = delivered.bind(shell)
        result = shell.invoke(["--release", RELEASE])
        shell.assert_failed_safely(self, result)
        for kind in ("support-audit", "amiga"):
            asset = next(item for item in manifest["assets"]
                         if item["kind"] == kind and
                         (kind == "amiga" or item["url"].endswith("/amiga-audit.json")))
            if kind == "amiga" and not shell.requested(asset["url"]):
                # Rejecting the already downloaded audit before fetching its archive
                # is equally safe and must not be mistaken for a reachability failure.
                continue
            self.assertTrue(shell.requested(asset["url"]), kind + " was never requested")
            observations = [event for event in shell.events("response-bytes")
                            if event["url"] == asset["url"]]
            self.assertTrue(observations, "delivered response lacks exact-byte observation")
            self.assertEqual(observations[-1]["sha256"], asset["sha256"])
        self.assertFalse([event for event in shell.events("subprocess")
                          if event["operation"] == "preparation"],
                         "incomplete locked obligations reached preparation")
        guidance = (result.stdout + result.stderr).lower()
        self.assertTrue(any(word in guidance for word in
                            ("audit", "notice", "source", "review", "origin", "role")))

    def empty_obligation(self, field: str) -> None:
        """Missing, empty and physically stripped obligations must each fail independently."""
        for mutation in ("missing", "empty", "stripped"):
            with self.subTest(mutation=mutation):
                _fixture, shell, delivered = self.scenario()
                component = delivered.audit["components"]["lha"]
                paths = component[field][:]
                if mutation == "missing":
                    del component[field]
                else:
                    component[field] = []
                if mutation == "stripped":
                    for path in paths:
                        del delivered.files[path]
                        del delivered.audit["files"][path]
                self.reject_before_preparation(shell, delivered)

    def test_should_require_a_nonempty_notice_array_for_each_component(self) -> None:
        """Existing support schemas require retained notices even with coherent transport hashes."""
        self.empty_obligation("notices")

    def test_should_require_source_when_the_selected_lock_requires_it(self) -> None:
        """A source-required lock cannot be weakened by removing its delivered source list."""
        self.empty_obligation("source")

    def test_should_require_notice_file_roles(self) -> None:
        """A file named in notices must have the notice role, not merely matching bytes."""
        _fixture, shell, delivered = self.scenario()
        delivered.audit["files"]["notices/lha.txt"]["role"] = "header"
        self.reject_before_preparation(shell, delivered)

    def test_should_require_source_file_roles(self) -> None:
        """A file named as required source must have the source role."""
        _fixture, shell, delivered = self.scenario()
        delivered.audit["files"]["sources/lha.sh"]["role"] = "program"
        self.reject_before_preparation(shell, delivered)

    def test_should_require_notice_origin_to_include_its_component(self) -> None:
        """Another real locked origin cannot satisfy this component's notice obligation."""
        _fixture, shell, delivered = self.scenario()
        delivered.audit["files"]["notices/lha.txt"]["origins"] = ["seed"]
        self.reject_before_preparation(shell, delivered)

    def test_should_require_source_origin_to_include_its_component(self) -> None:
        """Another locked source identity cannot satisfy LHA's required source obligation."""
        _fixture, shell, delivered = self.scenario()
        delivered.audit["files"]["sources/lha.sh"]["origins"] = ["seed"]
        self.reject_before_preparation(shell, delivered)

    def test_should_bind_review_bytes_to_the_selected_lock(self) -> None:
        """A coherent new archive/audit hash must not replace the locked review decision."""
        _fixture, shell, delivered = self.scenario()
        path = delivered.audit["components"]["lha"]["review"]
        delivered.files[path] = b"Changed synthetic review; not the locked review.\n"
        delivered.audit["files"][path]["sha256"] = digest(delivered.files[path])
        self.reject_before_preparation(shell, delivered)

    def test_should_reach_real_failing_preparation_with_intact_obligations(self) -> None:
        """The valid miniature control reaches a real failure, never fake runtime success."""
        _fixture, shell, delivered = self.scenario()
        delivered.bind(shell)
        result = shell.invoke(["--release", RELEASE])
        shell.assert_failed_safely(self, result)
        preparation = [event for event in shell.events("subprocess")
                       if event["operation"] == "preparation"]
        self.assertEqual(len(preparation), 1, "valid fixture never reached actual preparation")
        self.assertIn("synthetic failure fixture", result.stderr)
        self.assertFalse(shell.events("fault-match"), "no child result may be substituted")

"""Post-removal checks for the retained native inputs and the new pipeline.

The old native installer, support/distribution producers, their tests, and the
release workflows are removed after acceptance. This test keeps the retained
licence/origin inputs, notices, and the Docker SDK setup honest.
"""

from __future__ import annotations

import hashlib
import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
INPUTS = REPO / "tools/distribution/inputs"
REMOVED = [
    "tools/setup/install_native.py",
    "tools/setup/native_archive.py",
    "tools/setup/native_state.py",
    "tools/distribution/package_core.py",
    "tools/distribution/package_release.py",
    "tools/distribution/package_support.py",
    "tools/distribution/assemble_support.py",
    "tools/distribution/manifest.py",
    "tools/distribution/prepare_ndk.py",
    "tools/distribution/support.py",
    "tools/distribution/support_audit.py",
    "tools/distribution/support_build.py",
    "tools/distribution/support_native.py",
    "tools/distribution/support-lock.json",
    "tools/distribution/tests",
    "tools/distribution/README.md",
    ".github/workflows/distribution-release.yml",
    ".github/workflows/support-release.yml",
]
RETAINED = [
    "tools/setup/install_sdk.py",
    "tools/setup/tests/support.py",
    "tools/setup/tests/test_cli.py",
    "tools/setup/tests/test_detect.py",
    "tools/setup/tests/test_fetch.py",
    "tools/setup/install.sh",
    "tools/setup/package.sh",
    "tools/setup/acceptance.py",
    "tools/setup/acceptance.sh",
    "tools/distribution/inputs/libamiga.a",
    "tools/distribution/inputs/README.md",
]
CURRENT_DOCS = [
    "README.md",
    "docs/linux-cli.md",
    "docs/known-limitations.md",
    "site/guide/quick-start.md",
    "site/reference/cli.md",
    "site/status.md",
    "RELEASE-NOTES.md",
    "RELEASE.md",
    "AGENTS.md",
]
FORBIDDEN = [
    "install_native.py",
    "native_archive.py",
    "native_state.py",
    "package_release.py",
    "package_core.py",
    "package_support.py",
    "support-lock.json",
    "distribution-check",
    "support-release.yml",
    "distribution-release.yml",
    "install.pyz",
]


class RetainedInputTests(unittest.TestCase):
    def test_removed_paths_are_gone(self) -> None:
        for relative in REMOVED:
            self.assertFalse((REPO / relative).exists(), f"{relative} still exists")

    def test_retained_paths_exist(self) -> None:
        for relative in RETAINED:
            self.assertTrue((REPO / relative).exists(), f"missing retained {relative}")

    def test_libamiga_bytes_are_retained(self) -> None:
        path = INPUTS / "libamiga.a"
        self.assertEqual(path.stat().st_size, 297_694)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        self.assertEqual(
            digest,
            "10d78b6d303a99677c0bf29a8e1653d62669f69f395691646393020325084021",
        )

    def test_retained_notice_paths_resolve(self) -> None:
        notice = (REPO / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
        referenced = set(re.findall(r"tools/distribution/inputs/[A-Za-z0-9_.\/-]+", notice))
        self.assertTrue(referenced, "the notice must name its retained inputs")
        for relative in referenced:
            self.assertTrue((REPO / relative).is_file(), f"missing {relative}")

    def test_retained_readme_does_not_point_at_the_removed_lock(self) -> None:
        readme = (INPUTS / "README.md").read_text(encoding="utf-8")
        self.assertNotIn("support-lock.json", readme)
        for relative in [
            "licence-review.md",
            "build-licences.md",
            "python-licences.md",
            "payload-origins.md",
        ]:
            self.assertIn(relative, readme)

    def test_makefile_has_the_new_pipeline_only(self) -> None:
        makefile = (REPO / "Makefile").read_text(encoding="utf-8")
        for target in ["dist-check:", "dist-verify:", "dist:", "acceptance:", "acceptance-check:"]:
            self.assertIn(target, makefile)
        self.assertNotIn("distribution-check:", makefile)
        self.assertNotIn("$(MAKE) distribution-check", makefile)

    def test_installer_is_the_new_linux_script(self) -> None:
        installer = (REPO / "tools/setup/install.sh").read_text(encoding="utf-8")
        self.assertIn("--archive", installer)
        self.assertIn("@VERSION@", installer)
        self.assertNotIn("@CLORRAINE_HELPER_URL@", installer)
        self.assertNotIn("install.pyz", installer)

    def test_current_docs_have_no_removed_commands(self) -> None:
        for relative in CURRENT_DOCS:
            text = (REPO / relative).read_text(encoding="utf-8")
            for forbidden in FORBIDDEN:
                self.assertNotIn(forbidden, text, f"{relative} mentions {forbidden}")

    def test_release_runbook_uses_the_current_pipeline(self) -> None:
        # The runbook describes the current release; the removed pipeline's
        # commands are kept out of it by the current-docs check above.
        text = (REPO / "RELEASE.md").read_text(encoding="utf-8")
        self.assertIn("make dist", text)
        self.assertIn("publish-vscode", text)


if __name__ == "__main__":
    unittest.main()

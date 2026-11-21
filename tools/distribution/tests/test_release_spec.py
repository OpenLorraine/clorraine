"""Release specification: bounded workflows, immutable identities and promotion gates.

These cases derive from ST-47 through ST-52. They use the public producer and
version/document records; the miniature archives prove metadata and rejection
behavior only, never compiler, Amiga or real-machine acceptance.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import re
import sys
import unittest

from bootstrap_package_fixtures import BootstrapFixture, input_snapshot
from native_manifest_data import (
    COMPONENTS, RELEASE, SUPPORT, VERSION, digest, encode,
)
from native_manifest_producer_fixture import ScratchCase
from native_transport_fixture import run_process


ROOT = Path(__file__).resolve().parents[3]
"""Checkout root that owns version records, docs and workflow definitions."""
PRODUCER = ROOT / "tools/distribution/package_release.py"
"""The only release producer entry exercised by these specifications."""
WORKFLOWS = ROOT / ".github/workflows"
"""Manual release workflow definitions."""
SUPPORT_WORKFLOW = WORKFLOWS / "support-release.yml"
DISTRIBUTION_WORKFLOW = WORKFLOWS / "distribution-release.yml"
"""The exactly two bounded release workflows."""
CHECKOUT_SHA = "11bd71901bbe5b1630ceea73d27597364c9af683"
"""Pinned official checkout action, matching the existing host check."""
PROJECT_LICENCES = ("LICENSE", "LICENSE-EXCEPTION.md", "NOTICE", "TRADEMARKS.md",
                    "THIRD_PARTY_NOTICES.md")
"""First-party terms and notices required in the core archive."""


def _workflow_text(path: Path) -> str:
    """Require a readable workflow definition rather than an absent implicit skip."""
    if not path.is_file():
        raise AssertionError(f"missing release workflow: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def _run_bodies(text: str) -> list[str]:
    """Return literal run-script bodies, including shell-style multiline blocks."""
    lines = text.splitlines()
    bodies: list[str] = []
    index = 0
    while index < len(lines):
        match = re.match(r"^(\s*)(?:-\s*)?run:\s*(.*)$", lines[index])
        if match is None:
            index += 1
            continue
        indent, value = len(match.group(1)), match.group(2).strip()
        if value not in ("|", ">", "|-", ">-"):
            bodies.append(value)
            index += 1
            continue
        index += 1
        collected: list[str] = []
        while index < len(lines):
            line = lines[index]
            if line.strip() and len(line) - len(line.lstrip()) <= indent:
                break
            collected.append(line)
            index += 1
        bodies.append("\n".join(collected))
    return bodies


def _dispatch_inputs(text: str) -> dict[str, bool]:
    """Read required flags for workflow_dispatch inputs from their literal block."""
    result: dict[str, bool] = {}
    lines = text.splitlines()
    in_inputs = False
    current = None
    for line in lines:
        if re.match(r"^\s{2,}inputs:\s*$", line):
            in_inputs = True
            continue
        if in_inputs and line.strip() and len(line) - len(line.lstrip()) <= 2:
            if not line.lstrip().startswith("#"):
                in_inputs = False
        if not in_inputs:
            continue
        name = re.match(r"^\s{4,}([A-Za-z0-9_-]+):\s*$", line)
        if name is not None:
            current = name.group(1)
            result.setdefault(current, False)
            continue
        if current is not None and re.match(r"^\s+required:\s*true\s*$", line):
            result[current] = True
    return result


class ReleaseSpecCase(ScratchCase):
    """Share one valid miniature candidate and never touch caller input bytes."""

    def setUp(self) -> None:
        """A broken producer fails in setup, not inside a rejection assertion."""
        super().setUp()
        self.fixture = BootstrapFixture(self, self.root)

    def _producer(self, arguments: list[str]):
        """Run the public command with no build, fetch, install or publication tool."""
        return run_process([sys.executable, str(PRODUCER), *arguments],
                           self.root, self.fixture.environment)

    def _verify_component(self, existing: Path | None = None,
                          **overrides: object):
        """Bind a component verification request to the fixture's actual bytes."""
        values = {
            "components": COMPONENTS,
            "core": self.fixture.source.paths[COMPONENTS + "-core"],
            "vsix": self.fixture.source.paths[COMPONENTS + "-vsix"],
        }
        values.update(overrides)
        arguments = ["verify-component"]
        for name, value in values.items():
            arguments.extend(["--" + name, str(value)])
        if existing is not None:
            arguments.extend(["--existing", str(existing)])
        return self._producer(arguments)

    def _promotion_assets(self, raw: dict[str, bytes], manifest: dict) -> Path:
        """Write each referenced local asset under its literal URL basename."""
        directory = self.root / ("assets-" + str(len(list(self.root.glob("assets-*")))))
        directory.mkdir()
        for asset in manifest["assets"]:
            if asset["kind"] == "ndk":
                continue  # The upstream NDK is referenced, never redistributed here.
            name = asset["url"].rsplit("/", 1)[-1]
            if name not in raw:
                self.fail("manifest references an unbuilt fixture payload: " + name)
            (directory / name).write_bytes(raw[name])
        return directory

    def _approval(self, manifest_bytes: bytes, manifest: dict, **overrides: object) -> Path:
        """Write the bounded approval record, preserving only the declared schema."""
        value = {
            "schema": 1,
            "release": manifest["release"],
            "manifest_sha256": digest(manifest_bytes),
            "approved": True,
            "unresolved_major": 0,
            "st56": "pending",
        }
        value.update(overrides)
        path = self.root / ("approval-" + str(len(list(self.root.glob("approval-*")))) + ".json")
        path.write_bytes(encode(value))
        return path

    def _promote(self, manifest: Path, assets: Path, approval: Path):
        """Invoke the local pre-promotion eligibility command."""
        return self._producer(["promotion", "--manifest", str(manifest),
                               "--assets", str(assets), "--approval", str(approval)])

    def _ready(self, result) -> dict:
        """Require successful eligibility output and return its decoded record."""
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)


class ReleaseWorkflowSpecTests(ReleaseSpecCase):
    """ST-47: only manual, input-validated, least-privilege release automation."""

    def test_should_declare_exactly_two_manual_dispatch_workflows(self) -> None:
        """No push, pull request, schedule or repository-target trigger is accepted."""
        for path in (SUPPORT_WORKFLOW, DISTRIBUTION_WORKFLOW):
            with self.subTest(workflow=path.name):
                text = _workflow_text(path)
                trigger = text.split("\non:", 1)
                self.assertEqual(len(trigger), 2, "workflow must declare an on block")
                block = re.split(r"\n[A-Za-z]", trigger[1], maxsplit=1)[0]
                self.assertIn("workflow_dispatch:", block)
                self.assertNotRegex(block, r"(?m)^  (?:push|pull_request|pull_request_target|"
                                           r"schedule|release):")
                self.assertNotRegex(text, r"(?m)^\s*repository:\s")

    def test_should_require_named_inputs_and_reject_unsafe_identity_values(self) -> None:
        """Literal values stay data; traversal, case and terminal controls fail locally."""
        support_inputs = _dispatch_inputs(_workflow_text(SUPPORT_WORKFLOW))
        distribution_inputs = _dispatch_inputs(_workflow_text(DISTRIBUTION_WORKFLOW))
        self.assertEqual(set(support_inputs), {"source_commit", "support_identity"})
        self.assertTrue(all(support_inputs.values()), "all support inputs must be required")
        self.assertEqual(set(distribution_inputs), {
            "compiler_commit", "release", "components", "support",
            "previous_core", "previous_vsix", "approved_manifest_sha256",
        })
        self.assertTrue(all(distribution_inputs.values()),
                        "all distribution inputs must be required")
        valid = self._producer([
            "validate", "--release", RELEASE, "--components", COMPONENTS,
            "--support", SUPPORT, "--commit", "a" * 40,
        ])
        self.assertEqual(valid.returncode, 0, valid.stderr)
        self.assertEqual(json.loads(valid.stdout)["release"], RELEASE)
        for name, value in (("release", "v1.2.0\n"), ("release", "1.2.0"),
                            ("components", "../escape"), ("components", "Upper"),
                            ("support", "bad\x1b[31m"), ("commit", "A" * 40),
                            ("commit", "a" * 39)):
            with self.subTest(field=name, value=value):
                arguments = ["validate", "--release", RELEASE, "--components", COMPONENTS,
                             "--support", SUPPORT, "--commit", "a" * 40]
                arguments[arguments.index("--" + name) + 1] = value
                result = self._producer(arguments)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(any(ord(character) < 32 and character not in "\n\t"
                                      or ord(character) == 127 for character in result.stderr))

    def test_should_pin_actions_permissions_timeouts_and_no_shell_interpolation(self) -> None:
        """Only the upload job may write, and no trigger data becomes shell source."""
        for path in (SUPPORT_WORKFLOW, DISTRIBUTION_WORKFLOW):
            with self.subTest(workflow=path.name):
                text = _workflow_text(path)
                self.assertRegex(text, r"(?m)^permissions:\n  contents: read$")
                self.assertIn("persist-credentials: false", text)
                for action in re.findall(r"(?m)^\s*uses:\s*(\S+)(?:\s|$)", text):
                    if action.startswith("./"):
                        continue
                    self.assertRegex(action, r"@[0-9a-f]{40}$", "actions must be SHA-pinned")
                self.assertGreaterEqual(len(re.findall(r"(?m)^\s+timeout-minutes:\s*\d+$", text)), 2)
                self.assertRegex(text, r"(?m)^concurrency:\n  group: ")
                self.assertIn("cancel-in-progress: false", text)
                for body in _run_bodies(text):
                    self.assertNotIn("${{", body, "trigger data must pass through env, not shell source")
        distribution = _workflow_text(DISTRIBUTION_WORKFLOW)
        self.assertIn("contents: write", distribution)
        self.assertEqual(distribution.count("contents: write"), 1,
                         "only one upload/promotion job may write")
        support = _workflow_text(SUPPORT_WORKFLOW)
        self.assertIn("contents: write", support)
        self.assertEqual(support.count("contents: write"), 1,
                         "only one upload job may write")
        for text in (support, distribution):
            self.assertIn("package_release.py validate", text)
            self.assertIn("gh release create", text)
            self.assertIn("--draft", text)

    def test_should_keep_component_and_support_releases_out_of_latest(self) -> None:
        """Drafts upload first; promotion uses an explicit non-latest API flag."""
        for path in (SUPPORT_WORKFLOW, DISTRIBUTION_WORKFLOW):
            text = _workflow_text(path)
            self.assertIn("make_latest=false", text)
            self.assertNotRegex(text, r"gh release (?:create|edit)[^\n]*--latest(?:\s|$)")


class ReleasePayloadSpecTests(ReleaseSpecCase):
    """ST-48 and ST-52: current payload identity and documentation records."""

    def test_should_read_version_records_from_the_final_v1_2_0_release(self) -> None:
        """Compiler, lock and editor records agree; historical records stay historical."""
        cargo = (ROOT / "compiler/Cargo.toml").read_text(encoding="utf-8")
        self.assertRegex(cargo, r'(?m)^version = "1\.2\.0"$')
        lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
        self.assertRegex(lock, r'name = "clc"\nversion = "1\.2\.0"')
        package = json.loads((ROOT / "tools/vscode/package.json").read_bytes())
        package_lock = json.loads((ROOT / "tools/vscode/package-lock.json").read_bytes())
        self.assertEqual(package["version"], "1.2.0")
        self.assertEqual(package_lock["version"], "1.2.0")
        self.assertEqual(package_lock["packages"][""]["version"], "1.2.0")
        self.assertEqual(package["name"], "clorraine")
        self.assertEqual(package["publisher"], "openlorraine")

    def test_should_validate_the_actual_local_candidate_core_and_vsix(self) -> None:
        """Actual 1.2.0 archives and all notices are required; miniatures cannot pass."""
        core_value, vsix_value = (os.environ.get("CLORRAINE_RELEASE_CORE"),
                                  os.environ.get("CLORRAINE_RELEASE_VSIX"))
        if not core_value or not vsix_value:
            self.fail("set CLORRAINE_RELEASE_CORE and CLORRAINE_RELEASE_VSIX to the "
                      "locally built v1.2.0 core archive and VSIX")
        core, vsix = Path(core_value), Path(vsix_value)
        self.assertTrue(core.is_file(), "actual core archive is missing")
        self.assertTrue(vsix.is_file(), "actual VSIX is missing")
        result = self._verify_component(core=core, vsix=vsix)
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(result.stdout)
        self.assertEqual(record["core"]["identity"], COMPONENTS + "-core")
        self.assertEqual(record["vsix"]["identity"], COMPONENTS + "-vsix")
        with __import__("tarfile").open(core, "r:") as archive:
            names = archive.getnames()
        self.assertEqual(sorted(name for name in names if name.startswith("bin/")),
                         ["bin/clc", "bin/clorraine", "bin/clorraine-lsp"])
        self.assertTrue(set(PROJECT_LICENCES) <= set(names))
        with __import__("zipfile").ZipFile(vsix) as archive:
            metadata = json.loads(archive.read("extension/package.json"))
        self.assertEqual((metadata["name"], metadata["publisher"], metadata["version"]),
                         ("clorraine", "openlorraine", "1.2.0"))

    def test_should_validate_the_current_documentation_and_historical_record(self) -> None:
        """Current records name v1.2.0 while v1.1.2 MIT publication facts remain."""
        changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
        self.assertIn("## [1.2.0]", changelog)
        self.assertIn("## [1.1.2] - 2026-11-11", changelog)
        notes = (ROOT / "RELEASE-NOTES.md").read_text(encoding="utf-8")
        self.assertIn("Clorraine v1.2.0", notes)
        self.assertIn("v1.1.2", notes)
        self.assertIn("MIT", notes)
        runbook = (ROOT / "RELEASE.md").read_text(encoding="utf-8")
        self.assertIn("Clorraine v1.2.0", runbook)
        self.assertIn("v1.1.2", runbook)
        self.assertIn("Apache", runbook)
        self.assertIn("support-release.yml", runbook)
        self.assertIn("distribution-release.yml", runbook)
        # Published v1.1.2 identities are immutable history, not current targets.
        self.assertIn("5beec93", runbook)
        self.assertIn("52f9e36461e27d85f06bdd7242f4ab0a4d3fe9ff07aa5eccabde9d9d33d38fbe",
                      runbook)
        self.assertIn("published under the MIT License", runbook)
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertIn("v1.2.0", readme)
        self.assertIn(".clorraine", readme)
        quick = (ROOT / "site/guide/quick-start.md").read_text(encoding="utf-8")
        self.assertIn("v1.2.0", quick)
        self.assertIn("install.sh", quick)
        self.assertNotIn("releases/download/v1.1.2/clorraine-1.1.2", quick)
        notices = (ROOT / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
        self.assertIn("Apache", notices)


class ReleaseReuseSpecTests(ReleaseSpecCase):
    """ST-49 and ST-51: exact immutable identities and independent reuse."""

    def test_should_publish_new_or_reuse_identical_component_bytes_without_mutation(self) -> None:
        """The first run publishes; an identical rerun reuses both exact assets."""
        before = input_snapshot(self.fixture.source.paths)
        fresh = self._verify_component()
        self.assertEqual(fresh.returncode, 0, fresh.stderr)
        fresh_record = json.loads(fresh.stdout)
        self.assertEqual({fresh_record["core"]["decision"], fresh_record["vsix"]["decision"]},
                         {"publish"})
        existing = self.root / "existing-components"
        existing.mkdir()
        (existing / "core.tar").write_bytes(self.fixture.source.raw["core.tar"])
        (existing / "clorraine.vsix").write_bytes(self.fixture.source.raw["clorraine.vsix"])
        reused = self._verify_component(existing=existing)
        self.assertEqual(reused.returncode, 0, reused.stderr)
        record = json.loads(reused.stdout)
        self.assertEqual(record["core"]["decision"], "reuse")
        self.assertEqual(record["vsix"]["decision"], "reuse")
        self.assertEqual(record["core"]["sha256"], digest(self.fixture.source.raw["core.tar"]))
        self.assertEqual(input_snapshot(self.fixture.source.paths), before,
                         "identity verification changed caller inputs")

    def test_should_reject_changed_bytes_or_missing_assets_under_an_existing_identity(self) -> None:
        """A changed core or VSIX is a new identity, never an overwrite of the old one."""
        existing = self.root / "existing-components"
        existing.mkdir()
        core = self.fixture.source.raw["core.tar"]
        vsix = self.fixture.source.raw["clorraine.vsix"]
        (existing / "core.tar").write_bytes(core)
        (existing / "clorraine.vsix").write_bytes(vsix)
        for name, changed in (("core.tar", bytes([core[0] ^ 1]) + core[1:]),
                              ("clorraine.vsix", vsix[:-1])):
            with self.subTest(name=name):
                path = existing / name
                original = path.read_bytes()
                path.write_bytes(changed)
                try:
                    result = self._verify_component(existing=existing)
                    self.assertEqual(result.returncode, 1)
                    self.assertIn("identity", result.stderr.lower())
                    self.assertEqual(path.read_bytes(), changed,
                                     "verification repaired an immutable asset")
                finally:
                    path.write_bytes(original)

    def test_should_reuse_unchanged_core_for_a_support_only_revision(self) -> None:
        """A new support identity advances the distribution without rewriting core bytes."""
        first = self.fixture.output
        first_manifest = json.loads((first / "manifest.json").read_bytes())
        first_core = next(asset for asset in first_manifest["assets"]
                          if asset["kind"] == "core")
        second = self.root / "support-only-r2"
        result = self.fixture.invoke(
            self, self.fixture.arguments(second, release="v1.2.0-r2",
                                         support="other-support"))
        self.assertEqual(result.returncode, 0, result.stderr)
        second_manifest = json.loads((second / "manifest.json").read_bytes())
        self.assertEqual(second_manifest["release"], "v1.2.0-r2")
        self.assertEqual(second_manifest["support"]["identity"], "other-support")
        second_core = next(asset for asset in second_manifest["assets"]
                           if asset["kind"] == "core")
        self.assertEqual(second_core["identity"], COMPONENTS + "-core")
        self.assertEqual(second_core["sha256"], first_core["sha256"])
        self.assertEqual(second_core["bytes"], first_core["bytes"])
        self.assertNotEqual(second_manifest["support"]["lock"],
                            first_manifest["support"]["lock"])
        existing = self.root / "first-components"
        existing.mkdir()
        (existing / "core.tar").write_bytes(self.fixture.source.raw["core.tar"])
        (existing / "clorraine.vsix").write_bytes(self.fixture.source.raw["clorraine.vsix"])
        reused = json.loads(self._verify_component(existing=existing).stdout)
        self.assertEqual(reused["core"]["decision"], "reuse")
        self.assertEqual(reused["vsix"]["decision"], "reuse")


class ReleasePromotionSpecTests(ReleaseSpecCase):
    """ST-50: complete immutable assets, approval binding and ST-56 separation."""

    def _candidate(self) -> tuple[Path, Path, Path]:
        manifest_path = self.fixture.output / "manifest.json"
        manifest_bytes = manifest_path.read_bytes()
        manifest = json.loads(manifest_bytes)
        assets = self._promotion_assets(self.fixture.source.raw, manifest)
        approval = self._approval(manifest_bytes, manifest)
        return manifest_path, assets, approval

    def test_should_allow_pre_publication_promotion_only_with_complete_bound_evidence(self) -> None:
        """A complete candidate is promotion-ready while ST-56 remains pending."""
        manifest, assets, approval = self._candidate()
        record = self._ready(self._promote(manifest, assets, approval))
        self.assertTrue(record["promotion_ready"])
        self.assertFalse(record["final_completion_ready"])
        self.assertEqual(record["blockers"], [])
        self.assertEqual(record["post_publication"], ["ST-56"])

    def test_should_block_missing_private_changed_or_disapproved_payloads(self) -> None:
        """Every absent asset, mismatched hash or approval invalidation blocks promotion."""
        manifest, assets, approval = self._candidate()
        value = json.loads((manifest).read_bytes())

        def expect_blocked(result, label: str) -> None:
            """Require a nonzero exit and a named, printable blocker."""
            self.assertEqual(result.returncode, 1, label + ": " + result.stderr)
            record = json.loads(result.stdout)
            self.assertFalse(record["promotion_ready"], label)
            self.assertTrue(record["blockers"], label)
            for blocker in record["blockers"]:
                self.assertIsInstance(blocker, str)
                self.assertTrue(blocker.strip())

        missing = next(asset for asset in value["assets"] if asset["kind"] == "vsix")
        name = missing["url"].rsplit("/", 1)[-1]
        saved = (assets / name).read_bytes()
        (assets / name).unlink()
        expect_blocked(self._promote(manifest, assets, approval), "missing payload")
        (assets / name).write_bytes(saved)

        changed = next(asset for asset in value["assets"] if asset["kind"] == "core")
        core_name = changed["url"].rsplit("/", 1)[-1]
        core_saved = (assets / core_name).read_bytes()
        (assets / core_name).write_bytes(bytes([core_saved[0] ^ 1]) + core_saved[1:])
        expect_blocked(self._promote(manifest, assets, approval), "changed payload")
        (assets / core_name).write_bytes(core_saved)

        for field, replacement in (("approved", False), ("unresolved_major", 1),
                                   ("manifest_sha256", "f" * 64),
                                   ("release", "v1.2.0-r9")):
            with self.subTest(field=field):
                manifest_bytes = manifest.read_bytes()
                other = self._approval(manifest_bytes, value, **{field: replacement})
                expect_blocked(self._promote(manifest, assets, other), "invalid approval")

    def test_should_accept_passed_st56_only_as_post_publication_completion(self) -> None:
        """ST-56 never blocks promotion, but final completion needs its passed record."""
        manifest, assets, approval = self._candidate()
        manifest_bytes = manifest.read_bytes()
        value = json.loads(manifest_bytes)
        passed = self._approval(manifest_bytes, value, st56="passed")
        record = self._ready(self._promote(manifest, assets, passed))
        self.assertTrue(record["promotion_ready"])
        self.assertTrue(record["final_completion_ready"])
        self.assertEqual(record["post_publication"], [])
        failed = self._approval(manifest_bytes, value, st56="failed")
        not_ready = self._promote(manifest, assets, failed)
        self.assertEqual(not_ready.returncode, 1)
        record = json.loads(not_ready.stdout)
        self.assertTrue(record["promotion_ready"])
        self.assertFalse(record["final_completion_ready"])


if __name__ == "__main__":
    unittest.main()

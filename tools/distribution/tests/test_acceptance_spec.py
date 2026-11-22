"""Linux-only candidate acceptance checks for ST-53, ST-54 and ST-56.

The miniature fixture proves completeness and evidence-shape rejection only.
Actual clean-Ubuntu, desktop, native-FS-UAE and public-install evidence must be
supplied through the explicit environment paths; absent evidence fails, never
skips.
"""

from __future__ import annotations

import copy
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tarfile
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[3]
SETUP_TESTS = ROOT / "tools/setup/tests"
sys.path.insert(0, str(SETUP_TESTS))
sys.path.insert(0, str(ROOT))

from native_manifest_data import encode, make_manifest  # noqa: E402
from tools.distribution.manifest import parse_manifest  # noqa: E402


CANDIDATE_FILES = {
    "install.sh", "install.pyz", "manifest.json", "core.tar", "clorraine.vsix",
    "support-lock.json", "amiga.tar", "amiga-audit.json", "preparation.tar",
    "preparation-audit.json", "cp310.tar", "cp310-audit.json", "cp312.tar",
    "cp312-audit.json",
}
"""Every local input the acceptance evidence must bind; the NDK stays upstream."""

CORE_NAMES = {"clc", "clorraine", "clorraine-lsp"}
NOTICES = {"LICENSE", "LICENSE-EXCEPTION.md", "NOTICE", "TRADEMARKS.md",
           "THIRD_PARTY_NOTICES.md"}
"""The three executables and five first-party notices required in the core."""

EVIDENCE_FIELDS = {"schema", "release", "candidate", "st53", "st54", "st56"}
CANDIDATE_FIELDS = {"manifest_sha256", "core_sha256", "vsix_sha256",
                    "install_sh_sha256", "install_pyz_sha256"}
ST53_FIELDS = {"ubuntu", "python", "checks"}
ST54_FIELDS = {"private_display", "make_verify", "make_vscode_check",
               "make_site_check", "native_fs_uae", "legacy_gates"}
ST56_FIELDS = {"state", "anonymous", "links", "hashes_match", "install"}
ST53_CHECKS = {"cli", "lsp", "vsix", "host", "vamos", "exit7", "profiles",
               "update", "failure", "relocation", "hashes"}
"""Exact acceptance evidence shape; extra or missing fields are errors."""


class AcceptanceError(AssertionError):
    """Candidate or evidence is incomplete, inconsistent or out of scope."""


def _digest(path: Path) -> str:
    """Hash an ordinary candidate file in bounded blocks."""
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def _ordinary(path: Path, label: str) -> Path:
    """Reject a missing, aliased or non-file candidate input."""
    if path.is_symlink() or not path.is_file():
        raise AcceptanceError(label + ": expected an ordinary file")
    return path


def _exact(value: object, fields: set[str], label: str) -> dict:
    """Require exactly the declared object fields."""
    if not isinstance(value, dict) or set(value) != fields:
        raise AcceptanceError(label + ": unknown or missing fields")
    return value


def _true(value: object, label: str) -> None:
    """Require an explicit observed true, never a truthy substitute."""
    if value is not True:
        raise AcceptanceError(label + ": required observation is not true")


def _candidate_hashes(root: Path) -> dict:
    """Measure the five entry/core assets bound into the evidence record."""
    return {
        "manifest_sha256": _digest(root / "manifest.json"),
        "core_sha256": _digest(root / "core.tar"),
        "vsix_sha256": _digest(root / "clorraine.vsix"),
        "install_sh_sha256": _digest(root / "install.sh"),
        "install_pyz_sha256": _digest(root / "install.pyz"),
    }


def validate_candidate(root: Path, *, inspect_payloads: bool = False) -> dict:
    """Require a complete Linux x86_64 candidate and optionally inspect payloads."""
    if root.is_symlink() or not root.is_dir():
        raise AcceptanceError("candidate: expected an ordinary directory")
    names = set()
    for name in CANDIDATE_FILES:
        path = _ordinary(root / name, name)
        names.add(name)
    if set(path.name for path in root.iterdir() if path.is_file()) != CANDIDATE_FILES:
        raise AcceptanceError("candidate: unexpected or duplicate local files")
    manifest_bytes = (root / "manifest.json").read_bytes()
    manifest = parse_manifest(manifest_bytes)
    if (manifest["platform"]["os"] != "linux"
            or manifest["platform"]["architecture"] != "x86_64"
            or manifest["platform"]["cohorts"] != {"22.04": "cp310", "24.04": "cp312"}):
        raise AcceptanceError("candidate: platform is not supported Linux x86_64")
    for asset in manifest["assets"]:
        if asset["kind"] == "ndk":
            continue
        name = asset["url"].rsplit("/", 1)[-1]
        path = _ordinary(root / name, "asset")
        if path.stat().st_size != asset["bytes"] or _digest(path) != asset["sha256"]:
            raise AcceptanceError("candidate: asset bytes do not match the manifest")
    shell = (root / "install.sh").read_text(encoding="utf-8")
    for key in ("URL", "SHA256", "BYTES"):
        match = re.search(r"(?m)^HELPER_" + key + r"='([^']+)'$", shell)
        if match is None:
            raise AcceptanceError("install.sh: missing pinned helper " + key)
        if key == "URL" and not match.group(1).startswith("https://"):
            raise AcceptanceError("install.sh: helper URL is not HTTPS")
    helper = _ordinary(root / "install.pyz", "install.pyz")
    if re.search(r"(?m)^HELPER_SHA256='([0-9a-f]{64})'$", shell).group(1) != _digest(helper):
        raise AcceptanceError("install.sh: helper SHA-256 does not match install.pyz")
    if re.search(r"(?m)^HELPER_BYTES='([0-9]+)'$", shell).group(1) != str(helper.stat().st_size):
        raise AcceptanceError("install.sh: helper byte count does not match install.pyz")
    if inspect_payloads:
        with tarfile.open(root / "core.tar") as archive:
            members = archive.getmembers()
            files = {item.name: item for item in members if item.isfile()}
            if not {"bin/" + name for name in CORE_NAMES} <= set(files):
                raise AcceptanceError("core: missing compiler executables")
            if not NOTICES <= set(files):
                raise AcceptanceError("core: missing first-party notices")
            core = json.loads(archive.extractfile(files["core.json"]).read())
            if core.get("core_version") != manifest["core_version"]:
                raise AcceptanceError("core: version differs from the manifest")
        with zipfile.ZipFile(root / "clorraine.vsix") as archive:
            package = json.loads(archive.read("extension/package.json"))
        if (package.get("name"), package.get("publisher"), package.get("version")) != (
                "clorraine", "openlorraine", manifest["extension_version"]):
            raise AcceptanceError("VSIX: name, publisher or version differs")
    return _candidate_hashes(root)


def validate_evidence(evidence: dict, candidate: dict, *, stage: str) -> None:
    """Validate pre-publication or post-publication evidence without implicit skips."""
    if stage not in ("pre", "post"):
        raise AcceptanceError("stage: expected pre or post")
    root = _exact(evidence, EVIDENCE_FIELDS, "evidence")
    if type(root["schema"]) is not int or root["schema"] != 1:
        raise AcceptanceError("evidence: unsupported schema")
    if root["release"] != "v1.2.0":
        raise AcceptanceError("evidence: expected v1.2.0")
    bound = _exact(root["candidate"], CANDIDATE_FIELDS, "evidence.candidate")
    if bound != candidate:
        raise AcceptanceError("evidence: candidate hashes are stale or mismatched")
    st53 = _exact(root["st53"], ST53_FIELDS, "evidence.st53")
    if st53["ubuntu"] != ["22.04", "24.04"]:
        raise AcceptanceError("ST-53: both Ubuntu 22.04 and 24.04 are required")
    if st53["python"] != {"22.04": "3.10", "24.04": "3.12"}:
        raise AcceptanceError("ST-53: supported CPython cohorts are required")
    checks = _exact(st53["checks"], ST53_CHECKS, "evidence.st53.checks")
    for name, value in checks.items():
        _true(value, "ST-53." + name)
    st54 = _exact(root["st54"], ST54_FIELDS, "evidence.st54")
    for name, value in st54.items():
        _true(value, "ST-54." + name)
    st56 = _exact(root["st56"], ST56_FIELDS, "evidence.st56")
    if stage == "pre":
        if st56["state"] != "pending":
            raise AcceptanceError("ST-56: pre-publication evidence must be pending")
        return
    if st56["state"] != "passed":
        raise AcceptanceError("ST-56: public-install evidence is not passed")
    for name in ("anonymous", "links", "hashes_match", "install"):
        _true(st56[name], "ST-56." + name)


class AcceptanceSpecCase(unittest.TestCase):
    """Build one complete miniature candidate and reject every omission."""

    def setUp(self) -> None:
        """Create a complete fixture so each negative case isolates one defect."""
        self.temporary = tempfile.TemporaryDirectory(prefix="acceptance-spec-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.manifest = make_manifest()
        (self.root / "manifest.json").write_bytes(encode(self.manifest))
        for asset in self.manifest["assets"]:
            if asset["kind"] == "ndk":
                continue
            name = asset["url"].rsplit("/", 1)[-1]
            (self.root / name).write_bytes((name + "\n").encode())
        helper = b"miniature packaged helper\n"
        (self.root / "install.pyz").write_bytes(helper)
        (self.root / "install.sh").write_text(
            "#!/bin/sh\n"
            "HELPER_URL='https://example.invalid/v1.2.0-r1/install.pyz'\n"
            "HELPER_SHA256='" + hashlib.sha256(helper).hexdigest() + "'\n"
            "HELPER_BYTES='" + str(len(helper)) + "'\n", encoding="utf-8")
        self.hashes = _candidate_hashes(self.root)
        self.evidence = {
            "schema": 1, "release": "v1.2.0", "candidate": dict(self.hashes),
            "st53": {"ubuntu": ["22.04", "24.04"],
                     "python": {"22.04": "3.10", "24.04": "3.12"},
                     "checks": {name: True for name in ST53_CHECKS}},
            "st54": {name: True for name in ST54_FIELDS},
            "st56": {"state": "pending", "anonymous": False, "links": False,
                     "hashes_match": False, "install": False},
        }

    def _candidate(self) -> dict:
        """Require the complete fixture before a negative mutation."""
        return validate_candidate(self.root)

    def test_should_accept_a_complete_candidate_and_stage_aware_evidence(self) -> None:
        """Pre-publication accepts pending ST-56; post-publication requires passed."""
        hashes = self._candidate()
        validate_evidence(self.evidence, hashes, stage="pre")
        post = copy.deepcopy(self.evidence)
        post["st56"] = {"state": "passed", "anonymous": True, "links": True,
                        "hashes_match": True, "install": True}
        validate_evidence(post, hashes, stage="post")

    def test_should_reject_missing_candidate_files_and_hash_mismatches(self) -> None:
        """A missing file or changed byte is never an optional acceptance input."""
        self._candidate()
        path = self.root / "amiga.tar"
        original = path.read_bytes()
        path.unlink()
        with self.assertRaisesRegex(AcceptanceError, "ordinary file"):
            validate_candidate(self.root)
        path.write_bytes(original)
        path.write_bytes(b"changed\n")
        with self.assertRaisesRegex(AcceptanceError, "asset bytes"):
            validate_candidate(self.root)
        path.write_bytes(original)
        (self.root / "install.sh").write_text("#!/bin/sh\n", encoding="utf-8")
        with self.assertRaisesRegex(AcceptanceError, "pinned helper"):
            validate_candidate(self.root)

    def test_should_reject_incomplete_prepublication_evidence(self) -> None:
        """Every ST-53/ST-54 observation is mandatory before publication."""
        hashes = self._candidate()
        for label, mutate in (
                ("st53-check", lambda value: value["st53"]["checks"].update(cli=False)),
                ("st53-cohort", lambda value: value["st53"].update(ubuntu=["22.04"])),
                ("st54-display", lambda value: value["st54"].update(private_display=False)),
                ("st56-state", lambda value: value["st56"].update(state="passed")),
                ("candidate-hash", lambda value: value["candidate"].update(core_sha256="f" * 64))):
            with self.subTest(label=label):
                evidence = copy.deepcopy(self.evidence)
                mutate(evidence)
                with self.assertRaises(AcceptanceError):
                    validate_evidence(evidence, hashes, stage="pre")

    def test_should_reject_public_evidence_until_anonymous_install_passes(self) -> None:
        """ST-56 is post-publication evidence and cannot be claimed early."""
        hashes = self._candidate()
        with self.assertRaisesRegex(AcceptanceError, "not passed"):
            validate_evidence(self.evidence, hashes, stage="post")
        failed = copy.deepcopy(self.evidence)
        failed["st56"]["state"] = "failed"
        with self.assertRaises(AcceptanceError):
            validate_evidence(failed, hashes, stage="post")
        passed = copy.deepcopy(self.evidence)
        passed["st56"] = {"state": "passed", "anonymous": True, "links": True,
                          "hashes_match": True, "install": True}
        validate_evidence(passed, hashes, stage="post")

    def test_should_reject_wsl_or_windows_scope_evidence(self) -> None:
        """AR-49 makes v1.2.0 Linux-only; extra platform scope is not accepted."""
        hashes = self._candidate()
        for key in ("wsl", "windows"):
            with self.subTest(key=key):
                evidence = copy.deepcopy(self.evidence)
                evidence[key] = True
                with self.assertRaisesRegex(AcceptanceError, "unknown or missing fields"):
                    validate_evidence(evidence, hashes, stage="pre")

    def test_should_keep_exact_manual_acceptance_steps_in_the_runbook(self) -> None:
        """The runbook must name the real candidate, stages and evidence command."""
        runbook = (ROOT / "RELEASE.md").read_text(encoding="utf-8")
        for text in ("CLORRAINE_ACCEPTANCE_CANDIDATE", "CLORRAINE_ACCEPTANCE_EVIDENCE",
                     "ST-53", "ST-54", "ST-56", "22.04", "24.04",
                     "--release v1.2.0", "private", "anonymous", "relocation",
                     "exit 7"):
            with self.subTest(text=text):
                self.assertIn(text, runbook)


class ActualCandidateAcceptance(unittest.TestCase):
    """Mandatory real candidate and evidence; absent paths are failures."""

    def test_should_validate_actual_candidate_and_prepublication_evidence(self) -> None:
        """The caller supplies one complete directory and one dated evidence record."""
        candidate_value = os.environ.get("CLORRAINE_ACCEPTANCE_CANDIDATE")
        evidence_value = os.environ.get("CLORRAINE_ACCEPTANCE_EVIDENCE")
        stage = os.environ.get("CLORRAINE_ACCEPTANCE_STAGE", "pre")
        if not candidate_value or not evidence_value:
            self.fail("set CLORRAINE_ACCEPTANCE_CANDIDATE and "
                      "CLORRAINE_ACCEPTANCE_EVIDENCE to the real acceptance inputs")
        candidate = validate_candidate(Path(candidate_value), inspect_payloads=True)
        try:
            evidence = json.loads(Path(evidence_value).read_bytes())
        except (OSError, ValueError) as error:
            self.fail("acceptance evidence is unreadable: " + str(error))
        validate_evidence(evidence, candidate, stage=stage)


if __name__ == "__main__":
    unittest.main()

"""Structural manifest specifications using only the documented public parser."""

from __future__ import annotations

import copy
import unittest

from native_manifest_data import encode, make_manifest
from native_manifest_producer_fixture import parser_boundary


class ManifestStructureTests(unittest.TestCase):
    """Judge the pure parser using complete independently constructed records."""

    def setUp(self) -> None:
        """Require a valid positive control before accepting any rejection result."""
        self.parse, self.error = parser_boundary(self)
        self.valid = make_manifest()
        self.assertEqual(self.parse(encode(self.valid)), self.valid)

    def _reject(self, value: dict | bytes) -> None:
        """Require the documented error rather than arbitrary parser exceptions."""
        with self.assertRaises(self.error):
            self.parse(value if isinstance(value, bytes) else encode(value))

    def test_should_return_valid_metadata_unchanged_without_array_position_assumptions(self) -> None:
        """Unique kinds select core and VSIX even after reversing asset order."""
        self.valid["assets"].reverse()
        self.assertEqual(self.parse(encode(self.valid)), self.valid)

    def test_should_reject_duplicate_keys_at_every_record_depth(self) -> None:
        """No JSON object may silently choose the first or last conflicting key."""
        data = encode(self.valid)
        for token in (b'"schema":1', b'"os":"linux"', b'"identity":"fixture-core-core"',
                      b'"cp310":"fixture-support-cp310"', b'"22.04":["ca-certificates"'):
            with self.subTest(token=token):
                self.assertIn(token, data)
                if token.endswith(b'["ca-certificates"'):
                    changed = data.replace(b'"prerequisites":{', b'"prerequisites":{"22.04":[],', 1)
                else:
                    changed = data.replace(token, token + b"," + token, 1)
                self._reject(changed)

    def test_should_reject_missing_and_unknown_fields_in_exact_records(self) -> None:
        """Every fixed-schema object rejects additions and omissions."""
        for location in ((), ("platform",), ("support",), ("assets", 0)):
            for operation in ("missing", "unknown"):
                with self.subTest(location=location, operation=operation):
                    value = copy.deepcopy(self.valid)
                    record = value
                    for part in location:
                        record = record[part]
                    if operation == "missing":
                        del record[next(iter(record))]
                    else:
                        record["unexpected"] = 1
                    self._reject(value)

    def test_should_reject_wrong_types_and_boolean_integer_fields(self) -> None:
        """Schema integers, lengths and expansion counts cannot be booleans."""
        variants = [
            ("schema", True), ("schema", 2), ("schema", "1"),
            ("assets", {}), ("release", 12), ("core_version", None),
            ("prerequisites", []), ("support", []), ("platform", "linux"),
        ]
        for key, bad in variants:
            with self.subTest(key=key, bad=bad):
                value = copy.deepcopy(self.valid)
                value[key] = bad
                self._reject(value)
        for key in ("bytes", "entries", "expanded_bytes"):
            for bad in (True, -1, 1.5, "1", 2 ** 63):
                with self.subTest(key=key, bad=bad):
                    value = copy.deepcopy(self.valid)
                    value["assets"][0][key] = bad
                    self._reject(value)
        value = copy.deepcopy(self.valid)
        value["assets"][0]["bytes"] = 0
        self._reject(value)

    def test_should_reject_invalid_utf8_nonfinite_values_and_oversized_documents(self) -> None:
        """Malformed input cannot evade bounded, strict JSON decoding."""
        for data in (b"\xff", b"[]", b"null", b'{"schema":NaN}',
                     b'{"schema":Infinity}', b" " * (16 * 1024 * 1024 + 1)):
            with self.subTest(size=len(data), prefix=data[:24]):
                self._reject(data)

    def test_should_reject_invalid_release_version_and_identity_grammars(self) -> None:
        """Tags and identities remain data, never paths or shell expressions."""
        for tag in ("../v1.2.0", "components-fixture-core", "support-fixture-support",
                    "v1.1.2", "v01.2.0", "v1.2.0-r0", "v1.2.0-r01", "v1.2.0/next"):
            with self.subTest(tag=tag):
                value = copy.deepcopy(self.valid)
                value["release"] = tag
                self._reject(value)
        for key in ("core_version", "extension_version"):
            value = copy.deepcopy(self.valid)
            value[key] = "01.2.3"
            self._reject(value)
        for identity in ("", "../asset", "Upper", "a_b", "$(touch sentinel)"):
            value = copy.deepcopy(self.valid)
            value["assets"][0]["identity"] = identity
            self._reject(value)

    def test_should_reject_unsafe_urls_hashes_roots_and_raw_expansion_records(self) -> None:
        """Only safe HTTPS destinations and kind-specific payload roots are valid."""
        urls = ("http://example.invalid/a", "https://user:pass@example.invalid/a",
                "https://example.invalid/a#fragment", "https://example.invalid/a b",
                "https://example.invalid\\evil/a", "https://example.invalid:-1/a",
                "https://example.invalid:65536/a", "https:///no-host", "https://example.invalid/\n")
        for url in urls:
            with self.subTest(url=url):
                value = copy.deepcopy(self.valid)
                value["assets"][0]["url"] = url
                self._reject(value)
        for key, bad in (("sha256", "A" * 64), ("sha256", "f" * 63),
                         ("root", "../outside"), ("kind", "shell-command")):
            value = copy.deepcopy(self.valid)
            value["assets"][0][key] = bad
            self._reject(value)
        for key, bad in (("entries", 1), ("expanded_bytes", 1), ("root", ".")):
            value = copy.deepcopy(self.valid)
            next(item for item in value["assets"] if item["kind"] == "ndk")[key] = bad
            self._reject(value)

    def test_should_reject_duplicate_missing_wrong_kind_and_unreferenced_assets(self) -> None:
        """All twelve assets and all four unique audit references are accounted for."""
        variants = []
        duplicate = copy.deepcopy(self.valid)
        duplicate["assets"][1]["identity"] = duplicate["assets"][0]["identity"]
        variants.append(duplicate)
        missing = copy.deepcopy(self.valid)
        missing["assets"].pop()
        variants.append(missing)
        wrong_kind = copy.deepcopy(self.valid)
        wrong_kind["support"]["amiga"] = wrong_kind["support"]["lock"]
        variants.append(wrong_kind)
        shared_audit = copy.deepcopy(self.valid)
        archives = [item for item in shared_audit["assets"] if item["audit"] is not None]
        archives[1]["audit"] = archives[0]["audit"]
        variants.append(shared_audit)
        extra = copy.deepcopy(self.valid)
        extra["assets"].append({**extra["assets"][0], "identity": "unreferenced"})
        variants.append(extra)
        for index, value in enumerate(variants):
            with self.subTest(variant=index):
                self._reject(value)

    def test_should_reject_incompatible_cohorts_profiles_and_package_names(self) -> None:
        """Both supported cohorts and profiles are explicit and package names are inert."""
        for key, bad in (("os", "windows"), ("architecture", "aarch64"),
                         ("cohorts", {"22.04": "cp311", "24.04": "cp312"})):
            value = copy.deepcopy(self.valid)
            value["platform"][key] = bad
            self._reject(value)
        for profiles in ([], ["hosted-amiga"], ["hosted-amiga", "hosted-amiga"]):
            value = copy.deepcopy(self.valid)
            value["support"]["profiles"] = profiles
            self._reject(value)
        for bad in ("gcc;touch marker", "libc6:amd64", "../gcc", "python_venv", ""):
            value = copy.deepcopy(self.valid)
            value["prerequisites"]["22.04"].append(bad)
            self._reject(value)
        value = copy.deepcopy(self.valid)
        value["support"]["wheels"]["cp310"] = value["support"]["wheels"]["cp312"]
        self._reject(value)

"""Strict, side-effect-free distribution metadata shared by packaging and installation.

``parse_manifest(data)`` validates exact UTF-8 bytes without fetching assets or
changing the decoded records. Asset contents require separate authenticated
length, hash and payload checks before they can be used.
"""

from __future__ import annotations

from collections import Counter
import json
import re
from urllib.parse import urlsplit


MAX_MANIFEST_BYTES = 16 * 1024 * 1024
"""Maximum encoded manifest size accepted before JSON decoding."""
BASE_PACKAGES = frozenset({
    "gcc", "libc6-dev", "python3", "python3-venv", "curl", "ca-certificates",
})
"""System prerequisites present in every supported distribution's package set."""
_MAX_INTEGER = 2 ** 63 - 1
"""Largest size/count representable in the distribution format."""
_ID = re.compile(r"[a-z0-9][a-z0-9-]*")
"""Component identities are tokens, never filesystem paths."""
_HASH = re.compile(r"[0-9a-f]{64}")
"""Exact lowercase SHA-256 spelling."""
_PACKAGE = re.compile(r"[a-z0-9][a-z0-9+.-]+")
"""Unqualified Debian package-name grammar, including names such as g++."""
_VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
"""Three-part versions with no ambiguous leading zeroes."""
_RELEASE = re.compile(r"v(?P<version>" + _VERSION.pattern + r")(?:-r[1-9][0-9]*)?")
"""Installable distribution tags, excluding component and support releases."""
_COHORTS = {"22.04": "cp310", "24.04": "cp312"}
"""The only supported Ubuntu/interpreter pairs."""
_ROOTS = {
    "core": ".", "vsix": "extension", "amiga": ".", "preparation": ".",
    "wheels-cp310": ".", "wheels-cp312": ".", "support-lock": None,
    "support-audit": None, "ndk": None,
}
"""Expected archive roots; raw metadata and NDK inputs are not native tar extractions."""
_AUDITED = frozenset({"amiga", "preparation", "wheels-cp310", "wheels-cp312"})
"""Support archives requiring four distinct complete audit records."""
_COUNTS = {kind: (4 if kind == "support-audit" else 1) for kind in _ROOTS}
"""Exact coverage of all twelve component inputs."""


class ManifestError(ValueError):
    """Malformed, incompatible or internally inconsistent distribution metadata."""


def _fail(label: str, detail: str) -> None:
    """Raise a printable field-level error without echoing untrusted values."""
    raise ManifestError(f"{label}: {detail}")


def _object(value: object, fields: set[str], label: str) -> dict:
    """Require an object with exactly the declared field set."""
    if not isinstance(value, dict) or set(value) != fields:
        _fail(label, "unknown or missing fields; expected an object with the exact schema")
    return value


def _text(value: object, pattern: re.Pattern, label: str) -> str:
    """Validate a token before it becomes a lookup key, package name or identity."""
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        _fail(label, "invalid token")
    return value


def _integer(value: object, label: str, minimum: int = 0) -> None:
    """Keep booleans, negative sizes and overflowing declarations out of byte limits."""
    if type(value) is not int or not minimum <= value <= _MAX_INTEGER:
        _fail(label, "expected a bounded integer")


def validate_release_tag(value: str) -> str:
    """Return an installable tag or raise :class:`ManifestError`.

    For example, ``validate_release_tag("v1.2.0-r1")`` returns that same string.
    Numeric comparison does not allocate integers from publisher-controlled
    version components.
    """
    match = _RELEASE.fullmatch(value) if isinstance(value, str) else None
    if match is None:
        _fail("release", "expected an installable distribution tag")
    # For decimal strings without leading zeroes, length then lexical order is
    # numeric order. There is no need to convert publisher-controlled bigints.
    for part, minimum in zip(match["version"].split("."), ("1", "2", "0")):
        actual, floor = (len(part), part), (len(minimum), minimum)
        if actual > floor:
            return value
        if actual < floor:
            _fail("release", "distribution versions start at v1.2.0")
    return value


def validate_https_url(value: str, label: str = "url") -> str:
    """Return a literal safe HTTPS destination, or raise :class:`ManifestError`.

    ``label`` is a caller-supplied diagnostic field name, not remote content.
    Call this for both initial URLs and each redirect before requesting it.
    For example, ``validate_https_url("https://example.org/asset")`` succeeds.
    """
    if (not isinstance(value, str) or not value or "#" in value or "\\" in value
            or any(character.isspace() or not character.isprintable() for character in value)):
        _fail(label, "expected a safe absolute HTTPS URL")
    try:
        parsed = urlsplit(value)
        port = parsed.port
        if (parsed.scheme != "https" or not parsed.hostname
                or parsed.username is not None or parsed.password is not None
                or parsed.netloc.endswith(":") or port is not None and not 1 <= port <= 65535):
            _fail(label, "expected a safe absolute HTTPS URL")
    except ValueError:
        _fail(label, "invalid HTTPS host or port")
    return value


def _asset(value: object, index: int) -> dict:
    """Validate one input declaration without opening the referenced URL."""
    label = f"asset[{index}]"
    asset = _object(value, {
        "identity", "kind", "url", "bytes", "sha256", "root",
        "entries", "expanded_bytes", "audit",
    }, label)
    _text(asset["identity"], _ID, label + ".identity")
    _text(asset["sha256"], _HASH, label + ".sha256")
    kind = asset["kind"]
    if not isinstance(kind, str) or kind not in _ROOTS:
        _fail(label, "unknown asset kind")
    validate_https_url(asset["url"], label + ".url")
    _integer(asset["bytes"], label + ".bytes", minimum=1)
    _integer(asset["entries"], label + ".entries")
    _integer(asset["expanded_bytes"], label + ".expanded_bytes")
    if asset["root"] != _ROOTS[kind]:
        _fail(label, "wrong payload root for asset kind")
    if _ROOTS[kind] is None and (asset["entries"] or asset["expanded_bytes"]):
        _fail(label, "raw inputs cannot declare an extraction inventory")
    if kind in _AUDITED:
        _text(asset["audit"], _ID, label + ".audit")
    elif asset["audit"] is not None:
        _fail(label, "this asset kind cannot reference a support audit")
    return asset


def _reference(value: object, kind: str, assets: dict, label: str) -> None:
    """Require an existing identity of the correct kind, not merely any valid token."""
    identity = _text(value, _ID, label)
    if assets.get(identity, {}).get("kind") != kind:
        _fail(label, "missing or wrong-kind asset reference")


def _support(value: object, assets: dict) -> None:
    """Check the complete support pairing and both ABI/profile references."""
    support = _object(value, {
        "identity", "lock", "amiga", "preparation", "wheels", "ndk",
        "profiles", "prepared_inventory_sha256",
    }, "support")
    _text(support["identity"], _ID, "support.identity")
    _text(support["prepared_inventory_sha256"], _HASH, "support.prepared_inventory_sha256")
    for field, kind in (("lock", "support-lock"), ("amiga", "amiga"),
                        ("preparation", "preparation"), ("ndk", "ndk")):
        _reference(support[field], kind, assets, "support." + field)
    wheels = _object(support["wheels"], {"cp310", "cp312"}, "support.wheels")
    for abi in ("cp310", "cp312"):
        _reference(wheels[abi], "wheels-" + abi, assets, "support.wheels." + abi)
    if support["profiles"] != ["hosted-amiga", "hosted-amiga-13"]:
        _fail("support.profiles", "both supported profiles are required exactly once")


def _prerequisites(value: object) -> None:
    """Validate inert package guidance; exact runtime closure is bound to the lock later."""
    cohorts = _object(value, set(_COHORTS), "prerequisites")
    for ubuntu, packages in cohorts.items():
        label = "prerequisites." + ubuntu
        if not isinstance(packages, list) or not packages:
            _fail(label, "expected a nonempty package set")
        for package in packages:
            _text(package, _PACKAGE, label)
        if len(set(packages)) != len(packages) or not BASE_PACKAGES <= set(packages):
            _fail(label, "duplicate packages or missing base prerequisites")


def _validate(value: object) -> dict:
    """Validate all structural and identity relationships without changing record order."""
    manifest = _object(value, {
        "schema", "release", "core_version", "extension_version", "platform",
        "assets", "support", "prerequisites",
    }, "manifest")
    if type(manifest["schema"]) is not int or manifest["schema"] != 1:
        _fail("schema", "unsupported distribution schema")
    validate_release_tag(manifest["release"])
    for field in ("core_version", "extension_version"):
        _text(manifest[field], _VERSION, field)
    platform = _object(manifest["platform"], {"os", "architecture", "cohorts"}, "platform")
    if (platform["os"] != "linux" or platform["architecture"] != "x86_64"
            or platform["cohorts"] != _COHORTS):
        _fail("platform", "supported Linux/Ubuntu/CPython cohorts are required")
    raw_assets = manifest["assets"]
    if not isinstance(raw_assets, list) or len(raw_assets) != 12:
        _fail("assets", "exactly twelve input records are required")
    records = [_asset(asset, index) for index, asset in enumerate(raw_assets)]
    if dict(Counter(asset["kind"] for asset in records)) != _COUNTS:
        _fail("assets", "missing, duplicate or unexpected component kinds")
    assets = {asset["identity"]: asset for asset in records}
    if len(assets) != len(records):
        _fail("assets", "duplicate component identities")
    audits = set()
    for asset in records:
        if asset["kind"] in _AUDITED:
            identity = asset["audit"]
            _reference(identity, "support-audit", assets, "asset.audit")
            if identity in audits:
                _fail("asset.audit", "support archives cannot share an audit identity")
            audits.add(identity)
    if audits != {asset["identity"] for asset in records if asset["kind"] == "support-audit"}:
        _fail("assets", "unreferenced support audit")
    _support(manifest["support"], assets)
    _prerequisites(manifest["prerequisites"])
    return manifest


def _unique(pairs: list[tuple[str, object]]) -> dict:
    """Reject duplicate JSON keys at every nesting depth before choosing any field."""
    result = {}
    for name, value in pairs:
        if name in result:
            _fail("manifest", "duplicate JSON key")
        result[name] = value
    return result


def _number(_value: str) -> None:
    """No field in this format accepts floating-point or nonfinite JSON numbers."""
    _fail("manifest", "numeric fields must be bounded integers")


def _json_integer(value: str) -> int:
    """Bound integer conversion itself, including on Python versions without a digit cap."""
    if len(value.lstrip("-")) > 19:
        _fail("manifest", "integer exceeds the metadata bound")
    return int(value)


def parse_manifest(data: bytes) -> dict:
    """Return strict decoded metadata, or raise :class:`ManifestError`.

    This public boundary performs no I/O and never executes manifest values.
    For example, a caller may validate downloaded bytes with
    ``manifest = parse_manifest(response_bytes)`` before selecting any assets.
    Payload hashes and embedded metadata must still be checked before use.
    """
    if not isinstance(data, bytes) or len(data) > MAX_MANIFEST_BYTES:
        _fail("manifest", "expected UTF-8 bytes within the 16 MiB limit")
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=_unique,
                           parse_constant=_number, parse_float=_number,
                           parse_int=_json_integer)
        return _validate(value)
    except ManifestError:
        raise
    except (UnicodeError, ValueError, TypeError, RecursionError, OverflowError) as error:
        raise ManifestError("manifest: invalid JSON, UTF-8 or field types") from error

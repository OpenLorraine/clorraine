"""Bounded HTTPS input and safe private archive extraction for the native installer."""

from __future__ import annotations

from contextlib import contextmanager
from datetime import datetime
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import signal
import ssl
import stat
import struct
import tarfile
import time
from urllib.error import HTTPError, URLError
from urllib.parse import parse_qsl, urljoin, urlsplit
from urllib.request import HTTPRedirectHandler, HTTPSHandler, Request, build_opener
import zipfile

from tools.distribution.manifest import (
    MAX_MANIFEST_BYTES, ManifestError, parse_manifest, validate_https_url, validate_release_tag,
)


_API = "https://api.github.com/repos/OpenLorraine/clorraine"
"""Fixed public lookup authority; no manifest controls an API endpoint."""
_DOWNLOAD = "https://github.com/OpenLorraine/clorraine/releases/download/"
"""Distribution entry namespace; payload URLs remain separately authenticated."""
_MAX_ASSET = 512 * 1024 * 1024
"""Bound above the measured 364 MB largest support archive, without unbounded downloads."""
_MAX_EXPANDED = 1024 ** 3
"""Expanded-byte ceiling above the measured support payload and both ABI wheel sets."""
_MAX_ENTRIES = 16384
"""Member-count ceiling above the measured 704-member largest delivered archive."""
_CHUNK = 64 * 1024
"""Fixed streaming buffer; no archive declaration controls a memory allocation."""
_SECONDS = 300
"""Finite overall budget for one download or archive validation pass."""
_ZIP_METADATA = 1024 * 1024
"""Bound central-directory storage before zipfile builds its in-memory member list."""


def _unique(pairs: list[tuple[str, object]]) -> dict:
    """Reject duplicate JSON object keys before any metadata consumer selects a value."""
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("metadata: duplicate JSON key")
        result[key] = value
    return result


def _integer(value: str) -> int:
    """Bound conversion itself on Python versions without a built-in decimal digit cap."""
    if len(value.lstrip("-")) > 19:
        raise ValueError("metadata: integer exceeds the safe bound")
    return int(value)


def _float(value: str) -> float:
    """Permit finite unused API fields but never JSON nonfinite extensions."""
    number = float(value)
    if not math.isfinite(number):
        raise ValueError("metadata: nonfinite number")
    return number


def _constant(_value: str) -> None:
    """Reject NaN and Infinity tokens accepted by Python's default JSON decoder."""
    raise ValueError("metadata: nonfinite number")


def decode_json(data: bytes, label: str) -> object:
    """Decode bounded UTF-8 JSON without duplicates; return an object or API list."""
    if len(data) > MAX_MANIFEST_BYTES:
        raise ValueError(f"{label}: metadata exceeds 16 MiB")
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=_unique,
                          parse_int=_integer, parse_float=_float, parse_constant=_constant)
    except (ValueError, UnicodeError, RecursionError, OverflowError) as error:
        raise ValueError(f"{label}: invalid bounded UTF-8 JSON") from error


@contextmanager
def ordinary(path: Path):
    """Open only an ordinary non-aliased file, without blocking on a substituted FIFO."""
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        if not stat.S_ISREG(os.fstat(descriptor).st_mode):
            raise ValueError("input: expected an ordinary file")
        stream = os.fdopen(descriptor, "rb")
    except BaseException:
        os.close(descriptor)
        raise
    with stream:
        yield stream


def read_json(path: Path) -> dict:
    """Read a bounded ordinary JSON object rather than a device, FIFO or alias."""
    with ordinary(path) as source:
        if os.fstat(source.fileno()).st_size > MAX_MANIFEST_BYTES:
            raise ValueError("metadata: file exceeds 16 MiB")
        data = source.read(MAX_MANIFEST_BYTES + 1)
    value = decode_json(data, path.name)
    if not isinstance(value, dict):
        raise ValueError("metadata: expected a JSON object")
    return value


def _expected(asset: dict) -> tuple[int, str]:
    """Check transfer declarations again at the I/O boundary."""
    size, digest = asset["bytes"], asset["sha256"]
    if (type(size) is not int or not 0 < size <= _MAX_ASSET
            or not isinstance(digest, str) or re.fullmatch("[0-9a-f]{64}", digest) is None):
        raise ValueError("asset: invalid or excessive declared byte length/hash")
    return size, digest


def _verify(source, asset: dict) -> None:
    """Bind actual open-file bytes to an asset without allocating an archive-sized buffer."""
    size, expected = _expected(asset)
    if os.fstat(source.fileno()).st_size != size:
        raise ValueError(f"{asset['kind']}: byte length mismatch")
    digest = hashlib.sha256()
    for block in iter(lambda: source.read(_CHUNK), b""):
        digest.update(block)
    if digest.hexdigest() != expected:
        raise ValueError(f"{asset['kind']}: SHA-256 mismatch")
    source.seek(0)


def verify_file(path: Path, asset: dict) -> None:
    """Revalidate an ordinary cached/snapshotted input before using its contents."""
    with ordinary(path) as source:
        _verify(source, asset)


class _NoRedirect(HTTPRedirectHandler):
    """Leave redirects to the explicit loop, which validates before each request."""

    def redirect_request(self, request, response, code, message, headers, new_url):
        """Prevent urllib from contacting a Location before its scheme is checked."""
        return None


@contextmanager
def _transfer_budget(label: str):
    """Interrupt blocking HTTP operations on the installer's Linux main thread.

    Socket timeouts apply to individual reads, not an entire response. A peer
    can otherwise keep one read alive by slowly supplying more bytes. The
    process timer covers open, TLS/header work and body reads together.
    Never replace another caller's active timer; restore its idle handler on
    both success and failure.
    """
    if signal.getitimer(signal.ITIMER_REAL)[0]:
        raise ValueError(f"{label}: another process alarm timer is already active")
    previous = signal.getsignal(signal.SIGALRM)
    if previous is None:
        raise ValueError(f"{label}: cannot safely preserve the process alarm handler")

    def expired(_number, _frame) -> None:
        """Raise through the blocking operation so its ordinary cleanup path runs."""
        raise ValueError(f"{label}: download deadline exceeded")

    signal.signal(signal.SIGALRM, expired)
    try:
        signal.setitimer(signal.ITIMER_REAL, _SECONDS)
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)


def _download(url: str, sink, label: str, limit: int, expected: dict | None = None) -> dict:
    """Apply an overall blocking-call budget as well as transfer byte/hash bounds."""
    with _transfer_budget(label):
        return _transfer(url, sink, label, limit, expected)


def _transfer(url: str, sink, label: str, limit: int, expected: dict | None) -> dict:
    """Stream one authenticated destination through bounded, manually checked redirects."""
    deadline = time.monotonic() + _SECONDS
    opener = build_opener(_NoRedirect(), HTTPSHandler(context=ssl.create_default_context()))
    digest = hashlib.sha256()
    for _redirect in range(6):
        validate_https_url(url)
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ValueError(f"{label}: download deadline exceeded")
        request = Request(url, headers={"User-Agent": "Clorraine-installer/1"})
        try:
            try:
                response = opener.open(request, timeout=min(15, remaining))
            except HTTPError as error:
                response = error
            with response:
                headers = {key.lower(): value for key, value in response.info().items()}
                status = response.getcode()
                if status in (301, 302, 303, 307, 308):
                    location = headers.get("location")
                    if not isinstance(location, str):
                        raise ValueError(f"{label}: redirect has no valid Location")
                    url = urljoin(url, location)
                    validate_https_url(url)
                    continue
                if status != 200 or response.geturl() != url:
                    raise ValueError(f"{label}: unexpected HTTP status or destination")
                length = headers.get("content-length")
                if length is not None:
                    if (not isinstance(length, str) or not length.strip().isascii()
                            or not length.strip().isdigit() or len(length.strip()) > 19):
                        raise ValueError(f"{label}: invalid HTTP byte length")
                    declared = int(length.strip())
                    if declared > limit or expected is not None and declared != expected["bytes"]:
                        raise ValueError(f"{label}: declared byte length mismatch")
                count = 0
                while True:
                    if time.monotonic() >= deadline:
                        raise ValueError(f"{label}: download deadline exceeded")
                    block = response.read(min(_CHUNK, limit - count + 1))
                    if not block:
                        break
                    count += len(block)
                    if count > limit:
                        raise ValueError(f"{label}: response exceeds its declared byte length")
                    sink.write(block)
                    digest.update(block)
                if expected is not None and (count != expected["bytes"]
                                             or digest.hexdigest() != expected["sha256"]):
                    raise ValueError(f"{label}: downloaded byte length/SHA-256 mismatch")
                return headers
        except (OSError, URLError) as error:
            raise ValueError(f"{label}: HTTPS transfer failed; check connectivity and CA certificates") from error
    raise ValueError(f"{label}: too many HTTPS redirects")


def fetch_bytes(url: str, label: str) -> tuple[bytes, dict]:
    """Fetch a metadata document with bounded memory; return exact bytes and lowercase headers."""
    data = io.BytesIO()
    headers = _download(url, data, label, MAX_MANIFEST_BYTES)
    return data.getvalue(), headers


def fetch_asset(asset: dict, destination: Path) -> None:
    """Create one verified private download, removing only this new file on failure."""
    size, _digest = _expected(asset)
    created = False
    try:
        with destination.open("xb") as output:
            created = True
            _download(asset["url"], output, asset["kind"], size, expected=asset)
    except BaseException:
        if created:
            destination.unlink(missing_ok=True)
        raise


def _published(value: object, exact: str | None = None) -> tuple[datetime, str] | None:
    """Recognize published distribution records, excluding draft/support/component tags."""
    if not isinstance(value, dict) or not isinstance(value.get("tag_name"), str):
        raise ValueError("release lookup: invalid release record")
    tag = value["tag_name"]
    try:
        validate_release_tag(tag)
    except ManifestError:
        if exact is not None:
            raise ValueError("release lookup: exact selection is not a distribution")
        return None
    if type(value.get("draft")) is not bool or type(value.get("prerelease")) is not bool:
        raise ValueError("release lookup: invalid publication flags")
    if value["draft"] or value["prerelease"]:
        if exact is not None:
            raise ValueError("release lookup: selected distribution is not published")
        return None
    published = value.get("published_at")
    if not isinstance(published, str) or re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", published) is None:
        raise ValueError("release lookup: invalid publication timestamp")
    if exact is not None and tag != exact:
        raise ValueError("release lookup: returned a different distribution identity")
    return datetime.strptime(published, "%Y-%m-%dT%H:%M:%SZ"), tag


def _next_page(header: str | None) -> str | None:
    """Accept only ordinary same-repository pagination; never silently truncate malformed links."""
    if not header:
        return None
    selected = None
    for part in header.split(","):
        match = re.fullmatch(r'\s*<([^<>]+)>\s*;\s*rel="?(next|prev|first|last)"?\s*', part)
        if match is None:
            raise ValueError("release lookup: malformed pagination header")
        if match[2] != "next":
            continue
        if selected is not None:
            raise ValueError("release lookup: duplicate next page")
        selected = validate_https_url(match[1])
        parsed = urlsplit(selected)
        query = parse_qsl(parsed.query, strict_parsing=True, max_num_fields=4)
        fields = dict(query)
        if (parsed.scheme + "://" + parsed.netloc + parsed.path != _API + "/releases"
                or len(fields) != len(query) or set(fields) != {"per_page", "page"}
                or fields["per_page"] != "100" or not re.fullmatch("[1-9][0-9]{0,2}", fields["page"])):
            raise ValueError("release lookup: next page leaves the fixed repository listing")
    return selected


def fetch_manifest(tag: str | None) -> tuple[dict, bytes]:
    """Pin one published distribution and return its validated exact manifest bytes."""
    if tag is not None:
        validate_release_tag(tag)
        data, _headers = fetch_bytes(_API + "/releases/tags/" + tag, "release lookup")
        release = decode_json(data, "release lookup")
        _published(release, exact=tag)
    else:
        url, seen, best, release = _API + "/releases?per_page=100", set(), None, None
        for _page in range(100):
            if url in seen:
                raise ValueError("release lookup: pagination cycle")
            seen.add(url)
            data, headers = fetch_bytes(url, "release lookup")
            listing = decode_json(data, "release lookup")
            if not isinstance(listing, list):
                raise ValueError("release lookup: expected a release list")
            for candidate in listing:
                identity = _published(candidate)
                if identity is not None and (best is None or identity > best):
                    best, release = identity, candidate
            url = _next_page(headers.get("link"))
            if url is None:
                break
        else:
            raise ValueError("release lookup: incomplete listing exceeds 100 pages")
        if release is None:
            raise ValueError("release lookup: no published installable distribution found")
        tag = release["tag_name"]
    assets = release.get("assets")
    if not isinstance(assets, list) or any(not isinstance(item, dict) for item in assets):
        raise ValueError("release lookup: invalid entry asset list")
    manifests = [item for item in assets if item.get("name") == "manifest.json"]
    url = _DOWNLOAD + tag + "/manifest.json"
    if len(manifests) != 1 or manifests[0].get("browser_download_url") != url:
        raise ValueError("release lookup: missing or mismatched distribution manifest asset")
    data, _headers = fetch_bytes(url, "distribution manifest")
    manifest = parse_manifest(data)
    if manifest["release"] != tag:
        raise ValueError("distribution manifest: selected release identity mismatch")
    return manifest, data


def _path(name: str) -> str:
    """Require a canonical safe payload-relative path, without normalizing an escape away."""
    if (not name or name.startswith("/") or "\\" in name or "!" in name
            or any(not character.isprintable() for character in name)
            or any(part in ("", ".", "..") for part in name.split("/"))):
        raise ValueError("archive: unsafe member path")
    return name


class _TarReader:
    """Bound tar metadata reads and seeks before tarfile can allocate from a hostile header."""

    def __init__(self, source) -> None:
        """Retain one already authenticated descriptor and its finite scan deadline."""
        self.source = source
        self.size = os.fstat(source.fileno()).st_size
        self.deadline = time.monotonic() + _SECONDS

    def read(self, size: int) -> bytes:
        """Never allow read-all or an unchecked PAX/long-name allocation."""
        if size < 0 or size > MAX_MANIFEST_BYTES or time.monotonic() >= self.deadline:
            raise ValueError("archive: excessive metadata read or scan deadline")
        return self.source.read(size)

    def tell(self) -> int:
        """Expose the underlying bounded file position."""
        return self.source.tell()

    def seek(self, offset: int, whence: int = os.SEEK_SET) -> int:
        """Reject offsets outside the actual authenticated file."""
        target = (offset if whence == os.SEEK_SET else self.tell() + offset
                  if whence == os.SEEK_CUR else self.size + offset)
        if not 0 <= target <= self.size:
            raise ValueError("archive: truncated or excessive member offset")
        return self.source.seek(offset, whence)


def _links(entries: dict) -> None:
    """Reject file/link parents, escaping/dangling links and cycles before writing anything."""
    directories = {name for name, entry in entries.items() if entry["type"] == "directory"}
    for name in entries:
        parts = name.split("/")
        for count in range(1, len(parts)):
            parent = "/".join(parts[:count])
            if parent in entries and entries[parent]["type"] != "directory":
                raise ValueError("archive: a file or link is used as a parent")
            directories.add(parent)

    for name, entry in entries.items():
        if entry["type"] == "symlink":
            target = entry["target"]
            if (not target or target.startswith("/") or "\\" in target
                    or any(not character.isprintable() for character in target)):
                raise ValueError("archive: unsafe symbolic link")
        if entry["type"] == "directory" and not any(
                other.startswith(name + "/") for other in entries if other != name):
            raise ValueError("archive: unexpected empty directory")
    directories.add("")
    for name, entry in entries.items():
        if entry["type"] != "symlink":
            continue
        current = name.split("/")[:-1]
        pending = entry["target"].split("/")
        states, followed = set(), 0
        while pending:
            state = (tuple(current), tuple(pending))
            if state in states:
                raise ValueError("archive: cyclic internal link")
            states.add(state)
            part = pending.pop(0)
            if part in ("", "."):
                continue
            if part == "..":
                if not current:
                    raise ValueError("archive: symbolic link escapes staging")
                current.pop()
                continue
            current.append(part)
            target = "/".join(current)
            node = entries.get(target)
            if node is not None and node["type"] == "symlink":
                followed += 1
                if followed > 40:
                    raise ValueError("archive: cyclic or unresolvable internal link")
                # Follow an encountered alias before interpreting later '..'.
                # Lexically normalising first would not match filesystem lookup.
                current.pop()
                pending = node["target"].split("/") + pending
            elif target not in directories and (node is None or pending):
                raise ValueError("archive: dangling or non-directory symbolic link target")


def measure_tar(path: Path) -> tuple[int, int]:
    """Count bounded ordinary tar headers and bytes before assigning a producer inventory.

    This is a measurement, not permission to use the contents. Call
    :func:`extract_tar` for full path/link/type/hash validation afterwards.
    """
    count = expanded = 0
    with ordinary(path) as source:
        if os.fstat(source.fileno()).st_size > _MAX_ASSET:
            raise ValueError("archive: payload exceeds the download ceiling")
        with tarfile.open(fileobj=_TarReader(source), mode="r:") as archive:
            for member in archive:
                count += 1
                if member.isfile():
                    expanded += member.size
                if count > _MAX_ENTRIES or expanded > _MAX_EXPANDED or member.size < 0:
                    raise ValueError("archive: inventory exceeds the safe extraction bounds")
    return count, expanded


def extract_tar(path: Path, destination: Path, asset: dict, records: dict | None = None) -> dict:
    """Validate every member and hash, then extract only into a fresh private destination.

    ``records`` is the authenticated support audit's complete file/link map.
    Return observed file/link hashes, lengths and modes for subsequent embedded
    core/preparation checks. Hardlinks and special files are never required by
    these published tar payloads and are rejected.
    """
    if (type(asset["entries"]) is not int or not 0 < asset["entries"] <= _MAX_ENTRIES
            or type(asset["expanded_bytes"]) is not int
            or not 0 <= asset["expanded_bytes"] <= _MAX_EXPANDED):
        raise ValueError(f"{asset['kind']}: excessive extraction inventory")
    if destination.is_symlink() or destination.exists() and (
            not destination.is_dir() or any(destination.iterdir())):
        raise ValueError("archive: destination must be fresh ordinary private staging")
    with ordinary(path) as source:
        _verify(source, asset)
        with tarfile.open(fileobj=_TarReader(source), mode="r:") as archive:
            entries, members, expanded = {}, [], 0
            for member in archive:
                if len(members) >= min(_MAX_ENTRIES, asset["entries"]):
                    raise ValueError("archive: member count exceeds its inventory")
                name = _path(member.name[:-1] if member.isdir() and member.name.endswith("/")
                             else member.name)
                kind = ("file" if member.isfile() else "directory" if member.isdir()
                        else "symlink" if member.issym() else None)
                if (name in entries or kind is None or member.mode & 0o7000
                        or kind != "symlink" and member.mode & 0o022
                        or kind == "file" and not member.mode & 0o400):
                    raise ValueError("archive: duplicate member, unsafe type or mode")
                entry = {"type": kind, "mode": member.mode}
                if kind == "file":
                    expanded += member.size
                    if member.size < 0 or expanded > min(_MAX_EXPANDED, asset["expanded_bytes"]):
                        raise ValueError("archive: expanded bytes exceed its inventory")
                    digest, count = hashlib.sha256(), 0
                    with archive.extractfile(member) as content:
                        for block in iter(lambda: content.read(_CHUNK), b""):
                            digest.update(block)
                            count += len(block)
                    if count != member.size:
                        raise ValueError("archive: truncated member")
                    entry.update(bytes=count, sha256=digest.hexdigest())
                elif kind == "symlink":
                    entry["target"] = member.linkname
                entries[name] = entry
                members.append((member, name))
            if len(members) != asset["entries"] or expanded != asset["expanded_bytes"]:
                raise ValueError("archive: inventory count/expanded-byte mismatch")
            _links(entries)
            observed = {name: entry for name, entry in entries.items()
                        if entry["type"] != "directory"}
            if records is not None:
                if set(records) != set(observed):
                    raise ValueError("archive: support audit coverage mismatch")
                for name, entry in observed.items():
                    keys = ("type", "sha256") if entry["type"] == "file" else ("type", "target")
                    if not isinstance(records[name], dict) or any(
                            records[name].get(key) != entry[key] for key in keys):
                        raise ValueError("archive: support audit hash/link mismatch")
            destination.mkdir(mode=0o700, parents=True, exist_ok=True)
            for member, name in members:
                target = destination / name
                target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                if member.isdir():
                    target.mkdir(mode=0o700, exist_ok=True)
                elif member.isfile():
                    digest = hashlib.sha256()
                    with archive.extractfile(member) as content, target.open("xb") as output:
                        for block in iter(lambda: content.read(_CHUNK), b""):
                            output.write(block)
                            digest.update(block)
                    if digest.hexdigest() != entries[name]["sha256"]:
                        raise ValueError("archive: content changed while staging")
                    target.chmod(member.mode)
            for member, name in members:
                if member.issym():
                    (destination / name).symlink_to(member.linkname)
            return observed


def _zip_directory(source, size: int, expected: dict | None) -> int:
    """Bound and count central-directory records before zipfile allocates its member list."""
    source.seek(max(0, size - 65557))
    tail = source.read(65557)
    position = tail.rfind(b"PK\x05\x06")
    if position < 0 or len(tail) - position < 22:
        raise ValueError("VSIX: missing ZIP directory")
    record = struct.unpack("<4s4H2LH", tail[position:position + 22])
    _signature, disk, directory_disk, on_disk, count, length, offset, comment = record
    end = size - len(tail) + position
    if (position + 22 + comment != len(tail) or disk or directory_disk
            or on_disk != count or not 0 < count <= _MAX_ENTRIES
            or length > _ZIP_METADATA or offset + length != end):
        raise ValueError("VSIX: unsupported or excessive ZIP directory")
    if expected is not None and count != expected["entries"]:
        raise ValueError("VSIX: member count disagrees with the asset inventory")
    source.seek(offset)
    consumed = observed = 0
    while consumed < length:
        header = source.read(46)
        if len(header) != 46 or header[:4] != b"PK\x01\x02":
            raise ValueError("VSIX: invalid ZIP member directory")
        fields = struct.unpack("<4s6H3L5H2L", header)
        name_length, extra_length, comment_length, member_disk = fields[10:14]
        extent = 46 + name_length + extra_length + comment_length
        observed += 1
        consumed += extent
        if (observed > count or consumed > length or member_disk or fields[3] & 1
                or fields[4] not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED)):
            raise ValueError("VSIX: excessive, encrypted or unsupported ZIP member")
        source.seek(extent - 46, os.SEEK_CUR)
    if consumed != length or observed != count:
        raise ValueError("VSIX: ZIP directory count mismatch")
    source.seek(0)
    return count


def inspect_vsix(path: Path, asset: dict | None = None) -> tuple[dict, int, int]:
    """Validate bounded ZIP paths/types/data and return package metadata/count/expanded bytes.

    This reads the VSIX but never extracts or installs an extension. Ordinary
    group-writable file bits in the existing VSIX are data here, not permission
    instructions; special files, aliases and privileged modes are prohibited.
    An authenticated ``asset`` additionally binds the exact received bytes.
    """
    with ordinary(path) as source:
        size = os.fstat(source.fileno()).st_size
        if size <= 0 or size > _MAX_ASSET:
            raise ValueError("VSIX: invalid bounded archive size")
        if asset is not None:
            _verify(source, asset)
        count = _zip_directory(source, size, asset)
        with zipfile.ZipFile(source) as archive:
            members = archive.infolist()
            if len(members) != count:
                raise ValueError("VSIX: inconsistent directory member count")
            entries, expanded, package = {}, 0, None
            for member in members:
                name = _path(member.filename[:-1] if member.is_dir()
                             and member.filename.endswith("/") else member.filename)
                mode = member.external_attr >> 16
                kind = "directory" if member.is_dir() else "file"
                allowed = (0, stat.S_IFDIR) if member.is_dir() else (0, stat.S_IFREG)
                if (name in entries or stat.S_IFMT(mode) not in allowed or mode & 0o7000
                        or member.flag_bits & 1):
                    raise ValueError("VSIX: duplicate member, special file or unsafe mode")
                entries[name] = {"type": kind}
                if member.is_dir():
                    continue
                expanded += member.file_size
                if expanded > _MAX_EXPANDED:
                    raise ValueError("VSIX: expanded payload exceeds the safe bound")
                if name == "extension/package.json" and member.file_size > MAX_MANIFEST_BYTES:
                    raise ValueError("VSIX: package metadata exceeds 16 MiB")
            _links(entries)
            if asset is not None and expanded != asset["expanded_bytes"]:
                raise ValueError("VSIX: expanded bytes disagree with the asset inventory")
            # Stream every member so inconsistent local headers and CRC failures
            # cannot hide in an otherwise unused extension file.
            for member in members:
                if member.is_dir():
                    continue
                metadata = bytearray() if member.filename == "extension/package.json" else None
                with archive.open(member) as content:
                    for block in iter(lambda: content.read(_CHUNK), b""):
                        if metadata is not None:
                            metadata.extend(block)
                            if len(metadata) > MAX_MANIFEST_BYTES:
                                raise ValueError("VSIX: package metadata exceeds its safe bound")
                if metadata is not None:
                    package = decode_json(bytes(metadata), "VSIX package metadata")
            if not isinstance(package, dict):
                raise ValueError("VSIX: missing extension/package.json")
            return package, count, expanded

"""The live half of the release docs check.

After the site deploys, this module confirms that the latest GitHub Pages
deployment succeeded and carries the current site content, reads every page
in the sitemap as rendered text and applies the text rules to it, probes
every project link (GitHub through the maintainer's authenticated `gh`,
Discord through its invite API, the Marketplace through the versioned package
URL), and reads the published release body. Plain HTTPS requests go only to a
fixed set of hosts, follow redirects one checked hop at a time, and retry only
transient failures; a probe that never answers is reported as unreachable,
never as a pass.
"""

from __future__ import annotations

import http.client
import json
import os
import re
import subprocess
import time
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path
from typing import Callable
from urllib.parse import parse_qs, quote, urljoin, urlsplit

from docs_rules import Finding, Manifest, glob_regex, normalize, tokens

USER_AGENT = "clorraine-release-docs-check/1"
REQUEST_TIMEOUT = 20
RUN_BUDGET = 600
BODY_LIMIT = 5 * 1024 * 1024
RETRIES = 2
MAX_REDIRECTS = 5
MAX_RETRY_AFTER = 60
PLAIN_HOSTS = ("openlorraine.github.io", "discord.com", "marketplace.visualstudio.com")
MARKETPLACE = "marketplace.visualstudio.com"
CDN_SUFFIX = ".vsassets.io"
SITE_PATHS = ("site", "docs/amiga-sdk", "demo/poster.png", ".github/workflows/docs.yml")
# The pages VitePress leaves out of the build (`srcExclude` in the site config).
NOT_PAGES = ("site/README.md", "site/public/**")
OWNER_REPO = "OpenLorraine/clorraine"
GITHUB_PREFIX = f"https://github.com/{OWNER_REPO}"
ASSETS = ("clorraine-{version}-x86_64-linux.tar.gz", "clorraine-{version}-x86_64-windows.zip",
          "install.sh", "install.ps1")
RELEASE_URL = re.compile(r"https://github\.com/OpenLorraine/clorraine/releases/(?:download|tag)/[^\s\"'<>)`]+")
SEGMENT = re.compile(r"^[A-Za-z0-9._-]+$")
TAG = re.compile(r"^v\d+\.\d+\.\d+$")
INVITE = re.compile(r"^[A-Za-z0-9-]+$")

# Tests replace these so retries do not wait and budgets can be exercised.
sleep = time.sleep
clock = time.monotonic


class Unreachable(Exception):
    """A request that did not get an answer worth judging."""


@dataclass
class Response:
    status: int
    headers: dict = field(default_factory=dict)
    body: bytes = b""


@dataclass
class Outcome:
    findings: list[Finding] = field(default_factory=list)
    unreachable: list[Finding] = field(default_factory=list)
    not_deployed: Finding | None = None
    prerelease: bool | None = None


# --- Transport ---------------------------------------------------------------


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):  # urllib hook: never follow
        return None


_OPENER = urllib.request.build_opener(_NoRedirect())


def http_transport(url: str, read_body: bool = True, deadline: float | None = None) -> Response:
    """One HTTPS GET hop: no redirect is followed, no cookie or token is sent,
    and the body is read against a wall-clock limit and a size cap."""
    stop = min(clock() + REQUEST_TIMEOUT, deadline) if deadline else clock() + REQUEST_TIMEOUT
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "*/*"})
    try:
        with _OPENER.open(request, timeout=max(1.0, stop - clock())) as answer:
            chunks, size = [], 0
            while read_body:
                if clock() > stop:
                    raise Unreachable(f"{url}: the answer took longer than {REQUEST_TIMEOUT} s")
                chunk = answer.read(64 * 1024)
                if not chunk:
                    break
                size += len(chunk)
                if size > BODY_LIMIT:
                    raise Unreachable(f"{url}: the answer is larger than {BODY_LIMIT} bytes")
                chunks.append(chunk)
            return Response(answer.status, dict(answer.headers), b"".join(chunks))
    except urllib.error.HTTPError as error:
        return Response(error.code, dict(error.headers or {}), b"")


def gh_runner(args: list[str], timeout: float = REQUEST_TIMEOUT) -> tuple[int, str, str]:
    """Runs `gh` with an argument list and the maintainer's own login; the
    host and repository come from the arguments, never from GH_HOST/GH_REPO."""
    env = {key: value for key, value in os.environ.items() if key not in ("GH_HOST", "GH_REPO")}
    try:
        result = subprocess.run(["gh", *args], capture_output=True, encoding="utf-8", errors="replace",
                                timeout=timeout, env=env)
    except FileNotFoundError:
        raise Unreachable("the GitHub CLI `gh` is not installed") from None
    except subprocess.TimeoutExpired:
        raise Unreachable(f"gh {' '.join(args)} timed out") from None
    return result.returncode, result.stdout, result.stderr


class Client:
    """HTTPS GETs under the network policy, within one run's time budget."""

    def __init__(self, transport: Callable, deadline: float) -> None:
        self.transport = transport
        self.deadline = deadline

    def get(self, url: str, read_body: bool = True) -> Response:
        last = "no answer"
        for attempt in range(RETRIES + 1):
            try:
                answer = self._follow(url, read_body)
            except Unreachable:
                raise
            except ValueError as error:
                raise Unreachable(f"{url}: not a usable URL ({error})") from None
            except (OSError, TimeoutError, urllib.error.URLError, http.client.HTTPException) as error:
                last = str(error) or type(error).__name__
            else:
                if answer.status == 429 or answer.status >= 500:
                    last = f"HTTP {answer.status}"
                    if attempt < RETRIES:
                        sleep(self._retry_after(answer, attempt))
                    continue
                return answer
            if attempt < RETRIES:
                sleep(2 ** (attempt + 1))
        raise Unreachable(f"{url}: {last} after {RETRIES + 1} attempts")

    def _follow(self, url: str, read_body: bool) -> Response:
        current = url
        origin = (urlsplit(url).hostname or "").lower()
        for _ in range(MAX_REDIRECTS + 1):
            if clock() > self.deadline:
                raise Unreachable(f"{url}: the run's time budget is spent")
            parts = urlsplit(current)
            host = (parts.hostname or "").lower()
            plain = parts.username is None and parts.password is None and parts.port is None
            allowed = host in PLAIN_HOSTS or (origin == MARKETPLACE and host.endswith(CDN_SUFFIX))
            if parts.scheme != "https" or not plain or not allowed or parts.netloc.lower() != host:
                raise Unreachable(f"{url}: refused a request to {parts.netloc or current} (not an allowed host)")
            answer = self._hop(current, read_body)
            if answer.status in (301, 302, 303, 307, 308):
                location = answer.headers.get("Location") or answer.headers.get("location")
                if not location:
                    return answer
                current = urljoin(current, location)
                continue
            return answer
        raise Unreachable(f"{url}: more than {MAX_REDIRECTS} redirects")

    def _hop(self, url: str, read_body: bool) -> Response:
        if self.transport is http_transport:
            return http_transport(url, read_body, self.deadline)
        return self.transport(url, read_body)

    @staticmethod
    def _retry_after(answer: Response, attempt: int) -> float:
        value = answer.headers.get("Retry-After") or answer.headers.get("retry-after")
        if value and str(value).isdigit():
            return min(int(value), MAX_RETRY_AFTER)
        return 2 ** (attempt + 1)


# --- Rendered text -----------------------------------------------------------

_INLINE = {"a", "abbr", "b", "code", "em", "i", "kbd", "mark", "small", "span", "strong", "sub", "sup", "time"}
_SKIP = {"script", "style", "template"}


class _PageText(HTMLParser):
    """Body text, the title, the descriptions, and every href of one page."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []
        self.hrefs: list[str] = []
        self.skip = 0

    def handle_starttag(self, tag: str, attrs) -> None:
        values = dict(attrs)
        if tag in _SKIP:
            self.skip += 1
            return
        if tag == "meta":
            name = values.get("name") or values.get("property")
            if name in ("description", "og:description") and values.get("content"):
                self.parts.append("\n" + values["content"] + "\n")
            return
        if tag == "a" and values.get("href"):
            self.hrefs.append(values["href"])
        if tag not in _INLINE:
            self.parts.append("\n")

    def handle_endtag(self, tag: str) -> None:
        if tag in _SKIP:
            self.skip = max(0, self.skip - 1)
            return
        if tag not in _INLINE:
            self.parts.append("\n")

    def handle_data(self, data: str) -> None:
        if not self.skip:
            self.parts.append(data)


def page_text(html: str) -> tuple[str, list[str]]:
    """The normalized rendered text of a page and its hrefs."""
    parser = _PageText()
    parser.feed(html)
    parser.close()
    return normalize("".join(parser.parts)).text, parser.hrefs


# --- The live check ----------------------------------------------------------


class Live:
    def __init__(self, repo: Path, manifest: Manifest, version: str, transport: Callable, gh: Callable) -> None:
        self.repo = repo
        self.manifest = manifest
        self.version = version
        self.minor = ".".join(version.split(".")[:2])
        self.deadline = clock() + RUN_BUDGET
        self.client = Client(transport, self.deadline)
        self.gh = gh
        self.outcome = Outcome()
        self.site_url = manifest["site_url"]
        self._releases: dict[str, dict | None] = {}

    def finding(self, where: str, rule: str, message: str) -> None:
        self.outcome.findings.append(Finding(where, rule, message))

    def unreachable(self, where: str, message: str) -> None:
        self.outcome.unreachable.append(Finding(where, "unreachable", message))

    def run(self) -> Outcome:
        try:
            if not self.deployment_current():
                return self.outcome
        except Unreachable as error:
            self.unreachable("GitHub Pages deployment", str(error))
            return self.outcome
        pages = self.sitemap_pages()
        if pages is None:
            return self.outcome
        links: set[str] = set()
        for path in pages:
            links |= self.check_page(path)
        for link in sorted(links):
            self.probe(link)
        self.check_release_body()
        return self.outcome

    # Freshness -----------------------------------------------------------

    def deployment_current(self) -> bool:
        """The newest Pages deployment succeeded and has the current site content."""
        where = "GitHub Pages deployment"
        deployments = self.gh_json(["api", f"repos/{OWNER_REPO}/deployments?environment=github-pages&per_page=1"])
        if not isinstance(deployments, list) or not deployments or not isinstance(deployments[0], dict):
            self.outcome.not_deployed = Finding(where, "deploy-not-current", "GitHub lists no Pages deployment")
            return False
        deployment = deployments[0]
        sha = str(deployment.get("sha", ""))
        statuses = self.gh_json(["api", f"repos/{OWNER_REPO}/deployments/{int(deployment.get('id', 0))}/statuses?per_page=1"])
        state = statuses[0].get("state") if isinstance(statuses, list) and statuses and isinstance(statuses[0], dict) else None
        if state != "success":
            self.outcome.not_deployed = Finding(where, "deploy-not-current",
                                                f"the newest deployment ({sha[:8]}) is {state or 'without a status'}")
            return False
        if not re.fullmatch(r"[0-9a-f]{40}", sha) or not self._ancestor(sha):
            self.outcome.not_deployed = Finding(where, "deploy-not-current",
                                                f"the deployed commit {sha[:8]} is not in this checkout's history; fetch and retry")
            return False
        changed = subprocess.run(["git", "-C", str(self.repo), "diff", "--quiet", sha, "HEAD", "--", *SITE_PATHS])
        if changed.returncode != 0:
            self.outcome.not_deployed = Finding(where, "deploy-not-current",
                                                f"the site changed after the deployed commit {sha[:8]}; wait for the docs deploy")
            return False
        return True

    def _ancestor(self, sha: str) -> bool:
        result = subprocess.run(["git", "-C", str(self.repo), "merge-base", "--is-ancestor", sha, "HEAD"],
                                capture_output=True)
        return result.returncode == 0

    def sitemap_pages(self) -> list[str] | None:
        url = self.site_url + "sitemap.xml"
        try:
            answer = self.client.get(url)
        except Unreachable as error:
            self.unreachable(url, str(error))
            return None
        if answer.status != 200:
            self.finding(url, "sitemap", f"HTTP {answer.status}")
            return None
        try:
            root = ET.fromstring(answer.body)
        except ET.ParseError as error:
            self.finding(url, "sitemap", f"not valid XML: {error}")
            return None
        ns = {"s": "http://www.sitemaps.org/schemas/sitemap/0.9"}
        locations = []
        for entry in root.findall("s:url", ns):
            loc = (entry.findtext("s:loc", default="", namespaces=ns) or "").strip()
            if not loc.startswith(self.site_url):
                self.finding(url, "sitemap", f"{loc} is outside {self.site_url}")
                continue
            locations.append(loc[len(self.site_url):])
        tracked = self.tracked_pages()
        if set(locations) != tracked:
            self.outcome.not_deployed = Finding(
                url, "deploy-not-current",
                f"the sitemap pages differ from the tracked pages (missing {sorted(tracked - set(locations))}, "
                f"extra {sorted(set(locations) - tracked)})")
            return None
        return sorted(set(locations))

    def tracked_pages(self) -> set[str]:
        excludes = [glob_regex(pattern) for pattern in NOT_PAGES]
        pages = set()
        for name in self.manifest.tracked:
            if not (name.startswith("site/") and name.endswith(".md")) or any(rx.match(name) for rx in excludes):
                continue
            stem = name[len("site/"):-len(".md")]
            pages.add(stem[: -len("index")] if stem == "index" or stem.endswith("/index") else stem)
        return pages

    # Pages ---------------------------------------------------------------

    def sources(self, path: str) -> list[str]:
        base = "site/" + (path + "index" if path == "" or path.endswith("/") else path) + ".md"
        files = [base]
        for entry in self.manifest["theme_files"]:
            if entry["pages"] == "all" or (entry["pages"] == "home" and path == ""):
                files.append(entry["file"])
        return files

    def check_page(self, path: str) -> set[str]:
        url = self.site_url + path
        try:
            answer = self.client.get(f"{url}?release-docs-check={int(time.time())}")
        except Unreachable as error:
            self.unreachable(url, str(error))
            return set()
        where = urlsplit(url).path
        if answer.status != 200:
            self.finding(where, "page", f"HTTP {answer.status}")
            return set()
        text, hrefs = page_text(answer.body.decode("utf-8", "replace"))
        allowed = set()
        sources = self.sources(path)
        for entry in self.manifest["allowlist"]:
            if entry["file"] in sources:
                allowed |= {token.text for token in tokens(normalize(entry["phrase"]).text)}
        for token in tokens(text):
            if not token.is_current(self.version) and token.text not in allowed:
                expected = self.version if token.asset or token.number.count(".") == 2 else self.minor
                self.finding(where, "stale-version", f"{token.text} is not v{expected} on the deployed page")
        lowered = text.lower()
        for entry in self.manifest["banned"]:
            if normalize(entry["pattern"]).text.lower() in lowered:
                self.finding(where, "banned", f"'{entry['pattern']}': {entry['message']}")
        if path in ("", self.manifest["navigation"]["quick_start_url"].strip("/")):
            for platform in self.manifest["platforms"]:
                command = normalize(platform["command"].replace("{version}", self.version)).text
                if command not in text:
                    self.finding(where, "platform", f"the deployed page lacks the {platform['name']} install command")
        links = set()
        for href in hrefs:
            try:
                links.add(urljoin(url, href))
            except ValueError:
                self.finding(where, "link", f"malformed link {href!r}")
        links |= {match.rstrip(".,;:") for match in RELEASE_URL.findall(text)}
        return {link.split("#")[0] for link in links if link.startswith("https://") and "/edit/" not in link}

    # Probes --------------------------------------------------------------

    def probe(self, link: str) -> None:
        try:
            parts = urlsplit(link)
        except ValueError:
            self.finding(link, "link", "malformed link")
            return
        host = (parts.hostname or "").lower()
        try:
            if host == "github.com" and parts.path.lower().rstrip("/").split("/")[1:3] == ["openlorraine", "clorraine"]:
                self.probe_github(link, parts.path)
            elif host in ("discord.gg", "discord.com") and (host == "discord.gg" or parts.path.startswith("/invite/")):
                code = parts.path.rstrip("/").rsplit("/", 1)[-1]
                if not INVITE.match(code):
                    self.finding(link, "link", "not a Discord invite code")
                    return
                self.expect_ok(link, f"https://discord.com/api/v10/invites/{code}")
            elif host == MARKETPLACE and parts.path == "/items":
                if parse_qs(parts.query).get("itemName") != ["openlorraine.clorraine"]:
                    return
                self.expect_ok(link, link)
                self.expect_ok(link, f"https://{MARKETPLACE}/_apis/public/gallery/publishers/openlorraine/"
                                     f"vsextensions/clorraine/{self.version}/vspackage", read_body=False)
        except Unreachable as error:
            self.unreachable(link, str(error))

    def expect_ok(self, link: str, url: str, read_body: bool = True) -> None:
        answer = self.client.get(url, read_body)
        if answer.status != 200:
            self.finding(link, "link", f"{url} answered HTTP {answer.status}")

    def probe_github(self, link: str, path: str) -> None:
        segments = [segment for segment in path.split("/") if segment][2:]
        if any(segment in (".", "..") or not SEGMENT.match(segment) for segment in segments):
            self.finding(link, "link", "the link has a path segment the check does not follow")
            return
        api = f"repos/{OWNER_REPO}"
        if segments[:2] == ["releases", "tag"] and len(segments) == 3:
            self.release(segments[2], link)
        elif segments[:2] == ["releases", "download"] and len(segments) == 4:
            self.release_asset(segments[2], segments[3], link)
        elif segments[:3] == ["releases", "latest", "download"] and len(segments) == 4:
            self.release_asset("latest", segments[3], link)
        elif segments[:1] == ["releases"]:
            self.gh_ok(link, ["api", f"{api}/releases?per_page=1"])
        elif segments[:1] in (["blob"], ["tree"]) and len(segments) >= 2:
            file = "/".join(quote(segment, safe="") for segment in segments[2:])
            self.gh_ok(link, ["api", f"{api}/contents/{file}?ref={quote(segments[1], safe='')}"])
        elif segments[:1] in (["issues"], ["pull"]) and len(segments) == 2 and segments[1].isdigit():
            self.gh_ok(link, ["api", f"{api}/issues/{segments[1]}"])
        else:
            self.gh_ok(link, ["api", api])

    def gh_call(self, args: list[str]) -> tuple[int, str, str]:
        remaining = self.deadline - clock()
        if remaining <= 0:
            raise Unreachable(f"gh {' '.join(args)}: the run's time budget is spent")
        if args[:1] == ["api"]:
            args = ["api", "--hostname", "github.com", *args[1:]]
        if self.gh is gh_runner:
            code, out, err = gh_runner(args, min(REQUEST_TIMEOUT, remaining))
        else:
            code, out, err = self.gh(args)
        if code != 0 and "HTTP 404" not in err:
            raise Unreachable(f"gh {' '.join(args)} failed: {(err or out).strip()[:200]}")
        return code, out, err

    def gh_json(self, args: list[str]):
        code, out, err = self.gh_call(args)
        if code != 0:
            return None
        try:
            return json.loads(out)
        except json.JSONDecodeError:
            raise Unreachable(f"gh {' '.join(args)} did not answer JSON") from None

    def gh_ok(self, link: str, args: list[str]) -> bool:
        code, _, err = self.gh_call(args)
        if code != 0:
            self.finding(link, "link", f"GitHub answered: {err.strip()[:120]}")
            return False
        return True

    def release(self, tag: str, link: str) -> dict | None:
        if tag not in self._releases:
            if tag != "latest" and not TAG.match(tag):
                self._releases[tag] = None
            else:
                endpoint = "releases/latest" if tag == "latest" else f"releases/tags/{tag}"
                value = self.gh_json(["api", f"repos/{OWNER_REPO}/{endpoint}"])
                if value is not None and not isinstance(value, dict):
                    raise Unreachable(f"GitHub answered an unexpected release object for {tag}")
                self._releases[tag] = value
        if self._releases[tag] is None:
            self.finding(link, "link", f"there is no published release {tag}")
        return self._releases[tag]

    def release_asset(self, tag: str, name: str, link: str) -> None:
        release = self.release(tag, link)
        names = {asset.get("name") for asset in release.get("assets", []) if isinstance(asset, dict)} if release else set()
        if release is not None and name not in names:
            self.finding(link, "link", f"release {tag} has no asset {name}")

    # Published release ---------------------------------------------------

    def check_release_body(self) -> None:
        tag = f"v{self.version}"
        where = f"{GITHUB_PREFIX}/releases/tag/{tag}"
        try:
            release = self.release(tag, where)
            if release is None:
                return
            self.outcome.prerelease = bool(release.get("prerelease"))
            body = str(release.get("body") or "")
            if "PENDING-" in body:
                self.finding(where, "release-body", "the published body still contains PENDING-")
            names = [name.replace("{version}", self.version) for name in ASSETS]
            for name in names:
                if name not in body:
                    self.finding(where, "release-body", f"the published body does not name {name}")
            assets = {asset.get("name") for asset in release.get("assets", []) if isinstance(asset, dict)}
            if "SHA256SUMS" not in assets:
                self.finding(where, "release-body", "the release has no SHA256SUMS asset")
                return
            _, sums, _ = self.gh_call(["release", "download", tag, "--repo", f"github.com/{OWNER_REPO}",
                                       "--pattern", "SHA256SUMS", "--output", "-"])
        except Unreachable as error:
            self.unreachable(where, str(error))
            return
        expected = {}
        for line in sums.splitlines():
            parts = line.split()
            if len(parts) == 2:
                expected[parts[1].lstrip("*")] = parts[0].lower()
        for line in body.splitlines():
            if not line.lstrip().startswith("|"):
                continue
            name = next((candidate for candidate in names if candidate in line), None)
            digest = re.search(r"\b[0-9a-fA-F]{64}\b", line)
            if name and digest and expected.get(name) != digest.group().lower():
                self.finding(where, "release-body", f"the hash for {name} in the body differs from SHA256SUMS")


def check(repo: Path, manifest: Manifest, version: str, fetcher: Callable | None = None,
          gh: Callable | None = None) -> Outcome:
    """Runs the live check; `fetcher` and `gh` default to the real network."""
    return Live(repo, manifest, version, fetcher or http_transport, gh or gh_runner).run()

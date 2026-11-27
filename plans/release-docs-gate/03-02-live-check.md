# Live check

> **Document**: 03-02-live-check.md
> **Parent**: [Index](00-index.md)

`--live` runs the static rules first, then the live steps below in this order.
Any static finding is reported, and the live steps still run.

## 1. Deploy freshness (AR-24) → exit 4

Live mode needs a git checkout (it reads commit times); outside one it stops with exit 2.

This step does three things:
- Fetch `<site_url>sitemap.xml`.
- Require that its newest `<lastmod>` equals, as an instant, the committer time of the last commit
  that touches `site`, `docs/amiga-sdk`, `demo/poster.png` or `.github/workflows/docs.yml`. That
  time comes from `git log -1 --format=%cI -- …`.
- Require that its page set equals the tracked page set. Each tracked `site/**/*.md` maps to a page,
  except `README.md` and `public/**`: `index.md` maps to the directory URL, and `x.md` maps to `x`.

If either requirement fails, the result is `deploy-not-current` with exit 4, and no page checks run.

## 2. Pages

- **Fetching:** each sitemap page is fetched with a cache-busting query (`?release-docs-check=<epoch>`).
- **Rendered text** (AR-21) is read with the standard `html.parser`:
  - It includes the body text, `<title>`, `meta[name=description]` and `meta[property=og:description]`.
  - It skips `script`, `style`, `template` and `meta[name=generator]`.
  - Text inside the inline elements `a, abbr, b, code, em, i, kbd, mark, small, span, strong, sub, sup, time` is joined with no separator; every other boundary adds a newline. The result is then normalized (03-01).
- **Page rules:**
  - `banned`.
  - `stale-version` at token level: a token is allowed when an allowlist entry for the page's source file allows that exact token. Entries for a theme file whose `pages` covers the page count too (AR-20, AR-38).
  - `platform`: the home page and the quick start render both platform commands.
- **Links** are collected from every `href` on the page, plus Clorraine release URLs matched in the rendered text, since code blocks have no `href`. Links containing `/edit/` are skipped. All links are deduplicated across pages.

## 3. Probes (AR-16, AR-22)

| Link | Probe | Pass |
|------|-------|------|
| `github.com/OpenLorraine/clorraine` (root, `/issues`, `/actions`, other paths) | `gh api repos/OpenLorraine/clorraine` | 200 |
| `…/releases` | `gh api repos/OpenLorraine/clorraine/releases?per_page=1` | 200 |
| `…/releases/tag/vA.B.C` | `gh api repos/OpenLorraine/clorraine/releases/tags/vA.B.C` | 200 |
| `…/releases/download/vA.B.C/NAME` | same release object | lists asset `NAME` |
| `…/blob|tree/REF/PATH` | `gh api repos/OpenLorraine/clorraine/contents/PATH?ref=REF` | 200 |
| `…/issues/N`, `…/pull/N` | `gh api repos/OpenLorraine/clorraine/issues/N` | 200 |
| `discord.gg/CODE`, `discord.com/invite/CODE` | GET `https://discord.com/api/v10/invites/CODE` | 200 |
| `marketplace.visualstudio.com/items?itemName=openlorraine.clorraine` | GET of the listing page **and** of the versioned gallery package URL `…/_apis/public/gallery/publishers/openlorraine/vsextensions/clorraine/X.Y.Z/vspackage` | both 200 |

How `gh` is run:
- `gh` gets an argument list, never a shell.
- A non-zero exit with `HTTP 404` is a finding.
- Any other failure, including a missing `gh` or `gh` not being logged in, is `unreachable`.

## 4. Published release (AR-13)

For tag `vX.Y.Z`:
- **Body:** fetched through the release object. It must not contain `PENDING-`.
- **Assets:** the body must name all four assets (`clorraine-X.Y.Z-x86_64-linux.tar.gz`, `clorraine-X.Y.Z-x86_64-windows.zip`, `install.sh`, `install.ps1`).
- **Hashes:** every 64-hex hash in the body's asset table must equal the matching line of the release's `SHA256SUMS`, downloaded with `gh release download vX.Y.Z -p SHA256SUMS -O -`.
- **Prerelease flag:** reported in the summary line.

## 5. Review record

The record's `After deploy` items must all be ticked (03-01).

## Network policy (AR-23) → exit 3 when unreachable

- **Allowed requests:** HTTPS only. Plain GETs may go only to `openlorraine.github.io`, `discord.com` and `marketplace.visualstudio.com`, the last with any redirect target on `*.vsassets.io` or `*.gallerycdn.vsassets.io`. The host is checked on every redirect hop, at most 5 hops.
- **Request settings:** User-Agent `clorraine-release-docs-check/1`. No cookies, no auth headers, no tokens.
- **Limits:**
  - 20 s per request, and 10 min for the whole run;
  - bodies capped at 5 MB;
  - the Marketplace package body is not read beyond its headers.
- **Retries:** at most 2, with backoff, on a network error, 429 or 5xx, honouring `Retry-After` up to 60 s. Never on a 404.
- **Sitemap:** every location must start with `site_url`.
- **Exit codes:** unreachable after the retries gives exit 3. Exit 3 wins over 1 only when there are no findings; findings plus unreachable probes give exit 1, and the unreachable ones are listed.

## Tests

- **Isolation:** the HTTP fetcher and the `gh` runner are injected.
- **Spec tests:** run against a local fixture of HTML, sitemap and API answers.
- **Real network:** no test touches it (AR-29).

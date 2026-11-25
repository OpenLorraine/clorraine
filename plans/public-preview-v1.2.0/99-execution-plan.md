# Task T-01: Publish the v1.2.0 developer preview

> **Type**: Task (lightweight) · **CodeOps Artifact Schema**: 1
> **Progress**: 0/6 tasks (0%)
> **Reasoning**: high — public release, immutable tag, asset verification, rollback
> **Decision**: Public GitHub **prerelease**, tag `v1.2.0`, title
> `v1.2.0 developer preview`. Docs, website, README, release notes, changelog,
> and `RELEASE.md` stay unchanged until the user's test passes. GitHub keeps
> `v1.1.2` as Latest; no stable v1.2.0 release is created.

## Objective

Publish the accepted local candidate as a public, unadvertised developer
preview so the user can install and test the exact public assets on their own
Linux x86_64 machine. The release is public and downloadable, but it is marked
as a prerelease and is not announced in the repository docs or website.

**Smallest viable design:** Use the existing accepted `build/dist` assets and
one `gh release create --prerelease` operation. Add no build, installer, or
release automation. Prove the uploaded bytes by downloading them again and
comparing `SHA256SUMS`; leave the repository docs and site untouched. A
separate follow-up plan owns the public/stable release and the docs/website
update.

## Confirmed decisions

| Decision | Value |
|---|---|
| Release state | Public prerelease; not Latest; `v1.1.2` remains Latest |
| Tag | `v1.2.0` |
| Title | `v1.2.0 developer preview` |
| Assets | Archive, VSIX, `install.sh`, `SHA256SUMS` |
| Release body | Preview notice, hashes, install/test instructions, limitations |
| Repository docs/site | Frozen: no `docs/`, `site/`, `README.md`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `RELEASE.md`, or `AGENTS.md` changes |
| Test | The user downloads the public assets and tests on their own machine |
| Rollback | Only on the user's explicit instruction; never automatic |
| Follow-up | Separate plan updates docs/site and optionally promotes the release |

## Scope

**In scope:** preflight the candidate and GitHub permissions; write the preview
release body in `build/tmp`; create the public prerelease; verify its state,
asset names/sizes/hashes, and public URLs; give the user exact install/test
instructions; record the preview boundary and rollback options.

**Out of scope:** rebuilding the candidate, changing its bytes, a stable or
Latest release, docs/website/README/release-note/changelog edits, making the
repository private, deleting `v1.1.2`, migrating or rewriting the tag, public
announcements, and any change to the compiler or installer code.

## Tasks

- [ ] T-01.1 **Preflight the candidate and GitHub state.** Confirm the three
  candidate hashes match `06-publication-handoff.md`; confirm `gh` is
  authenticated as an admin of `OpenLorraine/clorraine`; confirm no `v1.2.0`
  release or tag exists; confirm `v1.1.2` is the current Latest release.
  Record the current `main` commit as the release target. Do not rebuild or
  alter `build/dist`.

- [ ] T-01.2 **Write the preview release body** under `build/tmp/` (not a
  repository document). Include: the developer-preview notice; the exact
  archive/installer/VSIX hashes and sizes; the four asset names; install and
  test instructions; the rc24, FS-UAE 3.2.0+, and licensed KS3.1 A1200
  requirements; the A1200-vs-A500 caveat; and a statement that docs/site
  updates are pending. Verify the body contains every asset hash and no
  public-announcement claim.

- [ ] T-01.3 **Create the public prerelease.** Run `gh release create v1.2.0`
  with `--prerelease`, the title above, the notes file, `--target main`, and
  the four assets from `build/dist`. Do not pass `--latest`. Record the
  release URL and database ID. Do not create a stable release.

- [ ] T-01.4 **Verify the uploaded release.** Use `gh release view v1.2.0`
  JSON to require `isPrerelease=true`, `isDraft=false`, `isLatest=false`, the
  tag `v1.2.0`, and exactly the four expected assets. Download the four assets
  from the public URLs; compare their sizes and SHA-256 with the handoff and
  the release body; run `sha256sum -c SHA256SUMS`. Require the public asset
  URLs to return HTTP 200 and the release page to be reachable. Confirm
  `v1.1.2` is still Latest.

- [ ] T-01.5 **Deliver the install/test instructions** to the user in the
  session summary and in the release body. The instructions must use the
  public URLs, verify `install.sh`/`SHA256SUMS` before executing anything, and
  cover the local `--archive` install, setup, hello build/run, FS-UAE run, and
  the VSIX install. Record that the user's test is the next decision point;
  the plan does not mark the release "done" on the user's behalf.

- [ ] T-01.6 **Close the preview task.** Confirm `git status` has no `docs/`,
  `site/`, `README.md`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `RELEASE.md`, or
  `AGENTS.md` changes; confirm no stable v1.2.0 release exists; record the
  rollback options and the follow-up plan boundary. Commit this plan with the
  `git-commit` skill.

## Verify

`gh release view v1.2.0 --json isPrerelease,isDraft,isLatest,tagName,assets,url`
reports `isPrerelease=true`, `isDraft=false`, `isLatest=false`, tag `v1.2.0`,
and exactly four assets whose downloaded sizes and SHA-256 hashes match
`06-publication-handoff.md`; `sha256sum -c SHA256SUMS` passes; the public asset
URLs return HTTP 200; `v1.1.2` remains Latest; and `git status` shows no
docs/website/release-note changes.

## Install and test instructions (release body)

```sh
mkdir -p "$HOME/clorraine-preview" && cd "$HOME/clorraine-preview"
base=https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0
for asset in clorraine-1.2.0-x86_64-linux.tar.gz clorraine-1.2.0.vsix install.sh SHA256SUMS; do
    curl -fLO "$base/$asset"
done
sha256sum -c SHA256SUMS

# Install the exact downloaded bytes; this does not re-download the archive.
sh install.sh --archive clorraine-1.2.0-x86_64-linux.tar.gz \
    --checksums SHA256SUMS --modify-path
"$HOME/.local/bin/clorraine" --version

# Select the user-supplied rc24 toolchain.
"$HOME/.local/bin/clorraine" setup --toolchain "$HOME/toolchains/m68k-amigaos-gcc-16.2"

cat > hello.clo <<'EOF'
module demo.Hello;
import { println } from std.io;
func main(): i32 {
    let who: str = "Amiga";
    println($"Hello, {who}, from Clorraine!");
    return 0;
}
EOF

"$HOME/.local/bin/clorraine" run hello.clo
"$HOME/.local/bin/clorraine" run -p hosted-amiga --emulator \
    --kickstart "$HOME/amiga/roms/amiga-os-310-a1200.rom" hello.clo

code --install-extension clorraine-1.2.0.vsix
```

Expected guest output: `Hello, Amiga, from Clorraine!`. FS-UAE must be 3.2.0 or
newer; the licensed Kickstart is a 512 KB KS3.1 A1200 image. The A500 profile
runs on the A1200 recipe but is not a Kickstart 1.3 proof.

## Rollback and follow-up

The tag and prerelease are public. If the user's test fails, do not delete or
rewrite anything automatically. Present the choice: delete the prerelease and
tag (public history change), or publish a fixed prerelease under a new tag.
A public/stable release, docs/website updates, and any promotion of this
prerelease belong to a separate approved plan after the user's test.

# Release review template

Every release has a review record, `vX.Y.Z.md` in this folder, copied from
this template. `make release-docs-check VERSION=X.Y.Z` requires the
"Before tag" items to be ticked before the release tag is pushed; with
`LIVE=1` it also requires the "After deploy" items, before the release is
announced or closed out (see [RELEASE.md](../../RELEASE.md)).

The items must stay exactly as written here. Add notes as indented sub-bullets
or plain lines. An agent may prepare the record and run the checks, but ticks
the boxes and fills in `Reviewer` only after the maintainer has confirmed each
item. Use GitHub handles, never e-mail addresses.

Copy from the next line down:

# Release review: vX.Y.Z

Prepared by: <who prepared the record>
Reviewer: @<maintainer's GitHub handle>
Date: YYYY-MM-DD

## Before tag

- [ ] Status page: one row per supported platform; what works and what is not ready; release state and license name this release
- [ ] Project brief: header, Status row, Roadmap and Next match this release
- [ ] Roadmap page: Delivered and Next match this release
- [ ] Known limitations: the site and docs/ say the same and match this release
- [ ] Quick start matches the real installer prompts and --help on every platform
- [ ] Home page, preview bar, sidebar and README show every platform's install command for this release
- [ ] No single-platform wording where more platforms are supported
- [ ] Screenshots and the social card still match the product
- [ ] Extension README and CHANGELOG are current (they ship in the extension; this is the last chance before it is published)
- [ ] Latest-stable and preview wording is correct for this release
- [ ] Every allowlist change was read and its reason is true
- [ ] The local preview build (make site-dev) was walked page by page

## After deploy

- [ ] make release-docs-check VERSION=X.Y.Z LIVE=1 passed
- [ ] The published GitHub release body was read
- [ ] The live home page, quick start, and both install commands were read

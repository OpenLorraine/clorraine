#!/usr/bin/env bash
# Drift check for a Clorraine assessment run.
#
# Usage: drift-check.sh <plan-folder> [<baseline-sha>]
#
# <plan-folder> is the run's folder under plans/ (for example
# plans/assessment-2027-01-10). The baseline SHA is read from the
# "> **Baseline**: <sha>" line of <plan-folder>/findings/WS0-baseline.md
# unless it is given as the second argument.
#
# The check passes when:
#   1. no whitespace error is staged or unstaged (git diff --check);
#   2. nothing outside plans/, tools/assessment/, and the assessment skill
#      differs between the baseline and HEAD, and the working tree has no
#      change there either (committed, staged, unstaged, or untracked);
#   3. under plans/, only the run's folder and plans/00-roadmap.md changed
#      since the baseline, in commits or in the working tree.
#
# It only reads the repository. Exit status 0 means no drift; 1 means
# drift (each violation is printed); 2 means a usage or setup error.
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "usage: $0 <plan-folder> [<baseline-sha>]" >&2
  exit 2
fi

root=$(git rev-parse --show-toplevel)
cd "$root"

plan=${1%/}
if [ ! -d "$plan" ]; then
  echo "drift-check: plan folder '$plan' not found" >&2
  exit 2
fi

if [ "$#" -eq 2 ]; then
  base=$2
else
  record="$plan/findings/WS0-baseline.md"
  if [ ! -f "$record" ]; then
    echo "drift-check: $record not found and no baseline SHA given" >&2
    exit 2
  fi
  base=$(sed -n 's/^> \*\*Baseline\*\*: *\([0-9a-f]\{7,40\}\).*/\1/p' "$record" | head -n 1)
  if [ -z "$base" ]; then
    echo "drift-check: no '> **Baseline**: <sha>' line in $record" >&2
    exit 2
  fi
fi

if ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
  echo "drift-check: baseline '$base' is not a commit" >&2
  exit 2
fi

fail=0
report() {
  echo "DRIFT: $1" >&2
  fail=1
}

# 1. Whitespace errors in unstaged and staged changes.
if ! git diff --check >/dev/null 2>&1; then
  report "git diff --check reports whitespace errors (unstaged)"
  git diff --check >&2 || true
fi
if ! git diff --cached --check >/dev/null 2>&1; then
  report "git diff --cached --check reports whitespace errors (staged)"
  git diff --cached --check >&2 || true
fi

# 2. No change outside the assessment's own paths.
outside=(. ':(exclude)plans' ':(exclude)tools/assessment'
         ':(exclude).opencode/skills/clorraine-assessment')
if ! git diff --quiet "$base" HEAD -- "${outside[@]}"; then
  report "committed changes outside the allowed paths since $base:"
  git diff --name-only "$base" HEAD -- "${outside[@]}" >&2
fi
dirty=$(git status --porcelain -- "${outside[@]}")
if [ -n "$dirty" ]; then
  report "working-tree changes outside the allowed paths:"
  echo "$dirty" >&2
fi

# 3. Under plans/, only the run's folder and the roadmap may change.
inplans=(plans ":(exclude)$plan" ':(exclude)plans/00-roadmap.md')
changed=$(git diff --name-only "$base" HEAD -- "${inplans[@]}")
if [ -n "$changed" ]; then
  report "committed changes under plans/ outside $plan and the roadmap:"
  echo "$changed" >&2
fi
dirty=$(git status --porcelain -- "${inplans[@]}")
if [ -n "$dirty" ]; then
  report "working-tree changes under plans/ outside $plan and the roadmap:"
  echo "$dirty" >&2
fi

if [ "$fail" -ne 0 ]; then
  echo "drift-check: FAIL (baseline $base)" >&2
  exit 1
fi
echo "drift-check: PASS (baseline $base, HEAD $(git rev-parse --short HEAD))"

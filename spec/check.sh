#!/usr/bin/env bash
# Validate the Clorraine specification:
#   - every chapter file starts with a '# ' title
#   - every chapter file declares a '> **Status:**' line
#   - every relative markdown link resolves to an existing file
#
# Usage: ./check.sh
set -euo pipefail
cd "$(dirname "$0")"

errors=0
files=0

for f in *.md tests/compile-fail/*.md; do
    [ -e "${f}" ] || continue
    files=$((files + 1))

    first="$(head -1 "$f")"
    case "${first}" in
        "# "*) ;;
        *)
            echo "ERROR: ${f}: first line must be a '# ' title"
            errors=$((errors + 1))
            ;;
    esac

    case "${f}" in
        tests/compile-fail/*) ;;   # suite notes are not chapters, so no Status line
        *)
            if ! grep -q '^> \*\*Status:\*\*' "$f"; then
                echo "ERROR: ${f}: missing '> **Status:**' declaration"
                errors=$((errors + 1))
            fi
            ;;
    esac

    fdir="$(dirname "${f}")"
    while IFS= read -r target; do
        case "${target}" in
            http://*|https://*|mailto:*|"") continue ;;
        esac
        target="${target%%#*}"
        [ -z "${target}" ] && continue
        if [ ! -e "${fdir}/${target}" ]; then
            echo "ERROR: ${f}: broken relative link '${target}'"
            errors=$((errors + 1))
        fi
    done < <(grep -oE ']\([^)]+\)' "$f" | sed -E 's/^\]\((.*)\)$/\1/')
done

# --- diagnostics cross-validation (decision N7) -------------------------------
# Every diagnostic code used in a chapter must be registered in Ch 14.
registry_codes="$(grep -oE '^\| [EW][0-9]{5} \|' 14-diagnostics.md | grep -oE '[EW][0-9]{5}' | sort -u)"

chapter_codes="$(grep -ohE '\b[EW][0-9]{5}\b' [0-9][0-9]-*.md appendix-*.md | sort -u || true)"
while IFS= read -r code; do
    [ -z "${code}" ] && continue
    if ! grep -qx "${code}" <<< "${registry_codes}"; then
        echo "ERROR: diagnostic ${code} is used in a chapter but not registered"
        errors=$((errors + 1))
    fi
done <<< "${chapter_codes}"

# --- compile-fail suite (Ch 16 section 6) -------------------------------------
# Each test declares its expected code on the first line; every registered
# compile-time code must have a test. Runtime faults are run-test cases, and
# E90000 is the generic staged-feature fallback with no unique trigger.
tests=0
tested_codes=""
for f in tests/compile-fail/*.clor; do
    [ -e "${f}" ] || continue
    tests=$((tests + 1))
    first="$(head -1 "${f}")"
    case "${first}" in
        "// expect-error "*|"// expect-warning "*)
            code="$(grep -oE '[EW][0-9]{5}' <<< "${first}" | head -1)"
            if [ -z "${code}" ] || ! grep -qx "${code}" <<< "${registry_codes}"; then
                echo "ERROR: ${f}: expectation code is missing or unregistered"
                errors=$((errors + 1))
            else
                tested_codes="${tested_codes}${code}"$'\n'
            fi
            ;;
        *)
            echo "ERROR: ${f}: first line must be '// expect-error CODE' or '// expect-warning CODE'"
            errors=$((errors + 1))
            ;;
    esac
done
tested_codes="$(sort -u <<< "${tested_codes}")"

excluded_codes="E11210 E11211 E11212 E11213 E90000"
while IFS= read -r code; do
    [ -z "${code}" ] && continue
    skip=0
    for x in ${excluded_codes}; do
        [ "${code}" = "${x}" ] && skip=1
    done
    [ "${skip}" -eq 1 ] && continue
    if ! grep -qx "${code}" <<< "${tested_codes}"; then
        echo "ERROR: no compile-fail test for registered diagnostic ${code}"
        errors=$((errors + 1))
    fi
done <<< "${registry_codes}"

if [ "${errors}" -gt 0 ]; then
    echo "spec check: ${errors} problem(s) in ${files} files"
    exit 1
fi

echo "spec check: OK (${files} files, ${tests} compile-fail tests)"

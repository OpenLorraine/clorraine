#!/usr/bin/env bash
# Validate the Clorraine specification:
#   - every chapter file starts with a '# ' title
#   - every chapter file declares a '> **Status:**' line
#   - every relative markdown link resolves to an existing file
#   - every diagnostic code used in a chapter is registered (live or retired)
#   - every registered compile-time code has a compile-fail test
#   - run and staged test files declare their expected code on the first line
#
# Usage: ./check.sh
set -euo pipefail
cd "$(dirname "$0")"

errors=0
files=0

# check_links FILE - resolve every relative markdown link in FILE against its
# own directory. Absolute URLs, mailto targets, and pure-fragment links are
# ignored.
check_links() {
    local f="$1" target fdir
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
}

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

    check_links "$f"
done

# The run and staged suite READMEs are notes, not chapters: link-check them
# without counting them among the specification files.
for f in tests/run/README.md tests/staged/README.md; do
    [ -e "${f}" ] || continue
    check_links "$f"
done

# --- diagnostics cross-validation (decision N7) -------------------------------
# Every diagnostic code used in a chapter must be registered in Ch 14. A chapter
# may still mention a retired code because it stays reserved, so the retired list
# counts as known too. The live registry drives the coverage rule below.
registry_codes="$(grep -oE '^\| [EW][0-9]{5} \|' 14-diagnostics.md | grep -oE '[EW][0-9]{5}' | sort -u)"
retired_codes="$(grep -oE '^\| `[EW][0-9]{5}` retired \|' 14-diagnostics.md | grep -oE '[EW][0-9]{5}' | sort -u)"
known_codes="$(printf '%s\n%s\n' "${registry_codes}" "${retired_codes}" | grep -E '[EW][0-9]{5}' | sort -u)"

chapter_codes="$(for f in [0-9][0-9]-*.md appendix-*.md; do
    [ "${f}" = "14-diagnostics.md" ] && continue
    grep -ohE '\b[EW][0-9]{5}\b' "${f}"
done | sort -u || true)"
while IFS= read -r code; do
    [ -z "${code}" ] && continue
    if ! grep -qx "${code}" <<< "${known_codes}"; then
        echo "ERROR: diagnostic ${code} is used in a chapter but not registered"
        errors=$((errors + 1))
    fi
done <<< "${chapter_codes}"

# --- compile-fail suite (Ch 16 section 6) -------------------------------------
# Each test declares its expected code on the first line; every registered
# compile-time code must have a test. Runtime faults are run-test cases under
# tests/run/; E90000 is the generic staged-feature fallback; E90003, E90004 and
# W10005 are staged under tests/staged/ because they cannot produce their code
# until the feature or harness exists (see tests/staged/README.md).
tests=0
tested_codes=""
for f in tests/compile-fail/*.clo; do
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

# --- run suite (tests/run/) ---------------------------------------------------
# Each run test declares its expected runtime fault on the first line:
# '// expect-fault CODE'. Runtime faults are excluded from the per-code coverage
# rule above because they are not compile errors.
for f in tests/run/*.clo; do
    [ -e "${f}" ] || continue
    first="$(head -1 "${f}")"
    case "${first}" in
        "// expect-fault "*)
            code="$(grep -oE '[EW][0-9]{5}' <<< "${first}" | head -1)"
            if [ -z "${code}" ] || ! grep -qx "${code}" <<< "${registry_codes}"; then
                echo "ERROR: ${f}: expectation code is missing or unregistered"
                errors=$((errors + 1))
            fi
            ;;
        *)
            echo "ERROR: ${f}: first line must be '// expect-fault CODE'"
            errors=$((errors + 1))
            ;;
    esac
done

# --- staged suite (tests/staged/) --------------------------------------------
# Staged tests use the compile-fail expectation prefixes but are excluded from
# the per-code coverage rule until their feature or harness exists.
for f in tests/staged/*.clo; do
    [ -e "${f}" ] || continue
    first="$(head -1 "${f}")"
    case "${first}" in
        "// expect-error "*|"// expect-warning "*)
            code="$(grep -oE '[EW][0-9]{5}' <<< "${first}" | head -1)"
            if [ -z "${code}" ] || ! grep -qx "${code}" <<< "${registry_codes}"; then
                echo "ERROR: ${f}: expectation code is missing or unregistered"
                errors=$((errors + 1))
            fi
            ;;
        *)
            echo "ERROR: ${f}: first line must be '// expect-error CODE' or '// expect-warning CODE'"
            errors=$((errors + 1))
            ;;
    esac
done

# E11210-E11213 are runtime faults (tests/run/); E90000 is the generic staged
# fallback; the rest are staged in tests/staged/ until their feature or harness
# exists.
excluded_codes="E11210 E11211 E11212 E11213 E90000 E90003 E90004 W10005"
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

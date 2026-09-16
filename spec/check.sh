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

for f in *.md; do
    files=$((files + 1))

    first="$(head -1 "$f")"
    case "${first}" in
        "# "*) ;;
        *)
            echo "ERROR: ${f}: first line must be a '# ' title"
            errors=$((errors + 1))
            ;;
    esac

    if ! grep -q '^> \*\*Status:\*\*' "$f"; then
        echo "ERROR: ${f}: missing '> **Status:**' declaration"
        errors=$((errors + 1))
    fi

    while IFS= read -r target; do
        case "${target}" in
            http://*|https://*|mailto:*|"") continue ;;
        esac
        target="${target%%#*}"
        [ -z "${target}" ] && continue
        if [ ! -e "${target}" ]; then
            echo "ERROR: ${f}: broken relative link '${target}'"
            errors=$((errors + 1))
        fi
    done < <(grep -oE ']\([^)]+\)' "$f" | sed -E 's/^\]\((.*)\)$/\1/')
done

if [ "${errors}" -gt 0 ]; then
    echo "spec check: ${errors} problem(s) in ${files} files"
    exit 1
fi

echo "spec check: OK (${files} files)"

#!/usr/bin/env bash
# Amiga SDK registry tooling.
#
# The registry is the long-lived source of truth for the missing Amiga
# capabilities. This script validates it, renders the human-readable coverage
# view, and writes one generated issue body per bundle.
#
# Usage:
#   registry.sh check
#   registry.sh coverage
#   registry.sh issues [out-dir]
#
# Environment overrides:
#   REGISTRY_FILE, BUNDLES_FILE
set -euo pipefail

export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
registry="${REGISTRY_FILE:-${repo_root}/docs/amiga-sdk/registry.tsv}"
bundles="${BUNDLES_FILE:-${repo_root}/docs/amiga-sdk/bundles.tsv}"
command="${1:-check}"

usage() {
    echo "usage: registry.sh {check|coverage|issues [out-dir]}" >&2
    exit 2
}

[ -f "${registry}" ] || { echo "registry: missing ${registry}" >&2; exit 1; }
[ -f "${bundles}" ] || { echo "registry: missing ${bundles}" >&2; exit 1; }

check_registry() {
    awk -F '\t' '
        BEGIN {
            b_expected = "id" FS "title" FS "tier" FS "min_os" FS "min_hw" FS "status" FS "tracking_issue" FS "plan"
            r_expected = "id" FS "bundle" FS "area" FS "capability" FS "priority" FS "tier" FS "min_os" FS "min_hw" FS "status" FS "depends" FS "acceptance" FS "evidence" FS "spec" FS "notes"
            split("missing planned in-progress blocked done deferred out-of-scope", values, " "); for (i in values) allowed_status[values[i]] = 1
            split("must should could wont", values, " "); for (i in values) allowed_priority[values[i]] = 1
            split("core advanced optional", values, " "); for (i in values) allowed_tier[values[i]] = 1
            split("1.3 2.0 2.1 3.0 3.1 any", values, " "); for (i in values) allowed_os[values[i]] = 1
            split("OCS/68000/512K ECS/68000/1M AGA/68020/2M any", values, " "); for (i in values) allowed_hw[values[i]] = 1
            errors = 0
        }
        FNR == 1 {
            if (NR == 1) {
                if ($0 != b_expected) {
                    print "registry: bundles header is wrong" > "/dev/stderr"
                    errors++
                }
            } else {
                if ($0 != r_expected) {
                    print "registry: registry header is wrong" > "/dev/stderr"
                    errors++
                }
            }
            next
        }
        NF == 0 || $1 ~ /^#/ { next }
        NR == FNR {
            if (NF != 8) {
                print "registry: bundle row has " NF " fields: " $0 > "/dev/stderr"
                errors++
                next
            }
            if ($1 in bundle_seen) {
                print "registry: duplicate bundle id: " $1 > "/dev/stderr"
                errors++
            }
            bundle_seen[$1] = 1
            bundle_order[++bundle_count] = $1
            if (!($3 in allowed_tier) || !($4 in allowed_os) || !($5 in allowed_hw)) {
                print "registry: invalid bundle metadata: " $1 > "/dev/stderr"
                errors++
            }
            if (!($6 in allowed_status)) {
                print "registry: invalid bundle status: " $1 " -> " $6 > "/dev/stderr"
                errors++
            }
            next
        }
        {
            if (NF != 14) {
                print "registry: capability row has " NF " fields: " $0 > "/dev/stderr"
                errors++
                next
            }
            if ($1 !~ /^SDK-[A-Z]+-[0-9][0-9][0-9]$/) {
                print "registry: malformed id: " $1 > "/dev/stderr"
                errors++
            }
            if ($1 in id_seen) {
                print "registry: duplicate id: " $1 > "/dev/stderr"
                errors++
            }
            id_seen[$1] = 1
            capability_count++
            if (!($2 in bundle_seen)) {
                print "registry: unknown bundle: " $1 " -> " $2 > "/dev/stderr"
                errors++
            }
            if (!($5 in allowed_priority)) {
                print "registry: invalid priority: " $1 " -> " $5 > "/dev/stderr"
                errors++
            }
            if (!($6 in allowed_tier)) {
                print "registry: invalid tier: " $1 " -> " $6 > "/dev/stderr"
                errors++
            }
            if (!($7 in allowed_os)) {
                print "registry: invalid min_os: " $1 " -> " $7 > "/dev/stderr"
                errors++
            }
            if (!($8 in allowed_hw)) {
                print "registry: invalid min_hw: " $1 " -> " $8 > "/dev/stderr"
                errors++
            }
            if (!($9 in allowed_status)) {
                print "registry: invalid status: " $1 " -> " $9 > "/dev/stderr"
                errors++
            }
            if ($9 == "done" && ($11 == "-" || $12 == "-")) {
                print "registry: done row lacks acceptance/evidence: " $1 > "/dev/stderr"
                errors++
            }
            if ($5 == "must" && $9 == "out-of-scope") {
                print "registry: must row is out-of-scope: " $1 > "/dev/stderr"
                errors++
            }
            dep_line[NR] = $1
            dep_text[NR] = $10
        }
        END {
            for (line in dep_line) {
                if (dep_text[line] == "-") continue
                n = split(dep_text[line], deps, ",")
                for (i = 1; i <= n; i++) {
                    if (!(deps[i] in id_seen)) {
                        print "registry: missing dependency: " dep_line[line] " -> " deps[i] > "/dev/stderr"
                        errors++
                    }
                }
            }
            if (errors > 0) {
                print "registry: FAILED (" errors " problems)" > "/dev/stderr"
                exit 1
            }
            print "registry: OK (" capability_count " capabilities, " bundle_count " bundles)"
        }
    ' "${bundles}" "${registry}"
}

render_coverage() {
    awk -F '\t' '
        BEGIN {
            print "# Amiga SDK Coverage"
            print ""
            print "> Generated from `docs/amiga-sdk/registry.tsv` and `docs/amiga-sdk/bundles.tsv`."
            print "> Run `make sdk-registry`; do not edit this file by hand."
            print ""
            print "| Bundle | Tier | Min OS | Done | Total | Must open | Tracking |"
            print "|---|---|---|---|---|---|---|"
        }
        FNR == 1 { next }
        NF == 0 || $1 ~ /^#/ { next }
        NR == FNR {
            b_order[++b_count] = $1
            b_title[$1] = $2
            b_tier[$1] = $3
            b_os[$1] = $4
            b_hw[$1] = $5
            b_issue[$1] = $7
            next
        }
        {
            total[$2]++
            if ($9 == "done") done[$2]++
            if ($5 == "must" && $9 != "done" && $9 != "out-of-scope") must_open[$2]++
            row_order[++r_count] = $1
            row_line[$1] = $0
        }
        END {
            for (i = 1; i <= b_count; i++) {
                id = b_order[i]
                issue = (b_issue[id] == "-" ? "" : b_issue[id])
                printf "| %s | %s | %s | %d | %d | %d | %s |\n", id, b_tier[id], b_os[id], done[id] + 0, total[id] + 0, must_open[id] + 0, issue
            }
            for (i = 1; i <= b_count; i++) {
                id = b_order[i]
                print ""
                print "## " id " — " b_title[id]
                print ""
                print "Tier: " b_tier[id] "; min OS: " b_os[id] "; min hardware: " b_hw[id] "."
                print ""
                print "| ID | Capability | Priority | Status | Acceptance |"
                print "|---|---|---|---|---|"
                for (j = 1; j <= r_count; j++) {
                    split(row_line[row_order[j]], f, "\t")
                    if (f[2] != id) continue
                    cap = f[4]; gsub(/\|/, "\\|", cap)
                    printf "| %s | %s | %s | %s | %s |\n", f[1], cap, f[5], f[9], f[11]
                }
            }
        }
    ' "${bundles}" "${registry}"
}

write_issues() {
    local out_dir="${1:-${repo_root}/build/amiga-sdk/issues}"
    mkdir -p "${out_dir}"
    local id title tier min_os min_hw count=0
    while IFS=$'\t' read -r id title tier min_os min_hw _; do
        [ -n "${id}" ] || continue
        local body="${out_dir}/${id}.md"
        {
            printf '# Bundle %s: %s\n\n' "${id}" "${title}"
            printf 'Baseline: %s; min OS %s; min hardware %s.\n' "${tier}" "${min_os}" "${min_hw}"
            printf '\nGenerated from `docs/amiga-sdk/registry.tsv`; do not edit this body by hand.\n\n'
            printf '## Capabilities\n\n'
            awk -F '\t' -v bundle="${id}" '
                NR == 1 { next }
                NF == 0 || $1 ~ /^#/ { next }
                $2 == bundle {
                    mark = ($9 == "done") ? "x" : " "
                    suffix = ""
                    if ($9 == "deferred") suffix = " (deferred)"
                    else if ($9 == "out-of-scope") suffix = " (out-of-scope)"
                    printf "- [%s] `%s` %s (%s, min OS %s)%s\n", mark, $1, $4, $5, $7, suffix
                }
            ' "${registry}"
        } > "${body}"
        count=$((count + 1))
    done < <(awk -F '\t' 'NR > 1 && NF > 0 && $1 !~ /^#/ { print $1 "\t" $2 "\t" $3 "\t" $4 "\t" $5 }' "${bundles}")
    echo "registry: wrote ${count} issue bodies to ${out_dir}"
}

case "${command}" in
    check)
        check_registry
        ;;
    coverage)
        check_registry >/dev/null
        render_coverage
        ;;
    issues)
        check_registry >/dev/null
        write_issues "${2:-}"
        ;;
    *)
        usage
        ;;
esac

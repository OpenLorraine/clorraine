#!/usr/bin/env bash
# Installer internals for the Linux CLI candidate.
#
# The public cases run the script as a user. These cases source the same
# template and exercise the helpers that are hard to reach through the command
# line: quoting, config resolution, lock liveness, archive validation, and
# activation rollback.

set -u

. "$(dirname "$0")/install_fixtures.sh"

# The template skips main when sourced for its helpers.
CLORRAINE_INSTALL_LIB=1 . "$installer_template"
set +eu

# Single-quote escaping produces one inert POSIX shell word.
single_quote_is_posix_safe() {
    local actual
    actual=$(single_quote "a b")
    if [ "$actual" = "'a b'" ]; then pass; else fail "space quoting: $actual"; fi
    actual=$(single_quote "a'b")
    if [ "$actual" = "'a'\\''b'" ]; then pass; else fail "quote escaping: $actual"; fi
    actual=$(single_quote 'a$b*c`d')
    if [ "$actual" = "'a\$b*c\`d'" ]; then pass; else fail "metacharacter quoting: $actual"; fi
}

# The config directory matches the CLI's XDG rule.
config_dir_uses_an_absolute_xdg_home_only() {
    local actual
    actual=$(XDG_CONFIG_HOME=/tmp/xdg config_dir)
    if [ "$actual" = /tmp/xdg/clorraine ]; then pass; else fail "absolute XDG: $actual"; fi
    actual=$(XDG_CONFIG_HOME=relative HOME=/tmp/home config_dir)
    if [ "$actual" = /tmp/home/.config/clorraine ]; then pass; else fail "relative XDG: $actual"; fi
}

# A live owner is not stale; a missing PID is.
lock_owner_liveness_is_checked() {
    local root owner
    root="$(new_root)"
    owner="$root/owner"
    printf '%s %s\n' "$$" "$(awk '{print $22}' "/proc/$$/stat")" >"$owner"
    if lock_is_stale "$owner"; then fail "live lock marked stale"; else pass; fi
    printf '%s %s\n' 999999 0 >"$owner"
    if lock_is_stale "$owner"; then pass; else fail "dead lock not marked stale"; fi
    printf '%s %s\n' "$$" "$(($(awk '{print $22}' "/proc/$$/stat") + 1))" >"$owner"
    if lock_is_stale "$owner"; then pass; else fail "reused PID with a different start time is stale"; fi
    rm -f "$owner"
    if lock_is_stale "$owner"; then fail "missing owner treated as stale"; else pass; fi
    rm -rf "$root"
}

# The marked block is removed while surrounding profile lines stay.
path_block_is_stripped() {
    local root profile
    root="$(new_root)"
    profile="$root/profile"
    cat >"$profile" <<'PROFILE'
before
# >>> clorraine >>>
clorraine_bin='/tmp/bin'
export PATH
# <<< clorraine <<<
after
PROFILE
    strip_path_block "$profile" "$root/stripped"
    check_contains "$root/stripped" "before" "line before the block"
    check_contains "$root/stripped" "after" "line after the block"
    check_not_contains "$root/stripped" "# >>> clorraine >>>" "block removed"
    rm -rf "$root"
}

# A well-formed archive reports its single top-level directory.
archive_validation_accepts_one_top_level_tree() {
    local root stage archive top
    root="$(new_root)"
    stage="$(make_stage "$root")"
    archive="$(make_archive "$root" "$stage")"
    top=$(validate_archive "$archive")
    if [ "$top" = "clorraine-1.2.0-x86_64-linux" ]; then pass; else fail "top-level: $top"; fi
    rm -rf "$root"
}

# A symlink member is rejected before extraction.
archive_validation_rejects_a_symlink() {
    local root stage archive status
    root="$(new_root)"
    stage="$(make_stage "$root")"
    ln -s /etc/passwd "$stage/bin/evil"
    archive="$(make_archive "$root" "$stage")"
    set +e
    ( validate_archive "$archive" ) >"$root/stdout" 2>"$root/stderr"
    status=$?
    set -e
    check_status 1 "$status" "symlink member"
    check_contains "$root/stderr" "unsafe member" "symlink message"
    rm -rf "$root"
}

# The extracted tree must report the pinned version.
verify_tree_checks_the_binary_version() {
    local root stage status
    root="$(new_root)"
    stage="$(make_stage "$root" 1.1.0)"
    set +e
    ( VERSION=1.2.0; verify_tree "$stage" ) >"$root/stdout" 2>"$root/stderr"
    status=$?
    set -e
    check_status 1 "$status" "wrong tree version"
    check_contains "$root/stderr" "version check" "tree-version message"
    rm -rf "$root"
}

# Activation rolls back to the previous current.
activation_and_rollback_restore_current() {
    local root share
    root="$(new_root)"
    share="$root/share"
    mkdir -p "$share/versions/old" "$share/versions/new"
    ln -s versions/old "$share/current"
    activate "$share" versions/new
    if [ "$(readlink "$share/current")" = versions/new ]; then pass; else fail "activation did not switch current"; fi
    rollback "$share"
    if [ "$(readlink "$share/current")" = versions/old ]; then pass; else fail "rollback did not restore current"; fi
    rm -rf "$root"
}

# A failure before activation must not remove a healthy current.
rollback_before_activation_keeps_current() {
    local root share
    root="$(new_root)"
    share="$root/share"
    mkdir -p "$share/versions/old"
    ln -s versions/old "$share/current"
    activation_started=0
    old_current=
    rollback "$share"
    if [ "$(readlink "$share/current")" = versions/old ]; then pass; else fail "pre-activation rollback removed current"; fi
    rm -rf "$root"
}

# Terminal control bytes never reach an error message.
sanitize_removes_terminal_controls() {
    local actual
    actual=$(sanitize "$(printf 'a\033b\001c')")
    if [ "$actual" = "abc" ]; then pass; else fail "sanitize: $actual"; fi
}

single_quote_is_posix_safe
sanitize_removes_terminal_controls
config_dir_uses_an_absolute_xdg_home_only
lock_owner_liveness_is_checked
path_block_is_stripped
archive_validation_accepts_one_top_level_tree
archive_validation_rejects_a_symlink
verify_tree_checks_the_binary_version
activation_and_rollback_restore_current
rollback_before_activation_keeps_current
finish

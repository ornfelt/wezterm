#!/usr/bin/env bash
# Launches the locally built (custom) wezterm from this checkout without touching
# the wezterm that is installed under /usr/bin. Both read the same config file,
# so the only difference is the binary.
#
# Usage examples:
#
# run the custom build (uses the same ~/.wezterm.lua as the installed wezterm):
# ./run-custom-wezterm.sh
#
# build release first, then run:
# ./run-custom-wezterm.sh --build
#
# run a checkout in another directory (positional or named):
# ./run-custom-wezterm.sh ~/src/wezterm
# ./run-custom-wezterm.sh --path ~/src/wezterm
#
# open the new window in a specific directory:
# ./run-custom-wezterm.sh --cwd ~/Code2
#
# try a throwaway config instead of the real one:
# ./run-custom-wezterm.sh --config /tmp/smear-test.lua
#
# print the equivalent commands instead of running anything:
# ./run-custom-wezterm.sh --build --show-cmd
#
# print this help and exit (help / -h / --help, any casing):
# ./run-custom-wezterm.sh help
# ./run-custom-wezterm.sh --help

set -u

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT_NAME="$(basename -- "${BASH_SOURCE[0]}")"

REPO_PATH="$SCRIPT_DIR"   # checkout to build/run; defaults to this script's dir
BUILD=0                   # run `cargo build --release` before launching
CONFIG=""                 # config file passed through to wezterm; default is
                          # wezterm's own lookup, which finds the same
                          # ~/.wezterm.lua the installed wezterm uses
CWD=""                    # directory the new window should start in
SHOW_CMD=0                # print the commands this run would execute instead

INSTALLED_EXE='/usr/bin/wezterm-gui'

color_ok()       { printf '\033[0;32m%s\033[0m\n' "$*"; }
color_err()      { printf '\033[0;31m%s\033[0m\n' "$*" >&2; }
color_warn()     { printf '\033[0;33m%s\033[0m\n' "$*"; }
color_info()     { printf '\033[0;36m%s\033[0m\n' "$*"; }
color_info_alt() { printf '\033[0;35m%s\033[0m\n' "$*"; }

usage() {
    color_info_alt "$SCRIPT_NAME - run the locally built (custom) wezterm"
    cat <<'USAGE'

Launches the wezterm built in this checkout without touching the wezterm
installed under /usr/bin. Both read the same config file, so the only
difference is the binary.
USAGE
    echo
    color_info 'Usage:'
    printf '  ./%s [path] [options]\n\n' "$SCRIPT_NAME"
    color_info 'Options:'
    printf '  path, --path <dir>    wezterm checkout to build/run; also accepted positionally\n'
    printf '                        (default: %s)\n' "$SCRIPT_DIR"
    cat <<'USAGE'
  --build               run cargo build --release before launching
  --config <file>       config file to pass to wezterm
                        (default: wezterm's own lookup, ~/.wezterm.lua)
  --cwd <dir>           directory the new window starts in
  --show-cmd            print the equivalent commands instead of running them
  -h, --help, help      show this help
USAGE
    echo
    color_info 'Examples:'
    printf '  ./%s                        run the custom build\n' "$SCRIPT_NAME"
    printf '  ./%s --build                build release, then run\n' "$SCRIPT_NAME"
    printf '  ./%s ~/src/wezterm          use another checkout\n' "$SCRIPT_NAME"
    printf '  ./%s --cwd ~/Code2          start the new window there\n' "$SCRIPT_NAME"
    printf '  ./%s --config /tmp/smear-test.lua\n' "$SCRIPT_NAME"
    printf '  ./%s --build --show-cmd     print, do not execute\n' "$SCRIPT_NAME"
    echo
    color_warn 'Notes:'
    cat <<'USAGE'
  Options are matched case-insensitively.
  Without --build the binary is looked up in CARGO_TARGET_DIR, target/, build/,
  any other target*/ or build*/ directory, then the target dir cargo reports;
  a release build is preferred, and a debug one is used only when there is no
  release build.
  If the build fails on missing system libraries, run the checkout's get-deps.
  On machines whose name contains "devpc", --build also sets
    CARGO_HTTP_CHECK_REVOKE=false
USAGE
}

# options are matched case-insensitively, like the ps1 version
while [ $# -gt 0 ]; do
    arg="$1"
    case "$(printf '%s' "$arg" | tr '[:upper:]' '[:lower:]')" in
        --path)     REPO_PATH="$2"; shift 2 ;;
        --build)    BUILD=1; shift ;;
        --config)   CONFIG="$2"; shift 2 ;;
        --cwd)      CWD="$2"; shift 2 ;;
        --show-cmd) SHOW_CMD=1; shift ;;
        -h|--help|help|/h|/help|/?|-?)  usage; exit 0 ;;
        -*)         color_err "Unknown option: $arg"; usage; exit 1 ;;
        *)          REPO_PATH="$arg"; shift ;;
    esac
done

# On devpc machines cargo's certificate revocation check fails, so builds there
# need CARGO_HTTP_CHECK_REVOKE=false. COMPUTERNAME is only set on Windows
# (Git Bash/MSYS), so fall back to the hostname; matched case-insensitively.
MACHINE_NAME="${COMPUTERNAME:-${HOSTNAME:-$(hostname 2>/dev/null)}}"
IS_DEVPC=0
case "$(printf '%s' "$MACHINE_NAME" | tr '[:upper:]' '[:lower:]')" in
    *devpc*) IS_DEVPC=1 ;;
esac

REPO_PATH="$(cd -- "$REPO_PATH" 2>/dev/null && pwd)" || {
    color_err "No such directory: $REPO_PATH"
    exit 1
}
# Under Git Bash the binary carries .exe; MSYS's test -x usually finds it
# either way, but check both
is_exe() { [ -x "$1" ] || [ -x "$1.exe" ]; }

# cargo builds into <target dir>/release (or /debug). The target dir is
# `target` unless CARGO_TARGET_DIR or a .cargo/config build.target-dir moves
# it; `cargo metadata` knows the answer, at the cost of running cargo.
cargo_target_dir() {
    local json dir
    if command -v cargo >/dev/null 2>&1; then
        json="$(cargo metadata --format-version 1 --no-deps --manifest-path "$REPO_PATH/Cargo.toml" 2>/dev/null)"
        dir="$(printf '%s' "$json" | sed -n 's/.*"target_directory":"\(\([^"\\]\|\\.\)*\)".*/\1/p' | sed 's/\\\\/\\/g')"
        if [ -n "$dir" ]; then
            # Git Bash reports a Windows path; make it one bash can use
            command -v cygpath >/dev/null 2>&1 && dir="$(cygpath -u "$dir")"
            printf '%s\n' "$dir"
            return
        fi
    fi
    if [ -n "${CARGO_TARGET_DIR:-}" ]; then
        printf '%s\n' "$CARGO_TARGET_DIR"
    else
        printf '%s\n' "$REPO_PATH/target"
    fi
}

# Directories that may hold a build, one per line, in the order they are
# searched: CARGO_TARGET_DIR, target, build, then any other target* / build*
list_target_dirs() {
    local d
    {
        [ -n "${CARGO_TARGET_DIR:-}" ] && printf '%s\n' "$CARGO_TARGET_DIR"
        printf '%s\n' "$REPO_PATH/target" "$REPO_PATH/build"
        for d in "$REPO_PATH"/target*/ "$REPO_PATH"/build*/; do
            [ -d "$d" ] && printf '%s\n' "${d%/}"
        done
    } | awk '!seen[$0]++'
}

# The binary to launch: a release build from the first directory that has one,
# else a debug build, else (cargo's own target dir may live elsewhere) the same
# search in the directory cargo metadata reports. Sets FOUND_EXE, FOUND_DEBUG.
find_exe() {
    local round dirs cargo_profile d
    dirs="$(list_target_dirs)"
    for round in 1 2; do
        for cargo_profile in release debug; do
            while IFS= read -r d; do
                [ -n "$d" ] || continue
                if is_exe "$d/$cargo_profile/wezterm-gui"; then
                    FOUND_EXE="$d/$cargo_profile/wezterm-gui"
                    FOUND_DEBUG=0
                    [ "$cargo_profile" = debug ] && FOUND_DEBUG=1
                    return 0
                fi
            done <<< "$dirs"
        done
        dirs="$(cargo_target_dir)"
    done
    return 1
}

DEFAULT_EXE_PATH="$REPO_PATH/target/release/wezterm-gui"
EXE_PATH="$DEFAULT_EXE_PATH"
LAUNCH_IS_DEBUG=0
if [ "$BUILD" -eq 1 ]; then
    # where this build's binary lands
    EXE_PATH="$(cargo_target_dir)/release/wezterm-gui"
elif find_exe; then
    EXE_PATH="$FOUND_EXE"
    LAUNCH_IS_DEBUG="$FOUND_DEBUG"
fi

# wezterm wants --config-file before the subcommand
wezterm_args=()
[ -n "$CONFIG" ] && wezterm_args+=(--config-file "$CONFIG")
wezterm_args+=(start)
[ -n "$CWD" ] && wezterm_args+=(--cwd "$CWD")

if [ "$SHOW_CMD" -eq 1 ]; then
    origin="# equivalent of: ./$SCRIPT_NAME"
    [ "$BUILD" -eq 1 ] && origin+=" --build"
    [ -n "$CONFIG" ] && origin+=" --config $CONFIG"
    [ -n "$CWD" ] && origin+=" --cwd $CWD"
    color_info_alt "$origin"
    if [ "$BUILD" -eq 1 ]; then
        if [ "$IS_DEVPC" -eq 1 ]; then
            color_info_alt "# devpc detected ($MACHINE_NAME)"
            printf 'export CARGO_HTTP_CHECK_REVOKE=false\n'
        fi
        printf 'cargo build --release --manifest-path "%s/Cargo.toml" -p wezterm-gui -p wezterm\n' "$REPO_PATH"
    fi
    printf '"%s"' "$EXE_PATH"
    printf ' "%s"' "${wezterm_args[@]}"
    printf '\n'
    exit 0
fi

if [ "$BUILD" -eq 1 ]; then
    color_info "Building release binaries in $REPO_PATH ..."
    if [ "$IS_DEVPC" -eq 1 ]; then
        color_info_alt "devpc detected (machine name: $MACHINE_NAME); setting CARGO_HTTP_CHECK_REVOKE=false for this build."
        export CARGO_HTTP_CHECK_REVOKE=false
    fi
    if ! cargo build --release --manifest-path "$REPO_PATH/Cargo.toml" -p wezterm-gui -p wezterm; then
        status=$?
        color_err "cargo build failed with exit code $status"
        color_warn "If system libraries are missing, run: $REPO_PATH/get-deps"
        exit $status
    fi
    color_ok 'Build finished.'
fi

if ! is_exe "$EXE_PATH"; then
    if [ "$BUILD" -eq 1 ]; then
        color_err "The build finished but its binary is missing: $EXE_PATH"
    else
        color_err 'Not built yet: no wezterm-gui in any release or debug build directory.'
        color_warn "Searched: $( { list_target_dirs; cargo_target_dir; } | awk '!seen[$0]++' | paste -sd, - | sed 's/,/, /g')"
    fi
    color_warn 'Run this script with --build, or run cargo build --release yourself.'
    exit 1
fi
if [ "$LAUNCH_IS_DEBUG" -eq 1 ]; then
    color_warn "No release build found; running a debug build, which is much slower: $EXE_PATH"
elif [ "$EXE_PATH" != "$DEFAULT_EXE_PATH" ]; then
    color_info_alt "No target/release build; using the one found in ${EXE_PATH%/release/wezterm-gui}."
fi

if [ -x "$INSTALLED_EXE" ]; then
    color_info "Installed wezterm is left alone: $INSTALLED_EXE"
fi
color_info "Launching custom build: $EXE_PATH"
if [ -n "$CONFIG" ]; then
    color_info_alt "Config file: $CONFIG"
else
    color_info_alt 'Config file: wezterm default lookup (same file as the installed wezterm)'
fi

if ! "$EXE_PATH" "${wezterm_args[@]}"; then
    status=$?
    color_err "wezterm-gui exited with code $status"
    exit $status
fi
color_ok 'Done.'

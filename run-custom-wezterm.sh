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
  If the build fails on missing system libraries, run the checkout's get-deps.
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

REPO_PATH="$(cd -- "$REPO_PATH" 2>/dev/null && pwd)" || {
    color_err "No such directory: $REPO_PATH"
    exit 1
}
EXE_PATH="$REPO_PATH/target/release/wezterm-gui"

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
        printf 'cargo build --release --manifest-path "%s/Cargo.toml" -p wezterm-gui -p wezterm\n' "$REPO_PATH"
    fi
    printf '"%s"' "$EXE_PATH"
    printf ' "%s"' "${wezterm_args[@]}"
    printf '\n'
    exit 0
fi

if [ "$BUILD" -eq 1 ]; then
    color_info "Building release binaries in $REPO_PATH ..."
    if ! cargo build --release --manifest-path "$REPO_PATH/Cargo.toml" -p wezterm-gui -p wezterm; then
        status=$?
        color_err "cargo build failed with exit code $status"
        color_warn "If system libraries are missing, run: $REPO_PATH/get-deps"
        exit $status
    fi
    color_ok 'Build finished.'
fi

if [ ! -x "$EXE_PATH" ]; then
    color_err "Not built yet: $EXE_PATH"
    color_warn 'Run this script with --build, or run cargo build --release yourself.'
    exit 1
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

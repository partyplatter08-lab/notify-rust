#!/usr/bin/env sh
set -e

BOLD_GREEN="\033[1;35m"
RESET="\033[0m"

status() {
    label="$1"
    shift
    printf "%b%12s%b %s\n" "$BOLD_GREEN" "$label" "$RESET" "$*"
}

EXAMPLE="${1:-un_async}"
APP=target/debug/examples/bundle/osx/notify-rust.app

status "Bundling" "$EXAMPLE (example)"
cargo bundle --example "$EXAMPLE"

status "Signing" "$APP (ad-hoc)"
codesign --force --deep --sign - "$APP"

status "Opening" "$APP"
pkill -x "$EXAMPLE" 2>/dev/null || true
open "$APP"

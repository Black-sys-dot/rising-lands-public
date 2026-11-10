#!/bin/sh
# Standalone component tests of the macOS host (no game data needed).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
SRC="$HERE/../src"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
run() {
    name=$1; shift
    sources=""
    for s in "$@"; do sources="$sources $SRC/$s.swift"; done
    # shellcheck disable=SC2086
    xcrun swiftc -swift-version 5 -module-cache-path "$TMP/modules" $sources "$HERE/$name.swift" -o "$TMP/$name"
    "$TMP/$name"
}
run HotkeyBindingsTests NativeHotkeyBindings
run HotkeyInputRouterTests NativeHotkeyBindings NativeHotkeyInputRouter
run HotkeyPreferencesModelTests NativeHotkeyBindings NativeHotkeyPreferences
run ViewportPolicyTests NativeViewportPolicy
run GameSpeedTests NativeGameSpeed
run PlayerOptionsTests NativePlayerOptions
run BenchmarkArgumentsTests NativeBenchmarkArguments
echo "All macOS component tests passed."

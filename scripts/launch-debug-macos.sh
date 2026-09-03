#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
bundle_path="${TMPDIR:-/private/tmp}/OziClock-Debug.app"
contents_path="$bundle_path/Contents"
executable_path="$contents_path/MacOS/OziClock"

cd "$project_root"
cargo build -p oziclock-desktop

mkdir -p "$contents_path/MacOS"
ln -sfn "$project_root/target/debug/oziclock-desktop" "$executable_path"

if [ ! -f "$contents_path/Info.plist" ]; then
    cat >"$contents_path/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>OziClock Debug</string>
    <key>CFBundleExecutable</key>
    <string>OziClock</string>
    <key>CFBundleIdentifier</key>
    <string>com.ozinka.oziclock.debug</string>
    <key>CFBundleName</key>
    <string>OziClock Debug</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
</dict>
</plist>
PLIST
fi

open -n "$bundle_path"

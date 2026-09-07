#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
profile=debug
case "${1:-}" in
  --release) profile=release; cargo build --release --locked ;;
  '') cargo build --locked ;;
  *) echo 'Usage: sh scripts/bundle-macos.sh [--release]' >&2; exit 2 ;;
esac
bundle="target/$profile/Asana GPUI.app"
mkdir -p "$bundle/Contents/MacOS"
cp "target/$profile/asana-gpui" "$bundle/Contents/MacOS/asana-gpui"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Asana GPUI</string>
<key>CFBundleDisplayName</key><string>Asana GPUI</string>
<key>CFBundleIdentifier</key><string>jp.ktutumi.asana-gpui</string>
<key>CFBundleExecutable</key><string>asana-gpui</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>12.0</string>
</dict></plist>
PLIST
printf '%s\n' "$bundle"

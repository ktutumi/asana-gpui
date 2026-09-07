#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

if [ "$(uname -s)" != Linux ]; then
  echo 'Run this script on Linux.' >&2
  exit 1
fi

profile=debug
case "${1:-}" in
  --release) profile=release ;;
  '') ;;
  *) echo 'Usage: sh scripts/bundle-linux.sh [--release]' >&2; exit 2 ;;
esac
if [ "$#" -gt 1 ]; then
  echo 'Usage: sh scripts/bundle-linux.sh [--release]' >&2
  exit 2
fi

# Keep the build and bundle paths deterministic even with Cargo overrides.
host=$(rustc -vV | sed -n 's/^host: //p')
if [ "$profile" = release ]; then
  cargo build --locked --release --target "$host" --target-dir target
else
  cargo build --locked --target "$host" --target-dir target
fi

bundle="target/$profile/asana-gpui-linux"
install -Dm755 "target/$host/$profile/asana-gpui" "$bundle/bin/asana-gpui"
install -Dm644 assets/linux/jp.ktutumi.asana-gpui.desktop \
  "$bundle/share/applications/jp.ktutumi.asana-gpui.desktop"
install -Dm644 assets/linux/jp.ktutumi.asana-gpui.svg \
  "$bundle/share/icons/hicolor/scalable/apps/jp.ktutumi.asana-gpui.svg"
printf '%s\n' "$bundle"

#!/usr/bin/env bash
# Build an AppImage from an already compiled lnpaint binary.
#
# Usage: lnpaint/build/appimage/build.sh [output]
#
# Environment:
#   BIN          path to the compiled lnpaint binary
#   APPIMAGETOOL path to appimagetool (default: appimagetool in PATH)

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
OUT="${1:-$ROOT/target/appimage/lnpaint.AppImage}"
APPDIR="$(dirname "$OUT")/AppDir"

VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/lnpaint/Cargo.toml" | head -n1)"
if [[ -z "$VERSION" ]]; then
    echo "cannot read version from lnpaint/Cargo.toml" >&2
    exit 1
fi

case "$VERSION" in
    *-dev) ICON="$ROOT/lnpaint/res/logos/icon_dev-512x.png" ;;
    *) ICON="$ROOT/lnpaint/res/logos/icon_lime-512x.png" ;;
esac

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
BIN="${BIN:-$ROOT/target/$TARGET/release/lnpaint}"
[[ -x "$BIN" ]] || BIN="$ROOT/target/release/lnpaint"
if [[ ! -x "$BIN" ]]; then
    echo "missing lnpaint binary; build it first or set BIN" >&2
    exit 1
fi

APPIMAGETOOL="${APPIMAGETOOL:-appimagetool}"
if ! command -v "$APPIMAGETOOL" >/dev/null 2>&1; then
    echo "appimagetool not found; install it or set APPIMAGETOOL" >&2
    exit 1
fi

rm -rf "$APPDIR"
install -Dm755 "$BIN" "$APPDIR/usr/bin/lnpaint"
install -Dm755 "$HERE/AppRun" "$APPDIR/AppRun"
install -Dm644 "$ICON" "$APPDIR/usr/share/icons/hicolor/512x512/apps/lnpaint.png"
ln -s usr/share/icons/hicolor/512x512/apps/lnpaint.png "$APPDIR/lnpaint.png"
ln -s lnpaint.png "$APPDIR/.DirIcon"

sed "s|@VERSION@|$VERSION|" "$HERE/lnpaint.desktop" > "$APPDIR/lnpaint.desktop"
install -Dm644 "$APPDIR/lnpaint.desktop" "$APPDIR/usr/share/applications/lnpaint.desktop"

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "$APPIMAGETOOL" --no-appstream "$APPDIR" "$OUT"

echo "$OUT"

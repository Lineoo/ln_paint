#!/usr/bin/env bash
# pwd: should be executed at workspace root
# copy icons

set -e

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# copy to builds

icon=${1:-icon_dev}
cp lnpaint/res/logos/$icon-512x.png lnpaint/build/android/app/src/main/res/mipmap-xxxhdpi/icon.png
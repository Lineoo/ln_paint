#!/usr/bin/env bash
# pwd: should be executed at workspace root
# update icons bitmaps in `res` folder

set -e

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# copy to builds

icon=${1:-icon_dev}
cp ln_paint/res/logos/$icon-512x.png ln_paint/build/android/app/src/main/res/mipmap-xxxhdpi/icon.png
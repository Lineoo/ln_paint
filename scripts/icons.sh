#!/usr/bin/env bash
# pwd: should be executed at workspace root
# update icons bitmaps in `res` folder

set -e

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# svg generation

files=(icon_dark icon_dev icon_light icon_lime icon_symbolic)
for f in "${files[@]}"; do
    resvg -w 512 -h 512 ln_paint/res/logos/$f.svg ln_paint/res/logos/$f-512x.png
done

# copy to builds

./scripts/cp-icons.sh $1
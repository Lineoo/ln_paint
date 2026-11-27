#!/usr/bin/env bash
# pwd: should be executed at workspace root

set -euo pipefail

for f in ln_paint/res/icon/*.svg; do
    [[ -f "$f" ]] || continue

    base="$(basename "$f" .svg)"
    ident="$(printf '%s' "$base" | tr '[:lower:]-.' '[:upper:]__')"

    printf 'pub const %s: &'\''static [u8] = include_bytes!("../res/icon/%s.svg");\n' \
        "$ident" "$base"
done
#!/usr/bin/env bash
# pwd: should be executed at workspace root
# Export regression images into the given directory, defaults to the baselines
# Regression test is performed in CI. Don't run it locally.

set -e

OUT=${1:-ln_paint/examples}

export LNPAINT_HEADLESS=1
export LNPAINT_TMP_SAVE=1
export LNPAINT_FORCE_FALLBACK_ADAPTER=1
export LNPAINT_DESKTOP_UI=1
export LNPAINT_SCALE_FACTOR=1.5
export LNPAINT_WINDOW_SIZE=1200x900

LNPAINT_EXPORT_PATH=$OUT/text_preview.png cargo run --example text_preview
LNPAINT_EXPORT_PATH=$OUT/widget_showcase.png cargo run --example widget_showcase

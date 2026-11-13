#!/usr/bin/env bash
# pwd: should be executed at workspace root
# modify ln_paint's configuration for release

set -e

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

#### Android & Gradle ####

# `ln_paint/build/android/app/build.gradle.kts`
# applicationId: "org.linn.lnpaint.dev" -> "org.linn.lnpaint"
sed -i 's/applicationId = "org\.linn\.lnpaint\.dev/applicationId = "org.linn.lnpaint/' ln_paint/build/android/app/build.gradle.kts

# `ln_paint/build/android/app/src/main/res/values/strings.xml`
# app_name: LnPaint Dev -> LnPaint
sed -i 's/>LnPaint Dev</>LnPaint</' ln_paint/build/android/app/src/main/res/values/strings.xml

#### Rustc ####

# `env!("LNPAINT_RELEASE")`
# Release Mode:
# `ln_paint/src/save.rs::get_file_path`
export LNPAINT_RELEASE=1
echo LNPAINT_RELEASE=1

#### Resources ####

./scripts/cp-icons.sh "icon_lime"
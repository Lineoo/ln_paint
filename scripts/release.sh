#!/usr/bin/env bash
# pwd: should be executed at workspace root
# modify ln_drawer's configuration for release

set -e

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

#### Android & Gradle ####

# `ln_drawer/build/android/app/build.gradle.kts`
# applicationId: "org.linn.lndrawer.dev" -> "org.linn.lndrawer"
sed -i 's/applicationId = "org\.linn\.lndrawer\.dev/applicationId = "org.linn.lndrawer/' ln_drawer/build/android/app/build.gradle.kts

# `ln_drawer/build/android/app/src/main/res/values/strings.xml`
# app_name: LnDrawer Dev -> LnDrawer
sed -i 's/>LnDrawer Dev</>LnDrawer</' ln_drawer/build/android/app/src/main/res/values/strings.xml

#### Rustc ####

# `env!("LNDRAWER_RELEASE")`
# Release Mode:
# `ln_drawer/src/save.rs::get_file_path`
export LNDRAWER_RELEASE=1
echo LNDRAWER_RELEASE=1

#### Resources ####

./scripts/icons.sh "icon_lime"
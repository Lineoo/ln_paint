<p><img src="ln_paint/res/logos/banner.svg" alt="LnPaint Banner" align="center"/></p>

**English** | [中文](./README.zh_CN.md)

----------

<img src="ln_paint/res/logos/icon_lime.svg" alt="LnPaint Icon" align="right"/>

**LnPaint** is a simple, light-weight, GPU-accelerated, cross-platform painting application written in Rust, driven by `winit` and `wgpu`.

[/laɪn'peɪnt/](https://itinerarium.github.io/phoneme-synthesis/?w=/laɪn'peɪnt/)

## Features ##

- Infinite-sized lossless canvas
- Smooth at any zoom level
- Efficient and reliable canvas storage
- Mobile support

## Build ##

Manually build through `cargo`:

```
$ git clone --depth 1 https://github.com/Lineoo/ln_paint
$ cd ln_paint
$ cargo build --package=ln_paint --bin ln_paint
```

### Android

Android platform needs Android SDK 34, Build Tools 34.0.0, NDK 29.0.14206865 and JDK 17.

Use `cargo-ndk` to help setup environment variables, see `docs/ndk.md` (Chinese) for troubleshooting.

```
$ cargo install cargo-ndk
$ cd ln_paint/build/android
$ ./gradlew assembleDebug
```

Or use `./scripts/apk-run.sh` for short.

## License ##

The entire repository is licensed under **GNU Affero General Public License v3.0 or later** (LICENSE or https://opensource.org/license/AGPL-3.0).

This workspace contains multiple crates with different licenses:

- Crate **ln_paint** is licensed under **AGPL-3.0-or-later**.
- Crate **ln_world** is dual-licensed under **MIT OR Apache-2.0**.

See the crate-level README file or license field for details.

Some icons in this project are from [Lucide Icons](https://lucide.dev/).
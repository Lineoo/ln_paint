# AGENTS.md

GPU-accelerated drawing app featured infinite canvas in Rust (winit + wgpu). Cargo workspace with two crates:

- `ln_paint/` - The main app. Desktop (binary) entry: `src/bin/ln_paint.rs` -> `lib.rs::desktop_main`; Android (C ABI library) entry: `lib.rs::android_main`.
- `ln_world/` - The management system. The entire app is built on it (`World`, `Element`, `Handle`, `Observer`, `Descriptor`).
- `docs/` - Documents. Most are in Chinese, read some first.

## Tasks

- If assigned to review in read-only mode, assume that users has run `cargo check` and `cargo fmt`. Skip cargo tool calls that'll need write-permission, except when significant type errors are spotted.
- Headless rendering is only experimentally supported, and just tell user when you need it.
- Don't run `cargo check --target aarch64-linux-android`. Tell user to perform Android check or build.
- Don't use clippy. No need to check that (it returns warnings, but we just don't care about that). 
- Tests: `cargo test` — tests are all are pure-compute and run headless.

## Critical gotchas

- `winit` is patched to a customized fork. This fork contains additional Android stylus support.
- `glam::I64Vec2` and `i64` may be used to represent `Q32.32` fixed-point types rather than integers, see `ln_paint/src/unit/fixed.rs`

## Conventions

- Comments should not exceed one line. Keep minimalistic. Don't leave any outdated, unrelated or redundant information.
- Never assuming any behavior. Codes like `unwrap_or_*()` is strictly forbidden unless users explicit allow it. **Panicking is better than skipping potential errors.**
- Never handle errors. Don't use `try_*` family functions in `ln_world`.  Use `unwrap()` if `Result`s are returned. If you REALLY understand the error and REALLY need to handle it, ask user first and make sure they ARE fully aware of the error and intended to handle it. **Panicking is better than skipping unknown errors.**
- WGSL shaders are embedded via `include_str!` and macro-expanded with `shader_compile()` in `ln_paint/src/shaders.rs`. Shared libs in `ln_paint/src/shaders/lib_*.wgsl` are included via `#lib_*` during compiling.

## Documents

Documents in `docs/`. Most of them are in Chinese if not specially marked with language suffix. Sort by importance.

- `index.md` - The overall codebase architecture.
- `style.en.md` - The coding style instructions, list in format `### <tags,> - <title>`.
- `world.md` - About how to use `ln_world`.
- `composite.md` - Color-space notation for rendering and drawing. Important when treating color conventions.
- `paint.md` - Draw call pipeline: batch-draw, stroke-submit, stroke-discard, undo/redo, failure paths, stream-thread synchronization.
- `render.md` - Rendering architecture: render camera, controls, pipeline, instances, redraw rules.
- `redb.md` - `redb` database tables and chunk binary formats.
- `env.md` - Available environment variables.
- `headless.en.md` - How to use headless rendering.
- `ndk.md` - Android `cargo ndk` usage and `cargo ndk-env` fingerprint pitfalls. Not very useful for agents environment.

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Note that for `0.x` releases per Cargo's interpretation of semver, a `0.x → 0.(x+1)` bump is the breaking-or-significant-change channel.

## [Unreleased]

## [0.2.0] — 2026-05-11

### Added

- **Pause / resume API.** `MatrixRainState::pause()`, `resume()`, `is_paused()`. `resume()` clears `last_tick`/`accum` so the next render is a first-render path (no catch-up stutter after a long pause). The standalone binary now toggles pause on the `p` key.
- **Mutation.** `mutation_rate` is now wired through the frame loop: each tick, every cell of every active trail rolls independently against `mutation_rate` and may be replaced with a fresh draw from the configured `CharSet`. Default rate is `0.05`.
- **Glitch.** New per-cell color-flicker pass: with probability `glitch` per cell per frame, the renderer paints that cell with `ColorRamp.head` instead of its gradient color. Head cell (i=0) is unaffected. Default rate is `0.0` (off).
- **Full `CharSet` variants.** `Ascii` (94 printable ASCII glyphs), `Hex` (`0-9 a-f`), `Binary` (`0`/`1`), and `Custom(Vec<char>)` are now wired with validation (rejects empty and `char::is_control`).
- **Full `Theme` variants.** `Amber`, `Cyan`, `Red`, and `Rainbow` now have proper 5-stop color ramps. `Custom(ColorRamp)` lets you supply your own.
- **`--charset <path>` file loading.** The binary now accepts a path to a UTF-8 charset file (≤ 1 MiB) in addition to the four built-in names. Whitespace and control characters are filtered, duplicates deduped (first-seen order preserved).
- **`--mutation-rate` and `--glitch` CLI flags.** Both default to the library defaults (0.05 and 0.0 respectively).
- **Full builder validation.** `MatrixConfigBuilder::build()` now checks every invariant from spec §5.3 with descriptive `InvalidConfig` messages. Boundary values (`density == 0.0`/`1.0`, `min_trail == max_trail`, etc.) remain accepted.
- **Backend passthrough features.** New `crossterm`, `termion`, `termwiz` feature flags each forward to ratatui's same-named feature. The library is now backend-agnostic — embedders pick their backend in one Cargo.toml feature line.
- **Public API additions.** `MatrixRainState::streams_len()`, `MatrixRainState::set_color_count()`, and the `MAX_TRAIL_LIMIT` const are now `pub`. Useful for instrumentation, accessibility/forced-tier rendering, and bound-checking.
- **Full rustdoc.** Every public item is documented with a one-line summary and usage example where applicable. `#![warn(missing_docs)]` is enforced at the crate root. Crate-level docs include a Caveats section per spec §11.
- **Determinism: `MatrixRainState::set_color_count`.** Lets you lock the rendering tier (16, 256, or `u16::MAX` for truecolor) regardless of `TERM`/`COLORTERM`. Used by the snapshot test suite for reproducible output across environments.
- **Snapshot tests.** Five fixed-seed insta snapshots covering 256-color, 16-color, truecolor tiers and extreme aspect ratios (1×N, N×1).
- **Property tests.** Six proptest invariants covering random configs/areas, the per-column stream invariant, sub-area clipping, and oscillating-resize sequences.
- **Benchmark harness.** `cargo bench` runs a criterion-based suite. Measured ~25 µs per frame at 200×60 on M-series Macs — 40× under the spec §9 target.
- **Examples.** New `custom_charset.rs` example demonstrates `CharSet::Custom`.
- **cargo-dist release infrastructure.** GitHub Actions workflow plus `dist-workspace.toml` produce per-target archives for Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64, plus shell + powershell curl-installers on every `v*` tag push.

### Changed

- **Renamed package to `matrix-rain`.** The binary is still named `matrix`. Library users now `use matrix_rain::*`.
- **Library no longer depends on `crossterm`.** The previous color-count detection at `crossterm::style::available_color_count` is now inlined as a direct env-var sniff (`COLORTERM` then `TERM`). `crossterm` is now an optional dep gated behind the `binary` feature.
- **`MatrixConfigBuilder::build()` is no longer always `Ok`.** It now validates every invariant from spec §5.3. Configs that worked under 0.1.0 still work; configs that violated the invariants now fail at build time rather than at first render.
- **Crate metadata.** Added `homepage`, `repository`, `documentation`, and a `package.exclude` list (trims `.beads/`, `.claude/`, `docs/`, `AGENTS.md`, `CLAUDE.md` from the published tarball — drops package size from 329 KiB to 200 KiB).

### Fixed

- **Snapshot stability.** The snapshot suite now uses `fps=1` + `speed=0.001` to suppress wall-clock advance, so renders driven by `state.tick()` produce byte-identical output regardless of test runner speed.

## [0.1.0] — 2026-05-10

### Added

- Initial release. Core `MatrixRain<'a>` widget + `MatrixRainState`, `MatrixConfig` with builder, `Theme::ClassicGreen`, `CharSet::Matrix`. Standalone `matrix` binary with clap CLI, terminal-lifecycle Drop guard, panic hook, Unix signal handling (`SIGINT`/`SIGTERM`/`SIGHUP`), BrokenPipe shutdown. Frame loop with sub-tick `accum`, `MAX_CATCHUP_TICKS=4` after suspend/resume, resize handling. Per-column `Stream` lifecycle with cooldown-gated respawn. Color depth detection cached on state; truecolor smooth interpolation / 256-color nearest-of-5 / 16-color named-collapse rendering tiers. ratatui `StatefulWidget` impl with `area.x`/`area.y` checked-arithmetic coordinate handling.

[Unreleased]: https://github.com/AdamIsrael/matrix-rain/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/AdamIsrael/matrix-rain/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/AdamIsrael/matrix-rain/releases/tag/v0.1.0

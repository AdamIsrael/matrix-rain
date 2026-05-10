# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this crate is

The classic "Matrix digital rain" effect for terminals, shipped as a [ratatui](https://ratatui.rs/) `StatefulWidget` library plus a standalone `matrix` binary. Published as `matrix-rain` on crates.io because the bare `matrix` name is taken; the installed binary is still called `matrix`. See `README.md` for the user-facing introduction.

`docs/001-spec.md` is the authoritative technical spec. `docs/000-spec.md` is a one-paragraph predecessor and is superseded.

## Project status

Roadmap (spec §13) progress:

- **0.1.0 done**: core widget + classic green theme + standalone binary on crossterm.
- **0.2.0 done**: full charset variants (ascii/hex/binary/path), remaining themes (amber/cyan/red/rainbow), full per-invariant builder validation, snapshot tests, property tests, custom_charset example.
- **0.3.0 (next)**: glitch + mutation-rate tuning. `head_white` and `bold_head` are already wired.
- **0.4.0**: criterion benchmarks (<1 ms/frame on 200×60), skip-painting optimizations, optional `simd` feature.
- **1.0.0**: stable API, full rustdoc, multi-backend (termion/termwiz) feature flags, cargo-dist releases.

Use `bd ready` to see what's actionable; `bd dep tree <issue>` for the dependency graph. Task tracking lives in beads (see AGENTS.md and the auto-generated section below).

## Key architectural decisions

(All consistent with spec §5–§6. Don't simplify these away — each exists for a specific reason.)

- **`StatefulWidget`-only**, no plain `Widget`. Animation needs state that persists across frames (`MatrixRainState` holds per-column streams, RNG, timing, cached color count). A stateless wrapper would either reset every frame or hide global mutable state.
- **One `Stream` per terminal column.** `streams: Vec<Stream>` is sized to `area.width`. A `StreamState::{Idle{cooldown}, Active}` enum inside `Stream` models the lifecycle so the glyph buffer (sized once to `max_trail`, per §9 perf hint) survives across retire→respawn cycles.
- **Frame-loop bookkeeping in `state.rs`** (§6.2): empty-area path resets `last_tick`/`last_area`/`accum` so the next non-empty render hits the first-render path; resize handling clamps `head_row` to `[0, h + max_trail)` on height change; tick budget = `elapsed * fps * speed`, capped at `MAX_CATCHUP_TICKS=4` so a process resumed from suspend doesn't try to render hundreds of frames. Fractional tick remainder carries in `accum`.
- **`MatrixRainState` is `Send` but `!Sync`** via a `PhantomData<Cell<()>>` marker field. Rendering the same state into two areas in one frame is intentionally treated as a resize per call.
- **Color depth detection** (`widget.rs`): three tiers — Truecolor (smooth RGB lerp between 5 stops), Color256 (nearest-of-5-stops), Color16 (collapse to nearest named via euclidean distance over a fixed VGA-approx palette). Detection failure → Color16 fallback. Cached on the state on first render via `set_color_count`; never re-detected unless explicitly reset.
- **Builder validates at `build()`** (§5.3, §5.4) and returns `MatrixError`. Boundary values are valid: `density == 0.0` / `1.0`, `min_trail == max_trail`, `mutation_rate == 0.0` / `1.0`, `glitch == 0.0` / `1.0`. Don't reject them.
- **Render coordinates use `checked_add`** and explicit `buf.area` bounds checks (§6.2 step 6) so a misconfigured `Rect` near `u16::MAX` cannot wrap or panic in `Buffer::get_mut`.

## Non-obvious deviations from / additions to the spec

A future Claude won't infer these from reading the spec alone:

- **`MatrixRainState` has a private `last_config: Option<MatrixConfig>`** field, not listed in spec §5.2. It exists because the spec-mandated `pub fn tick(&mut self)` has no parameters but needs `area` + `fps` + `density` + `charset` etc. to actually advance streams. Cached on each render; `tick()` does `Option::take` + put-back to avoid cloning. `tick()` before first render is a silent no-op.
- **Two additional `pub` methods on `MatrixRainState`**: `streams_len(&self) -> usize` (needed by integration tests; useful for instrumentation) and `set_color_count(&mut self, u16)` (needed for deterministic snapshot tests; useful for forcing a tier).
- **Truecolor detection uses `COLORTERM=truecolor|24bit`** as a supplement to `crossterm::style::available_color_count()`. In crossterm 0.27 the latter only returns 8 or 256 (env-sniffs `TERM` for `*256color*`). Truecolor is signalled via `TRUECOLOR_SENTINEL = u16::MAX` in the cached count.
- **Binary feature-gates `clap` + `anyhow` + `signal-hook`** behind a default-on `binary` feature. Library consumers can drop those deps via `default-features = false`.
- **Empty-area handling clears `last_tick` to `None`**. Spec §6.2 step 1 says both "Do not update last_tick" and "treat the next non-empty render as a first render" — taken literally these conflict when `last_tick` was previously `Some`. The resolution clears it operationally so the first-render path runs cleanly.
- **CLI ValueEnums match library capability exactly.** When a new theme/charset lands in the lib, the binary's `ThemeArg` / `CharsetSource` must be extended too — otherwise `--theme amber` etc. wouldn't reach the user. Tests/snapshots also need updating.

## Code layout

`src/lib.rs` re-exports the public API at crate root (`MatrixRain`, `MatrixRainState`, `MatrixConfig`, `MatrixConfigBuilder`, `CharSet`, `Theme`, `ColorRamp`, `MatrixError`). Modules are private (`mod`, not `pub mod`) so the API surface stays flat per spec §5.

- `src/widget.rs` — `MatrixRain<'a>` + `StatefulWidget` impl + tier-aware color picking.
- `src/state.rs` — `MatrixRainState` + frame-loop bookkeeping.
- `src/stream.rs` — per-column `Stream` lifecycle (internal).
- `src/config.rs` — `MatrixConfig` + `MatrixConfigBuilder` + `MAX_TRAIL_LIMIT` const.
- `src/charset.rs` — `CharSet` enum + glyph tables + `validate()`.
- `src/theme.rs` — `Theme` enum + `ColorRamp` + 5 built-in ramps.
- `src/error.rs` — `MatrixError` via `thiserror`.
- `src/bin/matrix.rs` — clap CLI + terminal lifecycle + signal handling + event loop.
- `examples/{standalone,embedded,custom_charset}.rs` — runnable demos.
- `tests/widget_smoke.rs` — 1000-frame TestBackend smoke + resize cycles.
- `tests/snapshot.rs` — fixed-seed insta snapshots (256/16/truecolor tiers; deterministic via `fps=1 + speed=0.001`).
- `tests/property.rs` — proptest invariants.

## Common commands

```bash
cargo build                           # build with default features (lib + bin)
cargo build --no-default-features     # library only (no clap/anyhow/signal-hook)
cargo test                            # all tests (unit + bin + smoke + snapshot + property)
cargo test --test snapshot            # only snapshot tests
cargo test -p matrix-rain <name>      # single test by name substring
cargo run                             # run the standalone binary
cargo run -- --theme rainbow --fps 60 # with CLI options
cargo run --example standalone        # full-screen demo
cargo run --example embedded          # widget-in-layout demo
cargo run --example custom_charset    # CharSet::Custom demo
cargo clippy                          # lint
cargo fmt                             # format
```

If snapshot tests fail and the diff is intentional, regenerate via `INSTA_UPDATE=always cargo test --test snapshot` (or `cargo insta review` if `cargo-insta` is installed) and commit the updated `.snap` files in `tests/snapshots/`.

## Task tracking

Per `AGENTS.md`: use `bd` for task tracking. See the auto-generated Beads section below for the quick-reference commands.


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->

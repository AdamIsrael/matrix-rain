# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

This is a greenfield Rust project. The crate currently contains only the default `cargo new` skeleton (`src/main.rs` printing "Hello, world!", empty `[dependencies]` in `Cargo.toml`). The actual implementation has not started — `docs/001-spec.md` is the source of truth for what is being built.

When starting work, read `docs/001-spec.md` first. It is the authoritative technical spec; `docs/000-spec.md` is a one-paragraph predecessor and should be considered superseded.

## What this crate is

A "Matrix digital rain" effect for terminals, shipped as both:

- A reusable [ratatui](https://ratatui.rs/) **`StatefulWidget`** (library) — published as `matrix-rain` on crates.io because the bare `matrix` name is taken.
- A **standalone binary** (`matrix`) that runs the effect full-screen.

Key architectural decisions baked into the spec (see §5 of `docs/001-spec.md`):

- Only `StatefulWidget` is implemented — no plain `Widget`. Animation requires per-frame state (`MatrixRainState` holds streams, RNG, timing, cached color depth), and a stateless wrapper would either reset every frame or hide global mutable state.
- One `Stream` per terminal column. Resize handling, sub-tick accumulation (`accum`), and a `MAX_CATCHUP_TICKS` cap are all in the render path — don't simplify these away; they exist to handle suspend/resume, fast render loops, and resize without visible jitter. See §6.2.
- `MatrixRainState` is `Send` but not `Sync`; rendering the same state into two areas in one frame is intentionally treated as a resize.
- Color depth detected once via `crossterm::style::available_color_count()` and cached on the state — truecolor / 256 / 16 / unknown all have defined fallbacks (§6.3).
- Builder validates config at `build()` time and returns `MatrixError` (§5.3, §5.4). Boundary values like `density == 0.0`, `min_trail == max_trail`, and `mutation_rate == 1.0` are valid and meaningful — don't reject them.

The intended layout (`src/lib.rs`, `src/widget.rs`, `src/state.rs`, `src/stream.rs`, `src/config.rs`, `src/charset.rs`, `src/theme.rs`, `src/error.rs`, plus `src/bin/matrix.rs`) is in §4 of the spec.

Note: `Cargo.toml` currently sets `edition = "2024"`, but the spec calls for edition 2021 with MSRV 1.74. Confirm with the user before changing either; this may be an intentional update or an oversight.

## Common commands

```bash
cargo build              # build
cargo run                # run the (placeholder) binary
cargo test               # run all tests
cargo test <name>        # run a single test by name substring
cargo clippy             # lint
cargo fmt                # format
```

Once the crate layout from §4 lands, expect `cargo run --example standalone` and `cargo install --path .` to be relevant as well.

## Task tracking

Per `AGENTS.md`: use `bd` for task tracking in this repo.


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

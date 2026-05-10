# Matrix Rain TUI Widget — Technical Specification

## 1. Overview

`matrix` is a Rust library that draws the classic "Matrix digital rain" effect inside a terminal: columns of glowing characters falling from top to bottom, each with a bright leading character, a fading tail, randomized speeds, and characters that flicker and change as they fall.

The project ships in two forms:

1. **Library** (`matrix-rain`) — a reusable widget for the [ratatui](https://ratatui.rs/) terminal-UI framework. Any ratatui application can drop it into a layout region.
2. **Binary** (`matrix`) — a standalone full-screen terminal app that runs the effect by itself, useful as a screensaver or demo.

The crate name `matrix` is already taken on crates.io, so the library is published as `matrix-rain` while the installed command is named `matrix`. See §12.

## 2. Goals & Non-Goals

### Goals
- Faithful reproduction of the Matrix rain look.
- Drop-in ratatui widget that fits inside any layout region.
- Configurable: characters, colors, speed, density, tail length, character-flicker rate.
- Smooth animation at typical terminal refresh rates (30–60 FPS).
- Cross-platform: Linux, macOS, Windows (via crossterm).
- Few dependencies; no `unsafe` code in the core crate.
- Works on 256-color and truecolor terminals; degrades gracefully on 16-color terminals.
- Robust under unusual conditions: tiny or zero-sized areas, terminal resize, system suspend/resume, non-TTY stdout (binary refuses cleanly), broken pipes, and panicking embedders.

### Non-Goals
- Audio, GIF export, or screen recording.
- Direct GPU rendering — output is text characters only.
- Mouse interaction (scrolling, clicking, etc.).
- Unicode beyond what ratatui and crossterm already support. In particular, full-width characters (such as CJK), zero-width combining marks, and grapheme clusters that span multiple code points are not supported in the built-in character sets, because they would either occupy two cells or render unpredictably and break the per-column layout. Users supplying `CharSet::Custom` are responsible for ensuring every character occupies exactly one terminal cell; the builder rejects control characters but cannot detect display width.

## 3. Tech Stack

- **Language:** Rust (2021 edition, minimum supported version 1.74).
- **TUI framework:** [ratatui](https://ratatui.rs/) ≥ 0.26.
- **Terminal backend:** [crossterm](https://crates.io/crates/crossterm) by default; optional features for `termion` and `termwiz`.
- **Random numbers:** `rand` plus `rand_xoshiro` (using `Xoshiro256PlusPlus` / `SmallRng` for cheap per-frame randomness).
- **CLI parsing (binary only):** `clap` v4 with the `derive` feature.
- **Time:** `std::time::Instant` and `Duration` (monotonic, so backward jumps are not a concern); no async runtime needed.
- **Errors:** `thiserror` for typed library errors; `anyhow` in the binary.
- **Logging (optional, dev-only):** `tracing`, behind a `debug` feature flag.

## 4. Crate Layout

```
matrix-rain/
├── Cargo.toml
├── README.md
├── LICENSE-MIT / LICENSE-APACHE
├── examples/
│   ├── standalone.rs          # full-screen demo
│   ├── embedded.rs            # widget inside a larger ratatui layout
│   └── custom_charset.rs      # using katakana / custom symbols
├── src/
│   ├── lib.rs                 # re-exports
│   ├── widget.rs              # MatrixRain widget + StatefulWidget impl
│   ├── state.rs               # MatrixRainState (streams, RNG, frame counter)
│   ├── stream.rs              # Stream / column drop logic
│   ├── config.rs              # MatrixConfig builder
│   ├── charset.rs             # built-in glyph sets
│   ├── theme.rs               # color palettes
│   └── error.rs
├── src/bin/
│   └── matrix.rs              # standalone TUI app
└── tests/
    ├── snapshot.rs            # deterministic-RNG render snapshots
    └── widget_smoke.rs
```

## 5. Public API (Library)

### 5.1 Widget

```rust
pub struct MatrixRain<'a> {
    config: &'a MatrixConfig,
}

impl<'a> MatrixRain<'a> {
    pub fn new(config: &'a MatrixConfig) -> Self;
}

impl<'a> StatefulWidget for MatrixRain<'a> {
    type State = MatrixRainState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

There is intentionally no plain `Widget` implementation. The animation needs state that persists across frames; a stateless wrapper would either reset every frame (breaking the animation) or hide global mutable state behind the API (surprising and not thread-safe). Callers should hold a `MatrixRainState` and call `render_stateful_widget`.

A given `MatrixRainState` is intended to back exactly one logical animation. Rendering the same state into two different `area`s in the same frame is supported but treated as a resize on each call — the streams will resize back and forth and produce visible jitter. Embedders that want two independent animations should hold two states.

### 5.2 State

```rust
pub struct MatrixRainState {
    streams: Vec<Stream>,        // one per column (resized when the area changes)
    last_tick: Option<Instant>,  // None until the first render
    accum: Duration,             // sub-tick remainder carried between renders
    frame: u64,
    rng: SmallRng,
    last_area: Option<Rect>,
    color_count: Option<u16>,    // cached from crossterm on first render
}

impl MatrixRainState {
    pub fn new() -> Self;            // seeded from system entropy
    pub fn with_seed(seed: u64) -> Self;
    pub fn tick(&mut self);          // advance one frame manually (ignores wall clock)
    pub fn reset(&mut self);         // clears streams + timing; preserves RNG seed
}

impl Default for MatrixRainState {
    fn default() -> Self { Self::new() }
}
```

`MatrixRainState` is `Send` but not `Sync`: render takes `&mut self`, and shared cross-thread access is not a supported use case.

The state holds one falling drop per column. On render, it advances based on the time elapsed since `last_tick` and the configured `fps` and `speed`. Lifecycle details:

- **First render:** `last_tick` is `None`; exactly one tick is applied and the timestamp is then recorded. This avoids a huge initial jump that would skip the spawn-in animation.
- **Sub-tick renders:** if the elapsed time produces fewer than one whole tick, the remainder is carried in `accum` and `last_tick` is updated. This prevents a render loop running far faster than `fps` from being starved.
- **`reset()`** clears `streams`, `last_tick`, `accum`, `last_area`, and `frame`; the next render behaves like the first. The RNG and its seed are preserved, so deterministic tests stay deterministic across resets.
- **Mixing `tick()` and wall-clock rendering** is allowed but not recommended. Each `tick()` call advances exactly one frame regardless of wall-clock time and does not touch `last_tick`; embedders that want fully manual driving should use `tick()` exclusively and ignore the wall-clock path.

### 5.3 Config

```rust
#[derive(Clone, Debug)]
pub struct MatrixConfig {
    pub charset: CharSet,
    pub theme: Theme,
    pub fps: u16,                    // animation cap (default 30); must be >= 1
    pub speed: f32,                  // global multiplier (default 1.0); must be > 0 and finite
    pub density: f32,                // 0.0..=1.0 fraction of columns active
    pub min_trail: u16,              // shortest tail length; must be >= 1
    pub max_trail: u16,              // longest tail length; must be >= min_trail
    pub mutation_rate: f32,          // chance per cell per frame that its glyph changes, 0.0..=1.0
    pub bold_head: bool,             // render the leading cell in bold
    pub head_white: bool,            // classic white head, green tail
    pub glitch: f32,                 // random color/glyph perturbation 0.0..=1.0
    pub background: Option<Color>,   // None = transparent (do not write a background style)
}

impl MatrixConfig {
    pub fn builder() -> MatrixConfigBuilder;
}
```

A fluent `MatrixConfigBuilder` exposes one setter per field plus a `.build() -> Result<MatrixConfig, MatrixError>` method that checks:

- `fps >= 1`
- `speed.is_finite() && speed > 0.0`
- `density.is_finite() && (0.0..=1.0).contains(&density)`
- `min_trail >= 1 && min_trail <= max_trail`
- `max_trail <= MAX_TRAIL_LIMIT` (constant, default 1024) — guards against accidentally allocating gigabytes of glyph buffers
- `mutation_rate.is_finite() && (0.0..=1.0).contains(&mutation_rate)`
- `glitch.is_finite() && (0.0..=1.0).contains(&glitch)`
- `charset` resolves to a non-empty character list (rejects `CharSet::Custom(vec![])` with `MatrixError::EmptyCharset`).

Boundary values are explicitly allowed and meaningful: `density == 0.0` produces a static empty field (no columns ever spawn); `density == 1.0` keeps every column busy; `min_trail == max_trail` makes every stream the same length; `mutation_rate == 0.0` freezes glyphs once they spawn; `mutation_rate == 1.0` rerolls every cell every frame.

### 5.4 Character Sets

```rust
pub enum CharSet {
    /// Half-width katakana, digits, and a few punctuation marks (the classic look).
    Matrix,
    /// Printable ASCII subset.
    Ascii,
    /// Hex digits 0–9 a–f.
    Hex,
    /// Binary 0/1.
    Binary,
    /// User-supplied characters. Must be non-empty and contain only single-cell characters.
    Custom(Vec<char>),
}
```

Validation, performed during `build()`:

- The list must be non-empty (`MatrixError::EmptyCharset`).
- Characters where `char::is_control()` returns true are rejected (`MatrixError::InvalidConfig`), including `\n`, `\r`, `\t`, and `\0`. These would corrupt the render buffer.
- Display width is **not** validated. The builder cannot reliably detect full-width or zero-width characters across terminals; callers must ensure each char occupies exactly one cell. Failure to do so produces visual misalignment, not memory unsafety.
- Single-character custom charsets are allowed — mutation simply re-picks the same glyph.

### 5.5 Themes

```rust
pub enum Theme {
    ClassicGreen,    // canonical green-on-black
    Amber,
    Cyan,
    Red,
    Rainbow,
    Custom(ColorRamp),
}

pub struct ColorRamp {
    pub head: Color,        // brightest cell (often white)
    pub bright: Color,      // first few cells of the trail
    pub mid: Color,
    pub dim: Color,
    pub fade: Color,        // last visible cell before the trail disappears
}
```

### 5.6 Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum MatrixError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("empty character set")]
    EmptyCharset,
}
```

## 6. Rendering Model

### 6.1 Streams
- Each terminal column has at most one active falling drop (a `Stream`) at a time.
- A `Stream` stores: `head_row: f32`, `length: u16`, `speed: f32` (rows per second), and `glyphs: Vec<char>` of length `length`.
- The tail position is `head_row - length + 1`. Once `head_row - length >= area.height` (the entire trail has scrolled off the bottom), the stream is retired and the column goes idle for a randomized cooldown.
- After the cooldown expires, idle columns become active again with probability proportional to `density`.
- Stream length may exceed `area.height`. In that case the head reaches the bottom before the tail has finished spawning, the upper part of the trail is simply clipped, and the stream retires when the *theoretical* tail position passes the bottom — preserving per-column timing regardless of viewport size.

### 6.2 Frame loop (per `render` call)
1. **Empty area:** if `area.width == 0` or `area.height == 0`, clear `streams` (or leave it empty) and return without painting. Do not update `last_tick`; treat the next non-empty render as a first render.
2. **Resize handling:** compare `area` against `last_area`.
   - If `area.width` changed, resize `streams` to the new column count. When growing, new columns start idle with a random cooldown so the field doesn't snap into a wall of new drops. When shrinking, dropped columns and their streams are simply discarded.
   - If `area.height` changed, clamp every active stream's `head_row` to `[0, new_height + max_trail)` so a window made dramatically smaller doesn't leave streams stuck above or below the viewport for long stretches. Streams whose entire trail is now off-screen are retired.
3. **Tick budget:** compute `elapsed = now - last_tick` (treating the first render as one tick) and add `accum`. Convert to a tick count: `total = (elapsed.as_secs_f32() * fps as f32 * speed) + leftover`, then `ticks = total.floor() as u32` capped at `MAX_CATCHUP_TICKS` (default 4) so a process resumed from suspend doesn't try to render hundreds of frames at once. Carry `total - ticks as f32` back into `accum` as a `Duration` so sub-tick renders aren't dropped.
4. For each tick:
   - Advance every active stream's `head_row` by its per-tick speed (derived from `fps`).
   - Apply `mutation_rate` to randomly swap glyphs inside trails. New glyphs are drawn from the configured `CharSet`; each cell rolls independently.
   - Spawn new streams in idle columns whose cooldown has expired, gated by `density`. When `density == 0.0`, the spawn step is a no-op.
5. Update `last_tick` to `Instant::now()` and store `area` in `last_area`. If `ticks == 0` (sub-tick render), `last_tick` is still updated and `accum` carries the remainder.
6. Paint into `buf`:
   - For each active stream, walk from head down to tail.
   - Cell `i` (counting from the head) gets a color from the theme's gradient, indexed by `i / max(length - 1, 1)` (the `max` guards against `length == 1`, in which case the head color is used directly).
   - The head uses `head` if `head_white` is true, otherwise `bright`.
   - Apply `Modifier::BOLD` to the head when `bold_head` is true.
   - Skip cells outside `area` (a head still above row 0 during spawn-in, or trail cells past `area.height`).
   - Honor `area.x` and `area.y` offsets — never write to absolute `(0, 0)`; always write to `(area.x + col, area.y + row)`. Coordinates are computed in `u16` with checked arithmetic so a misconfigured `Rect` near the edge of the buffer cannot wrap.

### 6.3 Color gradient
- Five-stop ramp, interpolated per cell over `[0, length-1]`.
- Color depth is detected once per state via `crossterm::style::available_color_count()` and cached in `color_count`. Truecolor terminals get a smooth gradient; 256-color terminals get a quantized 5-stop ramp; 16-color terminals collapse to 2 or 3 discrete colors.
- If detection fails or returns a value the widget doesn't recognize, the widget falls back to the 16-color path rather than panicking.
- When `background` is `Some(c)` and an interpolated cell would round to `c`, the cell is skipped (a small optimization). When `background` is `None`, cells past the visible fade threshold are skipped and never overwrite whatever is already in `buf`.

## 7. Standalone Binary

### 7.1 CLI

```
matrix [OPTIONS]

OPTIONS:
  -f, --fps <FPS>              Frames per second [default: 30]
  -s, --speed <FLOAT>          Speed multiplier [default: 1.0]
  -d, --density <0..1>         Column density [default: 0.6]
      --charset <NAME>         matrix|ascii|hex|binary|<path-to-file>
      --theme <NAME>           green|amber|cyan|red|rainbow
      --no-head-white          Disable the classic white head
      --no-bold                Disable bold head cells (bold is on by default)
      --seed <U64>             Deterministic RNG seed
  -q, --quit-on-any-key        Exit on any keypress (default: q/Esc/Ctrl-C only)
  -h, --help
  -V, --version
```

When `--charset` is a path:
- The file is read as UTF-8. Invalid UTF-8 produces a clear error and exit code 2.
- Whitespace, control characters, and duplicate code points are filtered out. The remaining list must be non-empty.
- Files larger than 1 MiB are rejected to prevent accidentally piping a binary in.

### 7.2 Behavior
- Refuses to start if stdout is not a TTY (`std::io::IsTerminal::is_terminal()`); prints a one-line message to stderr and exits with code 2. This avoids garbling logs when the binary is run under a pipe, in a CI runner, or as a systemd service by accident.
- Switches to the alternate screen, hides the cursor, and enables raw mode.
- Restores the terminal cleanly on exit, panic, or signal:
  - A `Drop` guard is installed before raw mode is enabled, so normal exit and `?`-bubbling unwind both restore.
  - A `std::panic::set_hook` wrapper additionally emits the leave sequence so a panicking thread does not strand the terminal in raw mode.
  - On Unix, `SIGINT`, `SIGTERM`, and `SIGHUP` set an atomic shutdown flag that the main loop checks each iteration; the guard's `Drop` then runs normally. (Crossterm reports `Ctrl-C` as a key event in raw mode; the signal handler is the fallback for terminals that deliver `SIGTERM` directly, e.g. when the parent shell exits.)
- Polls input with `crossterm::event::poll(Duration::from_millis(d))` where `d = max(1, 1000 / fps)`. Exits on `q`, `Esc`, or `Ctrl-C`. With `--quit-on-any-key`, exits on any `KeyEventKind::Press` (release and repeat events are ignored on platforms that distinguish them, so a held key doesn't fire repeatedly during startup).
- On terminal resize (delivered by crossterm as `Event::Resize`), the next render sees the new `area` and the widget transparently re-allocates streams. A `Resize(0, 0)`, which some terminals emit when the window is minimized, is handled by §6.2 step 1 (no paint, no state advance). SIGWINCH handling is left to crossterm; no direct signal handler is installed.
- On `std::io::ErrorKind::BrokenPipe` from the writer (e.g. the terminal emulator closes), the loop exits via the same shutdown path.

## 8. Embedding Example

```rust
use ratatui::{prelude::*, widgets::Block};
use matrix_rain::{MatrixRain, MatrixRainState, MatrixConfig};

let cfg = MatrixConfig::builder().fps(30).density(0.5).build()?;
let mut state = MatrixRainState::new();

terminal.draw(|f| {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    f.render_stateful_widget(MatrixRain::new(&cfg), chunks[0], &mut state);
    f.render_widget(
        Block::bordered().title("Matrix Demo"),
        chunks[1],
    );
})?;
```

## 9. Performance

- Target: under 1 ms per frame on a 200×60 terminal on a modern CPU.
- Avoid allocating per cell: each `Stream` reuses its `Vec<char>` buffer, sized once to `max_trail`.
- Use `SmallRng` (xoshiro) for per-frame randomness; it can be seeded for reproducible tests.
- Skip painting trail cells whose interpolated color rounds to the configured `background` (or, when `background` is `None`, cells past the visible fade threshold).
- Floating-point `head_row` accumulates rounding error over very long runs. Because retire/respawn cycles reset `head_row` to a small integer, error cannot grow unboundedly; no extra scrubbing is needed.
- Optional `simd` feature for batched glyph mutation (low priority; deferred to 0.4.0).

## 10. Testing Strategy

- **Unit tests:** config validation (one negative test per invariant in §5.3, plus charset validation from §5.4), character-set edge cases, the stream lifecycle (spawn → advance → retire → cooldown → respawn), gradient interpolation at length 1, 2, and N, and color-count fallback when detection returns 0 or 1.
- **Property tests** (`proptest`): random areas — including `width = 0`, `height = 0`, `1×1`, `1×u16::MAX`, and randomly oscillating sizes between frames — combined with random configs should never panic, never write outside `area`, and never produce NaN or infinite values in `head_row`. Resize sequences (grow → shrink → grow) must not leak streams.
- **Snapshot tests** (`insta`): with a fixed seed, fixed area, and `state.tick()` driving frames (not the wall clock), render N frames into a `TestBackend` buffer and compare the textual dump.
- **Edge-case scenarios:**
  - Sub-tick render loop (`fps = 30`, render called every 1 ms): verify that `accum` carries the remainder and frame counts converge to ~30/sec.
  - Long pause (simulate by mocking `Instant`): verify the tick budget is capped and no stutter ensues.
  - `density = 0.0`: no streams ever spawn, no panics, the buffer stays untouched outside `background`.
  - `density = 1.0` with cooldown jitter: no column dies forever.
  - Trail length larger than `area.height`: head reaches bottom and retires correctly.
- **Smoke test:** run an in-memory `TestBackend` for 1000 frames; assert it doesn't panic, that `streams.len() == area.width as usize`, and that live memory stays bounded across resize cycles.

## 11. Documentation

- Crate-level documentation with an animated GIF or asciinema link.
- Rustdoc on every public item, with examples.
- Runnable demos in `examples/` (`cargo run --example standalone`).
- `README.md` with install instructions (`cargo install matrix-rain`), screenshots, and an embedding snippet.
- A "Caveats" section calling out: full-width / combining characters in custom charsets, mixing `tick()` with wall-clock rendering, and the non-TTY refusal in the binary.

## 12. Distribution

- Published to crates.io as `matrix-rain` (the bare `matrix` name is already taken). The standalone binary inside that crate is named `matrix` via `[[bin]] name = "matrix"`. Users install with `cargo install matrix-rain` and run `matrix`.
- Pre-built binaries via GitHub Releases (Linux x86_64/aarch64, macOS, Windows), built with `cargo-dist`.
- Optional Homebrew tap and AUR package.

## 13. Roadmap

| Milestone | Scope |
|-----------|-------|
| 0.1.0     | Core widget, classic green theme, standalone binary, crossterm only |
| 0.2.0     | Themes, custom character sets, config builder, snapshot tests |
| 0.3.0     | Glitch mode, white-head toggle, mutation-rate tuning |
| 0.4.0     | Performance pass, optional `simd` feature |
| 1.0.0     | Stable API, full documentation, multi-backend feature flags |

## 14. Edge Cases & Failure Modes

A consolidated reference for unusual inputs and environments. Each row links to where the behavior is enforced.

| Scenario | Behavior | Where |
|---|---|---|
| Area is 0×N, N×0, or 0×0 | Render is a no-op; state advance is skipped; next non-empty render is treated as first | §6.2 step 1 |
| Area shrinks below current stream count | Surplus columns are dropped; clipped streams retire | §6.2 step 2 |
| Area grows | New columns start idle with random cooldown; field ramps up over a few seconds | §6.2 step 2 |
| Process suspended (Ctrl-Z) and resumed | Tick budget capped at `MAX_CATCHUP_TICKS`; animation resumes without a sprint | §6.2 step 3 |
| Render loop runs faster than `fps` | Sub-tick remainder accumulates in `accum`; frame rate naturally caps | §5.2, §6.2 step 3, step 5 |
| Mixing `tick()` with wall-clock render | Both modes work; mixing produces drift. Documented as not recommended | §5.2 |
| `density = 0.0` | No spawns ever; static, empty field | §5.3, §6.2 step 4 |
| `min_trail = max_trail = 1` | Single-cell streams; gradient uses head color directly | §5.3, §6.2 step 6 |
| `mutation_rate = 1.0` | Every cell rerolls every frame; visually noisy but valid | §5.3, §6.2 step 4 |
| Trail longer than viewport | Head reaches bottom before tail finishes spawning; retire timing preserved | §6.1 |
| `CharSet::Custom(vec![])` | Builder rejects with `EmptyCharset` | §5.3, §5.4 |
| Custom charset contains `\n`, `\t`, etc. | Builder rejects with `InvalidConfig` | §5.4 |
| Custom charset contains full-width or combining chars | Not detected; visual misalignment, no crash | §2, §5.4 |
| Charset file > 1 MiB or invalid UTF-8 | Binary exits with code 2 and a clear message | §7.1 |
| Stdout is not a TTY | Binary refuses, exits with code 2 | §7.2 |
| Terminal closed while running | Broken-pipe writes trigger graceful shutdown via `Drop` guard | §7.2 |
| Panic in embedder thread holding state | Panic hook restores terminal; state is dropped normally | §7.2 |
| `Event::Resize(0, 0)` (window minimized) | Treated as empty area; no paint, no advance | §6.2 step 1, §7.2 |
| 16-color or unknown terminal | Gradient collapses to 2–3 stops; detection failure falls back to 16-color | §6.3 |
| State shared across threads | Not supported; `MatrixRainState: !Sync` | §5.2 |
| Two areas rendered with the same state in one frame | Treated as a resize per call; visible jitter. Embedder should use two states | §5.1 |

## 15. Open Questions

- Should the widget own its own `Instant`-based clock, or accept an external `tick()` driver for apps that already have a frame loop? (Currently both: wall-clock by default, `state.tick()` for manual driving — but the two modes can drift if mixed in the same session, as noted in §5.2.)
- How aggressively should the gradient collapse on 16-color terminals — fixed three steps, or a `palette_mode` enum the caller can pick from?
- Should there be an explicit `pause`/`freeze` API? Skipping `render` calls produces a large `elapsed` on resume; the catch-up cap absorbs it but the visual stutter is still apparent during the first second after a long pause. A built-in `pause` would suppress that and let `accum` reset cleanly.
- Should `MAX_CATCHUP_TICKS` and `MAX_TRAIL_LIMIT` be configurable, or kept as crate constants?
- Final crate name on crates.io: `matrix-rain` (current pick), `tui-matrix`, or `ratatui-matrix`?
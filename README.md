# matrix-rain

The classic Matrix digital rain effect — glowing columns of characters falling from the top of the terminal — packaged as both a [ratatui](https://ratatui.rs/) `StatefulWidget` and a standalone `matrix` binary.

- Smooth animation at 30–60 FPS, configurable density / speed / trail length.
- Five built-in themes (classic green, amber, cyan, red, rainbow) and five charsets (matrix, ascii, hex, binary, or a UTF-8 file you supply).
- Three rendering tiers — smooth RGB interpolation on truecolor terminals, quantized 5-stop ramp on 256-color, nearest-named collapse on 16-color — automatically detected and cached per state.
- Drop-in widget that respects `area.x` / `area.y` offsets and handles resize, suspend/resume, broken-pipe shutdown, and panic-during-raw-mode without stranding the terminal.

## Install

```bash
cargo install matrix-rain
```

Or from a local checkout:

```bash
cargo install --path .
```

The package is `matrix-rain`; the installed binary is `matrix`.

## Run

```bash
matrix                          # classic green, 30 fps
matrix --theme rainbow          # red → yellow → green → blue trails
matrix --charset binary         # 0s and 1s
matrix --fps 60 --density 0.8   # snappier and denser
matrix --seed 42                # deterministic output (handy for screenshots)
matrix --help                   # full reference
```

Quit with `q`, `Esc`, or `Ctrl-C` (or use `--quit-on-any-key`). Press `p` to pause / resume the animation.

## CLI options

| Flag | Default | Description |
|---|---|---|
| `-f, --fps <FPS>` | `30` | Frames per second (must be ≥ 1) |
| `-s, --speed <FLOAT>` | `1.0` | Global speed multiplier (> 0, finite) |
| `-d, --density <0..1>` | `0.6` | Fraction of columns kept active |
| `--charset <NAME\|PATH>` | `matrix` | `matrix`, `ascii`, `hex`, `binary`, or a path to a UTF-8 charset file (≤ 1 MiB) |
| `--theme <NAME>` | `green` | `green`, `amber`, `cyan`, `red`, `rainbow` |
| `--no-head-white` | off | Disable the classic white head; use the theme's "bright" color instead |
| `--no-bold` | off | Disable bold rendering on the head cell |
| `--seed <U64>` | random | Deterministic RNG seed (system entropy by default) |
| `-q, --quit-on-any-key` | off | Exit on any keypress instead of just q/Esc/Ctrl-C |

## Library usage

Add to `Cargo.toml`:

```toml
[dependencies]
matrix-rain = { version = "0.1", default-features = false }
ratatui = "0.26"
```

(Disabling default features drops `clap`, `anyhow`, and `signal-hook`, which are only used by the standalone binary.)

Embed the widget inside a larger ratatui layout:

```rust
use ratatui::prelude::*;
use ratatui::widgets::Block;
use matrix_rain::{MatrixRain, MatrixRainState, MatrixConfig};

let cfg = MatrixConfig::builder()
    .fps(30)
    .density(0.5)
    .build()?;
let mut state = MatrixRainState::new();

terminal.draw(|f| {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)])
        .split(f.size());
    f.render_stateful_widget(MatrixRain::new(&cfg), chunks[0], &mut state);
    f.render_widget(Block::bordered().title("Demo"), chunks[1]);
})?;
```

Runnable examples in `examples/`:

- `cargo run --example standalone` — full-screen demo
- `cargo run --example embedded` — widget inside a larger layout
- `cargo run --example custom_charset` — `CharSet::Custom` with hand-picked glyphs

The widget is intentionally `StatefulWidget`-only. Animation needs state that persists across frames (`MatrixRainState` holds per-column streams, RNG, timing, and a cached color count); a stateless wrapper would either reset every frame or hide global mutable state behind the API.

## Built-in character sets

| `CharSet` | Contents |
|---|---|
| `Matrix` (default) | Half-width katakana `U+FF66..=U+FF9D` + digits 0-9 (66 glyphs) |
| `Ascii` | Printable ASCII `0x21..=0x7E` (94 glyphs, space excluded) |
| `Hex` | `0-9 a-f` (lowercase) |
| `Binary` | `0` and `1` |
| `Custom(Vec<char>)` | User-supplied glyphs. Builder rejects empty / control chars. |

## Built-in themes

| `Theme` | Look |
|---|---|
| `ClassicGreen` (default) | White head over green trail (`0xCCFFCC → 0x00FF00 → 0x009900 → 0x003300`) |
| `Amber` | Phosphor amber |
| `Cyan` | Cyan glow |
| `Red` | Hostile red |
| `Rainbow` | White head → red → yellow → green → blue across the trail |
| `Custom(ColorRamp)` | Hand-built 5-stop ramp |

## Pause / resume

```rust
state.pause();   // freezes wall-clock-driven advance; render still paints
state.resume();  // next render is treated as a first render (no catch-up stutter)
state.is_paused();
```

`tick()` bypasses pause — manual driving always advances. `reset()` clears the paused flag. The binary toggles pause on the `p` key.

## Determinism

Pass `--seed <U64>` (or use `MatrixRainState::with_seed`) to get a reproducible animation. The library's snapshot test suite drives frames purely via `state.tick()` so output is identical across runs given the same seed, area, and config.

`MatrixRainState::set_color_count(count)` overrides the cached terminal color count, letting you force a specific rendering tier (e.g. `16` for accessibility, `u16::MAX` to force smooth interpolation, or any value in tests where `TERM`/`COLORTERM` shouldn't matter).

## Caveats

- **Full-width and combining characters in custom charsets are not detected.** Each glyph must occupy exactly one terminal cell or the column layout misaligns. CJK ideographs, emoji with variation selectors, and zero-width combiners are all single `char`s in Rust but multi-cell in the terminal. Display width can't reliably be detected across terminals; verifying single-cell-ness is the caller's responsibility.
- **Mixing `MatrixRainState::tick()` with wall-clock rendering** produces visible drift over time. Tick driving is exact (each call advances exactly one frame); wall-clock driving advances based on elapsed `Instant`s. Pick one mode per session.
- **Non-TTY refusal.** The binary exits with code 2 when stdout isn't a terminal so it doesn't garble logs when accidentally run under a pipe, in a CI runner, or as a systemd service. `--help` and `--version` still work in non-TTY contexts (they run before the check).

## License

MIT. See `LICENSE` (or `Cargo.toml`'s `license` field) for details.

//! Classic "Matrix digital rain" effect for terminals, packaged as both a
//! [ratatui](https://ratatui.rs/) [`StatefulWidget`](ratatui::widgets::StatefulWidget)
//! library and a standalone `matrix` binary.
//!
//! The crate is published as `matrix-rain` on crates.io because the bare `matrix`
//! name is taken; the installed binary is still called `matrix`.
//!
//! # Quick start
//!
//! Drop the widget into any ratatui layout. `MatrixRainState` carries the
//! per-frame animation state (column streams, RNG, timing, cached color tier)
//! across renders.
//!
//! ```
//! use matrix_rain::{MatrixConfig, MatrixRain, MatrixRainState};
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::Rect;
//! use ratatui::widgets::StatefulWidget;
//!
//! let cfg = MatrixConfig::builder().fps(30).density(0.5).build().unwrap();
//! let mut state = MatrixRainState::with_seed(42);
//! let area = Rect::new(0, 0, 80, 24);
//! let mut buf = Buffer::empty(area);
//!
//! MatrixRain::new(&cfg).render(area, &mut buf, &mut state);
//! assert_eq!(state.streams_len(), 80);
//! ```
//!
//! For a full embed inside a [`ratatui::Terminal`] event loop, see
//! `examples/embedded.rs` in the source repo. For the standalone full-screen
//! demo, see `examples/standalone.rs`.
//!
//! # Driving frames
//!
//! There are two ways to advance the animation:
//!
//! - **Wall-clock (default).** Each call to
//!   [`MatrixRain::render`](ratatui::widgets::StatefulWidget::render) reads
//!   `Instant::now()` internally and applies as many ticks as the elapsed time
//!   buys (capped at `MAX_CATCHUP_TICKS=4` so a process resumed from suspend
//!   doesn't render hundreds of frames at once). This is what `terminal.draw(…)`
//!   does naturally and what the bundled binary uses.
//! - **Manual via [`MatrixRainState::tick`].** Each call advances exactly one
//!   frame regardless of wall-clock time. Useful for deterministic snapshot
//!   tests and external tick-loop apps.
//!
//! Mixing both modes in the same session produces visible drift; the snapshot
//! suite suppresses wall-clock advance by setting `fps=1` together with a tiny
//! `speed` (e.g. `0.001`) so the elapsed-time conversion floors to zero ticks
//! per render. See [`MatrixRainState::set_color_count`] if you also need to
//! lock the rendering tier for reproducibility.
//!
//! # Color tiers
//!
//! Color depth is detected once per state on the first render via
//! `crossterm::style::available_color_count` (supplemented by sniffing
//! `COLORTERM=truecolor|24bit`, which crossterm 0.27 itself doesn't surface).
//! The result is cached on the state and drives one of three rendering paths:
//!
//! - **Truecolor**: linear RGB interpolation between the 5 stops in
//!   [`ColorRamp`]. Smoothest gradient.
//! - **256-color**: nearest-of-5-stops; the terminal handles any further
//!   RGB→256 quantization.
//! - **16-color**: 3-zone collapse (head, then `bright`/`mid`/`fade` zones)
//!   with each stop mapped to the nearest of the 16 named [`Color`] variants
//!   by euclidean RGB distance. Detection failure or any value the widget
//!   doesn't recognize also falls back to this path rather than panicking.
//!
//! Force a specific tier with [`MatrixRainState::set_color_count`]: pass `16`
//! for accessibility, `256` for the quantized middle tier, or `u16::MAX` for
//! the smooth-interpolation path.
//!
//! [`Color`]: ratatui::style::Color
//!
//! # Caveats
//!
//! - **Full-width and combining characters in [`CharSet::Custom`] are not
//!   detected.** Each glyph must occupy exactly one terminal cell or the
//!   column layout misaligns. CJK ideographs, emoji with variation selectors,
//!   and zero-width combiners are all single `char`s in Rust but multi-cell
//!   in the terminal. Display width cannot reliably be detected across
//!   terminals; verifying single-cell-ness is the caller's responsibility.
//! - **Mixing [`MatrixRainState::tick`] with wall-clock rendering** produces
//!   visible drift over time. Tick driving is exact (each call advances
//!   exactly one frame); wall-clock driving advances based on elapsed
//!   [`Instant`](std::time::Instant)s. Pick one mode per session.
//! - **16-color fallback is a 3-zone collapse**, not the original 5-stop
//!   gradient. If your theme has stops that map to the same named color
//!   (common with monochrome themes on 16-color), zones will visually merge.
//!   Use [`MatrixRainState::set_color_count`] to force a higher tier if your
//!   terminal actually supports it, or supply a [`Theme::Custom`] ramp whose
//!   stops are already in the named-color palette.
//! - **Non-TTY refusal (binary only).** The standalone `matrix` binary exits
//!   with code 2 when stdout isn't a terminal so it doesn't garble logs when
//!   accidentally run under a pipe, in a CI runner, or as a systemd service.
//!   `--help` and `--version` still work in non-TTY contexts (they run before
//!   the check).

#![warn(missing_docs)]

mod charset;
mod config;
mod error;
mod state;
mod stream;
mod theme;
mod widget;

pub use charset::CharSet;
pub use config::{MatrixConfig, MatrixConfigBuilder, MAX_TRAIL_LIMIT};
pub use error::MatrixError;
pub use state::MatrixRainState;
pub use theme::{ColorRamp, Theme};
pub use widget::MatrixRain;

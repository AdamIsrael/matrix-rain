//! The [`MatrixRain`] widget — the public ratatui surface.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::StatefulWidget;

use crate::config::MatrixConfig;
use crate::state::MatrixRainState;
use crate::stream::Stream;
use crate::theme::ColorRamp;

#[cfg_attr(not(feature = "std"), allow(dead_code))]
const TRUECOLOR_SENTINEL: u16 = u16::MAX;

/// The ratatui widget rendering the Matrix digital rain effect.
///
/// `MatrixRain` borrows the [`MatrixConfig`] for the lifetime of the render
/// call and consumes itself when rendered (per the
/// [`StatefulWidget`](ratatui::widgets::StatefulWidget) contract). It is
/// intentionally **stateful-only** — there is no plain `Widget` implementation.
/// Animation requires per-frame state (column streams, RNG, timing, cached
/// color tier) that a stateless wrapper would either reset every frame or
/// hide behind surprising global mutable state.
///
/// # Example
///
/// ```
/// use matrix_rain::{MatrixConfig, MatrixRain, MatrixRainState};
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use ratatui::widgets::StatefulWidget;
///
/// let cfg = MatrixConfig::default();
/// let mut state = MatrixRainState::with_seed(0xC0FFEE);
/// let area = Rect::new(0, 0, 40, 12);
/// let mut buf = Buffer::empty(area);
///
/// // The widget is constructed once per frame and consumed by render().
/// MatrixRain::new(&cfg).render(area, &mut buf, &mut state);
/// ```
///
/// Inside a ratatui `Terminal::draw` closure:
///
/// ```ignore
/// terminal.draw(|f| {
///     f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
/// })?;
/// ```
pub struct MatrixRain<'a> {
    config: &'a MatrixConfig,
}

impl<'a> MatrixRain<'a> {
    /// Create a new `MatrixRain` widget bound to the given configuration.
    /// The config is borrowed for the lifetime of the widget; build it once
    /// outside the render loop and pass `&cfg` here each frame.
    pub fn new(config: &'a MatrixConfig) -> Self {
        Self { config }
    }
}

impl<'a> StatefulWidget for MatrixRain<'a> {
    type State = MatrixRainState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.advance(area, self.config);
        if area.width == 0 || area.height == 0 {
            return;
        }

        if state.color_count().is_none() {
            state.set_color_count(detect_color_count());
        }
        let tier = Tier::from_count(state.color_count().unwrap_or(8));

        let ramp = self.config.theme.ramp();
        let head_white = self.config.head_white;
        let bold_head = self.config.bold_head;
        let background = self.config.background;

        for (col, stream) in state.streams().iter().enumerate() {
            if !stream.is_active() {
                continue;
            }
            paint_stream(
                stream, area, buf, &ramp, head_white, bold_head, background, tier, col as u16,
            );
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    Truecolor,
    Color256,
    Color16,
}

impl Tier {
    fn from_count(count: u16) -> Self {
        if count > 256 {
            Tier::Truecolor
        } else if count == 256 {
            Tier::Color256
        } else {
            Tier::Color16
        }
    }
}

#[cfg(feature = "std")]
fn detect_color_count() -> u16 {
    // COLORTERM=truecolor|24bit is the de-facto standard for advertising
    // 24-bit color support (alacritty, iTerm2, kitty, recent xterm, etc.).
    // crossterm 0.28's available_color_count doesn't surface this, so we
    // check it directly here. Signalled to the renderer via TRUECOLOR_SENTINEL.
    let truecolor = std::env::var("COLORTERM")
        .map(|v| matches!(v.trim(), "truecolor" | "24bit"))
        .unwrap_or(false);
    if truecolor {
        return TRUECOLOR_SENTINEL;
    }
    // Best-effort TERM env sniff (the same logic crossterm uses). Returns 256
    // when TERM mentions 256color, 8 otherwise. Detection failure / unrecognized
    // values fall through to the 16-color path per spec §6.3 — Tier::from_count
    // collapses anything < 256 to Tier::Color16.
    std::env::var("TERM")
        .map(|t| if t.contains("256color") { 256u16 } else { 8 })
        .unwrap_or(8)
}

// no_std: no environment to sniff. Fall through to the 16-color path so
// painting is always safe; embedded callers should call
// `MatrixRainState::set_color_count` once at startup to pick a higher tier
// if the display supports one.
#[cfg(not(feature = "std"))]
fn detect_color_count() -> u16 {
    8
}

fn paint_stream(
    stream: &Stream,
    area: Rect,
    buf: &mut Buffer,
    ramp: &ColorRamp,
    head_white: bool,
    bold_head: bool,
    background: Option<Color>,
    tier: Tier,
    col: u16,
) {
    // `head_row` is invariantly >= 0 (clamped in handle_resize, only ever
    // incremented by positive speed/fps in stream.tick), so truncation
    // toward zero via `as i32` produces the same result as `floor()` —
    // and works in no_std where the `floor` method is unavailable.
    let head_int = stream.head_row() as i32;
    let length = stream.length();
    let glyphs = stream.glyphs();
    let buf_area = buf.area;

    for i in 0..length {
        let screen_row_i = head_int - i as i32;
        if screen_row_i < 0 || screen_row_i >= area.height as i32 {
            continue;
        }
        let screen_row = screen_row_i as u16;

        let Some(glyph) = glyphs.get(i as usize).copied() else {
            continue;
        };

        let mut color = pick_color(ramp, head_white, i, length, tier);
        if i > 0 && stream.is_glitched(i) {
            color = ramp.head;
        }

        if should_skip(i, length, color, ramp.fade, background) {
            continue;
        }

        let Some(x) = area.x.checked_add(col) else {
            continue;
        };
        let Some(y) = area.y.checked_add(screen_row) else {
            continue;
        };

        let buf_max_x = buf_area.x.saturating_add(buf_area.width);
        let buf_max_y = buf_area.y.saturating_add(buf_area.height);
        if x < buf_area.x || x >= buf_max_x || y < buf_area.y || y >= buf_max_y {
            continue;
        }

        let mut style = Style::default().fg(color);
        if i == 0 && bold_head {
            style = style.add_modifier(Modifier::BOLD);
        }

        let cell = &mut buf[(x, y)];
        cell.set_char(glyph);
        cell.set_style(style);
    }
}

fn pick_color(ramp: &ColorRamp, head_white: bool, i: u16, length: u16, tier: Tier) -> Color {
    if i == 0 {
        return if head_white { ramp.head } else { ramp.bright };
    }
    let denom = length.saturating_sub(1).max(1);
    let t = (i as f32) / (denom as f32);

    match tier {
        Tier::Truecolor => interpolate_smooth(ramp, t),
        Tier::Color256 => pick_nearest_stop(ramp, t),
        Tier::Color16 => pick_named_zone(ramp, t),
    }
}

fn pick_nearest_stop(ramp: &ColorRamp, t: f32) -> Color {
    let stops = [ramp.head, ramp.bright, ramp.mid, ramp.dim, ramp.fade];
    // `t * 4.0` is invariantly >= 0; round-half-up via `+ 0.5` then truncate.
    let idx = ((t * 4.0 + 0.5) as usize).min(4);
    stops[idx]
}

fn interpolate_smooth(ramp: &ColorRamp, t: f32) -> Color {
    let stops = [ramp.head, ramp.bright, ramp.mid, ramp.dim, ramp.fade];
    let scaled = (t.clamp(0.0, 1.0)) * 4.0;
    // `scaled` is in [0.0, 4.0]; truncation toward zero == floor.
    let lo = (scaled as usize).min(4);
    let hi = (lo + 1).min(4);
    let local = scaled - lo as f32;
    let (lr, lg, lb) = to_rgb(stops[lo]);
    let (hr, hg, hb) = to_rgb(stops[hi]);
    // Each blended channel is in [0.0, 255.0]; `+ 0.5` then truncate == round.
    let r = ((1.0 - local) * lr as f32 + local * hr as f32 + 0.5) as u8;
    let g = ((1.0 - local) * lg as f32 + local * hg as f32 + 0.5) as u8;
    let b = ((1.0 - local) * lb as f32 + local * hb as f32 + 0.5) as u8;
    Color::Rgb(r, g, b)
}

fn pick_named_zone(ramp: &ColorRamp, t: f32) -> Color {
    let stop = if t < 0.34 {
        ramp.bright
    } else if t < 0.67 {
        ramp.mid
    } else {
        ramp.fade
    };
    nearest_named(stop)
}

fn should_skip(i: u16, length: u16, color: Color, fade: Color, background: Option<Color>) -> bool {
    if let Some(bg) = background {
        return color == bg;
    }
    if i == 0 {
        return false;
    }
    if color == fade {
        return true;
    }
    let denom = length.saturating_sub(1).max(1);
    let t = (i as f32) / (denom as f32);
    t >= 0.875
}

fn to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (128, 0, 0),
        Color::Green => (0, 128, 0),
        Color::Yellow => (128, 128, 0),
        Color::Blue => (0, 0, 128),
        Color::Magenta => (128, 0, 128),
        Color::Cyan => (0, 128, 128),
        Color::Gray => (192, 192, 192),
        Color::DarkGray => (128, 128, 128),
        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (0, 0, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),
        Color::Indexed(_) | Color::Reset => (255, 255, 255),
    }
}

const NAMED_PALETTE: &[(Color, (u8, u8, u8))] = &[
    (Color::Black, (0, 0, 0)),
    (Color::Red, (128, 0, 0)),
    (Color::Green, (0, 128, 0)),
    (Color::Yellow, (128, 128, 0)),
    (Color::Blue, (0, 0, 128)),
    (Color::Magenta, (128, 0, 128)),
    (Color::Cyan, (0, 128, 128)),
    (Color::Gray, (192, 192, 192)),
    (Color::DarkGray, (128, 128, 128)),
    (Color::LightRed, (255, 0, 0)),
    (Color::LightGreen, (0, 255, 0)),
    (Color::LightYellow, (255, 255, 0)),
    (Color::LightBlue, (0, 0, 255)),
    (Color::LightMagenta, (255, 0, 255)),
    (Color::LightCyan, (0, 255, 255)),
    (Color::White, (255, 255, 255)),
];

fn nearest_named(target: Color) -> Color {
    let (tr, tg, tb) = to_rgb(target);
    let mut best = NAMED_PALETTE[0].0;
    let mut best_dist = u32::MAX;
    for &(named, (nr, ng, nb)) in NAMED_PALETTE {
        let dr = (tr as i32 - nr as i32).unsigned_abs();
        let dg = (tg as i32 - ng as i32).unsigned_abs();
        let db = (tb as i32 - nb as i32).unsigned_abs();
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best = named;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fully_active_config(seed_density: f32) -> MatrixConfig {
        MatrixConfig {
            density: seed_density,
            ..MatrixConfig::default()
        }
    }

    fn classic_ramp() -> ColorRamp {
        ColorRamp {
            head: Color::Rgb(0xFF, 0xFF, 0xFF),
            bright: Color::Rgb(0xCC, 0xFF, 0xCC),
            mid: Color::Rgb(0x00, 0xFF, 0x00),
            dim: Color::Rgb(0x00, 0x99, 0x00),
            fade: Color::Rgb(0x00, 0x33, 0x00),
        }
    }

    #[test]
    fn render_with_zero_width_area_is_noop() {
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(0);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, 0, 10), &mut buf, &mut state);
    }

    #[test]
    fn render_with_zero_height_area_is_noop() {
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(0);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, 10, 0), &mut buf, &mut state);
    }

    #[test]
    fn does_not_paint_outside_widget_area() {
        let cfg = fully_active_config(1.0);
        let mut state = MatrixRainState::with_seed(42);
        let buf_area = Rect::new(0, 0, 20, 20);
        let mut buf = Buffer::empty(buf_area);
        for y in 0..20 {
            for x in 0..20 {
                buf[(x, y)].set_char('#');
            }
        }
        let widget_area = Rect::new(5, 5, 10, 10);
        for _ in 0..50 {
            MatrixRain::new(&cfg).render(widget_area, &mut buf, &mut state);
            state.tick();
        }
        for y in 0..20 {
            for x in 0..20 {
                let inside = (5..15).contains(&x) && (5..15).contains(&y);
                if !inside {
                    assert_eq!(
                        buf[(x, y)].symbol(),
                        "#",
                        "cell ({x},{y}) outside widget area was modified"
                    );
                }
            }
        }
    }

    #[test]
    fn paints_at_least_some_cells_with_high_density() {
        let cfg = fully_active_config(1.0);
        let mut state = MatrixRainState::with_seed(42);
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 20));
        let widget_area = Rect::new(0, 0, 20, 20);
        for _ in 0..120 {
            MatrixRain::new(&cfg).render(widget_area, &mut buf, &mut state);
            state.tick();
        }
        let mut painted = 0;
        for y in 0..20 {
            for x in 0..20 {
                let sym = buf[(x, y)].symbol();
                if !sym.is_empty() && sym != " " {
                    painted += 1;
                }
            }
        }
        assert!(painted > 0, "expected some cells to be painted");
    }

    #[test]
    fn honors_non_zero_origin() {
        let cfg = fully_active_config(1.0);
        let mut state = MatrixRainState::with_seed(42);
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 30));
        let widget_area = Rect::new(7, 11, 8, 8);
        for _ in 0..120 {
            MatrixRain::new(&cfg).render(widget_area, &mut buf, &mut state);
            state.tick();
        }
        let mut painted_inside = 0;
        let mut painted_outside = 0;
        for y in 0..30 {
            for x in 0..30 {
                let sym = buf[(x, y)].symbol();
                if !sym.is_empty() && sym != " " {
                    let inside = (7..15).contains(&x) && (11..19).contains(&y);
                    if inside {
                        painted_inside += 1;
                    } else {
                        painted_outside += 1;
                    }
                }
            }
        }
        assert!(painted_inside > 0, "no cells painted inside offset area");
        assert_eq!(painted_outside, 0, "cells painted outside offset area");
    }

    #[test]
    fn resize_between_renders_does_not_panic() {
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(42);
        let sizes = [(20u16, 20u16), (5, 30), (40, 5), (1, 1), (0, 10), (15, 15)];
        for (w, h) in sizes {
            let mut buf = Buffer::empty(Rect::new(0, 0, w.max(1), h.max(1)));
            MatrixRain::new(&cfg).render(Rect::new(0, 0, w, h), &mut buf, &mut state);
        }
    }

    #[test]
    fn tier_from_count_buckets() {
        assert_eq!(Tier::from_count(8), Tier::Color16);
        assert_eq!(Tier::from_count(15), Tier::Color16);
        assert_eq!(Tier::from_count(16), Tier::Color16);
        assert_eq!(Tier::from_count(255), Tier::Color16);
        assert_eq!(Tier::from_count(256), Tier::Color256);
        assert_eq!(Tier::from_count(257), Tier::Truecolor);
        assert_eq!(Tier::from_count(u16::MAX), Tier::Truecolor);
    }

    #[test]
    fn nearest_stop_endpoints() {
        let r = classic_ramp();
        assert_eq!(pick_nearest_stop(&r, 0.0), r.head);
        assert_eq!(pick_nearest_stop(&r, 1.0), r.fade);
        assert_eq!(pick_nearest_stop(&r, 0.5), r.mid);
    }

    #[test]
    fn smooth_interpolation_endpoints_match_stops() {
        let r = classic_ramp();
        assert_eq!(interpolate_smooth(&r, 0.0), r.head);
        assert_eq!(interpolate_smooth(&r, 1.0), r.fade);
        assert_eq!(interpolate_smooth(&r, 0.25), r.bright);
        assert_eq!(interpolate_smooth(&r, 0.5), r.mid);
        assert_eq!(interpolate_smooth(&r, 0.75), r.dim);
    }

    #[test]
    fn smooth_interpolation_midpoint_is_between_stops() {
        let r = classic_ramp();
        // t=0.125 sits between head (white) and bright (pale green).
        match interpolate_smooth(&r, 0.125) {
            Color::Rgb(rr, gg, bb) => {
                // Should be between head (255,255,255) and bright (204,255,204).
                assert!(rr > 204 && rr < 255, "r out of range: {rr}");
                assert_eq!(gg, 255);
                assert!(bb > 204 && bb < 255, "b out of range: {bb}");
            }
            _ => panic!("expected Rgb"),
        }
    }

    #[test]
    fn named_zone_collapses_to_named_colors() {
        let r = classic_ramp();
        // bright zone (early trail): 0xCCFFCC is closest to LightGreen (0,255,0)... actually it's pale green.
        let early = pick_named_zone(&r, 0.1);
        let mid = pick_named_zone(&r, 0.5);
        let late = pick_named_zone(&r, 0.9);
        // All should be one of the 16 named variants (no Rgb).
        for c in [early, mid, late] {
            assert!(
                !matches!(c, Color::Rgb(..) | Color::Indexed(..)),
                "Color16 path returned non-named color: {c:?}"
            );
        }
    }

    #[test]
    fn nearest_named_white_for_white_input() {
        assert_eq!(nearest_named(Color::Rgb(0xFF, 0xFF, 0xFF)), Color::White);
        assert_eq!(nearest_named(Color::Rgb(0x00, 0x00, 0x00)), Color::Black);
        assert_eq!(nearest_named(Color::Rgb(0x00, 0xFF, 0x00)), Color::LightGreen);
    }

    #[test]
    fn pick_color_head_respects_head_white() {
        let r = classic_ramp();
        for tier in [Tier::Truecolor, Tier::Color256, Tier::Color16] {
            assert_eq!(pick_color(&r, true, 0, 10, tier), r.head);
            assert_eq!(pick_color(&r, false, 0, 10, tier), r.bright);
        }
    }

    #[test]
    fn skip_when_color_matches_background() {
        let r = classic_ramp();
        assert!(should_skip(3, 10, Color::Black, r.fade, Some(Color::Black)));
        assert!(!should_skip(3, 10, Color::Green, r.fade, Some(Color::Black)));
    }

    #[test]
    fn skip_fade_zone_when_background_none() {
        let r = classic_ramp();
        // Tail cell (i=length-1, t=1.0) should be skipped when background is None.
        assert!(should_skip(9, 10, r.fade, r.fade, None));
        // Head cell never skipped.
        assert!(!should_skip(0, 10, r.head, r.fade, None));
        // Middle cell not skipped.
        assert!(!should_skip(4, 10, r.mid, r.fade, None));
    }

    #[test]
    fn detection_caches_into_state_after_first_render() {
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(0);
        assert!(state.color_count().is_none());
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 5));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, 5, 5), &mut buf, &mut state);
        assert!(state.color_count().is_some());
    }

    #[test]
    fn detection_does_not_overwrite_pre_set_count() {
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(0);
        state.set_color_count(42);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 5));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, 5, 5), &mut buf, &mut state);
        assert_eq!(state.color_count(), Some(42));
    }

    #[test]
    fn renders_under_each_tier_without_panic() {
        let cfg = fully_active_config(1.0);
        for forced in [16u16, 256, TRUECOLOR_SENTINEL] {
            let mut state = MatrixRainState::with_seed(0xBEEF);
            state.set_color_count(forced);
            let mut buf = Buffer::empty(Rect::new(0, 0, 20, 10));
            for _ in 0..30 {
                MatrixRain::new(&cfg).render(Rect::new(0, 0, 20, 10), &mut buf, &mut state);
                state.tick();
            }
        }
    }
}

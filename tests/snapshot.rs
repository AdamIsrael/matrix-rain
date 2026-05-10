//! Deterministic snapshot tests for the Matrix rain widget.
//!
//! Determinism strategy (spec §10): fixed seed + state.tick() driving frames.
//! To suppress render()'s wall-clock advance, we configure fps=1 with a very
//! small global speed, so the elapsed-time conversion floors to 0 ticks per
//! render call. The only source of state advance becomes the explicit
//! state.tick() between renders. set_color_count() locks the rendering tier
//! so TERM/COLORTERM in the test environment cannot drift the output.

use insta::assert_snapshot;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

use matrix_rain::{MatrixConfig, MatrixRain, MatrixRainState};

fn buf_to_string(buf: &Buffer) -> String {
    let area = buf.area;
    let mut s = String::new();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let sym = buf.get(x, y).symbol();
            if sym.is_empty() {
                s.push(' ');
            } else {
                s.push_str(sym);
            }
        }
        s.push('\n');
    }
    s
}

fn render_frames(seed: u64, area: Rect, frames: usize, color_count: u16) -> String {
    let cfg = MatrixConfig::builder()
        .fps(1)
        .speed(0.001)
        .density(0.8)
        .min_trail(4)
        .max_trail(8)
        .build()
        .unwrap();
    let mut state = MatrixRainState::with_seed(seed);
    state.set_color_count(color_count);
    let mut buf = Buffer::empty(area);
    for i in 0..frames {
        MatrixRain::new(&cfg).render(area, &mut buf, &mut state);
        if i + 1 < frames {
            state.tick();
        }
    }
    buf_to_string(&buf)
}

#[test]
fn default_rain_30_frames_256_color() {
    let s = render_frames(0xC0FFEE, Rect::new(0, 0, 30, 14), 30, 256);
    assert_snapshot!(s);
}

#[test]
fn default_rain_30_frames_16_color() {
    let s = render_frames(0xC0FFEE, Rect::new(0, 0, 30, 14), 30, 16);
    assert_snapshot!(s);
}

#[test]
fn default_rain_30_frames_truecolor() {
    let s = render_frames(0xC0FFEE, Rect::new(0, 0, 30, 14), 30, u16::MAX);
    assert_snapshot!(s);
}

#[test]
fn single_column_30_frames() {
    let s = render_frames(0xC0FFEE, Rect::new(0, 0, 1, 20), 30, 256);
    assert_snapshot!(s);
}

#[test]
fn single_row_30_frames() {
    let s = render_frames(0xC0FFEE, Rect::new(0, 0, 30, 1), 30, 256);
    assert_snapshot!(s);
}

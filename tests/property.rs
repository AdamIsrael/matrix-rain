//! Property tests for the Matrix rain widget (spec §10).
//!
//! Random areas × random configs must never panic, never write outside the
//! widget area, and resize sequences must not corrupt the per-column stream
//! invariant.

use proptest::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

use matrix_rain::{MatrixConfig, MatrixRain, MatrixRainState};

fn valid_config() -> impl Strategy<Value = MatrixConfig> {
    (
        1u16..=120,             // fps
        0.1f32..=4.0,           // speed
        0.0f32..=1.0,           // density
        1u16..=40,              // min_trail
        40u16..=120,            // max_trail (>= min_trail by construction below)
        0.0f32..=1.0,           // mutation_rate
        0.0f32..=1.0,           // glitch
    )
        .prop_map(|(fps, speed, density, min_t, max_t, mut_rate, glitch)| {
            MatrixConfig::builder()
                .fps(fps)
                .speed(speed)
                .density(density)
                .min_trail(min_t)
                .max_trail(max_t.max(min_t))
                .mutation_rate(mut_rate)
                .glitch(glitch)
                .build()
                .unwrap()
        })
}

proptest! {
    #[test]
    fn render_random_area_random_config_never_panics(
        seed: u64,
        cfg in valid_config(),
        w in 0u16..=200,
        h in 0u16..=80,
    ) {
        let mut state = MatrixRainState::with_seed(seed);
        let mut buf = Buffer::empty(Rect::new(0, 0, w.max(1), h.max(1)));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, w, h), &mut buf, &mut state);
    }

    #[test]
    fn streams_len_equals_width_when_non_empty(
        seed: u64,
        w in 1u16..=200,
        h in 1u16..=80,
    ) {
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(seed);
        let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, w, h), &mut buf, &mut state);
        prop_assert_eq!(state.streams_len(), w as usize);
    }

    #[test]
    fn empty_area_clears_streams_to_zero(
        seed: u64,
        w_then in 1u16..=80,
        h_then in 1u16..=30,
    ) {
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(seed);
        let mut buf = Buffer::empty(Rect::new(0, 0, w_then, h_then));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, w_then, h_then), &mut buf, &mut state);
        // Now render into a 0-width area:
        let mut buf2 = Buffer::empty(Rect::new(0, 0, 1, h_then));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, 0, h_then), &mut buf2, &mut state);
        prop_assert_eq!(state.streams_len(), 0);
    }

    #[test]
    fn oscillating_resize_keeps_per_column_invariant(
        seed: u64,
        sizes in proptest::collection::vec((1u16..=80, 1u16..=30), 1..=10),
    ) {
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(seed);
        for (w, h) in sizes {
            let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
            MatrixRain::new(&cfg).render(Rect::new(0, 0, w, h), &mut buf, &mut state);
            prop_assert_eq!(state.streams_len(), w as usize);
        }
    }

    #[test]
    fn no_painting_outside_widget_subarea(
        seed: u64,
        outer_w in 20u16..=80,
        outer_h in 10u16..=30,
        widget_w in 1u16..=40,
        widget_h in 1u16..=20,
        ox in 0u16..=10,
        oy in 0u16..=5,
    ) {
        prop_assume!(ox + widget_w <= outer_w);
        prop_assume!(oy + widget_h <= outer_h);

        let cfg = MatrixConfig::builder().density(1.0).build().unwrap();
        let mut state = MatrixRainState::with_seed(seed);
        let buf_area = Rect::new(0, 0, outer_w, outer_h);
        let mut buf = Buffer::empty(buf_area);
        for y in 0..outer_h {
            for x in 0..outer_w {
                buf.get_mut(x, y).set_char('#');
            }
        }
        let widget_area = Rect::new(ox, oy, widget_w, widget_h);
        for _ in 0..15 {
            MatrixRain::new(&cfg).render(widget_area, &mut buf, &mut state);
            state.tick();
        }
        for y in 0..outer_h {
            for x in 0..outer_w {
                let inside = (ox..ox + widget_w).contains(&x)
                    && (oy..oy + widget_h).contains(&y);
                if !inside {
                    prop_assert_eq!(buf.get(x, y).symbol(), "#",
                        "cell ({}, {}) outside widget area was modified", x, y);
                }
            }
        }
    }

    #[test]
    fn extreme_thin_areas_no_panic(
        seed: u64,
        long: bool,
        size in 1u16..=500,
    ) {
        let (w, h) = if long { (size, 1) } else { (1, size) };
        let cfg = MatrixConfig::default();
        let mut state = MatrixRainState::with_seed(seed);
        let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
        MatrixRain::new(&cfg).render(Rect::new(0, 0, w, h), &mut buf, &mut state);
        for _ in 0..5 {
            state.tick();
            MatrixRain::new(&cfg).render(Rect::new(0, 0, w, h), &mut buf, &mut state);
        }
        prop_assert_eq!(state.streams_len(), w as usize);
    }
}

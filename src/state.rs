//! Per-frame animation state carried across [`MatrixRain`](crate::MatrixRain)
//! renders.

use alloc::vec::Vec;
use core::cell::Cell;
use core::marker::PhantomData;

#[cfg(feature = "std")]
use core::time::Duration;
#[cfg(feature = "std")]
use std::time::Instant;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use ratatui::layout::Rect;

use crate::config::MatrixConfig;
use crate::stream::Stream;

#[cfg(feature = "std")]
const MAX_CATCHUP_TICKS: u32 = 4;

/// Per-frame animation state for a [`MatrixRain`](crate::MatrixRain) widget.
///
/// Holds one stream per terminal column, a seeded RNG, timing bookkeeping,
/// and a cached terminal color count. The same state instance must be passed
/// across consecutive renders so the animation continues from frame to frame.
///
/// `MatrixRainState` is `Send` but not `Sync` — it's designed for
/// single-threaded use (render takes `&mut self`).
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
/// let mut state = MatrixRainState::with_seed(42);
/// let mut buf = Buffer::empty(Rect::new(0, 0, 40, 12));
/// MatrixRain::new(&cfg).render(Rect::new(0, 0, 40, 12), &mut buf, &mut state);
/// assert_eq!(state.streams_len(), 40);
/// ```
pub struct MatrixRainState {
    streams: Vec<Stream>,
    #[cfg(feature = "std")]
    last_tick: Option<Instant>,
    #[cfg(feature = "std")]
    accum: Duration,
    frame: u64,
    rng: SmallRng,
    last_area: Option<Rect>,
    color_count: Option<u16>,
    last_config: Option<MatrixConfig>,
    paused: bool,
    _not_sync: PhantomData<Cell<()>>,
}

impl MatrixRainState {
    /// Create a new state seeded from system entropy.
    ///
    /// Use [`with_seed`](Self::with_seed) instead when you need reproducible
    /// output (snapshot tests, screenshots, `--seed` in the binary).
    ///
    /// **Requires the `std` feature** (entropy comes from `getrandom`).
    /// `no_std` callers must use [`with_seed`](Self::with_seed).
    #[cfg(feature = "std")]
    pub fn new() -> Self {
        Self::from_rng(SmallRng::from_entropy())
    }

    /// Create a new state with a deterministic RNG seed.
    ///
    /// Two states constructed with the same seed and driven through the same
    /// area/config sequence produce identical streams.
    ///
    /// # Example
    ///
    /// ```
    /// use matrix_rain::MatrixRainState;
    /// let a = MatrixRainState::with_seed(42);
    /// let b = MatrixRainState::with_seed(42);
    /// assert_eq!(a.streams_len(), b.streams_len());
    /// ```
    pub fn with_seed(seed: u64) -> Self {
        Self::from_rng(SmallRng::seed_from_u64(seed))
    }

    fn from_rng(rng: SmallRng) -> Self {
        Self {
            streams: Vec::new(),
            #[cfg(feature = "std")]
            last_tick: None,
            #[cfg(feature = "std")]
            accum: Duration::ZERO,
            frame: 0,
            rng,
            last_area: None,
            color_count: None,
            last_config: None,
            paused: false,
            _not_sync: PhantomData,
        }
    }

    /// Advance the animation by exactly one frame, regardless of wall-clock
    /// time. Bypasses pause.
    ///
    /// Uses the area and configuration cached by the most recent
    /// [`MatrixRain::render`](crate::MatrixRain) call; before the first
    /// render, this is a silent no-op. `last_tick` is **not** touched, so
    /// mixing manual ticks with wall-clock-driven renders will drift over
    /// time — pick one driving mode per session.
    pub fn tick(&mut self) {
        let area = match self.last_area {
            Some(a) if a.width > 0 && a.height > 0 => a,
            _ => return,
        };
        let Some(config) = self.last_config.take() else {
            return;
        };
        self.apply_one_tick(area, &config);
        self.last_config = Some(config);
    }

    /// Clear streams, timing, cached area/config, frame counter, and the
    /// paused flag. RNG state and cached color count are preserved.
    ///
    /// After reset, the next render is treated as a first render (applies
    /// exactly one tick).
    pub fn reset(&mut self) {
        self.streams.clear();
        #[cfg(feature = "std")]
        {
            self.last_tick = None;
            self.accum = Duration::ZERO;
        }
        self.last_area = None;
        self.last_config = None;
        self.frame = 0;
        self.paused = false;
    }

    /// Pause wall-clock-driven advance. Subsequent `render()` / `advance()` calls
    /// still handle resize and paint the current state, but do not move streams
    /// forward. Manual `tick()` is unaffected. Idempotent.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume wall-clock-driven advance after a `pause()`. Discards any
    /// previously-recorded `last_tick`/`accum` so the next render is treated
    /// as a first render (exactly one tick applied) — preventing the
    /// catch-up-cap stutter that an accumulated pause-time would otherwise
    /// trigger. Idempotent.
    pub fn resume(&mut self) {
        self.paused = false;
        #[cfg(feature = "std")]
        {
            self.last_tick = None;
            self.accum = Duration::ZERO;
        }
    }

    /// Returns whether wall-clock advance is currently suppressed.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Returns the number of column streams currently allocated.
    ///
    /// After a render into a non-empty area, this equals `area.width as usize`.
    /// After a render into an empty area (`width == 0` or `height == 0`),
    /// returns `0`.
    pub fn streams_len(&self) -> usize {
        self.streams.len()
    }

    pub(crate) fn streams(&self) -> &[Stream] {
        &self.streams
    }

    pub(crate) fn color_count(&self) -> Option<u16> {
        self.color_count
    }

    /// Override the cached terminal color count, suppressing auto-detection on the next render.
    /// Useful for forcing a specific gradient tier (16-color collapse for accessibility,
    /// 256-color quantization, or u16::MAX for the smooth-interpolation path) and for
    /// deterministic testing where TERM/COLORTERM should not influence rendering.
    pub fn set_color_count(&mut self, count: u16) {
        self.color_count = Some(count);
    }

    pub(crate) fn advance(&mut self, area: Rect, config: &MatrixConfig) {
        if area.width == 0 || area.height == 0 {
            self.streams.clear();
            #[cfg(feature = "std")]
            {
                self.last_tick = None;
                self.accum = Duration::ZERO;
            }
            self.last_area = None;
            return;
        }

        self.handle_resize(area, config);

        #[cfg(feature = "std")]
        if !self.paused {
            let now = Instant::now();
            let ticks = self.compute_tick_budget(now, config);
            for _ in 0..ticks {
                self.apply_one_tick(area, config);
            }
            self.last_tick = Some(now);
        }

        self.last_area = Some(area);
        self.last_config = Some(config.clone());
    }

    fn handle_resize(&mut self, area: Rect, config: &MatrixConfig) {
        let prev = self.last_area;
        let new_w = area.width as usize;

        let width_changed = prev.map_or(true, |p| p.width != area.width);
        let height_changed = prev.map_or(false, |p| p.height != area.height);

        if width_changed {
            if self.streams.len() < new_w {
                for _ in self.streams.len()..new_w {
                    self.streams
                        .push(Stream::new_idle(config.max_trail, &mut self.rng));
                }
            } else if self.streams.len() > new_w {
                self.streams.truncate(new_w);
            }
        }

        if height_changed {
            let max_head = (area.height as f32) + (config.max_trail as f32);
            for stream in &mut self.streams {
                if stream.is_active() {
                    let clamped = stream.head_row().clamp(0.0, max_head);
                    stream.set_head_row(clamped);
                    if (clamped - stream.length() as f32) >= area.height as f32 {
                        stream.force_retire(&mut self.rng);
                    }
                }
            }
        }
    }

    #[cfg(feature = "std")]
    fn compute_tick_budget(&mut self, now: Instant, config: &MatrixConfig) -> u32 {
        let ticks_per_sec = (config.fps as f32) * config.speed;
        if !ticks_per_sec.is_finite() || ticks_per_sec <= 0.0 {
            self.accum = Duration::ZERO;
            return 0;
        }

        match self.last_tick {
            None => {
                self.accum = Duration::ZERO;
                1
            }
            Some(prev) => {
                let elapsed = now.saturating_duration_since(prev);
                let total_secs = elapsed.as_secs_f32() + self.accum.as_secs_f32();
                let total_ticks = total_secs * ticks_per_sec;
                if !total_ticks.is_finite() {
                    self.accum = Duration::ZERO;
                    return 0;
                }
                let ticks = (total_ticks.floor() as u32).min(MAX_CATCHUP_TICKS);
                let leftover_ticks = (total_ticks - ticks as f32).max(0.0);
                let leftover_secs = leftover_ticks / ticks_per_sec;
                self.accum = Duration::from_secs_f32(leftover_secs.max(0.0));
                ticks
            }
        }
    }

    fn apply_one_tick(&mut self, area: Rect, config: &MatrixConfig) {
        let chars = config.charset.chars();
        for stream in &mut self.streams {
            stream.tick(area.height, config.fps, &mut self.rng);
        }
        if config.mutation_rate > 0.0 {
            for stream in &mut self.streams {
                stream.mutate(&mut self.rng, chars, config.mutation_rate);
            }
        }
        if config.glitch > 0.0 {
            for stream in &mut self.streams {
                stream.glitch_roll(&mut self.rng, config.glitch);
            }
        }
        for stream in &mut self.streams {
            if stream.is_ready_to_spawn() && self.rng.gen::<f32>() < config.density {
                stream.spawn(
                    &mut self.rng,
                    chars,
                    config.min_trail,
                    config.max_trail,
                    config.fps,
                );
            }
        }
        self.frame = self.frame.wrapping_add(1);
    }
}

#[cfg(feature = "std")]
impl Default for MatrixRainState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(w: u16, h: u16) -> Rect {
        Rect::new(0, 0, w, h)
    }

    #[test]
    fn new_starts_with_no_streams_no_timing() {
        let s = MatrixRainState::new();
        assert!(s.streams.is_empty());
        assert!(s.last_tick.is_none());
        assert!(s.last_area.is_none());
        assert_eq!(s.frame, 0);
    }

    #[test]
    fn first_render_budget_is_one_tick() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig::default();
        let ticks = s.compute_tick_budget(Instant::now(), &cfg);
        assert_eq!(ticks, 1);
        assert_eq!(s.accum, Duration::ZERO);
    }

    #[test]
    fn first_render_allocates_streams_per_column() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig::default();
        s.advance(area(12, 10), &cfg);
        assert_eq!(s.streams().len(), 12);
        assert_eq!(s.frame, 1);
        assert!(s.last_tick.is_some());
    }

    #[test]
    fn width_resize_grows_and_shrinks_streams() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig::default();
        s.advance(area(5, 10), &cfg);
        assert_eq!(s.streams().len(), 5);
        s.advance(area(10, 10), &cfg);
        assert_eq!(s.streams().len(), 10);
        s.advance(area(3, 10), &cfg);
        assert_eq!(s.streams().len(), 3);
    }

    #[test]
    fn empty_area_clears_streams_and_resets_first_render_path() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig::default();
        s.advance(area(10, 10), &cfg);
        let frame_after_first = s.frame;

        s.advance(area(0, 10), &cfg);
        assert_eq!(s.streams().len(), 0);
        assert!(s.last_tick.is_none());
        assert!(s.last_area.is_none());

        s.advance(area(10, 10), &cfg);
        assert_eq!(s.frame, frame_after_first + 1);
    }

    #[test]
    fn empty_area_height_zero_also_handled() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig::default();
        s.advance(area(10, 0), &cfg);
        assert_eq!(s.streams().len(), 0);
        assert!(s.last_tick.is_none());
    }

    #[test]
    fn tick_before_first_render_is_noop() {
        let mut s = MatrixRainState::with_seed(0);
        s.tick();
        assert_eq!(s.frame, 0);
        assert!(s.last_tick.is_none());
    }

    #[test]
    fn tick_after_first_render_advances_one_frame() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig::default();
        s.advance(area(10, 20), &cfg);
        let frame_before = s.frame;
        let last_tick_before = s.last_tick;
        s.tick();
        assert_eq!(s.frame, frame_before + 1);
        assert_eq!(
            s.last_tick, last_tick_before,
            "tick() must not touch last_tick"
        );
    }

    #[test]
    fn reset_clears_streams_and_timing_keeps_color_count() {
        let mut s = MatrixRainState::with_seed(42);
        let cfg = MatrixConfig::default();
        s.advance(area(10, 20), &cfg);
        s.set_color_count(256);
        s.reset();
        assert_eq!(s.streams().len(), 0);
        assert!(s.last_tick.is_none());
        assert!(s.last_area.is_none());
        assert_eq!(s.frame, 0);
        assert_eq!(s.color_count(), Some(256));
    }

    #[test]
    fn deterministic_with_same_seed() {
        let cfg = MatrixConfig::default();
        let mut a = MatrixRainState::with_seed(0xC0FFEE);
        let mut b = MatrixRainState::with_seed(0xC0FFEE);
        a.advance(area(15, 15), &cfg);
        b.advance(area(15, 15), &cfg);
        assert_eq!(a.streams().len(), b.streams().len());
        for (sa, sb) in a.streams().iter().zip(b.streams()) {
            assert_eq!(sa.is_active(), sb.is_active());
            assert_eq!(sa.length(), sb.length());
            assert_eq!(sa.head_row(), sb.head_row());
        }
    }

    #[test]
    fn catchup_cap_limits_huge_elapsed() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig::default();
        s.last_tick = Some(Instant::now() - Duration::from_secs(60));
        let ticks = s.compute_tick_budget(Instant::now(), &cfg);
        assert_eq!(ticks, MAX_CATCHUP_TICKS);
    }

    #[test]
    fn sub_tick_render_carries_remainder() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig::default();
        let now = Instant::now();
        s.last_tick = Some(now - Duration::from_micros(500));
        let ticks = s.compute_tick_budget(now, &cfg);
        assert_eq!(ticks, 0);
        assert!(s.accum > Duration::ZERO);
    }

    #[test]
    fn pathological_zero_fps_no_panic() {
        let mut s = MatrixRainState::with_seed(0);
        let cfg = MatrixConfig {
            fps: 0,
            ..MatrixConfig::default()
        };
        assert_eq!(s.compute_tick_budget(Instant::now(), &cfg), 0);
    }

    #[test]
    fn color_count_default_none_then_set() {
        let mut s = MatrixRainState::new();
        assert!(s.color_count().is_none());
        s.set_color_count(16);
        assert_eq!(s.color_count(), Some(16));
    }

    #[test]
    fn state_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<MatrixRainState>();
    }

    #[test]
    fn mutation_rate_zero_keeps_glyphs_unchanged_per_tick() {
        // Tall area so the stream we're tracking can't retire mid-test.
        let cfg = MatrixConfig::builder()
            .fps(30)
            .density(1.0)
            .mutation_rate(0.0)
            .min_trail(8)
            .max_trail(8)
            .charset(crate::charset::CharSet::Custom(vec!['a', 'b', 'c']))
            .build()
            .unwrap();
        let mut s = MatrixRainState::with_seed(0x1234);
        s.advance(area(8, 400), &cfg);
        for _ in 0..15 {
            s.apply_one_tick(area(8, 400), &cfg);
        }
        let idx = s.streams.iter().position(|st| st.is_active()).expect("active");
        let before: Vec<char> = s.streams[idx].glyphs().to_vec();
        s.apply_one_tick(area(8, 400), &cfg);
        assert!(s.streams[idx].is_active());
        assert_eq!(s.streams[idx].glyphs(), before.as_slice());
    }

    #[test]
    fn pause_freezes_frame_advance_in_render_path() {
        let cfg = MatrixConfig::default();
        let mut s = MatrixRainState::with_seed(0xBABE);
        s.advance(area(8, 20), &cfg);
        let frame_after_first = s.frame;
        assert!(frame_after_first > 0);

        s.pause();
        assert!(s.is_paused());
        // Many renders while paused — frame counter must not advance.
        for _ in 0..50 {
            s.advance(area(8, 20), &cfg);
        }
        assert_eq!(s.frame, frame_after_first);
        // last_area is still cached (so resize handling stays consistent).
        assert_eq!(s.last_area, Some(area(8, 20)));
    }

    #[test]
    fn resume_clears_last_tick_so_next_render_is_first_render() {
        let cfg = MatrixConfig::default();
        let mut s = MatrixRainState::with_seed(0xBABE);
        s.advance(area(8, 20), &cfg);
        s.pause();
        s.advance(area(8, 20), &cfg);

        s.resume();
        assert!(!s.is_paused());
        assert!(s.last_tick.is_none());
        assert_eq!(s.accum, Duration::ZERO);

        let frame_before = s.frame;
        s.advance(area(8, 20), &cfg);
        assert_eq!(
            s.frame,
            frame_before + 1,
            "post-resume render should apply exactly one tick (first-render path)"
        );
    }

    #[test]
    fn tick_bypasses_pause() {
        let cfg = MatrixConfig::default();
        let mut s = MatrixRainState::with_seed(0xBABE);
        s.advance(area(8, 20), &cfg);
        s.pause();
        let frame_before = s.frame;
        s.tick();
        assert_eq!(s.frame, frame_before + 1);
        assert!(s.is_paused(), "tick must not implicitly resume");
    }

    #[test]
    fn pause_and_resume_are_idempotent() {
        let mut s = MatrixRainState::new();
        s.pause();
        s.pause();
        assert!(s.is_paused());
        s.resume();
        s.resume();
        assert!(!s.is_paused());
    }

    #[test]
    fn reset_clears_paused_state() {
        let mut s = MatrixRainState::new();
        s.pause();
        s.reset();
        assert!(!s.is_paused());
    }

    #[test]
    fn resize_while_paused_still_resizes_streams() {
        let cfg = MatrixConfig::default();
        let mut s = MatrixRainState::with_seed(0xBABE);
        s.advance(area(8, 20), &cfg);
        s.pause();
        s.advance(area(16, 20), &cfg);
        assert_eq!(s.streams.len(), 16);
        s.advance(area(4, 20), &cfg);
        assert_eq!(s.streams.len(), 4);
    }

    #[test]
    fn glitch_zero_leaves_flags_unset_after_apply_one_tick() {
        let cfg = MatrixConfig::builder()
            .fps(30)
            .density(1.0)
            .glitch(0.0)
            .build()
            .unwrap();
        let mut s = MatrixRainState::with_seed(0xFEED);
        s.advance(area(8, 200), &cfg);
        for _ in 0..10 {
            s.apply_one_tick(area(8, 200), &cfg);
        }
        for stream in &s.streams {
            if stream.is_active() {
                for i in 0..stream.length() {
                    assert!(!stream.is_glitched(i));
                }
            }
        }
    }

    #[test]
    fn glitch_one_sets_all_flags_after_apply_one_tick() {
        let cfg = MatrixConfig::builder()
            .fps(30)
            .density(1.0)
            .glitch(1.0)
            .min_trail(6)
            .max_trail(6)
            .build()
            .unwrap();
        let mut s = MatrixRainState::with_seed(0xFEED);
        s.advance(area(8, 200), &cfg);
        for _ in 0..15 {
            s.apply_one_tick(area(8, 200), &cfg);
        }
        let stream = s.streams.iter().find(|st| st.is_active()).expect("active");
        for i in 0..stream.length() {
            assert!(stream.is_glitched(i), "cell {i} should be glitched at rate=1.0");
        }
    }

    #[test]
    fn mutation_rate_one_changes_at_least_one_glyph_per_tick() {
        // Charset of 2 → each cell has 50% chance of flipping per tick when rate=1.
        // Across 8 cells the prob all stay same is (0.5)^8 = 1/256; with a fixed
        // seed this is deterministic.
        let cfg = MatrixConfig::builder()
            .fps(30)
            .density(1.0)
            .mutation_rate(1.0)
            .min_trail(8)
            .max_trail(8)
            .charset(crate::charset::CharSet::Custom(vec!['a', 'b']))
            .build()
            .unwrap();
        let mut s = MatrixRainState::with_seed(0xABCD);
        s.advance(area(8, 400), &cfg);
        for _ in 0..15 {
            s.apply_one_tick(area(8, 400), &cfg);
        }
        let idx = s.streams.iter().position(|st| st.is_active()).expect("active");
        let before: Vec<char> = s.streams[idx].glyphs().to_vec();
        s.apply_one_tick(area(8, 400), &cfg);
        assert!(s.streams[idx].is_active());
        let changed = s.streams[idx]
            .glyphs()
            .iter()
            .zip(before.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert!(changed > 0, "expected at least one glyph to mutate");
        for g in s.streams[idx].glyphs() {
            assert!(['a', 'b'].contains(g), "mutated glyph {g} not from charset");
        }
    }
}

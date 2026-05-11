use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::Rng;

const MAX_RESPAWN_COOLDOWN: u16 = 60;

#[derive(Clone, Debug)]
pub(crate) struct Stream {
    head_row: f32,
    length: u16,
    speed: f32,
    glyphs: Vec<char>,
    glitch_flags: Vec<bool>,
    state: StreamState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamState {
    Idle { cooldown: u16 },
    Active,
}

impl Stream {
    pub(crate) fn new_idle(max_trail: u16, rng: &mut SmallRng) -> Self {
        let cooldown = rng.gen_range(0..=MAX_RESPAWN_COOLDOWN);
        Self {
            head_row: 0.0,
            length: 0,
            speed: 0.0,
            glyphs: Vec::with_capacity(max_trail as usize),
            glitch_flags: Vec::with_capacity(max_trail as usize),
            state: StreamState::Idle { cooldown },
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        matches!(self.state, StreamState::Active)
    }

    pub(crate) fn is_ready_to_spawn(&self) -> bool {
        matches!(self.state, StreamState::Idle { cooldown: 0 })
    }

    pub(crate) fn spawn(
        &mut self,
        rng: &mut SmallRng,
        chars: &[char],
        min_trail: u16,
        max_trail: u16,
        fps: u16,
    ) {
        debug_assert!(!chars.is_empty(), "chars must be non-empty");
        debug_assert!(min_trail >= 1 && min_trail <= max_trail);
        debug_assert!(fps >= 1);

        let length = rng.gen_range(min_trail..=max_trail);
        let speed = rng.gen_range((fps as f32 * 0.4)..(fps as f32 * 1.2));

        self.head_row = 0.0;
        self.length = length;
        self.speed = speed;
        self.glyphs.clear();
        for _ in 0..length {
            self.glyphs.push(*chars.choose(rng).expect("non-empty"));
        }
        self.glitch_flags.clear();
        self.glitch_flags.resize(length as usize, false);
        self.state = StreamState::Active;
    }

    /// Advance one tick. Returns `true` if the stream retired this tick.
    pub(crate) fn tick(&mut self, area_height: u16, fps: u16, rng: &mut SmallRng) -> bool {
        let mut retired = false;
        match &mut self.state {
            StreamState::Idle { cooldown } => {
                if *cooldown > 0 {
                    *cooldown -= 1;
                }
            }
            StreamState::Active => {
                self.head_row += self.speed / fps as f32;
                if (self.head_row - self.length as f32) >= area_height as f32 {
                    retired = true;
                }
            }
        }
        if retired {
            self.retire(rng);
        }
        retired
    }

    fn retire(&mut self, rng: &mut SmallRng) {
        let cooldown = rng.gen_range(0..=MAX_RESPAWN_COOLDOWN);
        self.state = StreamState::Idle { cooldown };
        self.head_row = 0.0;
        self.length = 0;
        self.speed = 0.0;
        self.glyphs.clear();
        self.glitch_flags.clear();
    }

    pub(crate) fn force_retire(&mut self, rng: &mut SmallRng) {
        if self.is_active() {
            self.retire(rng);
        }
    }

    /// Roll each glyph in the trail independently against `mutation_rate`;
    /// replace those that hit with a fresh draw from `chars`. No-op on
    /// idle streams or when `mutation_rate <= 0.0`.
    pub(crate) fn mutate(&mut self, rng: &mut SmallRng, chars: &[char], mutation_rate: f32) {
        if !self.is_active() || mutation_rate <= 0.0 {
            return;
        }
        debug_assert!(!chars.is_empty(), "chars must be non-empty");
        for g in self.glyphs.iter_mut() {
            if rng.gen::<f32>() < mutation_rate {
                *g = *chars.choose(rng).expect("non-empty");
            }
        }
    }

    /// Re-roll the per-cell glitch flags for this frame. On hit, the renderer
    /// shifts the cell's color to `ColorRamp.head`, producing a sparkle. Always
    /// resets prior flags to keep behaviour clean when `rate` toggles between
    /// renders. No-op on idle streams.
    pub(crate) fn glitch_roll(&mut self, rng: &mut SmallRng, rate: f32) {
        if !self.is_active() {
            return;
        }
        let rate_positive = rate > 0.0;
        for f in self.glitch_flags.iter_mut() {
            *f = rate_positive && rng.gen::<f32>() < rate;
        }
    }

    pub(crate) fn is_glitched(&self, i: u16) -> bool {
        self.glitch_flags.get(i as usize).copied().unwrap_or(false)
    }

    pub(crate) fn set_head_row(&mut self, head_row: f32) {
        if self.is_active() {
            self.head_row = head_row;
        }
    }

    pub(crate) fn head_row(&self) -> f32 {
        self.head_row
    }

    pub(crate) fn length(&self) -> u16 {
        self.length
    }

    pub(crate) fn glyphs(&self) -> &[char] {
        &self.glyphs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn rng() -> SmallRng {
        SmallRng::seed_from_u64(0xC0FFEE)
    }

    #[test]
    fn new_idle_starts_inactive_with_bounded_cooldown() {
        let mut r = rng();
        let s = Stream::new_idle(20, &mut r);
        assert!(!s.is_active());
        match s.state {
            StreamState::Idle { cooldown } => assert!(cooldown <= MAX_RESPAWN_COOLDOWN),
            _ => panic!("expected Idle"),
        }
    }

    #[test]
    fn glyph_buffer_is_preallocated() {
        let mut r = rng();
        let s = Stream::new_idle(20, &mut r);
        assert_eq!(s.glyphs.capacity(), 20);
    }

    #[test]
    fn idle_tick_decrements_cooldown_to_zero() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        for _ in 0..=MAX_RESPAWN_COOLDOWN {
            s.tick(50, 30, &mut r);
        }
        assert!(s.is_ready_to_spawn());
    }

    #[test]
    fn spawn_fills_active_stream_from_chars_only() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        let chars: Vec<char> = "ABC".chars().collect();
        s.spawn(&mut r, &chars, 4, 10, 30);
        assert!(s.is_active());
        assert_eq!(s.head_row(), 0.0);
        assert!((4..=10).contains(&s.length()));
        assert_eq!(s.glyphs().len(), s.length() as usize);
        for g in s.glyphs() {
            assert!(chars.contains(g), "glyph {g} not from chars");
        }
        assert!(s.speed > 0.0);
    }

    #[test]
    fn active_tick_advances_head() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 4, 4, 30);
        let before = s.head_row();
        s.tick(50, 30, &mut r);
        assert!(s.head_row() > before);
    }

    #[test]
    fn retires_when_tail_passes_bottom() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 4, 4, 30);
        // Drive enough ticks to push the entire trail off-screen on a tiny area.
        let mut retired = false;
        for _ in 0..1000 {
            if s.tick(5, 30, &mut r) {
                retired = true;
                break;
            }
        }
        assert!(retired, "stream should retire eventually");
        assert!(!s.is_active());
        assert_eq!(s.glyphs().len(), 0);
    }

    #[test]
    fn trail_longer_than_viewport_still_retires() {
        let mut r = rng();
        let mut s = Stream::new_idle(50, &mut r);
        // Length 30 in a 5-row viewport: head reaches bottom long before tail exists.
        s.spawn(&mut r, &['x'], 30, 30, 30);
        let mut retired = false;
        for _ in 0..2000 {
            if s.tick(5, 30, &mut r) {
                retired = true;
                break;
            }
        }
        assert!(retired);
    }

    #[test]
    fn retire_returns_to_idle_with_fresh_cooldown() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 4, 4, 30);
        for _ in 0..500 {
            s.tick(3, 30, &mut r);
        }
        assert!(!s.is_active());
        // After retire, glyph capacity is preserved (no reallocation on respawn).
        assert!(s.glyphs.capacity() >= 4);
    }

    #[test]
    fn mutate_with_rate_zero_is_noop() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x', 'y', 'z'], 8, 8, 30);
        let before = s.glyphs.clone();
        s.mutate(&mut r, &['x', 'y', 'z'], 0.0);
        assert_eq!(s.glyphs, before);
    }

    #[test]
    fn mutate_on_idle_stream_is_noop() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        assert!(!s.is_active());
        // Should not panic or alter state.
        s.mutate(&mut r, &['x'], 1.0);
        assert!(!s.is_active());
    }

    #[test]
    fn mutate_with_rate_one_replaces_every_cell() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        // Spawn with a single-char set, then mutate with a different single-char set.
        s.spawn(&mut r, &['x'], 8, 8, 30);
        assert!(s.glyphs.iter().all(|&g| g == 'x'));
        s.mutate(&mut r, &['y'], 1.0);
        assert!(s.glyphs.iter().all(|&g| g == 'y'));
    }

    #[test]
    fn mutate_only_draws_from_provided_chars() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['a', 'b'], 8, 8, 30);
        s.mutate(&mut r, &['Q', 'R', 'S'], 1.0);
        for g in &s.glyphs {
            assert!(['Q', 'R', 'S'].contains(g), "glyph {g} not from mutation chars");
        }
    }

    #[test]
    fn mutate_partial_rate_changes_some_glyphs() {
        // With a fixed seed and rate=0.5 over 64 cells, mutation should
        // change SOME glyphs but not all (probabilistically near-certain).
        let mut r = rng();
        let mut s = Stream::new_idle(64, &mut r);
        s.spawn(&mut r, &['a'], 64, 64, 30);
        s.mutate(&mut r, &['b'], 0.5);
        let n_changed = s.glyphs.iter().filter(|&&g| g == 'b').count();
        let n_kept = s.glyphs.iter().filter(|&&g| g == 'a').count();
        assert!(n_changed > 0, "expected some glyphs to mutate");
        assert!(n_kept > 0, "expected some glyphs to remain");
        assert_eq!(n_changed + n_kept, 64);
    }

    #[test]
    fn glitch_flags_match_length_after_spawn() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 8, 8, 30);
        assert_eq!(s.glitch_flags.len(), 8);
        for f in &s.glitch_flags {
            assert!(!f, "fresh-spawn glitch flags must default to false");
        }
    }

    #[test]
    fn glitch_roll_rate_zero_leaves_all_false() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 8, 8, 30);
        s.glitch_roll(&mut r, 0.0);
        assert!(s.glitch_flags.iter().all(|&f| !f));
        assert!(!s.is_glitched(0));
        assert!(!s.is_glitched(7));
    }

    #[test]
    fn glitch_roll_rate_one_sets_all_true() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 8, 8, 30);
        s.glitch_roll(&mut r, 1.0);
        assert!(s.glitch_flags.iter().all(|&f| f));
    }

    #[test]
    fn glitch_roll_resets_prior_flags_when_rate_drops_to_zero() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 8, 8, 30);
        s.glitch_roll(&mut r, 1.0);
        assert!(s.glitch_flags.iter().all(|&f| f));
        s.glitch_roll(&mut r, 0.0);
        assert!(s.glitch_flags.iter().all(|&f| !f));
    }

    #[test]
    fn glitch_roll_on_idle_is_noop() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        // No spawn yet; glitch_flags is empty.
        s.glitch_roll(&mut r, 1.0);
        assert!(s.glitch_flags.is_empty());
        assert!(!s.is_glitched(0));
    }

    #[test]
    fn glitch_flags_cleared_on_retire() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 8, 8, 30);
        s.glitch_roll(&mut r, 1.0);
        s.force_retire(&mut r);
        assert!(s.glitch_flags.is_empty());
    }

    #[test]
    fn is_glitched_returns_false_out_of_bounds() {
        let mut r = rng();
        let mut s = Stream::new_idle(20, &mut r);
        s.spawn(&mut r, &['x'], 4, 4, 30);
        assert!(!s.is_glitched(0));
        assert!(!s.is_glitched(99));
    }
}

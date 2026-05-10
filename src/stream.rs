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
    }

    pub(crate) fn force_retire(&mut self, rng: &mut SmallRng) {
        if self.is_active() {
            self.retire(rng);
        }
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
}

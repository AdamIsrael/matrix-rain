//! [`MatrixConfig`] (the read-only configuration consumed by the widget),
//! its fluent [`MatrixConfigBuilder`], and the [`MAX_TRAIL_LIMIT`] cap.

use alloc::format;
use alloc::string::ToString;

use ratatui::style::Color;

use crate::charset::CharSet;
use crate::error::MatrixError;
use crate::theme::Theme;

/// Read-only configuration for [`MatrixRain`](crate::MatrixRain).
///
/// Construct via [`MatrixConfig::builder`] (validated) or
/// [`MatrixConfig::default`] (always valid, classic Matrix look). All fields
/// are public so the struct also supports destructuring for inspection.
///
/// # Example
///
/// ```
/// use matrix_rain::MatrixConfig;
///
/// let cfg = MatrixConfig::builder().fps(60).density(0.8).build().unwrap();
/// assert_eq!(cfg.fps, 60);
/// assert_eq!(cfg.density, 0.8);
/// ```
#[derive(Clone, Debug)]
pub struct MatrixConfig {
    /// Glyph source. Default: [`CharSet::Matrix`] (katakana + digits).
    pub charset: CharSet,
    /// Color theme. Default: [`Theme::ClassicGreen`].
    pub theme: Theme,
    /// Frames per second (must be `>= 1`). Acts as the wall-clock tick budget
    /// when wall-clock driving and as the divisor for `speed`. Default: `30`.
    pub fps: u16,
    /// Global speed multiplier (must be finite and `> 0.0`). Scales the
    /// effective tick rate consumed from wall-clock elapsed time. Default: `1.0`.
    pub speed: f32,
    /// Fraction of columns to keep active. `0.0` = no spawns ever; `1.0` =
    /// every idle column attempts respawn each tick after cooldown. Must be
    /// finite and in `[0.0, 1.0]`. Default: `0.6`.
    pub density: f32,
    /// Minimum trail length. Must be `>= 1` and `<= max_trail`. Default: `6`.
    pub min_trail: u16,
    /// Maximum trail length. Must be `<= MAX_TRAIL_LIMIT`. Default: `20`.
    pub max_trail: u16,
    /// Per-cell glyph-reroll probability per tick. Must be finite and in
    /// `[0.0, 1.0]`. `0.0` freezes glyphs after spawn; `1.0` rerolls every
    /// cell every frame. Default: `0.05`.
    pub mutation_rate: f32,
    /// Apply `Modifier::BOLD` to the head cell when true. Default: `true`.
    pub bold_head: bool,
    /// Use `ColorRamp.head` (typically white) for the head cell when true,
    /// `ColorRamp.bright` when false. Default: `true`.
    pub head_white: bool,
    /// Per-cell color-flicker probability per tick. On hit, the cell renders
    /// with `ColorRamp.head` instead of its gradient color (a "sparkle").
    /// Head cell (i=0) is unaffected. Must be finite and in `[0.0, 1.0]`.
    /// Default: `0.0` (off).
    pub glitch: f32,
    /// Optional background color. `None` (default) renders transparently,
    /// skipping cells in the fade zone so the underlying buffer shows
    /// through. `Some(c)` skips any cell whose computed color equals `c` —
    /// useful when compositing over a known background.
    pub background: Option<Color>,
}

impl MatrixConfig {
    /// Returns a new [`MatrixConfigBuilder`] seeded with defaults.
    /// Call setters to override fields, then [`build`](MatrixConfigBuilder::build)
    /// to validate and produce a [`MatrixConfig`].
    pub fn builder() -> MatrixConfigBuilder {
        MatrixConfigBuilder::new()
    }
}

impl Default for MatrixConfig {
    fn default() -> Self {
        Self {
            charset: CharSet::Matrix,
            theme: Theme::ClassicGreen,
            fps: 30,
            speed: 1.0,
            density: 0.6,
            min_trail: 6,
            max_trail: 20,
            mutation_rate: 0.05,
            bold_head: true,
            head_white: true,
            glitch: 0.0,
            background: None,
        }
    }
}

#[derive(Clone, Debug)]
/// Fluent builder for [`MatrixConfig`].
///
/// Construct via [`MatrixConfig::builder`] or [`MatrixConfigBuilder::new`].
/// Setters take and return `self` so they can be chained. Call
/// [`build`](Self::build) to validate and produce a [`MatrixConfig`], or
/// receive a [`MatrixError`] describing which invariant failed.
///
/// # Example
///
/// ```
/// use matrix_rain::{CharSet, MatrixConfig, Theme};
///
/// let cfg = MatrixConfig::builder()
///     .fps(60)
///     .density(0.8)
///     .charset(CharSet::Hex)
///     .theme(Theme::Cyan)
///     .build()
///     .unwrap();
/// ```
pub struct MatrixConfigBuilder {
    config: MatrixConfig,
}

impl MatrixConfigBuilder {
    /// Returns a new builder seeded with [`MatrixConfig::default`].
    pub fn new() -> Self {
        Self {
            config: MatrixConfig::default(),
        }
    }

    /// Set the glyph source. See [`CharSet`].
    pub fn charset(mut self, charset: CharSet) -> Self {
        self.config.charset = charset;
        self
    }

    /// Set the color theme. See [`Theme`].
    pub fn theme(mut self, theme: Theme) -> Self {
        self.config.theme = theme;
        self
    }

    /// Set frames per second. Must be `>= 1`; [`build`](Self::build) rejects 0.
    pub fn fps(mut self, fps: u16) -> Self {
        self.config.fps = fps;
        self
    }

    /// Set the global speed multiplier. Must be finite and `> 0.0`.
    pub fn speed(mut self, speed: f32) -> Self {
        self.config.speed = speed;
        self
    }

    /// Set the column-activity density. Must be finite and in `[0.0, 1.0]`.
    pub fn density(mut self, density: f32) -> Self {
        self.config.density = density;
        self
    }

    /// Set the minimum trail length. Must be `>= 1` and `<= max_trail`.
    pub fn min_trail(mut self, min_trail: u16) -> Self {
        self.config.min_trail = min_trail;
        self
    }

    /// Set the maximum trail length. Must be `<= MAX_TRAIL_LIMIT`.
    pub fn max_trail(mut self, max_trail: u16) -> Self {
        self.config.max_trail = max_trail;
        self
    }

    /// Set the per-cell glyph reroll probability per tick. Finite, `[0.0, 1.0]`.
    pub fn mutation_rate(mut self, mutation_rate: f32) -> Self {
        self.config.mutation_rate = mutation_rate;
        self
    }

    /// Apply `Modifier::BOLD` to the head cell when `true`.
    pub fn bold_head(mut self, bold_head: bool) -> Self {
        self.config.bold_head = bold_head;
        self
    }

    /// Use `ColorRamp.head` for the head cell when `true`, `ColorRamp.bright`
    /// when `false`.
    pub fn head_white(mut self, head_white: bool) -> Self {
        self.config.head_white = head_white;
        self
    }

    /// Set the per-cell glitch (color flicker) probability per tick.
    /// Finite, `[0.0, 1.0]`. See [`MatrixConfig::glitch`].
    pub fn glitch(mut self, glitch: f32) -> Self {
        self.config.glitch = glitch;
        self
    }

    /// Set the background color. `None` for transparent (skip cells in the
    /// fade zone); `Some(c)` for a known compositing background.
    pub fn background(mut self, background: Option<Color>) -> Self {
        self.config.background = background;
        self
    }

    /// Validate the configured fields and return a [`MatrixConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError::InvalidConfig`] if any of the following fails
    /// (checked in order):
    ///
    /// - `fps >= 1`
    /// - `speed` is finite and `> 0`
    /// - `density` is finite and in `[0.0, 1.0]`
    /// - `min_trail >= 1`
    /// - `min_trail <= max_trail`
    /// - `max_trail <= MAX_TRAIL_LIMIT`
    /// - `mutation_rate` is finite and in `[0.0, 1.0]`
    /// - `glitch` is finite and in `[0.0, 1.0]`
    ///
    /// Returns [`MatrixError::EmptyCharset`] when [`CharSet::Custom`]
    /// resolves to an empty `Vec`, and [`MatrixError::InvalidConfig`] when
    /// any character in a charset is a `char::is_control`.
    ///
    /// Boundary values are accepted: `density == 0.0` / `1.0`,
    /// `min_trail == max_trail`, `mutation_rate == 0.0` / `1.0`,
    /// `glitch == 0.0` / `1.0`.
    pub fn build(self) -> Result<MatrixConfig, MatrixError> {
        let c = &self.config;

        if c.fps < 1 {
            return Err(invalid("fps must be >= 1"));
        }
        if !c.speed.is_finite() || c.speed <= 0.0 {
            return Err(invalid(&format!(
                "speed must be a positive finite number (got {})",
                c.speed
            )));
        }
        if !c.density.is_finite() || !(0.0..=1.0).contains(&c.density) {
            return Err(invalid(&format!(
                "density must be a finite number in [0.0, 1.0] (got {})",
                c.density
            )));
        }
        if c.min_trail < 1 {
            return Err(invalid("min_trail must be >= 1"));
        }
        if c.min_trail > c.max_trail {
            return Err(invalid(&format!(
                "min_trail ({}) must be <= max_trail ({})",
                c.min_trail, c.max_trail
            )));
        }
        if c.max_trail > MAX_TRAIL_LIMIT {
            return Err(invalid(&format!(
                "max_trail ({}) exceeds limit of {}",
                c.max_trail, MAX_TRAIL_LIMIT
            )));
        }
        if !c.mutation_rate.is_finite() || !(0.0..=1.0).contains(&c.mutation_rate) {
            return Err(invalid(&format!(
                "mutation_rate must be a finite number in [0.0, 1.0] (got {})",
                c.mutation_rate
            )));
        }
        if !c.glitch.is_finite() || !(0.0..=1.0).contains(&c.glitch) {
            return Err(invalid(&format!(
                "glitch must be a finite number in [0.0, 1.0] (got {})",
                c.glitch
            )));
        }
        c.charset.validate()?;

        Ok(self.config)
    }
}

/// Hard upper bound on [`MatrixConfig::max_trail`].
///
/// Exposed publicly so callers can read the cap when validating their own
/// inputs; the builder enforces it in [`MatrixConfigBuilder::build`]. Set to
/// `1024` — already an order of magnitude beyond any realistic terminal
/// height; the cap exists to prevent accidental gigabyte glyph buffers from
/// typos rather than to enforce a stylistic constraint.
pub const MAX_TRAIL_LIMIT: u16 = 1024;

fn invalid(msg: &str) -> MatrixError {
    MatrixError::InvalidConfig(msg.to_string())
}

impl Default for MatrixConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let cfg = MatrixConfig::default();
        assert_eq!(cfg.fps, 30);
        assert_eq!(cfg.speed, 1.0);
        assert_eq!(cfg.density, 0.6);
        assert!(cfg.bold_head);
        assert!(cfg.head_white);
        assert!(matches!(cfg.charset, CharSet::Matrix));
        assert!(matches!(cfg.theme, Theme::ClassicGreen));
        assert_eq!(cfg.glitch, 0.0);
        assert_eq!(cfg.background, None);
        assert!(cfg.min_trail >= 1);
        assert!(cfg.max_trail >= cfg.min_trail);
    }

    #[test]
    fn builder_chains_overrides() {
        let cfg = MatrixConfig::builder()
            .fps(60)
            .density(0.3)
            .bold_head(false)
            .build()
            .expect("build should succeed");
        assert_eq!(cfg.fps, 60);
        assert_eq!(cfg.density, 0.3);
        assert!(!cfg.bold_head);
        assert!(cfg.head_white, "untouched fields keep defaults");
    }

    #[test]
    fn builder_default_round_trip() {
        let cfg = MatrixConfig::builder().build().unwrap();
        let default = MatrixConfig::default();
        assert_eq!(cfg.fps, default.fps);
        assert_eq!(cfg.speed, default.speed);
        assert_eq!(cfg.density, default.density);
        assert_eq!(cfg.min_trail, default.min_trail);
        assert_eq!(cfg.max_trail, default.max_trail);
    }

    #[test]
    fn build_rejects_empty_custom_charset() {
        let err = MatrixConfig::builder()
            .charset(CharSet::Custom(vec![]))
            .build()
            .unwrap_err();
        assert!(matches!(err, MatrixError::EmptyCharset));
    }

    #[test]
    fn build_rejects_control_chars_in_custom() {
        let err = MatrixConfig::builder()
            .charset(CharSet::Custom(vec!['a', '\n']))
            .build()
            .unwrap_err();
        assert!(matches!(err, MatrixError::InvalidConfig(_)));
    }

    fn invalid_err(r: Result<MatrixConfig, MatrixError>, expected_keyword: &str) {
        match r {
            Err(MatrixError::InvalidConfig(msg)) => assert!(
                msg.contains(expected_keyword),
                "expected '{expected_keyword}' in error, got: {msg}"
            ),
            other => panic!("expected InvalidConfig containing '{expected_keyword}', got {other:?}"),
        }
    }

    #[test]
    fn build_rejects_fps_zero() {
        invalid_err(MatrixConfig::builder().fps(0).build(), "fps");
    }

    #[test]
    fn build_rejects_speed_zero() {
        invalid_err(MatrixConfig::builder().speed(0.0).build(), "speed");
    }

    #[test]
    fn build_rejects_speed_negative() {
        invalid_err(MatrixConfig::builder().speed(-0.5).build(), "speed");
    }

    #[test]
    fn build_rejects_speed_nan() {
        invalid_err(MatrixConfig::builder().speed(f32::NAN).build(), "speed");
    }

    #[test]
    fn build_rejects_speed_infinite() {
        invalid_err(MatrixConfig::builder().speed(f32::INFINITY).build(), "speed");
    }

    #[test]
    fn build_rejects_density_above_one() {
        invalid_err(MatrixConfig::builder().density(1.1).build(), "density");
    }

    #[test]
    fn build_rejects_density_negative() {
        invalid_err(MatrixConfig::builder().density(-0.1).build(), "density");
    }

    #[test]
    fn build_rejects_density_nan() {
        invalid_err(MatrixConfig::builder().density(f32::NAN).build(), "density");
    }

    #[test]
    fn build_rejects_min_trail_zero() {
        invalid_err(MatrixConfig::builder().min_trail(0).build(), "min_trail");
    }

    #[test]
    fn build_rejects_min_greater_than_max_trail() {
        invalid_err(
            MatrixConfig::builder().min_trail(10).max_trail(5).build(),
            "min_trail",
        );
    }

    #[test]
    fn build_rejects_max_trail_above_limit() {
        invalid_err(
            MatrixConfig::builder()
                .min_trail(1)
                .max_trail(MAX_TRAIL_LIMIT + 1)
                .build(),
            "max_trail",
        );
    }

    #[test]
    fn build_rejects_mutation_rate_above_one() {
        invalid_err(
            MatrixConfig::builder().mutation_rate(1.1).build(),
            "mutation_rate",
        );
    }

    #[test]
    fn build_rejects_mutation_rate_negative() {
        invalid_err(
            MatrixConfig::builder().mutation_rate(-0.1).build(),
            "mutation_rate",
        );
    }

    #[test]
    fn build_rejects_mutation_rate_nan() {
        invalid_err(
            MatrixConfig::builder().mutation_rate(f32::NAN).build(),
            "mutation_rate",
        );
    }

    #[test]
    fn build_rejects_glitch_above_one() {
        invalid_err(MatrixConfig::builder().glitch(1.1).build(), "glitch");
    }

    #[test]
    fn build_rejects_glitch_negative() {
        invalid_err(MatrixConfig::builder().glitch(-0.1).build(), "glitch");
    }

    #[test]
    fn build_rejects_glitch_nan() {
        invalid_err(MatrixConfig::builder().glitch(f32::NAN).build(), "glitch");
    }

    #[test]
    fn build_accepts_density_boundaries() {
        assert!(MatrixConfig::builder().density(0.0).build().is_ok());
        assert!(MatrixConfig::builder().density(1.0).build().is_ok());
    }

    #[test]
    fn build_accepts_min_equals_max_trail() {
        assert!(MatrixConfig::builder()
            .min_trail(5)
            .max_trail(5)
            .build()
            .is_ok());
    }

    #[test]
    fn build_accepts_mutation_rate_boundaries() {
        assert!(MatrixConfig::builder().mutation_rate(0.0).build().is_ok());
        assert!(MatrixConfig::builder().mutation_rate(1.0).build().is_ok());
    }

    #[test]
    fn build_accepts_glitch_boundaries() {
        assert!(MatrixConfig::builder().glitch(0.0).build().is_ok());
        assert!(MatrixConfig::builder().glitch(1.0).build().is_ok());
    }

    #[test]
    fn build_accepts_max_trail_at_limit() {
        assert!(MatrixConfig::builder()
            .min_trail(1)
            .max_trail(MAX_TRAIL_LIMIT)
            .build()
            .is_ok());
    }
}

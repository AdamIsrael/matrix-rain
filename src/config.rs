use ratatui::style::Color;

use crate::charset::CharSet;
use crate::error::MatrixError;
use crate::theme::Theme;

#[derive(Clone, Debug)]
pub struct MatrixConfig {
    pub charset: CharSet,
    pub theme: Theme,
    pub fps: u16,
    pub speed: f32,
    pub density: f32,
    pub min_trail: u16,
    pub max_trail: u16,
    pub mutation_rate: f32,
    pub bold_head: bool,
    pub head_white: bool,
    pub glitch: f32,
    pub background: Option<Color>,
}

impl MatrixConfig {
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
pub struct MatrixConfigBuilder {
    config: MatrixConfig,
}

impl MatrixConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: MatrixConfig::default(),
        }
    }

    pub fn charset(mut self, charset: CharSet) -> Self {
        self.config.charset = charset;
        self
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.config.theme = theme;
        self
    }

    pub fn fps(mut self, fps: u16) -> Self {
        self.config.fps = fps;
        self
    }

    pub fn speed(mut self, speed: f32) -> Self {
        self.config.speed = speed;
        self
    }

    pub fn density(mut self, density: f32) -> Self {
        self.config.density = density;
        self
    }

    pub fn min_trail(mut self, min_trail: u16) -> Self {
        self.config.min_trail = min_trail;
        self
    }

    pub fn max_trail(mut self, max_trail: u16) -> Self {
        self.config.max_trail = max_trail;
        self
    }

    pub fn mutation_rate(mut self, mutation_rate: f32) -> Self {
        self.config.mutation_rate = mutation_rate;
        self
    }

    pub fn bold_head(mut self, bold_head: bool) -> Self {
        self.config.bold_head = bold_head;
        self
    }

    pub fn head_white(mut self, head_white: bool) -> Self {
        self.config.head_white = head_white;
        self
    }

    pub fn glitch(mut self, glitch: f32) -> Self {
        self.config.glitch = glitch;
        self
    }

    pub fn background(mut self, background: Option<Color>) -> Self {
        self.config.background = background;
        self
    }

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

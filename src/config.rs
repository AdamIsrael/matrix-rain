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
        Ok(self.config)
    }
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
}

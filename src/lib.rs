mod charset;
mod config;
mod error;
mod state;
mod stream;
mod theme;
mod widget;

pub use charset::CharSet;
pub use config::{MatrixConfig, MatrixConfigBuilder};
pub use error::MatrixError;
pub use state::MatrixRainState;
pub use theme::{ColorRamp, Theme};
pub use widget::MatrixRain;

//! Error types returned by the builder and config validation.

use thiserror::Error;

/// Errors produced by [`MatrixConfigBuilder::build`](crate::MatrixConfigBuilder::build)
/// and related validation routines.
#[derive(Debug, Error)]
pub enum MatrixError {
    /// A configuration value failed its invariant check (e.g. `fps < 1`,
    /// non-finite `speed`, `density` outside `[0.0, 1.0]`, `min_trail > max_trail`).
    /// The string carries a human-readable description of which field and why.
    #[error("invalid configuration: {0}")]
    InvalidConfig(/// Reason the config was rejected.
        String),

    /// The configured [`CharSet`](crate::CharSet) resolves to zero glyphs.
    /// Only [`CharSet::Custom`](crate::CharSet::Custom) with an empty `Vec`
    /// can hit this — the built-in variants always resolve to non-empty
    /// lists.
    #[error("empty character set")]
    EmptyCharset,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_config_renders_message() {
        let e = MatrixError::InvalidConfig("fps must be >= 1".into());
        assert_eq!(e.to_string(), "invalid configuration: fps must be >= 1");
    }

    #[test]
    fn empty_charset_renders_message() {
        assert_eq!(MatrixError::EmptyCharset.to_string(), "empty character set");
    }
}

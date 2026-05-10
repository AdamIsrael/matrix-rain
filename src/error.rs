use thiserror::Error;

#[derive(Debug, Error)]
pub enum MatrixError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
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

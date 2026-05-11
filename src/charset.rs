//! Built-in glyph sets and the [`CharSet`] enum for supplying your own.

use alloc::format;
use alloc::vec::Vec;

use crate::error::MatrixError;

/// Source of glyphs for the falling drops.
///
/// Each variant resolves to a slice of single-cell characters drawn from
/// randomly per cell at spawn (and per cell per frame when `mutation_rate > 0`).
///
/// # Example
///
/// ```
/// use matrix_rain::{CharSet, MatrixConfig};
///
/// let cfg = MatrixConfig::builder()
///     .charset(CharSet::Hex)
///     .build()
///     .unwrap();
/// assert!(matches!(cfg.charset, CharSet::Hex));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CharSet {
    /// Half-width katakana `U+FF66..=U+FF9D` plus digits `0..=9` (66 glyphs).
    /// The canonical Matrix look.
    Matrix,
    /// Printable ASCII `0x21..=0x7E` (94 glyphs; space is excluded so the
    /// rain stays visually dense).
    Ascii,
    /// Lowercase hexadecimal: `0`–`9` and `a`–`f` (16 glyphs).
    Hex,
    /// Just `0` and `1`.
    Binary,
    /// User-supplied glyph list. Validated at
    /// [`MatrixConfigBuilder::build`](crate::MatrixConfigBuilder::build) time:
    /// must be non-empty and free of `char::is_control` characters. Display
    /// width is **not** validated — see the crate-level Caveats.
    Custom(/// Glyphs to draw from.
        Vec<char>),
}

impl CharSet {
    pub(crate) fn chars(&self) -> &[char] {
        match self {
            Self::Matrix => MATRIX_CHARS,
            Self::Ascii => ASCII_CHARS,
            Self::Hex => HEX_CHARS,
            Self::Binary => BINARY_CHARS,
            Self::Custom(v) => v.as_slice(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), MatrixError> {
        let chars = self.chars();
        if chars.is_empty() {
            return Err(MatrixError::EmptyCharset);
        }
        for c in chars {
            if c.is_control() {
                return Err(MatrixError::InvalidConfig(format!(
                    "charset contains control character U+{:04X}",
                    *c as u32
                )));
            }
        }
        Ok(())
    }
}

const MATRIX_CHARS: &[char] = &[
    'ｦ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｬ', 'ｭ', 'ｮ', 'ｯ',
    'ｰ', 'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ',
    'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ', 'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ',
    'ﾄ', 'ﾅ', 'ﾆ', 'ﾇ', 'ﾈ', 'ﾉ', 'ﾊ', 'ﾋ', 'ﾌ', 'ﾍ',
    'ﾎ', 'ﾏ', 'ﾐ', 'ﾑ', 'ﾒ', 'ﾓ', 'ﾔ', 'ﾕ', 'ﾖ', 'ﾗ',
    'ﾘ', 'ﾙ', 'ﾚ', 'ﾛ', 'ﾜ', 'ﾝ', '0', '1', '2', '3',
    '4', '5', '6', '7', '8', '9',
];

const ASCII_CHARS: &[char] = &[
    '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*',
    '+', ',', '-', '.', '/', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', ':', ';', '<', '=', '>',
    '?', '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
    'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
    'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\',
    ']', '^', '_', '`', 'a', 'b', 'c', 'd', 'e', 'f',
    'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
    'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    '{', '|', '}', '~',
];

const HEX_CHARS: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

const BINARY_CHARS: &[char] = &['0', '1'];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_chars_non_empty() {
        assert!(!CharSet::Matrix.chars().is_empty());
    }

    #[test]
    fn matrix_chars_include_all_digits() {
        let chars = CharSet::Matrix.chars();
        for d in '0'..='9' {
            assert!(chars.contains(&d), "Matrix charset missing digit {d}");
        }
    }

    #[test]
    fn matrix_chars_include_katakana() {
        let chars = CharSet::Matrix.chars();
        assert!(chars.contains(&'ｦ'));
        assert!(chars.contains(&'ﾝ'));
    }

    #[test]
    fn ascii_chars_exclude_space_and_control() {
        let chars = CharSet::Ascii.chars();
        assert!(!chars.is_empty());
        assert!(!chars.contains(&' '));
        assert!(!chars.contains(&'\n'));
        assert!(!chars.contains(&'\t'));
        assert!(chars.contains(&'!'));
        assert!(chars.contains(&'~'));
        assert!(chars.contains(&'A'));
        assert!(chars.contains(&'z'));
        assert!(chars.contains(&'0'));
    }

    #[test]
    fn hex_chars_are_digits_and_lower_af() {
        let chars = CharSet::Hex.chars();
        assert_eq!(chars.len(), 16);
        for d in '0'..='9' {
            assert!(chars.contains(&d));
        }
        for d in 'a'..='f' {
            assert!(chars.contains(&d));
        }
        // No uppercase per spec ("0–9 a–f").
        assert!(!chars.contains(&'A'));
    }

    #[test]
    fn binary_chars_are_zero_and_one() {
        assert_eq!(CharSet::Binary.chars(), &['0', '1']);
    }

    #[test]
    fn custom_passthrough() {
        let cs = CharSet::Custom(vec!['a', 'b', 'c']);
        assert_eq!(cs.chars(), &['a', 'b', 'c']);
    }

    #[test]
    fn validate_passes_for_all_builtins() {
        for cs in [CharSet::Matrix, CharSet::Ascii, CharSet::Hex, CharSet::Binary] {
            assert!(cs.validate().is_ok(), "{cs:?} should validate");
        }
    }

    #[test]
    fn validate_rejects_empty_custom() {
        let err = CharSet::Custom(vec![]).validate().unwrap_err();
        assert!(matches!(err, MatrixError::EmptyCharset));
    }

    #[test]
    fn validate_rejects_control_chars() {
        for bad in ['\n', '\r', '\t', '\0', '\x07'] {
            let err = CharSet::Custom(vec!['a', bad, 'b']).validate().unwrap_err();
            assert!(
                matches!(err, MatrixError::InvalidConfig(_)),
                "control char {bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn validate_accepts_single_char_custom() {
        assert!(CharSet::Custom(vec!['x']).validate().is_ok());
    }

    #[test]
    fn validate_does_not_check_display_width() {
        // Full-width / combining chars are NOT detected per spec §5.4 — caller's responsibility.
        // Just confirm validation passes for one example so the test documents the policy.
        assert!(CharSet::Custom(vec!['漢']).validate().is_ok());
    }
}

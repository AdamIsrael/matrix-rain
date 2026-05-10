#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CharSet {
    Matrix,
    Ascii,
    Hex,
    Binary,
    Custom(Vec<char>),
}

impl CharSet {
    pub(crate) fn chars(&self) -> &[char] {
        match self {
            Self::Matrix => MATRIX_CHARS,
            Self::Custom(v) => v.as_slice(),
            Self::Ascii | Self::Hex | Self::Binary => unimplemented!(
                "CharSet::{:?} is not implemented until 0.2.0 (matrix-ftw.1)",
                self
            ),
        }
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
    fn custom_passthrough() {
        let cs = CharSet::Custom(vec!['a', 'b', 'c']);
        assert_eq!(cs.chars(), &['a', 'b', 'c']);
    }

    #[test]
    #[should_panic(expected = "0.2.0")]
    fn ascii_not_yet_implemented() {
        let _ = CharSet::Ascii.chars();
    }
}

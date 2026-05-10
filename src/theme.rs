use ratatui::style::Color;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Theme {
    ClassicGreen,
    Amber,
    Cyan,
    Red,
    Rainbow,
    Custom(ColorRamp),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorRamp {
    pub head: Color,
    pub bright: Color,
    pub mid: Color,
    pub dim: Color,
    pub fade: Color,
}

const CLASSIC_GREEN: ColorRamp = ColorRamp {
    head: Color::Rgb(0xFF, 0xFF, 0xFF),
    bright: Color::Rgb(0xCC, 0xFF, 0xCC),
    mid: Color::Rgb(0x00, 0xFF, 0x00),
    dim: Color::Rgb(0x00, 0x99, 0x00),
    fade: Color::Rgb(0x00, 0x33, 0x00),
};

impl Theme {
    pub(crate) fn ramp(&self) -> ColorRamp {
        match self {
            Self::ClassicGreen => CLASSIC_GREEN,
            Self::Custom(ramp) => *ramp,
            Self::Amber | Self::Cyan | Self::Red | Self::Rainbow => unimplemented!(
                "Theme::{:?} is not implemented until 0.2.0 (matrix-ftw.3)",
                self
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_green_ramp_has_white_head() {
        let ramp = Theme::ClassicGreen.ramp();
        assert_eq!(ramp.head, Color::Rgb(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn classic_green_ramp_distinct_stops() {
        let r = Theme::ClassicGreen.ramp();
        let stops = [r.head, r.bright, r.mid, r.dim, r.fade];
        for i in 0..stops.len() {
            for j in (i + 1)..stops.len() {
                assert_ne!(stops[i], stops[j], "stops {i} and {j} collide");
            }
        }
    }

    #[test]
    fn custom_passthrough() {
        let ramp = ColorRamp {
            head: Color::Red,
            bright: Color::LightRed,
            mid: Color::Yellow,
            dim: Color::DarkGray,
            fade: Color::Black,
        };
        assert_eq!(Theme::Custom(ramp).ramp(), ramp);
    }

    #[test]
    #[should_panic(expected = "0.2.0")]
    fn amber_not_yet_implemented() {
        let _ = Theme::Amber.ramp();
    }
}

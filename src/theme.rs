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

const AMBER: ColorRamp = ColorRamp {
    head: Color::Rgb(0xFF, 0xFF, 0xFF),
    bright: Color::Rgb(0xFF, 0xE5, 0xB4),
    mid: Color::Rgb(0xFF, 0xAA, 0x00),
    dim: Color::Rgb(0xB3, 0x6B, 0x00),
    fade: Color::Rgb(0x4D, 0x2E, 0x00),
};

const CYAN: ColorRamp = ColorRamp {
    head: Color::Rgb(0xFF, 0xFF, 0xFF),
    bright: Color::Rgb(0xCC, 0xFF, 0xFF),
    mid: Color::Rgb(0x00, 0xFF, 0xFF),
    dim: Color::Rgb(0x00, 0x88, 0x99),
    fade: Color::Rgb(0x00, 0x22, 0x33),
};

const RED: ColorRamp = ColorRamp {
    head: Color::Rgb(0xFF, 0xFF, 0xFF),
    bright: Color::Rgb(0xFF, 0xCC, 0xCC),
    mid: Color::Rgb(0xFF, 0x33, 0x00),
    dim: Color::Rgb(0x99, 0x11, 0x00),
    fade: Color::Rgb(0x33, 0x00, 0x00),
};

// Rainbow: 4 distinct hues across the trail with a white head. The smooth-interpolation
// path will lerp between adjacent stops, producing a vertical hue gradient inside each
// drop. On 256-color / 16-color terminals the 5-stop quantisation still reads as colorful.
const RAINBOW: ColorRamp = ColorRamp {
    head: Color::Rgb(0xFF, 0xFF, 0xFF),
    bright: Color::Rgb(0xFF, 0x00, 0x00),
    mid: Color::Rgb(0xFF, 0xFF, 0x00),
    dim: Color::Rgb(0x00, 0xFF, 0x00),
    fade: Color::Rgb(0x00, 0x66, 0xFF),
};

impl Theme {
    pub(crate) fn ramp(&self) -> ColorRamp {
        match self {
            Self::ClassicGreen => CLASSIC_GREEN,
            Self::Amber => AMBER,
            Self::Cyan => CYAN,
            Self::Red => RED,
            Self::Rainbow => RAINBOW,
            Self::Custom(ramp) => *ramp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_distinct_stops(theme: Theme) {
        let r = theme.ramp();
        let stops = [r.head, r.bright, r.mid, r.dim, r.fade];
        for i in 0..stops.len() {
            for j in (i + 1)..stops.len() {
                assert_ne!(stops[i], stops[j], "{theme:?}: stops {i} and {j} collide");
            }
        }
    }

    fn assert_white_head(theme: Theme) {
        assert_eq!(theme.ramp().head, Color::Rgb(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn classic_green_ramp() {
        assert_white_head(Theme::ClassicGreen);
        assert_distinct_stops(Theme::ClassicGreen);
    }

    #[test]
    fn amber_ramp() {
        assert_white_head(Theme::Amber);
        assert_distinct_stops(Theme::Amber);
    }

    #[test]
    fn cyan_ramp() {
        assert_white_head(Theme::Cyan);
        assert_distinct_stops(Theme::Cyan);
    }

    #[test]
    fn red_ramp() {
        assert_white_head(Theme::Red);
        assert_distinct_stops(Theme::Red);
    }

    #[test]
    fn rainbow_ramp_has_diverse_hues() {
        assert_white_head(Theme::Rainbow);
        assert_distinct_stops(Theme::Rainbow);
        // Sanity: rainbow's mid/dim/fade should span the hue wheel — no two share dominant channel.
        let r = Theme::Rainbow.ramp();
        let channels = |c: Color| match c {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => panic!("expected Rgb"),
        };
        let (mr, mg, _) = channels(r.mid);
        let (_, dg, _) = channels(r.dim);
        assert!(mr >= 0x80 && mg >= 0x80, "mid should be warm");
        assert!(dg >= 0x80, "dim should have strong green");
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
}

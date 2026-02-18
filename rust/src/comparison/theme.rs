/// This module contains some helpers which help theming the outputs.

use catppuccin::{ColorName, Flavor, PALETTE};
use plotly::common::{ColorScale, ColorScaleElement};

pub(super) fn lut_theme(theme: u8) -> Flavor {
    match theme {
        1 => PALETTE.latte,
        2 => PALETTE.frappe,
        3 => PALETTE.macchiato,
        4 => PALETTE.mocha,

        _ => PALETTE.frappe,
    }
}

pub(super) fn plotly_colorscale(_palette: &Flavor) -> ColorScale {
    let palette = PALETTE.latte;
    ColorScale::Vector(vec![
        ColorScaleElement(0.00, palette.get_color(ColorName::Green).hex.to_string()),
        ColorScaleElement(0.70, palette.get_color(ColorName::Peach).hex.to_string()),
        ColorScaleElement(
            1.00,
            palette
                .get_color(ColorName::Red)
                .hex
                .to_string()
                .to_string(),
        ),
    ])
}

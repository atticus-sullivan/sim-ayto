use catppuccin::{Flavor, PALETTE};

pub fn lut_theme(theme: u8) -> Flavor {
    match theme {
        1 => PALETTE.latte,
        2 => PALETTE.frappe,
        3 => PALETTE.macchiato,
        4 => PALETTE.mocha,

        _ => PALETTE.frappe,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lut_theme_maps_known_values() {
        // theme 1..4 expected to return different flavors
        let f1 = lut_theme(1);
        let f2 = lut_theme(2);
        assert_ne!(f1.colors.base.hex, f2.colors.base.hex);
    }
}

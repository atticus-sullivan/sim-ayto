pub mod comparison;
pub mod game;

mod constraint;
mod iterstate;
mod ruleset;
mod ruleset_data;
mod tree;

use std::collections::HashMap;

use comfy_table::Color;

type MatchingS = HashMap<String, Vec<String>>;
type Matching = Vec<Vec<u8>>;
type MapS = HashMap<String, String>;
type Map = HashMap<u8, u8>;
type Lut = HashMap<String, usize>;
type Rename = HashMap<String, String>;

type Rem = (Vec<Vec<u128>>, u128);

// colors for tables
const COLOR_ROW_MAX: Color = Color::Rgb {
    r: 69,
    g: 76,
    b: 102,
};
const COLOR_BOTH_MAX: Color = Color::Rgb {
    r: 65,
    g: 77,
    b: 71,
};
const COLOR_COL_MAX: Color = Color::Rgb {
    r: 74,
    g: 68,
    b: 89,
};

pub const COLOR_ALT_BG: Color = Color::Rgb {
    r: 41,
    g: 44,
    b: 60,
};

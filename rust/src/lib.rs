// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This is the crate's root.
//! It also defines some widely used type aliases and constants.

pub mod comparison;
pub mod constraint;
pub mod dump_mode;
pub mod game;
pub mod ignore_ops;
pub mod iterstate;
pub mod matching_repr;
pub mod progressbar;
pub mod ruleset;
pub mod ruleset_data;
pub mod tree;

use std::collections::HashMap;

use comfy_table::Color;

use crate::matching_repr::IdBase;

/// A type for matchings stored with the names as strings
type MatchingS = HashMap<String, Vec<String>>;

/// A type for storing matchings how they are deserialized from yaml with strings
type MapS = HashMap<String, String>;
/// Store a matching with ids already, but still as hashmap so the access to the raw ids is easier
pub type Map = HashMap<IdBase, IdBase>;

/// A type for lookup tables (name -> id), for the other way round a simple vector is sufficient
type Lut = HashMap<String, usize>;

/// A type to store rename mappings (old-name to new-name)
type Rename = HashMap<String, String>;

/// A type for the amount of 1:1 matchings in all remaining solutions
/// (table of 1:1 matchings, total amout of remaining solutions)
pub type Rem = (Vec<Vec<u128>>, u128);

/// A type for the amount of lights
pub type LightCnt = u8;

// colors for tables
/// color to be used for the maximum value in the row
const COLOR_ROW_MAX: Color = Color::Rgb {
    r: 69,
    g: 76,
    b: 102,
};
/// color to be used if the maximum is the maximum for both, the row as well as the column
const COLOR_BOTH_MAX: Color = Color::Rgb {
    r: 65,
    g: 77,
    b: 71,
};
/// color to be used for the maximum value in the column
const COLOR_COL_MAX: Color = Color::Rgb {
    r: 74,
    g: 68,
    b: 89,
};

/// color to be used for alternating table rows for easier following the lines with the eye
pub const COLOR_ALT_BG: Color = Color::Rgb {
    r: 41,
    g: 44,
    b: 60,
};

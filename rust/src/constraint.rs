/*
sim_ayto
Copyright (C) 2024  Lukas Heindl

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

pub mod eval;
pub mod parse;
mod utils;

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::ruleset_data::RuleSetData;
use crate::{Lut, Map, MapS, Matching};

#[derive(Deserialize, Debug, Clone, Hash)]
enum CheckType {
    Eq,
    Nothing,
    Sold,
    Lights(u8, #[serde(skip)] BTreeMap<u8, u128>),
}

impl CheckType {
    pub fn as_lights(&self) -> Option<u8> {
        if let CheckType::Lights(l, _) = *self {
            Some(l)
        } else {
            None
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
enum Offer {
    Single{
        amount: f32,
        by: String,
        #[serde(rename="reducedPot")]
        reduced_pot: bool,
        save: bool,
    },
    All{amount: f32, by: String},
}

#[derive(Deserialize, Debug, Clone)]
enum ConstraintType {
    Night { num: f32, comment: String },
    Box { num: f32, comment: String, offer: Option<Offer> },
}

impl Hash for ConstraintType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ConstraintType::Night { num, comment } => {
                0.hash(state); // A constant to distinguish this variant
                num.to_bits().hash(state);
                comment.hash(state);
            }
            ConstraintType::Box { num, comment , offer} => {
                1.hash(state); // A constant to distinguish this variant
                num.to_bits().hash(state);
                comment.hash(state);
                offer.hash(state)
            }
        }
    }
}
impl Hash for Offer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Offer::Single { amount, by, reduced_pot, save } => {
                0.hash(state); // A constant to distinguish this variant
                amount.to_bits().hash(state);
                by.hash(state);
                reduced_pot.hash(state);
                save.hash(state);
            },
            Offer::All { amount, by } => {
                1.hash(state); // A constant to distinguish this variant
                amount.to_bits().hash(state);
                by.hash(state);
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Constraint {
    r#type: ConstraintType,
    check: CheckType,
    hidden: bool,
    result_unknown: bool,
    build_tree: bool,

    map: Map,
    map_s: MapS,
    exclude: Option<(u8, HashSet<u8>)>,
    eliminated: u128,
    eliminated_tab: Vec<Vec<u128>>,

    information: Option<f64>,
    left_after: Option<u128>,
    left_poss: Vec<Matching>,

    hide_ruleset_data: bool,
    pub ruleset_data: Box<dyn RuleSetData>,
    known_lights: u8,
}

// functions for initialization / startup
impl Constraint {
    /// Sorts and key/value pairs such that lut_a[k] < lut_b[v] always holds.
    /// Only makes sense if lut_a == lut_b (defined on the same set)
    ///
    /// # Arguments
    ///
    /// - `lut_a`: A lookup table of type `Lut` used for value comparison with `self.map_s`.
    /// - `lut_b`: A lookup table of type `Lut` used for value comparison with `self.map_s`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut c: Constraint;
    /// c.sort_maps(&lut_a, &lut_b);
    /// ```
    ///
    /// # Panics
    ///
    /// This function may panic if `lut_a` or `lut_b` do not contain keys present in `self.map_s`.
    ///
    /// # Notes
    ///
    /// - The sorting and flipping operations are done in place.
    fn sort_maps(&mut self, lut_a: &Lut, lut_b: &Lut) {
        self.map = self
            .map
            .drain()
            .map(|(k, v)| if k < v { (v, k) } else { (k, v) })
            .collect();

        self.map_s = self
            .map_s
            .drain()
            .map(|(k, v)| {
                if lut_a[&k] < lut_b[&v] {
                    (v, k)
                } else {
                    (k, v)
                }
            })
            .collect();
    }
}

// functions for processing/executing the simulation
impl Constraint {
    // returns if the matching fits the constraint (is not eliminated)
    pub fn process(&mut self, m: &Matching) -> Result<bool> {
        // first step is to check if the constraint filters out this matching
        let mut fits = None;
        match &mut self.check {
            CheckType::Eq => {
                fits = Some(m.iter().all(|js| {
                    self.map
                        .values()
                        .map(|i2| js.contains(i2))
                        .fold(None, |acc, b| match acc {
                            Some(a) => Some(a == b),
                            None => Some(b),
                        })
                        .unwrap()
                }));
            }
            CheckType::Nothing | CheckType::Sold => fits = Some(true),
            CheckType::Lights(ref lights, ref mut light_count) => {
                let mut l = 0;
                for (i1, i2) in self.map.iter() {
                    if m[*i1 as usize].contains(i2) {
                        l += 1;
                    }
                }
                if let Some(ex) = &self.exclude {
                    for i in &m[ex.0 as usize] {
                        if ex.1.contains(i) {
                            fits = Some(false);
                            break;
                        }
                    }
                }
                // might be already set due to exclude
                fits.get_or_insert(l == *lights);
                // use calculated lights to collect stats on based on the matching possible until
                // here, how many lights are calculated how often for this map
                *light_count.entry(l).or_insert(0) += 1;
            }
        };

        // check fits actually has a value and make it immutable
        let fits = fits.with_context(|| {
            format!(
                "failure in calculating wether matching {:?} fits constraint {:?}",
                m, self
            )
        })? || self.result_unknown;

        if !fits {
            self.eliminate(m);
        } else {
            if self.build_tree && !self.hidden {
                self.left_poss.push(m.clone());
            }
            if !self.hide_ruleset_data && !self.hidden {
                self.ruleset_data.push(m)?;
            }
        }

        Ok(fits)
    }
}

// getter functions
impl Constraint {
    pub fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { comment, ..} => comment,
            ConstraintType::Box { comment, .. } => comment,
        }
    }

    pub fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, .. } => format!("MN#{}", num),
            ConstraintType::Box { num, .. } => format!("MB#{}", num),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruleset_data::dummy::DummyData;
    use crate::Rem;
    use std::collections::HashMap;

    #[test]
    fn test_sort_maps_basic() {
        let mut constraint = Constraint {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
                offer: None,
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            information: None,
            left_after: None,
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map.insert(1, 0);
        constraint.map.insert(2, 3);

        constraint.map_s.insert("B".to_string(), "A".to_string());
        constraint.map_s.insert("C".to_string(), "D".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(
            vec![
                ("A".to_string(), 0),
                ("B".to_string(), 1),
                ("C".to_string(), 2),
                ("D".to_string(), 3),
            ]
            .into_iter(),
        );
        let lut_b = lut_a.clone();

        // Perform sorting
        constraint.sort_maps(&lut_a, &lut_b);

        // Validate the map is sorted and flipped correctly
        assert_eq!(*constraint.map.get(&1).unwrap(), 0);
        assert_eq!(*constraint.map.get(&3).unwrap(), 2);

        // Validate map_s is sorted and flipped correctly according to the LUTs
        assert_eq!(constraint.map_s.get("B").unwrap(), "A");
        assert_eq!(constraint.map_s.get("D").unwrap(), "C");
    }

    #[test]
    fn test_sort_maps_no_flipping_needed() {
        let mut constraint = Constraint {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
                offer: None,
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            information: None,
            left_after: None,
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map.insert(1, 0);
        constraint.map.insert(3, 2);

        constraint.map_s.insert("B".to_string(), "A".to_string());
        constraint.map_s.insert("D".to_string(), "C".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(
            vec![
                ("A".to_string(), 0),
                ("B".to_string(), 1),
                ("C".to_string(), 2),
                ("D".to_string(), 3),
            ]
            .into_iter(),
        );
        let lut_b = lut_a.clone();

        // Perform sorting
        constraint.sort_maps(&lut_a, &lut_b);

        // Validate the map is sorted and flipped correctly
        assert_eq!(*constraint.map.get(&1).unwrap(), 0);
        assert_eq!(*constraint.map.get(&3).unwrap(), 2);

        // Validate map_s is sorted and flipped correctly according to the LUTs
        assert_eq!(constraint.map_s.get("B").unwrap(), "A");
        assert_eq!(constraint.map_s.get("D").unwrap(), "C");
    }

    #[test]
    #[should_panic]
    fn test_sort_maps_panic_on_missing_lut_keys() {
        let mut constraint = Constraint {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
                offer: None,
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            information: None,
            left_after: None,
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        constraint.map.insert(1, 0);
        // Initialize the map_s with keys not present in lut_a or lut_b
        constraint
            .map_s
            .insert("unknown".to_string(), "value".to_string());

        // Initialize lookup tables with different keys
        let lut_a = HashMap::new();
        let lut_b = HashMap::new();

        // Perform sorting (should panic due to missing keys)
        constraint.sort_maps(&lut_a, &lut_b);
    }

    fn constraint_def() -> Constraint {
        Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        }
    }

    #[test]
    fn test_process_light() {
        let mut c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(!c.process(&m).unwrap());
        // change amount of lights
        match &mut c.check {
            CheckType::Eq => {}
            CheckType::Nothing | CheckType::Sold => {}
            CheckType::Lights(l, _) => *l = 1,
        }
        assert!(c.process(&m).unwrap());
    }

    #[test]
    fn test_process_light_exclude() {
        let mut c = Constraint {
            result_unknown: false,
            exclude: Some((0, HashSet::from([2, 3]))),
            map_s: HashMap::new(),
            check: CheckType::Lights(1, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from(""),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(c.process(&m).unwrap());

        let m: Matching = vec![vec![0], vec![1], vec![2, 3], vec![4]];
        assert!(!c.process(&m).unwrap());

        let m: Matching = vec![vec![0, 2], vec![1], vec![4], vec![3]];
        assert!(!c.process(&m).unwrap());
    }

    #[test]
    fn test_process_eq() {
        let mut c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Eq,
            // 1 and 2 have the same match
            map: HashMap::from([(0, 1), (1, 2)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from(""),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(!c.process(&m).unwrap());
        let m: Matching = vec![vec![0], vec![1, 2], vec![3], vec![4]];
        assert!(c.process(&m).unwrap());
    }

    #[test]
    fn test_eliminate() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];

        c.eliminate(&m);
        assert_eq!(c.eliminated, 1);
        assert_eq!(
            c.eliminated_tab,
            vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 0, 0],
                vec![0, 0, 1, 0, 0],
                vec![0, 0, 0, 1, 1]
            ]
        );

        c.eliminate(&m);
        assert_eq!(c.eliminated, 2);
        assert_eq!(
            c.eliminated_tab,
            vec![
                vec![2, 0, 0, 0, 0],
                vec![0, 2, 0, 0, 0],
                vec![0, 0, 2, 0, 0],
                vec![0, 0, 0, 2, 2]
            ]
        );
    }

    #[test]
    fn test_apply() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];

        c.eliminate(&m);
        assert_eq!(c.eliminated, 1);

        let mut rem: Rem = (vec![vec![15; 5]; 4], 5 * 4 * 3 * 2 * 1 * 4 / 2);

        rem = c.apply_to_rem(rem).unwrap();
        assert_eq!(rem.1, 5 * 4 * 3 * 2 * 1 * 4 / 2 - 1);
        assert_eq!(
            rem.0,
            vec![
                vec![14, 15, 15, 15, 15],
                vec![15, 14, 15, 15, 15],
                vec![15, 15, 14, 15, 15],
                vec![15, 15, 15, 14, 14]
            ]
        );
    }

    #[test]
    fn test_merge() {
        let mut c_a = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 200,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(4.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };
        let c_b = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        c_a.merge(&c_b).unwrap();

        assert_eq!(c_a.eliminated, 300);

        assert_eq!(
            c_a.eliminated_tab,
            vec![
                vec![2, 0, 0, 0, 0],
                vec![0, 2, 0, 6, 0],
                vec![0, 0, 4, 0, 6],
                vec![0, 12, 0, 10, 0],
            ]
        );

        assert_eq!(c_a.information, None);
        assert_eq!(c_a.left_after, None);
    }

    #[test]
    fn test_stat_row() {
        let c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        let row = c.stat_row(
            false,
            &vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
            ],
            &Vec::default(),
        );
        let row = row.iter().map(|x| x.content()).collect::<Vec<_>>();
        assert_eq!(
            row,
            vec!["MN#1.0", "2", "b*", "c*", "a*", "d*", "", "", "3.5", "4", ""]
        );

        let row = c.stat_row(
            false,
            &vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
            ],
            &vec![&c],
        );
        let row = row.iter().map(|x| x.content()).collect::<Vec<_>>();
        assert_eq!(
            row,
            vec!["MN#1.0", "2", "b", "c", "a", "d", "", "", "3.5", "0", "0/MN#1"]
        );
    }

    #[test]
    fn test_stat_row_box_eq() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Eq,
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from(""),
                offer: None,
            },
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        let row = c.stat_row(
            false,
            &vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
            ],
            &Vec::default(),
        );
        let row = row.iter().map(|x| x.content()).collect::<Vec<_>>();
        assert_eq!(
            row,
            vec!["MB#1.0", "E", "b", "c", "a", "d", "", "", "3.5", "", ""]
        );
    }

    // #[test]
    // fn test_print_hdr() {
    //     let c = Constraint {
    //         exclude: None,
    //         exclude_s: None,
    //         no_exclude: false,
    //         map_s: HashMap::from([("A".to_string(), "b".to_string()), ("B".to_string(), "c".to_string()), ("C".to_string(), "a".to_string()), ("D".to_string(), "d".to_string())]),
    //         check: CheckType::Lights(2, BTreeMap::new()),
    //         map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
    //         eliminated: 100,
    //         eliminated_tab: vec![
    //             vec![1, 0, 0, 0, 0],
    //             vec![0, 1, 0, 3, 0],
    //             vec![0, 0, 2, 0, 3],
    //             vec![0, 6, 0, 5, 0],
    //         ],
    //         entropy: Some(3.5),
    //         left_after: None,
    //         hidden: false,
    //         r#type: ConstraintType::Night {
    //             num: 1.0,
    //             comment: String::from(""),
    //         },
    //     };
    //
    //     let row = c.print_hdr();
    // }

    // #[test]
    // fn test_write_stats() {
    //     let c = Constraint {
    //         exclude: None,
    //         exclude_s: None,
    //         no_exclude: false,
    //         map_s: HashMap::from([("A".to_string(), "b".to_string()), ("B".to_string(), "c".to_string()), ("C".to_string(), "a".to_string()), ("D".to_string(), "d".to_string())]),
    //         check: CheckType::Lights(2, BTreeMap::new()),
    //         map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
    //         eliminated: 100,
    //         eliminated_tab: vec![
    //             vec![1, 0, 0, 0, 0],
    //             vec![0, 1, 0, 3, 0],
    //             vec![0, 0, 2, 0, 3],
    //             vec![0, 6, 0, 5, 0],
    //         ],
    //         entropy: Some(3.5),
    //         left_after: None,
    //         hidden: false,
    //         r#type: ConstraintType::Night {
    //             num: 1.0,
    //             comment: String::from(""),
    //         },
    //     };
    //
    //     let row = c.write_stats();
    // }

    // show_expected_lights
    // show_pr_lights
    // comment
    // type_str

    #[test]
    fn test_comment_night() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from("comment"),
            },
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        assert_eq!(c.comment(), "comment");
    }

    #[test]
    fn test_comment_box() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from("comment"),
                offer: None,
            },
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        assert_eq!(c.comment(), "comment");
    }

    #[test]
    fn test_type_str_night() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from("comment"),
            },
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        assert_eq!(c.type_str(), "MN#1");
    }

    #[test]
    fn test_type_str_box() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from("comment"),
                offer: None,
            },
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        assert_eq!(c.type_str(), "MB#1");
    }
}

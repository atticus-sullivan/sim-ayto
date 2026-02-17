pub mod eval_compute;
pub mod eval_predicates;
pub mod eval_report;
pub mod eval_types;
pub mod parse;
pub mod parse_helpers;
mod utils;

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use anyhow::Result;

use rust_decimal::Decimal;

use serde::Deserialize;

use crate::matching_repr::{bitset::Bitset, MaskedMatching};
use crate::ruleset_data::RuleSetData;
use crate::{Lut, Map, MapS};

/// Types used to decide how to check a matching against a constraint.
///
/// - `Eq` checks that two entries are equal (used for "box" equality constraints).
/// - `Nothing` no-op check.
/// - `Sold` special check for sold events.
/// - `Lights(n, stats)` checks exact number of lights; `stats` is a mutable bucket
///   map used while processing to accumulate frequencies (kept in the Constraint).
#[derive(Deserialize, Debug, Clone, Hash)]
pub enum CheckType {
    Eq,
    Nothing,
    Sold,
    Lights(u8, #[serde(skip)] BTreeMap<u8, u128>),
}

impl CheckType {
    /// Return the lights-count if this `CheckType` is `Lights`.
    pub fn as_lights(&self) -> Option<u8> {
        if let CheckType::Lights(l, _) = *self {
            Some(l)
        } else {
            None
        }
    }
}

/// An offer attached to a box event. The enum mirrors the YAML structure and
/// contains optional amounts and actors.
#[derive(Deserialize, Debug, Clone)]
pub enum Offer {
    Single {
        amount: Option<u128>,
        by: String,
        #[serde(rename = "reducedPot")]
        reduced_pot: bool,
        save: bool,
    },
    SinglePair {
        amount: Option<u128>,
        #[serde(rename = "byA")]
        by_a: String,
        #[serde(rename = "byB")]
        by_b: String,
        #[serde(rename = "reducedPot")]
        reduced_pot: bool,
        save: bool,
    },
    Group {
        amount: Option<u128>,
        by: String,
    },
    GroupPair {
        amount: Option<u128>,
        #[serde(rename = "byA")]
        by_a: String,
        #[serde(rename = "byB")]
        by_b: String,
    },
}

impl Offer {
    /// Return the numeric `amount` if present on the offer.
    pub fn try_get_amount(&self) -> Option<u128> {
        match &self {
            Offer::Single { amount, .. } => *amount,
            Offer::SinglePair { amount, .. } => *amount,
            Offer::Group { amount, .. } => *amount,
            Offer::GroupPair { amount, .. } => *amount,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub enum ConstraintType {
    Night {
        num: Decimal,
        comment: String,
    },
    Box {
        num: Decimal,
        comment: String,
        offer: Option<Offer>,
    },
}

impl Hash for ConstraintType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ConstraintType::Night { num, comment } => {
                0.hash(state); // A constant to distinguish this variant
                num.hash(state);
                comment.hash(state);
            }
            ConstraintType::Box {
                num,
                comment,
                offer,
            } => {
                1.hash(state); // A constant to distinguish this variant
                num.hash(state);
                comment.hash(state);
                offer.hash(state)
            }
        }
    }
}
impl Hash for Offer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Offer::Single {
                amount,
                by,
                reduced_pot,
                save,
            } => {
                0.hash(state); // A constant to distinguish this variant
                if let Some(amount) = amount {
                    amount.hash(state);
                }
                by.hash(state);
                reduced_pot.hash(state);
                save.hash(state);
            }
            Offer::SinglePair {
                amount,
                reduced_pot,
                save,
                by_a,
                by_b,
            } => {
                1.hash(state); // A constant to distinguish this variant
                if let Some(amount) = amount {
                    amount.hash(state);
                }
                by_a.hash(state);
                by_b.hash(state);
                reduced_pot.hash(state);
                save.hash(state);
            }
            Offer::Group { amount, by } => {
                2.hash(state); // A constant to distinguish this variant
                if let Some(amount) = amount {
                    amount.hash(state);
                }
                by.hash(state);
            }
            Offer::GroupPair { amount, by_a, by_b } => {
                3.hash(state); // A constant to distinguish this variant
                if let Some(amount) = amount {
                    amount.hash(state);
                }
                by_a.hash(state);
                by_b.hash(state);
            }
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

    map: MaskedMatching,
    map_s: MapS,
    exclude: Option<(u8, Bitset)>,
    eliminated: u128,
    eliminated_tab: Vec<Vec<u128>>,

    information: Option<f64>,
    left_after: Option<u128>,
    left_poss: Vec<MaskedMatching>,

    hide_ruleset_data: bool,
    pub ruleset_data: Box<dyn RuleSetData>,
    known_lights: u8,
}

impl Constraint {
    /// Create a new unchecked `Constraint`. This is the lowest-level constructor
    /// used by `ConstraintParse::finalize_parsing`. It intentionally does **not**
    /// validate parameters — caller must ensure sizes and invariants.
    pub fn new_unchecked(
        t: ConstraintType,
        check: CheckType,
        map: MaskedMatching,
        rs_dat: Box<dyn RuleSetData>,
        a_len: usize,
        b_len: usize,
        known_lights: u8,
    ) -> Self {
        Constraint {
            r#type: t,
            check,
            hidden: false,
            result_unknown: false,
            build_tree: false,
            map,
            map_s: MapS::default(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![vec![0; b_len]; a_len],
            information: None,
            left_after: None,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: rs_dat,
            known_lights: known_lights,
        }
    }
}

/// Normalize and possibly flip `c_map` and `c_map_s` so that the internal ordering
/// is consistent: the key/value pairs are arranged such that `lut_a[key] < lut_b[value]`
/// if possible. This mutation happens in-place.
///
/// # Arguments
///
/// * `c_map` - map from `u8 -> u8` that may be flipped in-place.
/// * `c_map_s` - string-keyed map used for output; entries are flipped consistent with `c_map`.
/// * `lut_a` / `lut_b` - lookup tables used to compare names (must contain all keys present).
///
/// # Panics
///
/// Panics if `lut_a`/`lut_b` do not contain expected keys (caller must supply valid LUTs).
fn sort_maps(c_map: &mut Map, c_map_s: &mut MapS, lut_a: &Lut, lut_b: &Lut) {
    let c_map2 = c_map
        .drain()
        .map(|(k, v)| if k < v { (v, k) } else { (k, v) })
        .collect();

    let c_map_s2 = c_map_s
        .drain()
        .map(|(k, v)| {
            if lut_a[&k] < lut_b[&v] {
                (v, k)
            } else {
                (k, v)
            }
        })
        .collect();

    *c_map = c_map2;
    *c_map_s = c_map_s2;
}

// functions for executing the simulation
impl Constraint {
    /// Internal predicate: whether `m` would satisfy the constraint's `check`.
    ///
    /// This function is intentionally small and pure except for reading `self` state.
    /// It is used by `process()` (which handles side effects like elimination and
    /// pushing to `ruleset_data`).
    fn fits(&mut self, m: &MaskedMatching) -> bool {
        // first step is to check if the constraint filters out this matching
        match &mut self.check {
            CheckType::Eq => {
                let mask = self
                    .map
                    .iter()
                    .filter(|i| !i.is_empty())
                    .fold(Bitset::empty(), |acc, i| i | acc);
                m.contains_mask(mask)
            }
            CheckType::Nothing | CheckType::Sold => true,
            CheckType::Lights(ref lights, ref mut light_count) => {
                let l = self.map.calculate_lights(m);

                let f = self
                    .exclude
                    .as_ref()
                    .and_then(|ex| m.slot_mask(ex.0 as usize).map(|m| !m.contains_any(&ex.1)));

                // use calculated lights to collect stats on based on the matching possible until
                // here, how many lights are calculated how often for this map
                *light_count.entry(l).or_insert(0) += 1;

                if let Some(f) = f {
                    f
                } else {
                    l == *lights
                }
            }
        }
    }

    /// Process a matching `m` and apply side effects:
    /// - if `m` does not fit the constraint it is recorded as eliminated,
    /// - otherwise `m` may be pushed into `ruleset_data` for later usage,
    /// - if `build_tree` is enabled we collect `left_poss` examples for tree building.
    pub fn process(&mut self, m: &MaskedMatching) -> Result<bool> {
        // check fits actually has a value and make it immutable
        let fits = self.fits(m) || self.result_unknown;

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
    /// Return user-supplied comment from the underlying `ConstraintType`.
    pub fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { comment, .. } => comment,
            ConstraintType::Box { comment, .. } => comment,
        }
    }

    /// Textual representation of the constraint type (used in summary tables).
    pub fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, .. } => format!("MN#{}", num),
            ConstraintType::Box { num, .. } => format!("MB#{}", num),
        }
    }

    /// The numeric index associated with this constraint (MB or MN index).
    pub fn num(&self) -> Decimal {
        match &self.r#type {
            ConstraintType::Night { num, .. } => *num,
            ConstraintType::Box { num, .. } => *num,
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::Constraint;
    use super::*;
    use crate::ruleset_data::dummy::DummyData;
    use crate::Rem;
    use std::collections::HashMap;

    #[test]
    fn test_sort_maps_basic() {
        let mut map_s = HashMap::new();
        let mut map = HashMap::new();

        // Initialize the maps with unordered key/value pairs
        map.insert(1, 0);
        map.insert(2, 3);

        map_s.insert("B".to_string(), "A".to_string());
        map_s.insert("C".to_string(), "D".to_string());

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
        super::sort_maps(&mut map, &mut map_s, &lut_a, &lut_b);

        // Validate the map is sorted and flipped correctly
        assert_eq!(*map.get(&1).unwrap(), 0);
        assert_eq!(*map.get(&3).unwrap(), 2);

        // Validate map_s is sorted and flipped correctly according to the LUTs
        assert_eq!(map_s.get("B").unwrap(), "A");
        assert_eq!(map_s.get("D").unwrap(), "C");
    }

    #[test]
    fn test_sort_maps_no_flipping_needed() {
        let mut map_s = HashMap::new();
        let mut map = HashMap::new();

        // Initialize the maps with unordered key/value pairs
        map.insert(1, 0);
        map.insert(3, 2);

        map_s.insert("B".to_string(), "A".to_string());
        map_s.insert("D".to_string(), "C".to_string());

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
        super::sort_maps(&mut map, &mut map_s, &lut_a, &lut_b);

        // Validate the map is sorted and flipped correctly
        assert_eq!(*map.get(&1).unwrap(), 0);
        assert_eq!(*map.get(&3).unwrap(), 2);

        // Validate map_s is sorted and flipped correctly according to the LUTs
        assert_eq!(map_s.get("B").unwrap(), "A");
        assert_eq!(map_s.get("D").unwrap(), "C");
    }

    #[test]
    #[should_panic]
    fn test_sort_maps_panic_on_missing_lut_keys() {
        let mut map_s = HashMap::new();
        let mut map = HashMap::new();

        map.insert(1, 0);
        // Initialize the map_s with keys not present in lut_a or lut_b
        map_s.insert("unknown".to_string(), "value".to_string());

        // Initialize lookup tables with different keys
        let lut_a = HashMap::new();
        let lut_b = HashMap::new();

        // Perform sorting (should panic due to missing keys)
        super::sort_maps(&mut map, &mut map_s, &lut_a, &lut_b);
    }

    fn constraint_def(exclude: Option<(u8, Bitset)>, lights: u8) -> Constraint {
        Constraint {
            result_unknown: false,
            exclude,
            map_s: HashMap::new(),
            check: CheckType::Lights(lights, BTreeMap::new()),
            map: MaskedMatching::from_matching_ref(&vec![vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
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
        let mut c = constraint_def(None, 2);

        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
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
        let mut c = constraint_def(Some((0, Bitset::from_idxs(&[2, 3]))), 1);

        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert!(c.process(&m).unwrap());

        let m = MaskedMatching::from_matching_ref(&[vec![0, 2], vec![1], vec![4], vec![3]]);
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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2]]),
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
                num: dec![1.0],
                comment: String::from(""),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert!(!c.process(&m).unwrap());
        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1, 2], vec![3], vec![4]]);
        assert!(c.process(&m).unwrap());
    }

    #[test]
    fn test_eliminate() {
        let mut c = constraint_def(None, 2);
        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);

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
        let mut c = constraint_def(None, 2);
        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);

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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
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
            vec!["MN#1.0", "2", "b", "c", "a", "d", "", "", "3.5", "0", "0/MN#1.0"]
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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
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

    #[test]
    fn checktype_as_lights_and_offer_amount() {
        assert_eq!(CheckType::Eq.as_lights(), None);
        assert_eq!(CheckType::Lights(3, BTreeMap::new()).as_lights(), Some(3));

        let o = Offer::Single {
            amount: Some(42),
            by: "x".into(),
            reduced_pot: false,
            save: false,
        };
        assert_eq!(o.try_get_amount(), Some(42u128));
        let o2 = Offer::Group {
            amount: None,
            by: "x".into(),
        };
        assert_eq!(o2.try_get_amount(), None);
    }

    #[test]
    fn new_unchecked_helper_works() {
        let data: Box<dyn RuleSetData> = Box::new(DummyData::default());
        let mm = MaskedMatching::from_matching_ref(&vec![vec![0u8]]);
        let c = Constraint::new_unchecked(
            ConstraintType::Night {
                num: dec![1.0],
                comment: String::new(),
            },
            CheckType::Nothing,
            mm,
            data,
            1,
            1,
            0,
        );
        // basic invariants
        assert_eq!(c.eliminated, 0);
    }

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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
                comment: String::from("comment"),
            },
            result_unknown: false,
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        assert_eq!(c.type_str(), "MN#1.0");
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
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
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
                num: dec![1.0],
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

        assert_eq!(c.type_str(), "MB#1.0");
    }

    #[test]
    fn fits_eq_mask_behavior() {
        // Construct constraint that checks Eq with mapping 1->1 (two items)
        let mut c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Eq,
            // map: 1->1, 2->2 style (we only need a map that produces a non-empty mask)
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 5]; 4],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: String::from(""),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        // Matching `m` that *does not* include all required mask bits -> fits() should be false
        let m1 = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert_eq!(c.fits(&m1), false);

        // Matching `m` that includes the required mask -> fits() true
        // second matching has same structure but element 1 has options [1,2] so contains mask
        let m2 = MaskedMatching::from_matching_ref(&[vec![0], vec![1, 2], vec![3], vec![4]]);
        assert_eq!(c.fits(&m2), true);
    }

    #[test]
    fn fits_nothing_and_sold_are_always_true() {
        let mut c1 = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Nothing,
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 1]; 1],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: String::new(),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };
        let m = MaskedMatching::from_matching_ref(&[vec![0]]);
        assert!(c1.fits(&m));

        let mut c2 = c1.clone();
        c2.check = CheckType::Sold;
        assert!(c2.fits(&m));
    }

    #[test]
    fn fits_lights_updates_histogram_and_matches_count() {
        // use constraint_def helper to create Lights constraint (default known settings)
        let mut c = constraint_def(None, 2); // expects 2 lights

        // create a matching that yields 0 lights (based on constraint_def's internal map)
        let m_no = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        // histogram should start empty
        match &c.check {
            CheckType::Lights(_, hist) => assert!(hist.is_empty()),
            _ => panic!("expected lights"),
        }

        // call fits: expected false, but histogram will be updated for computed lights (some l)
        let res = c.fits(&m_no);
        assert_eq!(res, false);

        // After call, histogram should have one entry for the computed lights value
        match &c.check {
            CheckType::Lights(_, hist) => {
                // ensure an entry exists for the computed lights count
                assert!(hist.values().sum::<u128>() >= 1);
                // The histogram key matches the stored lights value `l` (we can't know exact l here,
                // but ensure there's a key present)
                assert!(!hist.is_empty());
            }
            _ => panic!("expected lights"),
        }

        // Now change the expected lights inside the constraint to something matching the computed `l`
        // We can inspect the histogram key to set the needed value to force success
        let computed_key = match &c.check {
            CheckType::Lights(_, hist) => *hist.keys().next().unwrap(),
            _ => unreachable!(),
        };
        // mutate check so the required lights matches computed_key
        c.check = CheckType::Lights(computed_key, BTreeMap::new());
        assert!(c.fits(&m_no));
    }

    #[test]
    fn fits_lights_with_exclude_respects_exclusion_mask() {
        // create a constraint that expects 1 light and has an exclude on slot 0 (positions masked)
        let exclude_bs = Bitset::from_idxs(&[2, 3]); // exclude values indices 2,3
        let mut c = constraint_def(Some((0, exclude_bs.clone())), 1);

        // matching m1: slot0 does not include any excluded index -> should pass (fits true)
        let m1 = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert!(c.fits(&m1));

        // matching m2: slot0 contains an excluded index -> should fail (fits false)
        let m2 = MaskedMatching::from_matching_ref(&[vec![0, 2], vec![1], vec![4], vec![3]]);
        assert!(!c.fits(&m2));
    }

    #[test]
    fn process_respects_result_unknown_and_does_not_eliminate() {
        // construct a Lights constraint that would normally reject a matching; set result_unknown
        let c = constraint_def(None, 2);
        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);

        // For baseline: process without result_unknown eliminates (returns false)
        let mut c2 = c.clone();
        let ok = c2.process(&m).unwrap();
        assert_eq!(ok, false);
        assert_eq!(c2.eliminated, 1);

        // Now set result_unknown true on a fresh constraint -> process should return true and not eliminate
        let mut c3 = constraint_def(None, 2);
        c3.result_unknown = true;
        let ok2 = c3.process(&m).unwrap();
        assert_eq!(ok2, true);
        // no elimination performed in this case
        assert_eq!(c3.eliminated, 0);
    }
}

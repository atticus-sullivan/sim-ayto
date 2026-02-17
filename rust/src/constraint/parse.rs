use std::hash::{Hash, Hasher};

use anyhow::Result;

use serde::Deserialize;

use crate::constraint::{sort_maps, CheckType, Constraint, ConstraintType};
use crate::ruleset_data::RuleSetData;
use crate::{Lut, MapS, Rename};

// this struct is only used when parsing the yaml file.
// The function `finalize_parsing` is intended to convert this to a regular constraint.
#[derive(Deserialize, Debug, Clone)]
pub struct ConstraintParse {
    r#type: ConstraintType,
    #[serde(rename = "map")]
    pub(super) map_s: MapS,
    check: CheckType,
    #[serde(default)]
    hidden: bool,
    #[serde(default, rename = "noExclude")]
    no_exclude: bool,
    #[serde(rename = "exclude")]
    exclude_s: Option<(String, Vec<String>)>,
    #[serde(default, rename = "resultUnknown")]
    pub result_unknown: bool,
    #[serde(default, rename = "buildTree")]
    build_tree: bool,
    #[serde(default, rename = "hideRulesetData")]
    hide_ruleset_data: bool,
}

impl Hash for ConstraintParse {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the r#type field
        self.r#type.hash(state);

        // Sort the map_s entries by key to ensure stable hashing
        let mut sorted_entries: Vec<_> = self.map_s.iter().collect();
        sorted_entries.sort_by(|(key_a, _), (key_b, _)| key_a.cmp(key_b)); // Sort by key lexicographically

        // Hash each sorted entry
        for (key, value) in sorted_entries {
            key.hash(state);
            value.hash(state);
        }

        // Hash the check field
        self.check.hash(state);
    }
}

// helpers
impl ConstraintParse {
    /// How many lights this constraint *adds* when converting a box constraint
    /// with lights==1 to a new effective constraint.
    pub fn added_known_lights(&self) -> u8 {
        if let ConstraintType::Box { .. } = self.r#type {
            if let CheckType::Lights(1, _) = self.check {
                return 1;
            }
        }
        0
    }

    /// Whether this constraint actually restricts the solution set (not a no-op).
    pub fn has_impact(&self) -> bool {
        if self.result_unknown {
            return false;
        }
        if let CheckType::Nothing | CheckType::Sold = &self.check {
            return false;
        }
        true
    }

    pub fn is_box(&self) -> bool {
        matches!(self.r#type, ConstraintType::Box { .. })
    }
}

// getter functions
impl ConstraintParse {
    #[allow(dead_code)]
    pub fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { comment, .. } => comment,
            ConstraintType::Box { comment, .. } => comment,
        }
    }

    pub fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, .. } => format!("MN#{}", num),
            ConstraintType::Box { num, .. } => format!("MB#{}", num),
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
}

impl ConstraintParse {
    /// Convert a `ConstraintParse` (raw YAML-deserialized structure) into a runtime `Constraint`.
    ///
    /// This performs:
    /// - finalization of optional parameters,
    /// - mapping of names via `rename`,
    /// - adding computed `exclude_s` if requested,
    /// - validation against `ruleset_data`.
    ///
    /// # Arguments
    /// - `lut_a`, `lut_b`: lookup tables for names -> ids for set a / b
    /// - `map_len`: expected cardinality
    /// - `map_b`: names present in set_b for auto exclusion heuristics
    /// - `add_exclude`: whether to compute `exclude_s` automatically
    /// - `sort_constraint`: whether to sort the maps (for canonicalization)
    /// - `rename`: tuple of rename maps used for normalization
    /// - `ruleset_data`: runtime data provider used for validations
    /// - `known_lights`: number of known lights (domain-specific)
    ///
    /// # Returns
    /// `Result<Constraint>` on success.
    #[allow(clippy::too_many_arguments)]
    pub fn finalize_parsing(
        self,
        lut_a: &Lut,
        lut_b: &Lut,
        map_len: usize,
        map_b: &[String],
        add_exclude: bool,
        sort_constraint: bool,
        rename: (&Rename, &Rename),
        ruleset_data: Box<dyn RuleSetData>,
        known_lights: u8,
    ) -> Result<Constraint> {
        // If add_exclude requested prefer computed add_exclude result, fallback to explicit exclude in YAML
        let exclude_s_final = if add_exclude {
            match self.add_exclude(map_b) {
                Some(e) => Some(e),
                None => self.exclude_s.clone(),
            }
        } else {
            self.exclude_s.clone()
        };

        // convert map_s names -> numeric ids
        let (mut c_map, mut c_map_s) = self.convert_map_s_to_ids(lut_a, lut_b)?;

        // optional sorting based on LUT comparators
        if sort_constraint {
            sort_maps(&mut c_map, &mut c_map_s, lut_a, lut_b);
        }

        // create the base Constraint (eliminated_tab sized using LUT lengths)
        let mut c = Constraint {
            r#type: self.r#type,
            check: self.check,
            hidden: self.hidden,
            result_unknown: self.result_unknown,
            build_tree: self.build_tree,
            map_s: c_map_s,
            map: c_map.try_into()?,
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![vec![0; lut_b.len()]; lut_a.len()],
            information: None,
            left_after: None,
            left_poss: Default::default(),
            ruleset_data,
            hide_ruleset_data: self.hide_ruleset_data,
            known_lights,
        };

        // validate shape invariants
        Self::validate_map_cardinalities(&self.exclude_s, &c, map_len)?;

        // rename keys in map_s for user-facing output
        let mut map_s = MapS::default();
        for (k, v) in &c.map_s {
            map_s.insert(
                rename.0.get(k).unwrap_or(k).to_owned(),
                rename.1.get(v).unwrap_or(v).to_owned(),
            );
        }
        c.map_s = map_s;

        // build exclude bitset if requested / given
        if let Some(excl) = Self::build_exclude_if_any(&exclude_s_final, lut_a, lut_b)? {
            c.exclude = Some(excl);
        }

        Ok(c)
    }

    /// Generates the exclude list for the constraint, by inserting the elements from `map_b`
    ///
    /// This function modifies the internal state of the `Constraint`. If exclusion is not needed (constraint type is no box or lights != 1), no exclude list is generated.
    ///
    /// # Arguments
    ///
    /// - `map_b`: A reference to a vector of strings (`Vec<String>`) from which exclusions will be drawn. The function will create a new exclusion vector by removing any elements from `map_b` that match the current value in `self.map_s`.
    fn add_exclude(&self, map_b: &[String]) -> Option<(String, Vec<String>)> {
        if self.no_exclude {
            return None;
        }
        if let CheckType::Lights(l, _) = self.check {
            if !(l == 1 && self.map_s.len() == 1 && self.exclude_s.is_none()) {
                return None;
            }
            if let ConstraintType::Box { .. } = self.r#type {
                // if the constraint is a box constraint the map contains only one item anyhow
                if let Some((k, v)) = self.map_s.iter().next() {
                    let bs: Vec<String> = map_b
                        .iter()
                        .filter(|&i| i != v)
                        .map(|i| i.to_string())
                        .collect();
                    return Some((k.to_string(), bs));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;
    use crate::matching_repr::bitset::Bitset;
    use crate::matching_repr::MaskedMatching;
    use crate::ruleset_data::dummy::DummyData;
    use std::collections::BTreeMap;
    use std::collections::HashMap;

    #[test]
    fn test_finalize_parsing_night_lights() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Night {
                num: dec![1.0],
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Lights(3, BTreeMap::new()),
            hidden: false,
            no_exclude: false,
            result_unknown: false,
            exclude_s: None,
            build_tree: false,
            hide_ruleset_data: false,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map_s.insert("A".to_string(), "B".to_string());
        constraint.map_s.insert("C".to_string(), "D".to_string());
        constraint.map_s.insert("D".to_string(), "C".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(vec![
            ("A".to_string(), 0),
            ("B".to_string(), 1),
            ("C".to_string(), 2),
            ("D".to_string(), 3),
        ]);
        let lut_b = lut_a.clone();

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                3,
                &[],
                false,
                false,
                (&Default::default(), &Default::default()),
                Box::new(DummyData::default()),
                0,
            )
            .unwrap();

        let map = MaskedMatching::from_matching_ref(&[vec![1], vec![], vec![3], vec![2]]);
        assert_eq!(map, constraint.map);
    }

    #[test]
    fn test_finalize_parsing_box_lights() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            map_s: HashMap::new(),
            check: CheckType::Lights(1, BTreeMap::new()),
            hidden: false,
            result_unknown: false,
            exclude_s: Some(("A".to_string(), vec!["C".to_string(), "D".to_string()])),
            no_exclude: false,
            build_tree: false,
            hide_ruleset_data: false,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map_s.insert("A".to_string(), "B".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(vec![
            ("A".to_string(), 0),
            ("B".to_string(), 1),
            ("C".to_string(), 2),
            ("D".to_string(), 3),
        ]);
        let lut_b = lut_a.clone();

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                20,
                &[],
                true,
                false,
                (&Default::default(), &Default::default()),
                Box::new(DummyData::default()),
                0,
            )
            .unwrap();

        let map_s = HashMap::from_iter(vec![("A".to_string(), "B".to_string())]);
        assert_eq!(map_s, constraint.map_s);
        let map = MaskedMatching::from_matching_ref(&[vec![1]]);
        assert_eq!(map, constraint.map);
        let excl = Some((0, Bitset::from_idxs(&[2, 3])));
        assert_eq!(excl, constraint.exclude);
    }

    #[test]
    fn test_finalize_parsing_box_eq() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            result_unknown: false,
            exclude_s: None,
            no_exclude: false,
            build_tree: false,
            hide_ruleset_data: false,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map_s.insert("A".to_string(), "B".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(vec![
            ("A".to_string(), 0),
            ("B".to_string(), 1),
            ("C".to_string(), 2),
            ("D".to_string(), 3),
        ]);
        let lut_b = lut_a.clone();

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                20,
                &[],
                false,
                false,
                (&Default::default(), &Default::default()),
                Box::new(DummyData::default()),
                0,
            )
            .unwrap();

        let map_s = HashMap::from_iter(vec![("A".to_string(), "B".to_string())]);
        assert_eq!(map_s, constraint.map_s);
        let map = MaskedMatching::from_matching_ref(&[vec![1]]);
        assert_eq!(map, constraint.map);
    }

    #[test]
    fn test_add_exclude() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            map_s: HashMap::new(),
            check: CheckType::Lights(1, BTreeMap::new()),
            hidden: false,
            exclude_s: None,
            no_exclude: false,
            result_unknown: false,
            build_tree: false,
            hide_ruleset_data: false,
        };

        constraint.map_s.insert("A".to_string(), "b".to_string());

        // Initialize lookup tables
        let map_b = vec!["b".to_string(), "c".to_string(), "d".to_string()];

        let exclude_s = constraint.add_exclude(&map_b);

        assert_eq!(
            exclude_s.unwrap(),
            ("A".to_string(), vec!["c".to_string(), "d".to_string()])
        );
    }

    #[test]
    fn convert_map_s_to_ids_works() {
        let mut cp = ConstraintParse {
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            no_exclude: false,
            exclude_s: None,
            result_unknown: false,
            build_tree: false,
            hide_ruleset_data: false,
        };
        cp.map_s.insert("A".to_string(), "B".to_string());
        let lut_a = HashMap::from_iter(vec![("A".to_string(), 0)]);
        let lut_b = HashMap::from_iter(vec![("B".to_string(), 1)]);

        let (c_map, c_map_s) = cp.convert_map_s_to_ids(&lut_a, &lut_b).unwrap();
        assert_eq!(c_map.get(&0).cloned(), Some(1u8));
        assert_eq!(c_map_s.get("A").unwrap(), "B");
    }
}

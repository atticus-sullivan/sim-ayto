use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::ruleset_data::RuleSetData;
use crate::{Lut, Map, MapS, Rename};

use crate::constraint::{CheckType, Constraint, ConstraintType};

// this struct is only used when parsing the yaml file.
// The function `finalize_parsing` is intended to convert this to a regular constraint.
#[derive(Deserialize, Debug, Clone)]
pub struct ConstraintParse {
    r#type: ConstraintType,
    #[serde(rename = "map")]
    map_s: MapS,
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

impl ConstraintParse {
    pub fn known_lights(&self) -> u8 {
        if let ConstraintType::Box{ num: _, comment: _ } = self.r#type {
            if let CheckType::Lights(1, _) = self.check {
                return 1
            }
        }
        0
    }

    pub fn has_impact(&self) -> bool {
        if self.result_unknown {
            return false;
        }
        if let CheckType::Nothing | CheckType::Sold = &self.check {
            return false;
        }
        true
    }
}

// getter functions
impl ConstraintParse {
    #[allow(dead_code)]
    pub fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { num: _, comment } => comment,
            ConstraintType::Box { num: _, comment } => comment,
        }
    }

    pub fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, comment: _ } => format!("MN#{}", num),
            ConstraintType::Box { num, comment: _ } => format!("MB#{}", num),
        }
    }
}

impl ConstraintParse {
    /// Finalize the initialization phase by translating the names (strings) to ids, validating the
    /// stored data, initialize the internal state of the constraint, optionally add an exclude map
    /// and optionally sort the constraints.
    ///
    /// # Arguments
    ///
    /// - `lut_a`: Reference to the lookup table for set_a (the keys)
    /// - `lut_b`: Reference to the lookup table for set_b (the values)
    /// - `map_len`: How many elements are expected to occur in the matching night
    /// - `map_b`: Reference to the set of elements in set_b used to generate the exclude map
    /// - `add_exclude`: whether to automatically add the exclude map
    /// - `sort_constraint`: whether to sort the maps used for this constraint
    /// - `rename`: Maps one name to another name for renaming the names of set_a and set_b
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
        let exclude_s = if add_exclude {
            match self.add_exclude(map_b) {
                Some(e) => Some(e),
                None => self.exclude_s.clone(),
            }
        } else {
            self.exclude_s.clone()
        };

        let mut c = Constraint {
            r#type: self.r#type,
            check: self.check,
            hidden: self.hidden,
            result_unknown: self.result_unknown,
            build_tree: self.build_tree,
            map_s: self.map_s,
            map: Map::default(),
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

        c.map = c
            .map_s
            .iter()
            .map(&|(k, v)| {
                let k = *lut_a.get(k).with_context(|| format!("Invalid Key {}", k))? as u8;
                let v = *lut_b
                    .get(v)
                    .with_context(|| format!("Invalid Value {}", v))? as u8;
                Ok((k, v))
            })
            .collect::<Result<_>>()?;

        // check if map size is valid
        match c.r#type {
            ConstraintType::Night { .. } => {
                ensure!(
                    c.map_s.len() == map_len,
                    "Map in a night must contain exactly as many entries as set_a {} (was: {})",
                    map_len,
                    c.map_s.len()
                );
                let value_len = c.map_s.values().collect::<HashSet<_>>().len();
                ensure!(
                    value_len == c.map_s.len(),
                    "Keys in the map of a night must be unique {:?}",
                    c.map_s
                );
                ensure!(
                    self.exclude_s.is_none(),
                    "Exclude is not yet supported for nights"
                );
            }
            ConstraintType::Box { .. } => match &c.check {
                CheckType::Eq => {}
                CheckType::Nothing | CheckType::Sold => {}
                CheckType::Lights(_, _) => {
                    ensure!(
                        c.map_s.len() == 1,
                        "Map in a box must contain exactly {} entry (was: {})",
                        1,
                        c.map_s.len()
                    );
                }
            },
        }

        // rename names in map_s for output use
        let mut map_s = MapS::default();
        for (k, v) in &c.map_s {
            map_s.insert(
                rename.0.get(k).unwrap_or(k).to_owned(),
                rename.1.get(v).unwrap_or(v).to_owned(),
            );
        }
        c.map_s = map_s;

        if sort_constraint {
            c.sort_maps(lut_a, lut_b);
        }

        // translate names to ids
        if let Some(ex) = &exclude_s {
            let (ex_a, ex_b) = ex;
            let mut bs = HashSet::with_capacity(ex_b.len());
            let a = *lut_a
                .get(ex_a)
                .with_context(|| format!("Invalid Key {}", ex_a))? as u8;
            for x in ex_b {
                bs.insert(
                    *lut_b
                        .get(x)
                        .with_context(|| format!("Invalid Value {}", x))? as u8,
                );
            }
            c.exclude = Some((a, bs));
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
    use super::*;
    use crate::ruleset_data::dummy::DummyData;
    use std::collections::BTreeMap;
    use std::collections::HashMap;

    #[test]
    fn test_finalize_parsing_night_lights() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Night {
                num: 1.0,
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

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                3,
                &vec![],
                false,
                false,
                (&Default::default(), &Default::default()),
                Box::new(DummyData::default()),
            )
            .unwrap();

        let map = HashMap::from_iter(vec![(0, 1), (2, 3), (3, 2)].into_iter());
        assert_eq!(map, constraint.map);
    }

    #[test]
    fn test_finalize_parsing_box_lights() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
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

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                20,
                &vec![],
                true,
                false,
                (&Default::default(), &Default::default()),
                Box::new(DummyData::default()),
            )
            .unwrap();

        let map_s = HashMap::from_iter(vec![("A".to_string(), "B".to_string())].into_iter());
        assert_eq!(map_s, constraint.map_s);
        let map = HashMap::from_iter(vec![(0, 1)].into_iter());
        assert_eq!(map, constraint.map);
        let excl = Some((0, HashSet::from([2, 3])));
        assert_eq!(excl, constraint.exclude);
    }

    #[test]
    fn test_finalize_parsing_box_eq() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
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

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                20,
                &vec![],
                false,
                false,
                (&Default::default(), &Default::default()),
                Box::new(DummyData::default()),
            )
            .unwrap();

        let map_s = HashMap::from_iter(vec![("A".to_string(), "B".to_string())].into_iter());
        assert_eq!(map_s, constraint.map_s);
        let map = HashMap::from_iter(vec![(0, 1)].into_iter());
        assert_eq!(map, constraint.map);
    }

    #[test]
    fn test_add_exclude() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
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
}

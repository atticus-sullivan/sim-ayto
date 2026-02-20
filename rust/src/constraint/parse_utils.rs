/// This module contains some helper functions which assist in parsing/converting/validating
/// constraints
use std::collections::{HashMap, HashSet};

use anyhow::{ensure, Context, Result};

use crate::constraint::parse::ConstraintParse;
use crate::constraint::{CheckType, ConstraintType};
use crate::ignore_ops::IgnoreOps;
use crate::{Lut, Map, MapS};

impl ConstraintParse {
    /// How many known lights this constraint *adds* when converting a box constraint with
    /// lights==1 to a new effective constraint.
    pub(crate) fn added_known_lights(&self) -> u8 {
        if self.hidden {
            return 0;
        }

        if let ConstraintType::Box { .. } = self.r#type {
            if let CheckType::Lights(1, _) = self.check {
                return 1;
            }
        }
        0
    }

    /// Whether this constraint actually restricts the solution set (not a no-op).
    pub(crate) fn has_impact(&self) -> bool {
        if self.result_unknown {
            return false;
        }
        if let CheckType::Nothing | CheckType::Sold = &self.check {
            return false;
        }
        true
    }

    pub(crate) fn ignore_on(&self, ops: &IgnoreOps) -> bool {
        match ops {
            IgnoreOps::Boxes => {
                // ignore if this constraint is a box
                matches!(self.r#type, ConstraintType::Box { .. })
            }
        }
    }

    pub(crate) fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, .. } => format!("MN#{}", num),
            ConstraintType::Box { num, .. } => format!("MB#{}", num),
        }
    }

    // convert the string map_s into numeric ids using LUTs.
    /// Convert `self.map_s` (names) into numeric id map and return `(c_map, c_map_s)`.
    /// `c_map` is `HashMap<u8,u8>` and `c_map_s` is the original string map (not renamed).
    /// This function returns an error if a name is not present in the given LUT.
    pub(crate) fn convert_map_s_to_ids(
        &self,
        lut_a: &Lut,
        lut_b: &Lut,
    ) -> Result<(HashMap<u8, u8>, MapS)> {
        let c_map = self
            .map_s
            .iter()
            .map(&|(k, v)| {
                let k_id = *lut_a.get(k).with_context(|| format!("Invalid Key {}", k))? as u8;
                let v_id = *lut_b
                    .get(v)
                    .with_context(|| format!("Invalid Value {}", v))?
                    as u8;
                Ok((k_id, v_id))
            })
            .collect::<Result<HashMap<u8, u8>>>()?;

        // c_map_s will be owned copy of map_s so caller can mutate it (sort/rename later)
        let c_map_s = self.map_s.clone();

        Ok((c_map, c_map_s))
    }

    /// Check cardinality / shape invariants for the parsed constraint.
    pub(crate) fn validate_constraint(&self, map_len: usize) -> Result<()> {
        match self.r#type {
            ConstraintType::Night { .. } => {
                ensure!(
                    self.map_s.len() == map_len,
                    "Map in a night must contain exactly as many entries as set_a {} (was: {})",
                    map_len,
                    self.map_s.len()
                );
                let value_len = self.map_s.values().collect::<HashSet<_>>().len();
                ensure!(
                    value_len == self.map_s.len(),
                    "Keys in the map of a night must be unique {:?}",
                    self.map_s
                );
                ensure!(
                    self.exclude_s.is_none(),
                    "Exclude is not yet supported for nights"
                );
            }
            ConstraintType::Box { .. } => match &self.check {
                CheckType::Eq => {}
                CheckType::Nothing | CheckType::Sold => {}
                CheckType::Lights(_, _) => {
                    ensure!(
                        self.map_s.len() == 1,
                        "Map in a box must contain exactly {} entry (was: {})",
                        1,
                        self.map_s.len()
                    );
                }
            },
        }
        Ok(())
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
    pub(super) fn sort_maps(c_map: &mut Map, c_map_s: &mut MapS, lut_a: &Lut, lut_b: &Lut) {
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
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use rust_decimal::dec;

    use super::*;
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn added_known_lights_simple() {
        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        assert_eq!(cp.added_known_lights(), 0);

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(0, BTreeMap::default());
        cp.r#type = ConstraintType::Box {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        assert_eq!(cp.added_known_lights(), 0);

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(2, BTreeMap::default());
        cp.r#type = ConstraintType::Box {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        assert_eq!(cp.added_known_lights(), 0);

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.r#type = ConstraintType::Box {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        assert_eq!(cp.added_known_lights(), 1);
    }

    #[test]
    fn has_impact_simple() {
        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.result_unknown = false;
        assert!(cp.has_impact());

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Sold;
        cp.result_unknown = false;
        assert!(!cp.has_impact());

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Nothing;
        cp.result_unknown = false;
        assert!(!cp.has_impact());

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.result_unknown = true;
        assert!(cp.has_impact());
    }

    #[test]
    fn ignore_on_simple() {
        let mut cp = ConstraintParse::default();
        cp.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        assert!(!cp.ignore_on(&IgnoreOps::Boxes));

        let mut cp = ConstraintParse::default();
        cp.r#type = ConstraintType::Box {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        assert!(cp.ignore_on(&IgnoreOps::Boxes));
    }

    #[test]
    fn type_str_simple() {
        let mut cp = ConstraintParse::default();
        cp.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        assert_eq!(cp.type_str(), "MN#1.0");

        let mut cp = ConstraintParse::default();
        cp.r#type = ConstraintType::Night {
            num: dec![3],
            comment: "".to_string(),
            offer: None,
        };
        assert_eq!(cp.type_str(), "MN#3.0");

        let mut cp = ConstraintParse::default();
        cp.r#type = ConstraintType::Box {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        assert_eq!(cp.type_str(), "MB#1.0");

        let mut cp = ConstraintParse::default();
        cp.r#type = ConstraintType::Box {
            num: dec![3],
            comment: "".to_string(),
            offer: None,
        };
        assert_eq!(cp.type_str(), "MB#3.0");
    }

    #[test]
    fn convert_map_s_to_ids_simple() {
        let mut cp = ConstraintParse::default();
        cp.map_s.insert("A".to_string(), "b".to_string());
        cp.map_s.insert("B".to_string(), "c".to_string());

        let lut_a = vec![("A", 0), ("B", 5)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<HashMap<_, _>>();
        let lut_b = vec![("b", 2), ("c", 20)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<HashMap<_, _>>();

        let (c_map, c_map_s) = cp.convert_map_s_to_ids(&lut_a, &lut_b).unwrap();

        let c_ref = vec![(0, 2), (5, 20)].into_iter().collect::<HashMap<_, _>>();

        let c_ref_s = vec![("A", "b"), ("B", "c")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();

        assert_eq!(c_map, c_ref);
        assert_eq!(c_map_s, c_ref_s);
    }

    #[test]
    fn validate_constraint_simple() {
        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        cp.map_s = vec![("A", "b"), ("B", "c")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        assert!(cp.validate_constraint(2).is_ok());

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        cp.map_s = vec![("A", "b")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        // Night has too few entries in map
        assert!(cp.validate_constraint(2).is_err());

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        cp.map_s = vec![("A", "b"), ("B", "b")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        // Night has duplicates in map
        assert!(cp.validate_constraint(2).is_err());

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        cp.map_s = vec![("A", "b"), ("B", "c")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        cp.exclude_s = Some(("".to_string(), vec!["".to_string()]));
        // exclude_s is not supported with nigt
        assert!(cp.validate_constraint(2).is_err());

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.r#type = ConstraintType::Box {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        cp.map_s = vec![("A", "b"), ("B", "c")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        // Box can always only have one entry in the map
        assert!(cp.validate_constraint(2).is_err());

        let mut cp = ConstraintParse::default();
        cp.check = CheckType::Lights(1, BTreeMap::default());
        cp.r#type = ConstraintType::Box {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        cp.map_s = vec![("A", "b")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        // Box can always only have one entry in the map
        assert!(cp.validate_constraint(2).is_ok());
    }

    #[test]
    fn sort_maps_w_flip() {
        let mut map_s = HashMap::new();
        let mut map = HashMap::new();

        // Initialize the maps with unordered key/value pairs
        map.insert(1, 0);
        map.insert(2, 3);

        map_s.insert("B".to_string(), "a".to_string());
        map_s.insert("C".to_string(), "d".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(vec![
            ("a".to_string(), 0),
            ("B".to_string(), 1),
            ("C".to_string(), 2),
            ("d".to_string(), 3),
        ]);
        let lut_b = lut_a.clone();

        // Perform sorting
        ConstraintParse::sort_maps(&mut map, &mut map_s, &lut_a, &lut_b);

        // Validate the map is sorted and flipped correctly
        let ref_map = vec![(1, 0), (3, 2)].into_iter().collect::<HashMap<_, _>>();
        assert_eq!(map, ref_map);

        // Validate map_s is sorted and flipped correctly according to the LUTs
        let ref_map = vec![("B", "a"), ("d", "C")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        assert_eq!(map_s, ref_map);
    }

    #[test]
    fn sort_maps_wo_flip() {
        let mut map_s = HashMap::new();
        let mut map = HashMap::new();

        // Initialize the maps with unordered key/value pairs
        map.insert(1, 0);
        map.insert(3, 2);

        map_s.insert("B".to_string(), "a".to_string());
        map_s.insert("D".to_string(), "c".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(vec![
            ("a".to_string(), 0),
            ("B".to_string(), 1),
            ("c".to_string(), 2),
            ("D".to_string(), 3),
        ]);
        let lut_b = lut_a.clone();

        // Perform sorting
        ConstraintParse::sort_maps(&mut map, &mut map_s, &lut_a, &lut_b);

        // Validate the map is sorted and flipped correctly
        let ref_map = vec![(1, 0), (3, 2)].into_iter().collect::<HashMap<_, _>>();
        assert_eq!(map, ref_map);

        // Validate map_s is sorted and flipped correctly according to the LUTs
        let ref_map = vec![("B", "a"), ("d", "C")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();
        assert_eq!(map_s, ref_map);
    }
}

use std::collections::{HashMap, HashSet};

use anyhow::{ensure, Context, Result};

use crate::constraint::parse::ConstraintParse;
use crate::constraint::{CheckType, Constraint, ConstraintType};
use crate::ignore_ops::IgnoreOps;
use crate::matching_repr::bitset::Bitset;
use crate::{Lut, Map, MapS};

impl ConstraintParse {
    /// How many lights this constraint *adds* when converting a box constraint
    /// with lights==1 to a new effective constraint.
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
}

impl ConstraintParse {
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
    /// This mirrors the previous inline `ensure!` checks in `finalize_parsing`.
    pub(crate) fn validate_map_cardinalities(
        exclude_s: &Option<(String, Vec<String>)>,
        c: &Constraint,
        map_len: usize,
    ) -> Result<()> {
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
                    exclude_s.is_none(),
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
        Ok(())
    }

    /// Build the optional exclude bitset based on `exclude_s` and the LUTs.
    pub(crate) fn build_exclude_if_any(
        exclude_s: &Option<(String, Vec<String>)>,
        lut_a: &Lut,
        lut_b: &Lut,
    ) -> Result<Option<(u8, Bitset)>> {
        if let Some(ex) = exclude_s {
            let (ex_a, ex_b) = ex;
            let mut bs = Bitset::empty();
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
            Ok(Some((a, bs)))
        } else {
            Ok(None)
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

#[cfg(test)]
mod test {
    use rust_decimal::dec;

    use super::*;

    // #[test]
    // fn convert_map_s_to_ids_works() {
    //     // TODO: new in tests with private members
    //     let mut cp = ConstraintParse {
    //         r#type: ConstraintType::Box {
    //             num: dec![1.0],
    //             comment: "".to_string(),
    //             offer: None,
    //         },
    //         map_s: HashMap::new(),
    //         check: CheckType::Eq,
    //         hidden: false,
    //         no_exclude: false,
    //         exclude_s: None,
    //         result_unknown: false,
    //         build_tree: false,
    //         hide_ruleset_data: false,
    //     };
    //     cp.map_s.insert("A".to_string(), "B".to_string());
    //     let lut_a = HashMap::from_iter(vec![("A".to_string(), 0)]);
    //     let lut_b = HashMap::from_iter(vec![("B".to_string(), 1)]);
    //
    //     let (c_map, c_map_s) = cp.convert_map_s_to_ids(&lut_a, &lut_b).unwrap();
    //     assert_eq!(c_map.get(&0).cloned(), Some(1u8));
    //     assert_eq!(c_map_s.get("A").unwrap(), "B");
    // }
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
        let lut_a = HashMap::from_iter(vec![
            ("A".to_string(), 0),
            ("B".to_string(), 1),
            ("C".to_string(), 2),
            ("D".to_string(), 3),
        ]);
        let lut_b = lut_a.clone();

        // Perform sorting
        sort_maps(&mut map, &mut map_s, &lut_a, &lut_b);

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
        let lut_a = HashMap::from_iter(vec![
            ("A".to_string(), 0),
            ("B".to_string(), 1),
            ("C".to_string(), 2),
            ("D".to_string(), 3),
        ]);
        let lut_b = lut_a.clone();

        // Perform sorting
        sort_maps(&mut map, &mut map_s, &lut_a, &lut_b);

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
        sort_maps(&mut map, &mut map_s, &lut_a, &lut_b);
    }
}

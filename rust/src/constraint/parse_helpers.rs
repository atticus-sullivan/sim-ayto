use std::collections::{HashMap, HashSet};

use anyhow::{ensure, Context, Result};

use crate::{
    constraint::{parse::ConstraintParse, CheckType, Constraint, ConstraintType},
    matching_repr::bitset::Bitset,
    Lut, MapS,
};

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

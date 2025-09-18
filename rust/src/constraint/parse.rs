use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::collections::HashSet;

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
    result_unknown: bool,
    #[serde(default, rename = "buildTree")]
    build_tree: bool,
    #[serde(default, rename = "hideRulesetData")]
    hide_ruleset_data: bool,
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
    pub fn finalize_parsing(
        self,
        lut_a: &Lut,
        lut_b: &Lut,
        map_len: usize,
        map_b: &Vec<String>,
        add_exclude: bool,
        sort_constraint: bool,
        rename: (&Rename, &Rename),
        ruleset_data: Box<dyn RuleSetData>,
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
                let value_len = c.map_s.iter().map(|(_, v)| v).collect::<HashSet<_>>().len();
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
                CheckType::Nothing => {}
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
    fn add_exclude(&self, map_b: &Vec<String>) -> Option<(String, Vec<String>)> {
        if self.no_exclude {
            return None;
        }
        if let CheckType::Lights(l, _) = self.check {
            if !(l == 1 && self.map_s.len() == 1 && self.exclude_s.is_none()) {
                return None;
            }
            if let ConstraintType::Box { .. } = self.r#type {
                // if the constraint is a box constraint the for loop will only run once anyhow
                for (k, v) in &self.map_s {
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

//! This module contains everything needed (in the game module) to query specific pairs. This
//! includes finishing the parsing from the config and generating the final report.

use std::{collections::HashSet, fmt};

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Row, Table};

use crate::{Lut, game::parse::QueryPair, iterstate::QueryPairData, matching_repr::IdBase};

pub(super) fn translate_query_pairs(
    pair: &QueryPair,
    lut_a: &Lut,
    lut_b: &Lut,
) -> Result<(HashSet<IdBase>, HashSet<IdBase>)> {
    let mut left = HashSet::new();
    let mut right = HashSet::new();

    for a in &pair.map_a {
        let idx = *lut_a
            .get(a)
            .with_context(|| format!("{} not found in lut_a", a))? as IdBase;
        left.insert(idx);
    }
    for b in &pair.map_b {
        let idx = *lut_b
            .get(b)
            .with_context(|| format!("{} not found in lut_b", b))? as IdBase;
        right.insert(idx);
    }
    Ok((left, right))
}

#[derive(Debug, PartialEq)]
pub(super) struct QueryPairReport {
    sections: Vec<QueryPairSection>,
}

impl QueryPairReport {
    pub(super) fn new(
        query_pair: &QueryPairData,
        map_a: &[String],
        map_b: &[String],
    ) -> Result<Self> {
        let mut sections = Vec::with_capacity(2);

        for (a, items) in query_pair.0.iter() {
            let header = map_a
                .get(*a as usize)
                .context("a index out of bounds")?
                .clone();

            let mut rows = Vec::new();
            for (b_list, count) in items {
                let names = b_list
                    .iter()
                    .map(|b| {
                        map_b
                            .get(b as usize)
                            .context("b index out of bounds")
                            .cloned()
                    })
                    .collect::<Result<Vec<_>>>()?;

                rows.push((*count, names));
            }
            sections.push(QueryPairSection { header, rows });
        }

        for (b, items) in query_pair.1.iter() {
            let header = map_b
                .get(*b as usize)
                .context("b index out of bounds")?
                .clone();

            let mut rows = Vec::new();

            for (a, count) in items {
                let name = map_a
                    .get(*a as usize)
                    .context("a index out of bounds")?
                    .clone();

                rows.push((*count, vec![name]));
            }
            sections.push(QueryPairSection { header, rows });
        }

        Ok(QueryPairReport { sections })
    }
}

impl fmt::Display for QueryPairReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in &self.sections {
            writeln!(f, "{i}")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
struct QueryPairSection {
    header: String,
    rows: Vec<(u64, Vec<String>)>, // (count, mapped names)
}

impl fmt::Display for QueryPairSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tab = Table::new();
        tab.force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .set_header(vec!["", &self.header]);

        tab.add_rows(self.rows.iter().map(|(c, i)| {
            let mut row = Row::new();
            row.add_cell(Cell::new(c));
            row.add_cell(Cell::new(format!("{:?}", i)));
            row
        }));
        writeln!(f, "{tab}")
    }
}

#[cfg(test)]
mod tests {
    use crate::matching_repr::bitset::Bitset;

    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::{HashMap, HashSet};

    fn make_lut(keys: &[&str]) -> Lut {
        let mut map = HashMap::new();
        for (i, k) in keys.iter().enumerate() {
            map.insert((*k).to_string(), i);
        }
        Lut::from(map)
    }

    fn make_query_pair(map_a: &[&str], map_b: &[&str]) -> QueryPair {
        QueryPair {
            map_a: map_a.iter().map(|s| (*s).to_string()).collect(),
            map_b: map_b.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[allow(clippy::type_complexity)]
    fn make_query_pair_data(
        left: &[(IdBase, &[(&[IdBase], u64)])],
        right: &[(IdBase, &[(IdBase, u64)])],
    ) -> QueryPairData {
        // left_map: a_index -> (Bitset of b-indices -> count)
        let mut left_map: HashMap<IdBase, HashMap<Bitset, u64>> = HashMap::new();

        for (a_idx, items) in left {
            // Build the inner HashMap for this particular a-index.
            let mut inner: HashMap<Bitset, u64> = HashMap::new();

            for (b_slice, cnt) in *items {
                // Convert the slice of `u8` into a Bitset that the production code
                // expects.  `Bitset::from_idxs` creates a deterministic bitset
                // from the provided indices.
                let bs = Bitset::from_idxs(b_slice);
                inner.insert(bs, *cnt);
            }

            left_map.insert(*a_idx, inner);
        }

        // right_map: b_index -> (a_index -> count)
        let mut right_map: HashMap<IdBase, HashMap<IdBase, u64>> = HashMap::new();

        for (b_idx, items) in right {
            let mut inner: HashMap<IdBase, u64> = HashMap::new();

            for (a_idx, cnt) in *items {
                inner.insert(*a_idx, *cnt);
            }

            right_map.insert(*b_idx, inner);
        }

        (left_map, right_map)
    }

    #[test]
    fn translate_query_pairs_happy_path() -> Result<()> {
        // Two left identifiers, one right identifier.
        let pair = make_query_pair(&["a", "b"], &["C"]);

        let lut_a = make_lut(&["a", "b"]);
        let lut_b = make_lut(&["C"]);

        let (left, right) = translate_query_pairs(&pair, &lut_a, &lut_b)?;

        assert_eq!(left, HashSet::from_iter([0, 1]));
        assert_eq!(right, HashSet::from_iter([0]));
        Ok(())
    }

    #[test]
    fn translate_query_pairs_missing_key_error() {
        // d does not exist in lut_a => Error
        let pair = make_query_pair(&["d"], &["C"]);

        let lut_a = make_lut(&["a"]);
        let lut_b = make_lut(&["C"]);

        let err = translate_query_pairs(&pair, &lut_a, &lut_b).unwrap_err();
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("d not found in lut_a"),
            "error should mention the missing left key"
        );
    }

    #[test]
    fn translate_query_pairs_empty_input() -> Result<()> {
        let pair = make_query_pair(&[], &[]);
        let lut_a = make_lut(&[]);
        let lut_b = make_lut(&[]);

        let (left, right) = translate_query_pairs(&pair, &lut_a, &lut_b)?;
        assert!(
            left.is_empty() && right.is_empty(),
            "both sets should be empty"
        );
        Ok(())
    }

    #[test]
    fn query_pair_report_happy_path() -> Result<()> {
        let qp_data = make_query_pair_data(
            // query the left side => means
            // "a" was querried and it was found 7 times in combination with B and G
            &[(0, &[(&[0u8, 1u8], 7u64)])],
            // query the right side => means
            // G was querried and it was found 3 times in combination with a
            &[(1, &[(0u8, 3u64)])],
        );

        let map_a = vec!["a".to_string()];
        let map_b = vec!["B".to_string(), "G".to_string()];

        let report = QueryPairReport::new(&qp_data, &map_a, &map_b)?;

        assert_eq!(
            report,
            QueryPairReport {
                sections: vec![
                    QueryPairSection {
                        header: "a".to_string(),
                        rows: vec![(7, vec!["B".to_string(), "G".to_string()])]
                    },
                    QueryPairSection {
                        header: "G".to_string(),
                        rows: vec![(3, vec!["a".to_string()])]
                    },
                ]
            }
        );

        Ok(())
    }

    #[test]
    fn query_pair_report_out_of_bounds_error() {
        // Use an a-index that is larger than the supplied map_a.
        let qp_data = make_query_pair_data(&[(5, &[(&[0u8], 1u64)])], &[]);
        let map_a = vec!["OnlyOne".to_string()];
        let map_b = vec!["B".to_string()];

        let err = QueryPairReport::new(&qp_data, &map_a, &map_b).unwrap_err();
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("a index out of bounds"),
            "should surface the out-of-bounds error"
        );
    }
}

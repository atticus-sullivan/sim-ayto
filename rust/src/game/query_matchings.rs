//! This module contains everything needed (in the game module) to query specific matchings. This
//! includes finishing the parsing from the config and generating the final report.

use std::fmt;

use anyhow::{Context, Result};
use comfy_table::{presets::NOTHING, Cell, Row, Table};
use smallvec::SmallVec;

use crate::matching_repr::{bitset::Bitset, MaskedMatching};
use crate::{Lut, MatchingS};

/// Translates the query for a matching
pub(super) fn translate_query_matchings(
    src: &[MatchingS],
    lut_a: &Lut,
    lut_b: &Lut,
) -> Result<Vec<MaskedMatching>> {
    let mut out = Vec::with_capacity(src.len());

    for q in src {
        // start with a zero-filled matrix sized to the left side
        let mut matching: SmallVec<[Bitset; 12]> =
            SmallVec::from_elem(Bitset::empty(), lut_a.len());

        for (k, v) in q {
            // Resolve left-hand side
            let left_idx = *lut_a
                .get(k)
                .with_context(|| format!("{} not found in lut_a", k))?;

            // Resolve right-hand side values
            let mut right_idxs: Vec<IdBase> = v
                .iter()
                .map(|r| {
                    lut_b
                        .get(r)
                        .map(|i| *i as IdBase)
                        .with_context(|| format!("{} not found in lut_b", r))
                })
                .collect::<Result<_>>()?;

            right_idxs.sort(); // deterministic order
            matching[left_idx] = Bitset::from_idxs(&right_idxs);
        }

        out.push(matching.into());
    }

    Ok(out)
}

#[derive(Debug, PartialEq)]
pub(super) struct MatchingReport {
    entries: Vec<MatchingEntry>,
}

impl MatchingReport {
    /// Only call when query_matchings actually contains data
    pub(super) fn new(
        query_matchings: &[(MaskedMatching, Option<String>)],
        map_a: &[String],
        map_b: &[String],
    ) -> Result<Option<Self>> {
        if !query_matchings.iter().any(|(_, x)| x.is_some()) {
            return Ok(None);
        }

        let mut entries = Vec::with_capacity(query_matchings.len());
        for (q, id) in query_matchings {
            let Some(id) = id else { continue };
            entries.push(MatchingEntry::new(q, id, map_a, map_b)?);
        }
        Ok(Some(MatchingReport { entries }))
    }

    pub(super) fn tab_cnt(&self) -> usize {
        self.entries.len()
    }
}

impl fmt::Display for MatchingReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Trace at which point a particular matching was elimiated:"
        )?;

        for entry in &self.entries {
            writeln!(f, "{}", entry)?;
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
struct MatchingEntry {
    rows: Vec<(String, Vec<String>)>,
    eliminated_in: String,
}

impl MatchingEntry {
    fn new(q: &MaskedMatching, id: &str, map_a: &[String], map_b: &[String]) -> Result<Self> {
        let mut rows = Vec::with_capacity(q.len());
        for (a, b) in q.iter().enumerate() {
            let a_s = map_a
                .get(a)
                .with_context(|| format!("{} is out of bounds for map_a", a))?
                .clone();

            let b_s = b
                .iter()
                .map(|b| {
                    map_b
                        .get(b as usize)
                        .cloned()
                        .with_context(|| format!("{} is out of bounds for map_b", b))
                })
                .collect::<Result<Vec<_>>>()?;

            rows.push((a_s, b_s));
        }

        Ok(MatchingEntry {
            rows,
            eliminated_in: id.to_string(),
        })
    }
}

impl fmt::Display for MatchingEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tab = Table::new();
        tab
            .force_no_tty()
            .enforce_styling()
            .load_preset(NOTHING)
            .set_style(comfy_table::TableComponent::VerticalLines, '\u{2192}')
        // .set_style(comfy_table::TableComponent::VerticalLines, '\u{21D2}')
        // .set_style(comfy_table::TableComponent::VerticalLines, '\u{21E8}')
        // .set_style(comfy_table::TableComponent::VerticalLines, '\u{21FE}')
        ;
        tab.add_rows(self.rows.iter().map(|(a, b)| {
            let mut row = Row::new();
            row.add_cell(Cell::new(a));
            row.add_cell(Cell::new(format!("{:?}", b)));
            row
        }));
        tab.column_mut(0).ok_or(fmt::Error)?.set_padding((0, 1));
        writeln!(f, "{tab}")?;
        writeln!(f, "=> Eliminated in {}", self.eliminated_in)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use pretty_assertions::assert_eq;

    use super::*;

    // Helper to build a simple LUT (lookup table) from a slice of strings.
    fn make_lut(keys: &[&str]) -> Lut {
        let mut map = HashMap::new();
        for (i, k) in keys.iter().enumerate() {
            map.insert((*k).to_string(), i);
        }
        Lut::from(map)
    }

    fn make_src(x: Vec<Vec<(String, Vec<String>)>>) -> Vec<MatchingS> {
        x.iter()
            .map(|xs| {
                xs.iter()
                    .map(|(a, bs)| {
                        (
                            a.to_string(),
                            bs.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
                        )
                    })
                    .collect::<HashMap<_, _>>()
            })
            .collect::<Vec<_>>()
    }

    #[test]
    fn translate_query_matchings_simple() -> Result<()> {
        // Input: one query that maps "a" -> {"x", "y"} and "b" -> {"z"}
        let src = make_src(vec![vec![
            ("a".to_string(), vec!["x".to_string(), "y".to_string()]),
            ("b".to_string(), vec!["z".to_string()]),
        ]]);

        // Build lookup tables for the left-hand side (A) and right-hand side (B)
        let lut_a = make_lut(&["a", "b"]);
        let lut_b = make_lut(&["x", "y", "z"]);

        // Run the function under test
        let out = translate_query_matchings(&src, &lut_a, &lut_b)?;

        // Expected bitsets:
        // - row 0 (for "a") contains indices of "x" and "y" -> [0,1]
        // - row 1 (for "b") contains index of "z"          -> [2]
        assert_eq!(
            out,
            vec![MaskedMatching::from_matching_ref(&[vec![0, 1], vec![2],])]
        );
        Ok(())
    }

    #[test]
    fn translate_query_matchings_missing_key_errors() {
        // Query refers to a key that does not exist in the LUTs.
        let src = make_src(vec![vec![(
            "missing_left".to_string(),
            vec!["x".to_string()],
        )]]);
        let lut_a = make_lut(&["a"]);
        let lut_b = make_lut(&["x"]);

        // The function should return an Err with a helpful context.
        let err = translate_query_matchings(&src, &lut_a, &lut_b).unwrap_err();
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("missing_left not found in lut_a"),
            "error message should mention the missing left key"
        );
    }

    #[test]
    fn translate_query_matchings_empty_input_returns_empty_vec() -> Result<()> {
        let src: Vec<MatchingS> = vec![];
        let lut_a = make_lut(&[]);
        let lut_b = make_lut(&[]);

        let out = translate_query_matchings(&src, &lut_a, &lut_b)?;
        assert!(out.is_empty(), "empty input should yield an empty result");
        Ok(())
    }

    #[test]
    fn matching_entry_new_simple() -> Result<()> {
        // Build a MaskedMatching manually:
        //   left index 0 -> right indices {0,2}
        //   left index 1 -> right index {1}
        let masked = MaskedMatching::from_matching_ref(&[vec![0, 2], vec![1]]);

        let map_a = vec!["left0".to_string(), "left1".to_string()];
        let map_b = vec![
            "right0".to_string(),
            "right1".to_string(),
            "right2".to_string(),
        ];

        let entry = MatchingEntry::new(&masked, "step-42", &map_a, &map_b)?;

        // Verify the human-readable rows.
        assert_eq!(
            entry,
            MatchingEntry {
                rows: vec![
                    (
                        "left0".to_string(),
                        vec!["right0", "right2"]
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                    ),
                    (
                        "left1".to_string(),
                        vec!["right1"]
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                    ),
                ],
                eliminated_in: "step-42".to_string(),
            }
        );

        Ok(())
    }

    #[test]
    fn matching_entry_new_out_of_bounds_error() {
        // MaskedMatching references an index that does not exist in map_b.
        let masked = MaskedMatching::from_matching_ref(&[vec![50]]); // 99 is out of range
        let map_a = vec!["only".to_string()];
        let map_b = vec!["right0".to_string()];

        let err = MatchingEntry::new(&masked, "id", &map_a, &map_b);
        let msg = format!("{:?}", err.unwrap_err());
        assert!(
            msg.contains("50 is out of bounds for map_b"),
            "should report out-of-bounds for map_b"
        );
    }

    #[test]
    fn matching_report_new_simple() -> Result<()> {
        // Two matchings, each with an identifier.
        let m1 = MaskedMatching::from_matching_ref(&[vec![0]]);
        let m2 = MaskedMatching::from_matching_ref(&[vec![1]]);

        let query_matchings = vec![
            (m1, Some("step-1".to_string())),
            (m2, Some("step-2".to_string())),
        ];

        let map_a = vec!["A".to_string()];
        let map_b = vec!["B0".to_string(), "B1".to_string()];

        let report_opt = MatchingReport::new(&query_matchings, &map_a, &map_b)?;
        assert!(
            report_opt.is_some(),
            "report should be created when IDs exist"
        );

        let report = report_opt.unwrap();
        assert_eq!(report.tab_cnt(), 2);
        assert_eq!(
            report,
            MatchingReport {
                entries: vec![
                    MatchingEntry {
                        rows: vec![("A".to_string(), vec!["B0".to_string(),],),],
                        eliminated_in: "step-1".to_string(),
                    },
                    MatchingEntry {
                        rows: vec![("A".to_string(), vec!["B1".to_string(),],),],
                        eliminated_in: "step-2".to_string(),
                    },
                ],
            }
        );

        Ok(())
    }

    #[test]
    fn matching_report_new_none_when_all_ids_missing() -> Result<()> {
        // All identifiers are None - the constructor should return Ok(None).
        let m = MaskedMatching::from_matching_ref(&[vec![0]]);
        let query_matchings = vec![(m, None)];

        let map_a = vec!["A".to_string()];
        let map_b = vec!["B".to_string()];

        let report_opt = MatchingReport::new(&query_matchings, &map_a, &map_b)?;
        assert!(
            report_opt.is_none(),
            "no report should be produced when no IDs"
        );
        Ok(())
    }

    #[test]
    fn matching_entry_display_looks_reasonable() -> Result<()> {
        let masked = MaskedMatching::from(SmallVec::from_slice(&[Bitset::from_idxs(&[0, 1])]));
        let map_a = vec!["alpha".to_string()];
        let map_b = vec!["beta".to_string(), "gamma".to_string()];

        let entry = MatchingEntry::new(&masked, "elim-99", &map_a, &map_b)?;
        let rendered = format!("{}", entry);

        // The output should contain the arrow and the elimination marker.
        assert!(rendered.contains("=> Eliminated in elim-99"));
        assert!(rendered.contains("alpha"));
        assert!(rendered.contains("[\"beta\", \"gamma\"]"));
        Ok(())
    }
}

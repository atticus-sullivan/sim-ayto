use std::fmt;

use anyhow::{Context, Result};
use comfy_table::{presets::NOTHING, Cell, Row, Table};

use crate::{matching_repr::MaskedMatching, Lut, Matching, MatchingS};

pub(super) fn translate_query_matchings(
    src: &[MatchingS],
    lut_a: &Lut,
    lut_b: &Lut,
) -> Result<Vec<MaskedMatching>> {
    let mut out = Vec::with_capacity(src.len());

    for q in src {
        // TODO: directly translate to masked_matching
        // start with a zero‑filled matrix sized to the left side
        let mut matching: Matching = vec![vec![0]; lut_a.len()];

        for (k, v) in q {
            // Resolve left‑hand side
            let left_idx = *lut_a
                .get(k)
                .with_context(|| format!("{} not found in lut_a", k))?;

            // Resolve right‑hand side values
            let mut right_idxs: Vec<u8> = v
                .iter()
                .map(|r| {
                    lut_b
                        .get(r)
                        .map(|i| *i as u8)
                        .with_context(|| format!("{} not found in lut_b", r))
                })
                .collect::<Result<_>>()?;

            right_idxs.sort(); // deterministic order
            matching[left_idx] = right_idxs;
        }

        out.push(matching.into());
    }

    Ok(out)
}


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

// TODO: test for translation
// TODO: test for MatchingEntry::new
// TODO: test for MatchingReport::new

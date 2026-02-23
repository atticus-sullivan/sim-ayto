use std::{collections::HashSet, fmt};

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Row, Table};

use crate::{game::parse::QueryPair, iterstate::QueryPairData, Lut};

pub(super) fn translate_query_pairs(
    pair: &QueryPair,
    lut_a: &Lut,
    lut_b: &Lut,
) -> Result<(HashSet<u8>, HashSet<u8>)> {
    let mut left = HashSet::new();
    let mut right = HashSet::new();

    for a in &pair.map_a {
        let idx = *lut_a
            .get(a)
            .with_context(|| format!("{} not found in lut_a", a))? as u8;
        left.insert(idx);
    }
    for b in &pair.map_b {
        let idx = *lut_b
            .get(b)
            .with_context(|| format!("{} not found in lut_b", b))? as u8;
        right.insert(idx);
    }
    Ok((left, right))
}


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

// TODO: test for translation
// TODO: test for QueryPairReport::new

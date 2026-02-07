/*
sim_ayto
Copyright (C) 2024  Lukas Heindl

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

pub mod parse;
mod sim;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};

use indicatif::{ProgressBar, ProgressStyle};

use serde_json::to_writer;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::Result;

use crate::constraint::eval::{CSVEntry, CSVEntryMB, CSVEntryMN, SumCounts};
use crate::constraint::Constraint;
use crate::ruleset::RuleSet;
use crate::{Lut, Matching, Rem};

// colors for tables
const COLOR_ROW_MAX: Color = Color::Rgb {
    r: 69,
    g: 76,
    b: 102,
};
const COLOR_BOTH_MAX: Color = Color::Rgb {
    r: 65,
    g: 77,
    b: 71,
};
const COLOR_COL_MAX: Color = Color::Rgb {
    r: 74,
    g: 68,
    b: 89,
};

pub const COLOR_ALT_BG: Color = Color::Rgb {
    r: 41,
    g: 44,
    b: 60,
};

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum DumpMode {
    Full,
    FullNames,
    Winning,
    WinningNames,
}

use permutator::CartesianProduct;

fn foreach_unwrapped_matching<F>(matching: &[Vec<u8>], mut f: F)
where
    F: FnMut(Vec<&u8>),
{
    let matching_slices: Vec<&[u8]> = matching.iter().map(|v| v.as_slice()).collect();
    for p in matching_slices.cart_prod() {
        f(p);
    }
}

#[derive(Debug)]
pub struct Game {
    solved: bool,
    constraints_orig: Vec<Constraint>,
    rule_set: RuleSet,
    frontmatter: serde_yaml::Value,

    // maps u8/usize to string
    map_a: Vec<String>,
    map_b: Vec<String>,

    // maps string to usize
    lut_a: Lut,
    lut_b: Lut,

    dir: PathBuf,
    stem: String,
    query_matchings: Vec<Matching>,
    query_pair: (HashSet<u8>, HashSet<u8>),
    cache_file: Option<PathBuf>,
    final_cache_hash: Option<PathBuf>,
}

impl Game {
    // returns (translationKeyForExplanation, shortcode)
    pub fn ruleset_str(&self) -> (String, String) {
        match &self.rule_set {
            RuleSet::XTimesDup((cnt, fixed)) => (
                format!("rs-XTimesDup-{}-{}", fixed.len(), cnt),
                format!("?{cnt}={}", fixed.len()),
            ),
            RuleSet::SomeoneIsTrip => ("rs-SomeoneIsTrip".to_string(), "?3".to_string()),
            RuleSet::NToN => ("rs-NToN".to_string(), "N:N".to_string()),
            RuleSet::FixedTrip(_) => ("rs-FixedTrip".to_string(), "=3".to_string()),
            RuleSet::Eq => ("rs-Eq".to_string(), "=".to_string()),
        }
    }
    pub fn players_str(&self) -> String {
        format!("{}/{}", self.map_a.len(), self.map_b.len())
    }

    fn md_output(&self, out: &mut File, md_tables: &[(String, u16, bool, bool)]) -> Result<()> {
        writeln!(out, "---")?;
        writeln!(out, "{}", serde_yaml::to_string(&self.frontmatter)?)?;
        writeln!(out, "---")?;

        let stem = &self.stem;

        writeln!(out, "\n{{{{% translateHdr \"tab-current\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
        writeln!(out, "{{{{% details closed=\"true\" %}}}}")?;
        writeln!(
            out,
            "{{{{% img src=\"/{stem}/{stem}_tab.png\" %}}}}"
        )?;
        writeln!(
            out,
            "{{{{% img src=\"/{stem}/{stem}_sum.png\" %}}}}"
        )?;
        writeln!(out, "{{{{% /details %}}}}")?;

        writeln!(out, "\n{{{{% translateHdr \"tab-individual\" %}}}}")?;
        for (name, idx, tree, detail) in md_tables.iter() {
            if *detail {
                writeln!(out, "\n{{{{% details title=\"{name}\" closed=\"true\" %}}}}")?;
            } else {
                writeln!(out, "\n{{{{% translatedDetails \"{name}\" %}}}}")?;
            }

            writeln!(
                out,
                "{{{{% img src=\"/{stem}/{stem}_{idx}.png\" %}}}}"
            )?;
            if *tree {
                writeln!(
                    out,
                    "{{{{% img src=\"/{stem}/{stem}_{idx}_tree.png\" %}}}}"
                )?;
            }

            if *detail {
                writeln!(out, "{{{{% /details %}}}}")?;
            } else {
                writeln!(out, "{{{{% /translatedDetails %}}}}")?;
            }
        }

        writeln!(out, "\n{{{{% translateHdr \"tab-everything\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
        writeln!(out, "{{{{% details closed=\"true\" %}}}}")?;
        writeln!(
            out,
            "{{{{% img src=\"/{stem}/{stem}.col.png\" %}}}}"
        )?;
        writeln!(out, "{{{{% /details %}}}}")?;

        Ok(())
    }

    fn do_statistics(&self, total: f64, merged_constraints: &[Constraint], solutions: Option<&Vec<Matching>>) -> Result<()> {
        let out_mb_path = self.dir.join("statMB").with_extension("csv");
        let out_mn_path = self.dir.join("statMN").with_extension("csv");
        let out_info_path = self.dir.join("statInfo").with_extension("csv");
        let out_sum_path = self.dir.join("statSum").with_extension("json");

        let (mut mbo, mut mno, mut info) = (
            csv::WriterBuilder::new()
                .delimiter(b',')
                .has_headers(false)
                .from_path(out_mb_path)?,
            csv::WriterBuilder::new()
                .delimiter(b',')
                .has_headers(false)
                .from_path(out_mn_path)?,
            csv::WriterBuilder::new()
                .delimiter(b',')
                .has_headers(false)
                .from_path(out_info_path)?,
        );
        info.serialize(CSVEntry {
            num: 0.0,
            lights_total: None,
            lights_known_before: 0,
            bits_left: total.log2(),
            comment: "initial".to_string(),
        })?;
        for i in merged_constraints.iter().map(|c| {
            c.get_stats(
                self.rule_set
                    .constr_map_len(self.lut_a.len(), self.lut_b.len()),
            )
        }) {
            let i = i?;
            if let Some(j) = &i.1 {
                let j = CSVEntryMN {
                    num: j.num,
                    won: j.won,
                    lights_total: j.lights_total,
                    lights_known_before: j.lights_known_before,
                    bits_gained: j.bits_gained,
                    comment: j.comment.clone(),
                };
                mno.serialize(j)?
            }
            if let Some(j) = &i.2 {
                let j = CSVEntry {
                    num: j.num,
                    lights_total: j.lights_total,
                    lights_known_before: j.lights_known_before,
                    bits_left: j.bits_left,
                    comment: j.comment.clone(),
                };
                info.serialize(j)?
            }
            // potentially updates known_lights -> do this in the end of the loop
            if let Some(j) = &i.0 {
                let j = CSVEntryMB {
                    num: j.num,
                    lights_total: j.lights_total,
                    lights_known_before: j.lights_known_before,
                    bits_gained: j.bits_gained,
                    comment: j.comment.clone(),
                };
                mbo.serialize(j)?
            }
        }
        mbo.flush()?;
        mno.flush()?;
        info.flush()?;

        let mut cnt = SumCounts{
            blackouts: 0,
            sold: 0,
            sold_but_match: 0,
            sold_but_match_active: solutions.is_some(),
            matches_found: 0,
        };
        for c in merged_constraints.iter() {
            if c.is_blackout() {
                cnt.blackouts += 1;
            }
            if c.is_sold() {
                cnt.sold += 1;
            }
            if c.is_match_found() {
                cnt.matches_found += 1;
            }
            if c.is_sold() && c.is_mb_hit(solutions) {
                cnt.sold_but_match += 1;
            }
        }
        let file = File::create(out_sum_path)?;
        let mut writer = BufWriter::new(file);

        serde_json::to_writer(&mut writer, &cnt)?;
        writer.flush()?;

        self.summary_table(false, merged_constraints)?;
        self.summary_table(true, merged_constraints)?;
        Ok(())
    }

    fn summary_table(&self, transpose: bool, merged_constraints: &[Constraint]) -> Result<()> {
        // let map_vert;
        let map_hor = if !transpose {
            &self.map_a
            // map_vert = &self.map_b;
        } else {
            &self.map_b
            // map_vert = &self.map_a;
        };

        let mut hdr = vec![
            Cell::new(""),
            Cell::new("L").set_alignment(comfy_table::CellAlignment::Center),
        ];
        hdr.extend(
            map_hor
                .iter()
                .map(|x| Cell::new(x).set_alignment(comfy_table::CellAlignment::Center)),
        );
        hdr.push(Cell::new("").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("I").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("#new").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("min dist").set_alignment(comfy_table::CellAlignment::Center));

        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);

        let mut past_constraints: Vec<&Constraint> = Vec::default();
        for (i, c) in merged_constraints.iter().enumerate() {
            if i % 2 == 0 {
                table.add_row(
                    c.stat_row(transpose, map_hor, &past_constraints)
                        .into_iter()
                        .map(|i| i.bg(COLOR_ALT_BG)),
                );
            } else {
                table.add_row(c.stat_row(transpose, map_hor, &past_constraints));
            }
            past_constraints.push(c);
        }
        println!("{table}");
        Ok(())
    }

    fn print_rem_generic(
        &self,
        rem: &Rem,
        map_vert: &[String],
        map_hor: &[String],
        norm_idx: fn(v: usize, h: usize) -> (usize, usize),
    ) -> Result<()> {
        let table_content = map_vert
            .iter()
            .enumerate()
            .map(|(vert_idx, vert_name)| {
                (
                    vert_name,
                    map_hor
                        .iter()
                        .enumerate()
                        .map(|(hor_idx, _)| {
                            let (vert_idx, hor_idx) = norm_idx(vert_idx, hor_idx);
                            if self.rule_set.ignore_pairing(vert_idx, hor_idx) {
                                None // ignore/empty
                            } else {
                                // calculate remaining percentage
                                let x = rem.0[vert_idx][hor_idx];
                                let val = (x as f64) / (rem.1 as f64) * 100.0;
                                Some(val)
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        // mapping of y-coord to indices with maximum
        let hor_max = table_content
            .iter()
            .map(|(_, vals)| {
                vals.iter()
                    .enumerate()
                    // iterator over the values in current row (skips the None entries)
                    .filter_map(|(col_idx, value)| value.map(|value| (col_idx, value)))
                    // find all indices containing the maximum
                    .fold((vec![], 0.0), |mut acc, (col_idx, value)| {
                        if acc.1 < value {
                            acc.0 = vec![col_idx];
                            acc.1 = value;
                        } else if acc.1 == value {
                            acc.0.push(col_idx);
                        }
                        acc
                    })
            })
            .collect::<Vec<_>>();

        // mapping of x-coord to indices with maximum
        let vert_max = (0..table_content[0].1.len())
            // loop over columns
            .map(|col_idx| {
                table_content
                    .iter()
                    .enumerate()
                    // iterator over the values in current column (skips the None entries)
                    .filter_map(|(row_idx, row)| row.1[col_idx].map(|value| (row_idx, value)))
                    // find all indices containing the maximum
                    .fold((vec![], 0.0), |mut acc, (row_idx, value)| {
                        if acc.1 < value {
                            acc.0 = vec![row_idx];
                            acc.1 = value;
                        } else if acc.1 == value {
                            acc.0.push(row_idx);
                        }
                        acc
                    })
            })
            .collect::<Vec<_>>();

        let mut table = Table::new();
        {
            let mut hdr = vec![Cell::new("")];
            hdr.extend(map_hor.iter().enumerate().map(|(i, x)| {
                if vert_max[i].1 == 100.0 {
                    Cell::new(x).fg(Color::Green)
                } else {
                    Cell::new(x)
                }
                .set_alignment(comfy_table::CellAlignment::Center)
                .bg(COLOR_COL_MAX)
            }));

            table
                .force_no_tty()
                .enforce_styling()
                .load_preset(UTF8_FULL_CONDENSED)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_header(hdr);
        }

        for (j, i) in table_content.iter().enumerate() {
            let mut row = vec![];
            row.push(
                if hor_max[j].1 == 100.0 {
                    Cell::new(i.0).fg(Color::Green)
                } else {
                    Cell::new(i.0)
                }
                .bg(COLOR_ROW_MAX),
            );

            // put formatted table content into the table
            row.extend(
                i.1.iter()
                    .enumerate()
                    // format according to value (sets the foreground)
                    .map(|(idx, val)| {
                        match val {
                            Some(val) => {
                                let val = *val;
                                if 79.0 < val && val < 101.0 {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    )
                                    .fg(Color::Green))
                                } else if 55.0 <= val {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    )
                                    .fg(Color::Cyan))
                                } else if 45.0 < val {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    )
                                    .fg(Color::Yellow))
                                } else if 1.0 < val {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    ))
                                } else if -1.0 < val {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    )
                                    .fg(Color::Red))
                                } else {
                                    Ok(Cell::new(""))
                                }
                            }
                            None => Ok(Cell::new("")),
                        }
                        .map(|cell| {
                            // format according to row and maxima (uses background)
                            let max_h = hor_max[j].0.contains(&idx);
                            let max_v = vert_max[idx].0.contains(&j);
                            if max_h {
                                if max_v {
                                    // max for both
                                    cell.bg(COLOR_BOTH_MAX)
                                } else {
                                    // row max
                                    cell.bg(COLOR_ROW_MAX)
                                }
                            } else {
                                #[allow(clippy::collapsible_else_if)]
                                if max_v {
                                    // column max
                                    cell.bg(COLOR_COL_MAX)
                                } else {
                                    // neither
                                    if j % 2 == 0 {
                                        cell.bg(COLOR_ALT_BG)
                                    } else {
                                        cell
                                    }
                                }
                            }
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            );

            table.add_row(row);
        }

        println!("{table}");
        println!(
            "{} left -> {} bits left",
            rem.1,
            format!("{:.4}", (rem.1 as f64).log2())
                .trim_end_matches('0')
                .trim_end_matches('.')
        );
        Ok(())
    }
}

pub struct IterState {
    constraints: Vec<Constraint>,
    keep_rem: bool,
    each: Vec<Vec<u128>>,
    total: u128,
    eliminated: u128,
    pub left_poss: Vec<Matching>,
    query_matchings: Vec<(Matching, Option<String>)>,
    query_pair: (
        HashMap<u8, HashMap<Vec<u8>, u64>>,
        HashMap<u8, HashMap<Vec<u8>, u64>>,
    ),
    cnt_update: usize,
    progress: ProgressBar,
    cache_file: Option<BufWriter<File>>,
}

impl IterState {
    pub fn new(
        keep_rem: bool,
        perm_amount: usize,
        constraints: Vec<Constraint>,
        query_matchings: &[Matching],
        query_pair: &(HashSet<u8>, HashSet<u8>),
        cache_file: &Option<PathBuf>,
        map_lens: (usize, usize),
    ) -> Result<IterState> {
        let file = cache_file
            .clone()
            .map(|f| File::create(f))
            .map_or(Ok(None), |r| r.map(Some))?
            .map(|g| BufWriter::new(g));
        let is = IterState {
            constraints,
            keep_rem,
            query_matchings: query_matchings.iter().map(|i| (i.clone(), None)).collect(),
            query_pair: (
                query_pair
                    .0
                    .iter()
                    .map(|i| (*i, Default::default()))
                    .collect(),
                query_pair
                    .1
                    .iter()
                    .map(|i| (*i, Default::default()))
                    .collect(),
            ),
            each: vec![vec![0; map_lens.1]; map_lens.0],
            total: 0,
            eliminated: 0,
            left_poss: vec![],
            progress: ProgressBar::new(100),
            cnt_update: std::cmp::max(perm_amount / 50, 1),
            cache_file: file,
        };
        is.progress.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar}] {pos:>3}/{len:3} (ETA: {eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );
        Ok(is)
    }

    pub fn start(&mut self) {
        self.progress.inc(0)
    }

    pub fn finish(&mut self) {
        self.progress.finish()
    }

    pub fn step(&mut self, i: usize, p: Matching, output: bool) -> Result<()> {
        if i % self.cnt_update == 0 && output {
            self.progress.inc(2);
        }
        for (a, i) in p.iter().enumerate() {
            for b in i.iter() {
                if let Some(x) = self.each.get_mut(a) {
                    if let Some(x_val) = x.get_mut(*b as usize) {
                        *x_val += 1;
                    }
                }
            }
        }
        self.total += 1;
        let mut left = true;
        for c in &mut self.constraints {
            if !c.process(&p)? {
                left = false;
                self.eliminated += 1;
                for (q, id) in &mut self.query_matchings {
                    if q == &p {
                        *id = Some(c.type_str().to_string() + " " + c.comment());
                    }
                }
                break;
            }
        }
        if left {
            if !self.query_pair.0.is_empty() || !self.query_pair.1.is_empty() {
                for (a, bs) in p.iter().enumerate() {
                    if self.query_pair.0.contains_key(&(a as u8)) {
                        if let Some(val) = self.query_pair.0.get_mut(&(a as u8)) {
                            val.entry(bs.clone())
                                .and_modify(|cnt| *cnt += 1)
                                .or_insert(0);
                        };
                    }
                    for b in bs.iter() {
                        if let Some(val) = self.query_pair.1.get_mut(b) {
                            val.entry(vec![a as u8])
                                .and_modify(|cnt| *cnt += 1)
                                .or_insert(0);
                        };
                    }
                }
            }
            if let Some(fs) = &mut self.cache_file {
                to_writer(&mut *fs, &p)?;
                writeln!(fs)?;
            }
            if self.keep_rem {
                self.left_poss.push(p);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwraped_matching() {
        let m = vec![
            vec![6],
            vec![3],
            vec![2, 10],
            vec![4],
            vec![1],
            vec![5],
            vec![0],
            vec![7],
            vec![8],
            vec![9],
        ];
        foreach_unwrapped_matching(&m, |m| println!("{:?}", m));
        println!();
        let m = vec![
            vec![6],
            vec![3],
            vec![2, 10],
            vec![4],
            vec![1],
            vec![5, 20],
            vec![0],
            vec![7],
            vec![8],
            vec![9],
        ];
        foreach_unwrapped_matching(&m, |m| println!("{:?}", m));
    }
}

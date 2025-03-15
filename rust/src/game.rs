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

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{NOTHING, UTF8_FULL_CONDENSED};
use comfy_table::{Cell, Color, Row, Table};

use indicatif::{ProgressBar, ProgressStyle};

use serde::Deserialize;

use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Context, Result};

use crate::constraint::Constraint;
use crate::constraint::ConstraintParse;
use crate::ruleset::RuleSet;
use crate::{Lut, Matching, MatchingS, Rem, Rename};

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

const COLOR_ALT_BG: Color = Color::Rgb {
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

fn foreach_unwrapped_matching<F>(matching: &Vec<Vec<u8>>, mut f: F)
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
    constraints_orig: Vec<Constraint>,
    rule_set: RuleSet,
    tree_gen: bool,
    frontmatter: serde_yaml::Value,
    tree_top: Option<String>,

    // maps u8/usize to string
    map_a: Vec<String>,
    map_b: Vec<String>,

    // maps string to usize
    lut_a: Lut,
    lut_b: Lut,

    dir: PathBuf,
    stem: String,
    query_matchings: Vec<Matching>,
}

// this struct is only used for parsing the yaml file
#[derive(Deserialize, Debug)]
struct GameParse {
    #[serde(rename = "constraints")]
    constraints_orig: Vec<ConstraintParse>,
    rule_set: RuleSet,
    tree_gen: bool,
    frontmatter: serde_yaml::Value,
    tree_top: Option<String>,
    #[serde(rename = "queryMatchings", default)]
    query_matchings_s: Vec<MatchingS>,

    #[serde(rename = "setA")]
    map_a: Vec<String>,
    #[serde(rename = "setB")]
    map_b: Vec<String>,

    #[serde(rename = "renameA", default)]
    rename_a: Rename,
    #[serde(rename = "renameB", default)]
    rename_b: Rename,
}

impl Game {
    // returns (translationKeyForExplanation, shortcode)
    pub fn ruleset_str(self: &Self) -> (&str, &str) {
        match self.rule_set{
            RuleSet::SomeoneIsDup => ("rs-SomeoneIsDup", "?2"),
            RuleSet::SomeoneIsTrip => ("rs-SomeoneIsTrip", "?3"),
            RuleSet::NToN => ("rs-NToN", "N:N"),
            RuleSet::FixedDup(_) => ("rs-FixedDup", "=2"),
            RuleSet::FixedTrip(_) => ("rs-FixedTrip", "=3"),
            RuleSet::Eq => ("rs-Eq", "="),
        }
    }
    pub fn players_str(self: &Self) -> String {
        format!("{}/{}", self.map_a.len(), self.map_b.len())
    }

    pub fn new_from_yaml(yaml_path: &Path, stem: &Path) -> Result<Game> {
        let gp: GameParse = serde_yaml::from_reader(File::open(yaml_path)?)?;

        let mut g = Game {
            map_a: gp.map_a,
            map_b: gp.map_b,
            constraints_orig: Vec::default(),
            rule_set: gp.rule_set,
            tree_gen: gp.tree_gen,
            tree_top: gp.tree_top,
            dir: stem
                .parent()
                .context("parent dir of stem not found")?
                .to_path_buf(),
            stem: stem
                .file_stem()
                .context("No filename provided in stem")?
                .to_string_lossy()
                .into_owned(),
            lut_a: Lut::default(),
            lut_b: Lut::default(),
            query_matchings: Vec::default(),
            frontmatter: gp.frontmatter,
        };

        // build up the look up tables (LUT)
        for (lut, map) in [(&mut g.lut_a, &g.map_a), (&mut g.lut_b, &g.map_b)] {
            for (index, name) in map.iter().enumerate() {
                lut.insert(name.clone(), index);
            }
        }

        ensure!(g.lut_a.len() == g.map_a.len(), "something is wrong with the sets. There might be duplicates in setA (len: {}, dedup len: {}).", g.lut_a.len(), g.map_a.len());
        ensure!(g.lut_b.len() == g.map_b.len(), "something is wrong with the sets. There might be duplicates in setB (len: {}, dedup len: {}).", g.lut_b.len(), g.map_b.len());
        // validate the lut in combination with the ruleset
        g.rule_set.validate_lut(&g.lut_a, &g.lut_b)?;

        // eg translates strings to indices (u8) but also adds the exclude rules if the ruleset demands it as well as sorts if the ruleset needs it
        for c in gp.constraints_orig {
            g.constraints_orig.push(c.finalize_parsing(
                &g.lut_a,
                &g.lut_b,
                g.rule_set.constr_map_len(g.lut_a.len(), g.lut_b.len()),
                &g.map_b,
                g.rule_set.must_add_exclude(),
                g.rule_set.must_sort_constraint(),
                (&gp.rename_a, &gp.rename_b)
            )?);
        }

        // translate the matchings that were querried for tracing
        for q in &gp.query_matchings_s {
            let mut matching: Matching = vec![vec![0]; g.lut_a.len()];
            for (k, v) in q {
                let x = v
                    .iter()
                    .map(|v| {
                        g.lut_b
                            .get(v)
                            .map(|v| *v as u8)
                            .with_context(|| format!("{} not found in lut_b", v))
                    })
                    .collect::<Result<Vec<_>>>()?;
                matching[*g
                    .lut_a
                    .get(k)
                    .with_context(|| format!("{} not found in lut_a", k))?] = x;
            }
            g.query_matchings.push(matching);
        }

        // rename names in map_a and map_b for output use
        for (rename, map) in [(&gp.rename_a, &mut g.map_a), (&gp.rename_b, &mut g.map_b)] {
            for n in map {
                *n = rename.get(n).unwrap_or(n).to_owned();
            }
        }

        Ok(g)
    }

    pub fn sim(&mut self, print_transposed: bool, dump_mode: Option<DumpMode>) -> Result<()> {
        let perm_amount = self
            .rule_set
            .get_perms_amount(self.map_a.len(), self.map_b.len());

        let mut is = IterState::new(
            self.tree_gen || dump_mode.is_some(),
            perm_amount,
            self.constraints_orig.clone(),
            &self.query_matchings,
        );
        self.rule_set
            .iter_perms(&self.lut_a, &self.lut_b, &mut is, true)?;

        // fix is so that it can't be mutated anymore
        let is = &is;

        // track table indices
        let mut tab_idx = 0;
        let mut md_tables: Vec<(String,u16,bool)> = vec![];

        // generate additional tables
        if is.query_matchings.iter().any(|(_, x)| x.is_some()) {
            println!("Trace at which point a particular matching was elimiated:");
            for (q, id) in &is.query_matchings {
                match id {
                    Some(id) => {
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
                        let mut rows = vec![Row::new(); q.len()];
                        for (a, b) in q.iter().enumerate() {
                            let ass = self.map_a.get(a).unwrap();
                            let bs = b
                                .iter()
                                .map(|b| self.map_b.get(*b as usize).unwrap())
                                .collect::<Vec<_>>();
                            rows[a].add_cell(ass.into());
                            rows[a].add_cell(format!("{:?}", bs).into());
                        }
                        tab.add_rows(rows);
                        tab.column_mut(0)
                            .context("no 0th column in table found")?
                            .set_padding((0, 1));
                        println!("{tab}");
                        println!("=> Eliminated in {}", id);
                        tab_idx += 1;
                    }
                    None => {}
                }
            }
            println!();
        }

        let mut rem: Rem = (
            vec![vec![is.each; self.map_b.len()]; self.map_a.len()],
            is.total,
        );
        if print_transposed {
            self.print_rem_generic(&rem, &self.map_b, &self.map_a, |v, h| (h, v))
                .context("Error printing")?;
        } else {
            self.print_rem_generic(&rem, &self.map_a, &self.map_b, |v, h| (v, h))
                .context("Error printing")?;
        }
        md_tables.push(("tab-start".to_owned(), tab_idx, false));
        tab_idx += 1;
        println!();

        let mut constr = vec![];
        let mut to_merge = vec![]; // collect hidden constraints to merge them down
        let mut past_constraints: Vec<&Constraint> = Vec::default();
        for c_ in &is.constraints {
            if c_.should_merge() {
                to_merge.push(c_);
            } else {
                let mut c = c_.clone();
                // merge down previous hidden constraints
                while !to_merge.is_empty() {
                    c.merge(to_merge.pop().unwrap())?;
                }
                rem = c.apply_to_rem(rem).context("Apply to rem failed")?;
                c.print_hdr(&past_constraints)?;
                if c.show_rem_table() {
                    if print_transposed {
                        self.print_rem_generic(&rem, &self.map_b, &self.map_a, |v, h| (h, v))
                            .context("Error printing")?;
                        md_tables.push((c.md_title(), tab_idx, true));
                        tab_idx += 1;
                    } else {
                        self.print_rem_generic(&rem, &self.map_a, &self.map_b, |v, h| (v, h))
                            .context("Error printing")?;
                        md_tables.push((c.md_title(), tab_idx, true));
                        tab_idx += 1;
                    }
                }
                past_constraints.push(&c_);
                println!();
                constr.push(c);
            }
        }

        let md_path = self.dir.join(self.stem.clone()).with_extension("md");
        self.md_output(&mut File::create(md_path.clone())?, &md_tables)?;

        if self.tree_gen {
            let dot_path = self.dir.join(self.stem.clone()).with_extension("dot");
            let ordering = self.tree_ordering(&is.left_poss);
            self.dot_tree(
                &is.left_poss,
                &ordering,
                &(constr[constr.len() - 1].type_str() + " / " + constr[constr.len() - 1].comment()),
                &mut File::create(dot_path.clone())?,
            )?;
        }

        if let Some(d) = dump_mode {
            match d {
                DumpMode::Full => {
                    for p in is.left_poss.iter() {
                        println!("{:?}", p.iter().enumerate().collect::<Vec<_>>())
                    }
                },
                DumpMode::FullNames => {
                    for p in is.left_poss.iter() {
                        println!("{:?}", p.into_iter().enumerate().map(|(a,bs)| (&self.map_a[a], bs.into_iter().map(|b| &self.map_b[*b as usize]).collect::<Vec<_>>())).collect::<Vec<_>>())
                    }
                },
                DumpMode::Winning => {
                    for p in is.left_poss.iter() {
                        foreach_unwrapped_matching(p, |m| println!("{:?}", m));
                    }
                },
                DumpMode::WinningNames => {
                    for p in is.left_poss.iter() {
                        foreach_unwrapped_matching(p, |m| println!("{:?}", m.into_iter().enumerate().map(|(a,b)| (&self.map_a[a], &self.map_b[*b as usize])).collect::<Vec<_>>()));
                    }
                },
            }
        }

        self.do_statistics(is.total as f64, &constr)?;

        println!(
            "Total permutations: {}  Permutations left: {}  Initial combinations for each pair: {}",
            is.total,
            is.total - is.eliminated,
            is.each
        );
        Ok(())
    }

    fn md_output(&self, out: &mut File, md_tables: &Vec<(String,u16, bool)>) -> Result<()> {
        writeln!(out, "---")?;
        writeln!(out, "{}", serde_yaml::to_string(&self.frontmatter)?)?;
        writeln!(out, "---")?;

        let stem = &self.stem;

        writeln!(out, "\n{{{{% translateHdr \"tab-current\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
        writeln!(out, "{{{{% details \"\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/sim-ayto/{stem}/{stem}_tab.png\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/sim-ayto/{stem}/{stem}_sum.png\" %}}}}")?;
        writeln!(out, "{{{{% /details %}}}}")?;

        writeln!(out, "\n{{{{% translateHdr \"tab-individual\" %}}}}")?;
        for (name,idx,detail) in md_tables.iter() {
            if *detail {
                writeln!(out, "\n{{{{% details \"{name}\" %}}}}")?;
            } else {
                writeln!(out, "\n{{{{% translatedDetails \"{name}\" %}}}}")?;
            }
            writeln!(out, "{{{{% img src=\"/sim-ayto/{stem}/{stem}_{idx}.png\" %}}}}")?;
            if *detail {
                writeln!(out, "{{{{% /details %}}}}")?;
            } else {
                writeln!(out, "{{{{% /translatedDetails %}}}}")?;
            }
        }

        writeln!(out, "\n{{{{% translateHdr \"tab-everything\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
        writeln!(out, "{{{{% details \"\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/sim-ayto/{stem}/{stem}.col.png\" %}}}}")?;
        writeln!(out, "{{{{% /details %}}}}")?;

        if self.tree_gen {
            writeln!(out, "\n{{{{% translateHdr \"tree-current\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
            writeln!(out, "{{{{% details \"\" %}}}}")?;
            writeln!(out, "{{{{% img src=\"/sim-ayto/{stem}/{stem}.png\" %}}}}")?;
            writeln!(out, "{{{{% /details %}}}}")?;
        }

        Ok(())
    }


    fn do_statistics(&self, total: f64, merged_constraints: &Vec<Constraint>) -> Result<()> {
        let out_mb_path = self.dir.join("statMB").with_extension("csv");
        let out_mn_path = self.dir.join("statMN").with_extension("csv");
        let out_info_path = self.dir.join("statInfo").with_extension("csv");

        let (mut mbo, mut mno, mut info) = (
            csv::WriterBuilder::new()
                .delimiter(b';')
                .has_headers(false)
                .from_path(out_mb_path)?,
            csv::WriterBuilder::new()
                .delimiter(b';')
                .has_headers(false)
                .from_path(out_mn_path)?,
            csv::WriterBuilder::new()
                .delimiter(b';')
                .has_headers(false)
                .from_path(out_info_path)?,
        );
        info.serialize((0, total.log2(), "initial"))?;
        for i in merged_constraints.iter().map(|c| {
            c.get_stats(
                self.rule_set
                    .constr_map_len(self.lut_a.len(), self.lut_b.len()),
            )
        }) {
            let i = i?;
            if let Some(j) = &i.0 {
                mbo.serialize(j)?
            }
            if let Some(j) = &i.1 {
                mno.serialize(j)?
            }
            if let Some(j) = &i.2 {
                info.serialize(j)?
            }
        }
        mbo.flush()?;
        mno.flush()?;
        info.flush()?;

        self.summary_table(false, merged_constraints)?;
        self.summary_table(true, merged_constraints)?;
        Ok(())
    }

    fn summary_table(&self, transpose: bool, merged_constraints: &Vec<Constraint>) -> Result<()> {
        let map_hor;
        // let map_vert;
        if !transpose {
            map_hor = &self.map_a;
            // map_vert = &self.map_b;
        } else {
            map_hor = &self.map_b;
            // map_vert = &self.map_a;
        }

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
        map_vert: &Vec<String>,
        map_hor: &Vec<String>,
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
                        return acc;
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
                        return acc;
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
                                    return Err(anyhow!(
                                        "unexpected value encountered in table {:6.3}",
                                        val
                                    ));
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
                    .into_iter()
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

    fn dot_tree(
        &self,
        data: &Vec<Matching>,
        ordering: &Vec<(usize, usize)>,
        title: &str,
        writer: &mut File,
    ) -> Result<()> {
        let mut nodes: HashSet<String> = HashSet::new();
        writeln!(
            writer,
            "digraph D {{ labelloc=\"b\"; label=\"Stand: {}\"; ranksep=0.8;",
            title
        )?;
        for p in data {
            let mut parent = String::from("root");
            for (i, _) in ordering {
                let mut node = parent.clone();
                node.push('/');
                node.push_str(
                    &p[*i]
                        .iter()
                        .map(|b| b.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );

                if nodes.insert(node.clone()) {
                    // if node is new
                    if p[*i].iter().filter(|&b| *b != u8::MAX).count() == 0 {
                        writeln!(writer, "\"{node}\"[label=\"\"]")?;
                    } else {
                        // only put content in that node if there is something meaning-full
                        // don't just skip the whole node since this would mess up the layering
                        writeln!(
                            writer,
                            "\"{node}\"[label=\"{}\"]",
                            self.map_a[*i].clone()
                                + "\\n"
                                + &p[*i]
                                    .iter()
                                    .filter(|&b| *b != u8::MAX)
                                    .map(|b| self.map_b[*b as usize].clone())
                                    .collect::<Vec<_>>()
                                    .join("\\n")
                        )?;
                    }
                    writeln!(writer, "\"{parent}\" -> \"{node}\";")?;
                }

                parent = node;
            }
        }
        writeln!(writer, "}}")?;
        Ok(())
    }

    fn tree_ordering(&self, data: &Vec<Matching>) -> Vec<(usize, usize)> {
        // tab maps people from set_a -> possible matches (set -> no duplicates)
        let mut tab = vec![HashSet::new(); self.map_a.len()];
        for p in data {
            for (i, js) in p.iter().enumerate() {
                // if js[0] != u8::MAX {
                tab[i].insert(js);
                // }
            }
        }

        // pairs people of set_a with amount of different matches
        let mut ordering: Vec<_> = tab
            .iter()
            .enumerate()
            .filter_map(|(i, x)| {
                if x.len() == 0 || x.iter().all(|y| y.len() == 1 && y[0] == u8::MAX) {
                    None
                } else {
                    Some((i, x.len()))
                }
            })
            .collect();

        match &self.tree_top {
            Some(ts) => {
                let t = self.lut_a[ts];
                ordering.sort_unstable_by_key(|(i, x)| {
                    // x values will always be positive, 1 will be the minimum / value for already
                    // fixed matches
                    // with (x-1)*2 we move that minimum to 0 and spread the values.
                    // In effect the value 1 will be unused. To sort the specified tree_top right
                    // below the already fixed matches this level is mapped to the value 1
                    // Why so complicated? To avoid using floats here, while still ensuring the
                    // order as specified.
                    if *i == t {
                        1
                    } else {
                        ((*x) - 1) * 2
                    }
                })
            }
            None => {
                ordering.sort_unstable_by_key(|(_, x)| *x);
            }
        }
        ordering
    }
}

pub struct IterState {
    constraints: Vec<Constraint>,
    keep_rem: bool,
    each: u128,
    total: u128,
    eliminated: u128,
    pub left_poss: Vec<Matching>,
    query_matchings: Vec<(Matching, Option<String>)>,
    cnt_update: usize,
    progress: ProgressBar,
}

impl IterState {
    pub fn new(
        keep_rem: bool,
        perm_amount: usize,
        constraints: Vec<Constraint>,
        query_matchings: &Vec<Matching>,
    ) -> IterState {
        let is = IterState {
            constraints,
            keep_rem,
            query_matchings: query_matchings.iter().map(|i| (i.clone(), None)).collect(),
            each: 0,
            total: 0,
            eliminated: 0,
            left_poss: vec![],
            progress: ProgressBar::new(100),
            cnt_update: std::cmp::max(perm_amount / 50, 1),
        };
        is.progress.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar}] {pos:>3}/{len:3} (ETA: {eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );
        is
    }

    pub fn start(&mut self) {
        self.progress.inc(0)
    }

    pub fn finish(&mut self) {
        self.progress.finish()
    }

    pub fn step(&mut self, i: usize, p: Matching, output: bool) -> Result<()> {
        // eprintln!("{:} {:?}", i, p);
        if i % self.cnt_update == 0 && output {
            self.progress.inc(2);
        }
        if p[1].contains(&0) {
            self.each += 1;
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
        if left && self.keep_rem {
            self.left_poss.push(p);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwraped_matching() {
        let m = vec![vec![6], vec![3], vec![2, 10], vec![4], vec![1], vec![5], vec![0], vec![7], vec![8], vec![9]];
        foreach_unwrapped_matching(&m, |m| println!("{:?}", m));
        println!();
        let m = vec![vec![6], vec![3], vec![2, 10], vec![4], vec![1], vec![5, 20], vec![0], vec![7], vec![8], vec![9]];
        foreach_unwrapped_matching(&m, |m| println!("{:?}", m));
    }
}

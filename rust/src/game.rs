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
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};

use indicatif::{ProgressBar, ProgressStyle};

use serde::Deserialize;

use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, ensure, Context, Result};

use crate::constraint::Constraint;
use crate::ruleset::RuleSet;
use crate::{Lut, Matching, Rem};

#[derive(Deserialize, Debug)]
pub struct Game {
    #[serde(rename = "constraints")]
    constraints_orig: Vec<Constraint>,
    rule_set: RuleSet,
    tree_gen: bool,
    tree_top: Option<String>,

    #[serde(rename = "setA")]
    map_a: Vec<String>,
    #[serde(rename = "setB")]
    map_b: Vec<String>,

    #[serde(skip)]
    dir: PathBuf,
    #[serde(skip)]
    stem: String,
    #[serde(skip)]
    lut_a: Lut,
    #[serde(skip)]
    lut_b: Lut,
}

impl Game {
    pub fn new_from_yaml(yaml_path: &Path, stem: &Path) -> Result<Game> {
        let mut g: Game = serde_yaml::from_reader(File::open(yaml_path)?)?;

        g.dir = stem
            .parent()
            .context("parent dir of stem not found")?
            .to_path_buf();
        g.stem = stem
            .file_stem()
            .context("No filename provided in stem")?
            .to_string_lossy()
            .into_owned();

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

        // postprocessing -> add exclude mapping list (with names)
        if g.rule_set.must_add_exclude() {
            for c in &mut g.constraints_orig {
                c.add_exclude(&g.map_b);
            }
        }

        // eg translates strings to indices (u8)
        for c in &mut g.constraints_orig {
            c.finalize_parsing(
                &g.lut_a,
                &g.lut_b,
                g.rule_set.constr_map_len(g.lut_a.len(), g.lut_b.len()),
            )?;
        }

        // postprocessing -> sort map if ruleset demands it
        if g.rule_set.must_sort_constraint() {
            for c in &mut g.constraints_orig {
                c.sort_maps(&g.lut_a, &g.lut_b);
            }
        }

        Ok(g)
    }

    pub fn sim(&mut self, print_transposed: bool) -> Result<()> {
        let perm_amount = self
            .rule_set
            .get_perms_amount(self.map_a.len(), self.map_b.len());

        let mut is = IterState::new(self.tree_gen, perm_amount, self.constraints_orig.clone());
        self.rule_set
            .iter_perms(&self.lut_a, &self.lut_b, &mut is, true)?;

        // fix is so that it can't be mutated anymore
        let is = &is;

        let mut rem: Rem = (
            vec![vec![is.each; self.map_b.len()]; self.map_a.len()],
            is.total,
        );
        if print_transposed {
            self.print_rem_generic(&rem, &self.map_b, &self.map_a, |v,h| (h,v)).context("Error printing")?;
        } else {
            self.print_rem_generic(&rem, &self.map_a, &self.map_b, |v,h| (v,h)).context("Error printing")?;
        }
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
                        self.print_rem_generic(&rem, &self.map_b, &self.map_a, |v,h| (h,v)).context("Error printing")?;
                    } else {
                        self.print_rem_generic(&rem, &self.map_a, &self.map_b, |v,h| (v,h)).context("Error printing")?;
                    }
                }
                past_constraints.push(&c_);
                println!();
                constr.push(c);
            }
        }

        if self.tree_gen {
            let dot_path = self.dir.join(self.stem.clone()).with_extension("dot");
            let ordering = self.tree_ordering(&is.left_poss);
            self.dot_tree(
                &is.left_poss,
                &ordering,
                &(constr[constr.len() - 1].type_str() + " / " + constr[constr.len() - 1].comment()),
                &mut File::create(dot_path.clone())?,
            )?;

            let pdf_path = dot_path.with_extension("pdf");
            Command::new("dot")
                .args([
                    "-Tpdf",
                    "-o",
                    pdf_path.to_str().context("pdf_path failed")?,
                    dot_path.to_str().context("dot_path failed")?,
                ])
                .output()
                .expect("dot command failed");

            let png_path = dot_path.with_extension("png");
            Command::new("dot")
                .args([
                    "-Tpng",
                    "-o",
                    png_path.to_str().context("png_path failed")?,
                    dot_path.to_str().context("dot_path failed")?,
                ])
                .output()
                .expect("dot command failed");
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
        for i in merged_constraints.iter().map(|c| c.get_stats()) {
            let i = i?;
            if let Some(j) = &i[0] {
                mbo.serialize(j)?
            }
            if let Some(j) = &i[1] {
                mno.serialize(j)?
            }
            if let Some(j) = &i[2] {
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
        for (i,c) in merged_constraints.iter().enumerate() {
            if i % 2 == 0 {
                table.add_row(c.stat_row(transpose, map_hor, &past_constraints).into_iter().map(|i| i.bg(comfy_table::Color::Rgb {
                        r: 41,
                        g: 44,
                        b: 60,
                    })));
            } else {
                table.add_row(c.stat_row(transpose, map_hor, &past_constraints));
            }
            past_constraints.push(c);
        }
        println!("{table}");
        Ok(())
    }

    fn print_rem_generic(&self, rem: &Rem, map_vert: &Vec<String>, map_hor: &Vec<String>, norm_idx: fn(v:usize,h:usize) -> (usize,usize)) -> Result<()> {
        let mut hdr = vec![Cell::new("")];
        hdr.extend(
            map_hor
                .iter()
                .map(|x| Cell::new(x).set_alignment(comfy_table::CellAlignment::Center)),
        );

        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);

        for (v, a) in map_vert.iter().enumerate() {
            let i = map_hor.iter().enumerate().map(|(h, _)| {
                let (i,j) = norm_idx(v,h);
                if self.rule_set.ignore_pairing(i, j) {
                    Ok(Cell::new(""))
                } else {
                    let x = rem.0[i][j];
                    let val = (x as f64) / (rem.1 as f64) * 100.0;
                    if 79.0 < val && val < 101.0 {
                        Ok(Cell::new(format!("{:6.3}", val).trim_end_matches('0').trim_end_matches('.')).fg(Color::Green))
                    } else if 55.0 <= val {
                        Ok(Cell::new(format!("{:6.3}", val).trim_end_matches('0').trim_end_matches('.')).fg(Color::Cyan))
                    } else if 45.0 < val {
                        Ok(Cell::new(format!("{:6.3}", val).trim_end_matches('0').trim_end_matches('.')).fg(Color::Yellow))
                    } else if 1.0 < val {
                        Ok(Cell::new(format!("{:6.3}", val).trim_end_matches('0').trim_end_matches('.')))
                    } else if -1.0 < val {
                        Ok(Cell::new(format!("{:6.3}", val).trim_end_matches('0').trim_end_matches('.')).fg(Color::Red))
                    } else {
                        return Err(anyhow!("unexpected value encountered in table {:6.3}", val))
                    }
                }
            });
            let mut row = vec![Cell::new(a)];
            row.extend(i.into_iter().collect::<Result<Vec<_>>>()?);
            if v % 2 == 0 {
                table.add_row(row.into_iter().map(|i| {
                    i.bg(comfy_table::Color::Rgb {
                        r: 41,
                        g: 44,
                        b: 60,
                    })
                }));
            } else {
                table.add_row(row);
            }
        }
        println!("{table}");
        println!("{} left -> {} bits left", rem.1, format!("{:.4}", (rem.1 as f64).log2()).trim_end_matches('0').trim_end_matches('.'));
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
    tree_gen: bool,
    each: u128,
    total: u128,
    eliminated: u128,
    pub left_poss: Vec<Matching>,
    cnt_update: usize,
    progress: ProgressBar,
}

impl IterState {
    pub fn new(tree_gen: bool, perm_amount: usize, constraints: Vec<Constraint>) -> IterState {
        let is = IterState {
            constraints,
            tree_gen,
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
                break;
            }
        }
        if left && self.tree_gen {
            self.left_poss.push(p);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}

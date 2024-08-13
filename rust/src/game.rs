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

use anyhow::{ensure, Context, Result};

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
            self.print_rem_transposed(&rem).context("Error printing")?;
        } else {
            self.print_rem(&rem).context("Error printing")?;
        }
        println!();

        let mut constr = vec![];
        let mut to_merge = vec![]; // collect hidden constraints to merge them down
        for c in &is.constraints {
            if c.hidden {
                to_merge.push(c);
            } else {
                let mut c = c.clone();
                // merge down previous hidden constraints
                while !to_merge.is_empty() {
                    c.merge(to_merge.pop().unwrap())?;
                }
                rem = c.apply_to_rem(rem).context("Apply to rem failed")?;
                c.print_hdr();
                if print_transposed {
                    self.print_rem_transposed(&rem).context("Error printing")?;
                } else {
                    self.print_rem(&rem).context("Error printing")?;
                }
                constr.push(c);
                println!();
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

        self.do_statistics(&constr)?;

        println!(
            "Total permutations: {}  Permutations left: {}  Initial combinations for each pair: {}",
            is.total,
            is.total - is.eliminated,
            is.each
        );
        Ok(())
    }

    fn do_statistics(&self, merged_constraints: &Vec<Constraint>) -> Result<()> {
        let out_mb_path = self
            .dir
            .join(self.stem.clone() + "_statMB")
            .with_extension("out");
        let out_mn_path = self
            .dir
            .join(self.stem.clone() + "_statMN")
            .with_extension("out");
        let out_info_path = self
            .dir
            .join(self.stem.clone() + "_statInfo")
            .with_extension("out");

        let (mut mbo, mut mno, mut info) = (
            File::create(out_mb_path)?,
            File::create(out_mn_path)?,
            File::create(out_info_path)?,
        );
        for c in merged_constraints {
            c.write_stats(&mut mbo, &mut mno, &mut info)?;
        }

        let mut hdr = vec![
            Cell::new(""),
            Cell::new("L").set_alignment(comfy_table::CellAlignment::Center),
        ];
        hdr.extend(
            self.map_a
                .iter()
                .map(|x| Cell::new(x).set_alignment(comfy_table::CellAlignment::Center)),
        );
        hdr.push(Cell::new("").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("I").set_alignment(comfy_table::CellAlignment::Center));

        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);

        for c in merged_constraints {
            table.add_row(c.stat_row(&self.map_a));
        }
        println!("{table}");

        Ok(())
    }

    fn print_rem_transposed(&self, rem: &Rem) -> Option<()> {
        let mut hdr = vec![Cell::new("")];
        hdr.extend(
            self.map_a
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

        for (j, a) in self.map_b.iter().enumerate() {
            let i = self.map_a.iter().enumerate().map(|(i, _)| {
                if self.rule_set.ignore_pairing(i, j) {
                    Cell::new("")
                } else {
                    let x = rem.0[i][j];
                    let val = (x as f64) / (rem.1 as f64) * 100.0;
                    if 79.0 < val && val < 101.0 {
                        Cell::new(format!("{:02.3}", val)).fg(Color::Green)
                    } else if -1.0 < val && val < 1.0 {
                        Cell::new(format!("{:02.3}", val)).fg(Color::Red)
                    } else {
                        Cell::new(format!("{:02.3}", val))
                    }
                }
            });
            let mut row = vec![Cell::new(a)];
            row.extend(i);
            table.add_row(row);
        }
        println!("{table}");
        println!("{} left -> {:.4} bits left", rem.1, (rem.1 as f64).log2());
        Some(())
    }

    fn print_rem(&self, rem: &Rem) -> Option<()> {
        let mut hdr = vec![Cell::new("")];
        hdr.extend(
            self.map_b
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

        for (i, a) in self.map_a.iter().enumerate() {
            let i = self.map_b.iter().enumerate().map(|(j, _)| {
                if self.rule_set.ignore_pairing(i, j) {
                    Cell::new("")
                } else {
                    let x = rem.0[i][j];
                    let val = (x as f64) / (rem.1 as f64) * 100.0;
                    if 79.0 < val && val < 101.0 {
                        Cell::new(format!("{:02.3}", val)).fg(Color::Green)
                    } else if -1.0 < val && val < 1.0 {
                        Cell::new(format!("{:02.3}", val)).fg(Color::Red)
                    } else {
                        Cell::new(format!("{:02.3}", val))
                    }
                }
            });
            let mut row = vec![Cell::new(a)];
            row.extend(i);
            table.add_row(row);
        }
        println!("{table}");
        println!("{} left -> {:.4} bits left", rem.1, (rem.1 as f64).log2());
        Some(())
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

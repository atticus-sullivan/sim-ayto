/// This module is concerned with building reports. This is coupled with evaluation and comparison.
/// But in contrast to these two concerns, this mostly covers presenting and visualizing the data.
/// So not much raw processing takes place here.
///
/// Note:
/// Some larger functionality has been factored out to individual modules:
/// - generating a row for the summary table
/// 
/// Also some predicates used for generating the reports has been moved to a new module:
/// - report_predicates

use anyhow::{Context, Result};
use std::{fs::File, path::PathBuf};

use comfy_table::{presets::NOTHING, Row, Table};

use crate::constraint::{CheckType, Constraint, ConstraintType};
use crate::matching_repr::bitset::Bitset;
use crate::tree::{dot_tree, tree_ordering};

impl Constraint {
    /// Write a `.dot` file for the tree of remaining possibilities in case this has been
    /// requested. If no tree is requested for this constraint, this function is a no-op.
    ///
    /// Returns whether a tree has been generated/drawn
    pub fn build_tree(&self, path: PathBuf, map_a: &[String], map_b: &[String]) -> Result<bool> {
        if !self.build_tree {
            return Ok(false);
        }

        // calculate the order in which the layers shall be shown
        let ordering = tree_ordering(&self.left_poss, map_a);
        // delegate drawing the tree to a dedicated module
        dot_tree(
            &self.left_poss,
            &ordering,
            &(self.type_str() + " / " + self.comment()),
            &mut File::create(path)?,
            map_a,
            map_b,
        )?;
        Ok(true)
    }

    pub fn distance(&self, other: &Constraint) -> Option<usize> {
        if !self.show_past_dist() || !other.show_past_dist() {
            return None;
        }
        if self.map.len() != other.map.len() {
            return None;
        }

        Some(
            self.map
                .iter()
                .enumerate()
                .filter(|&(k, v)| {
                    !v.is_empty()
                        && !other
                            .map
                            .slot_mask(k)
                            .unwrap_or(&Bitset::empty())
                            .contains_any(&v)
                })
                .count(),
        )
    }

    // TODO: split up / output
    pub fn print_hdr(&self, past_constraints: &Vec<&Constraint>) -> Result<()> {
        match &self.r#type {
            ConstraintType::Night { num, comment, .. } => print!("MN#{:02.1} {}", num, comment),
            ConstraintType::Box { num, comment, .. } => print!("MB#{:02.1} {}", num, comment),
        }
        println!();

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
        let mut rows = vec![("", Row::new()); self.map_s.len()];
        for (i, (k, v)) in self.map_s.iter().enumerate() {
            if self.show_past_cnt() {
                let cnt = past_constraints
                    .iter()
                    .filter(|&c| c.show_past_cnt() && c.map_s.get(k).is_some_and(|v2| v2 == v))
                    .count();
                rows[i].0 = k;
                rows[i].1.add_cell(format!("{}x {}", cnt, k).into());
                rows[i].1.add_cell(v.into());
                // println!("{}x {} -> {}", cnt, k, v);
            } else {
                rows[i].0 = k;
                rows[i].1.add_cell(k.into());
                rows[i].1.add_cell(v.into());
                // println!("{} -> {}", k, v);
            }
        }
        rows.sort_by_key(|i| i.0);
        tab.add_rows(rows.into_iter().map(|i| i.1).collect::<Vec<_>>());
        tab.column_mut(0)
            .context("no 0th column in table found")?
            .set_padding((0, 1));
        println!("{tab}");

        println!("---");
        match &self.check {
            CheckType::Eq => print!("Eq "),
            CheckType::Nothing | CheckType::Sold => print!("Nothing "),
            CheckType::Lights(l, ls) => {
                let total = ls.values().sum::<u128>() as f64;
                // information theory
                if self.show_lights_information() {
                    println!(
                        "-> I[l]/bits: {{{}}}",
                        ls.iter()
                            .map(|(l, c)| {
                                let mut i = -(*c as f64 / total).log2();
                                if i == -0.0 {
                                    i = 0.0;
                                }
                                format!("{}: {:.2}", l, i)
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if self.show_expected_information() {
                    let mut expected: f64 = ls
                        .values()
                        .map(|c| {
                            let p = *c as f64 / total;
                            p * p.log2()
                        })
                        .sum();
                    if expected == 0.0 {
                        expected = -0.0;
                    }
                    println!("-> E[I]/bits: {:.2} = H", -expected);
                }

                print!("{} lights ", l);
            }
        }

        println!(
            "=> I = {} bits",
            format!("{:.4}", self.information.unwrap_or(f64::INFINITY))
                .trim_end_matches('0')
                .trim_end_matches('.')
        );
        Ok(())
    }

    pub fn show_rem_table(&self) -> bool {
        !self.result_unknown
    }

    pub fn md_title(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, comment, .. } => format!(
                "MN#{:02.1} {}",
                num,
                comment.split("--").collect::<Vec<_>>()[0]
            ),
            ConstraintType::Box { num, comment, .. } => format!(
                "MB#{:02.1} {}",
                num,
                comment.split("--").collect::<Vec<_>>()[0]
            ),
        }
    }
}

#[cfg(test)]
mod test {
}

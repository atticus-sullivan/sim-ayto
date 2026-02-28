// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module offers the functionality to print a trail of the evaluated constraints.
//! This includes for the most part a table for each constraint with the remaining probabilities
//! along with some additional information before and after this table.

use anyhow::{Context, Result};

use crate::constraint::{report_hdr::ReportData, Constraint};
use crate::game::report_utils::print_rem_generic;
use crate::game::Game;
use crate::Rem;

pub(super) struct ReportEvent<'a> {
    rem: Rem,
    constr_report: ReportData<'a>,
    constraint: &'a Constraint,
}

pub(super) type Trail<'a> = (Rem, Vec<ReportEvent<'a>>);

pub(super) fn gen_report_data<'a>(
    constraints: &'a mut [Constraint],
    mut rem: Rem,
) -> Result<Trail<'a>> {
    let initial = rem.clone();

    let mut report_data = (vec![], vec![]);
    for c in constraints.iter_mut() {
        rem = c.apply_to_rem(rem).context("Apply to rem failed")?;
        report_data.0.push(rem.clone());
    }
    for (i, c) in constraints.iter().enumerate() {
        report_data.1.push((
            c,
            // .. is a half-opened range => upper bound is not included
            c.generate_hdr_report(&constraints[0..i]),
        ));
    }

    Ok((
        initial,
        report_data
            .0
            .into_iter()
            .zip(report_data.1)
            .map(|(r, (c, cd))| ReportEvent {
                rem: r,
                constr_report: cd,
                constraint: c,
            })
            .collect::<Vec<_>>(),
    ))
}

pub(super) struct MdTable {
    pub(super) name: String,
    pub(super) idx: usize,
    pub(super) tree: bool,
    pub(super) detail: bool,
}

impl Game {
    pub(super) fn gen_report(
        &self,
        data: &Trail,
        print_transposed: bool,
        full: bool,
        no_tree_output: bool,
        mut tab_idx: usize,
        md_tables: &mut Vec<MdTable>,
    ) -> Result<usize> {
        let (mv, mh) = if print_transposed {
            (&self.map_b, &self.map_a)
        } else {
            (&self.map_a, &self.map_b)
        };
        let norm_idx = if print_transposed {
            |v, h| (h, v)
        } else {
            |v, h| (v, h)
        };
        let ignore_pairing = |v, h| self.rule_set.ignore_pairing(v, h);

        println!(
            "{}",
            print_rem_generic(&data.0, mv, mh, norm_idx, ignore_pairing)
        );

        md_tables.push(MdTable {
            name: "tab-start".to_owned(),
            idx: tab_idx,
            tree: false,
            detail: false,
        });
        tab_idx += 1;
        println!();

        for event in &data.1 {
            println!("{}", event.constr_report);
            if !event.constraint.show_rem_table() {
                println!();
                continue;
            }
            let tree = if !no_tree_output {
                event.constraint.build_tree(
                    self.dir
                        .join(format!("{}_{}_tree", self.stem, tab_idx))
                        .with_extension("dot"),
                    &self.map_a,
                    &self.map_b,
                )?
            } else {
                false
            };

            println!(
                "{}",
                print_rem_generic(&event.rem, mv, mh, norm_idx, ignore_pairing)
            );
            event.constraint.ruleset_data.print(
                full,
                &self.rule_set,
                &self.map_a,
                &self.map_b,
                &self.lut_b,
                event.rem.1,
            )?;

            md_tables.push(MdTable {
                name: event.constraint.md_heading(),
                idx: tab_idx,
                tree,
                detail: true,
            });
            tab_idx += 1;
            println!();
        }

        Ok(tab_idx)
    }
}

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

/// event prepared for reporting
pub(super) struct ReportEvent<'a> {
    /// the amount of 1:1 matches left after this event
    rem: Rem,
    /// the report prepared from the constraint
    constr_report: ReportData<'a>,
    /// the constraint on which this reports on
    constraint: &'a Constraint,
}

/// descibres the trail which is reported later on
/// it consists of
/// 0. the remaining amounts for the 1:1 matches
/// 1. a sequence of events which are prepared for reporting
pub(super) type Trail<'a> = (Rem, Vec<ReportEvent<'a>>);

/// generate the data which then can be reported later
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

/// describes a table in the output which is converted to an image which then is to be referenced
/// in the .md page.
pub(super) struct MdTable {
    /// the name of this event which is even shown before expanding the details block
    pub(super) name: String,
    /// the index where to find the table
    pub(super) idx: usize,
    /// whether a tree is attached to this table
    pub(super) trees: Vec<String>,
    /// whether the table shall be wrapped in a detail block (makes the table collapsible)
    pub(super) detail: bool,
}

impl Game {
    /// generate a report for this game based on the `data`
    ///
    /// - `print_transposed` whether to transpose the tables in the report
    /// - `full` whether to print the full ruleset_data
    /// - `no_tree_output` allows to avoid generating .dot files
    /// - `tab_idx` the index to start with when refering to the generated tables in the output
    /// - `md_tables` output argument, where the tables which shall be refered to in the .md output
    ///   are collected
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
            trees: vec![],
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
            let trees = if !no_tree_output {
                event.constraint.build_tree(
                    |id| self.dir
                        .join(format!("{}_{}_tree_{}", self.stem, tab_idx, id))
                        .with_extension("dot"),
                    &self.map_a,
                    &self.map_b,
                )?
            } else {
                vec![]
            };

            println!(
                "{}",
                print_rem_generic(&event.rem, mv, mh, norm_idx, ignore_pairing)
            );
            if let Some(rs_dat) = event.constraint.ruleset_data.as_ref() {
                rs_dat.print(
                    full,
                    &self.rule_set,
                    &self.map_a,
                    &self.map_b,
                    &self.lut_b,
                    event.rem.1,
                )?;
            }

            md_tables.push(MdTable {
                name: event.constraint.md_heading(),
                idx: tab_idx,
                trees,
                detail: true,
            });
            tab_idx += 1;
            println!();
        }

        Ok(tab_idx)
    }
}

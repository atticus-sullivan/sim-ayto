use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{NOTHING, UTF8_FULL_CONDENSED};
use comfy_table::{Cell, Row, Table};

use rust_decimal::prelude::*;

use anyhow::{Context, Result};

use std::fs::File;
use std::io::{BufWriter, Write};

use crate::constraint::eval::{CSVEntry, CSVEntryMB, CSVEntryMN, SumCounts};
use crate::constraint::Constraint;
use crate::game::DumpMode;
use crate::game::Game;
use crate::iterstate::IterState;
use crate::matching_repr::MaskedMatching;
use crate::Rem;

impl Game {
    // TODO:
    pub fn eval(
        &mut self,
        print_transposed: bool,
        dump_mode: Option<DumpMode>,
        full: bool,
        is: &IterState,
        no_tree_output: bool,
    ) -> Result<()> {
        // track table indices
        let mut tab_idx = 0;
        let mut md_tables: Vec<(String, u16, bool, bool)> = vec![];

        // generate additional tables
        if is.query_matchings.iter().any(|(_, x)| x.is_some()) {
            println!("Trace at which point a particular matching was elimiated:");
            for (q, id) in &is.query_matchings {
                if let Some(id) = id {
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
                            .map(|b| self.map_b.get(b as usize).unwrap())
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
            }
            println!();
        }
        if !is.query_pair.0.is_empty() {
            for (a, i) in is.query_pair.0.iter() {
                let mut tab = Table::new();
                tab.force_no_tty()
                    .enforce_styling()
                    .load_preset(UTF8_FULL_CONDENSED)
                    .set_header(vec!["", self.map_a.get(*a as usize).unwrap()]);
                for b in i.iter() {
                    tab.add_row(vec![
                        format!("{}", b.1),
                        format!(
                            "{:?}",
                            b.0.iter()
                                .map(|b| self.map_b.get(b as usize).unwrap())
                                .collect::<Vec<_>>()
                        ),
                    ]);
                }
                println!("{tab}")
            }
        }
        if !is.query_pair.1.is_empty() {
            for (b, i) in is.query_pair.1.iter() {
                let mut tab = Table::new();
                tab.force_no_tty()
                    .enforce_styling()
                    .load_preset(UTF8_FULL_CONDENSED)
                    .set_header(vec!["", self.map_b.get(*b as usize).unwrap()]);
                println!("{}", self.map_b.get(*b as usize).unwrap());
                for a in i.iter() {
                    tab.add_row(vec![
                        format!("{}", a.1),
                        format!("{:?}", self.map_a.get(*a.0 as usize).unwrap()),
                    ]);
                }
                println!("{tab}")
            }
        }

        let mut rem: Rem = (is.each.clone(), is.total);
        if print_transposed {
            self.print_rem_generic(&rem, &self.map_b, &self.map_a, |v, h| (h, v))
                .context("Error printing")?;
        } else {
            self.print_rem_generic(&rem, &self.map_a, &self.map_b, |v, h| (v, h))
                .context("Error printing")?;
        }
        md_tables.push(("tab-start".to_owned(), tab_idx, false, false));
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
                    let tree = if !no_tree_output {
                        c.build_tree(
                            self.dir
                                .join(format!("{}_{}_tree", self.stem, tab_idx))
                                .with_extension("dot"),
                            &self.map_a,
                            &self.map_b,
                        )?
                    } else {
                        false
                    };
                    if print_transposed {
                        self.print_rem_generic(&rem, &self.map_b, &self.map_a, |v, h| (h, v))
                            .context("Error printing")?;
                        c.ruleset_data.print(
                            full,
                            &self.rule_set,
                            &self.map_a,
                            &self.map_b,
                            &self.lut_b,
                            rem.1,
                        )?;
                        md_tables.push((c.md_title(), tab_idx, tree, true));
                        tab_idx += 1;
                    } else {
                        self.print_rem_generic(&rem, &self.map_a, &self.map_b, |v, h| (v, h))
                            .context("Error printing")?;
                        c.ruleset_data.print(
                            full,
                            &self.rule_set,
                            &self.map_a,
                            &self.map_b,
                            &self.lut_b,
                            rem.1,
                        )?;
                        md_tables.push((c.md_title(), tab_idx, tree, true));
                        tab_idx += 1;
                    }
                }

                past_constraints.push(c_);
                println!();
                constr.push(c);
            }
        }

        let md_path = self.dir.join(self.stem.clone()).with_extension("md");
        self.write_page_md(&mut File::create(md_path.clone())?, &md_tables)?;

        if let Some(d) = dump_mode {
            match d {
                DumpMode::Full => {
                    for p in is.left_poss.iter() {
                        println!("{:?}", p.prepare_debug_print())
                    }
                }
                DumpMode::FullNames => {
                    for p in is.left_poss.iter() {
                        println!(
                            "{:?}",
                            p.prepare_debug_print_names(&self.map_a, &self.map_b)
                        );
                    }
                }
                DumpMode::Winning => {
                    for p in is.left_poss.iter() {
                        for pw in p.iter_unwrapped() {
                            println!("{:?}", pw.prepare_debug_print());
                        }
                    }
                }
                DumpMode::WinningNames => {
                    for p in is.left_poss.iter() {
                        for pw in p.iter_unwrapped() {
                            println!(
                                "{:?}",
                                pw.prepare_debug_print_names(&self.map_a, &self.map_b)
                            );
                        }
                    }
                }
            }
        }

        let solution = if is.keep_rem {
            Some(&is.left_poss)
        } else {
            None
        };

        self.do_statistics(is.total as f64, &constr, solution)?;

        println!(
            "Total permutations: {}  Permutations left: {}  Initial combinations for each pair: {}",
            is.total,
            is.total - is.eliminated,
            is.each[0][0]
        );
        Ok(())
    }

    // TODO:
    fn do_statistics(
        &self,
        total: f64,
        merged_constraints: &[Constraint],
        solutions: Option<&Vec<MaskedMatching>>,
    ) -> Result<()> {
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
            num: dec!(0),
            lights_total: None,
            lights_known_before: 0,
            bits_left: total.log2(),
            comment: "initial".to_string(),
        })?;
        for i in merged_constraints.iter().map(|c| c.get_stats()) {
            let i = i?;
            if let Some(j) = &i.1 {
                let j = CSVEntryMN {
                    num: j.num,
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
                    offer: j.offer,
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

        let mut cnt = SumCounts {
            blackouts: 0,
            sold: 0,
            sold_but_match: 0,
            sold_but_match_active: solutions.is_some(),
            matches_found: 0,
            won: false,
            offers_noted: !self.no_offerings_noted,
            offer_and_match: 0,
            offers: 0,
            offered_money: 0,
            solvable: None,
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
            let offer = c.try_get_offer();
            if let Some(o) = offer {
                cnt.offers += 1;
                if let Some(m) = o.try_get_amount() {
                    cnt.offered_money += m;
                }
                if c.is_mb_hit(solutions) {
                    cnt.offer_and_match += 1;
                }
            }
        }

        cnt.won = {
            let required_lights = self
                .rule_set
                .constr_map_len(self.lut_a.len(), self.lut_b.len());
            merged_constraints
                .iter()
                .find(|x| x.num() == 10.0 && x.might_won())
                .or_else(|| merged_constraints.last())
                .map(|x| x.won(required_lights))
                .unwrap_or(false)
        };

        cnt.solvable = merged_constraints
            .windows(2)
            .find(|x| x[1].num() == 10.0 && x[1].might_won())
            .map(|x| &x[0])
            .or_else(|| merged_constraints.last())
            .and_then(|x| x.was_solvable_before().ok().flatten());

        let file = File::create(out_sum_path)?;
        let mut writer = BufWriter::new(file);

        serde_json::to_writer(&mut writer, &cnt)?;
        writer.flush()?;

        println!("{}", self.summary_table(false, merged_constraints)?); // TODO: return table?
        println!("{}", self.summary_table(true, merged_constraints)?); // TODO: return table?
        Ok(())
    }

    fn summary_table(&self, transpose: bool, merged_constraints: &[Constraint]) -> Result<Table> {
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
                        .map(|i| i.bg(crate::COLOR_ALT_BG)),
                );
            } else {
                table.add_row(c.stat_row(transpose, map_hor, &past_constraints));
            }
            past_constraints.push(c);
        }
        Ok(table)
    }
}

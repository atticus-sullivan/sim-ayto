use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_FULL_CONDENSED};
use comfy_table::{Cell, Table};

use rust_decimal::prelude::*;

use anyhow::{Context, Result};

use std::fs::File;
use std::io::{BufWriter, Write};

use crate::constraint::compare::{
    EvalData, EvalEvent, EvalInitial, SumCounts, SumOffersMB, SumOffersMN,
};
use crate::constraint::Constraint;
use crate::game::{query_pairs, DumpMode};
use crate::game::Game;
use crate::game::query_matchings;
use crate::iterstate::IterState;
use crate::matching_repr::MaskedMatching;
use crate::Rem;

impl Game {
    /// Render evaluation outputs (tables and debugging artifacts) from an `IterState`.
    ///
    /// This function is primarily presentation + file output and delegates the
    /// pure counting work to `compute_cnts` (testable separately).
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
        let mut md_tables: Vec<(String, usize, bool, bool)> = vec![];

        // generate additional tables
        {
            let m_data = query_matchings::MatchingReport::new(&is.query_matchings, &self.map_a, &self.map_b)?;
            if let Some(m_data) = m_data {
                println!("{m_data}");
                tab_idx += m_data.tab_cnt();
            }
            let p_data = query_pairs::QueryPairReport::new(&is.query_pair, &self.map_a, &self.map_b)?;
            println!("{p_data}");
        }

        // TODO: first reporting_body step
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

        // TODO: extract the merging of the constraints
        // TODO: print_hdr returns something which has a render function? -> don't need to store
        // all constraints, only that data + rem needs to be stored for printing
        // TODO: well but due to past_constraints I need to store the (cloned) merged constraints
        // all at once anyhow => make the code simpler by fully separating the concerns here
        // TODO: but past_constraints only stores references...

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
                        md_tables.push((c.md_heading(), tab_idx, tree, true));
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
                        md_tables.push((c.md_heading(), tab_idx, tree, true));
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

    /// Pure helper: compute `SumCounts` from the provided constraints and optional solutions.
    ///
    /// This takes the loop part of `do_statistics` and returns an aggregated `SumCounts`.
    /// It intentionally does **not** attempt to determine `won` or `solvable` (those need more context).
    pub(crate) fn compute_cnts(
        &self,
        merged_constraints: &[Constraint],
        solutions: Option<&Vec<MaskedMatching>>,
        offers_noted: bool,
    ) -> SumCounts {
        let mut cnts = SumCounts {
            blackouts: 0,
            matches_found: 0,
            won: false,
            solvable_in: None,
            offers_mb: SumOffersMB {
                sold_cnt: 0,
                sold_but_match: 0,
                sold_but_match_active: solutions.is_some(),
                offers_noted,
                offer_and_match: 0,
                offers: 0,
                offered_money: 0,
            },
            offers_mn: SumOffersMN {
                sold_cnt: 0,
                offers_noted,
                offers: 0,
                offered_money: 0,
            },
        };

        for c in merged_constraints.iter() {
            if c.is_blackout() {
                cnts.blackouts += 1;
            }
            if c.is_match_found() {
                cnts.matches_found += 1;
            }
            if c.is_mb() {
                if c.is_sold() {
                    cnts.offers_mb.sold_cnt += 1;
                }
                if c.is_sold() && c.is_mb_hit(solutions) {
                    cnts.offers_mb.sold_but_match += 1;
                }
                if let Some(o) = c.try_get_offer() {
                    cnts.offers_mb.offers += 1;
                    if let Some(m) = o.try_get_amount() {
                        cnts.offers_mb.offered_money += m;
                    }
                    if c.is_mb_hit(solutions) {
                        cnts.offers_mb.offer_and_match += 1;
                    }
                }
            } else if c.is_mn() {
                if c.is_sold() {
                    cnts.offers_mn.sold_cnt += 1;
                }
                if let Some(o) = c.try_get_offer() {
                    cnts.offers_mn.offers += 1;
                    if let Some(m) = o.try_get_amount() {
                        cnts.offers_mn.offered_money += m;
                    }
                }
            }
        }

        cnts.won = {
            let required_lights = self
                .rule_set
                .constr_map_len(self.lut_a.len(), self.lut_b.len());
            merged_constraints
                .iter()
                .find(|x| x.num() == dec![10.0] && x.might_won())
                .or_else(|| merged_constraints.last())
                .map(|x| x.won(required_lights))
                .unwrap_or(false)
        };

        cnts.solvable_in = merged_constraints
            .windows(2)
            // search for the first constraint which would have been/is solvable with the
            // information available
            .find(|w| {
                let c_before = &w[0];
                let c = &w[1];

                matches!(c_before.is_solvable_after(), Ok(Some(true))) && c.might_won()
            })
            .map(|w| {
                let c = &w[1];
                (c.num() <= dec![10], c.type_str())
            })
            // use the last constraint which still is part of the regular show as fallback
            // check if it lead to a solvable state (maybe the players got lucky and guessed
            // correctly)
            .or_else(|| {
                merged_constraints
                    .iter()
                    .rev()
                    .find(|c| c.num() < dec![11] && c.might_won())
                    .and_then(|last| {
                        matches!(last.is_solvable_after(), Ok(Some(true)))
                            .then_some((last.num() + dec![1] <= dec![10], "End".to_string()))
                    })
            });
        cnts
    }

    // existing do_statistics and summary_table follow unchanged, but can call compute_cnts.
    fn do_statistics(
        &self,
        total: f64,
        merged_constraints: &[Constraint],
        solutions: Option<&Vec<MaskedMatching>>,
    ) -> Result<()> {
        let out_path = self.dir.join("stats").with_extension("json");
        let mut out_data = EvalData {
            events: vec![],
            cnts: self.compute_cnts(merged_constraints, solutions, !self.no_offerings_noted),
        };

        out_data.events.push(EvalEvent::Initial(EvalInitial {
            bits_left_after: total.log2(),
            comment: "initial".to_string(),
        }));
        for i in merged_constraints.iter().map(|c| c.get_stats()) {
            let i = i?;
            if let Some(i) = i {
                out_data.events.push(i);
            }
        }

        let file = File::create(out_path)?;
        let mut writer = BufWriter::new(file);

        serde_json::to_writer(&mut writer, &out_data)?;
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
            let row = c.summary_row_data(transpose, map_hor, &past_constraints);
            let style = if i % 2 == 0 {
                |cell: Cell| cell.bg(crate::COLOR_ALT_BG)
            } else {
                |cell: Cell| cell
            };
            table.add_row(row.render(style));

            past_constraints.push(c);
        }
        Ok(table)
    }
}

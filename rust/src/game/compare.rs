use std::fs::File;
use std::io::{BufWriter, Write};

use rust_decimal::dec;
use anyhow::Result;

use crate::constraint::compare::{EvalData, EvalEvent, EvalInitial, SumCounts, SumOffersMB, SumOffersMN};
use crate::constraint::Constraint;
use crate::game::Game;
use crate::matching_repr::MaskedMatching;


impl Game {
    /// Pure helper: compute `SumCounts` from the provided constraints and optional solutions.
    ///
    /// This takes the loop part of `do_statistics` and returns an aggregated `SumCounts`.
    /// It intentionally does **not** attempt to determine `won` or `solvable` (those need more context).
    fn compute_cnts(
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
    pub(super) fn do_statistics(
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

        Ok(())
    }
}

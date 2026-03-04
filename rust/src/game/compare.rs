// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides all functionalities to generate and store data which can later be used for
//! a comparison.
//! Prerequisite is that the evaluation already took place.

use std::fs::File;
use std::io::{BufWriter, Write};

use anyhow::Result;
use rust_decimal::dec;

use crate::constraint::compare::{ComparisonData, EvalEvent, EvalInitial, SumCounts};
use crate::constraint::evaluate::ConstraintSolvable;
use crate::constraint::evaluate_predicates::ConstraintEval;
use crate::constraint::{Constraint, ConstraintGetters};
use crate::game::Game;
use crate::matching_repr::MaskedMatching;

impl Game {
    /// writes data used in comparisons serialized as json to disk
    pub(super) fn write_comparison_data(
        &self,
        total: f64,
        merged_constraints: &[Constraint],
        solutions: Option<&Vec<MaskedMatching>>,
    ) -> Result<()> {
        let out_path = self.dir.join("stats").with_extension("json");

        let required_lights = self
            .rule_set
            .constr_map_len(self.lut_a.len(), self.lut_b.len());

        // all the data is collected here
        let mut out_data = ComparisonData {
            events: vec![],
            // obtain summary data for the whole season
            cnts: calculate_summary_data(
                merged_constraints,
                solutions,
                !self.no_offerings_noted,
                required_lights,
            ),
        };

        // insert the data for the course of the season
        // start with the initial state
        out_data.events.push(EvalEvent::Initial(EvalInitial {
            bits_left_after: total.log2(),
            comment: "initial".to_string(),
        }));
        // push data on every event that has been seen
        for i in merged_constraints.iter().map(|c| c.get_stats()) {
            let i = i?;
            if let Some(i) = i {
                out_data.events.push(i);
            }
        }

        // create file
        let file = File::create(out_path)?;
        let mut writer = BufWriter::new(file);

        // serialize data to file
        serde_json::to_writer(&mut writer, &out_data)?;
        writer.flush()?;

        Ok(())
    }
}

/// computes summary data to be used in a summary
fn calculate_summary_data<T: ConstraintEval + ConstraintGetters + ConstraintSolvable>(
    merged_constraints: &[T],
    solutions: Option<&Vec<MaskedMatching>>,
    offers_noted: bool,
    required_lights: usize,
) -> SumCounts {
    // initialization
    let mut cnts = SumCounts::default();
    cnts.offers_mb.sold_but_match_active = solutions.is_some();
    cnts.offers_mb.offers_noted = offers_noted;
    cnts.offers_mn.offers_noted = offers_noted;

    // accumulate stats over the course of the season
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
                cnts.offers_mb.offers_cnt += 1;
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
                cnts.offers_mn.offers_cnt += 1;
                if let Some(m) = o.try_get_amount() {
                    cnts.offers_mn.offered_money += m;
                }
            }
        }
    }

    // check if the season was won
    // for this they a winable constraint (MN) must be won before the show ended (num < 11)
    // store two things:
    // 1. was won during the show
    // 2. with which event was it won
    cnts.won_in = {
        merged_constraints
            .iter()
            .find(|x| x.won(required_lights))
            .map(|x| (x.num() < dec![11], x.type_str()))
    };

    // check in which event when all available information evaluated perfectly they would be
    // able to win without guessing
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
            (c.num() < dec![11], c.type_str())
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
                        .then_some((last.num() + dec![1] < dec![11], "End".to_string()))
                })
        });
    cnts
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rust_decimal::Decimal;

    use crate::constraint::compare::{SumOffersMB, SumOffersMN};
    use crate::constraint::Offer;

    use super::*;

    #[derive(Clone)]
    struct ConstraintMock {
        blackout: bool,
        match_found: bool,
        mb: bool,
        mn: bool,
        sold: bool,
        mb_hit: bool,
        offer: Option<Offer>,
        might_won: bool,
        won: bool,
        solvable_after: Option<bool>,
        num: Decimal,
        type_str: String,
        comment: String,
    }

    impl Default for ConstraintMock {
        fn default() -> Self {
            Self {
                blackout: false,
                match_found: false,
                mb: false,
                mn: false,
                sold: false,
                mb_hit: false,
                offer: None,
                might_won: false,
                won: false,
                solvable_after: None,
                num: dec![1],
                type_str: "".to_string(),
                comment: "".to_string(),
            }
        }
    }

    impl ConstraintGetters for ConstraintMock {
        fn comment(&self) -> &str {
            &self.comment
        }

        fn type_str(&self) -> String {
            self.type_str.clone()
        }

        fn num(&self) -> Decimal {
            self.num
        }
    }
    impl ConstraintEval for ConstraintMock {
        fn is_blackout(&self) -> bool {
            self.blackout
        }

        fn is_match_found(&self) -> bool {
            self.match_found
        }

        fn is_mb(&self) -> bool {
            self.mb
        }

        fn is_mn(&self) -> bool {
            self.mn
        }

        fn is_sold(&self) -> bool {
            self.sold
        }

        fn is_mb_hit(&self, _: Option<&Vec<MaskedMatching>>) -> bool {
            self.mb_hit
        }

        fn try_get_offer(&self) -> Option<Offer> {
            self.offer.clone()
        }

        fn might_won(&self) -> bool {
            self.might_won
        }

        fn won(&self, _: usize) -> bool {
            self.won
        }
    }
    impl ConstraintSolvable for ConstraintMock {
        fn is_solvable_after(&self) -> Result<Option<bool>> {
            Ok(self.solvable_after)
        }
    }

    #[test]
    fn calculate_summary_data_aggregation() {
        // empty
        let res = calculate_summary_data::<ConstraintMock>(&[], None, false, 10);
        let reference = SumCounts {
            blackouts: 0,
            won_in: None,
            matches_found: 0,
            solvable_in: None,
            offers_mn: SumOffersMN {
                sold_cnt: 0,
                offers_noted: false,
                offers_cnt: 0,
                offered_money: 0,
            },
            offers_mb: SumOffersMB {
                sold_cnt: 0,
                offers_noted: false,
                offers_cnt: 0,
                offered_money: 0,
                sold_but_match_active: false,
                sold_but_match: 0,
                offer_and_match: 0,
            },
        };
        assert_eq!(res, reference);

        // simple
        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    blackout: true,
                    ..Default::default()
                },
                ConstraintMock {
                    match_found: true,
                    sold: true,
                    ..Default::default()
                },
                ConstraintMock {
                    mb: true,
                    ..Default::default()
                },
                ConstraintMock {
                    mn: true,
                    ..Default::default()
                },
            ],
            None,
            false,
            10,
        );
        let reference = SumCounts {
            blackouts: 1,
            won_in: None,
            matches_found: 1,
            solvable_in: None,
            offers_mn: SumOffersMN {
                sold_cnt: 0,
                offers_noted: false,
                offers_cnt: 0,
                offered_money: 0,
            },
            offers_mb: SumOffersMB {
                sold_cnt: 0,
                offers_noted: false,
                offers_cnt: 0,
                offered_money: 0,
                sold_but_match_active: false,
                sold_but_match: 0,
                offer_and_match: 0,
            },
        };
        assert_eq!(res, reference);

        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    mb: true,
                    ..Default::default()
                },
                ConstraintMock {
                    mn: true,
                    offer: Some(Offer::Group {
                        amount: Some(5),
                        by: "".to_string(),
                    }),
                    sold: true,
                    ..Default::default()
                },
                ConstraintMock {
                    mn: true,
                    offer: Some(Offer::Group {
                        amount: Some(5),
                        by: "".to_string(),
                    }),
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        let reference = SumCounts {
            blackouts: 0,
            won_in: None,
            matches_found: 0,
            solvable_in: None,
            offers_mn: SumOffersMN {
                sold_cnt: 1,
                offers_noted: true,
                offers_cnt: 2,
                offered_money: 10,
            },
            offers_mb: SumOffersMB {
                sold_cnt: 0,
                offers_noted: true,
                offers_cnt: 0,
                offered_money: 0,
                sold_but_match_active: false,
                sold_but_match: 0,
                offer_and_match: 0,
            },
        };
        assert_eq!(res, reference);

        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    mb: true,
                    sold: true,
                    offer: Some(Offer::Group {
                        amount: Some(5),
                        by: "".to_string(),
                    }),
                    ..Default::default()
                },
                ConstraintMock {
                    mb: true,
                    offer: Some(Offer::Group {
                        amount: Some(5),
                        by: "".to_string(),
                    }),
                    ..Default::default()
                },
                ConstraintMock {
                    mn: true,
                    sold: true,
                    mb_hit: true,
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        assert_eq!(
            res.offers_mb,
            SumOffersMB {
                sold_cnt: 1,
                offers_noted: true,
                offers_cnt: 2,
                offered_money: 10,
                sold_but_match_active: false,
                sold_but_match: 0,
                offer_and_match: 0,
            }
        );

        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    mb: true,
                    sold: true,
                    offer: Some(Offer::Single {
                        amount: Some(5),
                        by: "".to_string(),
                        reduced_pot: false,
                        save: true,
                    }),
                    mb_hit: true,
                    ..Default::default()
                },
                ConstraintMock {
                    mb: true,
                    offer: Some(Offer::Single {
                        amount: Some(5),
                        by: "".to_string(),
                        reduced_pot: false,
                        save: true,
                    }),
                    mb_hit: true,
                    ..Default::default()
                },
            ],
            Some(&vec![MaskedMatching::from_matching_ref(&[
                vec![0],
                vec![1],
            ])]),
            true,
            10,
        );
        assert_eq!(
            res.offers_mb,
            SumOffersMB {
                sold_cnt: 1,
                offers_noted: true,
                offers_cnt: 2,
                offered_money: 10,
                sold_but_match_active: true,
                sold_but_match: 1,
                offer_and_match: 2,
            }
        );
    }

    #[test]
    fn calculate_summary_data_won_in() {
        // won in time
        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    won: false,
                    num: dec![5],
                    ..Default::default()
                },
                ConstraintMock {
                    won: true,
                    type_str: "MB+10.9".to_string(),
                    num: dec![10.9],
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        assert_eq!(res.won_in, Some((true, "MB+10.9".to_string())));

        // "won" out of time
        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    won: false,
                    num: dec![5],
                    ..Default::default()
                },
                ConstraintMock {
                    won: true,
                    type_str: "MB+11.0".to_string(),
                    num: dec![11.0],
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        assert_eq!(res.won_in, Some((false, "MB+11.0".to_string())));

        // not won
        let res = calculate_summary_data::<ConstraintMock>(
            &[ConstraintMock {
                won: false,
                num: dec![5],
                ..Default::default()
            }],
            None,
            true,
            10,
        );
        assert_eq!(res.won_in, None);
    }

    #[test]
    fn calculate_summary_data_solvable_in() {
        // solvable in time
        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    num: dec![5],
                    solvable_after: Some(true),
                    ..Default::default()
                },
                ConstraintMock {
                    might_won: true,
                    type_str: "MB+10.9".to_string(),
                    num: dec![10.9],
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        assert_eq!(res.solvable_in, Some((true, "MB+10.9".to_string())));

        // solvable out of time
        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    num: dec![5],
                    solvable_after: Some(true),
                    ..Default::default()
                },
                ConstraintMock {
                    might_won: true,
                    type_str: "MB+11".to_string(),
                    num: dec![11],
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        assert_eq!(res.solvable_in, Some((false, "MB+11".to_string())));

        // solvable but nothing comes after (e.g. they won the game through "guessing")
        // -> guessing was too late so there would not have been an opportunity left to officially
        // solve it now after the solution is known
        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    num: dec![5],
                    ..Default::default()
                },
                ConstraintMock {
                    solvable_after: Some(true),
                    might_won: true,
                    type_str: "MB+10.9".to_string(),
                    num: dec![10.9],
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        assert_eq!(res.solvable_in, Some((false, "End".to_string())));

        // solvable but nothing comes after (e.g. they won the game through "guessing")
        // -> guessing was early enough so now that the game is solved there would be an
        // opportunity left to officially solve it
        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    num: dec![5],
                    ..Default::default()
                },
                ConstraintMock {
                    solvable_after: Some(true),
                    might_won: true,
                    type_str: "MB+9.9".to_string(),
                    num: dec![9.9],
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        assert_eq!(res.solvable_in, Some((true, "End".to_string())));

        // not solvable
        let res = calculate_summary_data::<ConstraintMock>(
            &[
                ConstraintMock {
                    num: dec![5],
                    ..Default::default()
                },
                ConstraintMock {
                    solvable_after: Some(false),
                    might_won: true,
                    type_str: "MB+9.9".to_string(),
                    num: dec![9.9],
                    ..Default::default()
                },
            ],
            None,
            true,
            10,
        );
        assert_eq!(res.solvable_in, None);
    }
}

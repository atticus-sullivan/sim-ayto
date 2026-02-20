use anyhow::{Context, Result};
/// This module contains getters / evaluations which is used to pre-process the gathered data for a
/// comparison with other simulations.
/// The root is `EvalData` (which is at some point serialized/stored so the comparison can take
/// place later)
use rust_decimal::{dec, Decimal};
use serde::{Deserialize, Serialize};

use crate::constraint::{Constraint, ConstraintType};

/// Container of evaluation output used for plotting and summaries.
///
/// - `events` are the chronological evaluation events (MB/MN/Initial).
/// - `cnts` are aggregated counters and summary data.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EvalData {
    pub events: Vec<EvalEvent>,
    pub cnts: SumCounts,
}

/// One recorded evaluation event (MB/MN/Initial).
///
/// These are intended to be serialized as JSON for the site and to drive plots.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum EvalEvent {
    MB(EvalMB),
    MN(EvalMN),
    Initial(EvalInitial),
}

macro_rules! eval_event_query_data {
    (
        $data_name:ident,
        $data_expl:literal,
        $ret:ty,
        MN($mn_var:ident) => $mn_body:expr,
        MB($mb_var:ident) => $mb_body:expr,
        Initial($ini_var:ident) => $init_body:expr
    ) => {
        /// Map the `EvalEvent` to `$data_name`$data_expl.
        ///
        /// The closure parameters (for MB, MN, Initial) control whether the variant
        /// should be included.
        pub fn $data_name<MnPred, MbPred, InitPred>(
            &self,
            mn: MnPred,
            mb: MbPred,
            init: InitPred,
        ) -> Option<$ret>
        where
            MnPred: Fn(&EvalMN) -> bool,
            MbPred: Fn(&EvalMB) -> bool,
            InitPred: Fn(&EvalInitial) -> bool,
        {
            match self {
                EvalEvent::MN($mn_var) => {
                    if mn($mn_var) {
                        $mn_body
                    } else {
                        None
                    }
                }
                EvalEvent::MB($mb_var) => {
                    if mb($mb_var) {
                        $mb_body
                    } else {
                        None
                    }
                }
                EvalEvent::Initial($ini_var) => {
                    if init($ini_var) {
                        $init_body
                    } else {
                        None
                    }
                }
            }
        }
    };
}

/// Return the 'num' value for the event depending on the caller's filter.
impl EvalEvent {
    eval_event_query_data!(
        num,
        "",
        Decimal,
        MN(eval_mn) => Some(eval_mn.num),
        MB(eval_mb) => Some(eval_mb.num),
        Initial(ini) => Some(dec![0])
    );

    eval_event_query_data!(
        num_unified,
        " (compute num to be normally evenly spaced, monotonically increasing)",
        Decimal,
        MN(eval_mn) => Some(eval_mn.num * dec![2]),
        MB(eval_mb) => Some(eval_mb.num * dec![2] - dec![1]),
        Initial(ini) => Some(dec![0])
    );

    eval_event_query_data!(
        comment,
        "",
        String,
        MN(eval_mn) => Some(eval_mn.comment.clone()),
        MB(eval_mb) => Some(eval_mb.comment.clone()),
        Initial(ini) => Some("initial".to_string())
    );

    eval_event_query_data!(
        bits_gained,
        "",
        f64,
        MN(eval_mn) => Some(eval_mn.bits_gained),
        MB(eval_mb) => Some(eval_mb.bits_gained),
        Initial(ini) => None
    );

    eval_event_query_data!(
        bits_left_after,
        "",
        f64,
        MN(eval_mn) => Some(eval_mn.bits_left_after),
        MB(eval_mb) => Some(eval_mb.bits_left_after),
        Initial(ini) => Some(ini.bits_left_after)
    );

    eval_event_query_data!(
        lights_total,
        " (if available/set)",
        u8,
        MN(eval_mn) => Some(eval_mn.lights_total?),
        MB(eval_mb) => Some(eval_mb.lights_total?),
        Initial(ini) => None
    );

    eval_event_query_data!(
        lights_known_before,
        "",
        u8,
        MN(eval_mn) => Some(eval_mn.lights_known_before),
        MB(eval_mb) => Some(eval_mb.lights_known_before),
        Initial(ini) => None
    );

    eval_event_query_data!(
        new_lights,
        " (lights_total - lights_known_before)",
        u8,
        MN(eval_mn) => Some(eval_mn.lights_total? - eval_mn.lights_known_before),
        MB(eval_mb) => Some(eval_mb.lights_total? - eval_mb.lights_known_before),
        Initial(ini) => None
    );
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalInitial {
    pub bits_left_after: f64,
    pub comment: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalMB {
    #[serde(with = "rust_decimal::serde::float")]
    pub num: Decimal,
    pub bits_left_after: f64,
    pub lights_total: Option<u8>,
    pub lights_known_before: u8,
    pub bits_gained: f64,
    pub comment: String,
    pub offer: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalMN {
    #[serde(with = "rust_decimal::serde::float")]
    pub num: Decimal,
    pub bits_left_after: f64,
    pub lights_total: Option<u8>,
    pub lights_known_before: u8,
    pub bits_gained: f64,
    pub comment: String,
    pub offer: bool,
}

/// Aggregated counts and summary metrics for a run / ruleset.
///
/// Provide small helper methods to update and combine counts.
/// Keep `add(&mut self, other: &SumCounts)` as a pure & cheap aggregator.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SumCounts {
    pub blackouts: u8,
    pub won: bool,
    pub matches_found: u8,
    pub solvable_after: Option<(bool, String)>,

    pub offers_mn: SumOffersMN,
    pub offers_mb: SumOffersMB,
}

/// Collect sums regarding offers made for MBs
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SumOffersMB {
    pub sold_but_match_active: bool,
    pub sold_cnt: u8,
    pub sold_but_match: u8,

    pub offers_noted: bool,
    pub offers: u64,
    pub offer_and_match: u64,
    pub offered_money: u128,
}
/// Collect sums regarding offers made for MNs
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SumOffersMN {
    pub sold_cnt: u8,

    pub offers_noted: bool,
    pub offers: u64,
    pub offered_money: u128,
}

impl SumOffersMB {
    /// Increment this `SumOffersMB` with values found in `other`.
    ///
    /// This is an in-place, allocation-free aggregator used while building a summary.
    pub fn add(&mut self, other: &Self) {
        self.sold_cnt += other.sold_cnt;
        self.sold_but_match += other.sold_but_match;
        self.sold_but_match_active |= other.sold_but_match_active;

        self.offers += other.offers;
        self.offered_money += other.offered_money;
        self.offer_and_match += other.offer_and_match;
        self.offers_noted |= other.offers_noted;
    }
}
impl SumOffersMN {
    /// Increment this `SumOffersMB` with values found in `other`.
    ///
    /// This is an in-place, allocation-free aggregator used while building a summary.
    pub fn add(&mut self, other: &Self) {
        self.sold_cnt += other.sold_cnt;

        self.offers += other.offers;
        self.offered_money += other.offered_money;
        self.offers_noted |= other.offers_noted;
    }
}

impl SumCounts {
    /// Increment this `SumCounts` with values found in `other`.
    ///
    /// This is an in-place, allocation-free aggregator used while building a summary.
    pub fn add(&mut self, other: &Self) {
        self.blackouts += other.blackouts;

        self.offers_mn.add(&other.offers_mn);
        self.offers_mb.add(&other.offers_mb);
    }
}

impl Constraint {
    pub fn get_stats(&self) -> Result<Option<EvalEvent>> {
        if self.hidden {
            return Ok(None);
        }

        let meta_b = format!("{}-{}", self.type_str(), self.comment());
        match &self.r#type {
            ConstraintType::Night { num, offer, .. } => Ok(Some(EvalEvent::MN(EvalMN {
                offer: offer.is_some(),
                num: *num,
                lights_total: self.check.as_lights(),
                lights_known_before: self.known_lights,
                bits_gained: self.information.unwrap_or(f64::INFINITY),
                bits_left_after: (self.left_after.context("total_left unset")? as f64).log2(),
                comment: meta_b,
            }))),
            ConstraintType::Box { num, .. } => Ok(Some(EvalEvent::MB(EvalMB {
                offer: {
                    if let ConstraintType::Box { offer, .. } = &self.r#type {
                        offer.is_some()
                    } else {
                        false
                    }
                },
                num: *num,
                lights_total: self.check.as_lights(),
                lights_known_before: self.known_lights,
                bits_gained: self.information.unwrap_or(f64::INFINITY),
                bits_left_after: (self.left_after.context("total_left unset")? as f64).log2(),
                comment: meta_b,
            }))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use crate::{
        constraint::CheckType, matching_repr::MaskedMatching, ruleset_data::dummy::DummyData,
    };

    use super::*;

    #[test]
    fn get_stats_simple() {
        let c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::from([("A".to_string(), "a".to_string())]),
            check: CheckType::Lights(1, BTreeMap::new()),
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 1]; 1],
            information: Some(2.0),
            left_after: Some(1024),
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![3.0],
                comment: "".to_string(),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        if let Ok(Some(EvalEvent::MB(ev))) = c.get_stats() {
            assert_eq!(ev.num, dec![3.0]);
            assert_eq!(ev.lights_total, Some(1u8));
            assert!((ev.bits_left_after - (1024f64).log2()).abs() < 1e-9);
            assert!((ev.bits_gained - 2.0).abs() < 1e-9);
        } else {
            panic!("expected MB event");
        }
    }

    #[test]
    fn sumcounts_add_simple() {
        let mut a = SumCounts {
            blackouts: 1,
            offers_mb: SumOffersMB {
                sold_cnt: 2,
                sold_but_match: 0,
                sold_but_match_active: true,
                offers_noted: false,
                offer_and_match: 0,
                offered_money: 0,
                offers: 0,
            },
            offers_mn: SumOffersMN {
                sold_cnt: 2,
                offers_noted: false,
                offered_money: 0,
                offers: 0,
            },
            matches_found: 1,
            won: false,
            solvable_after: Some((true, "".to_string())),
        };
        let b = SumCounts {
            blackouts: 2,
            offers_mb: SumOffersMB {
                sold_cnt: 2,
                sold_but_match: 0,
                sold_but_match_active: true,
                offers_noted: false,
                offer_and_match: 0,
                offered_money: 0,
                offers: 0,
            },
            offers_mn: SumOffersMN {
                sold_cnt: 2,
                offers_noted: false,
                offered_money: 0,
                offers: 0,
            },
            matches_found: 0,
            won: true,
            solvable_after: Some((false, "".to_string())),
        };
        a.add(&b);

        assert_eq!(a.blackouts, 3);
        assert_eq!(a.matches_found, 1);
        assert_eq!(a.offers_mn.sold_cnt, 4);
        assert!(a.offers_mb.sold_but_match_active);
    }

    #[test]
    fn eval_event_query_data_simple() {
        // MB event
        let mb = EvalMB {
            num: dec![2.0],
            bits_left_after: 8.0,
            lights_total: Some(3),
            lights_known_before: 1,
            bits_gained: 2.5,
            comment: "mb".to_string(),
            offer: true,
        };
        let ev_mb = EvalEvent::MB(mb.clone());

        // MN event
        let mn = EvalMN {
            num: dec![4.0],
            bits_left_after: 16.0,
            lights_total: Some(2),
            lights_known_before: 0,
            bits_gained: 3.5,
            comment: "mn".to_string(),
            offer: false,
        };
        let ev_mn = EvalEvent::MN(mn.clone());

        // Initial event
        let ini = EvalInitial {
            bits_left_after: 32.0,
            comment: "init".to_string(),
        };
        let ev_ini = EvalEvent::Initial(ini.clone());

        // MB: get number using closures (mn_pred, mb_pred, init_pred)
        assert_eq!(ev_mb.num(|_| false, |_| true, |_| false), Some(dec![2.0]));
        assert_eq!(ev_mn.num(|_| true, |_| false, |_| false), Some(dec![4.0]));
        assert_eq!(ev_ini.num(|_| false, |_| false, |_| true), Some(dec![0]));

        // bits_gained: MB and MN present, Initial -> None
        assert_eq!(ev_mb.bits_gained(|_| false, |_| true, |_| false), Some(2.5));
        assert_eq!(ev_mn.bits_gained(|_| true, |_| false, |_| false), Some(3.5));
        assert_eq!(ev_ini.bits_gained(|_| false, |_| false, |_| true), None);

        // lights_total: present for MB/MN; Initial -> None
        assert_eq!(ev_mb.lights_total(|_| false, |_| true, |_| false), Some(3));
        assert_eq!(ev_mn.lights_total(|_| true, |_| false, |_| false), Some(2));
        assert_eq!(ev_ini.lights_total(|_| false, |_| false, |_| true), None);

        // new_lights = lights_total - lights_known_before
        assert_eq!(ev_mb.new_lights(|_| false, |_| true, |_| false), Some(2));
        assert_eq!(ev_mn.new_lights(|_| true, |_| false, |_| false), Some(2));

        // lights_known_before: present for MB/MN; Initial -> None
        assert_eq!(
            ev_mb.lights_known_before(|_| false, |_| true, |_| false),
            Some(1)
        );
        assert_eq!(
            ev_mn.lights_known_before(|_| true, |_| false, |_| false),
            Some(0)
        );
        assert_eq!(
            ev_ini.lights_known_before(|_| false, |_| false, |_| true),
            None
        );

        // num_unified: num but scaled so it climbs monotonically
        assert_eq!(
            ev_mb.num_unified(|_| false, |_| true, |_| false),
            Some(dec![3.0])
        );
        assert_eq!(
            ev_mn.num_unified(|_| true, |_| false, |_| false),
            Some(dec![8.0])
        );
        assert_eq!(
            ev_ini.num_unified(|_| false, |_| false, |_| true),
            Some(dec![0.0])
        );

        // bits_left_after: present for MB/MN; Initial -> None
        assert_eq!(
            ev_mb.bits_left_after(|_| false, |_| true, |_| false),
            Some(8.0)
        );
        assert_eq!(
            ev_mn.bits_left_after(|_| true, |_| false, |_| false),
            Some(16.0)
        );
        assert_eq!(
            ev_ini.bits_left_after(|_| false, |_| false, |_| true),
            Some(32.0)
        );

        // comment: present for MB/MN; Initial -> None
        assert_eq!(
            ev_mb.comment(|_| false, |_| true, |_| false),
            Some("mb".to_string())
        );
        assert_eq!(
            ev_mn.comment(|_| true, |_| false, |_| false),
            Some("mn".to_string())
        );
        assert_eq!(
            ev_ini.comment(|_| false, |_| false, |_| true),
            Some("initial".to_string())
        );
    }
}

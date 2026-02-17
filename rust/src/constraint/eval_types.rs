use rust_decimal::{dec, Decimal};
use serde::{Deserialize, Serialize};

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
    pub solvable: Option<bool>,

    pub sold: u8,
    pub sold_but_match: u8,
    pub sold_but_match_active: bool,

    pub offers_noted: bool,
    pub offers: u64,
    pub offer_and_match: u64,
    pub offered_money: u128,
}

impl SumCounts {
    /// Increment this `SumCounts` with values found in `other`.
    ///
    /// This is an in-place, allocation-free aggregator used while building a summary.
    pub fn add(&mut self, other: &Self) {
        self.blackouts += other.blackouts;

        self.sold += other.sold;
        self.sold_but_match += other.sold_but_match;
        // self.sold_but_match_active &= other.sold_but_match_active;

        self.matches_found += other.matches_found;

        self.offers += other.offers;
        self.offer_and_match += other.offer_and_match;
        self.offered_money += other.offered_money;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sumcounts_add_combines_counts() {
        let mut a = SumCounts {
            blackouts: 1,
            sold: 2,
            sold_but_match: 0,
            sold_but_match_active: true,
            matches_found: 1,
            won: false,
            offers_noted: false,
            offer_and_match: 0,
            offered_money: 0,
            offers: 0,
            solvable: Some(true),
        };
        let b = SumCounts {
            blackouts: 2,
            sold: 1,
            sold_but_match: 3,
            sold_but_match_active: false,
            matches_found: 0,
            won: true,
            offers_noted: true,
            offer_and_match: 1,
            offered_money: 100,
            offers: 2,
            solvable: Some(false),
        };
        a.add(&b);
        assert_eq!(a.blackouts, 3);
        assert_eq!(a.sold, 3);
        assert_eq!(a.matches_found, 1);
        assert_eq!(a.offers, 2);
    }
}

/// This module implements all types of constraints limiting the possible solutions as the game
/// goes on.
///
/// Usually the life-cycle of a constraint is as follows:
/// 1. Parsing -> see the parse module
///   1. parsed from yaml to `ConstraintParse`
///   2. converted to `Constraint`
/// 2. Simulating -> see the simulate module
///   - here `process` is the core function which plugs everything together
/// 3. Evaluating -> see the evaluate module
///   - after the simulation is done the stats collected during the simulation are evaluated
/// 4. Comparing -> see the compare module
///   - somehow similar to the evaluation. But here data is pre-processed and stored for a
///     comparison with other simulations at a later point in time.
///
/// Some submodules have helper modules to group some kinds of dedicated functions.
///
/// This specific module only implements the "real" (in contrast to parsing) datatypes and some
/// simple getters.

pub mod compare;
pub(super) mod evaluate;
pub(super) mod evaluate_predicates;
pub(super) mod parse;
pub(super) mod parse_utils;
pub(super) mod report;
pub(super) mod simulate;
mod report_utils;

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::matching_repr::{bitset::Bitset, MaskedMatching};
use crate::ruleset_data::RuleSetData;
use crate::MapS;

/// Types used to decide how to check a matching against a constraint.
///
/// - `Eq` checks that two entries are equal (used for "box" equality constraints).
/// - `Nothing` no-op check.
/// - `Sold` special check for sold events.
/// - `Lights(n, stats)` checks exact number of lights; `stats` is a mutable bucket
///   map used while processing to accumulate frequencies (kept in the Constraint).
#[derive(Deserialize, Debug, Clone, Hash)]
pub enum CheckType {
    Eq,
    Nothing,
    Sold,
    Lights(u8, #[serde(skip)] BTreeMap<u8, u128>),
}

impl CheckType {
    /// Return the lights-count if this `CheckType` is `Lights`.
    pub fn as_lights(&self) -> Option<u8> {
        if let CheckType::Lights(l, _) = *self {
            Some(l)
        } else {
            None
        }
    }
}

/// An offer attached to a box event. The enum mirrors the YAML structure and
/// contains optional amounts and actors.
#[derive(Deserialize, Debug, Clone)]
pub enum Offer {
    Single {
        amount: Option<u128>,
        by: String,
        #[serde(rename = "reducedPot")]
        reduced_pot: bool,
        save: bool,
    },
    SinglePair {
        amount: Option<u128>,
        #[serde(rename = "byA")]
        by_a: String,
        #[serde(rename = "byB")]
        by_b: String,
        #[serde(rename = "reducedPot")]
        reduced_pot: bool,
        save: bool,
    },
    Group {
        amount: Option<u128>,
        by: String,
    },
    GroupPair {
        amount: Option<u128>,
        #[serde(rename = "byA")]
        by_a: String,
        #[serde(rename = "byB")]
        by_b: String,
    },
}

impl Offer {
    /// Return the numeric `amount` if present on the offer.
    pub fn try_get_amount(&self) -> Option<u128> {
        match &self {
            Offer::Single { amount, .. } => *amount,
            Offer::SinglePair { amount, .. } => *amount,
            Offer::Group { amount, .. } => *amount,
            Offer::GroupPair { amount, .. } => *amount,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub enum ConstraintType {
    Night {
        num: Decimal,
        comment: String,
    },
    Box {
        num: Decimal,
        comment: String,
        offer: Option<Offer>,
    },
}

impl Hash for ConstraintType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ConstraintType::Night { num, comment } => {
                0.hash(state); // A constant to distinguish this variant
                num.hash(state);
                comment.hash(state);
            }
            ConstraintType::Box {
                num,
                comment,
                offer,
            } => {
                1.hash(state); // A constant to distinguish this variant
                num.hash(state);
                comment.hash(state);
                offer.hash(state)
            }
        }
    }
}
impl Hash for Offer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Offer::Single {
                amount,
                by,
                reduced_pot,
                save,
            } => {
                0.hash(state); // A constant to distinguish this variant
                if let Some(amount) = amount {
                    amount.hash(state);
                }
                by.hash(state);
                reduced_pot.hash(state);
                save.hash(state);
            }
            Offer::SinglePair {
                amount,
                reduced_pot,
                save,
                by_a,
                by_b,
            } => {
                1.hash(state); // A constant to distinguish this variant
                if let Some(amount) = amount {
                    amount.hash(state);
                }
                by_a.hash(state);
                by_b.hash(state);
                reduced_pot.hash(state);
                save.hash(state);
            }
            Offer::Group { amount, by } => {
                2.hash(state); // A constant to distinguish this variant
                if let Some(amount) = amount {
                    amount.hash(state);
                }
                by.hash(state);
            }
            Offer::GroupPair { amount, by_a, by_b } => {
                3.hash(state); // A constant to distinguish this variant
                if let Some(amount) = amount {
                    amount.hash(state);
                }
                by_a.hash(state);
                by_b.hash(state);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Constraint {
    r#type: ConstraintType,
    pub check: CheckType,
    hidden: bool,
    result_unknown: bool,
    build_tree: bool,

    map: MaskedMatching,
    map_s: MapS,
    exclude: Option<(u8, Bitset)>,
    eliminated: u128,
    eliminated_tab: Vec<Vec<u128>>,

    information: Option<f64>,
    left_after: Option<u128>,
    left_poss: Vec<MaskedMatching>,

    hide_ruleset_data: bool,
    pub ruleset_data: Box<dyn RuleSetData>,
    known_lights: u8,
}

impl Constraint {
    #[cfg(test)]
    pub fn new(
        r#type: ConstraintType,
        check: CheckType,
        hidden: bool,
        result_unknown: bool,
        build_tree: bool,
        map: MaskedMatching,
        map_s: MapS,
        exclude: Option<(u8, Bitset)>,
        eliminated: u128,
        eliminated_tab: Vec<Vec<u128>>,
        information: Option<f64>,
        left_after: Option<u128>,
        left_poss: Vec<MaskedMatching>,
        hide_ruleset_data: bool,
        ruleset_data: Box<dyn RuleSetData>,
        known_lights: u8,
    ) -> Self {
        Self {
            r#type,
            check,
            hidden,
            result_unknown,
            build_tree,
            map,
            map_s,
            exclude,
            eliminated,
            eliminated_tab,
            information,
            left_after,
            left_poss,
            hide_ruleset_data,
            ruleset_data,
            known_lights,
        }
    }

    /// Create a new `Constraint`. The most important data can be passed as arguments, the
    /// remaining fields will be filled with typical defaults.
    pub fn new_with_defaults(
        t: ConstraintType,
        check: CheckType,
        map: MaskedMatching,
        rs_dat: Box<dyn RuleSetData>,
        a_len: usize,
        b_len: usize,
        known_lights: u8,
    ) -> Self {
        Constraint {
            r#type: t,
            check,
            hidden: false,
            result_unknown: false,
            build_tree: false,
            map,
            map_s: MapS::default(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![vec![0; b_len]; a_len],
            information: None,
            left_after: None,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: rs_dat,
            known_lights,
        }
    }
}

// getter functions
impl Constraint {
    /// Return user-supplied comment from the underlying `ConstraintType`.
    pub(super) fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { comment, .. } => comment,
            ConstraintType::Box { comment, .. } => comment,
        }
    }

    /// Textual representation of the constraint type (used in summary tables).
    pub(super) fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, .. } => format!("MN#{}", num),
            ConstraintType::Box { num, .. } => format!("MB#{}", num),
        }
    }

    /// The numeric index associated with this constraint (MB or MN index).
    pub(super) fn num(&self) -> Decimal {
        match &self.r#type {
            ConstraintType::Night { num, .. } => *num,
            ConstraintType::Box { num, .. } => *num,
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use crate::ruleset_data::dummy::DummyData;

    use super::*;

    #[test]
    fn as_lights_simple() {
        assert_eq!(CheckType::Eq.as_lights(), None);
        assert_eq!(CheckType::Lights(3, BTreeMap::new()).as_lights(), Some(3));
    }

    #[test]
    fn try_get_amount_simple() {
        let o = Offer::Single {
            amount: Some(42),
            by: "x".into(),
            reduced_pot: false,
            save: false,
        };
        assert_eq!(o.try_get_amount(), Some(42u128));

        let o = Offer::Group {
            amount: None,
            by: "x".into(),
        };
        assert_eq!(o.try_get_amount(), None);
    }

    #[test]
    fn new_with_defaults_simple() {
        let data: Box<dyn RuleSetData> = Box::new(DummyData::default());
        let mm = MaskedMatching::from_matching_ref(&[vec![0u8]]);

        let c = Constraint::new_with_defaults(
            ConstraintType::Night {
                num: dec![1.0],
                comment: String::new(),
            },
            CheckType::Nothing,
            mm,
            data,
            1,
            1,
            0,
        );
        // basic invariants
        assert_eq!(c.eliminated, 0);
    }
}

// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements all types of constraints limiting the possible solutions as the game
//! goes on.
//!
//! Usually the life-cycle of a constraint is as follows:
//! 1. Parsing -> see the parse module
//!   1. parsed from yaml to `ConstraintParse`
//!   2. converted to `Constraint`
//! 2. Simulating -> see the simulate module
//!   - here `process` is the core function which plugs everything together
//! 3. Evaluating -> see the evaluate module
//!   - after the simulation is done the stats collected during the simulation are evaluated
//! 4. Comparing -> see the compare module
//!   - somehow similar to the evaluation. But here data is pre-processed and stored for a
//!     comparison with other simulations at a later point in time.
//!
//! Some submodules have helper modules to group some kinds of dedicated functions.
//!
//! This specific module only implements the "real" (in contrast to parsing) datatypes and some
//! simple getters.

pub mod check_type;
pub mod compare;
pub mod evaluate_predicates;

pub(super) mod evaluate;
pub(super) mod parse;
pub(super) mod parse_utils;
pub(super) mod report;
pub(super) mod report_hdr;
pub(super) mod report_summary;
pub(super) mod simulate;

mod report_predicates;

use std::hash::{Hash, Hasher};

use anyhow::Result;
use rust_decimal::{dec, Decimal};
use serde::Deserialize;

use crate::constraint::check_type::CheckType;
use crate::matching_repr::{bitset::Bitset, MaskedMatching};
use crate::ruleset_data::dummy::DummyData;
use crate::ruleset_data::RuleSetData;
use crate::{LightCnt, MapS};

/// An offer attached to an event. There are various ways in which offers can be made which is
/// represented in this enum.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum Offer {
    /// an offer where a single person wins the money
    Single {
        /// the amount of money which was offered
        amount: Option<u128>,
        /// the individual which accepted this offer
        by: String,
        #[serde(rename = "reducedPot")]
        /// whether this offer reduces the pot pot for the group
        reduced_pot: bool,
        /// whether it is save the money is won
        save: bool,
    },
    /// an offer where the a pair was offered money only for them
    SinglePair {
        /// the amount of money which was offered
        amount: Option<u128>,
        /// the individual from set_b to which this offer was made
        #[serde(rename = "byA")]
        by_a: String,
        /// the individual from set_b to which this offer was made
        #[serde(rename = "byB")]
        by_b: String,
        #[serde(rename = "reducedPot")]
        /// whether this offer reduces the pot pot for the group
        reduced_pot: bool,
        /// whether it is save the money is won
        save: bool,
    },
    /// an offer where the group was offered money for the whole group
    Group {
        /// the amount of money which was offered
        amount: Option<u128>,
        /// the individual which accepted this offer
        by: String,
    },
    /// an offer where a pair was offered money for the whole group
    GroupPair {
        /// the amount of money which was offered
        amount: Option<u128>,
        /// the individual from set_b to which this offer was made
        #[serde(rename = "byA")]
        by_a: String,
        /// the individual from set_b to which this offer was made
        #[serde(rename = "byB")]
        by_b: String,
    },
}

impl Offer {
    /// Return the numeric `amount` if present on the offer.
    pub(crate) fn try_get_amount(&self) -> Option<u128> {
        match &self {
            Offer::Single { amount, .. } => *amount,
            Offer::SinglePair { amount, .. } => *amount,
            Offer::Group { amount, .. } => *amount,
            Offer::GroupPair { amount, .. } => *amount,
        }
    }
}

/// collects the different types a constraint can have (MB/MN)
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum ConstraintType {
    /// a matching-night constraint
    Night {
        /// the number of this event, mostly used when ordering the events in the comparison
        num: Decimal,
        /// comment for this event
        comment: String,
        /// offer made for this constraint if set
        offer: Option<Offer>,
    },
    /// a match-box constraint
    Box {
        /// the number of this event, mostly used when ordering the events in the comparison
        num: Decimal,
        /// comment for this event
        comment: String,
        /// offer made for this constraint if set
        offer: Option<Offer>,
    },
}

impl Hash for ConstraintType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ConstraintType::Night {
                num,
                comment,
                offer,
            } => {
                0.hash(state); // A constant to distinguish this variant
                num.hash(state);
                comment.hash(state);
                offer.hash(state)
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

/// A struct describing a complete constraint
#[derive(Debug, Clone)]
pub struct Constraint {
    /// of what type this constraint is (e.g. MB/MN)
    r#type: ConstraintType,
    /// how the constraint needs to be checked (e.g. via lights)
    pub check: CheckType,
    /// whether this constraint shall be hidden in the output - the constraint will be merged to
    /// the next constraint in this case
    hidden: bool,
    /// whether the result of this is still unknown (despite how check is set)
    result_unknown: bool,
    /// whether to build a .dot-tree for this constraint/event
    build_tree: bool,

    /// the MaskedMatching representation of the matching related to the constraint
    map: MaskedMatching,
    /// the string+hashmap representation of the matching related to the constraint
    map_s: MapS,
    /// matchings with the individual `.0` in set_a which overlaps with `.1` from set_b (at least partially) are also
    /// eliminated by this constraint
    /// => individual `.0` is not allowed to match any individual contained in `.1`
    exclude: Option<(u8, Bitset)>,
    /// how many possibilitied were eliminated by this constraint
    eliminated: u128,
    /// how often a 1:1 matching was eliminated by this constraint. Can eventually be used to build
    /// the table of how large the share of a 1:1 matching on all remaining solutions is
    eliminated_tab: Vec<Vec<u128>>,

    /// the information gained with this constraint
    information: Option<f64>,
    /// the amount of solutions left after applying this constraint `left_poss.len()` (if the
    /// vector is filled)
    left_after: Option<u128>,
    /// all solutions left after applying this constraint (might not be filled)
    left_poss: Vec<MaskedMatching>,

    /// ruleset-specific data where ruleset-specific stats can be collected
    pub(crate) ruleset_data: Option<Box<dyn RuleSetData>>,
    /// how many lights are definitely known (via MB decisions) prior applying this constraint
    known_lights: LightCnt,
}

impl Hash for Constraint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the r#type field
        self.r#type.hash(state);

        // Sort the map_s entries by key to ensure stable hashing
        let mut sorted_entries: Vec<_> = self.map_s.iter().collect();
        sorted_entries.sort_by(|(key_a, _), (key_b, _)| key_a.cmp(key_b)); // Sort by key lexicographically

        // Hash each sorted entry
        for (key, value) in sorted_entries {
            key.hash(state);
            value.hash(state);
        }

        // Hash the check field
        self.check.hash(state);
    }
}

impl Default for Constraint {
    fn default() -> Self {
        Constraint {
            r#type: ConstraintType::Box {
                num: dec![1],
                comment: String::new(),
                offer: None,
            },
            check: CheckType::Lights(1, Default::default()),
            hidden: false,
            result_unknown: false,
            build_tree: false,
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![0], vec![0]]),
            map_s: MapS::default(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![vec![0; 3]; 3],
            information: None,
            left_after: None,
            left_poss: vec![],
            ruleset_data: Some(Box::new(DummyData::default())),
            known_lights: 0,
        }
    }
}

/// collects the functionalities used during the simulation needed from the constraint
///
/// Avoids having to pull in the whole constraint functionality when using generics
pub trait ConstraintSim {
    /// process the matching `m` with this constraint
    /// - gather stats on the way
    /// - returns whether `m` fits with this constraint (`false` -> `m` is eliminated)
    fn process(&mut self, m: &MaskedMatching) -> Result<bool>;
}

impl Constraint {
    /// Create a new `Constraint`. The most important data can be passed as arguments, the
    /// remaining fields will be filled with typical defaults.
    #[allow(clippy::field_reassign_with_default)]
    pub fn new_with_defaults(
        t: ConstraintType,
        check: CheckType,
        map: MaskedMatching,
        ruleset_data: Box<dyn RuleSetData>,
        a_len: usize,
        b_len: usize,
        known_lights: LightCnt,
    ) -> Self {
        Self {
            r#type: t,
            check,
            map,
            known_lights,
            ruleset_data: Some(ruleset_data),
            eliminated_tab: vec![vec![0; b_len]; a_len],
            ..Default::default()
        }
    }

    /// Tells how many known lights this constraint *adds*
    pub fn added_known_lights(&self) -> LightCnt {
        if self.hidden {
            return 0;
        }

        if let ConstraintType::Box { .. } = self.r#type {
            if let CheckType::Lights(1, _) = self.check {
                return 1;
            }
        }
        0
    }
}

/// collects raw getters for the constraint
///
/// Avoids having to pull in the whole constraint functionality when using generics
pub trait ConstraintGetters {
    /// Return user-supplied comment
    fn comment(&self) -> &str;
    /// Textual representation of the constraint type (used in summary tables).
    fn type_str(&self) -> String;
    /// The numeric index associated with this constraint (MB or MN index).
    fn num(&self) -> Decimal;
}

// getter functions
impl ConstraintGetters for Constraint {
    fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { comment, .. } => comment,
            ConstraintType::Box { comment, .. } => comment,
        }
    }

    fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, .. } => format!("MN#{}", num),
            ConstraintType::Box { num, .. } => format!("MB#{}", num),
        }
    }

    fn num(&self) -> Decimal {
        match &self.r#type {
            ConstraintType::Night { num, .. } => *num,
            ConstraintType::Box { num, .. } => *num,
        }
    }
}

/// A constraint can either have an impact on the remaining possibilities or not.
/// This trait can be required when it is necessary to interrogate the constraint in this regard.
pub(super) trait ConstraintImpact {
    /// whether the constraint is a no-op for the remaining possibilities or whether it has an
    /// impact.
    fn has_impact(&self) -> bool;
}

impl ConstraintImpact for Constraint {
    /// Whether this constraint actually restricts the solution set (-> is not a no-op).
    fn has_impact(&self) -> bool {
        if self.result_unknown {
            return false;
        }
        if let CheckType::Nothing | CheckType::Sold = &self.check {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use pretty_assertions::assert_eq;
    use rust_decimal::dec;

    use crate::ruleset_data::dummy::DummyData;

    use super::*;

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
                offer: None,
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

    #[test]
    fn has_impact_simple() {
        let c = Constraint {
            check: CheckType::Lights(1, BTreeMap::default()),
            result_unknown: false,
            ..Default::default()
        };
        assert!(c.has_impact());

        let c = Constraint {
            check: CheckType::Sold,
            result_unknown: false,
            ..Default::default()
        };
        assert!(!c.has_impact());

        let c = Constraint {
            check: CheckType::Nothing,
            result_unknown: false,
            ..Default::default()
        };
        assert!(!c.has_impact());

        let c = Constraint {
            check: CheckType::Lights(1, BTreeMap::default()),
            result_unknown: true,
            ..Default::default()
        };
        assert!(!c.has_impact());
    }

    #[test]
    fn type_str_simple() {
        let c = Constraint {
            r#type: ConstraintType::Night {
                num: dec![1],
                comment: "".to_string(),
                offer: None,
            },
            ..Default::default()
        };
        assert_eq!(c.type_str(), "MN#1");

        let c = Constraint {
            r#type: ConstraintType::Night {
                num: dec![3],
                comment: "".to_string(),
                offer: None,
            },
            ..Default::default()
        };
        assert_eq!(c.type_str(), "MN#3");

        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![1],
                comment: "".to_string(),
                offer: None,
            },
            ..Default::default()
        };
        assert_eq!(c.type_str(), "MB#1");

        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![3],
                comment: "".to_string(),
                offer: None,
            },
            ..Default::default()
        };
        assert_eq!(c.type_str(), "MB#3");
    }

    #[test]
    fn added_known_lights_true() {
        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(1, Default::default()),
            ..Default::default()
        };
        assert_eq!(c.added_known_lights(), 1);
    }

    #[test]
    fn added_known_lights_box_false() {
        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(2, Default::default()),
            ..Default::default()
        };
        assert_eq!(c.added_known_lights(), 0);

        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(0, Default::default()),
            ..Default::default()
        };
        assert_eq!(c.added_known_lights(), 0);
    }

    #[test]
    fn added_known_lights_night_false() {
        let c = Constraint {
            r#type: ConstraintType::Night {
                num: dec![0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(0, Default::default()),
            ..Default::default()
        };
        assert_eq!(c.added_known_lights(), 0);

        let c = Constraint {
            r#type: ConstraintType::Night {
                num: dec![0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(1, Default::default()),
            ..Default::default()
        };
        assert_eq!(c.added_known_lights(), 0);

        let c = Constraint {
            r#type: ConstraintType::Night {
                num: dec![0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(10, Default::default()),
            ..Default::default()
        };
        assert_eq!(c.added_known_lights(), 0);
    }
}

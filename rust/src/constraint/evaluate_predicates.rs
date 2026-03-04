// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains predicates to be used after the simulation has completed.
//!
//! Note there is also evaluate which contains the non-predicate functions

use crate::constraint::{CheckType, Constraint, ConstraintType, Offer};
use crate::matching_repr::{bitset::Bitset, MaskedMatching};

/// a trait which collect all functionalities to evaluate a constraint
///
/// -> can be used as generic to not have to pull in the whole constraint functionality
pub trait ConstraintEval {
    /// whether this constraint is a blackout
    fn is_blackout(&self) -> bool;
    /// whether a match was found with this constraint
    fn is_match_found(&self) -> bool;
    /// whether this constraint is a match-box
    fn is_mb(&self) -> bool;
    /// whether this constraint is a matching-night
    fn is_mn(&self) -> bool;
    /// whether this constraint was sold -> no information gain
    fn is_sold(&self) -> bool;
    /// whether this is a match-box and the match is definitive in the solution
    fn is_mb_hit(&self, sols: Option<&Vec<MaskedMatching>>) -> bool;
    /// get the offer if there has been one for this constraint
    fn try_get_offer(&self) -> Option<Offer>;
    /// whether this constraint might win the game
    fn might_won(&self) -> bool;
    /// whether the game was won with thie constraint
    fn won(&self, rl: usize) -> bool;
}

impl Constraint {
    /// whether this constraint uses lights as check-type
    pub fn is_lights(&self) -> bool {
        matches!(self.r#check, CheckType::Lights { .. })
    }
}

impl ConstraintEval for Constraint {
    fn is_blackout(&self) -> bool {
        if let ConstraintType::Night { .. } = self.r#type {
            if let CheckType::Lights(l, _) = self.check {
                return self.known_lights == l;
            }
        }
        false
    }

    fn is_mb(&self) -> bool {
        matches!(self.r#type, ConstraintType::Box { .. })
    }
    fn is_mn(&self) -> bool {
        matches!(self.r#type, ConstraintType::Night { .. })
    }

    fn is_sold(&self) -> bool {
        matches!(self.check, CheckType::Sold)
    }

    fn is_match_found(&self) -> bool {
        if let ConstraintType::Box { .. } = self.r#type {
            if let CheckType::Lights(1, _) = self.check {
                return true;
            }
        }
        false
    }

    fn is_mb_hit(&self, solutions: Option<&Vec<MaskedMatching>>) -> bool {
        if let Some(sols) = solutions {
            if let ConstraintType::Box { .. } = self.r#type {
                return sols.iter().all(|sol| {
                    self.map.iter_pairs().all(|(a, b)| {
                        sol.slot_mask(a as usize)
                            .unwrap_or(&Bitset::empty())
                            .contains_idx(b)
                    })
                });
            }
        }
        false
    }

    fn might_won(&self) -> bool {
        matches!(self.r#type, ConstraintType::Night { .. })
    }

    fn won(&self, required_lights: usize) -> bool {
        if let ConstraintType::Night { .. } = self.r#type {
            match self.check {
                CheckType::Eq => false,
                CheckType::Nothing | CheckType::Sold => false,
                CheckType::Lights(l, _) => l as usize == required_lights,
            }
        } else {
            false
        }
    }

    fn try_get_offer(&self) -> Option<Offer> {
        match &self.r#type {
            ConstraintType::Night { offer, .. } => offer.clone(),
            ConstraintType::Box { offer, .. } => offer.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::matching_repr::MaskedMatching;
    use rust_decimal::dec;
    use std::collections::BTreeMap;

    #[test]
    fn is_blackout_simple() {
        let c = Constraint {
            known_lights: 2,
            check: CheckType::Lights(2, BTreeMap::new()),
            r#type: ConstraintType::Night {
                num: dec![1],
                comment: "".to_string(),
                offer: None,
            },
            ..Default::default()
        };
        assert!(c.is_blackout());

        let c = Constraint {
            known_lights: 1,
            check: CheckType::Lights(2, BTreeMap::new()),
            r#type: ConstraintType::Night {
                num: dec![1],
                comment: "".to_string(),
                offer: None,
            },
            ..Default::default()
        };
        assert!(!c.is_blackout());
    }

    #[test]
    fn is_match_found_simple() {
        let c = Constraint {
            r#type: ConstraintType::Night {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(1, BTreeMap::new()),
            ..Default::default()
        };
        assert!(!c.is_match_found());

        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(0, BTreeMap::new()),
            ..Default::default()
        };
        assert!(!c.is_match_found());

        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            check: CheckType::Lights(1, BTreeMap::new()),
            ..Default::default()
        };
        assert!(c.is_match_found());
    }

    #[test]
    fn is_mb_hit_simple() {
        // match is 0 -> 2
        let sol = vec![MaskedMatching::from_matching_ref(&[
            vec![2],
            vec![1],
            vec![0],
        ])];

        let c = Constraint {
            r#type: ConstraintType::Night {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            map: MaskedMatching::from_matching_ref(&[vec![2]]),
            ..Default::default()
        };
        assert!(!c.is_mb_hit(Some(&sol)));

        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            ..Default::default()
        };
        assert!(!c.is_mb_hit(Some(&sol)));

        let c = Constraint {
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_string(),
                offer: None,
            },
            map: MaskedMatching::from_matching_ref(&[vec![2]]),
            ..Default::default()
        };
        assert!(c.is_mb_hit(Some(&sol)));
    }
}

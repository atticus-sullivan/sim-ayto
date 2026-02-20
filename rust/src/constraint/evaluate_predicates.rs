/// This module contains predicates to be used after the simulation has completed.
///
/// Note there is also evaluate which contains the non-predicate functions
use crate::constraint::{CheckType, Constraint, ConstraintType};
use crate::matching_repr::{bitset::Bitset, MaskedMatching};

impl Constraint {
    pub fn is_blackout(&self) -> bool {
        if let ConstraintType::Night { .. } = self.r#type {
            if let CheckType::Lights(l, _) = self.check {
                return self.known_lights == l;
            }
        }
        false
    }

    pub fn is_mb(&self) -> bool {
        matches!(self.r#type, ConstraintType::Box { .. })
    }
    pub fn is_mn(&self) -> bool {
        matches!(self.r#type, ConstraintType::Night { .. })
    }

    pub fn is_lights(&self) -> bool {
        matches!(self.r#check, CheckType::Lights { .. })
    }
    pub fn is_sold(&self) -> bool {
        matches!(self.check, CheckType::Sold)
    }

    pub fn is_match_found(&self) -> bool {
        if let ConstraintType::Box { .. } = self.r#type {
            if let CheckType::Lights(1, _) = self.check {
                return true;
            }
        }
        false
    }

    pub fn is_mb_hit(&self, solutions: Option<&Vec<MaskedMatching>>) -> bool {
        if let Some(sols) = solutions {
            if let ConstraintType::Box { .. } = self.r#type {
                return sols.iter().all(|sol| {
                    self.map.iter_pairs().all(|(a, b)| {
                        sol.slot_mask(a as usize)
                            .unwrap_or(&Bitset::empty())
                            .contains(b)
                    })
                });
            }
        }
        false
    }

    pub fn might_won(&self) -> bool {
        matches!(self.r#type, ConstraintType::Night { .. })
    }

    pub fn won(&self, required_lights: usize) -> bool {
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
        let mut c = Constraint::default();
        c.known_lights = 2;
        c.check = CheckType::Lights(2, BTreeMap::new());
        c.r#type = ConstraintType::Night { num: dec![1], comment: "".to_string(), offer: None };
        assert!(c.is_blackout());

        let mut c = Constraint::default();
        c.known_lights = 1;
        c.check = CheckType::Lights(2, BTreeMap::new());
        c.r#type = ConstraintType::Night { num: dec![1], comment: "".to_string(), offer: None };
        assert!(!c.is_blackout());
    }

    #[test]
    fn is_match_found_simple() {
        let mut c = Constraint::default();
        c.r#type = ConstraintType::Night {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.check = CheckType::Lights(1, BTreeMap::new());
        assert!(!c.is_match_found());

        let mut c = Constraint::default();
        c.r#type = ConstraintType::Box {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.check = CheckType::Lights(0, BTreeMap::new());
        assert!(!c.is_match_found());

        let mut c = Constraint::default();
        c.r#type = ConstraintType::Box {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.check = CheckType::Lights(1, BTreeMap::new());
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

        let mut c = Constraint::default();
        c.r#type = ConstraintType::Night {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.map = MaskedMatching::from_matching_ref(&[vec![2]]);
        assert!(!c.is_mb_hit(Some(&sol)));

        let mut c = Constraint::default();
        c.r#type = ConstraintType::Box {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.map = MaskedMatching::from_matching_ref(&[vec![0]]);
        assert!(!c.is_mb_hit(Some(&sol)));

        let mut c = Constraint::default();
        c.r#type = ConstraintType::Box {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.map = MaskedMatching::from_matching_ref(&[vec![2]]);
        assert!(c.is_mb_hit(Some(&sol)));
    }
}

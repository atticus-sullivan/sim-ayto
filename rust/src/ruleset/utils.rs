/// This module implements various helper functions to be used when working with the rulesets.
use anyhow::{ensure, Result};

use crate::ruleset::RuleSet;
use crate::ruleset_data::{dummy::DummyData, dup::DupData, dup_x::DupXData, RuleSetData};
use crate::Lut;

impl RuleSet {
    pub fn init_data(&self) -> Result<Box<dyn RuleSetData>> {
        Ok(match &self {
            RuleSet::SomeoneIsTrip => Box::new(DupData::default()),
            RuleSet::FixedTrip(_) => Box::new(DupData::default()),
            RuleSet::NToN => Box::new(DummyData::default()),
            RuleSet::Eq => Box::new(DummyData::default()),

            // RuleSet::XTimesDup(_, _) => Box::new(DupData::default()),
            RuleSet::XTimesDup(rs) => Box::new(DupXData::new(rs.clone())?),
        })
    }

    pub fn must_add_exclude(&self) -> bool {
        match &self {
            RuleSet::XTimesDup(_) | RuleSet::SomeoneIsTrip | RuleSet::FixedTrip(_) => true,
            RuleSet::Eq | RuleSet::NToN => false,
        }
    }

    pub fn constr_map_len(&self, a: usize, _b: usize) -> usize {
        match &self {
            RuleSet::XTimesDup(_)
            | RuleSet::SomeoneIsTrip
            | RuleSet::FixedTrip(_)
            | RuleSet::Eq => a,
            RuleSet::NToN => a / 2,
        }
    }

    pub fn must_sort_constraint(&self) -> bool {
        match &self {
            RuleSet::XTimesDup(_)
            | RuleSet::SomeoneIsTrip
            | RuleSet::FixedTrip(_)
            | RuleSet::Eq => false,
            RuleSet::NToN => true,
        }
    }

    pub fn validate_lut(&self, lut_a: &Lut, lut_b: &Lut) -> Result<()> {
        match self {
            RuleSet::XTimesDup((unkown_cnt, fixed)) => {
                let d = fixed.len() + unkown_cnt;
                ensure!(
                    lut_a.len() == lut_b.len() - d,
                    "length of setA ({}) and setB ({}) does not fit to XTimesDup (len: {}",
                    lut_a.len(),
                    lut_b.len(),
                    d
                );
                for d in fixed {
                    ensure!(
                        lut_b.contains_key(d),
                        "fixed dup ({}) is not contained in setB",
                        d
                    );
                }
            }
            RuleSet::SomeoneIsTrip => {
                ensure!(
                    lut_a.len() == lut_b.len() - 2,
                    "length of setA ({}) and setB ({}) does not fit to SomeoneIsTrip",
                    lut_a.len(),
                    lut_b.len()
                );
            }
            RuleSet::FixedTrip(s) => {
                ensure!(
                    lut_a.len() == lut_b.len() - 2,
                    "length of setA ({}) and setB ({}) does not fit to FixedTrip",
                    lut_a.len(),
                    lut_b.len()
                );
                ensure!(
                    lut_b.contains_key(s),
                    "fixed trip ({}) is not contained in setB",
                    s
                );
            }
            RuleSet::Eq => {
                ensure!(
                    lut_a.len() == lut_b.len(),
                    "length of setA ({}) and setB ({}) does not fit to Eq",
                    lut_a.len(),
                    lut_b.len()
                );
            }
            RuleSet::NToN => {
                ensure!(
                    lut_a.len() == lut_b.len(),
                    "length of setA ({}) and setB ({}) does not fit to NToN",
                    lut_a.len(),
                    lut_b.len()
                );
                ensure!(
                    lut_a == lut_b,
                    "with the n-to-n rule-set, both sets must be exactly the same"
                );
            }
        }
        Ok(())
    }

    pub fn ignore_pairing(&self, a: usize, b: usize) -> bool {
        match self {
            RuleSet::Eq
            | RuleSet::XTimesDup(_)
            | RuleSet::SomeoneIsTrip
            | RuleSet::FixedTrip(_) => false,
            RuleSet::NToN => a <= b,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_lut(keys: &[&str]) -> Lut {
        keys.iter()
            .enumerate()
            .map(|(i, k)| (k.to_string(), i))
            .collect()
    }

    #[test]
    fn init_data_simple() {
        let rs = RuleSet::Eq;
        assert!(rs.init_data().is_ok());

        let rs = RuleSet::NToN;
        assert!(rs.init_data().is_ok());

        let rs = RuleSet::SomeoneIsTrip;
        assert!(rs.init_data().is_ok());

        let rs = RuleSet::FixedTrip("x".to_string());
        assert!(rs.init_data().is_ok());

        let rs = RuleSet::XTimesDup((0, vec!["a".to_string()]));
        assert!(rs.init_data().is_ok());
    }

    #[test]
    fn must_add_exclude_true_cases() {
        assert!(RuleSet::XTimesDup((0, vec![])).must_add_exclude());
        assert!(RuleSet::SomeoneIsTrip.must_add_exclude());
        assert!(RuleSet::FixedTrip("x".to_string()).must_add_exclude());
    }

    #[test]
    fn must_add_exclude_false_cases() {
        assert!(!RuleSet::Eq.must_add_exclude());
        assert!(!RuleSet::NToN.must_add_exclude());
    }

    #[test]
    fn constr_map_len_simple() {
        let a = 5usize;
        for rs in [
            RuleSet::Eq,
            RuleSet::SomeoneIsTrip,
            RuleSet::FixedTrip("".to_string()),
            RuleSet::XTimesDup((0, vec![])),
        ] {
            assert_eq!(rs.constr_map_len(a, 0), a);
        }
    }

    #[test]
    fn constr_map_len_half() {
        let a = 8usize;
        let rs = RuleSet::NToN;
        assert_eq!(rs.constr_map_len(a, 0), a / 2);
    }

    #[test]
    fn must_sort_constraint_true_case() {
        assert!(RuleSet::NToN.must_sort_constraint());
    }

    #[test]
    fn must_sort_constraint_false_cases() {
        assert!(!RuleSet::Eq.must_sort_constraint());
        assert!(!RuleSet::XTimesDup((0, vec![])).must_sort_constraint());
        assert!(!RuleSet::SomeoneIsTrip.must_sort_constraint());
        assert!(!RuleSet::FixedTrip("x".to_string()).must_sort_constraint());
    }

    #[test]
    fn ignore_pairing_n_to_n() {
        let rs = RuleSet::NToN;
        assert!(rs.ignore_pairing(2, 5));
        assert!(rs.ignore_pairing(3, 3));
        assert!(!rs.ignore_pairing(5, 2));
    }

    #[test]
    fn ignore_pairing_other_rules_always_false() {
        let other = [
            RuleSet::Eq,
            RuleSet::XTimesDup((0, vec![])),
            RuleSet::SomeoneIsTrip,
            RuleSet::FixedTrip("x".to_string()),
        ];
        for rs in other.iter() {
            assert!(!rs.ignore_pairing(0, 0));
            assert!(!rs.ignore_pairing(1, 2));
        }
    }

    #[test]
    fn validate_lut_eq_success() {
        let rule = RuleSet::Eq;
        let a = dummy_lut(&["A", "B"]);
        let b = dummy_lut(&["a", "b"]);
        assert!(rule.validate_lut(&a, &b).is_ok());
    }

    #[test]
    fn validate_lut_eq_failure_length_mismatch() {
        let rule = RuleSet::Eq;
        let a = dummy_lut(&["A", "B"]);
        let b = dummy_lut(&["a"]);
        assert!(rule.validate_lut(&a, &b).is_err());
    }

    #[test]
    fn validate_lut_n_to_n_success() {
        let rule = RuleSet::NToN;
        let a = dummy_lut(&["A", "B"]);
        let b = dummy_lut(&["A", "B"]);
        assert!(rule.validate_lut(&a, &b).is_ok());
    }

    #[test]
    fn validate_lut_n_to_n_failure_not_identical() {
        let rule = RuleSet::NToN;
        let a = dummy_lut(&["A", "B"]);
        let b = dummy_lut(&["a", "b"]);
        assert!(rule.validate_lut(&a, &b).is_err());
    }

    #[test]
    fn validate_lut_fixed_trip_success() {
        let rule = RuleSet::FixedTrip("x".to_string());
        let a = dummy_lut(&["A", "B"]);
        let b = dummy_lut(&["a", "b", "c", "x"]);
        assert!(rule.validate_lut(&a, &b).is_ok());
    }

    #[test]
    fn validate_lut_fixed_trip_missing_key() {
        let rule = RuleSet::FixedTrip("x".to_string());
        let a = dummy_lut(&["A", "B"]);
        let b = dummy_lut(&["a", "b", "c", "d"]);
        assert!(rule.validate_lut(&a, &b).is_err());
    }

    #[test]
    fn validate_lut_someone_is_trip_success() {
        let rule = RuleSet::SomeoneIsTrip;
        let a = dummy_lut(&["A", "B"]);
        let b = dummy_lut(&["a", "b", "c", "d"]);
        assert!(rule.validate_lut(&a, &b).is_ok());
    }

    #[test]
    fn validate_lut_someone_is_trip_failure_length() {
        let rule = RuleSet::SomeoneIsTrip;
        let a = dummy_lut(&["A"]);
        let b = dummy_lut(&["a", "b"]);
        assert!(rule.validate_lut(&a, &b).is_err());
    }

    #[test]
    fn validate_lut_x_times_dup_success() {
        let rule = RuleSet::XTimesDup((1, vec!["x".to_string()]));
        let a = dummy_lut(&["A"]);
        let b = dummy_lut(&["a", "b", "x"]);
        assert!(rule.validate_lut(&a, &b).is_ok());
    }

    #[test]
    fn validate_lut_x_times_dup_missing_fixed() {
        let rule = RuleSet::XTimesDup((0, vec!["x".to_string()]));
        let a = dummy_lut(&["A"]);
        let b = dummy_lut(&["a", "b"]);
        assert!(rule.validate_lut(&a, &b).is_err());
    }
}

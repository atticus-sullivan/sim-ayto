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
    use std::collections::HashMap;

    #[test]
    fn test_validate_lut_nn() {
        let nn_rule = RuleSet::NToN;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        nn_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0)].map(|(k, v)| (k.to_string(), v)));
        assert!(nn_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(nn_rule.validate_lut(&lut_a, &lut_b).is_err());
    }

    #[test]
    fn test_validate_lut_eq() {
        let eq_rule = RuleSet::Eq;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        eq_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0)].map(|(k, v)| (k.to_string(), v)));
        assert!(eq_rule.validate_lut(&lut_a, &lut_b).is_err());
    }

    #[test]
    fn test_validate_lut_fixed_dup() {
        let dup_rule = RuleSet::XTimesDup((0, vec!["x".to_string()]));
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1), ("x", 3)].map(|(k, v)| (k.to_string(), v)));
        dup_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(dup_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1), ("c", 2)].map(|(k, v)| (k.to_string(), v)));
        assert!(dup_rule.validate_lut(&lut_a, &lut_b).is_err());
    }

    #[test]
    fn test_validate_lut_fixed_trip() {
        let trip_rule = RuleSet::FixedTrip("x".to_string());
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("a", 0), ("b", 1), ("c", 2), ("x", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        trip_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1), ("c", 2)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(trip_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("a", 0), ("b", 1), ("c", 2), ("d", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        assert!(trip_rule.validate_lut(&lut_a, &lut_b).is_err());
    }

    #[test]
    fn test_validate_lut_someone_is_dup() {
        let dup_rule = RuleSet::XTimesDup((1, vec![]));
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1), ("x", 3)].map(|(k, v)| (k.to_string(), v)));
        dup_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(dup_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1), ("c", 2)].map(|(k, v)| (k.to_string(), v)));
        dup_rule.validate_lut(&lut_a, &lut_b).unwrap();
    }

    #[test]
    fn test_validate_lut_soneone_is_trip() {
        let trip_rule = RuleSet::SomeoneIsTrip;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("a", 0), ("b", 1), ("c", 2), ("x", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        trip_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1), ("c", 2)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(trip_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("a", 0), ("b", 1), ("c", 2), ("d", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        trip_rule.validate_lut(&lut_a, &lut_b).unwrap();
    }
}

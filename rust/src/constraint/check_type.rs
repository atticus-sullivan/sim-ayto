/// This module provides a mean to represent how a constraint is checked (e.g. Lights, Sold, ...).
/// Along with the representation it also already contains means for the evaluation
use std::collections::BTreeMap;

use serde::Deserialize;

/// Type used to decide how to check a matching against a constraint.
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

    pub(super) fn calc_information_gain(&self) -> Option<Vec<(u8, f64)>> {
        match self {
            CheckType::Lights(_, ls) => {
                let total = ls.values().sum::<u128>() as f64;
                Some(
                    ls.iter()
                        .map(|(l, c)| {
                            let i = -(*c as f64 / total).log2();
                            if i == -0.0 {
                                (*l, 0.0)
                            } else {
                                (*l, i)
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            }
            _ => None,
        }
    }

    pub(super) fn calc_expected_value(&self) -> Option<f64> {
        match self {
            CheckType::Lights(_, ls) => {
                let total = ls.values().sum::<u128>() as f64;
                let expected: f64 = ls
                    .values()
                    .map(|c| {
                        let p = *c as f64 / total;
                        p * p.log2()
                    })
                    .sum();
                Some(if expected == 0.0 { -0.0 } else { expected })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round2(x: f64) -> f64 {
        (x * 100.0).round() / 100.0
    }

    #[test]
    fn as_lights_simple() {
        assert_eq!(CheckType::Eq.as_lights(), None);
        assert_eq!(CheckType::Lights(3, BTreeMap::new()).as_lights(), Some(3));
    }

    #[test]
    fn calc_information_gain_simple() {
        let ct = CheckType::Lights(
            2,
            vec![(1, 1), (2, 1)].into_iter().collect::<BTreeMap<_, _>>(),
        );
        let x = ct
            .calc_information_gain()
            .unwrap()
            .into_iter()
            .map(|(i, j)| (i, round2(j)))
            .collect::<Vec<_>>();
        assert_eq!(x, vec![(1, 1.0), (2, 1.0)]);

        let ct = CheckType::Lights(
            2,
            vec![(1, 3), (2, 1)].into_iter().collect::<BTreeMap<_, _>>(),
        );
        let x = ct
            .calc_information_gain()
            .unwrap()
            .into_iter()
            .map(|(i, j)| (i, round2(j)))
            .collect::<Vec<_>>();
        assert_eq!(x, vec![(1, 0.42), (2, 2.0)]);

        let ct = CheckType::Eq;
        let x = ct.calc_information_gain();
        assert_eq!(x, None);

        let ct = CheckType::Nothing;
        let x = ct.calc_information_gain();
        assert_eq!(x, None);

        let ct = CheckType::Sold;
        let x = ct.calc_information_gain();
        assert_eq!(x, None);
    }

    #[test]
    fn calc_expected_value_simple() {
        let ct = CheckType::Lights(
            2,
            vec![(1, 1), (2, 1)].into_iter().collect::<BTreeMap<_, _>>(),
        );
        let x = ct.calc_expected_value();
        // uniform counts: sum p*log2(p) = 0.5*log2(0.5)+0.5*log2(0.5) = -1.0, function returns -expected = 1.0
        assert_eq!(x, Some(1.0));

        let ct = CheckType::Lights(
            2,
            vec![(1, 3), (2, 1)].into_iter().collect::<BTreeMap<_, _>>(),
        );
        let x = round2(ct.calc_expected_value().unwrap());
        // expected = 3/4*log2(3/4) + 1/4*log2(1/4) = -0.311278 + (-0.5) = -0.811278 -> function returns -(-0.811278)=0.81
        assert_eq!(x, 0.81);

        let ct = CheckType::Eq;
        let x = ct.calc_expected_value();
        assert_eq!(x, None);

        let ct = CheckType::Nothing;
        let x = ct.calc_expected_value();
        assert_eq!(x, None);

        let ct = CheckType::Sold;
        let x = ct.calc_expected_value();
        assert_eq!(x, None);
    }
}

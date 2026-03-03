// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides the functionality related to the constraint which is needed during the
//! simulation. In the process statistics are stored/gathered, but the evaluation is the job of
//! another module(s).

use anyhow::Result;

use crate::constraint::ConstraintSim;
use crate::constraint::{CheckType, Constraint};
use crate::matching_repr::{bitset::Bitset, MaskedMatching};

impl ConstraintSim for Constraint {
    /// Process a matching `m` and apply side effects:
    /// - if `m` does not fit the constraint it is recorded as eliminated,
    /// - otherwise `m` may be pushed into `ruleset_data` for later usage,
    /// - if `build_tree` is enabled we collect `left_poss` examples for tree building.
    fn process(&mut self, m: &MaskedMatching) -> Result<bool> {
        // check fits actually has a value and make it immutable
        let fits = self.fits(m) || self.result_unknown;

        if !fits {
            self.eliminate(m);
        } else {
            if self.build_tree && !self.hidden {
                self.left_poss.push(m.clone());
            }
            if let Some(rs_dat) = self.ruleset_data.as_mut() {
                rs_dat.push(m)?;
            }
        }

        Ok(fits)
    }
}

impl Constraint {
    /// Internal predicate: whether `m` would satisfy the constraint's `check`.
    fn fits(&mut self, m: &MaskedMatching) -> bool {
        // first step is to check if the constraint filters out this matching
        match &mut self.check {
            CheckType::Eq => {
                let mask = self
                    .map
                    .iter()
                    .filter(|i| !i.is_empty())
                    .fold(Bitset::empty(), |acc, i| i | acc);
                m.contains_mask(mask)
            }
            CheckType::HintCntMatch(cnt) => {
                // iterator over all the singletons in the map of this constraint
                let mut singles = self.map.iter().filter(|x| x.is_singleton());

                // obtain the one singleton which has to exist
                let individual_b = singles
                    .next()
                    .expect("HintCntMatch's map contains less than a single entry (should have been already checked on parse)");
                // check there are not more singletons (violates the constraint-type)
                assert!(
                    singles.next().is_none(),
                    "HintCntMatch's map contains more than a single entry (should have already been checked on parse)"
                );

                // search for the Bitset which contains the found singleton on which the constraint
                // acts on
                let b = m
                    .iter()
                    .find(|b| b.contains_any(individual_b))
                    .expect("Permutation does not contain the individual from set_b -- something is wrong here");

                // at it's core the constraint checks whether the given singleton only is seen in
                // sets which size equals the given cnt
                b.count() == *cnt
            }
            CheckType::Nothing | CheckType::Sold => true,
            CheckType::Lights(ref lights, ref mut light_count) => {
                let l = self.map.calculate_lights(m);

                // true when exclude exists AND there's any overlap -> deny the matching
                let deny = self
                    .exclude
                    .as_ref()
                    .and_then(|ex| m.slot_mask(ex.0 as usize).map(|m| ex.1.contains_any(*m)))
                    .unwrap_or(false);

                // use calculated lights to collect stats on based on the matching possible until
                // here, how many lights are calculated how often for this map
                *light_count.entry(l).or_insert(0) += 1;

                !deny && l == *lights
            }
        }
    }

    #[cfg(test)]
    pub(super) fn test_eliminate(&mut self, m: &MaskedMatching) {
        self.eliminate(m)
    }

    /// aggregate stats about matching `m` which was eliminated by this constraint
    fn eliminate(&mut self, m: &MaskedMatching) {
        for (k, v) in m.iter_pairs() {
            self.eliminated_tab[k as usize][v as usize] += 1;
        }
        self.eliminated += 1;
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_process_remaining() {
        // should collect
        let mut c = Constraint {
            result_unknown: false,
            build_tree: true,
            hidden: false,
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            check: CheckType::Lights(3, Default::default()),
            eliminated_tab: vec![vec![0; 4]; 3],
            ..Default::default()
        };

        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];

        for (f, m) in &ms {
            let x = c.process(m).unwrap();
            assert_eq!(x, *f);
        }
        assert_eq!(
            c.left_poss,
            ms[0..3].iter().map(|(_, m)| m.clone()).collect::<Vec<_>>()
        );

        // shouldn't collect
        let mut c = Constraint {
            result_unknown: false,
            build_tree: false,
            hidden: false,
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            check: CheckType::Lights(3, Default::default()),
            eliminated_tab: vec![vec![0; 4]; 3],
            ..Default::default()
        };

        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];

        for (f, m) in &ms {
            let x = c.process(m).unwrap();
            assert_eq!(x, *f);
            assert_eq!(c.left_poss.len(), 0);
        }

        // shouldn't collect
        let mut c = Constraint {
            result_unknown: false,
            build_tree: true,
            hidden: true,
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            check: CheckType::Lights(3, Default::default()),
            eliminated_tab: vec![vec![0; 4]; 3],
            ..Default::default()
        };

        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];

        for (f, m) in &ms {
            let x = c.process(m).unwrap();
            assert_eq!(x, *f);
            assert_eq!(c.left_poss.len(), 0);
        }

        // should collect + everything fits as result is unkown
        let mut c = Constraint {
            result_unknown: true,
            build_tree: true,
            hidden: false,
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            check: CheckType::Lights(3, Default::default()),
            eliminated_tab: vec![vec![0; 4]; 3],
            ..Default::default()
        };

        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];

        for (_f, m) in &ms {
            let x = c.process(m).unwrap();
            assert!(x);
        }
        assert_eq!(
            c.left_poss,
            ms[0..].iter().map(|(_, m)| m.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn eliminate_simple() {
        let mut c = Constraint {
            eliminated_tab: vec![vec![0; 4]; 3],
            ..Default::default()
        };
        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];

        for (_f, m) in &ms {
            let old = c.eliminated;
            c.eliminate(m);
            assert_eq!(c.eliminated, old + 1);
        }
        assert_eq!(c.eliminated, ms.len() as u128);
        assert_eq!(
            c.eliminated_tab,
            vec![vec![3, 1, 0, 1], vec![1, 3, 0, 2], vec![0, 0, 4, 1],]
        );
    }

    #[test]
    fn fits_nothing() {
        let mut c = Constraint {
            check: CheckType::Nothing,
            ..Default::default()
        };
        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];
        for (_f, m) in ms {
            assert!(c.fits(&m));
        }
    }

    #[test]
    fn fits_sold() {
        let mut c = Constraint {
            check: CheckType::Sold,
            ..Default::default()
        };
        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];
        for (_f, m) in ms {
            assert!(c.fits(&m));
        }
    }

    #[test]
    fn fits_lights() {
        let mut c = Constraint {
            check: CheckType::Lights(3, Default::default()),
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            ..Default::default()
        };
        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];

        for (f, m) in &ms {
            assert_eq!(c.fits(m), *f);
        }
    }

    #[test]
    fn fits_lights_exclude() {
        let mut c = Constraint {
            check: CheckType::Lights(1, Default::default()),
            map: MaskedMatching::from_matching_ref(&[vec![], vec![1]]),
            // 1 must match with ONLY 1 => exclude matching with 0, 2 or 3
            exclude: Some((1, Bitset::from_idxs(&[0, 3]))),
            ..Default::default()
        };
        let ms = vec![
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            ),
            (
                false,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1, 3], vec![2]]),
            ),
            (
                true,
                MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2, 3]]),
            ),
            (
                false, // passes the exclude check, but 1->1 does not hold anymore => 0 lights expected
                MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0, 3]]),
            ),
        ];

        for (f, m) in &ms {
            assert_eq!(c.fits(m), *f);
        }
    }
}

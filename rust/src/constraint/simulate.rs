/// This module provides the functionality related to the constraint which is needed during the
/// simulation. In the process statistics are stored/gathered, but the evaluation is the job of
/// another module(s).
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
            if !self.hide_ruleset_data && !self.hidden {
                self.ruleset_data.push(m)?;
            }
        }

        Ok(fits)
    }
}

impl Constraint {
    /// Internal predicate: whether `m` would satisfy the constraint's `check`.
    ///
    /// This function is intentionally small and pure except for reading `self` state.
    /// It is used by `process()` (which handles side effects like elimination and
    /// pushing to `ruleset_data`).
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
            CheckType::Nothing | CheckType::Sold => true,
            CheckType::Lights(ref lights, ref mut light_count) => {
                let l = self.map.calculate_lights(m);

                let f = self
                    .exclude
                    .as_ref()
                    .and_then(|ex| m.slot_mask(ex.0 as usize).map(|m| !m.contains_any(ex.1)));

                // use calculated lights to collect stats on based on the matching possible until
                // here, how many lights are calculated how often for this map
                *light_count.entry(l).or_insert(0) += 1;

                if let Some(f) = f {
                    f
                } else {
                    l == *lights
                }
            }
        }
    }

    #[cfg(test)]
    pub(super) fn test_eliminate(&mut self, m: &MaskedMatching) {
        self.eliminate(m)
    }
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
            exclude: Some((1, Bitset::from_idxs(&[0, 2, 3]))),
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
                false,
                MaskedMatching::from_matching_ref(&[vec![1], vec![0, 3], vec![2]]),
            ),
        ];

        for (f, m) in &ms {
            assert_eq!(c.fits(m), *f);
        }
    }
}

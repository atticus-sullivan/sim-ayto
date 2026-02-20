/// This module provides the functionality related to the constraint which is needed during the
/// simulation. In the process statistics are stored/gathered, but the evaluation is the job of
/// another module(s).

use anyhow::Result;

use crate::constraint::{CheckType, Constraint};
use crate::matching_repr::{bitset::Bitset, MaskedMatching};

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
                    .and_then(|ex| m.slot_mask(ex.0 as usize).map(|m| !m.contains_any(&ex.1)));

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

    /// Process a matching `m` and apply side effects:
    /// - if `m` does not fit the constraint it is recorded as eliminated,
    /// - otherwise `m` may be pushed into `ruleset_data` for later usage,
    /// - if `build_tree` is enabled we collect `left_poss` examples for tree building.
    pub fn process(&mut self, m: &MaskedMatching) -> Result<bool> {
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
mod tests {
    use rust_decimal::dec;

    use crate::{constraint::ConstraintType, ruleset_data::dummy::DummyData};

    use super::*;

    #[test]
    fn test_process_remaining() {
    }

    #[test]
    fn test_process_rs_data() {
        // TODO: mock ruleset_data
    }

    #[test]
    fn test_process_light() {
        let mut c = constraint_def(None, 2);

        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert!(!c.process(&m).unwrap());
        // change amount of lights
        match &mut c.check {
            CheckType::Eq => {}
            CheckType::Nothing | CheckType::Sold => {}
            CheckType::Lights(l, _) => *l = 1,
        }
        assert!(c.process(&m).unwrap());
    }

    #[test]
    fn test_process_light_exclude() {
        let mut c = constraint_def(Some((0, Bitset::from_idxs(&[2, 3]))), 1);

        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert!(c.process(&m).unwrap());

        let m = MaskedMatching::from_matching_ref(&[vec![0, 2], vec![1], vec![4], vec![3]]);
        assert!(!c.process(&m).unwrap());
    }

    #[test]
    fn test_process_eq() {
        let mut c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Eq,
            // 1 and 2 have the same match
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2]]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: String::from(""),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert!(!c.process(&m).unwrap());
        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1, 2], vec![3], vec![4]]);
        assert!(c.process(&m).unwrap());
    }

    #[test]
    fn test_eliminate() {
        let mut c = constraint_def(None, 2);
        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);

        c.eliminate(&m);
        assert_eq!(c.eliminated, 1);
        assert_eq!(
            c.eliminated_tab,
            vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 0, 0],
                vec![0, 0, 1, 0, 0],
                vec![0, 0, 0, 1, 1]
            ]
        );

        c.eliminate(&m);
        assert_eq!(c.eliminated, 2);
        assert_eq!(
            c.eliminated_tab,
            vec![
                vec![2, 0, 0, 0, 0],
                vec![0, 2, 0, 0, 0],
                vec![0, 0, 2, 0, 0],
                vec![0, 0, 0, 2, 2]
            ]
        );
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn fits_eq_mask_behavior() {
        // Construct constraint that checks Eq with mapping 1->1 (two items)
        let mut c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Eq,
            // map: 1->1, 2->2 style (we only need a map that produces a non-empty mask)
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![2]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 5]; 4],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: String::from(""),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };

        // Matching `m` that *does not* include all required mask bits -> fits() should be false
        let m1 = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert_eq!(c.fits(&m1), false);

        // Matching `m` that includes the required mask -> fits() true
        // second matching has same structure but element 1 has options [1,2] so contains mask
        let m2 = MaskedMatching::from_matching_ref(&[vec![0], vec![1, 2], vec![3], vec![4]]);
        assert_eq!(c.fits(&m2), true);
    }

    #[test]
    fn fits_nothing_and_sold_are_always_true() {
        let mut c1 = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Nothing,
            map: MaskedMatching::from_matching_ref(&[vec![0]]),
            eliminated: 0,
            eliminated_tab: vec![vec![0; 1]; 1],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: dec![1.0],
                comment: String::new(),
                offer: None,
            },
            build_tree: false,
            left_poss: vec![],
            hide_ruleset_data: false,
            ruleset_data: Box::new(DummyData::default()),
            known_lights: 0,
        };
        let m = MaskedMatching::from_matching_ref(&[vec![0]]);
        assert!(c1.fits(&m));

        let mut c2 = c1.clone();
        c2.check = CheckType::Sold;
        assert!(c2.fits(&m));
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn fits_lights_updates_histogram_and_matches_count() {
        // use constraint_def helper to create Lights constraint (default known settings)
        let mut c = constraint_def(None, 2); // expects 2 lights

        // create a matching that yields 0 lights (based on constraint_def's internal map)
        let m_no = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        // histogram should start empty
        match &c.check {
            CheckType::Lights(_, hist) => assert!(hist.is_empty()),
            _ => panic!("expected lights"),
        }

        // call fits: expected false, but histogram will be updated for computed lights (some l)
        let res = c.fits(&m_no);
        assert_eq!(res, false);

        // After call, histogram should have one entry for the computed lights value
        match &c.check {
            CheckType::Lights(_, hist) => {
                // ensure an entry exists for the computed lights count
                assert!(hist.values().sum::<u128>() >= 1);
                // The histogram key matches the stored lights value `l` (we can't know exact l here,
                // but ensure there's a key present)
                assert!(!hist.is_empty());
            }
            _ => panic!("expected lights"),
        }

        // Now change the expected lights inside the constraint to something matching the computed `l`
        // We can inspect the histogram key to set the needed value to force success
        let computed_key = match &c.check {
            CheckType::Lights(_, hist) => *hist.keys().next().unwrap(),
            _ => unreachable!(),
        };
        // mutate check so the required lights matches computed_key
        c.check = CheckType::Lights(computed_key, BTreeMap::new());
        assert!(c.fits(&m_no));
    }

    #[test]
    fn fits_lights_with_exclude_respects_exclusion_mask() {
        // create a constraint that expects 1 light and has an exclude on slot 0 (positions masked)
        let exclude_bs = Bitset::from_idxs(&[2, 3]); // exclude values indices 2,3
        let mut c = constraint_def(Some((0, exclude_bs)), 1);

        // matching m1: slot0 does not include any excluded index -> should pass (fits true)
        let m1 = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);
        assert!(c.fits(&m1));

        // matching m2: slot0 contains an excluded index -> should fail (fits false)
        let m2 = MaskedMatching::from_matching_ref(&[vec![0, 2], vec![1], vec![4], vec![3]]);
        assert!(!c.fits(&m2));
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn process_respects_result_unknown_and_does_not_eliminate() {
        // construct a Lights constraint that would normally reject a matching; set result_unknown
        let c = constraint_def(None, 2);
        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);

        // For baseline: process without result_unknown eliminates (returns false)
        let mut c2 = c.clone();
        let ok = c2.process(&m).unwrap();
        assert_eq!(ok, false);
        assert_eq!(c2.eliminated, 1);

        // Now set result_unknown true on a fresh constraint -> process should return true and not eliminate
        let mut c3 = constraint_def(None, 2);
        c3.result_unknown = true;
        let ok2 = c3.process(&m).unwrap();
        assert_eq!(ok2, true);
        // no elimination performed in this case
        assert_eq!(c3.eliminated, 0);
    }
}

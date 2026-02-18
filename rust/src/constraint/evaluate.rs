/// This module contains functions to be used when evaluating after the simulation completed.
///
/// Note: There is also evaluate_predicates which contains functions serving as predicates during
/// the evaluation.

use crate::{constraint::{Constraint, ConstraintType, Offer}, Rem};

use anyhow::{bail, ensure, Result};

impl Constraint {
    /// Return whether the game was solvable *before* applying this constraint.
    ///
    /// - Returns Ok(Some(true)) if definitely solvable,
    /// - Ok(Some(false)) if definitely unsolvable,
    /// - Ok(None) if the constraint does not express solvability information.
    pub fn is_solvable_after(&self) -> Result<Option<bool>> {
        // not all constraints capture the remaining possibilities
        if self.left_poss.is_empty() {
            return Ok(None);
        }

        // choose one solution to be the prototype for the partial solution
        let mut sol = self.left_poss[0].clone();

        // overlay all other possible solutions to check if there is a common partial solution
        for i in &self.left_poss[1..] {
            if i.len() != sol.len() {
                // println!("length check failed");
                bail!("inequal length between the solutions");
            }
            if (i.calculate_lights(&sol) as usize) < sol.len() {
                return Ok(Some(false));
            }
            sol = sol & i;
        }
        Ok(Some(true))
    }

    pub fn should_merge(&self) -> bool {
        self.hidden
    }

    pub fn merge(&mut self, other: &Self) -> Result<()> {
        self.eliminated += other.eliminated;
        ensure!(
            self.eliminated_tab.len() == other.eliminated_tab.len(),
            "eliminated_tab lengths do not match (self: {}, other: {})",
            self.eliminated_tab.len(),
            other.eliminated_tab.len()
        );
        for (i, es) in self.eliminated_tab.iter_mut().enumerate() {
            ensure!(
                es.len() == other.eliminated_tab[i].len(),
                "eliminated_tab lengths do not match (self: {}, other: {})",
                es.len(),
                other.eliminated_tab[i].len()
            );
            for (j, e) in es.iter_mut().enumerate() {
                *e += other.eliminated_tab[i][j];
            }
        }
        self.information = None;
        self.left_after = None;
        Ok(())
    }

    pub fn apply_to_rem(&mut self, mut rem: Rem) -> Option<Rem> {
        rem.1 -= self.eliminated;

        for (i, rs) in rem.0.iter_mut().enumerate() {
            for (j, r) in rs.iter_mut().enumerate() {
                *r -= self.eliminated_tab.get(i)?.get(j)?;
            }
        }

        self.left_after = Some(rem.1);

        let tmp = 1.0 - (self.eliminated as f64) / (rem.1 + self.eliminated) as f64;
        self.information = if tmp == 1.0 {
            Some(0.0)
        } else if tmp > 0.0 {
            Some(-tmp.log2())
        } else {
            None
        };

        Some(rem)
    }

    pub fn try_get_offer(&self) -> Option<Offer> {
        if let ConstraintType::Box { offer, .. } = &self.r#type {
            offer.clone()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use crate::constraint::CheckType;
    use crate::matching_repr::MaskedMatching;

    // TODO: remove
    // fn constraint_def(exclude: Option<(u8, Bitset)>, lights: u8) -> Constraint {
    //     Constraint {
    //         result_unknown: false,
    //         exclude,
    //         map_s: HashMap::new(),
    //         check: CheckType::Lights(lights, BTreeMap::new()),
    //         map: MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]),
    //         eliminated: 0,
    //         eliminated_tab: vec![
    //             vec![0, 0, 0, 0, 0],
    //             vec![0, 0, 0, 0, 0],
    //             vec![0, 0, 0, 0, 0],
    //             vec![0, 0, 0, 0, 0],
    //         ],
    //         information: None,
    //         left_after: None,
    //         hidden: false,
    //         r#type: ConstraintType::Night {
    //             num: dec![1.0],
    //             comment: String::from(""),
    //             offer: None,
    //         },
    //         build_tree: false,
    //         left_poss: vec![],
    //         hide_ruleset_data: false,
    //         ruleset_data: Box::new(DummyData::default()),
    //         known_lights: 0,
    //     }
    // }

    #[test]
    #[allow(clippy::identity_op)]
    fn apply_to_rem_simple() {
        let mut c = Constraint::default();
        c.eliminated_tab = vec![
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0],
        ];
        c.map = MaskedMatching::from_matching_ref(&[vec![1], vec![2], vec![0], vec![3]]);
        c.check = CheckType::Lights(2, BTreeMap::new());
        c.eliminated = 0;

        let m = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2], vec![3, 4]]);

        c.test_eliminate(&m);
        assert_eq!(c.eliminated, 1);

        let mut rem: Rem = (vec![vec![15; 5]; 4], 5 * 4 * 3 * 2 * 1 * 4 / 2);

        rem = c.apply_to_rem(rem).unwrap();
        assert_eq!(rem.1, 5 * 4 * 3 * 2 * 1 * 4 / 2 - 1);
        assert_eq!(
            rem.0,
            vec![
                vec![14, 15, 15, 15, 15],
                vec![15, 14, 15, 15, 15],
                vec![15, 15, 14, 15, 15],
                vec![15, 15, 15, 14, 14]
            ]
        );
    }

    #[test]
    fn merge_simple() {
        let mut c_a = Constraint::default();
        c_a.eliminated_tab = vec![
            vec![1, 0, 0, 0, 0],
            vec![0, 1, 0, 3, 0],
            vec![0, 0, 2, 0, 3],
            vec![0, 6, 0, 5, 0],
        ];
        c_a.eliminated = 200;
        c_a.information = Some(4.5);
        c_a.left_after = Some(10);

        let mut c_b = Constraint::default();
        c_b.eliminated_tab = vec![
            vec![1, 0, 0, 0, 0],
            vec![0, 1, 0, 3, 0],
            vec![0, 0, 2, 0, 3],
            vec![0, 6, 0, 5, 0],
        ];
        c_b.eliminated = 100;
        c_b.information = Some(3.5);
        c_b.left_after = None;

        c_a.merge(&c_b).unwrap();

        assert_eq!(c_a.eliminated, 300);

        assert_eq!(
            c_a.eliminated_tab,
            vec![
                vec![2, 0, 0, 0, 0],
                vec![0, 2, 0, 6, 0],
                vec![0, 0, 4, 0, 6],
                vec![0, 12, 0, 10, 0],
            ]
        );

        assert_eq!(c_a.information, None);
        assert_eq!(c_a.left_after, None);
    }

    #[test]
    fn is_solvable_after_simple() {
        // left possibilities not captured -> cannot tell => none
        let mut c = Constraint::default();
        c.left_poss = vec![];
        assert!(c.is_solvable_after().unwrap().is_none());

        // only one possibility left => definitely solvable
        let mut c = Constraint::default();
        c.left_poss = vec![MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]])];
        assert!(c.is_solvable_after().unwrap().unwrap());

        // multiple total solutions left, but there is one unambiguous partial working solution which applies to
        // all solutions left
        let mut c = Constraint::default();
        c.left_poss = vec![
            MaskedMatching::from_matching_ref(&[vec![0],    vec![1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![0],    vec![1], vec![2, 3]]),
        ];
        assert!(c.is_solvable_after().unwrap().unwrap());

        // multiple total solutions left, also not one unambiguous partial solution existing
        let mut c = Constraint::default();
        c.left_poss = vec![
            MaskedMatching::from_matching_ref(&[vec![0],    vec![1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![0, 3], vec![1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![3],    vec![1], vec![2, 0]]),
        ];
        assert!(!c.is_solvable_after().unwrap().unwrap());
    }
}

use crate::{constraint::Constraint, matching_repr::MaskedMatching, Rem};

use anyhow::{bail, ensure, Result};

impl Constraint {
    /// Return whether the game was solvable *before* applying this constraint.
    ///
    /// - Returns Ok(Some(true)) if definitely solvable,
    /// - Ok(Some(false)) if definitely unsolvable,
    /// - Ok(None) if the constraint does not express solvability information.
    pub fn was_solvable_before(&self) -> Result<Option<bool>> {
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

    pub(super) fn eliminate(&mut self, m: &MaskedMatching) {
        for (k, v) in m.iter_pairs() {
            self.eliminated_tab[k as usize][v as usize] += 1;
        }
        self.eliminated += 1;
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
}

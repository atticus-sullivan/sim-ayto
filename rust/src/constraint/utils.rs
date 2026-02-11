use crate::constraint::{CheckType, Constraint, ConstraintType};
use crate::Rem;

use anyhow::{ensure, Result, bail};

// internal helper functions
impl Constraint {
    pub(super) fn show_lights_information(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => true,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing | CheckType::Sold => false,
        }
    }

    pub(super) fn show_expected_information(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => true,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing | CheckType::Sold => false,
        }
    }

    pub(super) fn show_past_cnt(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => false,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing | CheckType::Sold => false,
        }
    }

    pub(super) fn show_new(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => false,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing | CheckType::Sold => false,
        }
    }

    pub(super) fn show_past_dist(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => false,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing | CheckType::Sold => false,
        }
    }

    pub(super) fn adds_new(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => true,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing | CheckType::Sold => false,
        }
    }

    pub(super) fn eliminate(&mut self, m: &[Vec<u8>]) {
        for (i1, v) in m.iter().enumerate() {
            for &i2 in v {
                if i2 == u8::MAX {
                    continue;
                }
                self.eliminated_tab[i1][i2 as usize] += 1
            }
        }
        self.eliminated += 1;
    }

    pub fn md_title(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, comment, .. } => format!(
                "MN#{:02.1} {}",
                num,
                comment.split("--").collect::<Vec<_>>()[0]
            ),
            ConstraintType::Box { num, comment, .. } => format!(
                "MB#{:02.1} {}",
                num,
                comment.split("--").collect::<Vec<_>>()[0]
            ),
        }
    }
}

// helpers for evaluation
impl Constraint {
    pub fn was_solvable_before(&self) -> Result<Option<bool>> {
        // not all constraints capture the remaining possibilities
        if self.left_poss.is_empty() {
            return Ok(None);
        }

        // choose one solution to be the prototype for the partial solution
        let mut sol = self.left_poss[0].clone();
        // println!("sol: {:?}", sol);
        // println!("other left: {:?}", &self.left_poss[1..]);

        // overlay all other possible solutions to check if there is a common partial solution
        for i in &self.left_poss[1..] {
            if i.len() != sol.len() {
                // println!("length check failed");
                bail!("inequal length between the solutions");
            }
            for (a,bs) in i.iter().enumerate() {
                // with the length check above this unchecked indexing is sane
                let b_partial = &mut sol[a];
                b_partial.retain(|b| bs.contains(b));
                if b_partial.is_empty() {
                    // println!("partial empty");
                    return Ok(Some(false))
                }
            }
        }
        // println!("sol left: {:?}", &sol);
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
}

use crate::constraint::{CheckType, Constraint, ConstraintType};
use crate::Matching;
use crate::Rem;

use anyhow::{ensure, Result};

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
            CheckType::Nothing => false,
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
            CheckType::Nothing => false,
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
            CheckType::Nothing => false,
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
            CheckType::Nothing => false,
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
            CheckType::Nothing => false,
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
            CheckType::Nothing => false,
        }
    }

    pub(super) fn eliminate(&mut self, m: &Matching) {
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
}

// helpers for evaluation
impl Constraint {
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

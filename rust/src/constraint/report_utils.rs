use crate::constraint::{CheckType, Constraint, ConstraintType};

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
}

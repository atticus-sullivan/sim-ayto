// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides some simple predicates for the use of generating a report. They all decide
//! whether some kind of information shall be reported/shown or not

use crate::constraint::{CheckType, Constraint, ConstraintType};

// internal helper functions
impl Constraint {
    /// whether to show information about the probability distribution aka information gain
    /// produced by how many lights based on the matching in this constraint
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

    /// whether to show the expected information gain for this constraint
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

    /// whether to show how often a 1:1 matching occured in the past
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

    /// whether to show if there are new 1:1 matches in this constraint
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

    /// whether to show the distance to the previous constraints for this one
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

    /// whether the constraints adds *new* 1:1 matches
    ///
    /// currently we track whether a 1:1 match in a matching-night is *new*
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

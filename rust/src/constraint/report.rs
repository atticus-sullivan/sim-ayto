/// This module is concerned with building reports. This is coupled with evaluation and comparison.
/// But in contrast to these two concerns, this mostly covers presenting and visualizing the data.
/// So not much raw processing takes place here.
///
/// Note:
/// Some larger functionality has been factored out to individual modules:
/// - generating a row for the summary table
///
/// Also some predicates used for generating the reports has been moved to a new module:
/// - report_predicates
use anyhow::Result;
use std::{fs::File, path::PathBuf};

use crate::constraint::{Constraint, ConstraintType};
use crate::matching_repr::bitset::Bitset;
use crate::tree::{dot_tree, tree_ordering};

impl Constraint {
    /// Write a `.dot` file for the tree of remaining possibilities in case this has been
    /// requested. If no tree is requested for this constraint, this function is a no-op.
    ///
    /// Returns whether a tree has been generated/drawn
    pub fn build_tree(&self, path: PathBuf, map_a: &[String], map_b: &[String]) -> Result<bool> {
        if !self.build_tree {
            return Ok(false);
        }

        // calculate the order in which the layers shall be shown
        let ordering = tree_ordering(&self.left_poss, map_a);
        // delegate drawing the tree to a dedicated module
        dot_tree(
            &self.left_poss,
            &ordering,
            &(self.type_str() + " / " + self.comment()),
            &mut File::create(path)?,
            map_a,
            map_b,
        )?;
        Ok(true)
    }

    pub fn distance(&self, other: &Constraint) -> Option<usize> {
        if !self.show_past_dist() || !other.show_past_dist() {
            return None;
        }
        if self.map.len() != other.map.len() {
            return None;
        }

        Some(
            self.map
                .iter()
                .enumerate()
                .filter(|&(k, v)| {
                    !v.is_empty()
                        && !other
                            .map
                            .slot_mask(k)
                            .unwrap_or(&Bitset::empty())
                            .contains_any(&v)
                })
                .count(),
        )
    }

    pub fn show_rem_table(&self) -> bool {
        !self.result_unknown
    }

    pub fn md_heading(&self) -> String {
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

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use rust_decimal::dec;

    use crate::{constraint::check_type::CheckType, matching_repr::MaskedMatching};

    use super::*;

    #[test]
    fn distance_simple() {
        let mut c = Constraint::default();
        c.r#type = ConstraintType::Night {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.check = CheckType::Sold;
        // show_past_dist: false

        let mut other = Constraint::default();
        other.r#type = ConstraintType::Night {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        other.check = CheckType::Lights(1, Default::default());
        // show_past_dist: true

        let x = c.distance(&other);
        assert_eq!(x, None);

        let mut c = Constraint::default();
        c.r#type = ConstraintType::Night {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.check = CheckType::Lights(1, Default::default());
        // show_past_dist: true
        c.map = MaskedMatching::from_matching_ref(&[vec![0]]);

        let mut other = Constraint::default();
        other.r#type = ConstraintType::Night {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        other.check = CheckType::Lights(1, Default::default());
        // show_past_dist: true
        other.map = MaskedMatching::from_matching_ref(&[vec![0], vec![1]]);

        let x = c.distance(&other);
        // maps have different length
        assert_eq!(x, None);

        let mut c = Constraint::default();
        c.r#type = ConstraintType::Night {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        c.check = CheckType::Lights(1, Default::default());
        // show_past_dist: true
        c.map = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]);

        let mut other = Constraint::default();
        other.r#type = ConstraintType::Night {
            num: dec![1.0],
            comment: "".to_string(),
            offer: None,
        };
        other.check = CheckType::Lights(1, Default::default());
        // show_past_dist: true
        other.map = MaskedMatching::from_matching_ref(&[vec![2], vec![1], vec![0]]);

        let x = c.distance(&other);
        assert_eq!(x, Some(2));
    }

    #[test]
    fn show_rem_table_simple() {
        let mut c = Constraint::default();
        c.result_unknown = true;
        assert!(!c.show_rem_table());

        let mut c = Constraint::default();
        c.result_unknown = false;
        assert!(c.show_rem_table());
    }

    #[test]
    fn md_heading_simple() {
        let mut c = Constraint::default();
        c.r#type = ConstraintType::Night {
            num: dec![1.1],
            comment: "Test Night--extra".to_string(),
            offer: None,
        };
        let x = c.md_heading();
        assert_eq!(x, "MN#01.1 Test Night");

        let mut c = Constraint::default();
        c.r#type = ConstraintType::Box {
            num: dec![2],
            comment: "Box Comment--extra".to_string(),
            offer: None,
        };
        let x = c.md_heading();
        assert_eq!(x, "MB#02.0 Box Comment");
    }
}

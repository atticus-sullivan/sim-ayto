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

use anyhow::{Context, Result};
use std::{fs::File, path::PathBuf};

use comfy_table::{presets::NOTHING, Row, Table};

use crate::constraint::{CheckType, Constraint, ConstraintType};
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

#[cfg(test)]
mod test {
}

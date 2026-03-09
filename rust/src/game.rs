// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module represents the whole game.
//! A game has te following lifecycle:
//! 1. parsed from yaml as [`parse::GameParse`] -> parse module
//! 2. converted to a regular [`Game`] -> parse module
//! 3. simulated [`Game::sim`] -> main module
//! 4. evaluated [`Game::eval`] -> eval module
//! 5. report generated and printed `Game::report` -> eval/report module

pub mod cache;
pub mod cache_report;
pub mod parse;
pub mod parse_utils;

mod compare;
mod eval;
mod eval_utils;
mod md_output;
mod query_matchings;
mod query_pairs;
mod report_summary;
mod report_trail;
mod report_utils;

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;

use crate::constraint::Constraint;
use crate::dump_mode::DumpMode;
use crate::iterstate::IterState;
use crate::matching_repr::{IdBase, MaskedMatching};
use crate::progressbar::ProgressBarTrait;
use crate::ruleset::RuleSet;
use crate::Lut;

/// a struct to represent a complete game.
#[derive(Debug)]
pub struct Game {
    /// whether offers are noted in this game
    no_offerings_noted: bool,
    /// whether the remaining possible solutions should be collected
    keep_rem: bool,
    /// the constraints originally parsed from file (these will stay constant and won't be
    /// mutated during the simulation)
    pub constraints_orig: Vec<Constraint>,
    /// the ruleset which is to be applied to this game
    pub rule_set: RuleSet,
    /// frontmatter to set in the generated markdown output
    frontmatter: serde_yaml::Value,

    /// map IdBase from set_a to names
    map_a: Vec<String>,
    /// map IdBase from set_b to names
    map_b: Vec<String>,

    /// map names from set_a to IdBase
    lut_a: Lut,
    /// map names from set_b to IdBase
    lut_b: Lut,

    /// where to place output files (.json, .dot, .md)
    dir: PathBuf,
    /// the stem for the output file-names (.json, .dot, .md)
    stem: String,
    /// query these full matchings and when the were eliminated in the process (if so)
    query_matchings: Vec<MaskedMatching>,
    /// query these individuals from set_a and set_b regarding how often they occur with which
    /// other individuals from the other set
    query_pair: (HashSet<IdBase>, HashSet<IdBase>),

    /// *read* the cache from this file if set
    cache_file: Option<PathBuf>,
    /// *write* cache to this path if set
    cache_to: Option<PathBuf>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            no_offerings_noted: false,
            keep_rem: false,
            constraints_orig: vec![],
            rule_set: RuleSet::Eq,
            frontmatter: Default::default(),
            map_a: vec![],
            map_b: vec![],
            lut_a: Default::default(),
            lut_b: Default::default(),
            dir: Default::default(),
            stem: "abc".to_string(),
            query_matchings: vec![],
            query_pair: (Default::default(), Default::default()),
            cache_file: None,
            cache_to: None,
        }
    }
}

impl Game {
    /// Return a (translation-key, short-code) describing the ruleset.
    pub(super) fn ruleset_str(&self) -> (String, String) {
        match &self.rule_set {
            RuleSet::XTimesDup((cnt, fixed)) => (
                format!("rs-XTimesDup-{}-{}", fixed.len(), cnt),
                format!("?{cnt}={}", fixed.len()),
            ),
            RuleSet::SomeoneIsTrip => ("rs-SomeoneIsTrip".to_string(), "?3".to_string()),
            RuleSet::NToN => ("rs-NToN".to_string(), "N:N".to_string()),
            RuleSet::FixedTrip(_) => ("rs-FixedTrip".to_string(), "=3".to_string()),
            RuleSet::Eq => ("rs-Eq".to_string(), "=".to_string()),
        }
    }

    /// Return a formatted number-of-players string "A/B".
    pub(super) fn players_str(&self) -> String {
        format!("{}/{}", self.map_a.len(), self.map_b.len())
    }

    /// Run the simulation (populate an [`crate::iterstate::IterState`] by iterating ruleset permutations).
    ///
    /// by setting `dump_mode` the permutations which survived all constraints are stored for later
    /// evaluation/dumping
    ///
    /// Returns the final [`crate::iterstate::IterState`].
    pub fn sim<T: ProgressBarTrait>(
        &mut self,
        dump_mode: Option<DumpMode>,
    ) -> Result<IterState<T, Constraint>> {
        let mut is = {
            // mathematically calculate amount of permutations (for the progressbar)
            let perm_amount = self.rule_set.get_perms_amount(
                self.map_a.len(),
                self.map_b.len(),
                &self.cache_file,
            )?;

            IterState::new(
                // whether to store the permutations which are valid solutions
                dump_mode.is_some() || self.keep_rem,
                perm_amount,
                self.constraints_orig.clone(),
                // query which constraint eliminated a matching
                &self.query_matchings,
                // query possible matches for person A/B (any how many possible solutions for this)
                &self.query_pair,
                &self.cache_to,
                (self.map_a.len(), self.map_b.len()),
            )?
        };

        // run the entire simulation
        self.rule_set
            .iter_perms(&self.lut_a, &self.lut_b, &mut is, &self.cache_file)?;

        Ok(is)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn ruleset_str_simple() {
        let g = Game {
            rule_set: RuleSet::Eq,
            ..Default::default()
        };
        assert_eq!(g.ruleset_str(), ("rs-Eq".to_string(), "=".to_string()));

        let g = Game {
            rule_set: RuleSet::NToN,
            ..Default::default()
        };
        assert_eq!(g.ruleset_str(), ("rs-NToN".to_string(), "N:N".to_string()));

        let g = Game {
            rule_set: RuleSet::SomeoneIsTrip,
            ..Default::default()
        };
        assert_eq!(
            g.ruleset_str(),
            ("rs-SomeoneIsTrip".to_string(), "?3".to_string())
        );

        let g = Game {
            rule_set: RuleSet::FixedTrip("abc".to_string()),
            ..Default::default()
        };
        assert_eq!(
            g.ruleset_str(),
            ("rs-FixedTrip".to_string(), "=3".to_string())
        );

        let g = Game {
            rule_set: RuleSet::XTimesDup((3, vec!["a".to_string(), "b".to_string()])),
            ..Default::default()
        };
        assert_eq!(
            g.ruleset_str(),
            ("rs-XTimesDup-2-3".to_string(), "?3=2".to_string())
        );
    }

    #[test]
    fn players_str_simple() {
        let g = Game {
            map_a: vec!["a", "b", "c"]
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
            map_b: vec!["a"].into_iter().map(|x| x.to_string()).collect(),
            ..Default::default()
        };
        assert_eq!(g.players_str(), "3/1");
    }
}

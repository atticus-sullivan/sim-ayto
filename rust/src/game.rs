/// This module represents the whole game.
/// A game has te following lifecycle:
/// 1. parsed from yaml as `GameParse` -> parse module
/// 2. converted to a regular `Game` -> parse module
/// 3. simulated `sim()` -> main module
/// 4. evaluated `eval()` -> eval module
/// 5. report generated and printed `report()` -> eval/report module

pub mod cache;
pub mod cache_report;
pub mod parse;

pub(super) mod parse_utils;

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
use crate::matching_repr::MaskedMatching;
use crate::progressbar::ProgressBarTrait;
use crate::ruleset::RuleSet;
use crate::Lut;

#[derive(Debug)]
pub struct Game {
    no_offerings_noted: bool,
    solved: bool,
    constraints_orig: Vec<Constraint>,
    rule_set: RuleSet,
    frontmatter: serde_yaml::Value,

    // maps u8/usize to string
    map_a: Vec<String>,
    map_b: Vec<String>,

    // maps string to usize
    lut_a: Lut,
    lut_b: Lut,

    dir: PathBuf,
    stem: String,
    query_matchings: Vec<MaskedMatching>,
    query_pair: (HashSet<u8>, HashSet<u8>),

    cache_file: Option<PathBuf>,
    cache_to: Option<PathBuf>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            no_offerings_noted: false,
            solved: false,
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
    // returns (translationKeyForExplanation, shortcode)
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

    /// Return a formatted players string "A/B".
    pub(super) fn players_str(&self) -> String {
        format!("{}/{}", self.map_a.len(), self.map_b.len())
    }

    /// Run the simulation (populate an `IterState` by iterating ruleset permutations).
    ///
    /// `dump_mode` controls if permutations are collected
    /// Returns the final `IterState`.
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
                dump_mode.is_some() || self.solved,
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

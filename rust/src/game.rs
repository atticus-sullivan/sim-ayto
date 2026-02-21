mod eval;
mod output;
pub mod parse;
mod query_matchings;
mod query_pairs;
mod report_body;
mod eval_utils;
pub mod dump_mode;
mod compare;
mod report_summary;

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;

use crate::game::dump_mode::DumpMode;
use crate::constraint::Constraint;
use crate::iterstate::IterState;
use crate::matching_repr::MaskedMatching;
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
    final_cache_hash: Option<PathBuf>,
}

impl Game {
    // returns (translationKeyForExplanation, shortcode)
    /// Return a (translation-key, short-code) describing the ruleset.
    pub fn ruleset_str(&self) -> (String, String) {
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
    pub fn players_str(&self) -> String {
        format!("{}/{}", self.map_a.len(), self.map_b.len())
    }

    /// Run the simulation (populate an `IterState` by iterating ruleset permutations).
    ///
    /// `dump_mode` controls if permutations are collected; `use_cache` optionally
    /// instructs using a cache file identity. Returns the final `IterState`.
    pub fn sim(
        &mut self,
        dump_mode: Option<DumpMode>,
        use_cache: Option<String>,
    ) -> Result<IterState> {
        let input_file = if use_cache.is_some() {
            &self.cache_file
        } else {
            &None
        };

        let mut is = {
            // mathematically calculate amount of permutations (for the progressbar)
            let perm_amount =
                self.rule_set
                    .get_perms_amount(self.map_a.len(), self.map_b.len(), input_file)?;

            IterState::new(
                // whether to store the permutations which are valid solutions
                dump_mode.is_some() || self.solved,
                perm_amount,
                self.constraints_orig.clone(),
                // query which constraint eliminated a matching
                &self.query_matchings,
                // query possible matches for person A/B (any how many possible solutions for this)
                &self.query_pair,
                if use_cache.is_some() {
                    &self.final_cache_hash
                } else {
                    &None
                },
                (self.map_a.len(), self.map_b.len()),
            )?
        };

        // run the entire simulation
        self.rule_set
            .iter_perms(&self.lut_a, &self.lut_b, &mut is, true, input_file)?;

        Ok(is)
    }
}

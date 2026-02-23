/// This module is concerned with parsing a game from a config stored as yaml on disk.
/// Based on the data which is deserialized, it allows to construct a ready to use `Game` by using
/// the `finalize_parsing` function.
use std::fs::File;
use std::path::Path;

use serde::Deserialize;

use anyhow::{Context, Result};

use crate::constraint::parse::ConstraintParse;
use crate::game::cache::{CacheMode, CacheModeFallback};
use crate::game::parse_utils::{apply_renames, build_luts, process_constraints};
use crate::game::query_matchings::translate_query_matchings;
use crate::game::query_pairs::translate_query_pairs;
use crate::game::Game;
use crate::ignore_ops::IgnoreOps;
use crate::ruleset::parse::RuleSetParse;
use crate::{Lut, MatchingS, Rename};

#[derive(Deserialize, Debug, Default)]
pub(super) struct QueryPair {
    #[serde(rename = "setA", default)]
    pub(super) map_a: Vec<String>,
    #[serde(rename = "setB", default)]
    pub(super) map_b: Vec<String>,
}

/// Small helper used as a default for the `solved` field during deserialization.
/// We keep this as a free function so it's easily testable.
fn mk_true() -> bool {
    true
}

// this struct is only used for parsing the yaml file
#[derive(Deserialize, Debug)]
pub struct GameParse {
    #[serde(default)]
    no_offerings_noted: bool,
    #[serde(rename = "solved", default = "mk_true")]
    solved: bool,
    #[serde(rename = "constraints")]
    constraints_orig: Vec<ConstraintParse>,
    rule_set: RuleSetParse,
    frontmatter: serde_yaml::Value,
    #[serde(rename = "queryMatchings", default)]
    query_matchings_s: Vec<MatchingS>,
    #[serde(rename = "queryPair", default)]
    query_pair_s: QueryPair,

    #[serde(rename = "setA")]
    map_a: Vec<String>,
    #[serde(rename = "setB")]
    map_b: Vec<String>,

    #[serde(rename = "renameA", default)]
    rename_a: Rename,
    #[serde(rename = "renameB", default)]
    rename_b: Rename,

    // TODO: eventually move this to the constraint, maybe keep here as default
    #[serde(rename = "gen_cache", default)]
    pub gen_cache: bool,

    #[serde(rename = "useCache", default)]
    pub use_cache: Option<CacheMode>,
    #[serde(rename = "cacheFallback", default)]
    pub cache_fallback: Option<CacheModeFallback>,
}

impl GameParse {
    pub fn new_from_yaml(yaml_path: &Path) -> Result<GameParse> {
        let gp: GameParse = serde_yaml::from_reader(File::open(yaml_path)?)?;
        Ok(gp)
    }

    /// Consumes a `GameParse` and produces a fully-initialised `Game`.
    ///
    /// The function performs the following ordered steps:
    /// 1. Constructs lookup tables (`lut_a`, `lut_b`) from `setA`/`setB`.
    /// 2. Validates the lookup tables against the parsed rule set.
    /// 3. Transforms raw `ConstraintParse` objects into concrete `Constraint`s,
    ///    honouring the `ignore` flags and rename tables.
    /// 4. Converts the user-provided query matchings and query pairs into the
    ///    internal `MaskedMatching` representation.
    /// 5. Applies any rename mappings to `map_a`/`map_b` for output purposes.
    ///
    /// Errors from any step are propagated with context, making debugging easier.
    ///
    /// # Arguments
    /// * `stem` - Path to the YAML file (used to derive the game directory and
    ///   stem name).  
    /// * `ignore` - Global `IgnoreOps` that dictate which constraints should be
    ///   silently skipped.
    ///
    /// # Returns
    /// A fully-populated `Game` ready for solving or caching.
    pub fn finalize_parsing(self, stem: &Path, ignore: &IgnoreOps) -> Result<Game> {
        let mut g = Game {
            no_offerings_noted: self.no_offerings_noted,
            solved: self.solved,
            map_a: self.map_a,
            map_b: self.map_b,
            constraints_orig: Vec::default(),
            rule_set: self.rule_set.finalize_parsing(),
            dir: stem
                .parent()
                .context("parent dir of stem not found")?
                .to_path_buf(),
            stem: stem
                .file_stem()
                .context("No filename provided in stem")?
                .to_string_lossy()
                .into_owned(),
            lut_a: Lut::default(),
            lut_b: Lut::default(),
            query_matchings: Vec::default(),
            query_pair: (Default::default(), Default::default()),
            frontmatter: self.frontmatter,
            cache_file: None,
            cache_to: None,
        };

        // build up the look up tables (LUT)
        (g.lut_a, g.lut_b) = build_luts(&g.map_a, &g.map_b)?;

        // validate the lut in combination with the ruleset
        g.rule_set.validate_lut(&g.lut_a, &g.lut_b)?;

        (g.constraints_orig, _) = process_constraints(
            self.constraints_orig,
            ignore,
            &g.lut_a,
            &g.lut_b,
            &g.rule_set,
            &self.rename_a,
            &self.rename_b,
            &g.map_b
        )?;

        // translate the matchings that were querried for tracing
        g.query_matchings = translate_query_matchings(&self.query_matchings_s, &g.lut_a, &g.lut_b)?;

        // translate the pairs that were querried for tracing
        g.query_pair = translate_query_pairs(&self.query_pair_s, &g.lut_a, &g.lut_b)?;

        // rename names in map_a and map_b for output use
        apply_renames(&mut g.map_a, &mut g.map_b, &self.rename_a, &self.rename_b);

        Ok(g)
    }
}

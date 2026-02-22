use std::fs::File;
use std::path::Path;

use serde::Deserialize;

use anyhow::{ensure, Context, Result};

use crate::constraint::parse::ConstraintParse;
use crate::game::cache::{CacheMode, CacheModeFallback};
use crate::game::Game;
use crate::ignore_ops::IgnoreOps;
use crate::ruleset::parse::RuleSetParse;
use crate::{Lut, Matching, MatchingS, Rename};

#[derive(Deserialize, Debug, Default)]
struct QueryPair {
    #[serde(rename = "setA", default)]
    map_a: Vec<String>,
    #[serde(rename = "setB", default)]
    map_b: Vec<String>,
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

    // TODO: split up?
    pub fn finalize_parsing(self, stem: &Path, ignore_boxes: bool) -> Result<Game> {
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
        for (lut, map) in [(&mut g.lut_a, &g.map_a), (&mut g.lut_b, &g.map_b)] {
            for (index, name) in map.iter().enumerate() {
                lut.insert(name.clone(), index);
            }
        }

        ensure!(g.lut_a.len() == g.map_a.len(), "something is wrong with the sets. There might be duplicates in setA (len: {}, dedup len: {}).", g.lut_a.len(), g.map_a.len());
        ensure!(g.lut_b.len() == g.map_b.len(), "something is wrong with the sets. There might be duplicates in setB (len: {}, dedup len: {}).", g.lut_b.len(), g.map_b.len());
        // validate the lut in combination with the ruleset
        g.rule_set.validate_lut(&g.lut_a, &g.lut_b)?;

        // eg translates strings to indices (u8) but also adds the exclude rules if the ruleset demands it as well as sorts if the ruleset needs it
        let mut known_lights: u8 = 0;
        for c in self.constraints_orig {
            if ignore_boxes && c.ignore_on(&IgnoreOps::Boxes) {
                continue;
            }
            let l = c.added_known_lights();
            g.constraints_orig.push(c.finalize_parsing(
                &g.lut_a,
                &g.lut_b,
                g.rule_set.constr_map_len(g.lut_a.len(), g.lut_b.len()),
                &g.map_b,
                g.rule_set.must_add_exclude(),
                g.rule_set.must_sort_constraint(),
                (&self.rename_a, &self.rename_b),
                g.rule_set.init_data()?,
                known_lights,
            )?);
            known_lights += l;
        }

        // translate the matchings that were querried for tracing
        // TODO: directly translate to masked_matching
        for q in &self.query_matchings_s {
            let mut matching: Matching = vec![vec![0]; g.lut_a.len()];
            for (k, v) in q {
                let mut x = v
                    .iter()
                    .map(|v| {
                        g.lut_b
                            .get(v)
                            .map(|v| *v as u8)
                            .with_context(|| format!("{} not found in lut_b", v))
                    })
                    .collect::<Result<Vec<_>>>()?;
                x.sort();
                matching[*g
                    .lut_a
                    .get(k)
                    .with_context(|| format!("{} not found in lut_a", k))?] = x;
            }
            g.query_matchings.push(matching.into());
        }

        // translate the pairs that were querried for tracing
        for a in self.query_pair_s.map_a.iter() {
            let v = g
                .lut_a
                .get(a)
                .map(|v| *v as u8)
                .with_context(|| format!("{} not found in lut_a", a))?;
            g.query_pair.0.insert(v);
        }
        for b in self.query_pair_s.map_b.iter() {
            let v = g
                .lut_b
                .get(b)
                .map(|v| *v as u8)
                .with_context(|| format!("{} not found in lut_b", b))?;
            g.query_pair.1.insert(v);
        }

        // rename names in map_a and map_b for output use
        for (rename, map) in [
            (&self.rename_a, &mut g.map_a),
            (&self.rename_b, &mut g.map_b),
        ] {
            for n in map {
                *n = rename.get(n).unwrap_or(n).to_owned();
            }
        }

        Ok(g)
    }
}

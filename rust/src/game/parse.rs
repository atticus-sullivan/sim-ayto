use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use anyhow::{ensure, Context, Result};

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};

use crate::constraint::parse::ConstraintParse;
use crate::game::Game;
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

    #[serde(rename = "gen_cache", default)]
    gen_cache: bool,

    #[serde(skip_deserializing)]
    found_cache_file: Option<(String, PathBuf)>,
    #[serde(skip_deserializing)]
    final_cache_hash: Option<PathBuf>,
}

// caching
impl GameParse {
    /// Build a human-readable table with information about available caches.
    ///
    /// This function *inspects the filesystem* and returns a `comfy_table::Table`
    /// describing cache existence, line counts and estimated ETA strings.
    pub fn show_caches(&self) -> Result<Table> {
        let hdr = vec![
            Cell::new("What"),
            Cell::new("Cache-file"),
            Cell::new("exists"),
            Cell::new("#left"),
            Cell::new("size [MB]"),
            Cell::new("ETA [m]"),
        ];
        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);

        let caches = self.get_caches();
        for (i, c) in caches.into_iter().enumerate() {
            let c_1 = format!("{:?}", c.1);
            let (exists, left, size, eta) = if Path::new(&c.1).exists() {
                let file = File::open(c.1.clone())?;
                let reader = BufReader::new(file);
                let lines = reader.lines().count();
                // TODO: update this tool got much faster
                let eta = match lines {
                    333_406_530.. => "5:30",
                    48_592_584.. => "1:00",
                    19_079_984.. => "0:30",
                    16_640_896.. => "0:10",
                    1_007_984.. => "0:01",
                    131_330.. => "0:00",
                    _ => "0:00",
                };
                (
                    Cell::new("true").fg(Color::Green),
                    Cell::new(lines),
                    Cell::new(std::fs::metadata(c.1)?.len() / 1_000_000),
                    Cell::new(eta),
                )
            } else {
                (
                    Cell::new("false").fg(Color::Red),
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                )
            };
            if i % 2 == 0 {
                table.add_row(
                    vec![Cell::new(c.0), Cell::new(c_1), exists, left, size, eta]
                        .into_iter()
                        .map(|i| i.bg(crate::COLOR_ALT_BG)),
                );
            } else {
                table.add_row(vec![
                    Cell::new(c.0),
                    Cell::new(c_1),
                    exists,
                    left,
                    size,
                    eta,
                ]);
            }
        }
        Ok(table)
    }

    /// Compute cache file candidates (path + label) for the current `GameParse`.
    ///
    /// This function **does not** access the filesystem: it deterministically
    /// computes a set of hashed cache file paths (useful for `select_cache` and tests).
    pub fn get_caches(&self) -> Vec<(String, PathBuf)> {
        let cache_dir = Path::new("./.cache/");

        // collect hashes for each "layer" of constraints
        let mut input_hashes = vec![];
        let mut prev_hash = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.map_a.hash(&mut hasher);
            self.map_b.hash(&mut hasher);
            hasher.finish()
        };
        for c in self.constraints_orig.iter() {
            // hash c as a new layer to the previous hash
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            prev_hash.hash(&mut hasher);
            if c.has_impact() {
                c.hash(&mut hasher);
                prev_hash = hasher.finish();
                input_hashes.push((
                    c.type_str(),
                    cache_dir
                        .join(format!("{:x}", prev_hash))
                        .with_extension("cache"),
                ));
            }
        }
        input_hashes
    }

    /// Pick the cache file to use (set `found_cache_file` and `final_cache_hash`).
    ///
    /// - If `use_cache` contains a hash id, prefer that (if it is present in `get_caches()`).
    /// - Otherwise, prefer the most recent existing cache file.
    /// - If `gen_cache` is requested, set `final_cache_hash` to the last candidate.
    fn select_cache(gp: &mut GameParse, use_cache: Option<String>) {
        // retrieve the possible caches
        let cache_dir = Path::new("./.cache/");
        let input_hashes = gp.get_caches();

        // externally specified hash/cache-id
        gp.found_cache_file = if let Some(c) = use_cache {
            // convert cache-id to path
            let c = cache_dir.join(c).with_extension("cache");
            // retrieve element from input_hashes
            // this implicitly ensures the externally specified hash/id is actually legit
            input_hashes
                .iter()
                .find(|(_, hash)| *hash == c)
                .map(|(c, hash)| (c.to_string(), hash.clone()))
        } else {
            None
        };

        // try to fall back to most recent hash/cache-id which exists
        if gp.found_cache_file.is_none() {
            gp.found_cache_file = input_hashes.iter().rev().skip(1).find_map(|(c, hash)| {
                if Path::new(hash).exists() {
                    Some((c.to_string(), hash.clone()))
                } else {
                    None
                }
            });
        }

        // retrieve the hash/cache-id to which a new cache shall be written to (if requested)
        gp.final_cache_hash = if gp.gen_cache {
            input_hashes.iter().last().map(|(_, hash)| hash.clone())
        } else {
            None
        };
    }
}

impl GameParse {
    pub fn new_from_yaml(yaml_path: &Path, use_cache: Option<String>) -> Result<GameParse> {
        let mut gp: GameParse = serde_yaml::from_reader(File::open(yaml_path)?)?;
        GameParse::select_cache(&mut gp, use_cache);

        println!("Could use cache file: {:?}", gp.found_cache_file);
        println!("Writing to cache file: {:?}", gp.final_cache_hash);
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
            cache_file: self.found_cache_file.map(|(_, hash)| hash),
            final_cache_hash: self.final_cache_hash,
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
            if ignore_boxes && c.is_box() {
                continue;
            }
            let l = if !c.is_hidden() {
                c.added_known_lights()
            } else {
                0
            };
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

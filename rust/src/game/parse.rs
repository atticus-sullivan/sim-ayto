use serde::Deserialize;
use std::hash::{Hash, Hasher};

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};

use crate::constraint::parse::ConstraintParse;
use crate::ruleset::RuleSetParse;
use crate::{Lut, Matching, MatchingS, Rename};

use crate::game::Game;

#[derive(Deserialize, Debug, Default)]
struct QueryPair {
    #[serde(rename = "setA", default)]
    map_a: Vec<String>,
    #[serde(rename = "setB", default)]
    map_b: Vec<String>,
}

fn mk_true() -> bool {
    true
}

// this struct is only used for parsing the yaml file
#[derive(Deserialize, Debug)]
pub struct GameParse {
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

impl GameParse {
    pub fn show_caches(&self) -> Result<()> {
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
                        .map(|i| i.bg(crate::game::COLOR_ALT_BG)),
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
        println!("{table}");
        Ok(())
    }

    pub fn get_caches(&self) -> Vec<(String, PathBuf)> {
        let cache_dir = Path::new("./.cache/");

        let mut input_hashes = vec![];
        let mut prev_hash = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.map_a.hash(&mut hasher);
            self.map_b.hash(&mut hasher);
            hasher.finish()
        };
        for c in self.constraints_orig.iter() {
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

    pub fn new_from_yaml(yaml_path: &Path, use_cache: Option<String>) -> Result<GameParse> {
        let mut gp: GameParse = serde_yaml::from_reader(File::open(yaml_path)?)?;

        let cache_dir = Path::new("./.cache/");
        let input_hashes = gp.get_caches();
        let uc = use_cache.map(|x| cache_dir.join(x).with_extension("cache"));

        gp.found_cache_file = if let Some(c) = uc {
            input_hashes
                .iter()
                .find(|(_, hash)| *hash == c)
                .map(|(c, hash)| (c.to_string(), hash.clone()))
        } else {
            None
        };

        gp.found_cache_file = gp.found_cache_file.or_else(|| {
            input_hashes.iter().rev().skip(1).find_map(|(c, hash)| {
                if Path::new(hash).exists() {
                    Some((c.to_string(), hash.clone()))
                } else {
                    None
                }
            })
        });
        gp.final_cache_hash = if gp.gen_cache {
            input_hashes.iter().last().map(|(_, hash)| hash.clone())
        } else {
            None
        };
        println!("Could use cache file: {:?}", gp.found_cache_file);
        println!("Writing to cache file: {:?}", gp.final_cache_hash);
        Ok(gp)
    }

    pub fn finalize_parsing(self, stem: &Path) -> Result<Game> {
        let mut g = Game {
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
            let l = c.known_lights();
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
            g.query_matchings.push(matching);
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

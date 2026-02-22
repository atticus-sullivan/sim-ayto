use std::fmt::Display;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};
use serde::Deserialize;

use crate::constraint::{ConstraintImpact, ConstraintGetters};
use crate::game::Game;

/// Build a human-readable table with information about available caches.
///
/// This function *inspects the filesystem* and returns a `comfy_table::Table`
/// describing cache existence, line counts and estimated ETA strings.

/// Compute cache file candidates (path + label) for the current `GameParse`.
///
/// This function **does not** access the filesystem: it deterministically
/// computes a set of hashed cache file paths (useful for `select_cache` and tests).
pub(super) fn get_caches<T: Hash + ConstraintGetters + ConstraintImpact>(initial_hash: u64, constraints: &[T]) -> Caches {
    let cache_dir = Path::new("./.cache/");

    // collect hashes for each "layer" of constraints
    let mut input_hashes = vec![];
    let mut prev_hash = initial_hash;
    for c in constraints.iter() {
        // hash c as a new layer to the previous hash
        let mut hasher = DefaultHasher::new();
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
    Caches(input_hashes)
}

type CacheSpec = (String, PathBuf);
pub struct Caches(Vec<CacheSpec>);

impl Display for Caches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

        for (i, c) in self.0.iter().enumerate() {
            let c_1 = format!("{:?}", c.1);
            let (exists, left, size, eta) = if Path::new(&c.1).exists() {
                let file = File::open(c.1.clone()).map_err(|_| std::fmt::Error)?;
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
                    Cell::new(std::fs::metadata(c.1.clone()).map_err(|_| std::fmt::Error)?.len() / 1_000_000),
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
                    vec![Cell::new(c.0.clone()), Cell::new(c_1), exists, left, size, eta]
                        .into_iter()
                        .map(|i| i.bg(crate::COLOR_ALT_BG)),
                );
            } else {
                table.add_row(vec![
                    Cell::new(c.0.clone()),
                    Cell::new(c_1),
                    exists,
                    left,
                    size,
                    eta,
                ]);
            }
        }
        write!(f, "{table}")
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CacheModeArg {
    MostRecent,
    None,
    SpecificCache,
    SpecificEvent,
}

#[derive(Deserialize, Debug, Clone)]
pub enum CacheMode {
    MostRecent,
    None,
    SpecificCache(PathBuf),
    SpecificEvent(String),
}

impl CacheModeArg {
    pub fn finalize(&self, cache: &Option<PathBuf>, event: &Option<String>) -> Result<CacheMode> {
        match &self {
            CacheModeArg::MostRecent => Ok(CacheMode::MostRecent),
            CacheModeArg::None => Ok(CacheMode::None),
            CacheModeArg::SpecificCache => {
                let Some(cache) = cache else { bail!("Didn't specify which cache to use") };
                Ok(CacheMode::SpecificCache(cache.to_path_buf()))
            },
            CacheModeArg::SpecificEvent => {
                let Some(event) = event else { bail!("Didn't specify which event to use") };
                Ok(CacheMode::SpecificEvent(event.to_string()))
            },
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum, Deserialize)]
pub enum CacheModeFallback {
    MostRecent,
    None,
}

impl CacheModeFallback {
    fn select_cache(&self, caches: &Caches) -> Option<CacheSpec> {
        match &self {
            CacheModeFallback::MostRecent => {
                caches
                    .0
                    .iter()
                    .rev()
                    .skip(1)
                    .filter(|(_, p)| p.exists())
                    .next()
                    .cloned()
            },
            CacheModeFallback::None => None,
        }
    }
}

impl CacheMode {
    pub fn select_cache(&self, fallback: &Option<CacheModeFallback>, caches: &Caches) -> Option<CacheSpec> {
        match &self {
            CacheMode::MostRecent => {
                // re-use same logic like in fallback
                CacheModeFallback::MostRecent.select_cache(caches)
            },
            CacheMode::SpecificCache(path_buf) => {
                caches
                    .0
                    .iter()
                    .find(|(_, p)| p == path_buf)
                    .filter(|(_, p)| p.exists())
                    .cloned()
                    .or_else(|| fallback.as_ref().map(|f| f.select_cache(caches)).flatten())

            },
            CacheMode::SpecificEvent(name) => {
                caches
                    .0
                    .iter()
                    .find(|(n, _)| n == name)
                    .filter(|(_, p)| p.exists())
                    .cloned()
                    .or_else(|| fallback.as_ref().map(|f| f.select_cache(caches)).flatten())
            },
            CacheMode::None => {
                // re-use same logic like in fallback
                CacheModeFallback::None.select_cache(caches)
            },
        }
    }
}

impl Game {
    pub fn get_cache_candidates(&mut self) -> Caches {
        let initial_hash = {
            let mut hasher = DefaultHasher::new();
            self.map_a.hash(&mut hasher);
            self.map_b.hash(&mut hasher);
            hasher.finish()
        };
        get_caches(initial_hash, &self.constraints_orig)
    }

    pub fn select_cache(&mut self, caches: &Caches, mode: CacheMode, fallback: &Option<CacheModeFallback>, output: bool) -> Result<()> {
        let selected = mode.select_cache(fallback, &caches).context("no cache found")?;

        self.cache_file = Some(selected.1);
        if output {
            println!("Selected cache {:?}", self.cache_file);
        }
        Ok(())
    }

    pub fn set_gen_cache(&mut self, caches: &Caches, output: bool) -> Result<()> {
        self.cache_to = caches.0.last().map(|x| x.1.clone());
        if output {
            println!("Write cache to {:?}", self.cache_to);
        }
        Ok(())
    }
}

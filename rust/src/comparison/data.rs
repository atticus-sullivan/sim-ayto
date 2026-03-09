// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains the datastructure to collect all data which can be compared
//!
//! # Testing
//! The function directly serialize from files found on disk. This is very hard to test

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::de::DeserializeOwned;
use walkdir::WalkDir;

use crate::constraint::compare::{ComparisonData, EvalEvent, SumCounts};
use crate::game::parse::GameParse;
use crate::game::Game;
use crate::ignore_ops::IgnoreOps;

/// Compact comparison data for a game used by the comparison pages.
#[derive(Debug)]
pub(crate) struct CmpData {
    /// chronological evaluation events (MB/MN/Initial).
    pub(crate) eval_data: Vec<EvalEvent>,
    /// aggregated counts and summary metrics for the season.
    pub(crate) cnts: SumCounts,
    /// the specification of the game which is compared here
    pub(crate) game: Game,
}

/// Read JSON data from `path.join(fn_param)` and deserialize into `T`.
///
/// Returns `Ok(None)` if the file does not exist. Surface any I/O or
/// deserialization errors via the `Result`.
fn read_json_data<T: DeserializeOwned>(fn_param: &str, path: &Path) -> Result<Option<T>> {
    if !path.join(fn_param).exists() {
        return Ok(None);
    }
    let file = File::open(path.join(fn_param))?;
    let reader = BufReader::new(file);
    let dat: T = serde_json::from_reader(reader)?;
    Ok(Some(dat))
}

/// Parse the game YAML file located at `fn_path` (without extension).
///
/// The function expects `<fn_path>.yaml` to exist and uses the game's parsing
/// facilities ([`crate::game::parse::GameParse`]) to produce a [`crate::game::Game`]. This helper will panic on parse
/// error in current behavior (keeps earlier behavior intact).
fn read_yaml_spec(mut fn_path: PathBuf) -> Result<Game> {
    fn_path.set_extension("yaml");
    let gp = GameParse::new_from_yaml(fn_path.as_path())?;
    gp.finalize_parsing(Path::new("/tmp/"), &IgnoreOps::Nothing)
}

/// Scan `./data` and parse comparison files for the selected directories.
///
/// `filter_dirs` is a callback used to select which subdirectories of `./data`
/// should be included (e.g. `|s| s.starts_with("de")`).
///
/// For each accepted
/// directory the function reads the specification of the game and the stats collected and stored
/// for comparison.
/// It returns a sorted `Vec<(season_name, CmpData)>`
pub(crate) fn gather_cmp_data(filter_dirs: fn(&str) -> bool) -> Result<Vec<(String, CmpData)>> {
    let mut ret = vec![];

    // loop over the data directories selected/filterd by filter_dirs
    for entry in WalkDir::new("./data")
        .max_depth(1)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            e.file_name().to_str().is_some_and(filter_dirs)
                && e.metadata().is_ok_and(|e| e.is_dir())
        })
        .filter_map(Result::ok)
    {
        let game = read_yaml_spec(entry.path().join(entry.file_name()))?;

        let eval_data: ComparisonData = match read_json_data("stats.json", entry.path())? {
            Some(x) => x,
            None => continue,
        };

        ret.push((
            entry.file_name().to_str().unwrap_or("unknown").to_owned(),
            CmpData {
                eval_data: eval_data.events,
                cnts: eval_data.cnts,
                game,
            },
        ));
    }
    ret.sort_by_key(|i| i.0.clone());
    Ok(ret)
}

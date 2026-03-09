// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains the functionality to gather statistics on all eligible caches and nicely
//! show them.

use std::fmt::Display;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::Result;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};

use crate::game::cache::{CachableSpec, CacheSpec};

/// represents all stats collected for a cache
struct CacheStatus<'a> {
    /// the name which is to be attached to this cache
    name: &'a str,
    /// the path to this cache
    path: &'a PathBuf,
    /// whether this cache exists
    exists: bool,
    /// the amount of lines (aka left possible solutions) in the cache
    /// (if the cache does not exist, this obviously can't be set)
    line_count: Option<usize>,
    /// the size of this cache on disk in megabytes if set
    /// (if the cache does not exist, this obviously can't be set)
    size_mb: Option<u64>,
    // eta: Option<String>,
}

impl<'a> CacheStatus<'a> {
    /// collect stats on the `value` which is a cache so this can be displayed later
    fn new<S: CachableSpec>(value: &'a S) -> Result<Self> {
        let exists = value.exists();

        let (line_count, size_mb) = if exists {
            let file = File::open(value.path())?;
            let reader = BufReader::new(file);
            let line_count = reader.lines().count();
            let size_mb = fs::metadata(value.path())?.len() / 1_000_000;

            (Some(line_count), Some(size_mb))
        } else {
            (None, None)
        };

        Ok(Self {
            name: value.event_name(),
            path: value.path(),
            exists: value.exists(),
            line_count,
            size_mb,
        })
    }
}

/// represents the collection of all cache-statuses which can be displayed
struct CacheStatusAll<'a>(Vec<CacheStatus<'a>>);

impl<'a> Display for CacheStatusAll<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hdr = vec![
            Cell::new("What"),
            Cell::new("Cache-file"),
            Cell::new("exists"),
            Cell::new("#left"),
            Cell::new("size [MB]"),
            // Cell::new("ETA [m]"),
        ];
        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);

        for (i, r) in self.0.iter().enumerate() {
            let row = vec![
                Cell::new(r.name),
                Cell::new(format!("{:?}", r.path)),
                Cell::new(r.exists).fg(if r.exists { Color::Green } else { Color::Red }),
                Cell::new(r.line_count.map(|x| x.to_string()).unwrap_or_default()),
                Cell::new(r.size_mb.map(|x| x.to_string()).unwrap_or_default()),
                // eta,
            ];
            if i % 2 == 0 {
                table.add_row(row.into_iter().map(|i| i.bg(crate::COLOR_ALT_BG)));
            } else {
                table.add_row(row);
            }
        }

        write!(f, "{table}")
    }
}

/// Build a human-readable table with information about available caches.
///
/// This function *inspects the filesystem* and returns a [`comfy_table::Table`]
/// describing stats on the caches found.
pub fn show_caches(caches: Vec<CacheSpec>) -> Result<()> {
    let csa = CacheStatusAll(caches.iter().map(CacheStatus::new).collect::<Result<_>>()?);
    println!("{}", csa);
    Ok(())
}

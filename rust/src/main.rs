/*
sim_ayto
Copyright (C) 2024  Lukas Heindl

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

mod comparison;
mod constraint;
mod game;
mod iterstate;
mod ruleset;
mod ruleset_data;
mod tree;

use crate::game::Game;

use clap::{Parser, Subcommand};
use comfy_table::Color;
use game::DumpMode;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

// TODO code review (try with chatGPT)

type MatchingS = HashMap<String, Vec<String>>;
type Matching = Vec<Vec<u8>>;
type MapS = HashMap<String, String>;
type Map = HashMap<u8, u8>;
type Lut = HashMap<String, usize>;
type Rename = HashMap<String, String>;

type Rem = (Vec<Vec<u128>>, u128);

// colors for tables
const COLOR_ROW_MAX: Color = Color::Rgb {
    r: 69,
    g: 76,
    b: 102,
};
const COLOR_BOTH_MAX: Color = Color::Rgb {
    r: 65,
    g: 77,
    b: 71,
};
const COLOR_COL_MAX: Color = Color::Rgb {
    r: 74,
    g: 68,
    b: 89,
};

pub const COLOR_ALT_BG: Color = Color::Rgb {
    r: 41,
    g: 44,
    b: 60,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Sim {
        #[arg(long = "no-tree-output", action)]
        no_tree_output: bool,

        #[arg(long = "ignore-boxes", action)]
        ignore_boxes: bool,

        /// The path to the file to read
        yaml_path: PathBuf,

        #[arg(short = 'c', long = "color")]
        colored: bool,

        #[arg(long = "transpose")]
        transpose_tabs: bool,

        #[arg(short = 'o', long = "output")]
        stem: PathBuf,

        #[arg(
            long = "dump",
            help = "dump all combinations ({winning,all}{nums,names} in the end of the simulation"
        )]
        dump: Option<DumpMode>,

        #[arg(
            long = "full",
            help = "print all probabilities instead of just the topX below the tables"
        )]
        full: bool,

        #[arg(
            long = "use-cache",
            help = "Enable caching. Pass a cache-id to start the simulation from there. Needs to be passed in order to build a cache. To use the optimal cache provide an arbitrary string"
        )]
        use_cache: Option<String>,
    },
    Check {
        /// The path to the file to read
        yaml_path: PathBuf,
    },
    Comparison {
        #[arg(short = 'l', long = "theme-light", default_value = "1")]
        theme_light: u8,
        #[arg(short = 'd', long = "theme-dark", default_value = "3")]
        theme_dark: u8,
        html_path_de: PathBuf,
        html_path_us: PathBuf,
    },
    Cache {
        yaml_path: PathBuf,
    },
}

// TODO: extract templates?
fn main() {
    let args = Cli::parse();

    match args.cmd {
        Commands::Sim {
            no_tree_output,
            ignore_boxes,
            yaml_path,
            colored: _,
            transpose_tabs,
            stem,
            dump,
            full,
            use_cache,
        } => {
            let gp = crate::game::parse::GameParse::new_from_yaml(&yaml_path, use_cache.clone())
                .expect("Parsing failed");
            let mut g = gp
                .finalize_parsing(&stem, ignore_boxes)
                .expect("processing game failed");
            let start = Instant::now();
            let result = g.sim(dump.clone(), use_cache).unwrap();
            g.eval(transpose_tabs, dump, full, &result, no_tree_output)
                .unwrap();
            println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
        }
        Commands::Cache { yaml_path } => {
            let gp = crate::game::parse::GameParse::new_from_yaml(&yaml_path, Some("".to_string()))
                .expect("Parsing failed");
            println!("{}", gp.show_caches().expect("Failed evaluating caches"));
        }
        Commands::Check { yaml_path } => {
            let gp = crate::game::parse::GameParse::new_from_yaml(&yaml_path, None)
                .expect("Parsing failed");
            gp.finalize_parsing(std::path::Path::new(".trash"), false)
                .expect("processing game failed");
        }
        Commands::Comparison {
            theme_light,
            theme_dark,
            html_path_de,
            html_path_us,
        } => {
            comparison::write_pages(&html_path_de, &html_path_us, theme_light, theme_dark).unwrap();
        }
    }
}

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

mod constraint;
mod game;
mod graph;
mod ruleset;

use crate::game::Game;

use clap::{Parser, Subcommand};
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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Sim {
        /// The path to the file to read
        yaml_path: PathBuf,

        #[arg(short = 'c', long = "color")]
        colored: bool,

        #[arg(long = "transpose")]
        transpose_tabs: bool,

        #[arg(short = 'o', long = "output")]
        stem: PathBuf,
    },
    Check {
        /// The path to the file to read
        yaml_path: PathBuf,
    },
    Graph {
        #[arg(short = 'l', long = "theme-light", default_value="1")]
        theme_light: u8,
        #[arg(short = 'd', long = "theme-dark", default_value="3")]
        theme_dark: u8,
        html_path_de: PathBuf,
        html_path_us: PathBuf,
    },
}

fn main() {
    let args = Cli::parse();

    match args.cmd {
        Commands::Sim {
            yaml_path,
            colored: _,
            transpose_tabs,
            stem,
        } => {
            let mut g = Game::new_from_yaml(&yaml_path, &stem).expect("Parsing failed");
            let start = Instant::now();
            g.sim(transpose_tabs).unwrap();
            println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
        }
        Commands::Check { yaml_path } => {
            Game::new_from_yaml(&yaml_path, std::path::Path::new(".trash"))
                .expect("Parsing failed!");
        }
        Commands::Graph {
            theme_light,
            theme_dark,
            html_path_de,
            html_path_us,
        } => {
            let html_content_light = graph::build_stats_graph(|e| e.starts_with("de"), theme_light).unwrap();
            let html_content_dark = graph::build_stats_graph(|e| e.starts_with("de"), theme_dark).unwrap();
            std::fs::write(html_path_de, format!(r#"---
title: 'DE'
weight: 1
bookToc: false
---
<div class="theme-specific-content">
<div class="light-theme-content" style="display: none;" data-theme="light">
{}
</div>
<div class="dark-theme-content" style="display: none;" data-theme="dark">
{}
</div>
</div>
"#,  &html_content_light, &html_content_dark)).unwrap();

            let html_content_light = graph::build_stats_graph(|e| e.starts_with("uk") || e.starts_with("us"), theme_light).unwrap();
            let html_content_dark = graph::build_stats_graph(|e| e.starts_with("uk") || e.starts_with("us"), theme_dark).unwrap();
            std::fs::write(html_path_us, format!(r#"---
title: 'US + UK'
weight: 1
bookToc: false
---
<div class="theme-specific-content">
<div class="light-theme-content" style="display: none;" data-theme="light">
{}
</div>
<div class="dark-theme-content" style="display: none;" data-theme="dark">
{}
</div>
</div>
"#,  &html_content_light, &html_content_dark)).unwrap();
        }
    }
}

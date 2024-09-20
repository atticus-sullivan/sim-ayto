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
                .expect("Parsing failed");
        }
        Commands::Graph {
            html_path_de,
            html_path_us,
        } => {
            let html_content = graph::build_stats_graph(|e| e.starts_with("de")).unwrap();
            std::fs::write(html_path_de, html_content).unwrap();

            let html_content = graph::build_stats_graph(|e| e.starts_with("us")).unwrap();
            std::fs::write(html_path_us, html_content).unwrap();
        }
    }
}

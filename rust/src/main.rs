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
mod ruleset;

use crate::game::Game;

use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

// TODO code review (try with chatGPT)

type Matching = Vec<Vec<u8>>;
type MapS = HashMap<String, String>;
type Map = HashMap<u8, u8>;
type Lut = HashMap<String, usize>;

type Rem = (Vec<Vec<u128>>, u128);

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The path to the file to read
    yaml_path: PathBuf,

    #[arg(short = 'c', long = "color")]
    colored: bool,

    #[arg(long = "transpose")]
    transpose_tabs: bool,

    #[arg(short = 'o', long = "output")]
    stem: PathBuf,

    #[arg(long = "only-check")]
    only_check: bool,
}

fn main() {
    let args = Cli::parse();
    let mut g = Game::new_from_yaml(&args.yaml_path, &args.stem).expect("Parsing failed");

    if args.only_check {
        return;
    }

    let start = Instant::now();
    g.sim(args.transpose_tabs).unwrap();
    println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
}

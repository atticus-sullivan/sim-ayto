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

use charming::component::{Legend, Toolbox};
use anyhow::{ensure, Context, Result};
use charming::element::{Tooltip, Trigger};
use clap::Parser;
use std::{collections::HashMap, path::Path};
use std::path::PathBuf;
use std::time::Instant;

use charming::{component::Axis, element::AxisType, series::Line, Chart, HtmlRenderer};

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

fn test_charming() -> Chart {
    Chart::new()
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
        )
        .y_axis(Axis::new().type_(AxisType::Value))
        .series(Line::new().name("a").data(vec![150, 230, 224, 218, 135, 147, 260]))
        .series(Line::new().name("b").data(vec![300, 460, 448, 436, 270, 294, 520]))
        .legend(Legend::new().data(vec!["a", "b"]))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
}

use walkdir::WalkDir;

type OtherDataEntry = (u8, f64);
struct OtherData {
    mb: Vec<OtherDataEntry>,
    mn: Vec<OtherDataEntry>,
    all: Vec<OtherDataEntry>,
}

fn build_stats_graph() -> Result<Chart> {
    let mut chart = Chart::new()
        .y_axis(Axis::new().type_(AxisType::Value))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
    ;

    for entry in WalkDir::new("../")
        .max_depth(2)
        .min_depth(2)
        .into_iter()
        .filter_entry(|e| {
            e.file_name().to_str().map_or(false, |e| (e.starts_with("s") || e.starts_with("us")) && e.ends_with(".csv")) &&
            e.metadata().map_or(false, |e| e.is_file())
        }).filter_map(|e| e.ok())
    {
        let mut o = OtherData{mb: Vec::new(), mn: Vec::new(), all: Vec::new()};
        let field:&mut Vec<OtherDataEntry>;
        let n = entry.file_name().to_str().unwrap();

        if n.ends_with("_MB.csv") {
            field = &mut o.mb;
        } else if n.ends_with("_MN.csv"){
            field = &mut o.mn;
        } else {
            // skip that file
            continue
        }

        let mut rdr = csv::Reader::from_path(entry.path())?;
        for result in rdr.deserialize() {
            let record: OtherDataEntry = result?;
            field.push(record);
        }
        chart = chart.series(Line::new().name("a").data(field));

        println!("{} {:?}", entry.path().display(), entry.file_name());
    }

    Ok(chart)
}

fn main() {
    let args = Cli::parse();
    let mut g = Game::new_from_yaml(&args.yaml_path, &args.stem).expect("Parsing failed");

    let mut render = HtmlRenderer::new("title", 500, 500);
    let _ = render.save(&build_stats_graph().unwrap(), Path::new("./x.html"));

    return;

    if args.only_check {
        return;
    }

    let start = Instant::now();
    g.sim(args.transpose_tabs).unwrap();
    println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
}

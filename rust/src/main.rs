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

use anyhow::{ensure, Context, Result};
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use std::iter::zip;

use plotly::common::{Mode, Title};
use plotly::{Layout, Plot, Scatter};

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

    #[arg(long = "only-graph")]
    only_graph: bool,
}

use walkdir::WalkDir;

type OtherDataEntry = (f64, f64);

fn build_stats_graph() -> Result<String> {
    let layout = Layout::new()
        .hover_mode(plotly::layout::HoverMode::XUnified)
        .click_mode(plotly::layout::ClickMode::Event);

    let mut plots = [Plot::new(), Plot::new(), Plot::new()];
    plots[0].set_layout(layout.clone().title("")
            .x_axis(plotly::layout::Axis::new().title(Title::with_text("#MB")))
            .y_axis(plotly::layout::Axis::new().title(Title::with_text("I [bit]"))));
    plots[1].set_layout(layout.clone().title("")
            .x_axis(plotly::layout::Axis::new().title(Title::with_text("#MN")))
            .y_axis(plotly::layout::Axis::new().title(Title::with_text("I [bit]"))));
    plots[2].set_layout(layout.clone().title("")
            .x_axis(plotly::layout::Axis::new().title(Title::with_text("#MN/#MB")))
            .y_axis(plotly::layout::Axis::new().title(Title::with_text("H [bit]"))));

    for entry in WalkDir::new("../")
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| {
            e.file_name().to_str().map_or(false, |e| (e.starts_with("s") || e.starts_with("us"))) &&
            e.metadata().map_or(false, |e| e.is_dir())
        }).filter_map(Result::ok)
    {
        for (fn_param, plot) in zip(["statMB.csv", "statMN.csv", "statInfo.csv"], &mut plots) {
            if !entry.path().join(fn_param).exists() {
                continue
            }
            let mut field: Vec<OtherDataEntry> = Vec::new();
            let mut rdr = csv::ReaderBuilder::new().delimiter(b' ').from_path(entry.path().join(fn_param))?;
            for result in rdr.deserialize() {
                let record: OtherDataEntry = result?;
                field.push(record);
            }
            let trace = Scatter::new(field.iter().map(|i| i.0).collect(), field.iter().map(|i| i.1).collect())
                .name(fn_param)
                .mode(Mode::Lines);
            plot.add_trace(trace);
        }
    }
    let dat = plots.iter().map(|i| i.to_inline_html(None)).fold(String::new(), |a,b| a+&b);
    let complete_html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Plotly Graphs</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
</head>
<body>
    {}
</body>
</html>"#,
        dat
    );
    Ok(complete_html)
}

fn main() {
    let args = Cli::parse();
    if args.only_graph {
        let html_content = build_stats_graph().unwrap();
        std::fs::write("stat.html", html_content).unwrap();
        return;
    }

    let mut g = Game::new_from_yaml(&args.yaml_path, &args.stem).expect("Parsing failed");

    if args.only_check {
        return;
    }

    let start = Instant::now();
    g.sim(args.transpose_tabs).unwrap();
    println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
}

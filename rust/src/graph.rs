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

use anyhow::Result;
use plotly::common::{Mode, Title};
use plotly::{Layout, Plot, Scatter};
use std::iter::zip;
use walkdir::WalkDir;

type OtherDataEntry = (f64, f64, String);

pub fn build_stats_graph() -> Result<String> {
    let layout = Layout::new()
        .hover_mode(plotly::layout::HoverMode::X)
        .click_mode(plotly::layout::ClickMode::Event)
        .drag_mode(plotly::layout::DragMode::Pan)
        .height(800);

    let mut plots = [Plot::new(), Plot::new(), Plot::new()];
    plots[0].set_layout(
        layout
            .clone()
            .title("Matchingbox")
            .x_axis(
                plotly::layout::Axis::new()
                    .title(Title::with_text("#MB"))
                    .mirror(true)
                    .show_line(true),
            )
            .y_axis(plotly::layout::Axis::new().title(Title::with_text("I [bit]"))),
    );
    plots[1].set_layout(
        layout
            .clone()
            .title("Matchingnight")
            .x_axis(
                plotly::layout::Axis::new()
                    .title(Title::with_text("#MN"))
                    .mirror(true)
                    .show_line(true),
            )
            .y_axis(plotly::layout::Axis::new().title(Title::with_text("I [bit]"))),
    );
    plots[2].set_layout(
        layout
            .clone()
            .title("Left possibilities")
            .x_axis(
                plotly::layout::Axis::new()
                    .title(Title::with_text("#MB/#MN"))
                    .mirror(true)
                    .show_line(true),
            )
            .y_axis(plotly::layout::Axis::new().title(Title::with_text("H [bit]"))),
    );

    for p in &mut plots {
        p.set_configuration(
            plotly::Configuration::new()
                .display_logo(false)
                .scroll_zoom(true),
        );
    }

    for entry in WalkDir::new("./")
        .max_depth(1)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            e.file_name()
                .to_str()
                .map_or(false, |e| (e.starts_with("s") || e.starts_with("us")))
                && e.metadata().map_or(false, |e| e.is_dir())
        })
        .filter_map(Result::ok)
    {
        for (fn_param, plot) in zip(["statMB.csv", "statMN.csv", "statInfo.csv"], &mut plots) {
            if !entry.path().join(fn_param).exists() {
                continue;
            }
            let mut field: Vec<OtherDataEntry> = Vec::new();
            let mut rdr = csv::ReaderBuilder::new()
                .delimiter(b';')
                .has_headers(false)
                .from_path(entry.path().join(fn_param))?;
            for result in rdr.deserialize() {
                let record: OtherDataEntry = result?;
                field.push(record);
            }
            let trace = Scatter::new(
                field.iter().map(|i| i.0).collect(),
                field.iter().map(|i| i.1).collect(),
            )
            .name(entry.file_name().to_str().unwrap_or("unknown"))
            .text_array(field.iter().map(|i| i.2.clone()).collect())
            .mode(Mode::Lines);
            plot.add_trace(trace);
        }
    }
    let dat = plots
        .iter()
        .map(|i| i.to_inline_html(None))
        .fold(String::new(), |a, b| a + &b);
    let complete_html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Stats</title>
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

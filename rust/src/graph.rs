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
use walkdir::WalkDir;
use catppuccin::PALETTE;

use crate::constraint::{CSVEntry, CSVEntryMB};

pub fn build_stats_graph(filter_dirs: fn(&str) -> bool, theme: u8) -> Result<String> {
    let palette = if theme == 0 {
        PALETTE.frappe
    } else {
        PALETTE.latte
    };

    let layout = Layout::new()

        .paper_background_color(palette.colors.base.hex.to_string())
        .plot_background_color(palette.colors.base.hex.to_string())
        .font(plotly::common::Font::new().color(palette.colors.text.hex.to_string()))
        .colorway(vec![
            palette.colors.blue.hex.to_string(),
            palette.colors.yellow.hex.to_string(),
            palette.colors.green.hex.to_string(),
            palette.colors.red.hex.to_string(),
            palette.colors.mauve.hex.to_string(),
            palette.colors.rosewater.hex.to_string(),
            palette.colors.pink.hex.to_string(),
            palette.colors.peach.hex.to_string(),
            palette.colors.maroon.hex.to_string(),
            palette.colors.teal.hex.to_string(),
            palette.colors.sapphire.hex.to_string(),
        ])


        .hover_mode(plotly::layout::HoverMode::X)
        .click_mode(plotly::layout::ClickMode::Event)
        .drag_mode(plotly::layout::DragMode::Pan)
        .height(800);

    let mut plots = [Plot::new(), Plot::new(), Plot::new()];
    plots[0].set_layout(
        layout
            .clone()
            .title("Matchingnight / matching ceremony")
            .x_axis(
                plotly::layout::Axis::new()
                    .line_color(palette.colors.overlay0.hex.to_string())
                    .grid_color(palette.colors.overlay1.hex.to_string())
                    .zero_line_color(palette.colors.overlay2.hex.to_string())

                    .title(Title::with_text("#MB"))
                    .mirror(true)
                    .show_line(true),
            )
            .y_axis(
                plotly::layout::Axis::new()
                    .line_color(palette.colors.overlay0.hex.to_string())
                    .grid_color(palette.colors.overlay1.hex.to_string())
                    .zero_line_color(palette.colors.overlay2.hex.to_string())

                    .title(Title::with_text("I [bit]"))
            ),
    );
    plots[1].set_layout(
        layout
            .clone()
            .title("Matchbox / truth booth")
            .x_axis(
                plotly::layout::Axis::new()
                    .line_color(palette.colors.overlay0.hex.to_string())
                    .grid_color(palette.colors.overlay1.hex.to_string())
                    .zero_line_color(palette.colors.overlay2.hex.to_string())

                    .title(Title::with_text("#MN"))
                    .mirror(true)
                    .show_line(true),
            )
            .y_axis(
                plotly::layout::Axis::new()
                    .line_color(palette.colors.overlay0.hex.to_string())
                    .grid_color(palette.colors.overlay1.hex.to_string())
                    .zero_line_color(palette.colors.overlay2.hex.to_string())

                    .title(Title::with_text("I [bit]"))
            ),
    );
    plots[2].set_layout(
        layout
            .clone()
            .title("Left possibilities")
            .x_axis(
                plotly::layout::Axis::new()
                    .line_color(palette.colors.overlay0.hex.to_string())
                    .grid_color(palette.colors.overlay1.hex.to_string())
                    .zero_line_color(palette.colors.overlay2.hex.to_string())

                    .title(Title::with_text("#MB/#MN"))
                    .mirror(true)
                    .show_line(true),
            )
            .y_axis(
                plotly::layout::Axis::new()
                    .line_color(palette.colors.overlay0.hex.to_string())
                    .grid_color(palette.colors.overlay1.hex.to_string())
                    .zero_line_color(palette.colors.overlay2.hex.to_string())

                    .title(Title::with_text("H [bit]"))
            ),
    );

    for p in &mut plots {
        p.set_configuration(
            plotly::Configuration::new()
                .display_logo(false)
                .scroll_zoom(true),
        );
    }

    for entry in WalkDir::new("./data")
        .max_depth(1)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            e.file_name().to_str().map_or(false, |e| filter_dirs(e))
                && e.metadata().map_or(false, |e| e.is_dir())
        })
        .filter_map(Result::ok)
    {
        // read the matchbox csv first, since it has a different type and also to find out whether
        // they won or not
        let fn_param = "statMN.csv";
        if !entry.path().join(fn_param).exists() {
            continue;
        }
        let mut field: Vec<CSVEntryMB> = Vec::new();
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(false)
            .from_path(entry.path().join(fn_param))?;
        for result in rdr.deserialize() {
            let record: CSVEntryMB = result?;
            field.push(record);
        }
        let name_suffix = match field.iter().find(|x| x.1 == 10.0) {
            Some((true, _, _, _)) => "- W",
            Some((false, _, _, _)) => "- L",
            None => match field.last() {
                Some((true, _, _, _)) => "- W",
                Some((false, _, _, _)) => "- L",
                None => "",
            },
        };
        let trace = Scatter::new(
            field.iter().map(|i| i.1).collect(),
            field.iter().map(|i| i.2).collect(),
        )
        .name(entry.file_name().to_str().unwrap_or("unknown").to_owned() + name_suffix)
        .text_array(field.iter().map(|i| i.3.clone()).collect())
        .mode(Mode::Lines);
        plots[0].add_trace(trace);

        // read the other csv files both have the same structure -> use a loop
        for (plot_idx, fn_param) in ["statMB.csv", "statInfo.csv"].iter().enumerate() {
            if !entry.path().join(fn_param).exists() {
                continue;
            }
            let mut field: Vec<CSVEntry> = Vec::new();
            let mut rdr = csv::ReaderBuilder::new()
                .delimiter(b';')
                .has_headers(false)
                .from_path(entry.path().join(fn_param))?;
            for result in rdr.deserialize() {
                let record: CSVEntry = result?;
                field.push(record);
            }
            let trace = Scatter::new(
                field.iter().map(|i| i.0).collect(),
                field.iter().map(|i| i.1).collect(),
            )
            .name(entry.file_name().to_str().unwrap_or("unknown").to_owned() + name_suffix)
            .text_array(field.iter().map(|i| i.2.clone()).collect())
            .mode(Mode::Lines);
            plots[plot_idx + 1].add_trace(trace);
        }
    }
    let dat = plots
        .iter()
        .map(|i| i.to_inline_html(None))
        .fold(String::new(), |a, b| a + &b);
    let complete_html = format!(
        r#"<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
{}"#,
        dat
    );
    Ok(complete_html)
}

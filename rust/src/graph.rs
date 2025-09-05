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

use std::path::Path;

use anyhow::Result;
use catppuccin::PALETTE;
use plotly::common::{Mode, Title};
use plotly::{Layout, Plot, Scatter};
use walkdir::WalkDir;

use crate::constraint::{CSVEntry, CSVEntryMB, CSVEntryMN};

use crate::game::Game;

pub fn ruleset_tab_md(filter_dirs: fn(&str) -> bool) -> Result<String> {
    let mut tab_lines = vec![];
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
        let mut dat = entry.path().join(entry.file_name());
        dat.set_extension("yaml");
        let g = Game::new_from_yaml(dat.as_path(), Path::new("/tmp/")).expect("Parsing failed");

        let name = entry.file_name().to_str().unwrap_or("unknown").to_owned();

        let r = g.ruleset_str();
        tab_lines.push(format!(
            "| {} | {} | {} | {{{{< i18n \"{}\" >}}}} |",
            name,
            g.players_str(),
            r.1,
            r.0
        ));
    }
    tab_lines.sort();

    Ok(format!(
        r#"| {{{{< i18n "season" >}}}} | {{{{< i18n "players" >}}}} | {{{{< i18n "rulesetShort" >}}}} | {{{{< i18n "rulesetDesc" >}}}} |
| --- | --- | --- | --- |
{}
    "#,
        tab_lines.join("\n")
    ))
}

pub fn build_stats_graph(filter_dirs: fn(&str) -> bool, theme: u8) -> Result<String> {
    let palette = match theme {
        1 => PALETTE.latte,
        2 => PALETTE.frappe,
        3 => PALETTE.macchiato,
        4 => PALETTE.mocha,

        _ => PALETTE.frappe,
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
        .margin(plotly::layout::Margin::new().auto_expand(true))
        .auto_size(true);
    // .width(1000)
    // .height(800);

    let mut plots = [
        ("MN/MC", Plot::new()),
        ("MB/TB", Plot::new()),
        ("Combined", Plot::new()),
        ("#Lights MB/TB", Plot::new()),
        ("#Lights MN/MC", Plot::new()),
        ("#Lights-known MN/MC", Plot::new()),
    ];
    plots[0].1.set_layout(
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
                    .title(Title::with_text("I [bit]")),
            ),
    );
    plots[1].1.set_layout(
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
                    .title(Title::with_text("I [bit]")),
            ),
    );
    plots[2].1.set_layout(
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
                    .title(Title::with_text("H [bit]")),
            ),
    );
    plots[3].1.set_layout(
        layout
            .clone()
            .title("#Lights -- MB")
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
                    .title(Title::with_text("#Lights")),
            ),
    );
    plots[4].1.set_layout(
        layout
            .clone()
            .title("#Lights -- MN")
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
                    .title(Title::with_text("#Lights")),
            ),
    );
    plots[5].1.set_layout(
        layout
            .clone()
            .title("#Lights - known_lights -- MN")
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
                    .title(Title::with_text("#Lights - known_lights")),
            ),
    );

    for (_, p) in &mut plots {
        p.set_configuration(
            plotly::Configuration::new()
                .display_logo(false)
                .responsive(true)
                // .fill_frame(true)
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
        // read the matchingnight csv first, since it has a different type and also to find out whether
        // they won or not
        let fn_param = "statMN.csv";
        if !entry.path().join(fn_param).exists() {
            continue;
        }
        let mut field: Vec<CSVEntryMN> = Vec::new();
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(false)
            .from_path(entry.path().join(fn_param))?;
        for result in rdr.deserialize() {
            let record: CSVEntryMN = result?;
            field.push(record);
        }
        let name_suffix = match field.iter().find(|x| x.num == 10.0) {
            Some(x) => {
                if x.won {
                    "- W"
                } else {
                    "-L"
                }
            }
            None => match field.last() {
                Some(x) => {
                    if x.won {
                        "- W"
                    } else {
                        "-L"
                    }
                }
                None => "",
            },
        };
        let trace = Scatter::new(
            field.iter().map(|i| i.num).collect(),
            field.iter().map(|i| i.bits_gained).collect(),
        )
        .name(entry.file_name().to_str().unwrap_or("unknown").to_owned() + name_suffix)
        .text_array(field.iter().map(|i| i.comment.clone()).collect())
        .mode(Mode::Lines);
        plots[0].1.add_trace(trace);

        let trace = Scatter::new(
            field
                .iter()
                .filter_map(|i| i.lights_total.map(|_| i.num))
                .collect(),
            field.iter().filter_map(|i| i.lights_total).collect(),
        )
        .name(entry.file_name().to_str().unwrap_or("unknown").to_owned() + name_suffix)
        .text_array(field.iter().map(|i| i.comment.clone()).collect())
        .mode(Mode::LinesMarkers);
        plots[4].1.add_trace(trace);

        let trace = Scatter::new(
            field
                .iter()
                .filter_map(|i| i.lights_total.map(|_| i.num))
                .collect(),
            field
                .iter()
                .filter_map(|i| {
                    i.lights_total
                        .map(|lt| lt - i.lights_known_before.unwrap_or(0))
                })
                .collect(),
        )
        .name(entry.file_name().to_str().unwrap_or("unknown").to_owned() + name_suffix)
        .text_array(field.iter().map(|i| i.comment.clone()).collect())
        .mode(Mode::LinesMarkers);
        plots[5].1.add_trace(trace);

        // read matchbox stats
        let fn_param = "statMB.csv";
        if !entry.path().join(fn_param).exists() {
            continue;
        }
        let mut field: Vec<CSVEntryMB> = Vec::new();
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(false)
            .from_path(entry.path().join(fn_param))?;
        for result in rdr.deserialize() {
            let record: CSVEntryMB = result?;
            field.push(record);
        }
        let trace = Scatter::new(
            field.iter().map(|i| i.num).collect(),
            field.iter().map(|i| i.bits_gained).collect(),
        )
        .name(entry.file_name().to_str().unwrap_or("unknown").to_owned() + name_suffix)
        .text_array(field.iter().map(|i| i.comment.clone()).collect())
        .mode(Mode::Lines);
        plots[1].1.add_trace(trace);

        let trace = Scatter::new(
            field
                .iter()
                .filter_map(|i| i.lights_total.map(|_| i.num))
                .collect(),
            field.iter().filter_map(|i| i.lights_total).collect(),
        )
        .name(entry.file_name().to_str().unwrap_or("unknown").to_owned() + name_suffix)
        .text_array(field.iter().map(|i| i.comment.clone()).collect())
        .mode(Mode::LinesMarkers);
        plots[3].1.add_trace(trace);

        // read overall stats
        let fn_param = "statInfo.csv";
        if !entry.path().join(fn_param).exists() {
            continue;
        }
        let mut field: Vec<CSVEntry> = Vec::new();
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .has_headers(false)
            .from_path(entry.path().join(fn_param))?;
        for result in rdr.deserialize() {
            let record: CSVEntry = result?;
            field.push(record);
        }
        let trace = Scatter::new(
            field.iter().map(|i| i.num).collect(),
            field.iter().map(|i| i.bits_left).collect(),
        )
        .name(entry.file_name().to_str().unwrap_or("unknown").to_owned() + name_suffix)
        .text_array(field.iter().map(|i| i.comment.clone()).collect())
        .mode(Mode::Lines);
        plots[2].1.add_trace(trace);
    }
    let dat = plots
        .iter()
        .map(|(j, i)| {
            format!("{{{{% tab \"{j}\" %}}}}").to_string()
                + &i.to_inline_html(None)
                + "{{% /tab %}}"
        })
        .fold(String::new(), |a, b| a + &b);
    let complete_html = format!(
        r#"<script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
{{{{< tabs "id" >}}}}
{}
{{{{< /tabs >}}}}
<script>window.dispatchEvent(new Event('resize'));</script>"#,
        dat
    );
    Ok(complete_html)
}

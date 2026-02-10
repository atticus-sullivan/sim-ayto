use crate::comparison::CmpData;
use catppuccin::{Flavor, PALETTE};
use plotly::HeatMap;
use plotly::{
    common::{Mode, Title},
    Layout, Plot, Scatter,
};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

pub fn lut_theme(theme: u8) -> Flavor {
    match theme {
        1 => PALETTE.latte,
        2 => PALETTE.frappe,
        3 => PALETTE.macchiato,
        4 => PALETTE.mocha,

        _ => PALETTE.frappe,
    }
}

pub fn plotly_new_plot() -> Plot {
    let mut plot = Plot::new();
    plot.set_configuration(
        plotly::Configuration::new()
            .display_logo(false)
            .responsive(true)
            // .fill_frame(true)
            .scroll_zoom(true),
    );
    plot
}

pub fn plotly_gen_layout(palette: Flavor) -> Layout {
    Layout::new()
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
        .auto_size(true)
    // .width(1000)
    // .height(800)
}

#[allow(clippy::too_many_arguments)]
pub fn build_scatter_plot<X, Y, FX, FY>(
    cmp_data: &Vec<(String, CmpData)>,
    layout: &Layout,
    palette: &Flavor,
    title: &str,
    x_title: &str,
    y_title: &str,
    mode: Mode,
    x_fn: FX,
    y_fn: FY,
) -> String
where
    FX: Fn(&CmpData) -> Vec<X>,
    FY: Fn(&CmpData) -> Vec<Y>,
    X: Clone + Serialize + 'static,
    Y: Clone + Serialize + 'static,
{
    let mut plot = plotly_new_plot();

    plot.set_layout(
        layout
            .clone()
            .title(title)
            .x_axis(styled_axis(palette, x_title, true))
            .y_axis(styled_axis(palette, y_title, false)),
    );

    for (name, cd) in cmp_data {
        let trace = Scatter::new(x_fn(cd), y_fn(cd))
            .name(name)
            .text_array(cd.info.iter().map(|i| i.comment.clone()).collect())
            .mode(mode.clone());

        plot.add_trace(trace);
    }

    plot.to_inline_html(None)
}

fn styled_axis(palette: &Flavor, title: &str, mirror: bool) -> plotly::layout::Axis {
    plotly::layout::Axis::new()
        .line_color(palette.colors.overlay0.hex.to_string())
        .grid_color(palette.colors.overlay1.hex.to_string())
        .zero_line_color(palette.colors.overlay2.hex.to_string())
        .title(Title::with_text(title))
        .mirror(mirror)
        .show_line(true)
}

#[derive(Clone, Debug)]
pub struct EntryDatum {
    pub num: Decimal,          // the value you bucket by (we call .floor() on it)
    pub val: Option<f64>,      // the value to render in the heatmap cell (None => empty)
    pub hover: Option<String>, // comment/additional data shown on hover
}

/// Generic heatmap builder. `entries_fn` must produce a Vec<EntryDatum> for each CmpData.
pub fn build_heatmap_plot<F>(
    cmp_data: &Vec<(String, CmpData)>,
    layout: &Layout,
    palette: &Flavor,
    title: &str,
    x_title: &str,
    entries_fn: F,
) -> String
where
    F: Fn(&CmpData) -> Vec<EntryDatum>,
{
    let mut plot = plotly_new_plot();

    // build x edges and internal layouts (bucket -> #slots)
    let (x_edges, layouts) = generate_buckets_from_entries(cmp_data, &entries_fn);

    // y labels are the names/seasons
    let y_labels = cmp_data.iter().map(|i| i.0.clone()).collect::<Vec<_>>();

    // build z as rows per season
    let (_texts, z) = build_z_from_entries(cmp_data, &layouts, &entries_fn);

    let heatmap = HeatMap::new(x_edges, y_labels, z)
        // .hover_text(texts)
        // .hover_info(plotly::common::HoverInfo::Text)
    ;

    plot.add_trace(heatmap);

    plot.set_layout(
        layout
            .clone()
            .title(title)
            .x_axis(styled_axis(palette, x_title, true))
            .y_axis(styled_axis(palette, "Season", false)),
    );

    plot.to_inline_html(None)
}

/// Build the bucket edges and the layouts (bucket -> slot count) from entries produced by `entries_fn`.
fn generate_buckets_from_entries<F>(
    cmp_data: &Vec<(String, CmpData)>,
    entries_fn: &F,
) -> (Vec<Decimal>, Vec<(Decimal, usize)>)
where
    F: Fn(&CmpData) -> Vec<EntryDatum>,
{
    // bucket -> season -> values
    let mut bucket_map: BTreeMap<Decimal, HashMap<String, Vec<Decimal>>> = BTreeMap::new();

    // place the nums used by a season in buckets and store to which season it belongs in order to
    // obtain the maximum per season
    for (season, data) in cmp_data {
        let entries = entries_fn(data);
        for e in &entries {
            let b = e.num.floor();
            bucket_map
                .entry(b)
                .or_default()
                .entry(season.clone())
                .or_default()
                .push(e.num);
        }
    }

    // determine max slots per bucket and produce edges
    let mut layouts = Vec::new();
    let mut edges = Vec::new();

    for (bucket, seasons) in &bucket_map {
        let max_slots = seasons.values().map(|v| v.len()).max().unwrap_or(1);

        // width of a sub-slot inside a bucket
        let w = Decimal::ONE / Decimal::from_usize(max_slots).unwrap();
        edges.extend((0..max_slots).map(|i| Decimal::from_usize(i).unwrap() * w + *bucket));

        layouts.push((*bucket, max_slots));
    }

    (edges, layouts)
}

/// Build the Z matrix (rows per season) from entries generated by `entries_fn`.
/// The matrix element type is `Option<f64>` so `None` becomes null in plotly.
fn build_z_from_entries<F>(
    cmp_data: &[(String, CmpData)],
    layouts: &[(Decimal, usize)],
    entries_fn: &F,
) -> (Vec<Vec<String>>, Vec<Vec<Option<f64>>>)
where
    F: Fn(&CmpData) -> Vec<EntryDatum>,
{
    let total_cols: usize = layouts.iter().map(|(_, slots)| *slots).sum();

    let mut z_rows: Vec<Vec<Option<f64>>> = Vec::with_capacity(cmp_data.len());
    let mut texts_rows: Vec<Vec<String>> = Vec::with_capacity(cmp_data.len());

    for (_, season_data) in cmp_data.iter() {
        // bucket -> ordered values for this season
        let mut season_val_map: HashMap<Decimal, Vec<Option<f64>>> = HashMap::new();
        let mut season_text_map: HashMap<Decimal, Vec<Option<String>>> = HashMap::new();

        for e in &entries_fn(season_data) {
            let b = e.num.floor();
            season_val_map.entry(b).or_default().push(e.val);

            let b = e.num.floor();
            season_text_map.entry(b).or_default().push(e.hover.clone());
        }

        let mut z_row = Vec::with_capacity(total_cols);
        let mut text_row: Vec<String> = Vec::with_capacity(total_cols);
        for (bucket, slots) in layouts {
            match season_val_map.get(bucket) {
                Some(values) if !values.is_empty() => {
                    let last = values.last().cloned().unwrap_or(None);

                    for i in 0..*slots {
                        let cell = values.get(i).cloned().unwrap_or(last);
                        z_row.push(cell);
                    }
                }
                _ => {
                    // bucket does not exist for this season
                    for _ in 0..*slots {
                        z_row.push(None);
                    }
                }
            }

            // Hover texts (parallel logic so indices line up with z_row)
            match season_text_map.get(bucket) {
                Some(texts) if !texts.is_empty() => {
                    let last_text = texts.last().cloned().unwrap_or(None);
                    for i in 0..*slots {
                        let cell_text = texts.get(i).cloned().unwrap_or(last_text.clone());
                        // convert Option<String> -> String for flattened output
                        text_row.push(cell_text.unwrap_or_default());
                    }
                }
                _ => {
                    for _ in 0..*slots {
                        text_row.push(String::new());
                    }
                }
            }
        }
        z_rows.push(z_row);
        texts_rows.push(text_row);
    }
    (texts_rows, z_rows)
}

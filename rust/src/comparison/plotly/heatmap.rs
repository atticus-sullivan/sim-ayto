/// This module renders heatmaps with the help of plotly. It also supports "uneven" boxes. This
/// means that a box for on y-series can be of different length than the box on for the same
/// x-column for another y-series. It achieves this by splitting the larger box into multiple identical
/// ones.
use catppuccin::Flavor;
use plotly::common::{ColorScale, Title};
use plotly::{HeatMap, Layout};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::comparison::data::CmpData;
use crate::comparison::plotly::heatmap_utils::*;
use crate::comparison::plotly::layout::plotly_new_plot;
use crate::comparison::plotly::layout::styled_axis;
use crate::comparison::theme::plotly_colorscale;

/// Single input datum used to construct heatmap entries.
///
/// `num` - a numeric key used for bucketing (we call `.floor()` on it).
/// `val` - the numeric value used for the heatmap cell (None => empty cell).
/// `hover` - optional hover string for this datum.
#[derive(Clone, Debug)]
pub(crate) struct EntryDatum {
    pub(crate) num: Decimal, // the value you bucket by (we call .floor() on it)
    pub(crate) val: Option<f64>, // the value to render in the heatmap cell (None => empty)
    pub(crate) hover: Option<String>, // comment/additional data shown on hover
}

/// Render a heatmap (inline Plotly HTML) from a pre-built matrix representation.
///
/// `x_edges` are the x-axis bucket boundaries (columns). `y_labels` are the row labels
/// (one per row of `z`). `z` is a matrix of `Option<f64>` values where `None` represents
/// an empty cell. `texts` is a parallel structure (same shape as `z`) of hover strings.
///
/// This function is generic over the element type `T` used for `x_edges` so tests can
/// pass `f64` while production code can continue to pass `Decimal`.
#[allow(clippy::too_many_arguments)]
fn heatmap_from_matrix<T>(
    x_edges: Vec<T>,
    y_labels: Vec<String>,
    z: Vec<Vec<Option<f64>>>,
    texts: Vec<Vec<String>>,
    layout: &Layout,
    palette: &Flavor,
    title: &str,
    x_title: &str,
    z_title: &str,
    color_scale: ColorScale,
) -> String
where
    T: Serialize + Clone + 'static,
{
    let mut plot = plotly_new_plot();

    // Create the heatmap trace. Plotly can serialize the generically-typed x_edges.
    let heatmap = HeatMap::new(x_edges, y_labels.clone(), z)
        .color_scale(color_scale)
        .x_gap(4)
        .y_gap(4)
        .hover_template(format!(
            "{}: %{{z}}<br>%{{y}}<br>%{{text}}<extra></extra>",
            z_title
        ))
        .text_matrix(texts);

    // Add the trace and apply layout (title + axis styling).
    plot.add_trace(heatmap);

    plot.set_layout(
        layout
            .clone()
            .title(Title::with_text(title))
            .x_axis(styled_axis(palette, x_title, true))
            .y_axis(styled_axis(palette, "Season", false)),
    );

    plot.to_inline_html(None)
}

/// Generic heatmap builder. `entries_fn` must produce a `Vec<EntryDatum>` for a given data item.
///
/// Accepts `cmp_data` as `Vec<(label, data)>` where `data` will be passed to `entries_fn`.
/// Returns inline plot HTML.
pub(crate) fn build_heatmap_plot<F>(
    cmp_data: &Vec<(String, CmpData)>,
    layout: &Layout,
    palette: &Flavor,
    title: &str,
    x_title: &str,
    z_title: &str,
    entries_fn: F,
) -> String
where
    F: Fn(&CmpData) -> Vec<EntryDatum>,
{
    // build x edges and internal layouts (bucket -> #slots)
    let (x_edges, layouts) = generate_buckets_from_entries(cmp_data, &entries_fn);

    // y labels are the names/seasons
    let y_labels = cmp_data.iter().map(|i| i.0.clone()).collect::<Vec<_>>();

    // build z as rows per season
    let (texts, z) = build_z_from_entries(cmp_data, &layouts, &entries_fn);

    heatmap_from_matrix(
        x_edges,
        y_labels,
        z,
        texts,
        layout,
        palette,
        title,
        x_title,
        z_title,
        plotly_colorscale(palette),
    )
}

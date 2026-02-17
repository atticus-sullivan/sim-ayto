use std::collections::{BTreeMap, HashMap};

use catppuccin::Flavor;
use plotly::common::{ColorScale, Title};
use plotly::{HeatMap, Layout};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::{dec, Decimal};
use serde::Serialize;

use crate::comparison::plotly::layout::plotly_new_plot;
use crate::comparison::plotly::layout::styled_axis;
use crate::comparison::theme::plotly_colorscale;
use crate::comparison::CmpData;

/// Render a heatmap (inline Plotly HTML) from a pre-built matrix representation.
///
/// `x_edges` are the x-axis bucket boundaries (columns). `y_labels` are the row labels
/// (one per row of `z`). `z` is a matrix of `Option<f64>` values where `None` represents
/// an empty cell. `texts` is a parallel structure (same shape as `z`) of hover strings.
///
/// This function is generic over the element type `T` used for `x_edges` so tests can
/// pass `f64` while production code can continue to pass `Decimal`.
#[allow(clippy::too_many_arguments)]
pub fn heatmap_from_matrix<T>(
    x_edges: Vec<T>,
    y_labels: Vec<String>,
    z: Vec<Vec<Option<f64>>>,
    texts: Vec<Vec<String>>,
    layout: &Layout,
    palette: &Flavor,
    title: &str,
    x_title: &str,
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
        .hover_template("lights: %{z}<br>%{y}<br>%{text}<extra></extra>") // TODO: parametrize lights
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

/// Single input datum used to construct heatmap entries.
///
/// `num` - a numeric key used for bucketing (we call `.floor()` on it).
/// `val` - the numeric value used for the heatmap cell (None => empty cell).
/// `hover` - optional hover string for this datum.
#[derive(Clone, Debug)]
pub struct EntryDatum {
    pub num: Decimal,          // the value you bucket by (we call .floor() on it)
    pub val: Option<f64>,      // the value to render in the heatmap cell (None => empty)
    pub hover: Option<String>, // comment/additional data shown on hover
}

/// Generic heatmap builder. `entries_fn` must produce a `Vec<EntryDatum>` for a given data item.
///
/// Accepts `cmp_data` as `Vec<(label, data)>` where `data` will be passed to `entries_fn`.
/// Returns inline plot HTML.
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
        plotly_colorscale(palette),
    )
}

/// Build the bucket edges and layout (bucket -> slot count) from entries produced by `entries_fn`.
///
/// `cmp_data` is a slice of `(label, data)` pairs. The function is generic over the
/// data type `D` so the same logic can be tested with a minimal test struct. `entries_fn`
/// should yield a vector of `EntryDatum` for each `D`.
fn generate_buckets_from_entries<F, D>(
    cmp_data: &Vec<(String, D)>,
    entries_fn: &F,
) -> (Vec<Decimal>, Vec<(Decimal, usize)>)
where
    F: Fn(&D) -> Vec<EntryDatum>,
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

    // add one element more to the x axis so boxes are not centered
    edges.push(edges.last().unwrap() + dec![1]);

    (edges, layouts)
}

/// Build the Z matrix (rows per label) from entries generated by `entries_fn`.
///
/// `cmp_data` is a slice of `(label, data)` pairs where `data` is generic `D`.
/// Returns `(texts_rows, z_rows)` where `texts_rows` is `Vec<Vec<String>>` (hover
/// text per cell) and `z_rows` is `Vec<Vec<Option<f64>>>` (heatmap numeric cells).
fn build_z_from_entries<D, F>(
    cmp_data: &[(String, D)],
    layouts: &[(Decimal, usize)],
    entries_fn: &F,
) -> (Vec<Vec<String>>, Vec<Vec<Option<f64>>>)
where
    F: Fn(&D) -> Vec<EntryDatum>,
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

#[cfg(test)]
mod tests {
    use crate::comparison::{plotly::layout::plotly_gen_layout, theme::lut_theme};

    use super::*;

    #[test]
    fn heatmap_from_matrix_generates_html_with_f64_edges() {
        // simple 2x2 heatmap:
        // x_edges length: 2 (two columns)
        let x_edges = vec![0.0f64, 1.0f64];
        let y_labels = vec!["season_a".to_string(), "season_b".to_string()];

        // z must have same number of rows as y_labels, and each row must have columns == x_edges.len()
        let z = vec![
            vec![Some(1.0f64), None],         // row 0
            vec![Some(2.0f64), Some(3.0f64)], // row 1
        ];

        let texts = vec![
            vec!["a".to_string(), "".to_string()],
            vec!["b".to_string(), "c".to_string()],
        ];

        let palette = lut_theme(1);
        let layout = plotly_gen_layout(palette);

        let html = heatmap_from_matrix(
            x_edges,
            y_labels,
            z,
            texts,
            &layout,
            &palette,
            "MyHeatmap",
            "X axis",
            plotly_colorscale(&palette),
        );
        assert!(html.contains("<div")); // basic sanity: we got HTML back
        assert!(html.contains("MyHeatmap")); // title present
                                             // plotly inline output contains "Plotly" or "data" keys as JS — just check there's some content
        assert!(html.len() > 200);
    }

    #[test]
    fn generate_buckets_and_build_z_work_with_dummy_data() {
        use rust_decimal::Decimal;
        // define a tiny test data type that holds pre-built EntryDatum
        #[derive(Clone, Debug)]
        struct Dummy {
            entries: Vec<EntryDatum>,
        }

        // two seasons with different entries
        let d1 = Dummy {
            entries: vec![
                EntryDatum {
                    num: Decimal::from_f64(0.2).unwrap(),
                    val: Some(1.0),
                    hover: Some("a".to_string()),
                },
                EntryDatum {
                    num: Decimal::from_f64(0.7).unwrap(),
                    val: Some(2.0),
                    hover: Some("b".to_string()),
                },
            ],
        };
        let d2 = Dummy {
            entries: vec![EntryDatum {
                num: Decimal::from_f64(0.3).unwrap(),
                val: Some(3.0),
                hover: Some("c".to_string()),
            }],
        };

        let cmp_data = vec![
            ("season_one".to_string(), d1.clone()),
            ("season_two".to_string(), d2.clone()),
        ];

        // entries_fn just clones the entries
        let (edges, layouts) =
            generate_buckets_from_entries(&cmp_data, &|d: &Dummy| d.entries.clone());

        // we expect at least one edge
        assert!(!edges.is_empty());
        assert!(!layouts.is_empty());

        // now build z
        let (texts, z) = build_z_from_entries(&cmp_data, &layouts, &|d: &Dummy| d.entries.clone());

        // z rows == number of seasons
        assert_eq!(z.len(), 2);
        assert_eq!(texts.len(), 2);

        // every z row has total cols equal to sum(slots)
        let total_cols: usize = layouts.iter().map(|(_, s)| *s).sum();
        assert_eq!(z[0].len(), total_cols);
        assert_eq!(z[1].len(), total_cols);
    }
}

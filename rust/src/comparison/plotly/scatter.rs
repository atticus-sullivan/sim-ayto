use catppuccin::Flavor;
use plotly::{common::Mode, Layout, Scatter};
use serde::Serialize;

use crate::comparison::{plotly::layout::{plotly_new_plot, styled_axis}, CmpData};

/// Build a scatter plot HTML from a list of named series. Each series is
/// `(name, x_values, y_values, text_values)`.
pub fn scatter_from_series<Tx, Ty>(
    layout: &Layout,
    series: &Vec<(String, Vec<Tx>, Vec<Ty>, Vec<String>)>,
    mode: Mode,
) -> String
where
    Tx: Serialize + Clone + 'static,
    Ty: Serialize + Clone + 'static,
{
    let mut plot = plotly_new_plot();
    plot.set_layout(layout.clone());

    // add layout axis styling...
    // for each series push a Scatter trace with x,y,text
    for (name, xs, ys, texts) in series {
        let trace = Scatter::new(xs.clone(), ys.clone())
            .name(name)
            .text_array(texts.clone())
            .mode(mode.clone());
        plot.add_trace(trace);
    }

    plot.to_inline_html(None)
}

#[allow(clippy::too_many_arguments)]
pub fn build_scatter_plot<X, Y, FX, FY, FString>(
    cmp_data: &Vec<(String, CmpData)>,
    layout: &Layout,
    palette: &Flavor,
    title: &str,
    x_title: &str,
    y_title: &str,
    mode: Mode,
    x_fn: FX,
    y_fn: FY,
    text_fn: FString,
) -> String
where
    FX: Fn(&CmpData) -> Vec<X>,
    FY: Fn(&CmpData) -> Vec<Y>,
    FString: Fn(&CmpData) -> Vec<String>,
    X: Clone + Serialize + 'static,
    Y: Clone + Serialize + 'static,
{

    let render_layout = layout
            .clone()
            .title(title)
            .x_axis(styled_axis(palette, x_title, true))
            .y_axis(styled_axis(palette, y_title, false));

    let data = cmp_data.into_iter().map(|(name, cd)| (name.clone(), x_fn(cd), y_fn(cd), text_fn(cd))).collect::<Vec<_>>();
    scatter_from_series(
        &render_layout,
        &data,
        mode,
    )
}

#[cfg(test)]
mod tests {
    use crate::comparison::{plotly::layout::plotly_gen_layout, theme::lut_theme};

    use super::*;

    #[test]
    fn scatter_from_series_generates_html() {
        let palette = lut_theme(1);
        let layout = plotly_gen_layout(palette);
        let series = vec![
            ("A".to_string(), vec![1.0f64], vec![2.0f64], vec!["pt1".to_string()]),
            ("B".to_string(), vec![3.0f64], vec![4.0f64], vec!["pt2".to_string()]),
        ];
        let html = scatter_from_series(
            &layout,
            &series,
            Mode::Lines,
        );
        assert!(html.contains("<div"));
        assert!(html.contains("Plotly"));
    }
}

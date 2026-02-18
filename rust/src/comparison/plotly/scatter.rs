use catppuccin::Flavor;
use plotly::{common::Mode, Layout, Scatter};
use serde::Serialize;

use crate::comparison::plotly::layout::{plotly_new_plot, styled_axis};
use crate::comparison::data::CmpData;

/// Build a scatter plot HTML from a list of named series. Each series is
/// `(name, x_values, y_values, text_values)`.
#[allow(clippy::type_complexity)]
pub fn scatter_from_series<Tx, Ty>(
    layout: &Layout,
    series: &[(String, Vec<Tx>, Vec<Ty>, Vec<String>)],
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
    cmp_data: &[(String, CmpData)],
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

    let data = cmp_data
        .iter()
        .map(|(name, cd)| (name.clone(), x_fn(cd), y_fn(cd), text_fn(cd)))
        .collect::<Vec<_>>();
    scatter_from_series(&render_layout, &data, mode)
}

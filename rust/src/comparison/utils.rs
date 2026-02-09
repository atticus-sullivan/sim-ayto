use catppuccin::{Flavor, PALETTE};
use plotly::{
    common::{Mode, Title},
    Layout, Plot, Scatter,
};
use serde::Serialize;

use crate::comparison::CmpData;

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

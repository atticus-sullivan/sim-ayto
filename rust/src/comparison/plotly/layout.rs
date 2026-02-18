use catppuccin::Flavor;
use plotly::{common::Title, Layout, Plot};

/// Create a new Plotly `Plot` with default configuration used across the site
/// (responsive, no plotly logo, scroll zoom enabled).
pub(super) fn plotly_new_plot() -> Plot {
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

/// Create the base `Layout` used for plots, filled with colors from the given `palette`.
pub(crate) fn plotly_gen_layout(palette: Flavor) -> Layout {
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

/// Create a styled axis for the given `palette` and `title`.
///
/// `mirror` controls whether axis lines are mirrored to the opposite side.
pub(super) fn styled_axis(palette: &Flavor, title: &str, mirror: bool) -> plotly::layout::Axis {
    plotly::layout::Axis::new()
        .line_color(palette.colors.overlay0.hex.to_string())
        .grid_color(palette.colors.overlay1.hex.to_string())
        .zero_line_color(palette.colors.overlay2.hex.to_string())
        .title(Title::with_text(title))
        .mirror(mirror)
        .show_line(true)
}

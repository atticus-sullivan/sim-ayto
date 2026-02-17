use plotly::common::Mode;

use crate::comparison::plotly::{layout::plotly_gen_layout, scatter::build_scatter_plot};
use crate::comparison::theme::lut_theme;
use crate::comparison::CmpData;

/// Build the set of plots (scatter/heatmap) regarding information theory (knowledge (change) in bits) for the site.
///
/// `cmp_data` is expected to be a vector of `(ruleset_name, CmpData)` pairs.
/// Returns a vector of `(tab_title, inline_html_string)` ready to be embedded.
pub fn build_information_plots(
    cmp_data: &Vec<(String, CmpData)>,
    theme: u8,
) -> Vec<(String, String)> {
    let palette = lut_theme(theme);
    let layout = plotly_gen_layout(palette);

    vec![
        (
            "MN/MC".to_owned(),
            build_scatter_plot(
                cmp_data,
                &layout,
                &palette,
                "Matchingnight / matching ceremony",
                "#MB",
                "I [bit]",
                Mode::Lines,
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.num(|_| true, |_| false, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.bits_gained(|_| true, |_| false, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.comment(|_| true, |_| false, |_| false))
                        .collect()
                },
            ),
        ),
        (
            "MB/TB".to_owned(),
            build_scatter_plot(
                cmp_data,
                &layout,
                &palette,
                "Matchbox / truth booth",
                "#MN",
                "I [bit]",
                Mode::Lines,
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.num(|_| false, |_| true, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.bits_gained(|_| false, |_| true, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.comment(|_| false, |_| true, |_| false))
                        .collect()
                },
            ),
        ),
        (
            "Combined".to_owned(),
            build_scatter_plot(
                cmp_data,
                &layout,
                &palette,
                "Left possibilities",
                "#MB/#MN",
                "H [bit]",
                Mode::Lines,
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.num(|_| true, |_| true, |_| true))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.bits_left_after(|_| true, |_| true, |_| true))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.comment(|_| true, |_| true, |_| true))
                        .collect()
                },
            ),
        ),
    ]
}

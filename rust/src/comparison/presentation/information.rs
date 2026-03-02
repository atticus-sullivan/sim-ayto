/// This renders the plots which show information regarding the amount of information/uncertainty
/// left or the amount of information gained over the course of time
use plotly::common::Mode;

use crate::comparison::data::CmpData;
use crate::comparison::plotly::layout::plotly_gen_layout;
use crate::comparison::plotly::scatter::build_scatter_plot;
use crate::comparison::theme::lut_theme;

/// Build the set of plots (scatter/heatmap) regarding information theory (knowledge (change) in bits) for the site.
///
/// `cmp_data` is expected to be a vector of `(ruleset_name, CmpData)` pairs.
/// Returns a vector of `(tab_title, inline_html_string)` ready to be embedded.
pub(crate) fn plots(cmp_data: &[(String, CmpData)], theme: u8) -> Vec<(String, String)> {
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
                        .filter_map(|i| i.num_unified(|_| true, |_| true, |_| true))
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

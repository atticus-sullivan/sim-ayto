/// This renders the plots which show information regarding the amount of lights over the course of
/// time

use plotly::common::Mode;

use crate::comparison::plotly::heatmap::{build_heatmap_plot, EntryDatum};
use crate::comparison::plotly::scatter::build_scatter_plot;
use crate::comparison::plotly::layout::plotly_gen_layout;

use crate::comparison::theme::lut_theme;
use crate::comparison::data::CmpData;
use crate::constraint::eval_types::EvalEvent;

/// Build plots about "lights" (lighting related evaluation metrics).
///
/// Accepts the comparison dataset and a `theme` index. Returns
/// pairs `(tab label, plot HTML)` to be embedded in the generated pages.
pub fn plots(cmp_data: &Vec<(String, CmpData)>, theme: u8) -> Vec<(String, String)> {
    let palette = lut_theme(theme);
    let layout = plotly_gen_layout(palette);

    vec![
        (
            "#Lights MB/TB".to_owned(),
            build_scatter_plot(
                cmp_data,
                &layout,
                &palette,
                "#Lights -- MB",
                "#MB",
                "#Lights",
                Mode::LinesMarkers,
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.num(|_| false, |x| x.lights_total.is_some(), |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.lights_total(|_| false, |_| true, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| {
                            i.comment(|_| false, |x| x.lights_total.is_some(), |_| false)
                        })
                        .collect()
                },
            ),
        ),
        (
            "#Lights MN/MC".to_owned(),
            build_scatter_plot(
                cmp_data,
                &layout,
                &palette,
                "#Lights -- MN",
                "#MN",
                "#Lights",
                Mode::LinesMarkers,
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.num(|x| x.lights_total.is_some(), |_| false, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.lights_total(|_| true, |_| false, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| {
                            i.comment(|x| x.lights_total.is_some(), |_| false, |_| false)
                        })
                        .collect()
                },
            ),
        ),
        (
            "#Lights-known MN/MC".to_owned(),
            build_scatter_plot(
                cmp_data,
                &layout,
                &palette,
                "#Lights - known_lights -- MN",
                "#MN",
                "#Lights - known_lights",
                Mode::LinesMarkers,
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.num(|x| x.lights_total.is_some(), |_| false, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| i.new_lights(|_| true, |_| false, |_| false))
                        .collect()
                },
                |cd| {
                    cd.eval_data
                        .iter()
                        .filter_map(|i| {
                            i.comment(|x| x.lights_total.is_some(), |_| false, |_| false)
                        })
                        .collect()
                },
            ),
        ),
        (
            "HM #Lights MB".to_owned(),
            build_heatmap_plot(cmp_data, &layout, &palette, "Heatmap", "#MB", "Lights", |cd| {
                cd.eval_data
                    .iter()
                    .filter_map(|i| match i {
                        EvalEvent::MB(e) => Some(EntryDatum {
                            num: e.num,
                            val: e.lights_total.map(|v| v as f64),
                            hover: Some(e.comment.clone()),
                        }),
                        EvalEvent::MN(_) => None,
                        EvalEvent::Initial(_) => None,
                    })
                    .collect()
            }),
        ),
        (
            "HM #Lights MN".to_owned(),
            build_heatmap_plot(cmp_data, &layout, &palette, "Heatmap", "#MN", "Lights", |cd| {
                cd.eval_data
                    .iter()
                    .filter_map(|i| match i {
                        EvalEvent::MN(e) => Some(EntryDatum {
                            num: e.num,
                            val: e.lights_total.map(|v| v as f64),
                            hover: Some(e.comment.clone()),
                        }),
                        EvalEvent::MB(_) => None,
                        EvalEvent::Initial(_) => None,
                    })
                    .collect()
            }),
        ),
        (
            "HM #Lights-known MN".to_owned(),
            build_heatmap_plot(cmp_data, &layout, &palette, "Heatmap", "#MN", "Lights", |cd| {
                cd.eval_data
                    .iter()
                    .filter_map(|i| match i {
                        EvalEvent::MN(e) => Some(EntryDatum {
                            num: e.num,
                            val: e.lights_total.map(|v| (v - e.lights_known_before) as f64),
                            hover: Some(e.comment.clone()),
                        }),
                        EvalEvent::MB(_) => None,
                        EvalEvent::Initial(_) => None,
                    })
                    .collect()
            }),
        ),
    ]
}

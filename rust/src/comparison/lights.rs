use crate::comparison::{
    utils::{build_heatmap_plot, build_scatter_plot, lut_theme, plotly_gen_layout, EntryDatum},
    CmpData,
};
use plotly::common::Mode;

pub fn build_light_plots(cmp_data: &Vec<(String, CmpData)>, theme: u8) -> Vec<(String, String)> {
    let palette = lut_theme(theme);
    let layout = plotly_gen_layout(palette);

    // TODO: use hugo translation strings? (might not work with hextra tabs)
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
                    cd.mb
                        .iter()
                        .filter_map(|i| i.lights_total.map(|_| i.num))
                        .collect()
                },
                |cd| cd.mb.iter().filter_map(|i| i.lights_total).collect(),
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
                    cd.mn
                        .iter()
                        .filter_map(|i| i.lights_total.map(|_| i.num))
                        .collect()
                },
                |cd| cd.mn.iter().filter_map(|i| i.lights_total).collect(),
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
                    cd.mn
                        .iter()
                        .filter_map(|i| i.lights_total.map(|_| i.num))
                        .collect()
                },
                |cd| {
                    cd.mn
                        .iter()
                        .filter_map(|i| i.lights_total.map(|lt| lt - i.lights_known_before))
                        .collect()
                },
            ),
        ),
        (
            "HM #Lights MB".to_owned(),
            build_heatmap_plot(cmp_data, &layout, &palette, "Heatmap", "#MB", |cd| {
                cd.mb
                    .iter()
                    .enumerate()
                    .map(|(i, e)| EntryDatum {
                        num: e.num,
                        val: e.lights_total.map(|v| v as f64),
                        hover: cd.info.get(i).map(|x| x.comment.clone()),
                    })
                    .collect()
            }),
        ),
        (
            "HM #Lights MN".to_owned(),
            build_heatmap_plot(cmp_data, &layout, &palette, "Heatmap", "#MN", |cd| {
                cd.mn
                    .iter()
                    .enumerate()
                    .map(|(i, e)| EntryDatum {
                        num: e.num,
                        val: e.lights_total.map(|v| v as f64),
                        hover: cd.info.get(i).map(|x| x.comment.clone()),
                    })
                    .collect()
            }),
        ),
        (
            "HM #Lights-known MN".to_owned(),
            build_heatmap_plot(cmp_data, &layout, &palette, "Heatmap", "#MN", |cd| {
                cd.mn
                    .iter()
                    .enumerate()
                    .map(|(i, e)| EntryDatum {
                        num: e.num,
                        val: e.lights_total.map(|v| (v - e.lights_known_before) as f64),
                        hover: cd.info.get(i).map(|x| x.comment.clone()),
                    })
                    .collect()
            }),
        ),
    ]
}

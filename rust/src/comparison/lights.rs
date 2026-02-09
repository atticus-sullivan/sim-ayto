use plotly::{
    common::{Mode,},
};

use crate::comparison::{
    utils::{lut_theme, plotly_gen_layout, build_scatter_plot},
    CmpData,
};

pub fn build_light_plots(cmp_data: &Vec<(String, CmpData)>, theme: u8) -> Vec<(String, String)> {
    let palette = lut_theme(theme);
    let layout = plotly_gen_layout(palette);

    // TODO: use hugo translation strings? (might not work with hextra tabs)
    vec![
        (
            "#Lights MB/TB".to_owned(),
            build_scatter_plot(
                &cmp_data,
                &layout,
                &palette,
                "#Lights -- MB",
                "#MB",
                "#Lights",
                Mode::LinesMarkers,
                |cd|
            cd.mb
                .iter()
                .filter_map(|i| i.lights_total.map(|_| i.num))
                .collect(),
                |cd|
            cd.mb.iter().filter_map(|i| i.lights_total).collect(),
            )
        ),
        (
            "#Lights MN/MC".to_owned(),
            build_scatter_plot(
                &cmp_data,
                &layout,
                &palette,
                "#Lights -- MN",
                "#MN",
                "#Lights",
                Mode::LinesMarkers,
                |cd|
            cd.mn
                .iter()
                .filter_map(|i| i.lights_total.map(|_| i.num))
                .collect(),
                |cd|
            cd.mn.iter().filter_map(|i| i.lights_total).collect(),
            )
        ),
        (
            "#Lights-known MN/MC".to_owned(),
            build_scatter_plot(
                &cmp_data,
                &layout,
                &palette,
                "#Lights - known_lights -- MN",
                "#MN",
                "#Lights - known_lights",
                Mode::LinesMarkers,
                |cd|
            cd.mn
                .iter()
                .filter_map(|i| i.lights_total.map(|_| i.num))
                .collect(),
                |cd|
            cd.mn
                .iter()
                .filter_map(|i| i.lights_total.map(|lt| lt - i.lights_known_before))
                .collect(),
            )
        ),
    ]
}

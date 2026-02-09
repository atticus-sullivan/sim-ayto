use plotly::common::Mode;

use crate::comparison::{
    utils::{build_scatter_plot, lut_theme, plotly_gen_layout},
    CmpData,
};

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
                |cd| cd.mn.iter().map(|i| i.num).collect(),
                |cd| cd.mn.iter().map(|i| i.bits_gained).collect(),
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
                |cd| cd.mb.iter().map(|i| i.num).collect(),
                |cd| cd.mb.iter().map(|i| i.bits_gained).collect(),
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
                |cd| cd.info.iter().map(|i| i.num).collect(),
                |cd| cd.info.iter().map(|i| i.bits_left).collect(),
            ),
        ),
    ]
}

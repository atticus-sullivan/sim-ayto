mod data;
/// This complete module contains the functionality to
/// 1. collect stored data about seasons from disk
/// 2. render them in plots (plotly) and tables (markdown using hextra/hugo additions)
///
/// This module is the only one needed to the outside (`write_pages`). Everything is plugged together here.
mod plotly;
mod presentation;
mod theme;

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::comparison::data::gather_cmp_data;
use crate::comparison::presentation::information;
use crate::comparison::presentation::lights;
use crate::comparison::presentation::ruleset;
use crate::comparison::presentation::summary;

/// Configuration for producing one page (path + filtering).
///
/// - `link_title` is the human/title used on the index page.
/// - `base_path` is the path (file *without extension*) where the MD will be written.
/// - `ruleset_filter` selects which ruleset directories from `./data` should be included.
struct PageConfig<'a> {
    link_title: &'a str,
    base_path: PathBuf,
    ruleset_filter: fn(&str) -> bool,
}

/// Language selector used when rendering localized strings in the markdown output.
#[derive(Copy, Clone)]
pub(super) enum Language {
    De,
    En,
}

impl Language {
    /// Format a boolean as a localized "Yes/No" string.
    pub fn format_bool_yes_no(&self, val: bool) -> &str {
        match self {
            Language::De => {
                if val {
                    "Ja"
                } else {
                    "Nein"
                }
            }
            Language::En => {
                if val {
                    "Yes"
                } else {
                    "No"
                }
            }
        }
    }

    /// Return the number formatting `Locale` used for formatting monetary amounts / numbers.
    pub fn number_formatting(&self) -> num_format::Locale {
        match self {
            Language::De => num_format::Locale::de,
            Language::En => num_format::Locale::en,
        }
    }
}

/// Generate the full set of pages (DE and US/UK sections) using the given theme IDs.
///
/// `html_path_de` and `html_path_us` point to the base filenames where the
/// generated markdown will be written (file extension is added by `write_page`).
/// `theme_light` and `theme_dark` select the catppuccin palette numbers used for
/// light and dark versions of the Plotly plots.
///
/// This is the high-level entrypoint used by your site generator.
pub fn write_pages(
    html_path_de: &Path,
    html_path_us: &Path,
    theme_light: u8,
    theme_dark: u8,
) -> Result<()> {
    let pages = [
        PageConfig {
            link_title: "DE",
            base_path: html_path_de.to_path_buf(),
            ruleset_filter: |e| e.starts_with("de"),
        },
        PageConfig {
            link_title: "US + UK",
            base_path: html_path_us.to_path_buf(),
            ruleset_filter: |e| e.starts_with("us") || e.starts_with("uk"),
        },
    ];

    for page in pages {
        let data = gather_cmp_data(page.ruleset_filter)?;

        for lang in [Language::De, Language::En] {
            let mut plots_light = information::plots(&data, theme_light);
            plots_light.append(&mut lights::plots(&data, theme_light));
            let html_light = build_graph_hextra_tabs(&plots_light);

            let mut plots_dark = information::plots(&data, theme_dark);
            plots_dark.append(&mut lights::plots(&data, theme_dark));
            let html_dark = build_graph_hextra_tabs(&plots_dark);

            let md_ruleset_tab = ruleset::tab_md(&data);
            let md_summary_tab = summary::tab_md(&data, lang);

            write_page(
                &page,
                lang,
                &md_ruleset_tab,
                &md_summary_tab,
                &html_light,
                &html_dark,
            )?;
        }
    }
    Ok(())
}

/// Render a single page using the provided `PageConfig` and language.
///
/// This writes a Markdown file (`.md` for German, `en.md` for English) to the
/// `cfg.base_path` with the provided `md_ruleset_tab`, `md_summary_tab` and
/// plot HTMLs (`plot_light` / `plot_dark`).
fn write_page(
    cfg: &PageConfig<'_>,
    lang: Language,
    md_ruleset_tab: &str,
    md_summary_tab: &str,
    plot_light: &str,
    plot_dark: &str,
) -> std::io::Result<()> {
    let mut path = cfg.base_path.clone();

    match lang {
        Language::De => path.set_extension("md"),
        Language::En => path.set_extension("en.md"),
    };

    let content = match lang {
        Language::De => format!(
            include_str!("../templates/page_de.md"),
            cfg.link_title, md_ruleset_tab, md_summary_tab, plot_light, plot_dark
        ),
        Language::En => format!(
            include_str!("../templates/page_en.md"),
            cfg.link_title, md_ruleset_tab, md_summary_tab, plot_light, plot_dark
        ),
    };

    std::fs::write(path, content)
}

/// Wrap the provided plot HTML fragments into a Hugo/shortcode tabbed container
/// expected by the site (includes the Plotly JS CDN script tag).
///
/// The output is the combined HTML string that is embedded in the page's markdown.
fn build_graph_hextra_tabs(plots: &[(String, String)]) -> String {
    let dat = plots
        .iter()
        .map(|(_, i)| "{{% tab %}}".to_string() + i + "{{% /tab %}}")
        .fold(String::new(), |a, b| a + &b);
    let tab_items = plots
        .iter()
        .map(|i| i.0.clone())
        .collect::<Vec<_>>()
        .join(",");

    format!(
        r#"<script src="https://cdn.plot.ly/plotly-3.3.1.min.js"></script>
{{{{< tabs items="{tab_items}" >}}}}
{}
{{{{< /tabs >}}}}"#,
        dat
    )
}

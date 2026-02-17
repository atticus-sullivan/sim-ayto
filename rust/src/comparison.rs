mod information;
mod lights;
mod ruleset;
mod summary;
mod theme;
mod plotly;

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::de::DeserializeOwned;
use walkdir::WalkDir;

use crate::comparison::information::build_information_plots;
use crate::comparison::lights::build_light_plots;
use crate::comparison::ruleset::ruleset_tab_md;
use crate::comparison::summary::summary_tab_md;
use crate::constraint::eval_types::{EvalData, EvalEvent, SumCounts};
use crate::game::Game;

/// Compact comparison data for a ruleset / season used by the comparison pages.
///
/// - `eval_data` contains the chronological evaluation events (MB/MN/Initial).
/// - `cnts` contains aggregated counts and summary metrics for the season.
/// - `game` holds the parsed ruleset/game specification.
#[derive(Debug)]
pub struct CmpData {
    pub eval_data: Vec<EvalEvent>,
    pub cnts: SumCounts,
    pub game: Game,
}

// fn read_csv_data<T: DeserializeOwned>(fn_param: &str, path: &Path) -> Result<Option<Vec<T>>> {
//     if !path.join(fn_param).exists() {
//         return Ok(None);
//     }
//     let mut field: Vec<T> = Vec::new();
//     let mut rdr = csv::ReaderBuilder::new()
//         .delimiter(b',')
//         .has_headers(false)
//         .from_path(path.join(fn_param))?;
//     for result in rdr.deserialize() {
//         let record: T = result?;
//         field.push(record);
//     }
//     Ok(Some(field))
// }

/// Read JSON data from `path.join(fn_param)` and deserialize into `T`.
///
/// Returns `Ok(None)` if the file does not exist. Surface any I/O or
/// deserialization errors via the `Result`.
fn read_json_data<T: DeserializeOwned>(fn_param: &str, path: &Path) -> Result<Option<T>> {
    if !path.join(fn_param).exists() {
        return Ok(None);
    }
    let file = File::open(path.join(fn_param))?;
    let reader = BufReader::new(file);
    let dat: T = serde_json::from_reader(reader)?;
    Ok(Some(dat))
}

/// Parse the ruleset/game YAML file located at `fn_path` (without extension).
///
/// The function expects `<fn_path>.yaml` to exist and uses the game's parsing
/// facilities (`GameParse`) to produce a `Game`. This helper will panic on parse
/// error in current behavior (keeps earlier behavior intact).
fn read_yaml_spec(mut fn_path: PathBuf) -> Result<Game> {
    fn_path.set_extension("yaml");
    let gp = crate::game::parse::GameParse::new_from_yaml(fn_path.as_path(), None)
        .expect("Parsing failed");
    gp.finalize_parsing(Path::new("/tmp/"), false)
}

/// Scan `./data` and parse comparison files for the selected directories.
///
/// `filter_dirs` is a callback used to select which subdirectories of `./data`
/// should be included (e.g. `|s| s.starts_with("de")`). For each accepted
/// directory the function reads the YAML spec and the `stats.json` produced by
/// the data pipeline and returns a sorted `Vec<(ruleset_name, CmpData)>`.
///
/// This function walks the real filesystem and is integration-y; see
/// `gather_cmp_data_from` (test-friendly wrapper) in the suggestions section.
pub fn gather_cmp_data(filter_dirs: fn(&str) -> bool) -> Result<Vec<(String, CmpData)>> {
    let mut ret = vec![];

    // loop over the data directories selected/filterd by filter_dirs
    for entry in WalkDir::new("./data")
        .max_depth(1)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            e.file_name().to_str().is_some_and(filter_dirs)
                && e.metadata().is_ok_and(|e| e.is_dir())
        })
        .filter_map(Result::ok)
    {
        let game = read_yaml_spec(entry.path().join(entry.file_name()))?;

        let eval_data: EvalData = match read_json_data("stats.json", entry.path())? {
            Some(x) => x,
            None => continue,
        };

        ret.push((
            entry.file_name().to_str().unwrap_or("unknown").to_owned(),
            CmpData {
                eval_data: eval_data.events,
                cnts: eval_data.cnts,
                game,
            },
        ));
    }
    ret.sort_by_key(|i| i.0.clone());
    Ok(ret)
}

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
enum Language {
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
            let mut plots_light = build_information_plots(&data, theme_light);
            plots_light.append(&mut build_light_plots(&data, theme_light));
            let html_light = build_graph_hextra_tabs(&plots_light);

            let mut plots_dark = build_information_plots(&data, theme_dark);
            plots_dark.append(&mut build_light_plots(&data, theme_dark));
            let html_dark = build_graph_hextra_tabs(&plots_dark);

            let md_ruleset_tab = ruleset_tab_md(&data);
            let md_summary_tab = summary_tab_md(&data, lang);

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

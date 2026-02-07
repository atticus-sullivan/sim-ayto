/*
sim_ayto
Copyright (C) 2024  Lukas Heindl

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

mod constraint;
mod game;
mod graph;
mod ruleset;
mod ruleset_data;
mod tree;

use crate::game::Game;

use clap::{Parser, Subcommand};
use game::DumpMode;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

// TODO code review (try with chatGPT)

type MatchingS = HashMap<String, Vec<String>>;
type Matching = Vec<Vec<u8>>;
type MapS = HashMap<String, String>;
type Map = HashMap<u8, u8>;
type Lut = HashMap<String, usize>;
type Rename = HashMap<String, String>;

type Rem = (Vec<Vec<u128>>, u128);

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Sim {
        /// The path to the file to read
        yaml_path: PathBuf,

        #[arg(short = 'c', long = "color")]
        colored: bool,

        #[arg(long = "transpose")]
        transpose_tabs: bool,

        #[arg(short = 'o', long = "output")]
        stem: PathBuf,

        #[arg(
            long = "dump",
            help = "dump all combinations ({winning,all}{nums,names} in the end of the simulation"
        )]
        dump: Option<DumpMode>,

        #[arg(
            long = "full",
            help = "print all probabilities instead of just the topX below the tables"
        )]
        full: bool,

        #[arg(
            long = "use-cache",
            help = "Normally the optimal cache is used. This influences the output. Thus, this flag can be used to base the simulation on an not optimal cache"
        )]
        use_cache: Option<String>,
    },
    Check {
        /// The path to the file to read
        yaml_path: PathBuf,
    },
    Graph {
        #[arg(short = 'l', long = "theme-light", default_value = "1")]
        theme_light: u8,
        #[arg(short = 'd', long = "theme-dark", default_value = "3")]
        theme_dark: u8,
        html_path_de: PathBuf,
        html_path_us: PathBuf,
    },
    Cache {
        yaml_path: PathBuf,
    },
}

fn main() {
    let args = Cli::parse();

    match args.cmd {
        Commands::Sim {
            yaml_path,
            colored: _,
            transpose_tabs,
            stem,
            dump,
            full,
            use_cache,
        } => {
            let gp = crate::game::parse::GameParse::new_from_yaml(&yaml_path, use_cache.clone())
                .expect("Parsing failed");
            let mut g = gp.finalize_parsing(&stem).expect("processing game failed");
            let start = Instant::now();
            g.sim(transpose_tabs, dump, full, use_cache).unwrap();
            println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
        }
        Commands::Cache { yaml_path } => {
            let gp = crate::game::parse::GameParse::new_from_yaml(&yaml_path, Some("".to_string()))
                .expect("Parsing failed");
            gp.show_caches().expect("Failed evaluating caches");
        }
        Commands::Check { yaml_path } => {
            let gp = crate::game::parse::GameParse::new_from_yaml(&yaml_path, None)
                .expect("Parsing failed");
            gp.finalize_parsing(std::path::Path::new(".trash"))
                .expect("processing game failed");
        }
        Commands::Graph {
            theme_light,
            theme_dark,
            html_path_de,
            html_path_us,
        } => {
            // obtain the graphs for the german seasons
            let html_content_light =
                graph::build_stats_graph(|e| e.starts_with("de"), theme_light).unwrap();
            let html_content_dark =
                graph::build_stats_graph(|e| e.starts_with("de"), theme_dark).unwrap();
            let md_ruleset_tab = graph::ruleset_tab_md(|e| e.starts_with("de")).unwrap();

            let md_summary_tab = graph::summary_tab_md(|e| e.starts_with("de")).unwrap();

            // write the output localized for german language
            let mut html_path_local = html_path_de.clone();
            std::fs::write(&html_path_local, format!(r#"---
linkTitle: 'DE'
weight: 1
toc: false

---
# Anmerkungen
- [generelle Hinweise](/#noch-mehr-details) zu den Metriken (`H [bit]` und `I [bit]`).
  Letztlich ist `I` aber einfach nur eine Größe wieviel neue Informationen das gebracht hat und `H` nur eine andere Schreibweise für die Anzahl an übrigen Möglichkeiten.
- durch einen einfachen Klick in der Legende kann man einzelne Linien ausblenden
- durch einen Doppelklick in der Legende kann man alle Linien, außer der ausgewählten ausblenden
- ansonsten sind die Plots (bzgl Zoom/Verschieben) eigentlich ziemlich straight forward

# Regeln je Staffel
{}

# Zusammenfassung
{}

# Plots
<div class="plot-container plot-light">
{}
</div>
<div class="plot-container plot-dark">
{}
</div>
<script>
document.addEventListener("DOMContentLoaded", () => {{
    document.querySelectorAll('.hextra-tabs-toggle').forEach(tabButton => {{
        tabButton.addEventListener("click", () => {{
            window.dispatchEvent(new Event('resize'));
        }});
    }});
}});
</script>
"#,  &md_ruleset_tab, &md_summary_tab, &html_content_light, &html_content_dark)).unwrap();

            // write the output localized for english language
            html_path_local.set_extension("en.md");
            std::fs::write(&html_path_local, format!(r#"---
linkTitle: 'DE'
weight: 1
toc: false

---
# Remarks
- [general information](/en/#more-details) regarding the metrics (`H [bit]` and `I [bit]`).
  In the end `I` is just a measure for how much new information was gained and `H` just a different notation for the amount of left possibilities.
- with a single-click on items in the legend you can hide that line in the plot
- with a double-click on an item in the legend you can hide all other lines in the plot
- other things like zooming or panning of the plots should be pretty straight forward

# Ruleset per Season
{}

# Summary
{}

# Plots
<div class="plot-container plot-light">
{}
</div>
<div class="plot-container plot-dark">
{}
</div>
<script>
document.addEventListener("DOMContentLoaded", () => {{
    document.querySelectorAll('.hextra-tabs-toggle').forEach(tabButton => {{
        tabButton.addEventListener("click", () => {{
            window.dispatchEvent(new Event('resize'));
        }});
    }});
}});
</script>
"#,  &md_ruleset_tab, &md_summary_tab, &html_content_light, &html_content_dark)).unwrap();

            // obtain the graphs for the us+uk seasons
            let html_content_light = graph::build_stats_graph(
                |e| e.starts_with("uk") || e.starts_with("us"),
                theme_light,
            )
            .unwrap();
            let html_content_dark = graph::build_stats_graph(
                |e| e.starts_with("uk") || e.starts_with("us"),
                theme_dark,
            )
            .unwrap();
            let md_ruleset_tab =
                graph::ruleset_tab_md(|e| e.starts_with("uk") || e.starts_with("us")).unwrap();

            let md_summary_tab =
                graph::summary_tab_md(|e| e.starts_with("uk") || e.starts_with("us")).unwrap();

            // write the output localized for german language
            html_path_local = html_path_us.clone();
            std::fs::write(&html_path_local, format!(r#"---
linkTitle: 'US + UK'
weight: 1
toc: false
---

# Anmerkungen
- [generelle Hinweise](/#noch-mehr-details) zu den Metriken (`H [bit]` und `I [bit]`).
  Letztlich ist `I` aber einfach nur eine Größe wieviel neue Informationen das gebracht hat und `H` nur eine andere Schreibweise für die Anzahl an übrigen Möglichkeiten.
- durch einen einfachen Klick in der Legende kann man einzelne Linien ausblenden
- durch einen Doppelklick in der Legende kann man alle Linien, außer der ausgewählten ausblenden
- ansonsten sind die Plots (bzgl Zoom/Verschieben) eigentlich ziemlich straight forward


# Regeln je Staffel
{}

# Zusammenfassung
{}

# Plots
<div class="plot-container plot-light">
{}
</div>
<div class="plot-container plot-dark">
{}
</div>
<script>
document.addEventListener("DOMContentLoaded", () => {{
document.querySelectorAll('.hextra-tabs-toggle').forEach(tabButton => {{
    tabButton.addEventListener("click", () => {{
        window.dispatchEvent(new Event('resize'));
    }});
}});
}});
</script>
"#,  &md_ruleset_tab, &md_summary_tab, &html_content_light, &html_content_dark)).unwrap();

            // write the output localized for english language
            html_path_local.set_extension("en.md");
            std::fs::write(&html_path_local, format!(r#"---
linkTitle: 'US + UK'
weight: 1
toc: false

---
# Remarks
- [general information](/en/#more-details) regarding the metrics (`H [bit]` and `I [bit]`).
  In the end `I` is just a measure for how much new information was gained and `H` just a different notation for the amount of left possibilities.
- with a single-click on items in the legend you can hide that line in the plot
- with a double-click on an item in the legend you can hide all other lines in the plot
- other things like zooming or panning of the plots should be pretty straight forward

# Ruleset per Season
{}

# Summary
{}

# Plots
<div class="plot-container plot-light">
{}
</div>
<div class="plot-container plot-dark">
{}
</div>
<script>
document.addEventListener("DOMContentLoaded", () => {{
    document.querySelectorAll('.hextra-tabs-toggle').forEach(tabButton => {{
        tabButton.addEventListener("click", () => {{
            window.dispatchEvent(new Event('resize'));
        }});
    }});
}});
</script>
"#,  &md_ruleset_tab, &md_summary_tab, &html_content_light, &html_content_dark)).unwrap();
        }
    }
}


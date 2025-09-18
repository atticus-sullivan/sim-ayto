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

        #[arg(long = "dump")]
        dump: Option<DumpMode>,

        #[arg(long = "full")]
        full: bool,
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
        } => {
            let mut g = Game::new_from_yaml(&yaml_path, &stem).expect("Parsing failed");
            let start = Instant::now();
            g.sim(transpose_tabs, dump, full).unwrap();
            println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
        }
        Commands::Check { yaml_path } => {
            Game::new_from_yaml(&yaml_path, std::path::Path::new(".trash"))
                .expect("Parsing failed!");
        }
        Commands::Graph {
            theme_light,
            theme_dark,
            html_path_de,
            html_path_us,
        } => {
            // obtain the graphs for the german seasons
            let html_content_light =
                graph::build_stats_graph(|e| e.starts_with("de"), theme_light, "de-light").unwrap();
            let html_content_dark =
                graph::build_stats_graph(|e| e.starts_with("de"), theme_dark, "de-dark").unwrap();
            let md_ruleset_tab = graph::ruleset_tab_md(|e| e.starts_with("de")).unwrap();

            // write the output localized for german language
            let mut html_path_local = html_path_de.clone();
            std::fs::write(&html_path_local, format!(r#"---
title: 'DE'
weight: 1
bookToc: false

---
# Anmerkungen
- [generelle Hinweise](/sim-ayto#noch-mehr-details) zu den Metriken (`H [bit]` und `I [bit]`).
  Letztlich ist `I` aber einfach nur eine Größe wieviel neue Informationen das gebracht hat und `H` nur eine andere Schreibweise für die Anzahl an übrigen Möglichkeiten.
- durch einen einfachen Klick in der Legende kann man einzelne Linien ausblenden
- durch einen Doppelklick in der Legende kann man alle Linien, außer der ausgewählten ausblenden
- ansonsten sind die Plots (bzgl Zoom/Verschieben) eigentlich ziemlich straight forward

# Regeln je Staffel
{}

# Plots
<div class="theme-specific-content">
<div class="light-theme-content" style="display: none;" data-theme="light">
{}
</div>
<div class="dark-theme-content" style="display: none;" data-theme="dark">
{}
</div>
</div>
"#,  &md_ruleset_tab, &html_content_light, &html_content_dark)).unwrap();

            // write the output localized for english language
            html_path_local.set_extension("en.md");
            std::fs::write(&html_path_local, format!(r#"---
title: 'DE'
weight: 1
bookToc: false

---
# Remarks
- [general information](/sim-ayto/en/#more-details) regarding the metrics (`H [bit]` and `I [bit]`).
  In the end `I` is just a measure for how much new information was gained and `H` just a different notation for the amount of left possibilities.
- with a single-click on items in the legend you can hide that line in the plot
- with a double-click on an item in the legend you can hide all other lines in the plot
- other things like zooming or panning of the plots should be pretty straight forward

# Ruleset per Season
{}

# Plots
<div class="theme-specific-content">
<div class="light-theme-content" style="display: none;" data-theme="light">
{}
</div>
<div class="dark-theme-content" style="display: none;" data-theme="dark">
{}
</div>
</div>
"#,  &md_ruleset_tab, &html_content_light, &html_content_dark)).unwrap();

            // obtain the graphs for the us+uk seasons
            let html_content_light = graph::build_stats_graph(
                |e| e.starts_with("uk") || e.starts_with("us"),
                theme_light,
                "uk-light",
            )
            .unwrap();
            let html_content_dark = graph::build_stats_graph(
                |e| e.starts_with("uk") || e.starts_with("us"),
                theme_dark,
                "uk-dark",
            )
            .unwrap();
            let md_ruleset_tab =
                graph::ruleset_tab_md(|e| e.starts_with("uk") || e.starts_with("us")).unwrap();

            // write the output localized for german language
            html_path_local = html_path_us.clone();
            std::fs::write(&html_path_local, format!(r#"---
title: 'US + UK'
weight: 1
bookToc: false
---

# Anmerkungen
- [generelle Hinweise](/sim-ayto#noch-mehr-details) zu den Metriken (`H [bit]` und `I [bit]`).
  Letztlich ist `I` aber einfach nur eine Größe wieviel neue Informationen das gebracht hat und `H` nur eine andere Schreibweise für die Anzahl an übrigen Möglichkeiten.
- durch einen einfachen Klick in der Legende kann man einzelne Linien ausblenden
- durch einen Doppelklick in der Legende kann man alle Linien, außer der ausgewählten ausblenden
- ansonsten sind die Plots (bzgl Zoom/Verschieben) eigentlich ziemlich straight forward


# Regeln je Staffel
{}

# Plots
<div class="theme-specific-content">
<div class="light-theme-content" style="display: none;" data-theme="light">
{}
</div>
<div class="dark-theme-content" style="display: none;" data-theme="dark">
{}
</div>
</div>
"#,  &md_ruleset_tab, &html_content_light, &html_content_dark)).unwrap();

            // write the output localized for english language
            html_path_local.set_extension("en.md");
            std::fs::write(&html_path_local, format!(r#"---
title: 'US + UK'
weight: 1
bookToc: false

---
# Remarks
- [general information](/sim-ayto/en/#more-details) regarding the metrics (`H [bit]` and `I [bit]`).
  In the end `I` is just a measure for how much new information was gained and `H` just a different notation for the amount of left possibilities.
- with a single-click on items in the legend you can hide that line in the plot
- with a double-click on an item in the legend you can hide all other lines in the plot
- other things like zooming or panning of the plots should be pretty straight forward

# Ruleset per Season
{}

# Plots
<div class="theme-specific-content">
<div class="light-theme-content" style="display: none;" data-theme="light">
{}
</div>
<div class="dark-theme-content" style="display: none;" data-theme="dark">
{}
</div>
</div>
"#,  &md_ruleset_tab, &html_content_light, &html_content_dark)).unwrap();
        }
    }
}

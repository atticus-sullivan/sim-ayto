// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module serves as a CLI to the simulation/calculation/evaluation/reporting and comparison
//! code.

use ayto::comparison;
use ayto::game::cache::{CacheModeArg, CacheModeFallback, CacheSpec};
use ayto::game::cache_report::show_caches;
use ayto::game::parse::GameParse;

use ayto::dump_mode::DumpMode;
use ayto::ignore_ops::IgnoreOps;
use ayto::iterstate::IterState;
use ayto::progressbar::ProgressBar;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

/// Specifies the complete CLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// subcommands of the binary
    #[command(subcommand)]
    cmd: Commands,
}

/// Specifies the subcommands available on the CLI
#[derive(Subcommand, Debug)]
enum Commands {
    /// Simulate/Calculate an existing game-config with all the constraints and show the stats
    Sim {
        /// allows to disable writing .dot files
        #[arg(long = "no-tree-output", action)]
        no_tree_output: bool,

        // TODO: make possible to specify multiple values if makes sense (multiple ignoreOps
        // available)
        /// specify which operations/events shall be ignored
        #[arg(long = "ignore")]
        ignore: IgnoreOps,

        /// The path to the file to read as game-config
        yaml_path: PathBuf,

        // #[arg(short = 'c', long = "color")]
        // colored: bool,

        /// option to transpose the generated tables (swaps set_a with set_b)
        /// swapping via the config is not possible as it matters for the maps on which side
        /// duplicates are possible to store
        #[arg(long = "transpose")]
        transpose_tabs: bool,

        /// base-path where to write the outputs (.dot, .json)
        #[arg(short = 'o', long = "output")]
        stem: PathBuf,

        /// whether and if so how to dump all the remaining possible solutions
        #[arg(
            long = "dump",
            help = "dump all combinations ({winning,all}{nums,names} in the end of the simulation"
        )]
        dump: Option<DumpMode>,

        /// whether to print everything regarding the ruleset_data
        #[arg(
            long = "full",
            help = "print all probabilities instead of just the topX below the tables"
        )]
        full: bool,

        /// feature-flag for enabling the caching functionality
        #[arg(
            long = "allow-cache",
            help = "Allow caching to be used in principle. Whether it will be used depends on the config and/or the use-cache flag"
        )]
        allow_cache: bool,

        /// whether a cache shall be generated
        #[arg(
            long = "gen-cache",
            help = "Generate a cache of the current final stage. Overrides the setting in the config if set"
        )]
        gen_cache: bool,

        /// whether to use a cache
        /// overrides the game-config
        #[arg(
            value_enum,
            long = "use-cache",
            help = "Specifies which/whether cache shall be used. Overrides the setting of the config if set"
        )]
        use_cache: Option<CacheModeArg>,

        /// which fallback to use if the specified cache does not exist
        /// overrides the game-config
        #[arg(
            value_enum,
            long = "cache-fallback",
            help = "Specifies what shall be done if cache is requested but the specified one is not found. Overrides the setting of the config"
        )]
        cache_fallback: Option<CacheModeFallback>,

        /// whether to use a cache and which cache (concrete path) shall be used
        /// overrides the game-config
        #[arg(
            long,
            conflicts_with = "cache_event",
            help = "If specific cache is requested, this specifies which one"
        )]
        cache_path: Option<PathBuf>,

        /// whether to use a cache and the cache of which event shall be used
        /// overrides the game-config
        #[arg(
            long,
            conflicts_with = "cache_path",
            help = "if the cache of a specific event is requested, this specifies which one"
        )]
        cache_event: Option<String>,
    },
    /// Linter like checking of the game-config for errors
    Check {
        /// The path to the file to read
        yaml_path: PathBuf,
    },
    /// Build comparison HTML pages for the dataset directories
    Comparison {
        /// id for the palette used for generating the light-theme output
        #[arg(short = 'l', long = "theme-light", default_value = "1")]
        theme_light: u8,
        /// id for the palette used for generating the dark-theme output
        #[arg(short = 'd', long = "theme-dark", default_value = "3")]
        theme_dark: u8,
        /// base-path where to write the comparison site for the german seasons to
        html_path_de: PathBuf,
        /// base-path where to write the comparison site for the us+uk seasons to
        html_path_us: PathBuf,
    },
    /// Report cache availability for a YAML file
    Cache {
        /// Path to the game-config for which the caches shall be listed
        yaml_path: PathBuf
    },
}

/// Run the command selected by the CLI arguments. Factored out for easier testing or reuse.
fn main() {
    let args = Cli::parse();

    match args.cmd {
        Commands::Sim {
            no_tree_output,
            ignore,
            yaml_path,
            // colored: _,
            transpose_tabs,
            stem,
            dump,
            full,

            allow_cache,
            gen_cache,
            use_cache,
            cache_fallback,
            cache_path,
            cache_event,
        } => {
            let gp = GameParse::new_from_yaml(&yaml_path).expect("Parsing failed");
            let gp_cache = (
                gp.gen_cache,
                gp.use_cache.clone(),
                gp.cache_fallback.clone(),
            );
            let mut g = gp
                .finalize_parsing(&stem, &ignore)
                .expect("processing game failed");

            if allow_cache {
                // construct the full cache-mode (postprocess the cli arguments)
                let cache_mode = use_cache
                    .map(|x| x.finalize(&cache_path, &cache_event))
                    .transpose()
                    .unwrap();

                // cli arguments override the settings in the config
                let cache_mode = cache_mode.or(gp_cache.1);
                let cache_fallback = cache_fallback.or(gp_cache.2);

                let cs: Vec<CacheSpec> = g.get_cache_candidates();
                // try selecting a cache in case a cache mode was provided
                if let Some(cache_mode) = cache_mode {
                    g.select_cache(&cs, cache_mode, &cache_fallback, true)
                        .unwrap();
                }
                if gp_cache.0 || gen_cache {
                    g.set_gen_cache(&cs, true).unwrap();
                }
            }

            let start = Instant::now();
            let result: IterState<ProgressBar, _> = g.sim(dump.clone()).unwrap();
            g.eval(transpose_tabs, dump, full, &result, no_tree_output)
                .unwrap();
            println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
        }
        Commands::Cache { yaml_path } => {
            let gp = GameParse::new_from_yaml(&yaml_path).expect("Parsing failed");
            let mut g = gp
                .finalize_parsing(std::path::Path::new(".trash"), &IgnoreOps::Nothing)
                .expect("processing game failed");

            let cs = g.get_cache_candidates();
            show_caches(cs).unwrap();
        }
        Commands::Check { yaml_path } => {
            let gp = GameParse::new_from_yaml(&yaml_path).expect("Parsing failed");
            gp.finalize_parsing(std::path::Path::new(".trash"), &IgnoreOps::Nothing)
                .expect("processing game failed");
        }
        Commands::Comparison {
            theme_light,
            theme_dark,
            html_path_de,
            html_path_us,
        } => {
            comparison::write_pages(&html_path_de, &html_path_us, theme_light, theme_dark).unwrap();
        }
    }
}

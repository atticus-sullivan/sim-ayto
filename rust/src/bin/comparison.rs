// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module serves as a CLI to the simulation/calculation/evaluation/reporting and comparison
//! code.

use ayto::comparison;
use clap::Parser;
use std::path::PathBuf;

/// Build comparison HTML pages for the dataset directories
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
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
}

/// Run the command selected by the CLI arguments. Factored out for easier testing or reuse.
fn main() {
    let args = Cli::parse();

    comparison::write_pages(
        &args.html_path_de,
        &args.html_path_us,
        args.theme_light,
        args.theme_dark,
    )
    .unwrap();
}

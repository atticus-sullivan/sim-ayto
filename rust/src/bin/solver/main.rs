// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module is the binary module for running the solver. It only provides a CLI interface for
//! the solver.

mod engine;
mod init;
mod result;
mod rng;
mod runner;
mod strategies;
mod utils;
mod writer;
mod step;

use std::path::PathBuf;

use anyhow::Result;
use ayto::constraint::ConstraintGetters;
use clap::{Parser, Subcommand};

use crate::runner::run_many_and_write;
use crate::step::CfgParse;
use crate::strategies::{mb, mn, Strategy};
use crate::utils::calc_entropy;

/// The amount of players currently expected -> variable so it can be changed more easily later
const NUM_PLAYERS_SET_A: usize = 10;

#[derive(Parser, Debug)]
/// A struct for parsing the CLI arguments
struct Args {
    /// subcommands of the binary
    #[command(subcommand)]
    cmd: Commands,
}

/// Specifies the subcommands available on the CLI
#[derive(Subcommand, Debug)]
enum Commands {
    /// Benchmark a specific solver
    Bench {
        /// Number of simulations to run
        #[arg(short = 'n', long = "num", default_value_t = 16)]
        num_sims: usize,

        /// Output path for JSONL results
        #[arg(short = 'o', long = "out", default_value = "sim_results.jsonl")]
        out_path: PathBuf,

        /// RNG master seed - optional for reproducibility
        #[arg(short = 's', long = "seed")]
        seed: Option<u64>,
    },
    /// Calculate the next step for a given configuration based on a specific solver
    Step {
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    // build strategies...
    let strategy = Strategy {
        mb: mb::optimal::OptimalMbOptimizer,
        mn: mn::entropy_left::EntropyLeftMnOptimizer::new(5_000),
    };

    match args.cmd {
        Commands::Bench { num_sims, out_path, seed } => {
            run_many_and_write(num_sims, &out_path, seed, strategy.into())
        },
        Commands::Step {  } => {
            let p = PathBuf::from("./cfg.yml");
            let cfg_p = CfgParse::new_from_yaml(&p).expect("Parsing failed");
            let (sol, t, mut sim) = cfg_p
                .finalize_parsing(0, 0, strategy.into())
                .expect("processing config failed");

            if let Some(t) = t {
                let h = calc_entropy(&t, &sim.possibilities);
                println!("try:");
                println!("{}| {:?}", h, t);
                println!("{}| {:?}", h, t.prepare_debug_print());
                println!();
            }

            let (h, c) = sim.next_step(&sol)?;
            let m = c.matching();
            println!("solved:");
            println!("{}| {:?}", h, m);
            println!("{}| {:?}", h, m.prepare_debug_print());


            Ok(())
        },
    }

}

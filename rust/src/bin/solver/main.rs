// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod engine;
mod init;
mod result;
mod rng;
mod runner;
mod strategies;
mod utils;
mod writer;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::runner::run_many_and_write;
use crate::strategies::{mb, mn, Strategy};

const NUM_PLAYERS_SET_A: usize = 10;

#[derive(Parser, Debug)]
#[command(name = "sim_ayto")]
struct Args {
    /// Number of simulations to run
    #[arg(short = 'n', long = "num", default_value_t = 16)]
    num_sims: usize,

    /// Output path for JSONL results
    #[arg(short = 'o', long = "out", default_value = "sim_results.jsonl")]
    out_path: PathBuf,

    /// RNG master seed - optional for reproducibility
    #[arg(short = 's', long = "seed")]
    seed: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    // build strategies...
    let strategy = Strategy {
        mb: mb::optimal::OptimalMbOptimizer,
        mn: mn::entropy_left::EntropyLeftMnOptimizer::new(5_000),
    };

    run_many_and_write(args.num_sims, &args.out_path, args.seed, strategy.into())
}

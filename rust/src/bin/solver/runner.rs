//! This module orchestrates running multiple simulations independently in parallel.
//! It uses `rayon` to spwan X thrads running a fresh simulation in parallel.
//! A single writer thread is responsible for writing the gathered statistics/results to disk.

use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use rand::Rng;
use rayon::prelude::*;

use crate::engine::Simulation;
use crate::rng::create_rng;
use crate::strategies::StrategyBundle;
use crate::writer::{spawn_writer_thread, WriterMsg};

/// Run many simulations in parallel, collect results, and append JSON lines to `out_path`.
/// `num_sims` - how many independent simulations to run
pub(super) fn run_many_and_write<S: StrategyBundle>(
    num_sims: usize,
    out_path: &Path,
    seed: Option<u64>,
    strategy: Arc<S>,
) -> Result<()> {
    let (tx, writer_handle) = spawn_writer_thread(num_sims, out_path)?;

    let seeds = generate_seeds(num_sims, seed);

    execute_parallel_simulations(seeds, strategy, tx);
    match writer_handle.join() {
        // propagate writer error
        Ok(res) => res,
        Err(panic) => Err(anyhow!("WriterThreadPanicked: {:?}", panic)),
    }
}

/// Generates reproducible per-simulation seeds.
fn generate_seeds(num_sims: usize, seed: Option<u64>) -> Vec<u64> {
    let mut master_rng = match seed {
        Some(s) => create_rng(s),
        None => rand::make_rng(),
    };

    (0..num_sims).map(|_| master_rng.next_u64()).collect()
}

/// Executes all simulations in parallel using Rayon.
fn execute_parallel_simulations<S: StrategyBundle>(
    seeds: Vec<u64>,
    strategy: Arc<S>,
    tx: mpsc::Sender<WriterMsg>,
) {
    seeds
        .into_par_iter()
        .enumerate()
        .for_each_with(tx.clone(), |tx, (sim_id, seed)| {
            let start_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let _ = tx.send(WriterMsg::Started { sim_id, start_ms });
            let sim = Simulation::new(sim_id, seed, strategy.clone());

            match sim.run() {
                Ok(res) => {
                    let _ = tx.send(WriterMsg::Finished(res));
                }
                Err(e) => {
                    let _ = tx.send(WriterMsg::Failed(sim_id, format!("{:?}", e)));
                }
            }
        });

    drop(tx);
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn generate_seeds_is_deterministic_with_seed() {
        let s1 = generate_seeds(5, Some(42));
        let s2 = generate_seeds(5, Some(42));

        assert_eq!(s1, s2);
        assert_eq!(s1.len(), 5);
        assert_eq!(s2.len(), 5);
    }

    #[test]
    fn generate_seeds_differs_without_seed() {
        let s1 = generate_seeds(3, None);
        let s2 = generate_seeds(3, None);

        assert_ne!(s1, s2);
        assert_eq!(s1.len(), 3);
        assert_eq!(s2.len(), 3);
    }
}

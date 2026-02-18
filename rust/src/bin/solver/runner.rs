use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

use crate::engine::Simulation;
use crate::strategies::StrategyBundle;
use crate::writer::{spawn_writer_thread, WriterMsg};

/// Run many simulations in parallel, collect results, and append JSON lines to `out_path`.
/// `num_sims` - how many independent simulations to run
pub fn run_many_and_write<S: StrategyBundle>(
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
        Some(s) => StdRng::seed_from_u64(s),
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
    use std::fs;
    use std::sync::Arc;
    use rand::Rng;
    use tempfile::tempdir;

    use ayto::matching_repr::MaskedMatching;

    // ------------------------------------------------------------
    // Deterministic test strategy
    // ------------------------------------------------------------

    #[derive(Clone)]
    struct TestStrategy;

    impl StrategyBundle for TestStrategy {
        fn initial_value(&self) -> MaskedMatching {
            let ids: Vec<u8> = (0..crate::NUM_PLAYERS_SET_A as u8).collect();
            MaskedMatching::from(ids.as_slice())
        }

        fn choose_mb(
            &self,
            _data: &[Vec<u128>],
            _total: u128,
            _rng: &mut dyn Rng,
        ) -> MaskedMatching {
            self.initial_value()
        }

        fn choose_mn(
            &self,
            left_poss: &[MaskedMatching],
            _rng: &mut dyn Rng,
        ) -> MaskedMatching {
            left_poss.first().cloned().unwrap()
        }
    }

    // ------------------------------------------------------------
    // generate_seeds
    // ------------------------------------------------------------

    #[test]
    fn generate_seeds_is_deterministic_with_seed() {
        let s1 = generate_seeds(5, Some(42));
        let s2 = generate_seeds(5, Some(42));

        assert_eq!(s1, s2);
    }

    #[test]
    fn generate_seeds_has_correct_length() {
        let seeds = generate_seeds(10, Some(1));
        assert_eq!(seeds.len(), 10);
    }

    #[test]
    fn generate_seeds_differs_without_seed() {
        let s1 = generate_seeds(3, None);
        let s2 = generate_seeds(3, None);

        assert_ne!(s1, s2);
    }

    // ------------------------------------------------------------
    // execute_parallel_simulations
    // ------------------------------------------------------------

    #[test]
    fn execute_parallel_simulations_sends_messages() {
        let (tx, rx) = mpsc::channel();
        let strategy = Arc::new(TestStrategy);

        let seeds = generate_seeds(3, Some(123));

        execute_parallel_simulations(seeds, strategy, tx);

        let messages: Vec<_> = rx.iter().collect();

        // For each sim we expect:
        // 1 Started
        // 1 Finished (since deterministic strategy shouldn't fail)
        assert_eq!(messages.len(), 6);
    }

    // ------------------------------------------------------------
    // run_many_and_write (end-to-end)
    // ------------------------------------------------------------

    #[test]
    fn run_many_and_write_creates_output_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("out.jsonl");

        let strategy = Arc::new(TestStrategy);

        let result = run_many_and_write(
            3,
            &file_path,
            Some(42),
            strategy,
        );

        assert!(result.is_ok());

        // file should exist
        assert!(file_path.exists());

        let content = fs::read_to_string(file_path).unwrap();

        // Should contain at least 3 lines (Finished events)
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() >= 3);
    }

    // ------------------------------------------------------------
    // run_many_and_write error propagation
    // ------------------------------------------------------------

    #[test]
    fn run_many_and_write_fails_on_invalid_path() {
        let strategy = Arc::new(TestStrategy);

        // Invalid path (directory that cannot be created)
        let invalid_path = Path::new("/this/path/should/not/exist/output.json");

        let result = run_many_and_write(
            1,
            invalid_path,
            Some(1),
            strategy,
        );

        assert!(result.is_err());
    }
}

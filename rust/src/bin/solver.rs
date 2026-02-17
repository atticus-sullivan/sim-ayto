// TODO: grep for "expect"/"unwrap" and fixup (or ignore) -- issue might be that that's now
// different threads
//
// TODO: more initial_value(s) -> conditionally -- the possible outcomes of the first constraint
// are usually limited, so we can hardcode the (optimal) responses for the second step as well

use ayto::constraint::eval_types::EvalEvent;
use ayto::matching_repr::{bitset::Bitset, MaskedMatching};
use clap::Parser;

use chrono::{DateTime, TimeZone, Utc};
use indicatif::{ProgressBar, ProgressStyle};

use rand::rngs::StdRng;
use rust_decimal::{dec, Decimal};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use ayto::constraint::{self, Constraint};
use ayto::iterstate::IterState;
use ayto::ruleset::RuleSet;
use ayto::Rem;

use rand::rand_core::Rng;
use rand::seq::IndexedRandom;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::sync::mpsc;
use std::sync::Arc;

use rayon::prelude::*;

/// Chooses an MB.
/// `data` has the structure you provided earlier (Vec<Vec<u128>>).
pub trait MbOptimizer: Send + Sync {
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, rng: &mut dyn Rng) -> MaskedMatching;
}

/// Chooses an MN
pub trait MnOptimizer: Send + Sync {
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching;
}

pub struct DefaultMbOptimizer;

impl MbOptimizer for DefaultMbOptimizer {
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, _rng: &mut dyn Rng) -> MaskedMatching {
        let target = total / 2; // that is the optimum we want to be close
        let mut closest_diff = u128::MAX;
        let mut closest_index = (0u8, 0u8);

        for (i, row) in data.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                let diff = val.abs_diff(target);
                if diff < closest_diff {
                    closest_diff = diff;
                    closest_index = (i as u8, j as u8);
                }
            }
        }

        closest_index.into()
    }
}

/// Default MN optimizer that picks the candidate maximizing entropy (your original).
pub struct DefaultMnOptimizer {
    /// sampling threshold for performance
    pub sample_threshold: usize,
}

impl DefaultMnOptimizer {
    pub fn new(sample_threshold: usize) -> Self {
        Self { sample_threshold }
    }
}

impl MnOptimizer for DefaultMnOptimizer {
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching {
        // using all of the perms results in too long computations
        // => only use left_poss they have a better chance for a good result anyhow

        // in case there are many possibilities left, don't use them all. Instead sample them randomly
        // down to a threshold
        let threshold = 5_000;

        if left_poss.len() > threshold {
            left_poss
                .sample(rng, threshold)
                .map(|m| (calc_entropy(m, left_poss), m))
                .max_by(|(e1, _), (e2, _)| e1.partial_cmp(e2).unwrap())
                .map(|(_, m)| m)
                .unwrap()
                .clone()
        } else {
            left_poss
                .iter()
                .map(|m| (calc_entropy(m, left_poss), m))
                .max_by(|(e1, _), (e2, _)| e1.partial_cmp(e2).unwrap())
                .map(|(_, m)| m)
                .unwrap()
                .clone()
        }
    }
}

/// A single trait that groups both MB and MN strategy behaviour
/// and provides an initial value for a set of perms.
///
/// - `choose_mb`: choose a (u8,u8) MB pair
/// - `choose_mn`: choose a Vec<u8> MN matching
/// - `initial_value`: produce an initial HashMap
///
/// The `usize` value is a practical default; change the return type if you want another payload.
pub trait StrategyBundle: Send + Sync {
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, rng: &mut dyn Rng) -> MaskedMatching;
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching;

    /// Produce an initial value for the first constraint. Up to this point no information is known
    fn initial_value(&self) -> MaskedMatching;
}

/// Simple concrete implementation that composes your existing optimizers.
pub struct DefaultStrategy {
    pub mb: DefaultMbOptimizer,
    pub mn: DefaultMnOptimizer,
}

impl DefaultStrategy {
    pub fn new(mn_sample_threshold: usize) -> Self {
        Self {
            mb: DefaultMbOptimizer {},
            mn: DefaultMnOptimizer::new(mn_sample_threshold),
        }
    }
}

impl StrategyBundle for DefaultStrategy {
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, rng: &mut dyn Rng) -> MaskedMatching {
        // delegate to your previous implementation
        self.mb.choose_mb(data, total, rng)
    }

    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching {
        // delegate to your previous implementation
        self.mn.choose_mn(left_poss, rng)
    }

    fn initial_value(&self) -> MaskedMatching {
        // match (0,0)
        MaskedMatching::from_masks(
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                .into_iter()
                .map(Bitset::from_word)
                .collect(),
        )
    }
    // let mns:Vec<Vec<(u8, u8)>> = vec![
    //     // vec![
    //     //     (0u8,0u8),
    //     //     (1u8,1u8),
    //     //     (2u8,2u8),
    //     //     (3u8,3u8),
    //     //     (4u8,4u8),
    //     //     (5u8,5u8),
    //     //     (6u8,6u8),
    //     //     (7u8,7u8),
    //     //     (8u8,8u8),
    //     //     (9u8,9u8)
    //     // ],
    // ];
}

#[derive(Serialize)]
struct SimulationResult {
    sim_id: usize,
    seed: u64,
    stats: Vec<EvalEvent>,
    iterations_count: usize, // TODO: check for off-by-one in the evaluation
    duration_ms: u128,
}

/// Run a single simulation. Returns `SimulationResult`.
///
/// The function takes ownership of strategies via `Arc<Box<...>>` or referenced boxed trait objects,
/// but to keep example simple we accept references to the trait objects and create a per-sim RNG.
fn run_single_simulation<S: StrategyBundle>(
    sim_id: usize,
    seed: u64,
    strategy: &S,
) -> Result<SimulationResult> {
    let start = Instant::now();

    // create a reproducible RNG for this simulation
    let mut rng = StdRng::seed_from_u64(seed);

    let mut solution: Vec<u8> = (0..10).map(|x| x as u8).collect();
    solution.shuffle(&mut rng);
    let solution: MaskedMatching = (*solution).into();
    let mut lights_known_before = 0;

    let rs = RuleSet::Eq;
    let lut_a = vec![
        ("a".to_owned(), 0),
        ("b".to_owned(), 1),
        ("c".to_owned(), 2),
        ("d".to_owned(), 3),
        ("e".to_owned(), 4),
        ("f".to_owned(), 5),
        ("g".to_owned(), 6),
        ("h".to_owned(), 7),
        ("i".to_owned(), 8),
        ("j".to_owned(), 9),
    ]
    .into_iter()
    .collect();

    // perform the first step
    let m = strategy.initial_value();
    let l = m.calculate_lights(&solution);

    let c = Constraint::new_unchecked(
        if m.len() == 1 {
            constraint::ConstraintType::Box {
                num: dec![1.0],
                comment: "".to_owned(),
                offer: None,
            }
        } else {
            constraint::ConstraintType::Night {
                num: dec![1.0],
                comment: "".to_owned(),
            }
        },
        ayto::constraint::CheckType::Lights(l, Default::default()),
        m,
        rs.init_data()?,
        10,
        10,
        lights_known_before,
    );
    lights_known_before = l;

    let mut is = IterState::new(
        true,
        10,
        vec![c.clone()],
        &[],
        &(HashSet::new(), HashSet::new()),
        &None,
        (10, 10),
    )?;
    rs.iter_perms(&lut_a, &HashMap::new(), &mut is, false, &None)?;
    let mut poss: Vec<MaskedMatching> = is.left_poss.clone();

    let mut cs = is.constraints;
    let mut rem: Rem = (is.each, is.total);
    rem = cs
        .last_mut()
        .unwrap()
        .apply_to_rem(rem)
        .context("Apply to rem failed")?;

    for i in 3usize.. {
        let (m, l, ct, lkn) = if i.is_multiple_of(2) {
            let m = strategy.choose_mb(&rem.0, rem.1, &mut rng);
            let l = m.calculate_lights(&solution);
            let ct = constraint::ConstraintType::Box {
                num: (Decimal::from(i) / dec![2]).floor(),
                comment: "".to_owned(),
                offer: None,
            };
            let lkn_old = lights_known_before;
            if l == 1 {
                lights_known_before += 1;
            }
            (m, l, ct, lkn_old)
        } else {
            let m = strategy.choose_mn(&poss, &mut rng);
            let l = m.calculate_lights(&solution);
            let ct = constraint::ConstraintType::Night {
                num: (Decimal::from(i) / dec![2]).floor(),
                comment: "".to_owned(),
            };
            (m, l, ct, lights_known_before)
        };

        cs.push(Constraint::new_unchecked(
            ct,
            ayto::constraint::CheckType::Lights(l, Default::default()),
            m,
            rs.init_data()?,
            10,
            10,
            lkn,
        ));

        if let Some(c) = cs.last_mut() {
            poss.retain(|p| c.process(p).unwrap());
            rem = c.apply_to_rem(rem).context("Apply to rem failed")?;
        }

        if poss.len() <= 1 {
            let cnt_iter = cs.len();
            return Ok(SimulationResult {
                sim_id,
                seed,
                stats: cs
                    .into_iter()
                    .flat_map(|c| c.get_stats().transpose())
                    .collect::<Result<Vec<_>>>()
                    .unwrap(),
                iterations_count: cnt_iter,
                duration_ms: start.elapsed().as_millis(),
            });
        }
    }
    bail!("Unexpected termination")
}

enum WriterMsg {
    Started { sim_id: usize, start_ms: u128 },
    Finished(SimulationResult),
    Failed(usize, String),
}

/// Format a millisecond timestamp into HH:MM:SS for the progress display.
fn format_time(ms: u128) -> String {
    let secs = (ms / 1000) as i64;
    let nsecs = ((ms % 1000) * 1_000_000) as u32;

    let dt: DateTime<Utc> = Utc.timestamp_opt(secs, nsecs).unwrap();
    dt.format("%H:%M:%S").to_string()
}

fn set_pb_msg(pb: &ProgressBar, active: &HashMap<usize, u128>) {
    pb.set_message(format!(
        "active:{} {}",
        active.len(),
        active
            .iter()
            .map(|(id, start)| format!("#{}@{}", id, format_time(*start)))
            .collect::<Vec<_>>()
            .join(", ")
    ));
}

/// Run many simulations in parallel, collect results, and append JSON lines to `out_path`.
/// `num_sims` - how many independent simulations to run
pub fn run_many_and_write<S: StrategyBundle>(
    num_sims: usize,
    out_path: &PathBuf,
    seed: Option<u64>,
    strategy: S,
) -> Result<()> {
    // create mpsc channel for results; single writer thread will serialize
    let (tx, rx) = mpsc::channel::<WriterMsg>();

    // writer thread - owns the file and writes JSON lines
    let out_path = out_path.to_owned();
    let writer_handle = std::thread::spawn(move || {
        let pb = ProgressBar::new(num_sims as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar}] {pos:>3}/{len:3} (ETA: {eta}) {msg}",
            )
            .unwrap(),
        );

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&out_path)
            .expect("failed to open output file");

        let mut active: HashMap<usize, u128> = HashMap::new();

        while let Ok(msg) = rx.recv() {
            match msg {
                WriterMsg::Started { sim_id, start_ms } => {
                    active.insert(sim_id, start_ms);
                    set_pb_msg(&pb, &active);
                }
                WriterMsg::Finished(sim_res) => {
                    let line = serde_json::to_string(&sim_res)
                        .expect("serde shouldn't fail for SimulationResult");
                    file.write_all(line.as_bytes()).expect("write failed");
                    file.write_all(b"\n").expect("write newline failed");

                    pb.inc(1);
                    active.remove(&sim_res.sim_id);
                    set_pb_msg(&pb, &active);
                }
                WriterMsg::Failed(sim_id, msg) => {
                    pb.inc(1);
                    active.remove(&sim_id);
                    pb.println(msg);
                    set_pb_msg(&pb, &active);
                }
            }

            // optionally flush periodically or based on file size. For simplicity, flush per write:
            file.flush().expect("flush failed");
        }

        pb.finish_with_message("done");
    });

    // wrap strategies in Arc for shared reference in parallel threads
    let strategy_arc: Arc<S> = Arc::new(strategy);

    // create a range of seeds so runs are reproducible
    // use seeds to obtain an individual rng for each job -> avoid locking involved whith using a
    // shared rng
    // Storing the seed for a job (in the json) makes that job reproducible
    let mut master_rng = if let Some(seed) = seed {
        StdRng::seed_from_u64(seed)
    } else {
        rand::make_rng()
    };

    // DONE: why collect when next step it to iterate over them? (num_sims might be large so the allocation might be bad)
    // well rayon needs a ParallelIterator but map() returns a SequentialIterator
    //
    // in case seeds gets too large (for large num_sims): we can iterate (in parallel) over (0..num_sims) and then "hash" the number in order to obtain a random seed
    // TODO: In fact didn't rand ensure different rngs even if the seed is "near" (1 vs 2)? Then we
    // could just use (0..num_sims) directly as seed
    let seeds: Vec<u64> = (0..num_sims).map(|_| master_rng.next_u64()).collect();

    // parallel execution with rayon
    seeds
        .into_par_iter()
        .enumerate()
        .for_each_with(tx.clone(), |tx_clone, (sim_id, seed)| {
            // each closure runs in a Rayon worker thread

            let start_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let _ = tx_clone.send(WriterMsg::Started { sim_id, start_ms });

            let strategy_ref: &S = &strategy_arc;

            match run_single_simulation(sim_id, seed, strategy_ref) {
                Ok(sim_res) => {
                    // send result to writer – ignore errors (writer closed) gracefully
                    let _ = tx_clone.send(WriterMsg::Finished(sim_res));
                }
                Err(e) => {
                    // on failure, we still want to write an error record; here we just print
                    let msg = format!("simulation {} failed: {:?}", sim_id, e);

                    // notify about failing for bookkeeping
                    let _ = tx_clone.send(WriterMsg::Failed(sim_id, msg));
                }
            }
        });

    // drop tx so writer thread sees channel closed and exits
    drop(tx);
    // join writer
    let _ = writer_handle.join();

    Ok(())
}

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
    let strategy = DefaultStrategy::new(5_000);

    run_many_and_write(args.num_sims, &args.out_path, args.seed, strategy)?;
    Ok(())
}

/// Entropy calculation for a candidate `m` across `left_poss`.
///
/// This is a small pure function and unit tested below.
fn calc_entropy(m: &MaskedMatching, left_poss: &[MaskedMatching]) -> f64 {
    let total = left_poss.len() as f64;

    let mut lights = [0u32; 11];
    for p in left_poss {
        // assume:
        // - p is the solution
        // - m is how they sit in the night
        let l = m.calculate_lights(p);
        lights[l as usize] += 1;
    }

    lights
        .into_iter()
        .filter(|&i| i > 0)
        .map(|i| {
            let p = (i as f64) / total;
            -p * p.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ayto::matching_repr::bitset::Bitset;
    use ayto::matching_repr::MaskedMatching;

    #[test]
    fn format_time_zero() {
        assert_eq!(format_time(0), "00:00:00");
    }

    #[test]
    fn calc_entropy_small_case() {
        // m: masks {A0->{0}, A1->{0}, A2->{1}}
        let m = MaskedMatching::from_masks(vec![
            Bitset::from_word(1),
            Bitset::from_word(1),
            Bitset::from_word(2),
        ]);
        // left_poss: p1=[0,0,1], p2=[0,1,1], p3=[1,0,1], p4=[1,1,1]
        let p1 = MaskedMatching::from_matching_ref(&[vec![0], vec![0], vec![1]]);
        let p2 = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![1]]);
        let p3 = MaskedMatching::from_matching_ref(&[vec![1], vec![0], vec![1]]);
        let p4 = MaskedMatching::from_matching_ref(&[vec![1], vec![1], vec![1]]);
        let left = vec![p1, p2, p3, p4];
        let h = calc_entropy(&m, &left);
        // expected distribution: l=3 (1), l=2 (2), l=1 (1) -> probs 0.25,0.5,0.25 -> entropy 1.5
        let expected = 1.5;
        let diff = (h - expected).abs();
        assert!(diff < 1e-9, "entropy mismatch: {} vs {}", h, expected);
    }
}

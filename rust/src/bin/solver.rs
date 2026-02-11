/*
sim_ayto
Copyright (C) 2026  Lukas Heindl

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

// TODO: grep for "expect"/"unwrap" and fixup (or ignore)

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::time::Instant;
use rand::rngs::StdRng;
use serde::Serialize;

use anyhow::{Context,Result,bail};
use ayto::constraint::{self, Constraint};
use ayto::iterstate::IterState;
use ayto::ruleset::RuleSet;
use ayto::{Matching, Rem};

use rand::rand_core::Rng;
use rand::seq::IndexedRandom;
use rand::RngExt;

/// Chooses an MB.
/// `data` has the structure you provided earlier (Vec<Vec<u128>>).
pub trait MbOptimizer: Send + Sync {
    fn choose_mb(&self, data: &Vec<Vec<u128>>, total: u128, rng: &mut dyn Rng) -> (u8, u8);
}

/// Chooses an MN
pub trait MnOptimizer: Send + Sync {
    fn choose_mn(&self, perms: &[Vec<u8>], left_poss: &[Vec<u8>], rng: &mut dyn Rng) -> Vec<u8>;
}

pub struct DefaultMbOptimizer;

impl MbOptimizer for DefaultMbOptimizer {
    fn choose_mb(&self, data: &Vec<Vec<u128>>, total: u128, _rng: &mut dyn Rng) -> (u8, u8) {
        let target = total / 2; // that is the optimum we want to be close
        let mut closest_diff = u128::MAX;
        let mut closest_index = (0u8, 0u8);

        for (i, row) in data.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                let diff = if val >= target { val - target } else { target - val };
                if diff < closest_diff {
                    closest_diff = diff;
                    closest_index = (i as u8, j as u8);
                }
            }
        }

        closest_index
    }
}

/// Default MN optimizer that picks the candidate maximizing entropy (your original).
pub struct DefaultMnEntropyOptimizer {
    /// sampling threshold for performance
    pub sample_threshold: usize,
}

impl DefaultMnEntropyOptimizer {
    pub fn new(sample_threshold: usize) -> Self {
        Self { sample_threshold }
    }
}

// // TODO: shuffle probably a bad idea with such a long slice
// impl MnOptimizer for DefaultMnEntropyOptimizer {
//     fn choose_mn(&self, perms: &[Vec<u8>], left_poss: &[Vec<u8>], rng: &mut dyn Rng) -> Vec<u8> {
//         // prefer left_poss (as you did)
//         let sample_space = left_poss;
//         let threshold = self.sample_threshold;
//
//         if sample_space.len() > threshold {
//             // sample randomly down to `threshold` unique elements
//             let mut chosen = Vec::with_capacity(threshold);
//             // using SliceRandom::choose_multiple to sample without replacement
//             let mut tmp = sample_space.to_vec();
//             tmp.as_mut_slice().shuffle(&mut rand::rngs::StdRng::from_entropy());
//             tmp.truncate(threshold);
//             // evaluate
//             tmp.into_iter()
//                 .map(|m| (calc_entropy(&m, left_poss), m))
//                 .max_by(|(e1, _), (e2, _)| e1.partial_cmp(e2).unwrap())
//                 .map(|(_, m)| m)
//                 .unwrap()
//                 .clone()
//         } else {
//             sample_space
//                 .iter()
//                 .map(|m| (calc_entropy(&m, left_poss), m))
//                 .max_by(|(e1, _), (e2, _)| e1.partial_cmp(e2).unwrap())
//                 .map(|(_, m)| m)
//                 .unwrap()
//                 .clone()
//         }
//     }
// }

// TODO: adjust stats
#[derive(Serialize)]
struct IterationRecord {
    iteration: usize,
    // the `m` used this iteration — store as vector of pairs (a,b)
    m: Vec<(u8, u8)>,
    lights: u8,
}

#[derive(Serialize)]
struct SimulationResult {
    sim_id: usize,
    seed: u64,
    iterations: Vec<IterationRecord>,
    iterations_count: usize,
    duration_ms: u128,
}


/// Run a single simulation. Returns `SimulationResult`.
///
/// The function takes ownership of strategies via `Arc<Box<...>>` or referenced boxed trait objects,
/// but to keep example simple we accept references to the trait objects and create a per-sim RNG.
fn run_single_simulation(
    sim_id: usize,
    seed: u64,
    mb_strategy: &dyn MbOptimizer,
    mn_strategy: &dyn MnOptimizer,
) -> Result<SimulationResult> {
    let start = Instant::now();

    // create a reproducible RNG for this simulation
    let mut rng = StdRng::seed_from_u64(seed); // TODO:

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
        ("j".to_owned(), 9)
    ].into_iter().collect();

    // generate everything
    let mut is = IterState::new(true, 10, vec![], &vec![], &(HashSet::new(), HashSet::new()), &None, (10,10))?;
    rs.iter_perms(&lut_a, &HashMap::new(), &mut is, false, &None)?;
    let mut poss = is.left_poss.iter().map(|is| is.iter().map(|i| i[0]).collect::<Vec<_>>()).collect::<Vec<_>>();

    let all_perms = poss.clone();
    let mut rem: Rem = (is.each, is.total);
    let solution = poss[rng.random_range(0..poss.len())].clone();

    // TODO: maybe make this part of the strategy => also group MN and MB stragegies into one
    let mbs = vec![
        vec![(0u8,0u8)]
    ];
    // TODO: maybe make this part of the strategy
    let mns:Vec<Vec<(u8, u8)>> = vec![
        // vec![
        //     (0u8,0u8),
        //     (1u8,1u8),
        //     (2u8,2u8),
        //     (3u8,3u8),
        //     (4u8,4u8),
        //     (5u8,5u8),
        //     (6u8,6u8),
        //     (7u8,7u8),
        //     (8u8,8u8),
        //     (9u8,9u8)
        // ],
    ];


    let mut cs = Vec::with_capacity(20);
    let mut iteration_records = Vec::new();

    for i in 0usize.. {
        let m = if i.is_multiple_of(2) {
            mbs.get(i).map(|x| x.clone()).unwrap_or_else(|| {
                vec![mb_strategy.choose_mb(&rem.0, rem.1, &mut rng)]
            }).into_iter().collect()
        } else {
            // TODO: use the boxed strategy here as well
            mns.get(i).map(|x| x.clone()).unwrap_or_else(|| {
                // optimize_mn_entropy(&all_perms, &poss).into_iter().enumerate().map(|(k,v)| (k as u8,v)).collect::<Vec<_>>()
                optimize_mn_entropy(&all_perms, &poss).into_iter().enumerate().map(|(k,v)| (k as u8, v)).collect::<Vec<_>>()
            }).into_iter().collect()
        };

        let l = constraint::Constraint::calculate_lights_simple3(&m, &solution);
        cs.push(Constraint::new_unchecked(
            constraint::ConstraintType::Box {
                num: 1.0,
                comment: "".to_owned(),
                offer: None
            },
            ayto::constraint::CheckType::Lights(l, Default::default()),
            m,
            rs.init_data()?,
            10,10,
        ));

        // TODO:
        // record the iteration's m and lights (we store m and l)
        // iteration_records.push(IterationRecord {
        //     iteration: i,
        //     m: m.clone(),
        //     lights: l,
        // });

        if let Some(c) = cs.last_mut() {
            poss.retain(|p| c.process(std::slice::from_ref(p)).unwrap());
            rem = c.apply_to_rem(rem).context("Apply to rem failed")?;
        }

        if poss.len() <= 1 {
            // println!("{i} ({} sec)", start.elapsed().as_secs_f64());
            return Ok(SimulationResult {
                sim_id,
                seed,
                iterations: iteration_records,
                iterations_count: i+1,
                duration_ms: start.elapsed().as_millis(),
            });
        }
    }
    bail!("Unexpected termination")
}

use std::io::Write; // TODO: familiarize write_all what is the advantage?
use std::sync::mpsc; // TODO: familiarize
use std::sync::Arc; // TODO: familiarize

/// Run many simulations in parallel, collect results, and append JSON lines to `out_path`.
/// `num_sims` - how many independent simulations to run
pub fn run_many_and_write(
    num_sims: usize,
    out_path: &str,
    mb_strategy: Box<dyn MbOptimizer + Send + Sync>,
    mn_strategy: Option<Box<dyn MnOptimizer + Send + Sync>>,
) -> Result<()> {
    // create mpsc channel for results; single writer thread will serialize
    let (tx, rx) = mpsc::channel::<SimulationResult>();

    // writer thread - owns the file and writes JSON lines
    let out_path = out_path.to_owned();
    let writer_handle = std::thread::spawn(move || {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&out_path)
            .expect("failed to open output file");
        while let Ok(sim_res) = rx.recv() {
            let line = serde_json::to_string(&sim_res).expect("serde shouldn't fail for SimulationResult");
            file.write_all(line.as_bytes()).expect("write failed");
            file.write_all(b"\n").expect("write newline failed");
            // optionally flush periodically or based on file size. For simplicity, flush per write:
            file.flush().expect("flush failed");
        }
    });

    // wrap strategies in Arc for shared reference in parallel threads
    let mb_arc = Arc::new(mb_strategy);
    let mn_arc = Arc::new(mn_strategy);

    // create a range of seeds so runs are reproducible
    // TODO: investigate randomness with threads in general
    let mut master_rng = StdRng::from_entropy(); // TODO:
    let seeds: Vec<u64> = (0..num_sims).map(|_| master_rng.next_u64()).collect(); // TODO: why collect when next step it to iterate over them? (num_sims might be large so the allocation might be bad)

    // parallel execution with rayon
    // TODO: how are the number of workers determined?
    // TODO: familiarize with rayon (no "use" needed?)
    seeds.into_par_iter().enumerate().for_each_with(tx.clone(), |tx_clone, (sim_id, seed)| {
        // each closure runs in a Rayon worker thread
        let mb_ref: &dyn MbOptimizer = &*mb_arc;
        let mn_ref: &dyn MnOptimizer = &*mn_arc;
        match run_single_simulation(sim_id, seed, mb_ref, mn_ref) {
            Ok(sim_res) => {
                // send result to writer – ignore errors (writer closed) gracefully
                let _ = tx_clone.send(sim_res);
            }
            Err(e) => {
                // on failure, we still want to write an error record; here we just print
                eprintln!("simulation {} failed: {:?}", sim_id, e);
            }
        }
    });

    // drop tx so writer thread sees channel closed and exits
    drop(tx);
    // join writer
    let _ = writer_handle.join();

    Ok(())
}

fn main() -> Result<()> {
    // Example usage: run 16 simulations in parallel and append JSON lines to "sim_results.jsonl"
    let mb_strategy: Box<dyn MbOptimizer + Send + Sync> = Box::new(DefaultMbOptimizer {});
    let mn_strategy: Box<dyn MnOptimizer + Send + Sync> = Box::new(DefaultMnOptimizer::new(5_000));

    run_many_and_write(16, "sim_results.jsonl", mb_strategy, mn_strategy)?;
    Ok(())
}


fn optimize_mn_entropy(_perms: &[Vec<u8>], left_poss: &[Vec<u8>]) -> Vec<u8> {
    // using all of the perms results in too long computations
    // => only use left_poss they have a better chance for a good result anyhow

    // in case there are many possibilities left, don't use them all. Instead sample them randomly
    // down to a threshold
    let threshold = 5_000;
    let mut rng = rand::rng();

    if left_poss.len() > threshold {
        left_poss.sample(&mut rng, threshold)
            .map(|m| (calc_entropy(&m, left_poss), m))
            .max_by(|(e1,_), (e2,_)| e1.partial_cmp(e2).unwrap())
            .map(|(_,m)| m)
            .unwrap()
            .clone()
    } else {
        left_poss
            .iter()
            .map(|m| (calc_entropy(&m, left_poss), m))
            .max_by(|(e1,_), (e2,_)| e1.partial_cmp(e2).unwrap())
            .map(|(_,m)| m)
            .unwrap()
            .clone()
    }
}

fn calc_entropy(m: &Vec<u8>, left_poss: &[Vec<u8>]) -> f64 {
    let total = left_poss.len() as f64;

    let mut lights = [0u32; 11];
    for p in left_poss {
        // assume:
        // - p is the solution
        // - m is how they sit in the night
        let l = constraint::Constraint::calculate_lights_simple2(&m, p);
        lights[l as usize] += 1;
    }

    lights
        .into_iter()
        .filter(|&i| i > 0)
        .map(|i| {
            let p = (i as f64) / total;
            -p * p.log2()
        }).sum()
}

///////////////////////////////////////////
// iterator to convert a (full) Matching //
// to ways how to sit at a MN            //
///////////////////////////////////////////

pub struct MatchingIter<'a> {
    matching: &'a Matching,
    indices: Vec<usize>, // current index per position
    done: bool,
}

impl<'a> MatchingIter<'a> {
    pub fn new(matching: &'a Matching) -> Self {
        let done = matching.iter().any(|v| v.is_empty()); // early empty case
        MatchingIter {
            matching,
            indices: vec![0; matching.len()],
            done,
        }
    }
}

impl<'a> Iterator for MatchingIter<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Build current map
        let current = self
            .matching
            .iter()
            .zip(&self.indices)
            .map(|(choices, &i)| choices[i])
            .collect::<Vec<u8>>();

        // Increment indices
        for pos in (0..self.indices.len()).rev() {
            self.indices[pos] += 1;
            if self.indices[pos] < self.matching[pos].len() {
                break; // no carry, done incrementing
            } else {
                self.indices[pos] = 0; // carry
                if pos == 0 {
                    self.done = true;
                }
            }
        }

        Some(current)
    }
}

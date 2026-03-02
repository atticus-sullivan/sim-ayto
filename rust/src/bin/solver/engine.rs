// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module offers the functionality to run a single simulation. It is so to speak the engine
//! which drives the benchmarking.

use std::sync::Arc;
use std::{collections::HashMap, time::Instant};

use anyhow::{bail, Context, Result};
use indicatif::ProgressBar;
use rand::rngs::StdRng;
use rust_decimal::{dec, Decimal};

use ayto::constraint::{check_type::CheckType, Constraint, ConstraintSim, ConstraintType};
use ayto::matching_repr::MaskedMatching;
use ayto::ruleset::RuleSet;
use ayto::{LightCnt, Rem};

use crate::init::{build_initial_constraint, create_iteration_state, generate_solution};
use crate::result::SimulationResult;
use crate::rng::create_rng;
use crate::strategies::StrategyBundle;
use crate::NUM_PLAYERS_SET_A;

/// Encapsulates the full state and lifecycle of a single simulation run.
///
/// A simulation:
/// 1. Generates a solution
/// 2. Applies constraints iteratively according to a strategy
/// 3. Terminates once only one possibility remains
pub struct Simulation<S: StrategyBundle> {
    /// an identifier which can be used to track this simulation
    sim_id: usize,
    /// the strategy used for playing this game
    strategy: Arc<S>,
    /// seed usefd for reproducible randomness/simulation
    seed: u64,
    /// provides randomness for the optimizers and the game too
    rng: StdRng,
    /// when this simulation started
    start: Instant,
    /// the ruleset used for playing this game
    ruleset: RuleSet,

    /////////////////////////////////////////////////////
    // fields which are modified during the simulation //
    /////////////////////////////////////////////////////
    /// accumulated list of constraints used to solve this game
    constraints: Vec<Constraint>,
    /// list of all remaining possible solutions
    possibilities: Vec<MaskedMatching>,
    /// a table tracking the remaining possibilities per 1:1 matching
    rem: Rem,
    /// The amount of lights which are already *known* (proven in a MB decision) up to this point
    lights_known_before: usize,
}

impl<S: StrategyBundle> Simulation<S> {
    /// Lightweight constructor.
    pub fn new(sim_id: usize, seed: u64, strategy: Arc<S>, ruleset: RuleSet) -> Self {
        Self {
            ruleset,
            sim_id,
            seed,
            strategy,
            rng: create_rng(seed),
            start: Instant::now(),
            constraints: Vec::new(),
            possibilities: Vec::new(),
            rem: (vec![], 0),
            lights_known_before: 0,
        }
    }

    /// Initializes the simulation state and computes the initial possibility space.
    ///
    /// Returns the solution
    fn init(&mut self, pb: &ProgressBar) -> Result<MaskedMatching> {
        pb.inc(0);
        let solution = generate_solution(&mut self.rng);
        let initial_matching = self.strategy.initial_value();
        let lights = initial_matching.calculate_lights(&solution);

        let constraint = build_initial_constraint(
            initial_matching,
            lights as usize,
            &self.ruleset,
            self.lights_known_before,
        )?;

        // a new light might be known after the constraint -> update
        self.lights_known_before = lights as usize;

        let mut iter_state = create_iteration_state(&constraint)?;

        let lut = vec![
            ("a", 0),
            ("b", 1),
            ("c", 2),
            ("d", 3),
            ("e", 4),
            ("f", 5),
            ("g", 6),
            ("h", 7),
            ("i", 8),
            ("j", 9),
        ]
        .into_iter()
        .map(|(i, j)| (i.to_string(), j))
        .collect();

        // use the ruleset for the first constraint
        self.ruleset
            .iter_perms(&lut, &HashMap::new(), &mut iter_state, &None)?;

        let mut rem: Rem = (iter_state.each, iter_state.total);
        rem = iter_state
            .constraints
            .last_mut()
            .unwrap() // I know there is exactly one constraint in that vector
            .apply_to_rem(rem)
            .context("Apply to rem failed")?;

        self.constraints = iter_state.constraints;
        self.possibilities = iter_state.left_poss;
        self.rem = rem;
        pb.set_position(iter_state.total as u64 - self.possibilities.len() as u64);

        Ok(solution)
    }

    /// Full simulation execution.
    pub fn run(mut self, pb: &ProgressBar) -> Result<SimulationResult> {
        let solution = self.init(pb)?;
        self.run_loop(&solution, &pb)?;
        self.try_into()
    }

    /// Executes simulation loop.
    fn run_loop(&mut self, solution: &MaskedMatching, pb: &ProgressBar) -> Result<()> {
        for i in 3usize.. {
            let constraint = self.next_step(solution, i, pb)?;

            self.apply_constraint(constraint, pb)?;

            if self.possibilities.len() <= 1 {
                pb.finish_with_message("done");
                return Ok(());
            }
        }

        bail!("Unexpected termination")
    }

    /// Generates and constructs the next constraint
    /// according to the selected strategy and iteration number/index.
    fn next_step(&mut self, solution: &MaskedMatching, iteration: usize, pb: &ProgressBar) -> Result<Constraint> {
        pb.set_message(format!("{:2} -:{:2}", self.sim_id, self.constraints.len()));

        let (m, ct) = if iteration.is_multiple_of(2) {
            // this is a match-box decision
            let m = self
                .strategy
                .choose_mb(&self.rem.0, self.rem.1, &mut self.rng);

            let ct = ConstraintType::Box {
                num: (Decimal::from(iteration) / dec![2]).floor(),
                comment: String::new(),
                offer: None,
            };

            (m, ct)
        } else {
            // this is a matching-night
            let m = self.strategy.choose_mn(&self.possibilities, &mut self.rng);

            let ct = ConstraintType::Night {
                num: (Decimal::from(iteration) / dec![2]).floor(),
                comment: String::new(),
                offer: None,
            };

            pb.set_message(format!("{:2} c:{:2}+1", self.sim_id, self.constraints.len()));

            (m, ct)
        };

        let l = m.calculate_lights(solution);
        let c = Constraint::new_with_defaults(
            ct,
            CheckType::Lights(l, Default::default()),
            m,
            self.ruleset.init_data()?,
            NUM_PLAYERS_SET_A,
            NUM_PLAYERS_SET_A,
            self.lights_known_before as LightCnt,
        );
        self.lights_known_before += c.added_known_lights() as usize;
        Ok(c)
    }

    /// Applies a new constraint to:
    /// - Possibility space (without new allocations) -- Order is not preserved
    /// - Remaining counts
    /// - Constraint list
    fn apply_constraint(&mut self, mut constraint: Constraint, pb: &ProgressBar) -> Result<()> {
        let mut i = 0;

        while i < self.possibilities.len() {
            if constraint.process(&self.possibilities[i])? {
                i += 1;
            } else {
                // does not retain order!
                self.possibilities.swap_remove(i);
                pb.tick();
            }
        }

        self.rem = constraint
            .apply_to_rem(self.rem.clone())
            .context("Apply to rem failed")?;
        self.constraints.push(constraint);
        pb.set_message(format!("{:2} C:{:2}", self.sim_id, self.constraints.len()));

        Ok(())
    }
}

/// Convert finished Simulation into SimulationResult.
impl<S: StrategyBundle> TryInto<SimulationResult> for Simulation<S> {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<SimulationResult, Self::Error> {
        let iterations_count = self.constraints.len();

        let stats = self
            .constraints
            .into_iter()
            .flat_map(|c| c.get_stats().transpose())
            .collect::<Result<Vec<_>>>()?;

        Ok(SimulationResult::new(
            self.sim_id,
            self.seed,
            stats,
            iterations_count,
            self.start.elapsed().as_millis(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ayto::{constraint::ConstraintGetters, matching_repr::IdBase};
    use pretty_assertions::assert_eq;
    use rand::Rng;

    /// Deterministic test strategy.
    /// Always picks the first available possibility.
    #[derive(Clone)]
    struct DeterministicStrategy;

    impl StrategyBundle for DeterministicStrategy {
        fn initial_value(&self) -> MaskedMatching {
            // Single deterministic matching
            let ids: Vec<IdBase> = (0..NUM_PLAYERS_SET_A as IdBase).collect();
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

        fn choose_mn(&self, left_poss: &[MaskedMatching], _rng: &mut dyn Rng) -> MaskedMatching {
            left_poss.first().cloned().expect("no possibilities left")
        }
    }

    fn build_sim(seed: u64) -> Simulation<DeterministicStrategy> {
        Simulation::new(1, seed, Arc::new(DeterministicStrategy))
    }

    #[test]
    fn new_initializes_empty_state() {
        let sim = build_sim(42);

        assert_eq!(sim.sim_id, 1);
        assert_eq!(sim.seed, 42);
        assert!(sim.constraints.is_empty());
        assert!(sim.possibilities.is_empty());
        assert_eq!(sim.lights_known_before, 0);
    }

    #[test]
    fn init_populates_state() {
        let mut sim = build_sim(42);

        let solution = sim.init().expect("init failed");

        assert!(!sim.constraints.is_empty());
        assert!(!sim.possibilities.is_empty());
        assert!(sim.rem.1 > 0);

        // solution must be a valid masked matching
        assert_eq!(
            solution.clone().calculate_lights(&solution) as usize,
            NUM_PLAYERS_SET_A
        );
    }

    #[test]
    fn next_step_generates_constraint() {
        let mut sim = build_sim(42);
        let solution = sim.init().unwrap();

        let constraint = sim.next_step(&solution, 3);
        assert!(constraint.is_ok());
    }

    #[test]
    fn next_step_alternates_even_odd_behavior() {
        let mut sim = build_sim(42);
        let solution = sim.init().unwrap();

        let odd = sim.next_step(&solution, 3).unwrap();
        let even = sim.next_step(&solution, 4).unwrap();

        // They should not both be identical constraint types
        assert_ne!(odd.type_str(), even.type_str());
    }

    #[test]
    fn apply_constraint_reduces_or_keeps_possibilities() {
        let mut sim = build_sim(42);
        let solution = sim.init().unwrap();

        let initial_len = sim.possibilities.len();

        let constraint = sim.next_step(&solution, 3).unwrap();
        sim.apply_constraint(constraint).unwrap();

        assert!(sim.possibilities.len() <= initial_len);
        assert!(!sim.constraints.is_empty());
    }

    #[test]
    fn run_loop_terminates_when_one_possibility_left() {
        let mut sim = build_sim(42);
        let solution = sim.init().unwrap();

        let result = sim.run_loop(&solution);

        assert!(result.is_ok());
        assert!(sim.possibilities.len() <= 1);
    }

    #[test]
    fn run_produces_simulation_result() {
        let sim = build_sim(42);

        let result = sim.run();
        assert!(result.is_ok());

        let res = result.unwrap();
        assert_eq!(res.identifier(), 1);
    }

    #[test]
    fn same_seed_produces_same_iteration_count() {
        let sim1 = build_sim(123);
        let sim2 = build_sim(123);

        let r1 = sim1.run().unwrap();
        let r2 = sim2.run().unwrap();

        assert_eq!(r1.identifier(), r2.identifier());
    }
}

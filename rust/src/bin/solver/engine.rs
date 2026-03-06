// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module offers the functionality to run a single simulation. It is so to speak the engine
//! which drives the benchmarking.

use std::sync::Arc;
use std::{collections::HashMap, time::Instant};

use anyhow::{Context, Result};
use rand::rngs::StdRng;
use rust_decimal::{dec, Decimal};

use ayto::constraint::{check_type::CheckType, Constraint, ConstraintSim, ConstraintType};
use ayto::matching_repr::MaskedMatching;
use ayto::ruleset::RuleSet;
use ayto::{LightCnt, Lut, Rem};

use crate::init::{create_iteration_state, generate_solution};
use crate::result::SimulationResult;
use crate::rng::create_rng;
use crate::strategies::StrategyBundle;
use crate::trail::{constraint_type_order, CT};
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
    /// maximum number of iterations
    max: Option<usize>,

    /////////////////////////////////////////////////////
    // fields which are modified during the simulation //
    /////////////////////////////////////////////////////
    /// accumulated list of constraints used to solve this game
    constraints: Vec<Constraint>,
    /// list of all remaining possible solutions
    pub possibilities: Vec<MaskedMatching>,
    /// a table tracking the remaining possibilities per 1:1 matching
    rem: Rem,
    /// The amount of lights which are already *known* (proven in a MB decision) up to this point
    lights_known_before: LightCnt,
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
            max: None,
        }
    }

    /// Constructor which already initialites the simulation with user supplied information
    ///
    /// Basically new + init but init only with
    /// - create_iteration_state
    /// - iter_perms
    /// - initialize some internal fields
    #[allow(clippy::too_many_arguments)]
    pub fn new_user_initialized(sim_id: usize, seed: u64, strategy: Arc<S>, ruleset: RuleSet, constraints: Vec<Constraint>, lights_known: LightCnt, lut_a: Lut, max: Option<usize>) -> Result<Self> {
        let (constraints, possibilities, rem) = Self::perform_initial_permutations(constraints, &lut_a, &ruleset)?;

        Ok(Self {
            ruleset,
            sim_id,
            seed,
            strategy,
            rng: create_rng(seed),
            start: Instant::now(),
            constraints,
            possibilities,
            rem,
            lights_known_before: lights_known,
            max,
        })
    }

    /// Based on the initial constraints and the ruleset, compute all possible solutions
    fn perform_initial_permutations(constraints: Vec<Constraint>, lut: &Lut, rs: &RuleSet) -> Result<(Vec<Constraint>, Vec<MaskedMatching>, Rem)> {
        let mut iter_state = create_iteration_state(constraints)?;

        // use the ruleset for the first constraint
        rs.iter_perms(lut, &HashMap::new(), &mut iter_state, &None)?;

        let mut rem: Rem = (iter_state.each, iter_state.total);
        for c in iter_state.constraints.iter_mut() {
            rem = c
                .apply_to_rem(rem)
                .context("Apply to rem failed")?;
        }

        Ok((
            iter_state.constraints,
            iter_state.left_poss,
            rem,
        ))
    }

    /// Initializes the simulation state and computes the initial possibility space.
    ///
    /// Returns the solution
    fn init(&mut self) -> Result<MaskedMatching> {
        let solution = generate_solution(&mut self.rng);
        let mut constraints = vec![];

        while let Some((m, ct)) = self.strategy.initial_value(&constraints)? {
            let lights = m.calculate_lights(&solution);

            let c = Constraint::new_with_defaults(
                ct,
                CheckType::Lights(lights, Default::default()),
                m,
                self.ruleset.init_data()?,
                NUM_PLAYERS_SET_A,
                NUM_PLAYERS_SET_A,
                self.lights_known_before as LightCnt,
            );
            self.lights_known_before += c.added_known_lights();
            constraints.push(c);
        }

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

        (
            self.constraints,
            self.possibilities,
            self.rem,
        ) = Self::perform_initial_permutations(constraints, &lut, &self.ruleset)?;

        Ok(solution)
    }

    /// Full simulation execution.
    pub fn run(mut self) -> Result<SimulationResult> {
        let solution = self.init()?;
        self.run_loop(&solution, self.max)?;
        self.try_to_result(solution)
    }

    /// Executes simulation loop.
    fn run_loop(&mut self, solution: &MaskedMatching, max: Option<usize>) -> Result<()> {
        for i in 0.. {
            if let Some(limit) = max {
                if i >= limit {
                    break
                }
            }

            let constraint = self.next_step(solution)?.1;

            self.apply_constraint(constraint)?;

            if self.possibilities.len() <= 1 {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Generates and constructs the next constraint
    /// according to the selected strategy and iteration number/index.
    pub(super) fn next_step(&mut self, solution: &MaskedMatching) -> Result<(f64, Constraint)> {
        let iteration = self.constraints.len();
        let num = (Decimal::from(iteration) / dec![2]).floor();
        let comment = String::new();
        let offer = None;

        let ((h, m), ct) = match constraint_type_order(iteration) {
            CT::Box => {
                // this is a match-box decision
                let m = self
                    .strategy
                    .choose_mb(&self.rem.0, self.rem.1, &mut self.rng);

                let ct = ConstraintType::Box {
                    num,
                    comment,
                    offer,
                };

                ((0.0, m), ct)
            },
            CT::Night => {
                // this is a matching-night
                let m = self.strategy.choose_mn(&self.possibilities, &mut self.rng);

                let ct = ConstraintType::Night {
                    num,
                    comment,
                    offer,
                };

                (m, ct)
            },
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
        self.lights_known_before += c.added_known_lights();
        Ok((h, c))
    }

    /// Applies a new constraint to:
    /// - Possibility space (without new allocations) -- Order is not preserved
    /// - Remaining counts
    /// - Constraint list
    fn apply_constraint(&mut self, mut constraint: Constraint) -> Result<()> {
        let mut i = 0;

        while i < self.possibilities.len() {
            if constraint.process(&self.possibilities[i])? {
                i += 1;
            } else {
                // does not retain order!
                self.possibilities.swap_remove(i);
            }
        }

        self.rem = constraint
            .apply_to_rem(self.rem.clone())
            .context("Apply to rem failed")?;
        self.constraints.push(constraint);

        Ok(())
    }

    /// convert the finished simulation to a simulation-result which can be serialized and stored
    /// on disk
    fn try_to_result(self, solution: MaskedMatching) -> Result<SimulationResult> {
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
            solution,
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
        fn initial_value(&self, constraints: &[Constraint]) -> Result<Option<(MaskedMatching, ConstraintType)>> {
            if !constraints.is_empty() {
                return Ok(None);
            }
            // Single deterministic matching
            let ids: Vec<IdBase> = (0..NUM_PLAYERS_SET_A as IdBase).collect();
            Ok(Some((
                MaskedMatching::from(ids.as_slice()),
                ConstraintType::Box{num: dec![0], offer: None, comment: "gg".to_string()},
            )))
        }

        fn choose_mb(
            &self,
            _data: &[Vec<u128>],
            _total: u128,
            _rng: &mut dyn Rng,
        ) -> MaskedMatching {
            self.initial_value(&[]).unwrap().unwrap().0
        }

        fn choose_mn(&self, left_poss: &[MaskedMatching], _rng: &mut dyn Rng) -> (f64, MaskedMatching) {
            (1.0, left_poss.first().cloned().expect("no possibilities left"))
        }
    }

    fn build_sim(seed: u64) -> Simulation<DeterministicStrategy> {
        Simulation::new(1, seed, Arc::new(DeterministicStrategy), RuleSet::Eq)
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

        let x = sim.next_step(&solution);
        assert!(x.is_ok());
    }

    #[test]
    fn next_step_alternates_even_odd_behavior() {
        let mut sim = build_sim(42);
        let solution = sim.init().unwrap();

        // in order to generate the 3rd and 4th one later
        sim.constraints.push(Constraint::default());
        sim.constraints.push(Constraint::default());

        let odd = sim.next_step(&solution).unwrap();
        let odd_t = odd.1.type_str();

        sim.constraints.push(odd.1);

        let even = sim.next_step(&solution).unwrap();

        // They should not both be identical constraint types
        assert_ne!(odd_t, even.1.type_str());
    }

    #[test]
    fn apply_constraint_reduces_or_keeps_possibilities() {
        let mut sim = build_sim(42);
        let solution = sim.init().unwrap();

        let initial_len = sim.possibilities.len();

        // in order to generate the 3rd constraint
        sim.constraints.push(Constraint::default());
        sim.constraints.push(Constraint::default());
        let constraint = sim.next_step(&solution).unwrap();
        sim.apply_constraint(constraint.1).unwrap();

        assert!(sim.possibilities.len() <= initial_len);
        assert!(!sim.constraints.is_empty());
    }

    #[test]
    fn run_loop_terminates_when_one_possibility_left() {
        let mut sim = build_sim(42);
        let solution = sim.init().unwrap();

        let result = sim.run_loop(&solution, None);

        assert!(result.is_ok());
        assert!(sim.possibilities.len() <= 1);
    }

    #[test]
    fn run_produces_simulation_result() {
        let sim = build_sim(42);

        let result = sim.run();
        assert!(result.is_ok());

        let _res = result.unwrap();
    }

    #[test]
    fn same_seed_produces_same_iteration_count() {
        let sim1 = build_sim(123);
        let sim2 = build_sim(123);

        let _r1 = sim1.run().unwrap();
        let _r2 = sim2.run().unwrap();
    }

    #[test]
    fn perform_initial_permutations_without_constraints() {
        let constraints = vec![];
        let lut: Lut = (0..NUM_PLAYERS_SET_A)
            .map(|i| ((b'a' + i as u8) as char).to_string())
            .enumerate()
            .map(|(i, s)| (s, i))
            .collect();

        let rs = RuleSet::default();

        let (returned_constraints, possibilities, rem) = Simulation::<DeterministicStrategy>::perform_initial_permutations(constraints, &lut, &rs).unwrap();

        // no constraints should remain none
        assert!(returned_constraints.is_empty());

        // remaining possibilities should match total permutations observed
        assert_eq!(possibilities.len() as u128, rem.1);
        assert_eq!(possibilities.len(), rs.get_perms_amount(lut.len(), lut.len(), &None).unwrap());

        // rem table should exist and not be empty
        assert!(!rem.0.is_empty());

        // total permutations should be > 0
        assert!(rem.1 > 0);
    }
}

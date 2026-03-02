// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements the struct which collects all statistics collected during the simulation
//! which are stored on disk for a later evaluation.

use serde::{Deserialize, Serialize};

use ayto::constraint::compare::EvalEvent;

/// Collects the results of one entire simulation (playing the game once)
#[derive(Serialize, Deserialize)]
pub(super) struct SimulationResult {
    /// identifier of the simulation, can be used for tracking
    sim_id: usize,
    /// seed used for the randomness in the simulation
    seed: u64,
    /// stats on the events that were generated in order to solve the game
    stats: Vec<EvalEvent>,
    /// How many iterations ([MB, MN, MB, MN] would be 4) it took to come to a solution
    iterations_count: usize,
    /// How long the simulation ran in miliseconds
    duration_ms: u128,
}

impl SimulationResult {
    /// Create a new SimulationResult
    pub(super) fn new(
        sim_id: usize,
        seed: u64,
        stats: Vec<EvalEvent>,
        iterations_count: usize,
        duration_ms: u128,
    ) -> Self {
        Self {
            sim_id,
            seed,
            stats,
            iterations_count,
            duration_ms,
        }
    }
}

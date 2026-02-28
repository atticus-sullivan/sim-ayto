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
    sim_id: usize,
    seed: u64,
    stats: Vec<EvalEvent>,
    iterations_count: usize,
    duration_ms: u128,
}

impl SimulationResult {
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

    pub(super) fn identifier(&self) -> usize {
        self.sim_id
    }
}

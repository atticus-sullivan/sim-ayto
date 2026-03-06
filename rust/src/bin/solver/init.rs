// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains some helper functions used when initializing the simulation.

use std::collections::HashSet;

use anyhow::Result;
use ayto::progressbar::MockProgressBar;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

use ayto::constraint::Constraint;
use ayto::iterstate::IterState;
use ayto::matching_repr::{IdBase, MaskedMatching};

use crate::NUM_PLAYERS_SET_A;

/// Generates a random solution and converts it into `MaskedMatching`.
pub(super) fn generate_solution(rng: &mut StdRng) -> MaskedMatching {
    let mut solution: Vec<IdBase> = (0..NUM_PLAYERS_SET_A).map(|x| x as IdBase).collect();
    solution.shuffle(rng);
    (*solution).into()
}

/// Creates the initial `IterState` for a simulation based on
/// the first constraint.
///
/// This prepares the permutation engine and initializes
/// possibility tracking.
///
/// # Arguments
/// - `constraint` - The initial constraint of the simulation
///
/// # Returns
/// An initialized (but not executed) `IterState`.
pub(super) fn create_iteration_state(
    constraints: Vec<Constraint>,
) -> Result<IterState<MockProgressBar, Constraint>> {
    IterState::new(
        true,
        NUM_PLAYERS_SET_A,
        constraints,
        &[],
        &(HashSet::new(), HashSet::new()),
        &None,
        (NUM_PLAYERS_SET_A, NUM_PLAYERS_SET_A),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ayto::matching_repr::bitset::Bitset;
    use pretty_assertions::assert_eq;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn generate_solution_has_correct_length() {
        let mut rng = StdRng::seed_from_u64(42);
        let solution = generate_solution(&mut rng);

        assert_eq!(solution.len(), NUM_PLAYERS_SET_A);
    }

    #[test]
    fn generate_solution_is_permutation() {
        let mut rng = StdRng::seed_from_u64(42);
        let solution = generate_solution(&mut rng);

        // all values are set
        for i in 0..NUM_PLAYERS_SET_A as u8 {
            assert!(solution.contains_mask(Bitset::from_idxs(&[i])));
        }
    }

    #[test]
    fn generate_solution_is_deterministic_for_same_seed() {
        let mut rng1 = StdRng::seed_from_u64(123);
        let mut rng2 = StdRng::seed_from_u64(123);

        let s1 = generate_solution(&mut rng1);
        let s2 = generate_solution(&mut rng2);

        assert_eq!(s1, s2);
    }
}

use std::collections::HashSet;

use anyhow::Result;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rust_decimal::dec;

use ayto::constraint::{CheckType, Constraint, ConstraintType};
use ayto::iterstate::IterState;
use ayto::matching_repr::MaskedMatching;
use ayto::ruleset::RuleSet;

use crate::NUM_PLAYERS_SET_A;

/// Generates a random solution and converts it into `MaskedMatching`.
pub(crate) fn generate_solution(rng: &mut StdRng) -> MaskedMatching {
    let mut solution: Vec<u8> = (0..NUM_PLAYERS_SET_A).map(|x| x as u8).collect();
    solution.shuffle(rng);
    (*solution).into()
}

/// Builds the very first constraint of a simulation.
///
/// This determines whether the first constraint is a `Box` or `Night`
/// constraint depending on the length of the initial matching.
///
/// # Arguments
/// - `matching` - The initial matching selected by the strategy
/// - `lights` - The number of lights calculated for the solution
/// - `ruleset` - The active rule set
/// - `lights_known_before` - Number of already known lights before this step
///
/// # Returns
/// A fully initialized `Constraint`.
pub(crate) fn build_initial_constraint(
    matching: MaskedMatching,
    lights: usize,
    ruleset: &RuleSet,
    lights_known_before: usize,
) -> Result<Constraint> {
    let constraint_type = if matching.len() == 1 {
        ConstraintType::Box {
            num: dec![1.0],
            comment: String::new(),
            offer: None,
        }
    } else {
        ConstraintType::Night {
            num: dec![1.0],
            comment: String::new(),
            offer: None,
        }
    };

    Ok(Constraint::new_with_defaults(
        constraint_type,
        CheckType::Lights(lights as u8, Default::default()),
        matching,
        ruleset.init_data()?,
        NUM_PLAYERS_SET_A,
        NUM_PLAYERS_SET_A,
        lights_known_before as u8,
    ))
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
/// A fully initialized `IterState`.
pub(crate) fn create_iteration_state(constraint: &Constraint) -> Result<IterState> {
    IterState::new(
        true,
        NUM_PLAYERS_SET_A,
        vec![constraint.clone()],
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
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    // ------------------------------------------------------------
    // generate_solution
    // ------------------------------------------------------------

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

        let mut seen = std::collections::HashSet::new();
        for i in 0..NUM_PLAYERS_SET_A as u8 {
            assert!(solution.contains_mask(Bitset::from_idxs(&[i])));
            seen.insert(i);
        }

        assert_eq!(seen.len(), NUM_PLAYERS_SET_A);
    }

    #[test]
    fn generate_solution_is_deterministic_for_same_seed() {
        let mut rng1 = StdRng::seed_from_u64(123);
        let mut rng2 = StdRng::seed_from_u64(123);

        let s1 = generate_solution(&mut rng1);
        let s2 = generate_solution(&mut rng2);

        assert_eq!(s1, s2);
    }

    // ------------------------------------------------------------
    // build_initial_constraint
    // ------------------------------------------------------------

    #[test]
    fn build_initial_constraint_box_if_len_one() {
        let ruleset = RuleSet::Eq;

        // Matching of length 1 triggers Box constraint
        let matching = MaskedMatching::from(&[0u8][..]);

        let constraint = build_initial_constraint(
            matching,
            1,
            &ruleset,
            0,
        ).unwrap();

        assert!(constraint.is_mb());
    }

    #[test]
    fn build_initial_constraint_night_if_len_gt_one() {
        let ruleset = RuleSet::Eq;

        let matching = MaskedMatching::from(
            &vec![(0..NUM_PLAYERS_SET_A as u8).collect::<Vec<_>>()]
        );

        let constraint = build_initial_constraint(
            matching,
            2,
            &ruleset,
            0,
        ).unwrap();

        assert!(constraint.is_mb());
    }

    #[test]
    fn build_initial_constraint_sets_lights_correctly() {
        let ruleset = RuleSet::Eq;
        let matching = MaskedMatching::from(
            vec![(0..NUM_PLAYERS_SET_A as u8).collect::<Vec<_>>()]
        );

        let constraint = build_initial_constraint(
            matching,
            3,
            &ruleset,
            0,
        ).unwrap();

        assert_eq!(constraint.check.as_lights(), Some(3));
    }

    // ------------------------------------------------------------
    // create_iteration_state
    // ------------------------------------------------------------

    #[test]
    fn create_iteration_state_initializes_properly() {
        let ruleset = RuleSet::Eq;

        let matching = MaskedMatching::from(
            vec![(0..NUM_PLAYERS_SET_A as u8).collect::<Vec<_>>()]
        );

        let constraint = build_initial_constraint(
            matching,
            0,
            &ruleset,
            0,
        ).unwrap();

        let iter_state = create_iteration_state(&constraint);

        assert!(iter_state.is_ok());

        let state = iter_state.unwrap();

        // initial constraint should be present
        assert_eq!(state.constraints.len(), 1);
    }
}

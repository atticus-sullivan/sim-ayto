// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module-tree implements different strategies to play the game.

pub(super) mod mb;
pub(super) mod mn;

use anyhow::Result;
use ayto::{constraint::{check_type::CheckType, evaluate_predicates::ConstraintEval, Constraint, ConstraintGetters, ConstraintType}, matching_repr::{bitset::Bitset, MaskedMatching}};
use rand::Rng;
use rust_decimal::{dec, Decimal};

use crate::{strategies::{mb::MbOptimizer, mn::MnOptimizer}, trail::{constraint_type_order, CT}};

/// A single trait that groups both MB and MN strategy behaviour
/// and provides an initial value for a set of perms.
///
/// - `choose_mb`: choose a (u8,u8) MB pair
/// - `choose_mn`: choose a Vec<u8> MN matching
/// - `initial_value`: produce an initial HashMap
pub(super) trait StrategyBundle: Send + Sync {
    /// come up with a matching for a match-box
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, rng: &mut dyn Rng) -> MaskedMatching;
    /// come up with a full-matching for a matching night, also return the H
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> (f64, MaskedMatching);

    /// Produce the next constraint in the sequence of initial constraints. In particular, this
    /// sequence is generated without having extended information about the remaining permutations.
    /// The decision is only based on knowledge about the past (already generated) constraints and
    /// how many lights they produced.
    ///
    /// Like an iterator, this returns None if no more initial constraints can be generated
    fn initial_value(&self, constraints: &[Constraint]) -> Result<Option<(MaskedMatching, ConstraintType)>>;
}

/// Combines the different strategies needed.
pub(super) struct Strategy<S: MbOptimizer, T: MnOptimizer> {
    /// The match-box solver
    pub(super) mb: S,
    /// The matching-night solver
    pub(super) mn: T,
}

impl<S, T> StrategyBundle for Strategy<S, T>
where
    S: MbOptimizer,
    T: MnOptimizer,
{
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, rng: &mut dyn Rng) -> MaskedMatching {
        // delegate to the real implementation
        self.mb.choose_mb(data, total, rng)
    }

    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> (f64, MaskedMatching) {
        // delegate to the real implementation
        self.mn.choose_mn(left_poss, rng)
    }

    fn initial_value(&self, constraints: &[Constraint]) -> Result<Option<(MaskedMatching, ConstraintType)>> {
        let i = constraints.len();
        let num = (Decimal::from(i)/ dec![2]).floor() + dec![1];
        let comment = "".to_string();
        let offer = None;

        let match00 = MaskedMatching::from_masks(
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                .into_iter()
                .map(Bitset::from_word)
                .collect(),
        );

        let next_ct = constraint_type_order(i);

        Ok(
            match constraints {
                [] if next_ct == CT::Box => {
                    // match (0,0)
                    Some((
                        match00,
                        ConstraintType::Box { num, comment, offer },
                    ))
                },
                [c] if next_ct == CT::Night
                && c.is_mb()
                && matches!(c.check, CheckType::Lights(..))
                && c.is_match_found()
                && c.matching() == &match00 => {
                     // best to use one which does contain the known match (0,0)
                    let m = MaskedMatching::from_masks(
                        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
                            .into_iter()
                            .map(|i| Bitset::from_word(1 << i))
                            .collect(),
                    );
                    Some((
                        m,
                        ConstraintType::Night {num, comment, offer}
                    ))
                },
                [c] if next_ct == CT::Night
                && c.is_mb()
                && matches!(c.check, CheckType::Lights(..))
                && !c.is_match_found()
                && c.matching() == &match00 => {
                    // best to use one which does not contain the known no-match (0,0)
                    let m = MaskedMatching::from_masks(
                        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]
                            .into_iter()
                            .map(|i| Bitset::from_word(1 << i))
                            .collect(),
                    );
                    Some((
                        m,
                        ConstraintType::Night {num, comment, offer}
                    ))
                },
                _ => None,
            }
        )
    }
}

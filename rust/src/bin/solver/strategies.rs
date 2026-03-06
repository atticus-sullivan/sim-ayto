// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module-tree implements different strategies to play the game.

pub(super) mod mb;
pub(super) mod mn;

use anyhow::{ensure, Result};
use ayto::{constraint::{evaluate_predicates::ConstraintEval, Constraint}, matching_repr::{bitset::Bitset, MaskedMatching}};
use rand::Rng;

use crate::strategies::{mb::MbOptimizer, mn::MnOptimizer};

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

    /// Produce an initial value for the first constraint. Up to this point no information is known
    fn initial_value(&self, constraints: &[Constraint]) -> Result<Option<MaskedMatching>>;
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

    fn initial_value(&self, constraints: &[Constraint]) -> Result<Option<MaskedMatching>> {
        Ok(match constraints.len() {
            0 => {
                // match (0,0)
                Some(MaskedMatching::from_masks(
                    vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                        .into_iter()
                        .map(Bitset::from_word)
                        .collect(),
                ))
            },
            1 => {
                let last = constraints.first().unwrap();
                ensure!(last.is_mb(), "First event should be a match-box");

                if last.is_match_found() {
                    // best to use one which does contain the known match (0,0)
                    Some(MaskedMatching::from_masks(
                        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
                            .into_iter()
                            .map(|i| Bitset::from_word(1 << i))
                            .collect(),
                    ))
                } else {
                    // best to use one which does not contain the known no-match (0,0)
                    Some(MaskedMatching::from_masks(
                        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]
                            .into_iter()
                            .map(|i| Bitset::from_word(1 << i))
                            .collect(),
                    ))
                }
            },
            _ => None,
        })
    }
}

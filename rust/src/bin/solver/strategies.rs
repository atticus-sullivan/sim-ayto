// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module-tree implements different strategies to play the game.

pub(super) mod mb;
pub(super) mod mn;

use ayto::matching_repr::{bitset::Bitset, MaskedMatching};
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
    /// come up with a full-matching for a matching night
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching;

    /// Produce an initial value for the first constraint. Up to this point no information is known
    fn initial_value(&self) -> MaskedMatching;
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
}

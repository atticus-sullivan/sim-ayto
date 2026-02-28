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
///
/// The `usize` value is a practical default; change the return type if you want another payload.
pub(super) trait StrategyBundle: Send + Sync {
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, rng: &mut dyn Rng) -> MaskedMatching;
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching;

    /// Produce an initial value for the first constraint. Up to this point no information is known
    fn initial_value(&self) -> MaskedMatching;
}

/// Combines the different strategies needed.
pub(super) struct Strategy<S: MbOptimizer, T: MnOptimizer> {
    pub(super) mb: S,
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

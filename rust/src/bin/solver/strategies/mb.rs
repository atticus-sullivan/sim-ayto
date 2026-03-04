// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Optimize/Select a matching to place in a Match-Box

pub(crate) mod optimal;

use rand::Rng;

use ayto::matching_repr::MaskedMatching;

/// Chooses an MB.
///
/// `data` is the table with how many remaining solutions are with this 1:1 match. Together with
/// `total` this can be converted to percentages.
pub(crate) trait MbOptimizer: Send + Sync {
    /// Come up with a matching for a match-box according to the respective strategy
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, rng: &mut dyn Rng) -> MaskedMatching;
}

// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Optimize/Select a full matching to seat at the Matching-Night

pub(crate) mod entropy_left;

use ayto::matching_repr::MaskedMatching;
use rand::Rng;

/// Chooses an MN
pub(crate) trait MnOptimizer: Send + Sync {
    /// Come up with a full-matching for a matching-night according to the strategy
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching;
}

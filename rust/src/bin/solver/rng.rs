// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Helper function which builds an (reproducible) rng from a seed

use rand::{rngs::StdRng, SeedableRng};

/// Creates a reproducible RNG for a simulation.
pub(super) fn create_rng(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

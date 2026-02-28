//! Helper function which builds an (reproducible) rng from a seed

use rand::{rngs::StdRng, SeedableRng};

/// Creates a reproducible RNG for a simulation.
pub(super) fn create_rng(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

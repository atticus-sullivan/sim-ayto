pub(crate) mod entropy_left;

use ayto::matching_repr::MaskedMatching;
use rand::Rng;

/// Chooses an MN
pub(crate) trait MnOptimizer: Send + Sync {
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching;
}

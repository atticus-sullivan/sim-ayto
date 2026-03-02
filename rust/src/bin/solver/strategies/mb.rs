pub(crate) mod optimal;

use rand::Rng;

use ayto::matching_repr::MaskedMatching;

/// Chooses an MB.
/// `data` has the structure you provided earlier (Vec<Vec<u128>>).
pub(crate) trait MbOptimizer: Send + Sync {
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, rng: &mut dyn Rng) -> MaskedMatching;
}

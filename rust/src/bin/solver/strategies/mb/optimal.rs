//! An implementation for an optimizer for Match-Box decisions.
//! Selects the optimal match to place in the Match-Box. The optimum is the match which is closest
//! to 50% probability.

use ayto::matching_repr::{IdBase, MaskedMatching};
use rand::Rng;

use crate::strategies::mb::MbOptimizer;

/// Selects the optimal match to place in the Match-Box. The optimum is the match which is closest
/// to 50% probability.
pub(crate) struct OptimalMbOptimizer;

impl MbOptimizer for OptimalMbOptimizer {
    fn choose_mb(&self, data: &[Vec<u128>], total: u128, _rng: &mut dyn Rng) -> MaskedMatching {
        let target = total / 2; // that is the optimum we want to be close
        let mut closest_diff = u128::MAX;
        let mut closest_index = (0u8, 0u8);

        for (i, row) in data.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                let diff = val.abs_diff(target);
                if diff < closest_diff {
                    closest_diff = diff;
                    closest_index = (i as IdBase, j as IdBase);
                }
            }
        }
        closest_index.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ayto::matching_repr::MaskedMatching;
    use pretty_assertions::assert_eq;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn choose_mb_closest_to_half_total() {
        let optimizer = OptimalMbOptimizer;

        let data = vec![vec![10, 20, 30], vec![40, 50, 60], vec![70, 80, 90]];
        let total = 100; // target = 50
        let mut rng = StdRng::seed_from_u64(1);

        let selected: MaskedMatching = optimizer.choose_mb(&data, total, &mut rng);

        // The element closest to 50% is 50 at (1,1)
        let expected: MaskedMatching = (1u8, 1u8).into();
        assert_eq!(selected, expected);
    }

    #[test]
    fn choose_mb_first_if_multiple_equal_closest() {
        let optimizer = OptimalMbOptimizer;

        let data = vec![vec![49, 51]];
        let total = 100u128;
        let mut rng = StdRng::seed_from_u64(1);

        let selected: MaskedMatching = optimizer.choose_mb(&data, total, &mut rng);

        // It should pick the first closest, i.e., 49 at (0,0)
        let expected: MaskedMatching = (0u8, 0u8).into();
        assert_eq!(selected, expected);
    }

    #[test]
    fn choose_mb_single_element() {
        let optimizer = OptimalMbOptimizer;

        let data = vec![vec![42]];
        let total = 100u128;
        let mut rng = StdRng::seed_from_u64(1);

        let selected: MaskedMatching = optimizer.choose_mb(&data, total, &mut rng);
        let expected: MaskedMatching = (0u8, 0u8).into();
        assert_eq!(selected, expected);
    }
}

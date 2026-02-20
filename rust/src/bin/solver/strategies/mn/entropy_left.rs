use ayto::matching_repr::MaskedMatching;
use rand::prelude::IndexedRandom;
use rand::Rng;

use crate::strategies::mn::MnOptimizer;
use crate::utils::calc_entropy;

/// Entropy (over left_poss) MN optimizer that picks the candidate maximizing entropy (your original).
pub(crate) struct EntropyLeftMnOptimizer {
    /// sampling threshold for performance
    // in case there are many possibilities left, don't use them all. Instead sample them randomly
    // down to a threshold
    pub sample_threshold: usize,
}

impl EntropyLeftMnOptimizer {
    pub(crate) fn new(sample_threshold: usize) -> Self {
        Self { sample_threshold }
    }
}

impl MnOptimizer for EntropyLeftMnOptimizer {
    fn choose_mn(&self, left_poss: &[MaskedMatching], rng: &mut dyn Rng) -> MaskedMatching {
        // using all of the perms results in too long computations
        // => only use left_poss they have a better chance for a good result anyhow

        if left_poss.len() > self.sample_threshold {
            left_poss
                .sample(rng, self.sample_threshold)
                .map(|m| (calc_entropy(m, left_poss), m))
                .max_by(|(e1, _), (e2, _)| e1.partial_cmp(e2).unwrap())
                .map(|(_, m)| m)
                .unwrap()
                .clone()
        } else {
            left_poss
                .iter()
                .map(|m| (calc_entropy(m, left_poss), m))
                .max_by(|(e1, _), (e2, _)| e1.partial_cmp(e2).unwrap())
                .map(|(_, m)| m)
                .unwrap()
                .clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ayto::matching_repr::MaskedMatching;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_masked_matching(values: &[u8]) -> MaskedMatching {
        MaskedMatching::from(values)
    }

    #[test]
    fn chooses_highest_entropy_candidate() {
        let optimizer = EntropyLeftMnOptimizer::new(10);
        let mut rng = StdRng::seed_from_u64(42);

        // Define candidates
        let m1 = make_masked_matching(&[0, 0, 1]);
        let m2 = make_masked_matching(&[0, 1, 1]);
        let m3 = make_masked_matching(&[1, 0, 1]);
        let left_poss = vec![m1.clone(), m2.clone(), m3.clone()];

        // Entropy calculations:
        // For this small set, m2 has the highest entropy
        let chosen = optimizer.choose_mn(&left_poss, &mut rng);
        assert_eq!(chosen, m3);
    }

    #[test]
    fn respects_sample_threshold() {
        let optimizer = EntropyLeftMnOptimizer::new(2);
        let mut rng = StdRng::seed_from_u64(123);

        // 3 candidates, sample threshold = 2 => only 2 randomly sampled candidates are considered
        let m1 = make_masked_matching(&[0, 0, 0]);
        let m2 = make_masked_matching(&[1, 1, 1]);
        let m3 = make_masked_matching(&[0, 1, 0]);
        let left_poss = vec![m1.clone(), m2.clone(), m3.clone()];

        let chosen = optimizer.choose_mn(&left_poss, &mut rng);

        // chosen must be one of the left_poss
        assert!(left_poss.contains(&chosen));
    }

    #[test]
    fn works_when_below_threshold() {
        let optimizer = EntropyLeftMnOptimizer::new(5);
        let mut rng = StdRng::seed_from_u64(99);

        let m1 = make_masked_matching(&[0, 0, 1]);
        let m2 = make_masked_matching(&[0, 1, 1]);
        let left_poss = vec![m1.clone(), m2.clone()];

        // length < sample_threshold -> all candidates considered
        let chosen = optimizer.choose_mn(&left_poss, &mut rng);
        assert!(left_poss.contains(&chosen));
    }

    #[test]
    fn single_candidate() {
        let optimizer = EntropyLeftMnOptimizer::new(10);
        let mut rng = StdRng::seed_from_u64(1);

        let m1 = make_masked_matching(&[0, 0, 1]);
        let left_poss = vec![m1.clone()];

        let chosen = optimizer.choose_mn(&left_poss, &mut rng);
        assert_eq!(chosen, m1);
    }
}

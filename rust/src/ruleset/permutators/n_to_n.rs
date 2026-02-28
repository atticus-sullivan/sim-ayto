//! This module implements a generator for n-to-n matchings

use anyhow::ensure;
use permutator::{Combination, Permutation};

use crate::matching_repr::{IdBase, bitset::Bitset};

/// In-place generator for N-to-N assignments.
///
/// Semantics:
/// - `slots` must be even, `k = slots / 2`.
/// - For every choice `ks` of `k` slot indices (combination) select a permutation of the remaining
///   `k` values `vs` and place `vs[i]` as the singleton at slot `ks[i]`.
/// - This emitter produces `&[Bitset]` buffers where only the `ks` positions are singletons; other
///   positions are empty `Bitset::empty()`.
///
/// Notes:
/// - To avoid allocations, permutations of `vs` are generated in-place using `heaps_permute`.
/// - The function uses a reusable `c` output buffer which is overwritten for each emission.
/// - Reject assignments where the slot index is less than or equal to the value index, because such a pairing would be symmetric with another generated permutation
#[inline]
pub(crate) fn n_to_n_inplace<F>(slots: usize, mut emit: F) -> anyhow::Result<()>
where
    F: FnMut(&[Bitset]) -> anyhow::Result<()>,
{
    if slots == 0 {
        return Ok(());
    }
    ensure!(slots.is_multiple_of(2));

    let len = slots / 2;

    // Reusable output buffer: all empty initially.
    let mut c = vec![Bitset::empty(); slots];

    // Precompute full index set as u8 vector for perm/combo helpers
    let full_indices: Vec<IdBase> = (0..slots as IdBase).collect();

    // Iterate combinations of indices (ks)
    for ks in full_indices.combination(len) {
        // produce the list of remaining values (vs)
        let mut vs = (0..slots as IdBase)
            .filter(|x| !ks.contains(&x))
            .collect::<Vec<_>>();

        // For every permutation of remaining values
        for perm_vs in vs.permutation() {
            // reset buffer c to empty
            for elem in &mut c {
                *elem = Bitset::empty();
            }

            // put assigned values into ks positions (zip ks and perm_vs)
            let mut ok = true;
            for (k_u8, v_u8) in ks.iter().zip(perm_vs.iter()) {
                let k = **k_u8 as usize;
                let v = *v_u8;
                // original semantics had a guard `if k <= v { return None }` (reject)
                // so we keep the same check:
                if (k as IdBase) <= v {
                    ok = false;
                    break;
                }
                c[k] = Bitset::from_idxs(&[v]);
            }
            if ok {
                emit(&c)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn check_invariance_singleton(m: &[Bitset]) -> bool {
        m.iter().all(|bs| bs.is_empty() || bs.is_singleton())
    }

    fn check_invariance_ordering(m: &[Bitset]) -> bool {
        m.iter()
            .enumerate()
            .all(|(i, bs)| bs.is_empty() || i > bs.single_idx().unwrap() as usize)
    }

    #[test]
    fn n_to_n_inplace_zero() -> anyhow::Result<()> {
        let mut out = Vec::new();

        n_to_n_inplace(0, |s| {
            out.push(s.to_vec());
            Ok(())
        })?;

        assert!(out.is_empty());
        Ok(())
    }

    #[test]
    fn n_to_n_inplace_odd() -> anyhow::Result<()> {
        let mut out = Vec::new();

        // num = 5 => rejected
        let res = n_to_n_inplace(5, |s| {
            out.push(s.to_vec());
            Ok(())
        });

        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn n_to_n_inplace_two() -> anyhow::Result<()> {
        let mut out = Vec::new();

        n_to_n_inplace(2, |s| {
            out.push(s.to_vec());
            Ok(())
        })?;

        let expected = vec![vec![
            Bitset::from_idxs(&[]), // 0 matches to none
            Bitset::from_idxs(&[0]),
        ]];

        assert_eq!(out, expected);
        assert!(out.iter().all(|i| check_invariance_ordering(i)));
        assert!(out.iter().all(|i| check_invariance_singleton(i)));
        Ok(())
    }

    #[test]
    fn n_to_n_inplace_four() -> anyhow::Result<()> {
        let mut out = Vec::new();

        n_to_n_inplace(4, |s| {
            out.push(s.to_vec());
            Ok(())
        })?;

        let expected = vec![
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[2]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[1]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[0]),
            ],
        ];

        assert_eq!(out, expected);
        assert!(out.iter().all(|i| check_invariance_ordering(i)));
        assert!(out.iter().all(|i| check_invariance_singleton(i)));
        Ok(())
    }

    #[test]
    fn n_to_n_inplace_six() -> anyhow::Result<()> {
        let mut out = Vec::new();

        n_to_n_inplace(6, |s| {
            out.push(s.to_vec());
            Ok(())
        })?;

        let expected = vec![
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[2]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[4]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[2]),
                Bitset::from_idxs(&[3]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[3]),
                Bitset::from_idxs(&[2]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[4]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[4]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[3]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[3]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[3]),
                Bitset::from_idxs(&[1]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[3]),
                Bitset::from_idxs(&[0]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[2]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[2]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[2]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[1]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[2]),
                Bitset::from_idxs(&[1]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[2]),
                Bitset::from_idxs(&[0]),
            ],
            vec![
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[2]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[0]),
            ],
        ];

        assert_eq!(out, expected);
        assert!(out.iter().all(|i| check_invariance_ordering(i)));
        assert!(out.iter().all(|i| check_invariance_singleton(i)));
        Ok(())
    }
}

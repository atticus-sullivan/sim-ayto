// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module handles all kinds of duplicates:
//! 1. set_a is extended which leads to a duplicate where the added element must be part of.
//!    set_b is extended which leads to a duplicate where it is unknown whether the added element is
//!    part of the duplicate or not.

use super::heaps_permute;
use crate::matching_repr::{bitset::Bitset, IdBase};

/// Generates all valid matchings for the AB-duplication ruleset.
///
/// Both sets are assumed to have the same length `total`, where the original
/// sets had size `n = total - 1`. The element at index `total - 1` in A is
/// treated as the added A element. The corresponding added B element is
/// likewise assumed to occupy index `total - 1`, although this function does
/// not need to refer to it explicitly.
///
/// Each generated matching contains exactly:
///
/// - one `2A -> 1B` extended pairing containing the added A element,
/// - one `1A -> 2B` extended pairing,
/// - `total - 3` ordinary `1A -> 1B` pairings.
///
/// Every valid matching is generated exactly once.
///
/// The supplied `buf` is used as reusable scratch storage. It is intentionally
/// **not restored** to its original contents when this function returns;
/// after successful completion it contains the last matching that was emitted.
/// The callback receives `buf` for each generated matching and may return an
/// error to stop generation early.
///
/// For `total < 3`, no matching can be generated and the function returns
/// successfully without calling `emit`.
#[inline]
pub(crate) fn ab_dup_inplace<F>(buf: &mut [Bitset], mut emit: F) -> anyhow::Result<()>
where
    F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
{
    let total = buf.len();

    // If original sets had size n, both extended sets have size n + 1.
    // The construction needs:
    //
    //   (2A -> 1B) + (1A -> 2B) + (n - 2) normal pairings
    //
    // so n >= 2 => total >= 3.
    if total < 3 {
        return Ok(());
    }

    let added_a = total - 1;
    // let _added_b = total - 1;

    // precompute singletons to reuse them later
    let singletons: Vec<Bitset> = (0..total as IdBase)
        .map(|i| Bitset::from_idxs(&[i]))
        .collect();

    // Scratch storage for remaining A/B indices.
    let mut remaining_a = Vec::with_capacity(total - 3);
    let mut remaining_b = Vec::with_capacity(total - 3);

    // We iterate over every A index except the mandatory added A.
    //
    //   added_a + a2 -> b_left
    //
    for a2 in 0..added_a {
        // Choose the A belonging to the compensating
        //
        //     a_right -> {b1, b2}
        //
        // among all A except added_a and a2.
        for a_right in 0..a2 {
            remaining_a.clear();
            remaining_a.extend(0..a_right);
            remaining_a.extend((a_right + 1)..a2);
            remaining_a.extend((a2 + 1)..added_a);
            generate_for_a_pair(
                buf,
                total,
                added_a,
                a2,
                a_right,
                &singletons,
                &mut remaining_a,
                &mut remaining_b,
                &mut emit,
            )?;
        }
        for a_right in (a2 + 1)..added_a {
            remaining_a.clear();
            remaining_a.extend(0..a2);
            remaining_a.extend((a2 + 1)..a_right);
            remaining_a.extend((a_right + 1)..added_a);
            generate_for_a_pair(
                buf,
                total,
                added_a,
                a2,
                a_right,
                &singletons,
                &mut remaining_a,
                &mut remaining_b,
                &mut emit,
            )?;
        }
    }

    Ok(())
}

/// Generates all matchings for a fixed choice of the two A elements that
/// participate in the two extended pairings.
///
/// `a2` is the second A element in the mandatory `2A -> 1B` pairing with
/// `added_a`. `a_right` is the A element in the compensating `1A -> 2B`
/// pairing.
///
/// `remaining_a` must already contain exactly those A indices that are not
/// `added_a`, `a2`, or `a_right`. For every possible `b_left`, this function
/// chooses an unordered pair of B elements different from `b_left`, assigns
/// that pair to `a_right`, and generates all permutations of the remaining B
/// elements over `remaining_a`.
///
/// `buf` is reused as scratch storage and is not restored by this function.
#[allow(clippy::too_many_arguments)]
#[inline]
fn generate_for_a_pair<F>(
    buf: &mut [Bitset],
    total: usize,
    added_a: usize,
    a2: usize,
    a_right: usize,
    singletons: &[Bitset],
    remaining_a: &mut [usize],
    remaining_b: &mut Vec<usize>,
    emit: &mut F,
) -> anyhow::Result<()>
where
    F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
{
    debug_assert_eq!(remaining_a.len(), total - 3);

    // Choose the B belonging to the 2A -> 1B triple.
    for b_left in 0..total {
        let b_left_bit = singletons[b_left];

        // Mandatory extended pairing:
        //
        //     added_a ----\
        //                   -> b_left
        //     a2 ---------/
        //
        buf[added_a] = b_left_bit;
        buf[a2] = b_left_bit;

        // Choose b1 and b2 from all B except b_left.
        //
        // added_b is deliberately NOT treated specially here.
        // Therefore it naturally ends up in one of:
        //
        //   2A -> 1B
        //   1A -> 2B
        //   normal 1A -> 1B
        for b1 in 0..b_left {
            // Case 1:
            //     b1 < b2 < b_left
            for b2 in (b1 + 1)..b_left {
                // TODO: maybe this can be optimized so remaining_b is mutated from one iteration to
                // the next one
                remaining_b.clear();
                remaining_b.extend(0..b1);
                remaining_b.extend((b1 + 1)..b2);
                remaining_b.extend((b2 + 1)..b_left);
                remaining_b.extend((b_left + 1)..total);

                emit_b_pair(
                    buf,
                    a_right,
                    b1,
                    b2,
                    singletons,
                    remaining_a,
                    remaining_b,
                    emit,
                )?;
            }

            // Case 2:
            //     b1 < b_left < b2
            for b2 in (b_left + 1)..total {
                remaining_b.clear();
                remaining_b.extend(0..b1);
                remaining_b.extend((b1 + 1)..b_left);
                remaining_b.extend((b_left + 1)..b2);
                remaining_b.extend((b2 + 1)..total);

                emit_b_pair(
                    buf,
                    a_right,
                    b1,
                    b2,
                    singletons,
                    remaining_a,
                    remaining_b,
                    emit,
                )?;
            }
        }

        for b1 in (b_left + 1)..total {
            // Case 3:
            //     b_left < b1 < b2
            for b2 in (b1 + 1)..total {
                remaining_b.clear();
                remaining_b.extend(0..b_left);
                remaining_b.extend((b_left + 1)..b1);
                remaining_b.extend((b1 + 1)..b2);
                remaining_b.extend((b2 + 1)..total);

                emit_b_pair(
                    buf,
                    a_right,
                    b1,
                    b2,
                    singletons,
                    remaining_a,
                    remaining_b,
                    emit,
                )?;
            }
        }
    }

    Ok(())
}

/// Emits all complete matchings for a fixed `1A -> 2B` pairing.
///
/// `a_right` is assigned the two B elements `b1` and `b2`. `remaining_b` must
/// already contain exactly the B elements not used by the `2A -> 1B` and
/// `1A -> 2B` extended pairings. Every permutation of `remaining_b` is mapped
/// positionally onto `remaining_a` to create the ordinary `1A -> 1B`
/// pairings.
///
/// When no B elements remain, the current `buf` is emitted directly.
///
/// `buf` is modified in place and is not restored by this function.
#[allow(clippy::too_many_arguments)]
#[inline]
fn emit_b_pair<F>(
    buf: &mut [Bitset],
    a_right: usize,
    b1: usize,
    b2: usize,
    singletons: &[Bitset],
    remaining_a: &[usize],
    remaining_b: &mut [usize],
    emit: &mut F,
) -> anyhow::Result<()>
where
    F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
{
    // Put the 1A -> 2B pairing in place.
    buf[a_right] = singletons[b1] | singletons[b2];

    // The remaining number of A and B elements is equal.
    debug_assert_eq!(remaining_a.len(), remaining_b.len());

    // No normal pairings left: emit directly.
    //
    // This matters for total == 3, where the remaining
    // permutation has length zero.
    if remaining_b.is_empty() {
        emit(buf)?;
        return Ok(());
    }

    // Every permutation of remaining B values produces
    // one complete matching:
    //
    //   remaining_a[0] -> perm[0]
    //   remaining_a[1] -> perm[1]
    //   ...
    //
    heaps_permute(remaining_b, |perm| {
        for (&a, &b) in remaining_a.iter().zip(perm.iter()) {
            buf[a] = singletons[b];
        }

        emit(buf)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use anyhow::Result;

    #[test]
    fn ab_dup_len_lt_three() -> Result<()> {
        let base = vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8])];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        ab_dup_inplace(&mut buf, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        Ok(())
    }

    #[test]
    fn ab_dup_inplace_total_three() -> Result<()> {
        // A has indices 0, 1, 2 where 2 is the added A.
        // B has indices 0, 1, 2 where 2 is the added B.
        //
        // With total == 3 there are:
        //
        //   3 choices for b_left
        //   2 choices for a2
        //   1 choice for a_right
        //   1 choice for {b1, b2}
        //
        // => 6 emitted matchings.
        //
        // There are no remaining normal pairings.

        let base = vec![
            Bitset::from_word(0),
            Bitset::from_word(0),
            Bitset::from_word(0),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        ab_dup_inplace(&mut buf, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert_eq!(emitted.len(), 6);

        let expected = vec![
            vec![
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[1, 2]),
                Bitset::from_idxs(&[0]),
            ],
            vec![
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[0, 2]),
                Bitset::from_idxs(&[1]),
            ],
            vec![
                Bitset::from_idxs(&[2]),
                Bitset::from_idxs(&[0, 1]),
                Bitset::from_idxs(&[2]),
            ],
            vec![
                Bitset::from_idxs(&[1, 2]),
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[0]),
            ],
            vec![
                Bitset::from_idxs(&[0, 2]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[1]),
            ],
            vec![
                Bitset::from_idxs(&[0, 1]),
                Bitset::from_idxs(&[2]),
                Bitset::from_idxs(&[2]),
            ],
        ];

        assert_eq!(emitted, expected);

        Ok(())
    }

    #[test]
    fn ab_dup_inplace_four() -> Result<()> {
        // Original sets have size 3, both are extended to size 4.
        //
        // Expected count:
        //
        //   n(n - 1)(n + 1)n! / 2
        //   = 3 * 2 * 4 * 6 / 2
        //   = 72
        //
        // Equivalently:
        //
        //   3 choices for a2
        //   2 choices for a_right
        //   4 choices for b_left
        //   3 choices for {b1, b2}
        //   1! normal permutations
        //   = 72.

        let base = vec![
            Bitset::from_word(0),
            Bitset::from_word(0),
            Bitset::from_word(0),
            Bitset::from_word(0),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        ab_dup_inplace(&mut buf, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert_eq!(emitted.len(), 72);
        assert_eq!(
            emitted.iter().cloned().collect::<HashSet<_>>().len(),
            emitted.len()
        );

        for p in &emitted {
            // There are 4 A slots and 4 B values.
            //
            // The total number of set B bits across all A slots must
            // therefore be 4 + 1 = 5:
            //
            //   2A -> 1B   contributes 2 bits
            //   1A -> 2B   contributes 2 bits
            //   1 normal pair contributes 1 bit
            //
            // Also, each individual B index must occur exactly once
            // semantically, except that b_left occurs in two A slots
            // because it belongs to the 2A -> 1B pairing.

            // in sum 5 bits
            assert_eq!(p.iter().map(|b| b.count() as usize).sum::<usize>(), 5);

            // Exactly one 2-bit slot.
            assert_eq!(p.iter().filter(|b| b.count() == 2).count(), 1);

            let singleton_indices = p.iter().filter_map(|b| b.single_idx()).collect::<Vec<_>>();

            // Exactly three 1-bit slots
            assert_eq!(singleton_indices.len(), 3);

            // One of those 3 singleton values is repeated once;
            // the other singleton belongs to the normal pairing.
            let mut counts = [0usize; 4];
            for idx in singleton_indices {
                counts[idx as usize] += 1;
            }

            assert_eq!(counts.iter().filter(|&&c| c == 2).count(), 1);
            assert_eq!(counts.iter().filter(|&&c| c == 1).count(), 1);

            assert_eq!(counts.iter().filter(|&&c| c == 0).count(), 2);

            // Added A is the last index.
            assert!(p[3].is_singleton());

            let shared_b = p[3].single_idx().unwrap();

            // Exactly one other A slot contains the same singleton B.
            assert_eq!(
                p.iter()
                    .enumerate()
                    .filter(|(a, b)| *a != 3
                        && b.is_singleton()
                        && b.single_idx() == Some(shared_b))
                    .count(),
                1
            );
        }

        Ok(())
    }
}

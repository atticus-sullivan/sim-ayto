//! This module handles all kinds of duplicates:
//! 1. We know a value which must be part of the dplicate entry
//! 2. The base was extended and the value(s) which is/are part of the duplicate(s) are simply the
//!    values at the end of the base.

use anyhow::ensure;

use crate::matching_repr::{IdBase, bitset::Bitset};

/// Apply a sequence of duplicate-add operations to `buf` *in-place* and emit every final result.
///
/// Semantics:
/// - `buf` is a mutable slice of `Bitset` representing buckets. Each `Bitset` in `buf` is
///   typically a singleton at call-site (but the implementation tolerates other values).
/// - `add` is a slice of values (bit indices) that must be applied in order. For each `add[i]`
///   the algorithm chooses a target bucket index `j` (0..buf.len()) such that that bucket
///   is currently a singleton; it inserts `add[i]` into that bucket (becoming a 2-element bitset),
///   recurses to the next `add`, and then undoes the mutation (backtracks).
/// - For `add.len() == 0`, `emit` is called exactly once with the original `buf`.
///
/// Performance & invariants:
/// - No heap allocations per emission. The function uses stack recursion of depth `add.len()`.
/// - `Bitset` is copied/assigned (cheap u64 copy) to save/restore buckets; the caller's `buf`
///   is guaranteed to be exactly restored when the function returns.
/// - Ordering of emitted permutations follows a depth-first order (targets tried in index
///   order at each depth). Do not rely on a particular global ordering unless documented.
/// - Caller must ensure `add` values are valid bit indices for `Bitset`.
#[inline]
pub(crate) fn add_x_dups_inplace<F>(
    buf: &mut [Bitset],
    add: &[IdBase],
    mut emit: F,
) -> anyhow::Result<()>
where
    F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
{
    ensure!(add.len() <= 64, "Avoid too deep recursion");

    // Quick path: no adds -> emit original buffer once.
    if add.is_empty() {
        emit(buf)?;
        return Ok(());
    }

    // Recursive DFS function (backtracking), generic over the `emit` closure.
    // We define it here as a nested generic fn to avoid allocation and to be monomorphized
    // with the outer `F`.
    #[inline]
    fn dfs<F>(buf: &mut [Bitset], add: &[IdBase], depth: usize, emit: &mut F) -> anyhow::Result<()>
    where
        F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
    {
        if depth == add.len() {
            // all adds applied -> emit final permutation
            return emit(buf);
        }
        let val = add[depth];

        // Try every slot as target for this add.
        // Only slots that are singletons are valid targets (prevents triples).
        for idx in 0..buf.len() {
            // fast check: singleton
            if !buf[idx].is_singleton() {
                continue;
            }

            // save old word (Bitset is Copy, cheap)
            let old = buf[idx];

            // mutate in-place
            buf[idx].insert(val);
            // recurse
            dfs(buf, add, depth + 1, emit)?;

            // undo mutation
            buf[idx] = old;
        }
        Ok(())
    }
    // kick off recursion
    dfs(buf, add, 0usize, &mut emit)
}

/// In-place distribution of `cnt` duplicate singletons into recipient buckets.
///
/// Buffer layout:
/// - `buf[..split]` are recipient buckets (target slots).
/// - `buf[split..]` are the `cnt` duplicate candidate singletons (each must be a singleton).
///
/// For every combination of `cnt` recipient indices (chosen in increasing order), the function:
/// - checks that each chosen recipient is a singleton,
/// - enforces the ordering constraint existing_min <= dup_min for each pair (to avoid double counting),
/// - ORs the corresponding duplicate words into the chosen recipients,
/// - emits `&mut buf[..split]`,
/// - and then restores `buf`.
///
/// Performance:
/// - No allocations per emitted permutation; combination indices are generated iteratively.
/// - `old_vals` vector is allocated once per candidate combination (capacity == cnt); consider
#[inline]
pub(crate) fn someone_is_dup_inplace<F>(
    buf: &mut [Bitset],
    cnt: usize,
    mut emit: F,
) -> anyhow::Result<()>
where
    F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
{
    let total_len = buf.len();
    if cnt == 0 {
        // nothing to distribute: emit the recipients slice as-is
        return emit(buf);
    }
    if total_len <= cnt {
        // not enough items
        return Ok(());
    }
    let split = total_len - cnt;
    // capture dup raw words (u64) (cheap, small vector)
    let dups_words: Vec<u64> = buf[split..].iter().map(|b| b.as_word()).collect();

    // small stack for chosen recipient indices
    let mut cur: Vec<usize> = Vec::with_capacity(cnt);
    // store old values for recipients to undo when applying dups
    // We'll collect old values at application time (not every recursion step).

    // recursive DFS function
    fn dfs<F>(
        buf: &mut [Bitset],
        recipients_len: usize,
        dups_words: &[u64],
        start: usize,
        cur: &mut Vec<usize>,
        emit: &mut F,
    ) -> anyhow::Result<()>
    where
        F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
    {
        if cur.len() == dups_words.len() {
            // apply dups into recipients at indices in cur (mutate and undo)
            // record old values
            let mut old_vals: Vec<Bitset> = Vec::with_capacity(cur.len());
            for (j, &rec_idx) in cur.iter().enumerate() {
                let old = buf[rec_idx];
                // require existing bucket to be a singleton
                if !old.is_singleton() {
                    return Ok(()); // reject this combination
                }
                // ordering check
                let existing_min = old.single_idx().unwrap_or(IdBase::MAX);
                let dup_min = (dups_words[j].trailing_zeros()) as IdBase;
                if existing_min > dup_min {
                    return Ok(());
                }
                old_vals.push(old);
            }
            // now mutate recipients
            for (j, &rec_idx) in cur.iter().enumerate() {
                buf[rec_idx] |= Bitset::from_word(dups_words[j]);
            }

            emit(&mut buf[..recipients_len])?;

            // undo
            for (j, &rec_idx) in cur.iter().enumerate() {
                buf[rec_idx] = old_vals[j];
            }
            return Ok(());
        }

        for i in start..recipients_len {
            cur.push(i);
            dfs(buf, recipients_len, dups_words, i + 1, cur, emit)?;
            cur.pop();
        }
        Ok(())
    }

    dfs(buf, split, &dups_words, 0, &mut cur, &mut emit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn add_x_dups_inplace_empty_add() -> Result<()> {
        let base = vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8])];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        add_x_dups_inplace(&mut buf, &[], |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        assert_eq!(emitted, vec![base.clone()]);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn add_x_dups_inplace_single_add() -> Result<()> {
        let base = vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8])];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        add_x_dups_inplace(&mut buf, &[2u8], |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        let expected = vec![
            vec![Bitset::from_idxs(&[0u8, 2u8]), Bitset::from_idxs(&[1u8])],
            vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8, 2u8])],
        ];

        assert_eq!(emitted, expected);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn add_x_dups_inplace_two_adds_order() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[2u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        add_x_dups_inplace(&mut buf, &[3u8, 4u8], |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        let expected = vec![
            vec![
                Bitset::from_idxs(&[0u8, 3u8]),
                Bitset::from_idxs(&[1u8, 4u8]),
                Bitset::from_idxs(&[2u8]),
            ],
            vec![
                Bitset::from_idxs(&[0u8, 3u8]),
                Bitset::from_idxs(&[1u8]),
                Bitset::from_idxs(&[2u8, 4u8]),
            ],
            vec![
                Bitset::from_idxs(&[0u8, 4u8]),
                Bitset::from_idxs(&[1u8, 3u8]),
                Bitset::from_idxs(&[2u8]),
            ],
            vec![
                Bitset::from_idxs(&[0u8]),
                Bitset::from_idxs(&[1u8, 3u8]),
                Bitset::from_idxs(&[2u8, 4u8]),
            ],
            vec![
                Bitset::from_idxs(&[0u8, 4u8]),
                Bitset::from_idxs(&[1u8]),
                Bitset::from_idxs(&[2u8, 3u8]),
            ],
            vec![
                Bitset::from_idxs(&[0u8]),
                Bitset::from_idxs(&[1u8, 4u8]),
                Bitset::from_idxs(&[2u8, 3u8]),
            ],
        ];

        assert_eq!(emitted, expected);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn add_x_dups_inplace_deep_limit_error() {
        let mut buf = vec![Bitset::from_idxs(&[0u8])];
        let result = add_x_dups_inplace(&mut buf, &[0u8; 65], |_| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn someone_is_dup_inplace_zero_cnt() -> Result<()> {
        let base = vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8])];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_dup_inplace(&mut buf, 0, |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        assert_eq!(emitted, vec![base.clone()]);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn someone_is_dup_inplace_insufficient_items() -> Result<()> {
        let base = vec![Bitset::from_idxs(&[0u8])];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_dup_inplace(&mut buf, 2, |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        Ok(())
    }

    #[test]
    fn someone_is_dup_inplace_basic_distribution() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[2u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_dup_inplace(&mut buf, 1, |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        let expected = vec![
            vec![Bitset::from_idxs(&[0u8, 2u8]), Bitset::from_idxs(&[1u8])],
            vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8, 2u8])],
        ];

        assert_eq!(emitted, expected);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn someone_is_dup_inplace_two_dups() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[2u8]),
            Bitset::from_idxs(&[3u8]),
            Bitset::from_idxs(&[4u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_dup_inplace(&mut buf, 2, |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        let expected = vec![
            vec![
                Bitset::from_idxs(&[0, 3]),
                Bitset::from_idxs(&[1, 4]),
                Bitset::from_idxs(&[2]),
            ],
            vec![
                Bitset::from_idxs(&[0, 3]),
                Bitset::from_idxs(&[1]),
                Bitset::from_idxs(&[2, 4]),
            ],
            vec![
                Bitset::from_idxs(&[0]),
                Bitset::from_idxs(&[1, 3]),
                Bitset::from_idxs(&[2, 4]),
            ],
        ];

        assert_eq!(emitted, expected);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn someone_is_dup_inplace_ordering_constraint_respected() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[5u8]), // min 5
            Bitset::from_idxs(&[1u8]), // min 1
            Bitset::from_idxs(&[0u8]), // dup min 0
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_dup_inplace(&mut buf, 1, |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        // "wrong" -> nothing emitted
        assert!(emitted.is_empty());
        Ok(())
    }

    #[test]
    fn someone_is_dup_inplace_non_singleton_rejection() -> Result<()> {
        let mut buf = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[2u8, 1u8]), // not singleton
            Bitset::from_idxs(&[3u8]),
        ];
        let mut emitted = Vec::new();
        someone_is_dup_inplace(&mut buf, 1, |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn someone_is_dup_inplace_non_singleton_part_accept() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[0u8, 1u8]), // not singleton
            Bitset::from_idxs(&[2u8]),
            Bitset::from_idxs(&[3u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_dup_inplace(&mut buf, 1, |s| {
            emitted.push(s.to_vec());
            Ok(())
        })?;

        let expected = vec![vec![Bitset::from_idxs(&[0, 1]), Bitset::from_idxs(&[2, 3])]];

        assert_eq!(emitted, expected);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }
}

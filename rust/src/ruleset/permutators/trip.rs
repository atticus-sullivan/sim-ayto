//! This module handles all kinds of duplicates:
//! 1. We know a value which must be part of the triple entry
//! 2. The base was extended and the value(s) which is/are part of the triple(s) are simply the
//!    values at the end of the base => only one triple is supported here (in contrast to the more
//!    generic dup implementation)
//!
//! Notes:
//! - even add_trip_inplace only allows to fix 1/3 of the triple. The other one is taken from the
//!   end of the base.

use crate::matching_repr::{IdBase, bitset::Bitset};

/// In-place version of the "add trip" generator.
///
/// Layout:
/// - `buf` is a mutable slice where the last element (`buf[len-1]`) is the singleton
///   that will be moved/combined into one of the earlier slots to form a triple candidate.
/// - For each index `idx` in 0..len-1 this function checks:
///     * both `buf[idx]` and `buf[last]` are singletons (prevents triples),
///     * ordering constraint `single_idx(idx) >= single_idx(last)` to avoid double counting.
///
/// If checks pass, it temporarily ORs `buf[last]` and `add` into `buf[idx]`, and emits
/// `&mut buf[..last]` (the effective permutation has last item removed).
///
/// Guarantees:
/// - `buf` is restored to its original state after return.
/// - No per-emission heap allocations.
#[inline]
pub(crate) fn add_trip_inplace<F>(buf: &mut [Bitset], add: IdBase, mut emit: F) -> anyhow::Result<()>
where
    F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
{
    let len = buf.len();
    if len < 2 {
        return Ok(());
    }
    let last_idx = len - 1;
    // we will not shrink the buffer; instead we modify buf[idx] to include the last singleton and `add`,
    // then emit &buf[..len-1]
    for idx in 0..last_idx {
        // only add the duplicate if it becomes a duplicate (not a triple)
        if !buf[idx].is_singleton() || !buf[last_idx].is_singleton() {
            continue;
        }
        // ordering test: representative of bucket's single value
        let first_val = buf[idx].single_idx().unwrap_or(IdBase::MAX);
        let last_val = buf[last_idx].single_idx().unwrap_or(IdBase::MAX);
        if first_val < last_val {
            continue;
        }

        // save old value
        let old = buf[idx];

        // the element at uf[last_idx] is the dup => add it
        // OR in the last singleton and add 'add'
        buf[idx] |= buf[last_idx];
        buf[idx].insert(add);

        // emit slice representing the vector with last item removed
        emit(&mut buf[..last_idx])?;

        // undo
        buf[idx] = old;
    }
    Ok(())
}

/// In-place 'someone has a triplet' generator.
///
/// Buffer layout:
/// - Last two elements `buf[len-2]`, `buf[len-1]` are the trip candidate singletons.
/// - For each index `idx` in 0..len-2, if `buf[idx]`, `buf[len-2]`, `buf[len-1]` are singletons and
///   `single_idx(idx) < single_idx(len-1) < single_idx(len-2)` holds, the function ORs the two last
///   singletons into `buf[idx]`, emits `&mut buf[..len-2]` (effective length reduced by 2), and undoes.
///
/// Guarantees:
/// - No allocations per emission.
/// - `buf` is restored to original state before return.
#[inline]
pub(crate) fn someone_is_trip_inplace<F>(buf: &mut [Bitset], mut emit: F) -> anyhow::Result<()>
where
    F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
{
    let len = buf.len();
    if len < 3 {
        return Ok(());
    }
    let last1 = len - 1;
    let last2 = len - 2;

    for idx in 0..(len - 2) {
        let maybe_a = buf[idx].single_idx();
        let maybe_b = buf[last1].single_idx();
        let maybe_c = buf[last2].single_idx();
        if !(maybe_a.is_some() && maybe_b.is_some() && maybe_c.is_some()) {
            continue;
        }
        let a = maybe_a.unwrap();
        let b = maybe_b.unwrap();
        let c = maybe_c.unwrap();
        if !(a < b && b < c) {
            continue;
        }

        // save
        let old = buf[idx];

        // the element at buf[len-2], buf[len-1] are the trip => add them
        // combine last two into idx
        buf[idx] |= buf[last1] | buf[last2];

        emit(&mut buf[..last2])?; // effective slice has last two removed

        // undo
        buf[idx] = old;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn add_trip_inplace_len_lt_two() -> Result<()> {
        let base = vec![Bitset::from_idxs(&[0u8])];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        add_trip_inplace(&mut buf, 5, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn add_trip_inplace_simple() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[2u8]),
            Bitset::from_idxs(&[3u8]),
            Bitset::from_idxs(&[1u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        add_trip_inplace(&mut buf, 0, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        let expected = vec![
            vec![
                Bitset::from_idxs(&[2u8, 0u8, 1u8]),
                Bitset::from_idxs(&[3u8]),
            ],
            vec![
                Bitset::from_idxs(&[2u8]),
                Bitset::from_idxs(&[3u8, 0u8, 1u8]),
            ],
        ];

        assert_eq!(emitted, expected);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn add_trip_inplace_ordering_violation() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[1u8]),
            // last value needs to be smaller than the one where it is inserted
            Bitset::from_idxs(&[3u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        add_trip_inplace(&mut buf, 0, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn add_trip_inplace_non_singleton_skip() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[1u8, 2u8]), // target must be non-singleton
            Bitset::from_idxs(&[0u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        add_trip_inplace(&mut buf, 4, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn someone_is_trip_inplace_len_lt_three() -> Result<()> {
        let base = vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8])];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_trip_inplace(&mut buf, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn someone_is_trip_inplace_simple() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[3u8]),
            Bitset::from_idxs(&[2u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_trip_inplace(&mut buf, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        let expected = vec![
            vec![
                Bitset::from_idxs(&[0u8, 2u8, 3u8]),
                Bitset::from_idxs(&[1u8]),
            ],
            vec![
                Bitset::from_idxs(&[0u8]),
                Bitset::from_idxs(&[1u8, 2u8, 3u8]),
            ],
        ];

        assert_eq!(emitted, expected);
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn someone_is_trip_inplace_ordering_violation_a() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[5u8]), // target must be the smallest value
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[0u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_trip_inplace(&mut buf, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn someone_is_trip_inplace_ordering_violation_b() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[2u8]), // last value must be the second but largest value
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_trip_inplace(&mut buf, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }

    #[test]
    fn someone_is_trip_inplace_non_singleton_skip() -> Result<()> {
        let base = vec![
            Bitset::from_idxs(&[0u8, 1u8]), // target must be non-singleton
            Bitset::from_idxs(&[3u8]),
            Bitset::from_idxs(&[2u8]),
        ];

        let mut buf = base.clone();
        let mut emitted = Vec::new();

        someone_is_trip_inplace(&mut buf, |p| {
            emitted.push(p.to_vec());
            Ok(())
        })?;

        assert!(emitted.is_empty());
        // ensure the buffer was restored correctly
        assert_eq!(buf, base);
        Ok(())
    }
}

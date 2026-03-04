// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module-tree offers various ways of generating permutations.
//! The basic idea is to write functions which are pluggable. This means that one function
//! generates a sequence of permutations. Then another function takes one of these permutations,
//! modifies/explodes/etc it and generates a new sequence this way. Since this is not that easily
//! done in rust with the strong typing, I chose to implement this via nested function calls and
//! closures.

pub(super) mod dup;
pub(super) mod n_to_n;
pub(super) mod trip;

/// Heap's permutation in-place.
///
/// `a` is permuted in-place; `f` is invoked for every permutation with a mutable
/// slice reference to `a`. No allocations are performed except for the small
/// `c` control vector. Use this when you need fast, allocation-free permutations
/// of a small-to-medium sized array.
///
/// Notes:
/// - `f` may be called many times (n! times). Keep `f` cheap and allocation-free
///   where possible.
#[inline]
pub(super) fn heaps_permute<T, F>(a: &mut [T], mut f: F) -> anyhow::Result<()>
where
    F: FnMut(&mut [T]) -> anyhow::Result<()>,
{
    let n = a.len();
    // small optimisation: handle trivial cases
    if n == 0 {
        return Ok(());
    }
    if n == 1 {
        f(a)?;
        return Ok(());
    }

    // control array (counts)
    let mut c = vec![0usize; n];
    // initial permutation
    f(a)?;

    let mut i = 1usize;
    while i < n {
        if c[i] < i {
            // is_multiple_of(2) but with bit magic -> faster
            if i & 1 == 0 {
                a.swap(0, i);
            } else {
                a.swap(c[i], i);
            }
            f(a)?;
            c[i] += 1;
            i = 1;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::collections::HashSet;

    #[test]
    fn heaps_permute_empty() -> Result<()> {
        let mut data: Vec<u8> = vec![];
        let mut called = false;
        heaps_permute(&mut data, |_| {
            called = true;
            Ok(())
        })?;
        assert!(!called);
        Ok(())
    }

    #[test]
    fn heaps_permute_single() -> Result<()> {
        let mut data = vec![42u8];
        let mut seen = Vec::new();
        heaps_permute(&mut data, |s| {
            seen.push(s[0]);
            Ok(())
        })?;
        assert_eq!(seen, vec![42]);
        Ok(())
    }

    #[test]
    fn heaps_permute_three() -> Result<()> {
        let mut data = vec![1u8, 2, 3];
        let mut perms = HashSet::new();
        heaps_permute(&mut data, |s| {
            perms.insert(s.to_vec());
            Ok(())
        })?;
        let expected: HashSet<Vec<u8>> = vec![
            vec![1, 2, 3],
            vec![1, 3, 2],
            vec![2, 1, 3],
            vec![2, 3, 1],
            vec![3, 1, 2],
            vec![3, 2, 1],
        ]
        .into_iter()
        .collect();
        assert_eq!(perms, expected);
        Ok(())
    }

    #[test]
    fn heaps_permute_four() -> Result<()> {
        let mut data = vec![0u8, 1, 2, 3];
        let mut count = 0usize;
        heaps_permute(&mut data, |_| {
            count += 1;
            Ok(())
        })?;
        assert_eq!(count, 24); // 4! = 24
        Ok(())
    }

    #[test]
    fn heaps_permute_error_propagation() {
        let mut data = vec![1u8, 2, 3];
        let err = heaps_permute(&mut data, |_| Err(anyhow::anyhow!("boom"))).unwrap_err();
        assert_eq!(err.to_string(), "boom");
    }
}

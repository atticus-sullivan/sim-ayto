use crate::matching_repr::bitset::Bitset;
use permutator::{Combination, Permutation};

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
/// - `a` is left in some permutation state when the function returns (Heap's algorithm
///   does not guarantee restoring the original ordering). If you need the original
///   order afterwards, clone `a` before calling or restore it yourself.
#[inline]
pub fn heaps_permute<T, F>(a: &mut [T], mut f: F) -> anyhow::Result<()>
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
            if i % 2 == 0 {
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
pub(super) fn add_x_dups_inplace<F>(buf: &mut [Bitset], add: &[u8], mut emit: F) -> anyhow::Result<()>
where
    F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
{
    // Quick path: no adds -> emit original buffer once.
    if add.is_empty() {
        emit(buf)?;
        return Ok(());
    }

    // Recursive DFS function (backtracking), generic over the `emit` closure.
    // We define it here as a nested generic fn to avoid allocation and to be monomorphized
    // with the outer `F`.
    #[inline]
    fn dfs<F>(buf: &mut [Bitset], add: &[u8], depth: usize, emit: &mut F) -> anyhow::Result<()>
    where
        F: FnMut(&mut [Bitset]) -> anyhow::Result<()>,
    {
        if depth == add.len() {
            // all adds applied -> emit final permutation
            return emit(buf)
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

/// In-place version of the "add trip" generator.
///
/// Layout:
/// - `buf` is a mutable slice where the last element (`buf[len-1]`) is the singleton
///   that will be moved/combined into one of the earlier slots to form a triple candidate.
/// - For each index `idx` in 0..len-1 this function checks:
///     * both `buf[idx]` and `buf[last]` are singletons (prevents triples),
///     * ordering constraint `single_idx(idx) >= single_idx(last)` to avoid double counting.
///   If checks pass, it temporarily ORs `buf[last]` and `add` into `buf[idx]`, and emits
///   `&mut buf[..last]` (the effective permutation has last item removed).
///
/// Guarantees:
/// - `buf` is restored to its original state after return.
/// - No per-emission heap allocations.
#[inline]
pub(super) fn add_trip_inplace<F>(buf: &mut [Bitset], add: u8, mut emit: F) -> anyhow::Result<()>
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
        let first_val = buf[idx].single_idx().unwrap_or(u8::MAX);
        let last_val = buf[last_idx].single_idx().unwrap_or(u8::MAX);
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
///   `SmallVec` if `cnt` is extremely small and you want to avoid heap usage entirely.
#[inline]
pub(super) fn someone_is_dup_inplace<F>(buf: &mut [Bitset], cnt: usize, mut emit: F) -> anyhow::Result<()>
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
    ) -> anyhow::Result<()> where
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
                let existing_min = old.single_idx().unwrap_or(u8::MAX);
                let dup_min = (dups_words[j].trailing_zeros()) as u8;
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
pub(super) fn someone_is_trip_inplace<F>(buf: &mut [Bitset], mut emit: F) -> anyhow::Result<()>
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
/// - The ordering guard `if k <= v { reject }` from the reference implementation is preserved.
#[inline]
pub fn n_to_n_inplace<F>(slots: usize, mut emit: F) -> anyhow::Result<()>
where
    F: FnMut(&[Bitset]) -> anyhow::Result<()>,
{
    if slots == 0 {
        return Ok(());
    }
    if slots % 2 != 0 {
        // semantics in existing code assume even slots (n-to-n); silently do nothing or return error.
        // Choose to return Ok(()) to match "no permutations".
        return Ok(());
    }

    let len = slots / 2;

    // Reusable output buffer: all empty initially.
    let mut c = vec![Bitset::empty(); slots];

    // Precompute full index set as u8 vector for perm/combo helpers
    let full_indices: Vec<u8> = (0..slots as u8).collect();

    // Iterate combinations of indices (ks)
    for ks in full_indices.combination(len) {
        // produce the list of remaining values (vs)
        let mut vs = (0..slots as u8).filter(|x| !ks.contains(&x)).collect::<Vec<_>>();

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
                if (k as u8) <= v {
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
mod generator_tests {
    use std::collections::HashSet;

    use super::*;
    use crate::matching_repr::{bitset::Bitset, MaskedMatching, IdBase};

    #[test]
    fn someone_is_dup_inplace_bitset_simple() {
        // recipients = [[0], [1]]; dups = [2]
        let mut base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[2u8])
        ];
        let mut out = vec![];
        someone_is_dup_inplace(&mut base, 1usize, |p| {
            out.push(p.to_vec());
            Ok(())
        }).unwrap();
        assert!(out.iter().any(|perm| perm.iter().any(|b| b.count() == 2)));

        let out_ref = vec![
            vec![Bitset(5), Bitset(2)],
            vec![Bitset(1), Bitset(6)]
        ];
        assert_eq!(out, out_ref);
    }

    #[test]
    fn someone_is_trip_inplace_bitset_basic() {
        // Setup: ... ensure we get at least one combined triple
        let mut base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[2u8]),
            Bitset::from_idxs(&[3u8]),
            Bitset::from_idxs(&[1u8]),
        ];
        let mut out = vec![];
        // The last two are trip candidates; someone_is_trip should create outputs
        someone_is_trip_inplace(&mut base, |p| {
            out.push(p.to_vec());
            Ok(())
        }).unwrap();
        assert!(!out.is_empty());
        // check that one output has a bucket with at least 3 bits set (trip)
        assert!(out.iter().any(|perm| perm.iter().any(|b| b.count() >= 3)));

        let out_ref = vec![
            vec![Bitset(11), Bitset(4)]
        ];
        assert_eq!(out, out_ref);
    }

    #[test]
    fn heaps_permute_produces_all_perms() {
        let mut v = vec![1u8, 2u8, 3u8];
        let mut seen = std::collections::HashSet::new();
        heaps_permute(&mut v, |s| {
            seen.insert(s.to_vec());
            Ok(())
        }).unwrap();
        let expected: std::collections::HashSet<Vec<u8>> = vec![
            vec![1,2,3],
            vec![1,3,2],
            vec![2,1,3],
            vec![2,3,1],
            vec![3,1,2],
            vec![3,2,1],
        ].into_iter().collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn test_n_to_n_inplace_matches_reference() {
        // small example: 6 slots => half = 3 (matches the existing test in ruleset.rs)
        let slots = 6usize;
        let mut out_inplace = vec![];

        n_to_n_inplace(slots, |slice| {
            out_inplace.push(slice.to_vec());
            Ok(())
        }).unwrap();

        let expected: Vec<MaskedMatching> = vec![
            vec![vec![], vec![0 as IdBase], vec![], vec![], vec![2 as IdBase], vec![3 as IdBase]],
            vec![vec![], vec![0 as IdBase], vec![], vec![], vec![3 as IdBase], vec![2 as IdBase]],
            vec![vec![], vec![0 as IdBase], vec![], vec![2 as IdBase], vec![], vec![4 as IdBase]],
            vec![vec![], vec![], vec![0 as IdBase], vec![1 as IdBase], vec![], vec![4 as IdBase]],
            vec![vec![], vec![], vec![0 as IdBase], vec![], vec![1 as IdBase], vec![3 as IdBase]],
            vec![vec![], vec![], vec![0 as IdBase], vec![], vec![3 as IdBase], vec![1 as IdBase]],
            vec![vec![], vec![], vec![1 as IdBase], vec![0 as IdBase], vec![], vec![4 as IdBase]],
            vec![vec![], vec![], vec![1 as IdBase], vec![], vec![0 as IdBase], vec![3 as IdBase]],
            vec![vec![], vec![], vec![1 as IdBase], vec![], vec![3 as IdBase], vec![0 as IdBase]],
            vec![vec![], vec![], vec![], vec![0 as IdBase], vec![1 as IdBase], vec![2 as IdBase]],
            vec![vec![], vec![], vec![], vec![0 as IdBase], vec![2 as IdBase], vec![1 as IdBase]],
            vec![vec![], vec![], vec![], vec![1 as IdBase], vec![0 as IdBase], vec![2 as IdBase]],
            vec![vec![], vec![], vec![], vec![1 as IdBase], vec![2 as IdBase], vec![0 as IdBase]],
            vec![vec![], vec![], vec![], vec![2 as IdBase], vec![0 as IdBase], vec![1 as IdBase]],
            vec![vec![], vec![], vec![], vec![2 as IdBase], vec![1 as IdBase], vec![0 as IdBase]],
        ].into_iter().map(|x| x.into()).collect();
        let expected = expected.into_iter().map(|x| x.into_masks()).collect::<HashSet<_>>();

        // compare as unordered sets to avoid relying on generation order
        let out: HashSet<Vec<Bitset>> = out_inplace.into_iter().collect();
        assert_eq!(out, expected);
    }

        #[test]
    fn add_x_dups_inplace_two_adds_expected_order_and_restore() {
        // base permutation: [[0], [1], [2]]
        let base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[2u8]),
        ];
        let mut buf = base.clone();

        // adds to apply in order
        let adds = vec![3u8, 4u8];

        // Collect actual outputs in emission order
        let mut actual: Vec<Vec<Bitset>> = Vec::new();
        add_x_dups_inplace(&mut buf, &adds, |s| {
            actual.push(s.to_vec());
            Ok(())
        }).unwrap();

        // Ensure buffer was restored exactly
        assert_eq!(buf, base, "buffer was not restored to original state");

        // Build expected list explicitly (order matters - matches DFS order: depth-first with targets 0..n)
        let expected: Vec<Vec<Bitset>> = vec![
            // add3 -> slot0, add4 -> slot1
            vec![
                Bitset::from_idxs(&[0u8, 3u8]),
                Bitset::from_idxs(&[1u8, 4u8]),
                Bitset::from_idxs(&[2u8]),
            ],
            // add3 -> slot0, add4 -> slot2
            vec![
                Bitset::from_idxs(&[0u8, 3u8]),
                Bitset::from_idxs(&[1u8]),
                Bitset::from_idxs(&[2u8, 4u8]),
            ],
            // add3 -> slot1, add4 -> slot0
            vec![
                Bitset::from_idxs(&[0u8, 4u8]),
                Bitset::from_idxs(&[1u8, 3u8]),
                Bitset::from_idxs(&[2u8]),
            ],
            // add3 -> slot1, add4 -> slot2
            vec![
                Bitset::from_idxs(&[0u8]),
                Bitset::from_idxs(&[1u8, 3u8]),
                Bitset::from_idxs(&[2u8, 4u8]),
            ],
            // add3 -> slot2, add4 -> slot0
            vec![
                Bitset::from_idxs(&[0u8, 4u8]),
                Bitset::from_idxs(&[1u8]),
                Bitset::from_idxs(&[2u8, 3u8]),
            ],
            // add3 -> slot2, add4 -> slot1
            vec![
                Bitset::from_idxs(&[0u8]),
                Bitset::from_idxs(&[1u8, 4u8]),
                Bitset::from_idxs(&[2u8, 3u8]),
            ],
        ];

        assert_eq!(
            actual.len(),
            expected.len(),
            "number of emitted permutations differs"
        );
        assert_eq!(actual, expected, "emitted permutations differ from expected");
    }

    #[test]
    fn add_x_dups_inplace_single_add_expected_and_restore() {
        // base: [[0], [1]]
        let base = vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8])];
        let mut buf = base.clone();

        // single add -> two possibilities
        let adds = vec![2u8];

        let mut actual: Vec<Vec<Bitset>> = Vec::new();
        add_x_dups_inplace(&mut buf, &adds, |s| {
            actual.push(s.to_vec());
            Ok(())
        }).unwrap();

        // buffer must be restored
        assert_eq!(buf, base);

        // expected order: add goes to slot 0 then slot 1
        let expected = vec![
            vec![Bitset::from_idxs(&[0u8, 2u8]), Bitset::from_idxs(&[1u8])],
            vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8, 2u8])],
        ];

        assert_eq!(actual, expected, "single-add outputs differ from expected");
    }

    #[test]
    fn add_x_dups_inplace_restores_buffer_and_matches_expected_examples() {
        // explicit example (order matters) — same test you had but kept independent
        let base = vec![
            Bitset::from_idxs(&[0u8]),
            Bitset::from_idxs(&[1u8]),
            Bitset::from_idxs(&[2u8]),
        ];
        let mut buf = base.clone();
        let adds = vec![3u8, 4u8];

        // collect actual outputs in emission order
        let mut actual: Vec<Vec<Bitset>> = Vec::new();
        add_x_dups_inplace(&mut buf, &adds, |s| {
            actual.push(s.to_vec());
            Ok(())
        }).unwrap();

        // buffer must be restored
        assert_eq!(buf, base);

        // expected DFS order (depth-first, targets 0..n)
        let expected: Vec<Vec<Bitset>> = vec![
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

        assert_eq!(actual, expected, "inplace emission order differs from expected DFS order");
    }
}

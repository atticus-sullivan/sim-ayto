pub(super) fn add_dup<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
    add: u8,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        (0..perm.len()).filter_map({
            let perm = perm.clone();
            move |idx| {
                // only add the duplicate if it becomes a duplicate (not a triple)
                if perm[idx].len() > 1 {
                    return None;
                }
                let mut c = perm.clone();
                c[idx].push(add);
                Some(c)
            }
        })
    })
}

pub(super) fn add_trip<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
    add: u8,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        // select who has the dup
        (0..perm.len() - 1).filter_map(move |idx| {
            // only add the duplicate if it becomes a duplicate (not a triple)
            if perm[idx].len() > 1 {
                return None;
            }
            // only count once regardless the ordering
            if perm[idx][0] < perm[perm.len() - 1][0] {
                return None;
            }
            // the element at perm[len-1] is the dup => add it
            let mut c = perm.clone();
            let x = c.pop()?;
            c[idx].push(x[0]);
            c[idx].push(add);
            Some(c)
        })
    })
}

pub(super) fn someone_is_dup<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
    cnt: usize,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        let split = perm.len() - cnt;

        let recipients = perm[..split].to_vec();
        let dups = perm[split..].iter().map(|v| v[0]).collect::<Vec<_>>();

        let mut outputs: Vec<Vec<Vec<u8>>> = Vec::new();

        // internal values used for backtracking
        let mut cur = Vec::<usize>::with_capacity(cnt);

        // recursive DFS defined as local function (doesn't capture environment)
        fn dfs(
            recipients: &Vec<Vec<u8>>,
            dups: &Vec<u8>,
            // indices already containing a duplicate
            // used: &mut Vec<bool>,
            start: usize,
            // maps dups-idx to at which index to insert the duplicate
            cur: &mut Vec<usize>,
            outputs: &mut Vec<Vec<Vec<u8>>>,
        ) {
            if cur.len() == dups.len() {
                // build resulting c from base and cur mapping
                let mut c = recipients.clone();
                for (j, &rec_idx) in cur.iter().enumerate() {
                    if c[rec_idx][0] > dups[j] {
                        return;
                    }
                    c[rec_idx].push(dups[j]);
                }
                outputs.push(c);
                return;
            }
            // select/search index where to insert duplicate
            for i in start..recipients.len() {
                cur.push(i);
                dfs(recipients, dups, i + 1, cur, outputs);
                cur.pop();
            }
        }

        dfs(&recipients, &dups, 0, &mut cur, &mut outputs);
        outputs.into_iter()
    })
}

pub(super) fn someone_is_trip<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        // if perm[perm.len() - 1][0] < perm[perm.len() - 2][0] {
        //     return;
        // }
        // select who has the trip
        (0..perm.len() - 2).filter_map(move |idx| {
            // only count once regardless the ordering
            if !(perm[idx][0] < perm[perm.len() - 1][0]
                && perm[perm.len() - 1][0] < perm[perm.len() - 2][0])
            {
                return None;
            }
            // the element at perm[len-2],perm[len-1] are the trip => add them
            let mut c = perm.clone();
            let x = c.pop()?;
            c[idx].push(x[0]);
            let x = c.pop()?;
            c[idx].push(x[0]);
            Some(c)
        })
    })
}

#[cfg(test)]
mod generator_tests {
    use super::*;
    use crate::matching_repr::bitset::Bitset;

    #[test]
    fn add_dup_basic() {
        let v = vec![vec![0u8], vec![1u8]];
        let iter = add_dup(std::iter::once(v), 2u8);
        let out: Vec<_> = iter.collect();
        // we can add '2' to either slot (both slots are size 1)
        let expected = vec![vec![0u8, 2u8], vec![1u8]];
        let expected2 = vec![vec![0u8], vec![1u8, 2u8]];
        assert!(out.contains(&expected) && out.contains(&expected2));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn add_trip_basic() {
        let v = vec![vec![0u8], vec![2u8], vec![1u8]];
        // adding 3 as trip candidate should yield permutations where some bucket gets two new items
        let out: Vec<_> = add_trip(std::iter::once(v), 3u8).collect();
        // expect that one of the buckets now contains both the popped element and the new `3`
        assert!(out.iter().any(|perm| perm.iter().any(|bucket| bucket.contains(&3u8))));
    }

    #[test]
    fn someone_is_dup_simple() {
        // base permutation: recipients = [ [0], [1] ], dups = [2]
        let base = vec![vec![0u8], vec![1u8], vec![2u8]];
        // The helper expects the combined sequence where last N entries are duplicates.
        // We call someone_is_dup over an iterator with that perm structure and cnt = 1
        let iter = someone_is_dup(std::iter::once(base.clone()), 1usize);
        let out: Vec<_> = iter.collect();
        // There should be outputs where 2 is placed into one of the recipients (but not triple)
        assert!(out.iter().any(|perm| perm.iter().any(|bucket| bucket.len() == 2)));
    }

    // #[test]
    // fn add_dup_bitset_basic() {
    //     // two buckets: [0], [1] represented as Bitsets
    //     let v = vec![Bitset::from_idxs(&[0u8]), Bitset::from_idxs(&[1u8])];
    //     let iter = add_dup(std::iter::once(v), 2u8);
    //     let out: Vec<_> = iter.collect();
    //     // results should be two permutations where one bucket now has an extra bit
    //     assert_eq!(out.len(), 2);
    //     assert!(out.iter().any(|perm| perm[0].contains(2u8)));
    //     assert!(out.iter().any(|perm| perm[1].contains(2u8)));
    // }
    //
    // #[test]
    // fn someone_is_dup_bitset_simple() {
    //     // recipients = [[0], [1]]; dups = [2]
    //     let base = vec![
    //         Bitset::from_idxs(&[0u8]),
    //         Bitset::from_idxs(&[1u8]),
    //         Bitset::from_idxs(&[2u8])
    //     ];
    //     let iter = someone_is_dup(std::iter::once(base), 1usize);
    //     let out: Vec<_> = iter.collect();
    //     assert!(out.iter().any(|perm| perm.iter().any(|b| b.count() == 2)));
    // }
}

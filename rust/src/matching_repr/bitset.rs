use std::{
    collections::HashSet,
    fmt::Binary,
    ops::{BitAndAssign, BitOrAssign},
};

use serde::{Deserialize, Serialize};

use crate::matching_repr::{IdBase, Word, WORD_BITS};

/// Small strongly-typed wrapper around a single-word bitset.
///
/// Encapsulates the low-level bit fiddling so higher-level code doesn't
/// directly work with `u64` everywhere. This makes it easier to later
/// replace the implementation with a multi-word bitset if necessary.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Bitset(pub Word);

impl Binary for Bitset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print as a plain binary number (no "0b" prefix). Example: Bitset(5) -> "101".
        write!(f, "{:b}", self.0)
    }
}

impl Bitset {
    /// Construct an empty bitset.
    #[inline(always)]
    pub fn empty() -> Self {
        Bitset(0)
    }

    /// Construct from raw word.
    #[inline(always)]
    pub fn from_word(w: Word) -> Self {
        Bitset(w)
    }

    /// Construct with bits set from a slice of indices.
    #[inline(always)]
    pub fn from_idxs(ws: &[IdBase]) -> Self {
        let mut b = Bitset::empty();
        for &w in ws {
            // defensive: shifting by >= WORD_BITS is undefined / surprising in some builds.
            debug_assert!(
                (w as usize) < WORD_BITS,
                "index >= WORD_BITS in Bitset::from_idxs"
            );
            b.0 |= (1 as Word) << (w as usize);
        }
        b
    }

    /// Insert `x` into the bitset (mutates in-place).
    #[inline(always)]
    pub fn insert(&mut self, x: IdBase) {
        debug_assert!(
            (x as usize) < WORD_BITS,
            "index >= WORD_BITS in Bitset::insert"
        );
        self.0 |= (1 as Word) << (x as usize);
    }

    /// Returns `true` if the bitset contains any element from `eles`.
    #[inline(always)]
    pub fn contains_any_idx(&self, eles: &HashSet<IdBase>) -> bool {
        eles.iter()
            .any(|i| (self.0 & ((1 as Word) << (*i as usize))) != 0)
    }

    /// Returns `true` if the intersection of the two sets is non-empty.
    #[inline(always)]
    pub fn contains_any(&self, eles: &Bitset) -> bool {
        self.0 & eles.0 != 0
    }

    /// Return the raw word.
    #[inline(always)]
    pub(super) fn as_word(self) -> Word {
        self.0
    }

    /// Test whether the bitset contains index `x`.
    #[inline(always)]
    pub fn contains(self, x: IdBase) -> bool {
        (self.0 & ((1 as Word) << (x as usize))) != 0
    }

    /// Count bits set.
    #[inline(always)]
    pub fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Return true if empty.
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return number of trailing zeros (for lowest set bit).
    #[inline(always)]
    pub fn trailing_zeros(self) -> u32 {
        self.0.trailing_zeros()
    }

    /// Clear the lowest set bit (mutates).
    #[inline(always)]
    pub fn clear_lowest_bit(&mut self) {
        self.0 &= self.0 - 1;
    }

    /// Bitwise AND.
    #[inline(always)]
    pub fn and(self, other: Bitset) -> Bitset {
        Bitset(self.0 & other.0)
    }

    /// Bitwise OR.
    #[inline(always)]
    pub fn or(self, other: Bitset) -> Bitset {
        Bitset(self.0 | other.0)
    }

    /// Iterator over set indices (in increasing order).
    #[inline(always)]
    pub fn iter(self) -> BitIter {
        BitIter { w: self }
    }
}

/// Bitwise operators for `Bitset`.
impl std::ops::BitAnd for Bitset {
    type Output = Bitset;
    #[inline(always)]
    fn bitand(self, rhs: Bitset) -> Bitset {
        Bitset(self.0 & rhs.0)
    }
}
impl std::ops::BitOr for Bitset {
    type Output = Bitset;
    #[inline(always)]
    fn bitor(self, rhs: Bitset) -> Bitset {
        Bitset(self.0 | rhs.0)
    }
}
impl BitAndAssign for Bitset {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Bitset) {
        self.0 &= rhs.0;
    }
}
impl BitOrAssign for Bitset {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Bitset) {
        self.0 |= rhs.0;
    }
}

/// Iterator over set bits (yields indices). Holds a Bitset and is Clone.
#[derive(Clone)]
pub struct BitIter {
    pub w: Bitset,
}

impl Iterator for BitIter {
    type Item = IdBase;

    fn next(&mut self) -> Option<Self::Item> {
        if self.w.is_empty() {
            return None;
        }
        // Find index of lowest set bit
        let tz = self.w.trailing_zeros() as IdBase;
        // clear lowest set bit
        self.w.clear_lowest_bit();
        Some(tz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitset_basic() {
        let mut b = Bitset::empty();
        assert!(b.is_empty());
        b.insert(3);
        assert!(!b.is_empty());
        assert!(b.contains(3));
        assert_eq!(b.count(), 1);
        b.insert(1);
        assert_eq!(b.count(), 2);
        let collected: Vec<IdBase> = b.iter().collect();
        assert_eq!(collected, vec![1, 3]);
    }

    #[test]
    fn test_bitset_ops_and_assign() {
        let a = Bitset::from_idxs(&[1u8, 3u8]);
        let b = Bitset::from_idxs(&[3u8, 5u8]);
        let and = a & b;
        assert_eq!(and, Bitset::from_idxs(&[3u8]));
        let or = a | b;
        assert!(or.contains(1));
        assert!(or.contains(3));
        assert!(or.contains(5));

        let mut c = a;
        c &= b;
        assert_eq!(c, Bitset::from_idxs(&[3u8]));
        let mut d = Bitset::empty();
        d |= a;
        assert_eq!(d, a);
    }

    #[test]
    fn test_bitset_binary_fmt_and_ord() {
        let a = Bitset::from_idxs(&[0u8, 2u8]); // 1 + 4 = 5 -> "101"
        assert_eq!(format!("{:b}", a), "101");

        // Ord/PartialOrd are derived over the inner word; check numeric ordering.
        let small = Bitset::from_idxs(&[1u8]); // 2
        let large = Bitset::from_idxs(&[3u8]); // 8
        assert!(small < large);
        assert_eq!(small.cmp(&large), std::cmp::Ordering::Less);
    }

    #[test]
    fn bitset_empty_and_from_word_and_from_idxs() {
        let e = Bitset::empty();
        assert_eq!(e.0, 0);
        let w = Bitset::from_word(0b1010);
        assert_eq!(w.as_word(), 0b1010);
        let s = Bitset::from_idxs(&[1u8, 3u8]);
        assert_eq!(s, w);
    }

    #[test]
    fn bitset_insert_and_contains_and_count_is_empty() {
        let mut b = Bitset::empty();
        assert!(b.is_empty());
        b.insert(5);
        assert!(!b.is_empty());
        assert!(b.contains(5));
        assert_eq!(b.count(), 1);
        b.insert(2);
        assert_eq!(b.count(), 2);
    }

    #[test]
    fn bitset_contains_any_idx() {
        let b = Bitset::from_idxs(&[1u8, 4u8]);
        let mut set = HashSet::new();
        set.insert(0u8);
        set.insert(4u8);
        assert!(b.contains_any_idx(&set));
        set.remove(&4u8);
        assert!(!b.contains_any_idx(&set));
    }

    #[test]
    fn bitset_trailing_clear_and_iter_and_trailing_zeros() {
        let b = Bitset::from_word(0b10110);
        // trailing zeros of 0b10110 is 1
        assert_eq!(b.trailing_zeros(), 1);
        let mut it = b.iter();
        assert_eq!(it.next(), Some(1));
        assert_eq!(it.next(), Some(2));
        assert_eq!(it.next(), Some(4));
        assert_eq!(it.next(), None);

        // clear_lowest_bit works
        let mut b2 = Bitset::from_word(0b10110);
        b2.clear_lowest_bit();
        assert_eq!(b2.as_word(), 0b10100);
    }

    #[test]
    fn bitset_and_or_and_assign_ops() {
        let a = Bitset::from_idxs(&[1u8, 3u8]);
        let b = Bitset::from_idxs(&[3u8, 5u8]);
        assert_eq!((a & b), Bitset::from_idxs(&[3u8]));
        assert_eq!((a | b), Bitset::from_idxs(&[1u8, 3u8, 5u8]));
        let mut c = a;
        c &= b;
        assert_eq!(c, Bitset::from_idxs(&[3u8]));
        let mut d = Bitset::empty();
        d |= a;
        assert_eq!(d, a);
    }

    #[test]
    fn bitset_binary_format_and_ord() {
        let a = Bitset::from_idxs(&[0u8, 2u8]); // 5 -> "101"
        assert_eq!(format!("{:b}", a), "101");
        let small = Bitset::from_idxs(&[1u8]); // 2
        let large = Bitset::from_idxs(&[3u8]); // 8
        assert!(small < large);
    }

    #[test]
    fn test_bitset_and_or_methods() {
        let a = Bitset::from_idxs(&[0u8, 2u8]); // 101
        let b = Bitset::from_idxs(&[2u8, 3u8]); // 1100
        // method forms:
        let m_and = a.and(b);
        assert_eq!(m_and, Bitset::from_idxs(&[2u8]));
        let m_or = a.or(b);
        assert!(m_or.contains(0));
        assert!(m_or.contains(2));
        assert!(m_or.contains(3));
    }

    #[test]
    fn test_bititer_on_empty_returns_none() {
        let b = Bitset::empty();
        let mut it = b.iter();
        assert_eq!(it.next(), None);
    }

}

// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides a complete implementation of a *Bitset*. A *Bitset* can be compared with a
//! HashSet but much more efficient as it stores its elements in a binary representation. In this
//! representation, the indices (with a set bit) are the elements which are stored. This is also
//! the cause for the major constraint: The elements of such a set can only be integers (of a
//! limited range).

use std::fmt::Binary;
use std::ops::{BitAndAssign, BitOrAssign};

use serde::{Deserialize, Serialize};

use crate::matching_repr::{IdBase, Word, WORD_BITS};

/// Small strongly-typed wrapper around a single-word bitset.
///
/// Encapsulates the low-level bit fiddling so higher-level code doesn't
/// directly work with `u64` everywhere. This makes it easier to later
/// replace the implementation with a multi-word bitset if necessary.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Bitset(pub(super) Word);

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

    /// Clear the lowest set bit (mutates).
    #[inline(always)]
    pub fn clear_lowest_bit(&mut self) {
        self.0 &= self.0 - 1;
    }

    /// Return number of trailing zeros (for lowest set bit).
    #[inline(always)]
    pub fn trailing_zeros(self) -> u32 {
        self.0.trailing_zeros()
    }

    /// Test whether the bitset contains index `x`.
    #[inline(always)]
    pub fn contains_idx(self, x: IdBase) -> bool {
        (self.0 & ((1 as Word) << (x as usize))) != 0
    }

    /// Returns `true` if the intersection of the two sets is non-empty.
    #[inline(always)]
    pub fn contains_any(self, eles: Bitset) -> bool {
        self.0 & eles.0 != 0
    }

    /// Return the raw word.
    #[inline(always)]
    pub fn as_word(self) -> Word {
        self.0
    }

    /// Count bits set.
    /// => equivalent to HashSet/Vec::len()
    #[inline(always)]
    pub fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Return true if empty.
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// True if word has exactly one bit set.
    /// More efficient than count == 1 (-> popcount instruction vs raw ALU bit magic)
    #[inline(always)]
    pub fn is_singleton(self) -> bool {
        let w = self.0;
        w != 0 && (w & (w - 1)) == 0
    }

    /// If this Bitset contains exactly one value, return that index.
    /// Otherwise return None.
    #[inline(always)]
    pub fn single_idx(self) -> Option<IdBase> {
        self.is_singleton().then(|| self.trailing_zeros() as IdBase)
    }

    /// Bitwise AND. => intersection
    #[inline(always)]
    pub fn and(self, other: Bitset) -> Bitset {
        Bitset(self.0 & other.0)
    }

    /// Bitwise OR. => union
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

/// Iterator over set bits (yields indices). Holds a Bitset.
#[derive(Clone, Copy)]
pub struct BitIter {
    /// the base for the iterator
    pub(super) w: Bitset,
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
    fn binary_fmt_simple() {
        let b = Bitset::from_word(0b10101);
        let s = format!("{:b}", b);
        assert_eq!(s, "10101");
    }

    #[test]
    fn empty_simple() {
        let b = Bitset::empty();
        assert_eq!(b.0, 0);
        assert!(b.is_empty());
    }

    #[test]
    fn from_word_simple() {
        let w: Word = 0b10101;
        let b = Bitset::from_word(w);
        assert_eq!(b.0, w);
    }

    #[test]
    fn from_idxs_simple() {
        let ids: &[IdBase] = &[0, 2, 5];
        let b = Bitset::from_idxs(ids);
        let expected = (1 << 0) | (1 << 2) | (1 << 5);
        assert_eq!(b.0, expected);
    }

    #[test]
    fn insert_simple() {
        let mut b = Bitset::empty();
        b.insert(3);
        assert_eq!(b.0, 1 << 3);
        b.insert(0);
        assert_eq!(b.0, (1 << 3) | (1 << 0));
    }

    #[test]
    fn clear_lowest_bit_simple() {
        let mut b = Bitset::from_word(0b11010);
        b.clear_lowest_bit();
        assert_eq!(b.0, 0b11000);
        b.clear_lowest_bit();
        assert_eq!(b.0, 0b10000);
    }

    #[test]
    fn trailing_zeros_simple() {
        let b = Bitset::from_word(0b1000);
        assert_eq!(b.trailing_zeros(), 3);

        let b = Bitset::from_word(0b1001);
        assert_eq!(b.trailing_zeros(), 0);
    }

    #[test]
    fn contains_idx_simple() {
        let b = Bitset::from_idxs(&[1, 4]);
        assert!(b.contains_idx(1));
        assert!(!b.contains_idx(2));

        let b = Bitset::from_idxs(&[]);
        assert!(!b.contains_idx(1));
        assert!(!b.contains_idx(2));
    }

    #[test]
    fn contains_any_simple() {
        let a = Bitset::from_idxs(&[1, 3]);
        let b = Bitset::from_idxs(&[3, 4]);
        let c = Bitset::from_idxs(&[0, 2]);
        assert!(a.contains_any(b));
        assert!(!a.contains_any(c));
    }

    #[test]
    fn as_word_simple() {
        let w: Word = 0b1110;
        let b = Bitset::from_word(w);
        assert_eq!(b.as_word(), w);
    }

    #[test]
    fn count_simple() {
        let a = [0, 2, 4, 7];
        let b = Bitset::from_idxs(&a);
        assert_eq!(b.count(), a.len());
    }

    #[test]
    fn is_empty_simple() {
        let b = Bitset::empty();
        assert!(b.is_empty());
        let c = Bitset::from_word(1);
        assert!(!c.is_empty());
    }

    #[test]
    fn is_singleton_true() {
        let b = Bitset::from_word(1 << 5);
        assert!(b.is_singleton());
    }

    #[test]
    fn is_singleton_false() {
        let b = Bitset::from_word((1 << 2) | (1 << 4));
        assert!(!b.is_singleton());
    }

    #[test]
    fn single_idx_some() {
        let b = Bitset::from_word(1 << 6);
        assert_eq!(b.single_idx(), Some(6));
    }

    #[test]
    fn single_idx_none() {
        let b = Bitset::from_word(0);
        assert_eq!(b.single_idx(), None);
        let c = Bitset::from_word((1 << 1) | (1 << 3));
        assert_eq!(c.single_idx(), None);
    }

    #[test]
    fn and_method_simple() {
        let a = Bitset::from_idxs(&[1, 3, 5]);
        let b = Bitset::from_idxs(&[3, 4]);
        let c = a.and(b);
        assert_eq!(c, Bitset::from_idxs(&[3]));
    }

    #[test]
    fn or_method_simple() {
        let a = Bitset::from_idxs(&[0, 2]);
        let b = Bitset::from_idxs(&[1, 2]);
        let c = a.or(b);
        assert_eq!(c, Bitset::from_idxs(&[0, 1, 2]));
    }

    #[test]
    fn bitand_operator_simple() {
        let a = Bitset::from_word(0b1010);
        let b = Bitset::from_word(0b1100);
        let c = a & b;
        assert_eq!(c, Bitset::from_word(0b1000));
    }

    #[test]
    fn bitor_operator_simple() {
        let a = Bitset::from_word(0b0011);
        let b = Bitset::from_word(0b0101);
        let c = a | b;
        assert_eq!(c, Bitset::from_word(0b0111));
    }

    #[test]
    fn bitand_assign_operator_simple() {
        let mut a = Bitset::from_word(0b1110);
        let b = Bitset::from_word(0b1010);
        a &= b;
        assert_eq!(a, Bitset::from_word(0b1010));
    }

    #[test]
    fn bitor_assign_operator_simple() {
        let mut a = Bitset::from_word(0b0010);
        let b = Bitset::from_word(0b0101);
        a |= b;
        assert_eq!(a, Bitset::from_word(0b0111));
    }

    #[test]
    fn iter_simple() {
        let b = Bitset::from_idxs(&[0, 3, 5]);
        let mut it = b.iter();
        assert_eq!(it.next(), Some(0));
        assert_eq!(it.next(), Some(3));
        assert_eq!(it.next(), Some(5));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn bititer_next_simple() {
        let b = Bitset::from_idxs(&[2, 4]);
        let mut it = BitIter { w: b };
        assert_eq!(it.next(), Some(2));
        assert_eq!(it.next(), Some(4));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn bititer_clone_independent_simple() {
        let b = Bitset::from_idxs(&[1, 2]);
        let mut it1 = BitIter { w: b };
        let mut it2 = it1;
        assert_eq!(it1.next(), Some(1));
        assert_eq!(it2.next(), Some(1));
        assert_eq!(it1.next(), Some(2));
        assert_eq!(it2.next(), Some(2));
    }

    #[test]
    fn ordering_simple() {
        let a = Bitset::from_word(0b0010);
        let b = Bitset::from_word(0b0100);
        let c = Bitset::from_word(0b0010);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, c);
        let mut vec = vec![b, a];
        vec.sort();
        assert_eq!(vec, vec![a, b]);
    }
}

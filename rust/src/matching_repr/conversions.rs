// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains all kinds of conversions from and to MaskedMatching.

use core::fmt;
use std::collections::HashMap;

use smallvec::SmallVec;

use crate::matching_repr::{bitset::Bitset, IdBase, MaskedMatching, Word, MATCH_MAX_LEN};

/// Error type for when the conversion fails
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionError {
    /// a type used during the conversion was too small
    UniverseTooLarge(usize, usize),
    /// the slot required in rhe conversion was not found
    RequiredSlotsNotFound,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConversionError::UniverseTooLarge(universe, max) => {
                write!(
                    f,
                    "universe ({}) too large to convert back to Matching (max {})",
                    universe, max
                )
            }
            ConversionError::RequiredSlotsNotFound => {
                write!(f, "Could not obtain the number of slots required",)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

impl MaskedMatching {
    /// Consume self and return the owned `SmallVec<Bitset>` masks.
    /// Use for zero-copy handoff: move the internal vector out, then re-use it.
    #[inline]
    pub fn into_masks(self) -> SmallVec<[Bitset; MATCH_MAX_LEN]> {
        self.masks
    }

    /// Construct from raw bitset masks.
    #[inline]
    pub fn from_masks(masks: SmallVec<[Bitset; MATCH_MAX_LEN]>) -> Self {
        MaskedMatching { masks }
    }

    /// Swap the internal Vec<Bitset> with `other`.
    /// This provides zero-copy handoff of mask storage to/from the MaskedMatching.
    /// After calling `self.swap_masks(&mut buf)`, `self` will own the contents of
    /// `buf` and `buf` will own what `self` used to own.
    #[inline]
    pub fn swap_masks(&mut self, other: &mut SmallVec<[Bitset; MATCH_MAX_LEN]>) {
        std::mem::swap(&mut self.masks, other)
    }

    /// Create empty with `slots` amount of space.
    #[inline]
    pub fn with_slots(slots: usize) -> Self {
        MaskedMatching {
            masks: SmallVec::from_elem(Bitset::empty(), slots),
        }
    }

    /// Replace the internal masks with the provided slice *without* allocating on every call,
    /// provided the internal Vec already has enough capacity.
    ///
    /// This method performs a *copy* of `slice` elements (cheap, u64-sized), but it avoids
    /// heap allocations if the internal Vec capacity >= slice.len().
    ///
    /// Notes:
    /// - the `slice` and self.masks must never overlap -> undefined behevior otherwise
    #[inline]
    pub fn set_masks_from_slice(&mut self, slice: &[Bitset])
    where
        Bitset: Copy,
    {
        // Ultra fast path - most common -> place as fist case
        if self.masks.len() == slice.len() {
            self.masks.as_mut_slice().copy_from_slice(slice);
            return;
        }
        // If capacity is insufficient, reserve once (might allocate once).
        if self.masks.capacity() < slice.len() {
            self.masks.reserve(slice.len() - self.masks.capacity());
        }
        // Fast path: if we can, overwrite existing elements and adjust length.
        // Safety notes:
        //  - We reserved enough capacity above.
        //  - as_mut_ptr() returns a pointer to the buffer; writing into uninitialized spare
        //    memory is allowed if we do not create invalid references and we set_len afterwards.
        //  - copy_nonoverlapping here is correct because src and dst do not overlap.
        unsafe {
            let dst = self.masks.as_mut_ptr();
            std::ptr::copy_nonoverlapping(slice.as_ptr(), dst, slice.len());
            self.masks.set_len(slice.len());
        }
    }

    /// The `m` is expected to be a slice of slots; each slot is a `Vec<IdBase>`
    /// listing value indices.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ayto::matching_repr::MaskedMatching;
    /// let m = vec![vec![0, 2], vec![1]];
    /// let mm = MaskedMatching::from_matching_ref(&m);
    /// assert_eq!(mm.computed_universe(), 3);
    /// ```
    pub fn from_matching_ref(m: &[Vec<IdBase>]) -> Self {
        let mut masks = SmallVec::with_capacity(m.len());
        for slot in m.iter() {
            // construct the value representing the vector 'slot'
            let mut w: Word = 0;
            for &v in slot.iter() {
                let idx = v as usize;
                // safe because we assume inputs fit in WORD_BITS
                w |= (1 as Word) << idx;
            }
            masks.push(Bitset::from_word(w));
        }
        MaskedMatching { masks }
    }
}

impl From<&[IdBase]> for MaskedMatching {
    fn from(ms: &[IdBase]) -> Self {
        let mut slots = SmallVec::with_capacity(ms.len());
        for m in ms {
            let b = Bitset::from_idxs(&[*m]);
            slots.push(b);
        }
        MaskedMatching::from_masks(slots)
    }
}

impl From<(IdBase, IdBase)> for MaskedMatching {
    fn from(m: (IdBase, IdBase)) -> Self {
        let mut slots = SmallVec::from_elem(Bitset::empty(), m.0 as usize + 1);
        let x: &mut Bitset = slots.get_mut(m.0 as usize).unwrap();
        x.insert(m.1);
        MaskedMatching::from_masks(slots)
    }
}

impl From<SmallVec<[Bitset; MATCH_MAX_LEN]>> for MaskedMatching {
    fn from(m: SmallVec<[Bitset; MATCH_MAX_LEN]>) -> Self {
        MaskedMatching { masks: m }
    }
}

impl TryFrom<HashMap<IdBase, IdBase>> for MaskedMatching {
    type Error = ConversionError;
    fn try_from(masked: HashMap<IdBase, IdBase>) -> Result<Self, Self::Error> {
        let max = *masked
            .keys()
            .max()
            .ok_or(ConversionError::RequiredSlotsNotFound)?;
        let mut slots = SmallVec::from_elem(Bitset::empty(), (max as usize) + 1);
        for (k, v) in masked.into_iter() {
            let x: &mut Bitset = slots.get_mut(k as usize).unwrap();
            x.insert(v);
        }
        Ok(MaskedMatching::from_masks(slots))
    }
}

impl TryFrom<&MaskedMatching> for Vec<Vec<IdBase>> {
    type Error = ConversionError;
    fn try_from(masked: &MaskedMatching) -> Result<Self, Self::Error> {
        let universe = masked.computed_universe();
        if universe > (IdBase::MAX as usize) + 1 {
            return Err(ConversionError::UniverseTooLarge(
                universe,
                IdBase::MAX as usize,
            ));
        }
        let mut out: Vec<Vec<IdBase>> = Vec::with_capacity(masked.len());
        for b in masked.masks.iter() {
            let mut slot = Vec::new();
            for i in b.iter() {
                slot.push(i);
            }
            out.push(slot);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn from_masks_simple() {
        let masks = SmallVec::from_slice(&[Bitset::from_word(0b101), Bitset::from_word(0b10)]);
        let mm = MaskedMatching::from_masks(masks.clone());
        assert_eq!(mm.masks, masks);
    }
    #[test]
    fn into_masks_simple() {
        let original_masks =
            SmallVec::from_slice(&[Bitset::from_word(0b1010), Bitset::from_word(0b0101)]);
        let mm = MaskedMatching::from_masks(original_masks.clone());
        let moved = mm.into_masks();
        assert_eq!(moved, original_masks);
    }

    #[test]
    fn swap_masks_simple() {
        let mut mm = MaskedMatching::from_masks(SmallVec::from_slice(&[
            Bitset::from_word(0b101),
            Bitset::from_word(0b010),
        ]));
        let mut external = SmallVec::from_slice(&[Bitset::from_word(0b111)]);

        mm.swap_masks(&mut external);

        assert_eq!(mm.masks.as_slice(), &[Bitset::from_word(0b111)]);
        assert_eq!(
            external.as_slice(),
            &[Bitset::from_word(0b101), Bitset::from_word(0b010),]
        );
    }

    #[test]
    fn with_slots_simple() {
        let mm2 = MaskedMatching::with_slots(3);
        assert_eq!(mm2.len(), 3);
        for (i, mask) in mm2.iter().enumerate() {
            assert!(mask.is_empty(), "slot {} should be empty", i);
        }
    }

    #[test]
    fn set_masks_from_slice_cap_enough() {
        // Path where capacity is already sufficient.
        let mut mm = MaskedMatching::with_slots(5);
        let slice = [
            Bitset::from_word(0b001),
            Bitset::from_word(0b010),
            Bitset::from_word(0b100),
        ];
        mm.set_masks_from_slice(&slice);
        assert_eq!(mm.masks.as_slice(), slice);
    }

    #[test]
    fn set_masks_from_slice_cap_too_small() {
        // Path where we need to reserve additional capacity.
        let mut mm = MaskedMatching::with_slots(0);
        let long_slice = [
            Bitset::from_word(0b1),
            Bitset::from_word(0b10),
            Bitset::from_word(0b100),
            Bitset::from_word(0b1000),
        ];

        let prev_cap = mm.masks.capacity();
        mm.set_masks_from_slice(&long_slice);

        assert_eq!(mm.masks.as_slice(), long_slice);
        assert!(mm.masks.capacity() >= long_slice.len());
        assert!(mm.masks.capacity() >= prev_cap);
    }

    #[test]
    fn from_matching_ref_and_into_vector() -> Result<()> {
        let m = vec![vec![1u8], vec![2u8, 3u8]];
        let mm = MaskedMatching::from_matching_ref(&m);
        assert_eq!(
            mm.masks.as_slice(),
            &[Bitset::from_idxs(&[1]), Bitset::from_idxs(&[2, 3]),]
        );
        let back: Vec<Vec<IdBase>> = Vec::try_from(&mm)?;
        assert_eq!(back, m);
        Ok(())
    }

    // [IdBase] -> MaskedMatching
    #[test]
    fn from_idbase_slice() {
        let mm = MaskedMatching::from(&[1 as IdBase, 2 as IdBase][..]);
        assert_eq!(
            mm.masks.as_slice(),
            &[Bitset::from_idxs(&[1]), Bitset::from_idxs(&[2]),]
        )
    }

    // (idBase, idBase) -> MaskedMatching
    #[test]
    fn from_idbase_tuple() {
        let mm = MaskedMatching::from((1 as IdBase, 2 as IdBase));
        assert_eq!(
            mm.masks.as_slice(),
            &[Bitset::from_idxs(&[]), Bitset::from_idxs(&[2]),]
        )
    }

    // Vec<Bitset> -> MaskedMatching
    #[test]
    fn from_bitset_vector() {
        let mm = MaskedMatching::from(SmallVec::from_slice(&[
            Bitset::from_idxs(&[1, 2]),
            Bitset::from_idxs(&[3, 4]),
        ]));
        assert_eq!(
            mm.masks.as_slice(),
            &[Bitset::from_idxs(&[1, 2]), Bitset::from_idxs(&[3, 4]),]
        )
    }

    // HashMap<IdBase, IdBase> -> MaskedMatching (tryfrom)
    #[test]
    fn try_from_idbase_hashmap() -> Result<()> {
        let mm = MaskedMatching::try_from(HashMap::from_iter([(0, 7), (2, 3)]))?;
        assert_eq!(
            mm.masks.as_slice(),
            &[
                Bitset::from_idxs(&[7]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[3]),
            ]
        );
        Ok(())
    }
    #[test]
    fn try_from_idbase_hashmap_conversion_error() {
        let empty_map: HashMap<IdBase, IdBase> = HashMap::new();
        let err = MaskedMatching::try_from(empty_map).unwrap_err();
        match err {
            ConversionError::RequiredSlotsNotFound => {}
            _ => panic!("expected RequiredSlotsNotFound error"),
        }
    }

    // MaskedMatching -> VecVecIdBase (tryfrom)
    #[test]
    fn try_to_matching_ref() -> Result<()> {
        let a = vec![vec![1, 2], vec![], vec![3, 4]];
        let b = Vec::<Vec<IdBase>>::try_from(&MaskedMatching::from_matching_ref(&a))?;
        assert_eq!(b, a);
        Ok(())
    }
}

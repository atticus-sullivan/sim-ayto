use core::fmt;
use std::convert::{From, TryFrom};

/// Legacy type (assumed in crate root)
use crate::Matching; // type Matching = Vec<Vec<u8>>;

/// Error for TryFrom conversion back to legacy Matching
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionError {
    UniverseTooLarge(usize, usize),
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
        }
    }
}

type Word = u64;
const WORD_BITS: usize = 64;
const WORD_BITS_LOG: usize = 6; // log2(64)

/// Private internal representation
#[derive(Clone, Debug)]
enum MaskRepr {
    /// Fast single-word per-slot
    Single(Vec<Word>), // masks[slot]
    /// Multi-word per-slot (words per slot may be > 1)
    Multi(Vec<Vec<Word>>), // masks[slot][word_idx]
}

/// Public type used by hot code.
#[derive(Clone, Debug)]
pub struct MaskedMatching {
    repr: MaskRepr,
    universe: usize,
}

impl MaskedMatching {
    /// Construct from legacy Matching reference (computes minimal universe).
    pub fn from_matching_ref(m: &Matching) -> Self {
        // find maximum value in input to determine universe
        let universe: usize = m
            .iter()
            .flat_map(|i| i.iter())
            .max()
            .map(|&x| (x as usize) + 1)
            .unwrap_or(0);
        Self::from_matching_with_universe(m, universe)
    }

    /// Construct with explicit universe (choose representation automatically).
    pub fn from_matching_with_universe(m: &Matching, universe: usize) -> Self {
        if universe <= WORD_BITS {
            // single word case
            let mut masks: Vec<Word> = vec![0; m.len()]; // 0 means empty here (bitmask)
            for (i, slot) in m.iter().enumerate() {
                // construct the value representing the vector 'slot'
                let mut w: Word = 0;
                for &v in slot.iter() {
                    let idx = v as usize;
                    // safe because universe <= 64
                    w |= (1 as Word) << idx;
                }
                masks[i] = w;
            }
            MaskedMatching {
                repr: MaskRepr::Single(masks),
                universe,
            }
        } else {
            // multi-word case
            let words = words_for_universe(universe);
            let mut masks: Vec<Vec<Word>> = Vec::with_capacity(m.len());
            for slot in m.iter() {
                // construct the value representing the vector 'slot'
                let mut w: Vec<Word> = vec![0; words]; // 0 means empty here (bitmask)
                for &v in slot.iter() {
                    let idx = v as usize;
                    let word = idx >> WORD_BITS_LOG;
                    let bit = idx & (WORD_BITS - 1);
                    w[word] |= (1 as Word) << bit;
                }
                masks.push(w);
            }
            MaskedMatching {
                repr: MaskRepr::Multi(masks),
                universe,
            }
        }
    }

    /// Construct directly from masks (caller ensures consistency).
    pub fn from_masks_single(masks: Vec<Word>, universe: usize) -> Self {
        debug_assert!(universe <= WORD_BITS);
        MaskedMatching {
            repr: MaskRepr::Single(masks),
            universe,
        }
    }

    /// Construct directly from masks (caller ensures consistency).
    pub fn from_masks_multi(masks: Vec<Vec<Word>>, universe: usize) -> Self {
        MaskedMatching {
            repr: MaskRepr::Multi(masks),
            universe,
        }
    }

    /// Number of slots
    pub fn len(&self) -> usize {
        match &self.repr {
            MaskRepr::Single(m) => m.len(),
            MaskRepr::Multi(m) => m.len(),
        }
    }

    /// Universe size
    pub fn universe(&self) -> usize {
        self.universe
    }

    /// Check membership of `val` in slot `i`.
    #[inline]
    pub fn contains(&self, i: usize, val: usize) -> bool {
        debug_assert!(val < self.universe || self.universe == 0);
        match &self.repr {
            MaskRepr::Single(masks) => {
                // single-word path: one lookup only
                let w = masks[i];
                (w & ((1 as Word) << (val & (WORD_BITS - 1)))) != 0
            }
            MaskRepr::Multi(masks) => {
                let slot = &masks[i];
                let word = val >> WORD_BITS_LOG;
                (slot[word] & ((1 as Word) << (val & (WORD_BITS - 1)))) != 0
            }
        }
    }

    /// Fast path for p consisting of single values per slot.
    /// match-once outside the loop -> monomorphic inner loop
    #[inline]
    pub fn count_matches_with_singles(&self, p_singles: &[u8]) -> u8 {
        match &self.repr {
            MaskRepr::Single(masks) => {
                let mut l = 0u8;
                // local alias and monomorphic loop
                let masks = masks;
                let n = masks.len().min(p_singles.len());
                for i in 0..n {
                    let v = p_singles[i] as usize;
                    if (masks[i] & ((1 as Word) << (v & (WORD_BITS - 1)))) != 0 {
                        l += 1;
                    }
                }
                l
            }
            MaskRepr::Multi(masks) => {
                let mut l = 0u8;
                let masks = masks;
                let n = masks.len().min(p_singles.len());
                for i in 0..n {
                    let v = p_singles[i] as usize;
                    let word = v >> WORD_BITS_LOG;
                    if (masks[i][word] & ((1 as Word) << (v & (WORD_BITS - 1)))) != 0 {
                        l += 1;
                    }
                }
                l
            }
        }
    }

    /// Generic full-p matching (handles multi-value p slots).
    #[inline]
    pub fn count_matches_full_p(&self, p: &Matching) -> u8 {
        match &self.repr {
            MaskRepr::Single(masks) => {
                let mut l = 0u8;
                let masks = masks;
                let n = masks.len().min(p.len());
                for i in 0..n {
                    let slot_mask = masks[i];
                    if let [v] = p[i][..] {
                        // fast path for singleton slot in p: length == 1
                        let vi = v as usize;
                        if (slot_mask & ((1 as Word) << (vi & (WORD_BITS - 1)))) != 0 {
                            l += 1;
                        }
                    } else {
                        // general path: test each v in p[i]
                        for &v in &p[i] {
                            let vi = v as usize;
                            if (slot_mask & ((1 as Word) << (vi & (WORD_BITS - 1)))) != 0 {
                                l += 1;
                                break;
                            }
                        }
                    }
                }
                l
            }
            MaskRepr::Multi(masks) => {
                let mut l = 0u8;
                let masks = masks;
                let n = masks.len().min(p.len());
                for i in 0..n {
                    let slot_mask = &masks[i];
                    if let [v] = p[i][..] {
                        // fast path for singleton slot in p: length == 1
                        let vi = v as usize;
                        let word = vi >> WORD_BITS_LOG;
                        if (slot_mask[word] & ((1 as Word) << (vi & (WORD_BITS - 1)))) != 0 {
                            l += 1;
                        }
                    } else {
                        // general path: test each v in p[i]
                        for &v in &p[i] {
                            let vi = v as usize;
                            let word = vi >> WORD_BITS_LOG;
                            if (slot_mask[word] & ((1 as Word) << (vi & (WORD_BITS - 1)))) != 0 {
                                l += 1;
                                break;
                            }
                        }
                    }
                }
                l
            }
        }
    }

    /// Iterate over slots: returns an iterator of `SlotProxy`, each slot can produce its own `.iter()`.
    pub fn iter(&self) -> SlotsIter<'_> {
        SlotsIter { mm: self, idx: 0 }
    }
}

#[inline]
fn words_for_universe(universe: usize) -> usize {
    if universe == 0 {
        0
    } else {
        (universe + WORD_BITS - 1) / WORD_BITS
    }
}

/// From<&Matching> and From<Matching>
impl From<&Matching> for MaskedMatching {
    fn from(m: &Matching) -> Self {
        MaskedMatching::from_matching_ref(m)
    }
}
impl From<Matching> for MaskedMatching {
    fn from(m: Matching) -> Self {
        MaskedMatching::from_matching_ref(&m)
    }
}

/// TryFrom back to Matching (fails if universe > 256 to avoid losing u8)
impl TryFrom<MaskedMatching> for Matching {
    type Error = ConversionError;
    fn try_from(masked: MaskedMatching) -> Result<Self, Self::Error> {
        if masked.universe > 256 {
            return Err(ConversionError::UniverseTooLarge(masked.universe, 255));
        }
        let mut out: Matching = Vec::with_capacity(masked.len());
        match masked.repr {
            MaskRepr::Single(masks) => {
                for (_, w) in masks.into_iter().enumerate() {
                    let mut slot = Vec::new();
                    let mut bits = w;
                    let base = 0usize;
                    while bits != 0 {
                        let tz = bits.trailing_zeros() as usize;
                        let val = base + tz;
                        if val >= masked.universe {
                            break;
                        }
                        slot.push(val as u8);
                        // turns the rightmost 1 to a 0
                        bits &= bits - 1;
                    }
                    out.push(slot);
                }
            }
            MaskRepr::Multi(masks) => {
                for slot_mask in masks.into_iter() {
                    let mut slot = Vec::new();
                    for (word_idx, word) in slot_mask.into_iter().enumerate() {
                        if word == 0 {
                            continue;
                        }
                        let base = word_idx * WORD_BITS;
                        let mut bits = word;
                        while bits != 0 {
                            let tz = bits.trailing_zeros() as usize;
                            let val = base + tz;
                            if val >= masked.universe {
                                break;
                            }
                            slot.push(val as u8);
                            // turns the rightmost 1 to a 0
                            bits &= bits - 1;
                        }
                    }
                    out.push(slot);
                }
            }
        }
        Ok(out)
    }
}

// TODO: proof read from here

/* ---------- Iterators ---------- */

/// Iterator over slots
pub struct SlotsIter<'a> {
    mm: &'a MaskedMatching,
    idx: usize,
}

/// Proxy representing a single slot (borrowed). It exposes `.iter()` to get values.
pub struct SlotProxy<'a> {
    mm: &'a MaskedMatching,
    idx: usize,
}

/// Iterator over values inside a slot (yields u8)
pub enum SlotValueIter<'a> {
    Single {
        bits: Word,
        base: usize,
    },
    Multi {
        words: &'a [Word],
        word_idx: usize,
        bits: Word,
        base: usize,
        universe: usize,
    },
}

impl<'a> Iterator for SlotsIter<'a> {
    type Item = SlotProxy<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.mm.len() {
            None
        } else {
            let cur = self.idx;
            self.idx += 1;
            Some(SlotProxy {
                mm: self.mm,
                idx: cur,
            })
        }
    }
}

impl<'a> SlotProxy<'a> {
    /// produce an iterator over the values present in this slot
    pub fn iter(&self) -> SlotValueIter<'a> {
        match &self.mm.repr {
            MaskRepr::Single(masks) => {
                let bits = masks[self.idx];
                SlotValueIter::Single { bits, base: 0 }
            }
            MaskRepr::Multi(masks) => {
                let words_slice = &masks[self.idx];
                let initial_bits = if words_slice.is_empty() {
                    0
                } else {
                    words_slice[0]
                };
                SlotValueIter::Multi {
                    words: words_slice,
                    word_idx: 0,
                    bits: initial_bits,
                    base: 0,
                    universe: self.mm.universe,
                }
            }
        }
    }
}

impl<'a> Iterator for SlotValueIter<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SlotValueIter::Single { bits, base } => {
                if *bits == 0 {
                    return None;
                }
                let tz = bits.trailing_zeros() as usize;
                let val = *base + tz;
                // clear the lowest set bit
                *bits &= *bits - 1;
                // Safety: caller should ensure val fits u8 if they rely on that; we yield u8 nonetheless
                Some(val as u8)
            }
            SlotValueIter::Multi {
                words,
                word_idx,
                bits,
                base,
                universe,
            } => {
                let mut idx = *word_idx;
                let mut b = *bits;
                loop {
                    if b != 0 {
                        let tz = b.trailing_zeros() as usize;
                        let val = *base + tz;
                        if val >= *universe {
                            return None;
                        }
                        // clear lowest set bit and update
                        b &= b - 1;
                        *bits = b;
                        return Some(val as u8);
                    } else {
                        idx += 1;
                        if idx >= words.len() {
                            // end of words -> no more values
                            *word_idx = idx;
                            *bits = 0;
                            return None;
                        }
                        // advance base and load next word
                        *base = idx * WORD_BITS;
                        b = words[idx];
                        *word_idx = idx;
                        *bits = b;
                        // loop continues to test b
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_matching_and_contains_single() {
        let legacy: Matching = vec![vec![1], vec![2, 3], vec![], vec![0, 63]];
        let mm = MaskedMatching::from(&legacy);
        assert_eq!(mm.universe(), 64);
        assert_eq!(mm.len(), legacy.len());

        // contains checks
        assert!(mm.contains(0, 1));
        assert!(!mm.contains(0, 0));
        assert!(mm.contains(1, 2));
        assert!(mm.contains(1, 3));
        assert!(!mm.contains(1, 4));
        assert!(mm.contains(3, 63));
        assert!(!mm.contains(3, 62));
    }

    #[test]
    fn test_count_matches_with_singles() {
        let legacy: Matching = vec![vec![1], vec![2, 3], vec![5], vec![]];
        let mm = MaskedMatching::from(&legacy);
        let p_singles: Vec<u8> = vec![1, 3, 5, 0];
        assert_eq!(mm.count_matches_with_singles(&p_singles), 3);
    }

    #[test]
    fn test_count_matches_full_p_various() {
        // slot0: {1}, slot1: {2,3}, slot2: {}, slot3: {4,6}
        let legacy: Matching = vec![vec![1], vec![2, 3], vec![], vec![4, 6]];
        let mm = MaskedMatching::from(&legacy);

        // p with slots empty / single / multiple
        let p1: Matching = vec![vec![1], vec![2], vec![], vec![6]];
        assert_eq!(mm.count_matches_full_p(&p1), 3);

        // p with multiple entries in a slot
        let p2: Matching = vec![vec![9], vec![9, 3], vec![], vec![6, 0]];
        assert_eq!(mm.count_matches_full_p(&p2), 2);

        // empty slot candidates
        let p3: Matching = vec![vec![], vec![], vec![], vec![]];
        assert_eq!(mm.count_matches_full_p(&p3), 0);
    }

    #[test]
    fn test_iter_roundtrip() {
        let legacy: Matching = vec![vec![1], vec![2, 3], vec![], vec![5]];
        let mm = MaskedMatching::from(&legacy);
        // Collect via iterator API
        let mut from_iter: Matching = Vec::new();
        for slot in mm.iter() {
            let vals: Vec<u8> = slot.iter().collect();
            from_iter.push(vals);
        }
        // But note: the order of elements inside a slot is by increasing numeric value (pop lowest-bit),
        // which matches how we constructed the legacy matching and thus should be equal.
        assert_eq!(from_iter, legacy);
    }

    #[test]
    fn test_try_from_roundtrip_and_error() {
        // small universe -> roundtrip ok
        let legacy: Matching = vec![vec![1], vec![2, 3], vec![0]];
        let mm = MaskedMatching::from(&legacy);
        let back: Matching = mm.clone().try_into().expect("should convert back");
        // ordering of elements in each slot is increasing value; legacy happens to be in such order
        assert_eq!(back, legacy);

        // To create a real >255 value you must have used a different type originally.
        // For the test we emulate a masked with universe > 256 directly:
        let mm_large = MaskedMatching::from_masks_multi(vec![vec![0u64; 3]], 300usize);
        let err = Matching::try_from(mm_large).unwrap_err();
        assert_eq!(err, ConversionError::UniverseTooLarge(300, 255));
    }

    #[test]
    fn test_multi_word_representation() {
        // values > 64 should cause multi-word
        let legacy: Matching = vec![vec![1], vec![70], vec![130, 3]];
        let mm = MaskedMatching::from_matching_with_universe(&legacy, 131);
        // verify multi-word path works
        assert!(mm.contains(0, 1));
        assert!(mm.contains(1, 70));
        assert!(mm.contains(2, 130));
        assert!(mm.contains(2, 3));
    }
}

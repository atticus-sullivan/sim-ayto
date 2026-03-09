// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This ruleset_data can be used if no specific data should be collected. The functions which need
//! to be implemented are simple no-ops.

use anyhow::Result;

use crate::matching_repr::MaskedMatching;
use crate::ruleset::RuleSet;
use crate::ruleset_data::RuleSetData;
use crate::Lut;

/// Dummy implementation of [`super::RuleSetData`] used when the ruleset does not need
/// per-solution statistics => no-op
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DummyData {}

impl RuleSetData for DummyData {
    fn push(&mut self, _m: &MaskedMatching) -> Result<()> {
        Ok(())
    }

    fn print(
        &self,
        _full: bool,
        _ruleset: &RuleSet,
        _map_a: &[String],
        _map_b: &[String],
        _lut_b: &Lut,
        _total: u128,
    ) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching_repr::MaskedMatching;

    #[test]
    fn dummy_push_and_print_are_noop() {
        let mut d = DummyData::default();
        let m = MaskedMatching::from_matching_ref(&[vec![0u8]]);
        d.push(&m).expect("push failed");
        d.print(
            false,
            &crate::ruleset::RuleSet::Eq,
            &[],
            &[],
            &crate::Lut::default(),
            1,
        )
        .expect("print failed");
    }
}

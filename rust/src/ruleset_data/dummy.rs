use crate::ruleset::RuleSet;
use crate::ruleset_data::RuleSetData;
use crate::Lut;
use crate::Matching;
use anyhow::Result;

#[derive(Debug, Clone, Default)]
pub struct DummyData {}

impl RuleSetData for DummyData {
    fn push(&mut self, _m: &Matching) -> Result<()> {
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

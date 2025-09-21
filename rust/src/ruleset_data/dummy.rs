use crate::ruleset::RuleSet;
use crate::ruleset_data::RuleSetData;
use crate::Lut;
use crate::Matching;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct DummyData {}

impl std::default::Default for DummyData {
    fn default() -> Self {
        Self {}
    }
}

impl RuleSetData for DummyData {
    fn push(&mut self, _m: &Matching) -> Result<()> {
        Ok(())
    }

    fn print(
        &self,
        _full: bool,
        _ruleset: &RuleSet,
        _map_a: &Vec<String>,
        _map_b: &Vec<String>,
        _lut_b: &Lut,
        _total: u128,
    ) -> Result<()> {
        Ok(())
    }
}

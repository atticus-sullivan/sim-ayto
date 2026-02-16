pub mod dummy;
pub mod dup;
pub mod dup_x;

use crate::matching_repr::MaskedMatching;
use crate::ruleset::RuleSet;
use crate::Lut;
use anyhow::Result;

pub trait RuleSetDataClone {
    fn clone_box(&self) -> Box<dyn RuleSetData>;
}
impl<T> RuleSetDataClone for T
where
    T: 'static + RuleSetData + Clone,
{
    fn clone_box(&self) -> Box<dyn RuleSetData> {
        Box::new(self.clone())
    }
}

pub trait RuleSetData: std::fmt::Debug + RuleSetDataClone {
    fn push(&mut self, m: &MaskedMatching) -> Result<()>;
    fn print(
        &self,
        full: bool,
        ruleset: &RuleSet,
        map_a: &[String],
        map_b: &[String],
        lut_b: &Lut,
        total: u128,
    ) -> Result<()>;
}

impl Clone for Box<dyn RuleSetData> {
    fn clone(&self) -> Box<dyn RuleSetData> {
        self.clone_box()
    }
}

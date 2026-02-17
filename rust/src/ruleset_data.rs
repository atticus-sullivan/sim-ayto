pub mod dummy;
pub mod dup;
pub mod dup_x;

use crate::matching_repr::MaskedMatching;
use crate::ruleset::RuleSet;
use crate::Lut;
use anyhow::Result;

/// Small helper trait to allow cloning boxed trait objects.
///
/// Implementors of `RuleSetData` should derive/impl `Clone` and `RuleSetDataClone`
/// will provide a boxed clone via `clone_box`.
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

/// Per-ruleset data collector used while evaluating permutations.
///
/// Implementations may collect statistics (e.g. duplicate/trip counts) while
/// the simulation runs, then render human-readable output via `print`.
pub trait RuleSetData: std::fmt::Debug + RuleSetDataClone {
    /// Called for each solution matching encountered (append/accumulate).
    fn push(&mut self, m: &MaskedMatching) -> Result<()>;

    /// Print or otherwise render collected statistics.
    ///
    /// `full` indicates whether to emit the full report or a short "top-k" summary.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruleset_data::dummy::DummyData;

    #[test]
    fn boxed_rulesetdata_clone_works() {
        let d: Box<dyn RuleSetData> = Box::new(DummyData::default());
        let _d2 = d.clone(); // should not panic
    }
}

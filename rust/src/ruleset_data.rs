/*
sim_ayto
Copyright (C) 2025  Lukas Heindl

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

pub mod dummy;
pub mod dup;
pub mod dup_x;

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
    fn push(&mut self, m: &[Vec<u8>]) -> Result<()>;
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

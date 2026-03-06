use std::sync::Arc;
use std::{fs::File, path::Path};

use ayto::constraint::parse_utils::convert_map_s_to_ids;
use ayto::constraint::ConstraintGetters;
use ayto::game::parse_utils::{build_luts, process_constraints};
use ayto::ignore_ops::IgnoreOps;
use ayto::matching_repr::MaskedMatching;
use ayto::ruleset::RuleSet;
use ayto::MapS;
use ayto::{constraint::parse::ConstraintParse, ruleset::parse::RuleSetParse};
use serde::Deserialize;
use anyhow::{bail, ensure, Result};

use crate::engine::Simulation;
use crate::strategies::StrategyBundle;
use crate::NUM_PLAYERS_SET_A;


/// this struct is only used for parsing the yaml file
#[derive(Deserialize, Debug)]
pub struct CfgParse {
    /// the ruleset which is to be applied to this game
    rule_set: RuleSetParse,
    /// the constraints in this game
    #[serde(rename = "constraints")]
    constraints_orig: Vec<ConstraintParse>,

    /// the set of individuals in set_a (also maps idx_a to name_a)
    #[serde(rename = "setA")]
    map_a: Vec<String>,
    /// the set of individuals in set_b (also maps idx_b to name_b)
    #[serde(rename = "setB")]
    map_b: Vec<String>,

    solution: MapS,

    #[serde(default, rename = "try_entropy")]
    try_entropy: Option<MapS>,

}

impl CfgParse {
    /// create a `CfgParse` from a yaml config. This struct can then be finalized to a `Cfg`
    pub fn new_from_yaml(yaml_path: &Path) -> Result<CfgParse> {
        let gp: CfgParse = serde_yaml::from_reader(File::open(yaml_path)?)?;
        Ok(gp)
    }

    pub fn finalize_parsing<S: StrategyBundle>(self, sim_id: usize, seed: u64, strategy: Arc<S>) -> Result<(MaskedMatching,Option<MaskedMatching>,Simulation<S>)> {
        ensure!(self.map_a.len() == NUM_PLAYERS_SET_A);
        ensure!(self.map_b.len() == NUM_PLAYERS_SET_A);

        // build up the look up tables (LUT)
        let (lut_a, lut_b) = build_luts(&self.map_a, &self.map_b)?;
        ensure!(lut_a.len() == NUM_PLAYERS_SET_A);
        ensure!(lut_b.len() == NUM_PLAYERS_SET_A);

        let rule_set = self.rule_set.finalize_parsing();
        ensure!(matches!(rule_set, RuleSet::Eq));

        let (constraints, lights) = process_constraints(
            self.constraints_orig,
            &IgnoreOps::Nothing,
            &lut_a,
            &lut_b,
            &rule_set,
            &Default::default(),
            &Default::default(),
            &self.map_b,
        )?;

        let (solution, _) = convert_map_s_to_ids(&self.solution, &lut_a, &lut_b)?;
        let solution = solution.try_into()?;

        let try_entropy = self.try_entropy.map(|t| -> Result<MaskedMatching> {
            let (map, _) = convert_map_s_to_ids(&t, &lut_a, &lut_b)?;
            Ok(map.try_into()?)
        }).transpose()?;

        for c in &constraints {
            let l = c.matching().calculate_lights(&solution);
            if let Some(l_prime) = c.check.as_lights() {
                ensure!(l == l_prime);
            } else {
                bail!("Only constraints with light-check are supported");
            }
        }

        Ok((
            solution,
            try_entropy,
            Simulation::new_user_initialized(
                sim_id,
                seed,
                strategy,
                rule_set,
                constraints,
                lights,
                lut_a,
                Some(1),
            )?
        ))
    }
}

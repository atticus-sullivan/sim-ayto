/*
sim_ayto
Copyright (C) 2024  Lukas Heindl

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

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs::File;
use std::path::PathBuf;

use comfy_table::presets::NOTHING;
use comfy_table::{Cell, Row, Table};

use crate::ruleset::RuleSetData;
use crate::{Lut, Map, MapS, Matching, Rem, Rename};

#[derive(Serialize, Deserialize, Debug)]
pub struct CSVEntry {
    pub num: f64,
    pub bits_left: f64,
    pub lights_total: Option<u8>,
    pub lights_known_before: Option<u8>,
    pub comment: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CSVEntryMB {
    pub num: f64,
    pub lights_total: Option<u8>,
    pub lights_known_before: Option<u8>,
    pub bits_gained: f64,
    pub comment: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CSVEntryMN {
    pub won: bool,
    pub lights_total: Option<u8>,
    pub lights_known_before: Option<u8>,
    pub num: f64,
    pub bits_gained: f64,
    pub comment: String,
}

#[derive(Deserialize, Debug, Clone)]
enum CheckType {
    Eq,
    Nothing,
    Lights(u8, #[serde(skip)] BTreeMap<u8, u128>),
}

impl CheckType {
    pub fn as_lights(&self) -> Option<u8> {
        if let CheckType::Lights(l, _) = *self {
            Some(l)
        } else {
            None
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
enum ConstraintType {
    Night { num: f32, comment: String },
    Box { num: f32, comment: String },
}

// this struct is only used when parsing the yaml file.
// The function `finalize_parsing` is intended to convert this to a regular constraint.
#[derive(Deserialize, Debug, Clone)]
pub struct ConstraintParse {
    r#type: ConstraintType,
    #[serde(rename = "map")]
    map_s: MapS,
    check: CheckType,
    #[serde(default)]
    hidden: bool,
    #[serde(default, rename = "noExclude")]
    no_exclude: bool,
    #[serde(rename = "exclude")]
    exclude_s: Option<(String, Vec<String>)>,
    #[serde(default, rename = "resultUnknown")]
    result_unknown: bool,
    #[serde(default, rename = "buildTree")]
    build_tree: bool,
    #[serde(default, rename = "hideRulesetData")]
    hide_ruleset_data: bool,
}

#[derive(Debug, Clone)]
pub struct Constraint {
    r#type: ConstraintType,
    check: CheckType,
    hidden: bool,
    result_unknown: bool,
    build_tree: bool,

    map: Map,
    map_s: MapS,
    exclude: Option<(u8, HashSet<u8>)>,
    eliminated: u128,
    eliminated_tab: Vec<Vec<u128>>,

    information: Option<f64>,
    left_after: Option<u128>,
    left_poss: Vec<Matching>,

    hide_ruleset_data: bool,
    pub ruleset_data: Box<dyn RuleSetData>,
}

// functions for initialization / startup
impl ConstraintParse {
    /// Finalize the initialization phase by translating the names (strings) to ids, validating the
    /// stored data, initialize the internal state of the constraint, optionally add an exclude map
    /// and optionally sort the constraints.
    ///
    /// # Arguments
    ///
    /// - `lut_a`: Reference to the lookup table for set_a (the keys)
    /// - `lut_b`: Reference to the lookup table for set_b (the values)
    /// - `map_len`: How many elements are expected to occur in the matching night
    /// - `map_b`: Reference to the set of elements in set_b used to generate the exclude map
    /// - `add_exclude`: whether to automatically add the exclude map
    /// - `sort_constraint`: whether to sort the maps used for this constraint
    /// - `rename`: Maps one name to another name for renaming the names of set_a and set_b
    pub fn finalize_parsing(
        self,
        lut_a: &Lut,
        lut_b: &Lut,
        map_len: usize,
        map_b: &Vec<String>,
        add_exclude: bool,
        sort_constraint: bool,
        rename: (&Rename, &Rename),
        ruleset_data: Box<dyn RuleSetData>,
    ) -> Result<Constraint> {
        let exclude_s = if add_exclude {
            match self.add_exclude(map_b) {
                Some(e) => Some(e),
                None => self.exclude_s.clone(),
            }
        } else {
            self.exclude_s.clone()
        };

        let mut c = Constraint {
            r#type: self.r#type,
            check: self.check,
            hidden: self.hidden,
            result_unknown: self.result_unknown,
            build_tree: self.build_tree,
            map_s: self.map_s,
            map: Map::default(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![vec![0; lut_b.len()]; lut_a.len()],
            information: None,
            left_after: None,
            left_poss: Default::default(),
            ruleset_data,
            hide_ruleset_data: self.hide_ruleset_data,
        };

        c.map = c
            .map_s
            .iter()
            .map(&|(k, v)| {
                let k = *lut_a.get(k).with_context(|| format!("Invalid Key {}", k))? as u8;
                let v = *lut_b
                    .get(v)
                    .with_context(|| format!("Invalid Value {}", v))? as u8;
                Ok((k, v))
            })
            .collect::<Result<_>>()?;

        // check if map size is valid
        match c.r#type {
            ConstraintType::Night { .. } => {
                ensure!(
                    c.map_s.len() == map_len,
                    "Map in a night must contain exactly as many entries as set_a {} (was: {})",
                    map_len,
                    c.map_s.len()
                );
                let value_len = c.map_s.iter().map(|(_, v)| v).collect::<HashSet<_>>().len();
                ensure!(
                    value_len == c.map_s.len(),
                    "Keys in the map of a night must be unique"
                );
                ensure!(
                    self.exclude_s.is_none(),
                    "Exclude is not yet supported for nights"
                );
            }
            ConstraintType::Box { .. } => match &c.check {
                CheckType::Eq => {}
                CheckType::Nothing => {}
                CheckType::Lights(_, _) => {
                    ensure!(
                        c.map_s.len() == 1,
                        "Map in a box must contain exactly {} entry (was: {})",
                        1,
                        c.map_s.len()
                    );
                }
            },
        }

        // rename names in map_s for output use
        let mut map_s = MapS::default();
        for (k, v) in &c.map_s {
            map_s.insert(
                rename.0.get(k).unwrap_or(k).to_owned(),
                rename.1.get(v).unwrap_or(v).to_owned(),
            );
        }
        c.map_s = map_s;

        if sort_constraint {
            c.sort_maps(lut_a, lut_b);
        }

        // translate names to ids
        if let Some(ex) = &exclude_s {
            let (ex_a, ex_b) = ex;
            let mut bs = HashSet::with_capacity(ex_b.len());
            let a = *lut_a
                .get(ex_a)
                .with_context(|| format!("Invalid Key {}", ex_a))? as u8;
            for x in ex_b {
                bs.insert(
                    *lut_b
                        .get(x)
                        .with_context(|| format!("Invalid Value {}", x))? as u8,
                );
            }
            c.exclude = Some((a, bs));
        }

        Ok(c)
    }

    /// Generates the exclude list for the constraint, by inserting the elements from `map_b`
    ///
    /// This function modifies the internal state of the `Constraint`. If exclusion is not needed (constraint type is no box or lights != 1), no exclude list is generated.
    ///
    /// # Arguments
    ///
    /// - `map_b`: A reference to a vector of strings (`Vec<String>`) from which exclusions will be drawn. The function will create a new exclusion vector by removing any elements from `map_b` that match the current value in `self.map_s`.
    fn add_exclude(&self, map_b: &Vec<String>) -> Option<(String, Vec<String>)> {
        if self.no_exclude {
            return None;
        }
        if let CheckType::Lights(l, _) = self.check {
            if !(l == 1 && self.map_s.len() == 1 && self.exclude_s.is_none()) {
                return None;
            }
            if let ConstraintType::Box { .. } = self.r#type {
                // if the constraint is a box constraint the for loop will only run once anyhow
                for (k, v) in &self.map_s {
                    let bs: Vec<String> = map_b
                        .iter()
                        .filter(|&i| i != v)
                        .map(|i| i.to_string())
                        .collect();
                    return Some((k.to_string(), bs));
                }
            }
        }
        None
    }
}

// functions for initialization / startup
impl Constraint {
    /// Sorts and key/value pairs such that lut_a[k] < lut_b[v] always holds.
    /// Only makes sense if lut_a == lut_b (defined on the same set)
    ///
    /// # Arguments
    ///
    /// - `lut_a`: A lookup table of type `Lut` used for value comparison with `self.map_s`.
    /// - `lut_b`: A lookup table of type `Lut` used for value comparison with `self.map_s`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut c: Constraint;
    /// c.sort_maps(&lut_a, &lut_b);
    /// ```
    ///
    /// # Panics
    ///
    /// This function may panic if `lut_a` or `lut_b` do not contain keys present in `self.map_s`.
    ///
    /// # Notes
    ///
    /// - The sorting and flipping operations are done in place.
    fn sort_maps(&mut self, lut_a: &Lut, lut_b: &Lut) {
        self.map = self
            .map
            .drain()
            .map(|(k, v)| if k < v { (v, k) } else { (k, v) })
            .collect();

        self.map_s = self
            .map_s
            .drain()
            .map(|(k, v)| {
                if lut_a[&k] < lut_b[&v] {
                    (v, k)
                } else {
                    (k, v)
                }
            })
            .collect();
    }
}

// internal helper functions
impl Constraint {
    fn show_lights_information(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => true,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing => false,
        }
    }

    fn show_expected_information(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => true,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing => false,
        }
    }

    fn show_past_cnt(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => false,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing => false,
        }
    }

    fn show_new(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => false,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing => false,
        }
    }

    fn show_past_dist(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => false,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing => false,
        }
    }

    fn adds_new(&self) -> bool {
        let r = match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => true,
        };
        r && match &self.check {
            CheckType::Lights { .. } => true,
            CheckType::Eq => false,
            CheckType::Nothing => false,
        }
    }

    fn eliminate(&mut self, m: &Matching) {
        for (i1, v) in m.iter().enumerate() {
            for &i2 in v {
                if i2 == u8::MAX {
                    continue;
                }
                self.eliminated_tab[i1][i2 as usize] += 1
            }
        }
        self.eliminated += 1;
    }
}

// functions for processing/executing the simulation
impl Constraint {
    // returns if the matching fits the constraint (is not eliminated)
    pub fn process(&mut self, m: &Matching) -> Result<bool> {
        // first step is to check if the constraint filters out this matching
        let mut fits = None;
        match &mut self.check {
            CheckType::Eq => {
                fits = Some(m.iter().enumerate().all(|(_, js)| {
                    self.map
                        .iter()
                        .map(|(_, i2)| js.contains(i2))
                        .fold(None, |acc, b| match acc {
                            Some(a) => Some(a == b),
                            None => Some(b),
                        })
                        .unwrap()
                }));
            }
            CheckType::Nothing => fits = Some(true),
            CheckType::Lights(ref lights, ref mut light_count) => {
                let mut l = 0;
                for (i1, i2) in self.map.iter() {
                    if m[*i1 as usize].contains(i2) {
                        l += 1;
                    }
                }
                if let Some(ex) = &self.exclude {
                    for i in &m[ex.0 as usize] {
                        if ex.1.contains(i) {
                            fits = Some(false);
                            break;
                        }
                    }
                }
                // might be already set due to exclude
                fits.get_or_insert(l == *lights);
                // use calculated lights to collect stats on based on the matching possible until
                // here, how many lights are calculated how often for this map
                *light_count.entry(l).or_insert(0) += 1;
            }
        };

        // check fits actually has a value and make it immutable
        let fits = fits.with_context(|| {
            format!(
                "failure in calculating wether matching {:?} fits constraint {:?}",
                m, self
            )
        })? || self.result_unknown;

        if !fits {
            self.eliminate(m);
        } else {
            if self.build_tree && !self.hidden {
                self.left_poss.push(m.clone());
            }
            if !self.hide_ruleset_data && !self.hidden {
                self.ruleset_data.push(m)?;
            }
        }

        Ok(fits)
    }
}

// functions for evaluation
impl Constraint {
    pub fn should_merge(&self) -> bool {
        self.hidden
    }

    pub fn merge(&mut self, other: &Self) -> Result<()> {
        self.eliminated += other.eliminated;
        ensure!(
            self.eliminated_tab.len() == other.eliminated_tab.len(),
            "eliminated_tab lengths do not match (self: {}, other: {})",
            self.eliminated_tab.len(),
            other.eliminated_tab.len()
        );
        for (i, es) in self.eliminated_tab.iter_mut().enumerate() {
            ensure!(
                es.len() == other.eliminated_tab[i].len(),
                "eliminated_tab lengths do not match (self: {}, other: {})",
                es.len(),
                other.eliminated_tab[i].len()
            );
            for (j, e) in es.iter_mut().enumerate() {
                *e += other.eliminated_tab[i][j];
            }
        }
        self.information = None;
        self.left_after = None;
        Ok(())
    }

    pub fn apply_to_rem(&mut self, mut rem: Rem) -> Option<Rem> {
        rem.1 -= self.eliminated;

        for (i, rs) in rem.0.iter_mut().enumerate() {
            for (j, r) in rs.iter_mut().enumerate() {
                *r -= self.eliminated_tab.get(i)?.get(j)?;
            }
        }

        self.left_after = Some(rem.1);

        let tmp = 1.0 - (self.eliminated as f64) / (rem.1 + self.eliminated) as f64;
        self.information = if tmp == 1.0 {
            Some(0.0)
        } else if tmp > 0.0 {
            Some(-tmp.log2())
        } else {
            None
        };

        Some(rem)
    }

    pub fn build_tree(
        &self,
        path: PathBuf,
        map_a: &Vec<String>,
        map_b: &Vec<String>,
    ) -> Result<bool> {
        if !self.build_tree {
            return Ok(false);
        }

        let ordering = crate::utils::tree_ordering(&self.left_poss, map_a);
        crate::utils::dot_tree(
            &self.left_poss,
            &ordering,
            &(self.type_str() + " / " + self.comment()),
            &mut File::create(path)?,
            &map_a,
            &map_b,
        )?;
        Ok(true)
    }

    pub fn stat_row(
        &self,
        transpose: bool,
        map_hor: &[String],
        past_constraints: &Vec<&Constraint>,
    ) -> Vec<Cell> {
        let map_rev: MapS;
        let map_s: &MapS;
        if !transpose {
            map_s = &self.map_s;
        } else {
            map_rev = self
                .map_s
                .iter()
                .map(|(k, v)| (v.clone(), k.clone()))
                .collect();
            map_s = &map_rev;
        }

        let mut ret = vec![];
        match self.r#type {
            ConstraintType::Night { num, .. } => ret.push(Cell::new(format!("MN#{:02.1}", num))),
            ConstraintType::Box { num, .. } => ret.push(Cell::new(format!("MB#{:02.1}", num))),
        }
        let mut color = None;
        if self.result_unknown {
            ret.push(Cell::new("?"));
        } else {
            match &self.check {
                CheckType::Eq => ret.push(Cell::new("E")),
                CheckType::Nothing => match self.r#type {
                    ConstraintType::Night { .. } => ret.push(Cell::new("?")),
                    ConstraintType::Box { .. } => {
                        ret.push(Cell::new("?").fg(comfy_table::Color::Yellow))
                    }
                },
                CheckType::Lights(lights, _) => {
                    let lights = *lights;
                    match self.r#type {
                        ConstraintType::Night { .. } => ret.push(Cell::new(lights)),
                        ConstraintType::Box { .. } => {
                            if lights == 1 {
                                ret.push(Cell::new(lights).fg(comfy_table::Color::Green));
                                color = Some(comfy_table::Color::Green);
                            } else if lights == 0 {
                                ret.push(Cell::new(lights).fg(comfy_table::Color::Red));
                                color = Some(comfy_table::Color::Red);
                            } else {
                                ret.push(Cell::new(lights));
                            }
                        }
                    }
                }
            }
        }
        ret.extend(map_hor.iter().map(|v1| match map_s.get(v1) {
            Some(v2) => {
                let a;
                let b;
                if !transpose {
                    a = v1;
                    b = v2;
                } else {
                    a = v2;
                    b = v1;
                }
                if self.show_new()
                    && !past_constraints
                        .iter()
                        .any(|&c| c.adds_new() && c.map_s.get(a).is_some_and(|v2| v2 == b))
                {
                    match color {
                        Some(c) => Cell::new(format!("{}*", v2)).fg(c),
                        None => Cell::new(format!("{}*", v2)),
                    }
                } else {
                    match color {
                        Some(c) => Cell::new(&String::from(v2)).fg(c),
                        None => Cell::new(&String::from(v2)),
                    }
                }
            }
            None => Cell::new(&String::from("")),
        }));
        ret.push(Cell::new(String::from("")));

        match &self.check {
            CheckType::Eq | CheckType::Lights(..) => ret.push(Cell::new(
                format!("{:6.4}", self.information.unwrap_or(std::f64::INFINITY))
                    .trim_end_matches('0')
                    .trim_end_matches('.'),
            )),
            CheckType::Nothing => ret.push(Cell::new(String::from(""))),
        }

        // show how many new matches are present
        if let ConstraintType::Night { .. } = self.r#type {
            let cnt = self.map.len()
                - self
                    .map
                    .iter()
                    .filter(|&(k, v)| {
                        past_constraints
                            .iter()
                            .any(|&c| c.adds_new() && c.map.get(k).is_some_and(|v2| v2 == v))
                    })
                    .count();
            ret.push(Cell::new(cnt.to_string()));
        } else {
            ret.push(Cell::new(String::from("")));
        }

        if self.show_past_dist() {
            let dist = past_constraints
                .iter()
                .filter(|&c| c.show_past_dist())
                .map(|&c| (c.type_str(), self.distance(c).unwrap_or(usize::MAX)))
                .min_by_key(|i| i.1);
            match dist {
                Some(dist) => ret.push(Cell::new(format!("{}/{}", dist.1, dist.0))),
                None => ret.push(Cell::new(String::from(""))),
            }
        } else {
            ret.push(Cell::new(String::from("")));
        }

        ret
    }

    pub fn distance(&self, other: &Constraint) -> Option<usize> {
        if !self.show_past_dist() || !other.show_past_dist() {
            return None;
        }
        if self.map.len() != other.map.len() {
            return None;
        }

        Some(
            self.map.len()
                - self
                    .map
                    .iter()
                    .filter(|&(k, v)| other.map.get(k).is_some_and(|v2| v2 == v))
                    .count(),
        )
    }

    // returned array contains mbInfo, mnInfo, info
    pub fn get_stats(
        &self,
        required_lights: usize,
    ) -> Result<(Option<CSVEntryMB>, Option<CSVEntryMN>, Option<CSVEntry>)> {
        if self.hidden {
            return Ok((None, None, None));
        }

        let won = match self.check {
            CheckType::Eq => false,
            CheckType::Nothing => false,
            CheckType::Lights(l, _) => l as usize == required_lights,
        };

        let meta_a = format!("{}", self.comment());
        let meta_b = format!("{}-{}", self.type_str(), self.comment());
        match self.r#type {
            ConstraintType::Night { num, .. } => Ok((
                None,
                Some(CSVEntryMN {
                    won,
                    num: num.into(),
                    lights_total: self.check.as_lights(),
                    lights_known_before: None,
                    bits_gained: self.information.unwrap_or(std::f64::INFINITY),
                    comment: meta_a,
                }),
                Some(CSVEntry {
                    num: (num * 2.0).into(),
                    lights_total: self.check.as_lights(),
                    lights_known_before: None,
                    bits_left: (self.left_after.context("total_left unset")? as f64).log2(),
                    comment: meta_b,
                }),
            )),
            ConstraintType::Box { num, .. } => Ok((
                Some(CSVEntryMB {
                    num: num.into(),
                    lights_total: self.check.as_lights(),
                    lights_known_before: None,
                    bits_gained: self.information.unwrap_or(std::f64::INFINITY),
                    comment: meta_a,
                }),
                None,
                Some(CSVEntry {
                    num: (num * 2.0 - 1.0).into(),
                    lights_total: self.check.as_lights(),
                    lights_known_before: None,
                    bits_left: (self.left_after.context("total_left unset")? as f64).log2(),
                    comment: meta_b,
                }),
            )),
        }
    }

    pub fn md_title(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, comment, .. } => format!(
                "MN#{:02.1} {}",
                num,
                comment.split("--").collect::<Vec<_>>()[0]
            ),
            ConstraintType::Box { num, comment, .. } => format!(
                "MB#{:02.1} {}",
                num,
                comment.split("--").collect::<Vec<_>>()[0]
            ),
        }
    }

    pub fn print_hdr(&self, past_constraints: &Vec<&Constraint>) -> Result<()> {
        match &self.r#type {
            ConstraintType::Night { num, comment, .. } => print!("MN#{:02.1} {}", num, comment),
            ConstraintType::Box { num, comment, .. } => print!("MB#{:02.1} {}", num, comment),
        }
        println!();

        let mut tab = Table::new();
        tab
            .force_no_tty()
            .enforce_styling()
            .load_preset(NOTHING)
            .set_style(comfy_table::TableComponent::VerticalLines, '\u{2192}')
            // .set_style(comfy_table::TableComponent::VerticalLines, '\u{21D2}')
            // .set_style(comfy_table::TableComponent::VerticalLines, '\u{21E8}')
            // .set_style(comfy_table::TableComponent::VerticalLines, '\u{21FE}')
        ;
        let mut rows = vec![Row::new(); self.map_s.len()];
        for (i, (k, v)) in self.map_s.iter().enumerate() {
            if self.show_past_cnt() {
                let cnt = past_constraints
                    .iter()
                    .filter(|&c| c.show_past_cnt() && c.map_s.get(k).is_some_and(|v2| v2 == v))
                    .count();
                rows[i].add_cell(format!("{}x {}", cnt, k).into());
                rows[i].add_cell(v.into());
                // println!("{}x {} -> {}", cnt, k, v);
            } else {
                rows[i].add_cell(k.into());
                rows[i].add_cell(v.into());
                // println!("{} -> {}", k, v);
            }
        }
        tab.add_rows(rows);
        tab.column_mut(0)
            .context("no 0th column in table found")?
            .set_padding((0, 1));
        println!("{tab}");

        println!("---");
        match &self.check {
            CheckType::Eq => print!("Eq "),
            CheckType::Nothing => print!("Nothing "),
            CheckType::Lights(l, ls) => {
                let total = ls.values().sum::<u128>() as f64;
                // information theory
                if self.show_lights_information() {
                    println!(
                        "-> I[l]/bits: {{{}}}",
                        ls.iter()
                            .map(|(l, c)| {
                                let mut i = -(*c as f64 / total).log2();
                                if i == -0.0 {
                                    i = 0.0;
                                }
                                format!("{}: {:.2}", l, i)
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if self.show_expected_information() {
                    let mut expected: f64 = ls
                        .iter()
                        .map(|(_, c)| {
                            let p = *c as f64 / total;
                            p * p.log2()
                        })
                        .sum();
                    if expected == 0.0 {
                        expected = -0.0;
                    }
                    println!("-> E[I]/bits: {:.2}", -expected);
                }

                print!("{} lights ", l);
            }
        }

        println!(
            "=> I = {} bits",
            format!("{:.4}", self.information.unwrap_or(std::f64::INFINITY))
                .trim_end_matches('0')
                .trim_end_matches('.')
        );
        Ok(())
    }

    pub fn show_rem_table(&self) -> bool {
        return !self.result_unknown;
    }
}

// getter functions
impl Constraint {
    pub fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { num: _, comment } => &comment,
            ConstraintType::Box { num: _, comment } => &comment,
        }
    }

    pub fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, comment: _ } => format!("MN#{}", num),
            ConstraintType::Box { num, comment: _ } => format!("MB#{}", num),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_sort_maps_basic() {
        let mut constraint = Constraint {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            information: None,
            left_after: None,
            result_unknown: false,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map.insert(1, 0);
        constraint.map.insert(2, 3);

        constraint.map_s.insert("B".to_string(), "A".to_string());
        constraint.map_s.insert("C".to_string(), "D".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(
            vec![
                ("A".to_string(), 0),
                ("B".to_string(), 1),
                ("C".to_string(), 2),
                ("D".to_string(), 3),
            ]
            .into_iter(),
        );
        let lut_b = lut_a.clone();

        // Perform sorting
        constraint.sort_maps(&lut_a, &lut_b);

        // Validate the map is sorted and flipped correctly
        assert_eq!(*constraint.map.get(&1).unwrap(), 0);
        assert_eq!(*constraint.map.get(&3).unwrap(), 2);

        // Validate map_s is sorted and flipped correctly according to the LUTs
        assert_eq!(constraint.map_s.get("B").unwrap(), "A");
        assert_eq!(constraint.map_s.get("D").unwrap(), "C");
    }

    #[test]
    fn test_sort_maps_no_flipping_needed() {
        let mut constraint = Constraint {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            information: None,
            left_after: None,
            result_unknown: false,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map.insert(1, 0);
        constraint.map.insert(3, 2);

        constraint.map_s.insert("B".to_string(), "A".to_string());
        constraint.map_s.insert("D".to_string(), "C".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(
            vec![
                ("A".to_string(), 0),
                ("B".to_string(), 1),
                ("C".to_string(), 2),
                ("D".to_string(), 3),
            ]
            .into_iter(),
        );
        let lut_b = lut_a.clone();

        // Perform sorting
        constraint.sort_maps(&lut_a, &lut_b);

        // Validate the map is sorted and flipped correctly
        assert_eq!(*constraint.map.get(&1).unwrap(), 0);
        assert_eq!(*constraint.map.get(&3).unwrap(), 2);

        // Validate map_s is sorted and flipped correctly according to the LUTs
        assert_eq!(constraint.map_s.get("B").unwrap(), "A");
        assert_eq!(constraint.map_s.get("D").unwrap(), "C");
    }

    #[test]
    #[should_panic]
    fn test_sort_maps_panic_on_missing_lut_keys() {
        let mut constraint = Constraint {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            information: None,
            left_after: None,
            result_unknown: false,
        };

        constraint.map.insert(1, 0);
        // Initialize the map_s with keys not present in lut_a or lut_b
        constraint
            .map_s
            .insert("unknown".to_string(), "value".to_string());

        // Initialize lookup tables with different keys
        let lut_a = HashMap::new();
        let lut_b = HashMap::new();

        // Perform sorting (should panic due to missing keys)
        constraint.sort_maps(&lut_a, &lut_b);
    }

    #[test]
    fn test_finalize_parsing_night_lights() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Lights(3, BTreeMap::new()),
            hidden: false,
            no_exclude: false,
            result_unknown: false,
            exclude_s: None,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map_s.insert("A".to_string(), "B".to_string());
        constraint.map_s.insert("C".to_string(), "B".to_string());
        constraint.map_s.insert("D".to_string(), "B".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(
            vec![
                ("A".to_string(), 0),
                ("B".to_string(), 1),
                ("C".to_string(), 2),
                ("D".to_string(), 3),
            ]
            .into_iter(),
        );
        let lut_b = lut_a.clone();

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                3,
                &vec![],
                false,
                false,
                (&Default::default(), &Default::default()),
            )
            .unwrap();

        let map = HashMap::from_iter(vec![(0, 1), (2, 1), (3, 1)].into_iter());
        assert_eq!(map, constraint.map);
    }

    #[test]
    fn test_finalize_parsing_box_lights() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Lights(1, BTreeMap::new()),
            hidden: false,
            result_unknown: false,
            exclude_s: Some(("A".to_string(), vec!["C".to_string(), "D".to_string()])),
            no_exclude: false,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map_s.insert("A".to_string(), "B".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(
            vec![
                ("A".to_string(), 0),
                ("B".to_string(), 1),
                ("C".to_string(), 2),
                ("D".to_string(), 3),
            ]
            .into_iter(),
        );
        let lut_b = lut_a.clone();

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                20,
                &vec![],
                true,
                false,
                (&Default::default(), &Default::default()),
            )
            .unwrap();

        let map_s = HashMap::from_iter(vec![("A".to_string(), "B".to_string())].into_iter());
        assert_eq!(map_s, constraint.map_s);
        let map = HashMap::from_iter(vec![(0, 1)].into_iter());
        assert_eq!(map, constraint.map);
        let excl = Some((0, HashSet::from([2, 3])));
        assert_eq!(excl, constraint.exclude);
    }

    #[test]
    fn test_finalize_parsing_box_eq() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            result_unknown: false,
            exclude_s: None,
            no_exclude: false,
        };

        // Initialize the maps with unordered key/value pairs
        constraint.map_s.insert("A".to_string(), "B".to_string());

        // Initialize lookup tables
        let lut_a = HashMap::from_iter(
            vec![
                ("A".to_string(), 0),
                ("B".to_string(), 1),
                ("C".to_string(), 2),
                ("D".to_string(), 3),
            ]
            .into_iter(),
        );
        let lut_b = lut_a.clone();

        let constraint = constraint
            .finalize_parsing(
                &lut_a,
                &lut_b,
                20,
                &vec![],
                false,
                false,
                (&Default::default(), &Default::default()),
            )
            .unwrap();

        let map_s = HashMap::from_iter(vec![("A".to_string(), "B".to_string())].into_iter());
        assert_eq!(map_s, constraint.map_s);
        let map = HashMap::from_iter(vec![(0, 1)].into_iter());
        assert_eq!(map, constraint.map);
    }

    #[test]
    fn test_add_exclude() {
        let mut constraint = ConstraintParse {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Lights(1, BTreeMap::new()),
            hidden: false,
            exclude_s: None,
            no_exclude: false,
            result_unknown: false,
        };

        constraint.map_s.insert("A".to_string(), "b".to_string());

        // Initialize lookup tables
        let map_b = vec!["b".to_string(), "c".to_string(), "d".to_string()];

        let exclude_s = constraint.add_exclude(&map_b);

        assert_eq!(
            exclude_s.unwrap(),
            ("A".to_string(), vec!["c".to_string(), "d".to_string()])
        );
    }

    fn constraint_def() -> Constraint {
        Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
        }
    }

    #[test]
    fn test_process_light() {
        let mut c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
        };

        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(!c.process(&m).unwrap());
        // change amount of lights
        match &mut c.check {
            CheckType::Eq => {}
            CheckType::Nothing => {}
            CheckType::Lights(l, _) => *l = 1,
        }
        assert!(c.process(&m).unwrap());
    }

    #[test]
    fn test_process_light_exclude() {
        let mut c = Constraint {
            result_unknown: false,
            exclude: Some((0, HashSet::from([2, 3]))),
            map_s: HashMap::new(),
            check: CheckType::Lights(1, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from(""),
            },
        };

        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(c.process(&m).unwrap());

        let m: Matching = vec![vec![0], vec![1], vec![2, 3], vec![4]];
        assert!(!c.process(&m).unwrap());

        let m: Matching = vec![vec![0, 2], vec![1], vec![4], vec![3]];
        assert!(!c.process(&m).unwrap());
    }

    #[test]
    fn test_process_eq() {
        let mut c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Eq,
            // 1 and 2 have the same match
            map: HashMap::from([(0, 1), (1, 2)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            information: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from(""),
            },
        };

        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(!c.process(&m).unwrap());
        let m: Matching = vec![vec![0], vec![1, 2], vec![3], vec![4]];
        assert!(c.process(&m).unwrap());
    }

    #[test]
    fn test_eliminate() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];

        c.eliminate(&m);
        assert_eq!(c.eliminated, 1);
        assert_eq!(
            c.eliminated_tab,
            vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 0, 0],
                vec![0, 0, 1, 0, 0],
                vec![0, 0, 0, 1, 1]
            ]
        );

        c.eliminate(&m);
        assert_eq!(c.eliminated, 2);
        assert_eq!(
            c.eliminated_tab,
            vec![
                vec![2, 0, 0, 0, 0],
                vec![0, 2, 0, 0, 0],
                vec![0, 0, 2, 0, 0],
                vec![0, 0, 0, 2, 2]
            ]
        );
    }

    #[test]
    fn test_apply() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];

        c.eliminate(&m);
        assert_eq!(c.eliminated, 1);

        let mut rem: Rem = (vec![vec![15; 5]; 4], 5 * 4 * 3 * 2 * 1 * 4 / 2);

        rem = c.apply_to_rem(rem).unwrap();
        assert_eq!(rem.1, 5 * 4 * 3 * 2 * 1 * 4 / 2 - 1);
        assert_eq!(
            rem.0,
            vec![
                vec![14, 15, 15, 15, 15],
                vec![15, 14, 15, 15, 15],
                vec![15, 15, 14, 15, 15],
                vec![15, 15, 15, 14, 14]
            ]
        );
    }

    #[test]
    fn test_merge() {
        let mut c_a = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 200,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(4.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
        };
        let c_b = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::new(),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
        };

        c_a.merge(&c_b).unwrap();

        assert_eq!(c_a.eliminated, 300);

        assert_eq!(
            c_a.eliminated_tab,
            vec![
                vec![2, 0, 0, 0, 0],
                vec![0, 2, 0, 6, 0],
                vec![0, 0, 4, 0, 6],
                vec![0, 12, 0, 10, 0],
            ]
        );

        assert_eq!(c_a.information, None);
        assert_eq!(c_a.left_after, None);
    }

    #[test]
    fn test_stat_row() {
        let c = Constraint {
            result_unknown: false,
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
        };

        let row = c.stat_row(
            false,
            &vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
            ],
            &Vec::default(),
        );
        let row = row.iter().map(|x| x.content()).collect::<Vec<_>>();
        assert_eq!(
            row,
            vec!["MN#1.0", "2", "b*", "c*", "a*", "d*", "", "", "3.5", "4", ""]
        );

        let row = c.stat_row(
            false,
            &vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
            ],
            &vec![&c],
        );
        let row = row.iter().map(|x| x.content()).collect::<Vec<_>>();
        assert_eq!(
            row,
            vec!["MN#1.0", "2", "b", "c", "a", "d", "", "", "3.5", "0", "0/MN#1"]
        );
    }

    #[test]
    fn test_stat_row_box_eq() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Eq,
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from(""),
            },
            result_unknown: false,
        };

        let row = c.stat_row(
            false,
            &vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
            ],
            &Vec::default(),
        );
        let row = row.iter().map(|x| x.content()).collect::<Vec<_>>();
        assert_eq!(
            row,
            vec!["MB#1.0", "E", "b", "c", "a", "d", "", "", "3.5", "", ""]
        );
    }

    // #[test]
    // fn test_print_hdr() {
    //     let c = Constraint {
    //         exclude: None,
    //         exclude_s: None,
    //         no_exclude: false,
    //         map_s: HashMap::from([("A".to_string(), "b".to_string()), ("B".to_string(), "c".to_string()), ("C".to_string(), "a".to_string()), ("D".to_string(), "d".to_string())]),
    //         check: CheckType::Lights(2, BTreeMap::new()),
    //         map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
    //         eliminated: 100,
    //         eliminated_tab: vec![
    //             vec![1, 0, 0, 0, 0],
    //             vec![0, 1, 0, 3, 0],
    //             vec![0, 0, 2, 0, 3],
    //             vec![0, 6, 0, 5, 0],
    //         ],
    //         entropy: Some(3.5),
    //         left_after: None,
    //         hidden: false,
    //         r#type: ConstraintType::Night {
    //             num: 1.0,
    //             comment: String::from(""),
    //         },
    //     };
    //
    //     let row = c.print_hdr();
    // }

    // #[test]
    // fn test_write_stats() {
    //     let c = Constraint {
    //         exclude: None,
    //         exclude_s: None,
    //         no_exclude: false,
    //         map_s: HashMap::from([("A".to_string(), "b".to_string()), ("B".to_string(), "c".to_string()), ("C".to_string(), "a".to_string()), ("D".to_string(), "d".to_string())]),
    //         check: CheckType::Lights(2, BTreeMap::new()),
    //         map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
    //         eliminated: 100,
    //         eliminated_tab: vec![
    //             vec![1, 0, 0, 0, 0],
    //             vec![0, 1, 0, 3, 0],
    //             vec![0, 0, 2, 0, 3],
    //             vec![0, 6, 0, 5, 0],
    //         ],
    //         entropy: Some(3.5),
    //         left_after: None,
    //         hidden: false,
    //         r#type: ConstraintType::Night {
    //             num: 1.0,
    //             comment: String::from(""),
    //         },
    //     };
    //
    //     let row = c.write_stats();
    // }

    // show_expected_lights
    // show_pr_lights
    // comment
    // type_str

    #[test]
    fn test_comment_night() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from("comment"),
            },
            result_unknown: false,
        };

        assert_eq!(c.comment(), "comment");
    }

    #[test]
    fn test_comment_box() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from("comment"),
            },
            result_unknown: false,
        };

        assert_eq!(c.comment(), "comment");
    }

    #[test]
    fn test_type_str_night() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from("comment"),
            },
            result_unknown: false,
        };

        assert_eq!(c.type_str(), "MN#1");
    }

    #[test]
    fn test_type_str_box() {
        let c = Constraint {
            exclude: None,
            map_s: HashMap::from([
                ("A".to_string(), "b".to_string()),
                ("B".to_string(), "c".to_string()),
                ("C".to_string(), "a".to_string()),
                ("D".to_string(), "d".to_string()),
            ]),
            check: CheckType::Lights(2, BTreeMap::new()),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 100,
            eliminated_tab: vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 3, 0],
                vec![0, 0, 2, 0, 3],
                vec![0, 6, 0, 5, 0],
            ],
            information: Some(3.5),
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: String::from("comment"),
            },
            result_unknown: false,
        };

        assert_eq!(c.type_str(), "MB#1");
    }
}

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;

use comfy_table::presets::NOTHING;
use comfy_table::{Cell, Row, Table};

use crate::MapS;

use crate::constraint::{CheckType, Constraint, ConstraintType};

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

impl Constraint {
    pub fn build_tree(&self, path: PathBuf, map_a: &[String], map_b: &[String]) -> Result<bool> {
        if !self.build_tree {
            return Ok(false);
        }

        let ordering = crate::tree::tree_ordering(&self.left_poss, map_a);
        crate::tree::dot_tree(
            &self.left_poss,
            &ordering,
            &(self.type_str() + " / " + self.comment()),
            &mut File::create(path)?,
            map_a,
            map_b,
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
                        Some(c) => Cell::new(String::from(v2)).fg(c),
                        None => Cell::new(String::from(v2)),
                    }
                }
            }
            None => Cell::new(String::from("")),
        }));
        ret.push(Cell::new(String::from("")));

        match &self.check {
            CheckType::Eq | CheckType::Lights(..) => ret.push(Cell::new(
                format!("{:6.4}", self.information.unwrap_or(f64::INFINITY))
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

        #[allow(clippy::useless_format)]
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
                    bits_gained: self.information.unwrap_or(f64::INFINITY),
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
                    bits_gained: self.information.unwrap_or(f64::INFINITY),
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
            format!("{:.4}", self.information.unwrap_or(f64::INFINITY))
                .trim_end_matches('0')
                .trim_end_matches('.')
        );
        Ok(())
    }

    pub fn show_rem_table(&self) -> bool {
        !self.result_unknown
    }
}

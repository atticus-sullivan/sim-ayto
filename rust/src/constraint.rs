use anyhow::{ensure, Context, Result};
use comfy_table::Cell;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;

use crate::{Lut, Map, MapS, Matching, Rem};

#[derive(Deserialize, Debug, Clone)]
enum CheckType {
    Eq,
    Lights(u8, #[serde(skip)] BTreeMap<u8, u128>),
}

#[derive(Deserialize, Debug, Clone)]
enum ConstraintType {
    Night { num: f32, comment: String },
    Box { num: f32, comment: String },
}

#[derive(Deserialize, Debug, Clone)]
pub struct Constraint {
    r#type: ConstraintType,
    #[serde(rename = "map")]
    map_s: MapS,
    check: CheckType,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default, rename = "noExclude")]
    no_exclude: bool,
    #[serde(rename = "exclude")]
    exclude_s: Option<(String, Vec<String>)>,

    #[serde(skip)]
    map: Map,
    #[serde(skip)]
    exclude: Option<(u8, HashSet<u8>)>,
    #[serde(skip)]
    eliminated: u128,
    #[serde(skip)]
    eliminated_tab: Vec<Vec<u128>>,

    #[serde(skip)]
    entropy: Option<f64>,
    #[serde(skip)]
    left_after: Option<u128>,
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
    pub fn sort_maps(&mut self, lut_a: &Lut, lut_b: &Lut) {
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

    /// Generates the exclude list for the constraint, by inserting the elements from `map_b`
    ///
    /// This function modifies the internal state of the `Constraint`. If exclusion is not needed (constraint type is no box or lights != 1), no exclude list is generated.
    ///
    /// # Arguments
    ///
    /// - `map_b`: A reference to a vector of strings (`Vec<String>`) from which exclusions will be drawn. The function will create a new exclusion vector by removing any elements from `map_b` that match the current value in `self.map_s`.
    pub fn add_exclude(&mut self, map_b: &Vec<String>) {
        if self.no_exclude {
            return;
        }
        if let CheckType::Lights(l, _) = self.check {
            if !(l == 1 && self.map_s.len() == 1 && self.exclude_s.is_none()) {
                return;
            }
            if let ConstraintType::Box { .. } = self.r#type {
                for (k, v) in &self.map_s {
                    let bs: Vec<String> = map_b
                        .iter()
                        .filter(|&i| i != v)
                        .map(|i| i.to_string())
                        .collect();
                    self.exclude_s = Some((k.to_string(), bs));
                }
            }
        }
    }

    /// Finalize the initialization phase by translating the names (strings) to ids, validating the stored data and initialize the internal state of the constraint.
    ///
    /// # Arguments
    ///
    /// - `lut_a`: Reference to the lookup table for set_a (the keys)
    /// - `lut_b`: Reference to the lookup table for set_b (the values)
    /// - `map_len`: How many elements are expected to occur in the matching night
    pub fn finalize_parsing(&mut self, lut_a: &Lut, lut_b: &Lut, map_len: usize) -> Result<()> {
        // check if map size is valid
        match self.r#type {
            ConstraintType::Night { .. } => {
                ensure!(
                    self.map_s.len() == map_len,
                    "Map in a night must contain exactly as many entries as set_a {} (was: {})",
                    map_len,
                    self.map_s.len()
                );
                ensure!(
                    self.exclude_s.is_none(),
                    "Exclude is not yet supported for nights"
                );
            }
            ConstraintType::Box { .. } => {
                ensure!(
                    self.map_s.len() == 1,
                    "Map in a box must contain exactly {} entry (was: {})",
                    1,
                    self.map_s.len()
                );
            }
        }

        // init eliminated table
        self.eliminated_tab.reserve_exact(lut_a.len());
        for _ in 0..lut_a.len() {
            self.eliminated_tab.push(vec![0; lut_b.len()])
        }

        // translate names to ids
        self.map.reserve(self.map_s.len());
        for (k, v) in &self.map_s {
            self.map.insert(
                *lut_a.get(k).with_context(|| format!("Invalid Key {}", k))? as u8,
                *lut_b
                    .get(v)
                    .with_context(|| format!("Invalid Value {}", v))? as u8,
            );
        }

        // translate names to ids
        if let Some(ex) = &self.exclude_s {
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
            self.exclude = Some((a, bs));
        }

        Ok(())
    }
}

// internal helper functions
impl Constraint {
    fn show_expected_lights(&self) -> bool {
        match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => false,
        }
    }

    fn show_pr_lights(&self) -> bool {
        match &self.r#type {
            ConstraintType::Night { .. } => true,
            ConstraintType::Box { .. } => true,
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
        })?;
        if !fits {
            self.eliminate(m);
        }

        Ok(fits)
    }
}

// functions for evaluation
impl Constraint {
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
        self.entropy = None;
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
        self.entropy = if tmp > 0.0 { Some(-tmp.log2()) } else { None };

        Some(rem)
    }

    pub fn stat_row(&self, map_a: &[String]) -> Vec<Cell> {
        let mut ret = vec![];
        match self.r#type {
            ConstraintType::Night { num, .. } => ret.push(Cell::new(format!("MN#{:02.1}", num))),
            ConstraintType::Box { num, .. } => ret.push(Cell::new(format!("MB#{:02.1}", num))),
        }
        match &self.check {
            CheckType::Eq => ret.push(Cell::new("E")),
            CheckType::Lights(lights, _) => ret.push(Cell::new(lights)),
        }
        ret.extend(
            map_a
                .iter()
                .map(|a| Cell::new(self.map_s.get(a).unwrap_or(&String::from("")))),
        );
        ret.push(Cell::new(String::from("")));
        ret.push(Cell::new(self.entropy.unwrap_or(std::f64::INFINITY)));

        ret
    }

    pub fn write_stats(&self, mbo: &mut File, mno: &mut File, info: &mut File) -> Result<()> {
        if self.hidden {
            return Ok(());
        }

        match self.r#type {
            ConstraintType::Night { num, .. } => {
                writeln!(
                    info,
                    "{} {}",
                    num * 2.0,
                    (self.left_after.context("total_left unset")? as f64).log2()
                )?;
                writeln!(
                    mno,
                    "{} {}",
                    num,
                    self.entropy.unwrap_or(std::f64::INFINITY)
                )?;
            }
            ConstraintType::Box { num, .. } => {
                writeln!(
                    info,
                    "{} {}",
                    num * 2.0 - 1.0,
                    (self.left_after.context("total_left unset")? as f64).log2()
                )?;
                writeln!(
                    mbo,
                    "{} {}",
                    num,
                    self.entropy.unwrap_or(std::f64::INFINITY)
                )?;
            }
        }
        Ok(())
    }

    pub fn print_hdr(&self) {
        match &self.r#type {
            ConstraintType::Night { num, comment, .. } => print!("MN#{:02.1} {}", num, comment),
            ConstraintType::Box { num, comment, .. } => print!("MB#{:02.1} {}", num, comment),
        }
        println!();

        for (k, v) in &self.map_s {
            println!("{} -> {}", k, v);
        }

        println!("---");
        match &self.check {
            CheckType::Eq => print!("Eq "),
            CheckType::Lights(l, ls) => {
                let total = ls.values().sum::<u128>() as f64;
                if self.show_pr_lights() {
                    println!(
                        "-> Pr[lights]: {{{}}}",
                        ls.iter()
                            .map(|(l, c)| format!("{}: {:.1}%", l, (c * 100) as f64 / total))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if self.show_expected_lights() {
                    let expected: f64 = ls.iter().map(|(l, c)| *l as f64 * *c as f64 / total).sum();
                    println!("->  E[lights]: {:.3}", expected);
                }
                print!("{} lights ", l);
            }
        }

        println!(
            "=> I = {:.4} bits",
            self.entropy.unwrap_or(std::f64::INFINITY)
        );
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
            no_exclude: false,
            exclude_s: None,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            entropy: None,
            left_after: None,
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
            no_exclude: false,
            exclude_s: None,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            entropy: None,
            left_after: None,
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
            no_exclude: false,
            exclude_s: None,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            entropy: None,
            left_after: None,
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
    fn test_finalize_parsing() {
        let mut constraint = Constraint {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Eq,
            hidden: false,
            no_exclude: false,
            exclude_s: None,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            entropy: None,
            left_after: None,
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

        constraint.finalize_parsing(&lut_a, &lut_b, 20).unwrap();

        let map_s = HashMap::from_iter(vec![("A".to_string(), "B".to_string())].into_iter());
        assert_eq!(map_s, constraint.map_s);
        let map = HashMap::from_iter(vec![(0, 1)].into_iter());
        assert_eq!(map, constraint.map);
    }

    #[test]
    fn test_add_exclude() {
        let mut constraint = Constraint {
            r#type: ConstraintType::Box {
                num: 1.0,
                comment: "".to_string(),
            },
            map_s: HashMap::new(),
            check: CheckType::Lights(1, BTreeMap::new()),
            hidden: false,
            no_exclude: false,
            exclude_s: None,
            map: HashMap::new(),
            exclude: None,
            eliminated: 0,
            eliminated_tab: vec![],
            entropy: None,
            left_after: None,
        };

        constraint.map_s.insert("A".to_string(), "b".to_string());

        // Initialize lookup tables
        let map_b = vec!["b".to_string(), "c".to_string(), "d".to_string()];

        constraint.add_exclude(&map_b);

        assert_eq!(
            constraint.exclude_s.unwrap(),
            ("A".to_string(), vec!["c".to_string(), "d".to_string()])
        );
    }

    // show_expected_lights
    // show_pr_lights
    // merge
    // stat_row
    // write_stats
    // print_hdr
    // comment
    // type_str

    fn constraint_def() -> Constraint {
        Constraint {
            exclude: None,
            exclude_s: None,
            no_exclude: false,
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
            entropy: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
        }
    }

    #[test]
    fn test_constraint_process() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(!c.process(&m).unwrap());
        match &mut c.check {
            CheckType::Eq => {}
            CheckType::Lights(l, _) => *l = 1,
        }
        assert!(c.process(&m).unwrap());
    }

    #[test]
    fn test_constraint_eliminate() {
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
    fn test_constraint_apply() {
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
}

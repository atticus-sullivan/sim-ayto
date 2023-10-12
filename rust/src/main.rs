use anyhow::{anyhow, Context, Result};
use clap::Parser;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Cell, Color, Table,
};
use indicatif::ProgressBar;
use permutator::Permutation;
use serde::Deserialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use std::{collections::HashMap, fs::File};

// TODO cleanup (multiple files?)
// TODO code review (try with chatGPT)

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn constraint_def() -> Constraint {
        Constraint {
            map_s: HashMap::new(),
            check: CheckType::Lights(2),
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
    fn test_constraint_fits() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(!c.fits(&m).unwrap());
        match &mut c.check {
            CheckType::Eq => {}
            CheckType::Lights(l) => *l = 1,
        }
        assert!(c.fits(&m).unwrap());
    }

    #[test]
    fn test_constraint_eliminate() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];

        c.eliminate(&m).unwrap();
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

        c.eliminate(&m).unwrap();
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

        c.eliminate(&m).unwrap();
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
    fn test_factorial() {
        assert_eq!(factorial(3).unwrap(), 6);
        assert_eq!(factorial(4).unwrap(), 24);
    }
}

// TODO where to put this
// TODO write tests for this
fn add_dup<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
    add: u8,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        (0..perm.len()).map(move |idx| {
            let mut c = perm.clone();
            c[idx].push(add);
            c
        })
    })
}

// TODO where to put this
// TODO write tests for this
fn someone_is_dup<I: Iterator<Item = Vec<Vec<u8>>>>(vals: I) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        (0..perm.len() - 1).filter_map(move |idx| {
            if perm[idx][0] < perm[perm.len() - 1][0] {
                return None;
            }
            let mut c = perm.clone();
            c[idx].push(perm[perm.len() - 1][0]);
            c.pop();
            Some(c)
        })
    })
}

#[derive(Deserialize, Debug, Clone)]
enum CheckType {
    Eq,
    Lights(u8)
}

#[derive(Deserialize, Debug, Clone)]
enum ConstraintType {
    Night { num: f32, comment: String },
    Box { num: f32, comment: String },
}

type Matching = Vec<Vec<u8>>;
type MapS = HashMap<String, String>;
type Map = HashMap<u8, u8>;
type Lut = HashMap<String, usize>;

type Rem = (Vec<Vec<u128>>, u128);

#[derive(Deserialize, Debug, Clone)]
struct Constraint {
    r#type: ConstraintType,
    #[serde(rename = "map")]
    map_s: MapS,
    check: CheckType,
    #[serde(default)]
    hidden: bool,

    #[serde(skip)]
    map: Map,
    #[serde(skip)]
    eliminated: u128,
    #[serde(skip)]
    eliminated_tab: Vec<Vec<u128>>,

    #[serde(skip)]
    entropy: Option<f64>,
    #[serde(skip)]
    left_after: Option<u128>,
}

impl Constraint {
    fn finalize_parsing(&mut self, lut_a: &Lut, lut_b: &Lut) -> Result<()> {
        // check if map size is valid
        match self.r#type {
            ConstraintType::Night { .. } => {
                if self.map_s.len() != lut_a.len() {
                    return Err(anyhow!(
                        "Map in a night must contain exactly as many entries as set_a"
                    ));
                }
            }
            ConstraintType::Box { .. } => {
                // if self.map_s.len() != 1 {
                //     return Err(anyhow!("Map in a Box must exactly contain one entry"));
                // }
            }
        }

        self.eliminated_tab.reserve_exact(lut_a.len());
        for _ in 0..lut_a.len() {
            self.eliminated_tab.push(vec![0; lut_b.len()])
        }

        self.map.reserve(self.map_s.len());
        for (k, v) in &self.map_s {
            self.map.insert(
                *lut_a.get(k).context("Invalid Key")? as u8,
                *lut_b.get(v).context("Invalid Value")? as u8,
            );
        }

        Ok(())
    }

    fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { num: _, comment } => &comment,
            ConstraintType::Box { num: _, comment } => &comment,
        }
    }

    fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, comment: _ } => format!("MN#{}", num),
            ConstraintType::Box { num, comment: _ } => format!("MB#{}", num),
        }
    }

    fn eliminate(&mut self, m: &Matching) -> Result<()> {
        for (i1, v) in m.iter().enumerate() {
            for &i2 in v {
                self.eliminated_tab[i1][i2 as usize] += 1
            }
        }
        self.eliminated += 1;
        Ok(())
    }

    fn fits(&self, m: &Matching) -> Result<bool> {
        match &self.check {
            CheckType::Eq => {
                Ok(m.iter().enumerate().all(|(_,js)| {
                    self.map.iter().map(|(_,i2)| js.contains(i2)).fold(None, |acc, b| {
                        match acc {
                            Some(a) => Some(a == b),
                            None => Some(b),
                        }
                    }).unwrap()
                }))
            }
            CheckType::Lights(lights) => {
                let mut l = 0;
                for (i1, i2) in self.map.iter() {
                    if m[*i1 as usize].contains(i2) {
                        if l >= *lights {
                            return Ok(false);
                        }
                        l += 1;
                    }
                }
                Ok(l == *lights)
            }
        }
    }

    fn merge(&mut self, other: &Self) -> Result<()> {
        self.eliminated += other.eliminated;
        if self.eliminated_tab.len() != other.eliminated_tab.len() {
            return Err(anyhow!("eliminated_tab lengths do not match"));
        }
        for (i, es) in self.eliminated_tab.iter_mut().enumerate() {
            if es.len() != other.eliminated_tab[i].len() {
                return Err(anyhow!("eliminated_tab lengths do not match"));
            }
            for (j, e) in es.iter_mut().enumerate() {
                *e += other.eliminated_tab[i][j];
            }
        }
        self.entropy = None;
        self.left_after = None;
        Ok(())
    }

    fn apply_to_rem(&mut self, mut rem: Rem) -> Option<Rem> {
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

    fn stat_row(&self, map_a: &[String]) -> Vec<Cell> {
        let mut ret = vec![];
        match self.r#type {
            ConstraintType::Night { num, .. } => ret.push(Cell::new(format!("MN#{:02.1}", num))),
            ConstraintType::Box { num, .. } => ret.push(Cell::new(format!("MB#{:02.1}", num))),
        }
        match &self.check {
            CheckType::Eq => ret.push(Cell::new("E")),
            CheckType::Lights(lights) => ret.push(Cell::new(lights)),
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

    fn write_stats(&self, mbo: &mut File, mno: &mut File, info: &mut File) -> Result<()> {
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

    fn print_hdr(&self) {
        match &self.check {
            CheckType::Eq => print!("Eq "),
            CheckType::Lights(l) => print!("{} ", l),
        }
        match &self.r#type {
            ConstraintType::Night { num, comment, .. } => print!("MN#{:02.1} {}", num, comment),
            ConstraintType::Box { num, comment, .. } => print!("MB#{:02.1} {}", num, comment),
        }
        println!();

        for (k, v) in &self.map_s {
            println!("{} -> {}", k, v);
        }

        println!("-> I = {}", self.entropy.unwrap_or(std::f64::INFINITY));
    }
}

pub fn factorial(n: usize) -> Result<usize> {
    (1..=n)
        .try_fold(1, usize::checked_mul)
        .context("factorial failed. Probably too large")
}

#[derive(Deserialize, Debug)]
enum RuleSet {
    SomeoneIsDup,
    FixedDup(String),
    Eq,
}

impl std::default::Default for RuleSet {
    fn default() -> Self {
        RuleSet::Eq
    }
}

impl RuleSet {
    fn get_perms<'a, I: 'a + Iterator<Item = Vec<Vec<u8>>>>(
        &self,
        perm: I,
        _lut_a: &Lut,
        lut_b: &Lut,
    ) -> Result<Box<dyn 'a + Iterator<Item = Vec<Vec<u8>>>>> {
        match self {
            RuleSet::SomeoneIsDup => Ok(Box::new(someone_is_dup(perm))),
            RuleSet::FixedDup(s) => Ok(Box::new(add_dup(
                perm,
                *lut_b.get(s).context("Invalid index")? as u8,
            ))),
            RuleSet::Eq => Ok(Box::new(perm)),
        }
    }

    fn get_perm_base(&self, size_map_a: usize, size_map_b: usize) -> Matching {
        match self {
            RuleSet::SomeoneIsDup => (0..size_map_b as u8).map(|i| vec![i]).collect(),
            RuleSet::FixedDup(_) => (0..size_map_a as u8).map(|i| vec![i]).collect(),
            RuleSet::Eq => (0..size_map_a as u8).map(|i| vec![i]).collect(),
        }
    }

    fn get_perms_amount(&self, size_map_a: usize, size_map_b: usize) -> Result<usize> {
        match self {
            RuleSet::SomeoneIsDup => Ok(factorial(size_map_b)? * size_map_a / 2),
            RuleSet::FixedDup(_) => Ok(factorial(size_map_a)? * size_map_a),
            RuleSet::Eq => Ok(factorial(size_map_a)?),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Game {
    constraints: Vec<Constraint>,
    rule_set: RuleSet,
    tree_gen: bool,
    tree_top: Option<String>,

    #[serde(rename = "setA")]
    map_a: Vec<String>,
    #[serde(rename = "setB")]
    map_b: Vec<String>,

    #[serde(skip)]
    dir: PathBuf,
    #[serde(skip)]
    stem: String,
    #[serde(skip)]
    lut_a: Lut,
    #[serde(skip)]
    lut_b: Lut,
}

impl Game {
    fn new_from_yaml(yaml_path: &Path, stem: &Path) -> Result<Game> {
        let mut g: Game = serde_yaml::from_reader(File::open(yaml_path)?)?;

        g.dir = stem
            .parent()
            .context("parent dir of stem not found")?
            .to_path_buf();
        g.stem = stem
            .file_stem()
            .context("No filename provided in stem")?
            .to_string_lossy()
            .into_owned();

        for (lut, map) in [(&mut g.lut_a, &g.map_a), (&mut g.lut_b, &g.map_b)] {
            for (index, name) in map.iter().enumerate() {
                lut.insert(name.clone(), index);
            }
        }

        for c in &mut g.constraints {
            c.finalize_parsing(&g.lut_a, &g.lut_b)?;
        }

        Ok(g)
    }

    fn sim(&mut self) -> Result<()> {
        let mut x: Matching = self
            .rule_set
            .get_perm_base(self.map_a.len(), self.map_b.len());
        let perm = x.permutation();
        let perm = self.rule_set.get_perms(perm, &self.lut_a, &self.lut_b)?;

        let perm_amount = self
            .rule_set
            .get_perms_amount(self.map_a.len(), self.map_b.len())?;
        let cnt_update = perm_amount / 20;
        let progress = ProgressBar::new(100);

        let mut each = 0;
        let mut total = 0;
        let mut eliminated = 0;
        let mut left_poss = vec![];

        if !self.tree_gen {
            for (i, p) in perm.enumerate() {
                if i % cnt_update == 0 {
                    progress.inc(5);
                }
                if p[0].contains(&0) {
                    each += 1;
                }
                total += 1;
                for c in &mut self.constraints {
                    if !c.fits(&p)? {
                        c.eliminate(&p)?;
                        eliminated += 1;
                        break;
                    }
                }
            }
        } else {
            for (i, p) in perm.enumerate() {
                if i % cnt_update == 0 {
                    progress.inc(5);
                }
                if p[0].contains(&0) {
                    each += 1;
                }
                total += 1;
                let mut left = true;
                for c in &mut self.constraints {
                    if !c.fits(&p)? {
                        left = false;
                        c.eliminate(&p)?;
                        eliminated += 1;
                        break;
                    }
                }
                if left {
                    left_poss.push(p); // is clone really neccecary here? (p should be a new copy on evry iteration anyhow)
                }
            }
        }

        let mut rem: Rem = (vec![vec![each; self.map_b.len()]; self.map_a.len()], total);
        self.print_rem(&rem).context("Error printing")?;
        println!();

        let mut constr = vec![];
        let mut to_merge = vec![]; // collect hidden constraints to merge them down
        for c in &self.constraints {
            if c.hidden {
                to_merge.push(c);
            } else {
                let mut c = c.clone();
                // merge down previous hidden constraints
                while !to_merge.is_empty() {
                    c.merge(to_merge.pop().unwrap())?;
                }
                rem = c.apply_to_rem(rem).context("Apply to rem failed")?;
                c.print_hdr();
                self.print_rem(&rem).context("Error printing")?;
                constr.push(c);
                println!();
            }
        }

        if self.tree_gen {
            let dot_path = self.dir.join(self.stem.clone()).with_extension("dot");
            let ordering = self.tree_ordering(&left_poss);
            self.dot_tree(
                &left_poss,
                &ordering,
                &(constr[constr.len() - 1].type_str() + " / " + constr[constr.len() - 1].comment()),
                &mut File::create(dot_path.clone())?,
            )?;

            let pdf_path = dot_path.with_extension("pdf");
            Command::new("dot")
                .args([
                    "-Tpdf",
                    "-o",
                    pdf_path.to_str().context("pdf_path failed")?,
                    dot_path.to_str().context("dot_path failed")?,
                ])
                .output()
                .expect("dot command failed");

            let png_path = dot_path.with_extension("png");
            Command::new("dot")
                .args([
                    "-Tpng",
                    "-o",
                    png_path.to_str().context("png_path failed")?,
                    dot_path.to_str().context("dot_path failed")?,
                ])
                .output()
                .expect("dot command failed");
        }

        let dot_path = self
            .dir
            .join(self.stem.clone() + "_tab")
            .with_extension("dot");
        self.write_rem_dot(
            &rem,
            &(constr[constr.len() - 1].type_str() + " / " + constr[constr.len() - 1].comment()),
            &mut File::create(dot_path.clone())?,
        )?;

        let pdf_path = dot_path.with_extension("pdf");
        Command::new("dot")
            .args([
                "-Tpdf",
                "-o",
                pdf_path.to_str().context("pdf_path failed")?,
                dot_path.to_str().context("dot_path failed")?,
            ])
            .output()
            .expect("dot command failed");

        let png_path = dot_path.with_extension("png");
        Command::new("dot")
            .args([
                "-Tpng",
                "-o",
                png_path.to_str().context("png_path failed")?,
                dot_path.to_str().context("dot_path failed")?,
            ])
            .output()
            .expect("dot command failed");
        // println!("dir: {:?} dot_path: {:?} png_path: {:?} pdf_path: {:?}", self.dir, dot_path, png_path, pdf_path);

        self.do_statistics(&constr)?;

        println!(
            "Total permutations: {}  Permutations left: {}  Initial combinations for each pair: {}",
            total,
            total - eliminated,
            each
        );
        Ok(())
    }

    fn do_statistics(&self, merged_constraints: &Vec<Constraint>) -> Result<()> {
        let out_mb_path = self
            .dir
            .join(self.stem.clone() + "_statMB")
            .with_extension("out");
        let out_mn_path = self
            .dir
            .join(self.stem.clone() + "_statMN")
            .with_extension("out");
        let out_info_path = self
            .dir
            .join(self.stem.clone() + "_statInfo")
            .with_extension("out");

        let (mut mbo, mut mno, mut info) = (
            File::create(out_mb_path)?,
            File::create(out_mn_path)?,
            File::create(out_info_path)?,
        );
        for c in merged_constraints {
            c.write_stats(&mut mbo, &mut mno, &mut info)?;
        }

        let mut hdr = vec![
            Cell::new(""),
            Cell::new("L").set_alignment(comfy_table::CellAlignment::Center),
        ];
        hdr.extend(
            self.map_a
                .iter()
                .map(|x| Cell::new(x).set_alignment(comfy_table::CellAlignment::Center)),
        );
        hdr.push(Cell::new("").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("I").set_alignment(comfy_table::CellAlignment::Center));

        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);

        for c in merged_constraints {
            table.add_row(c.stat_row(&self.map_a));
        }
        println!("{table}");

        Ok(())
    }

    fn write_rem_dot(&self, rem: &Rem, title: &str, writer: &mut File) -> Result<()> {
        writeln!(
            writer,
            "digraph structs {{ labelloc=\"b\"; label=\"Stand: {}\"; node[shape=plaintext] struct[label=<", title)?;
        writeln!(
            writer,
            "<table cellspacing=\"2\" border=\"0\" rows=\"*\" columns=\"*\">"
        )?;

        // header row
        writeln!(writer, "<tr>")?;
        writeln!(writer, "<td></td>")?; // first empty cell
        for h in &self.map_b {
            writeln!(writer, "<td><B>{h}</B></td>")?;
        }
        writeln!(writer, "</tr>")?;

        for (i, a) in self.map_a.iter().enumerate() {
            writeln!(writer, "<tr>")?;
            writeln!(writer, "<td><B>{a}</B></td>")?;

            let i = rem
                .0
                .get(i)
                .context("Indexing rem with map failed")?
                .iter()
                .map(|x| {
                    let val = (*x as f64) / (rem.1 as f64) * 100.0;
                    if 79.0 < val && val < 101.0 {
                        (val, "darkgreen")
                    } else if -1.0 < val && val < 1.0 {
                        (val, "red")
                    } else {
                        (val, "black")
                    }
                });
            for (v, font) in i {
                writeln!(
                    writer,
                    "<td><font color=\"{}\">{:03.4}</font></td>",
                    font, v
                )?;
            }
            writeln!(writer, "</tr>")?;
        }
        writeln!(writer, "</table>")?;
        writeln!(writer, ">];}}")?;

        Ok(())
    }

    fn print_rem(&self, rem: &Rem) -> Option<()> {
        let mut hdr = vec![Cell::new("")];
        hdr.extend(
            self.map_b
                .iter()
                .map(|x| Cell::new(x).set_alignment(comfy_table::CellAlignment::Center)),
        );
        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);
        for (i, a) in self.map_a.iter().enumerate() {
            let i = rem.0.get(i)?.iter().map(|x| {
                let val = (*x as f64) / (rem.1 as f64) * 100.0;
                if 79.0 < val && val < 101.0 {
                    Cell::new(format!("{:02.3}", val)).fg(Color::Green)
                } else if -1.0 < val && val < 1.0 {
                    Cell::new(format!("{:02.3}", val)).fg(Color::Red)
                } else {
                    Cell::new(format!("{:02.3}", val))
                }
            });
            let mut row = vec![Cell::new(a)];
            row.extend(i);
            table.add_row(row);
        }
        println!("{table}");
        println!("{} left -> {} bits left", rem.1, (rem.1 as f64).log2());
        Some(())
    }

    fn dot_tree(
        &self,
        data: &Vec<Matching>,
        ordering: &Vec<(usize, usize)>,
        title: &str,
        writer: &mut File,
    ) -> Result<()> {
        let mut nodes: HashSet<String> = HashSet::new();
        writeln!(
            writer,
            "digraph D {{ labelloc=\"b\"; label=\"Stand: {}\"; ranksep=0.8;",
            title
        )?;
        for p in data {
            let mut parent = String::from("root");
            for (i, _) in ordering {
                let mut node = parent.clone();
                node.push('/');
                node.push_str(
                    &p[*i]
                        .iter()
                        .map(|b| b.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );

                if nodes.insert(node.clone()) {
                    writeln!(
                        writer,
                        "\"{node}\"[label=\"{}\"]",
                        self.map_a[*i].clone()
                            + "\\n"
                            + &p[*i]
                                .iter()
                                .map(|b| self.map_b[*b as usize].clone())
                                .collect::<Vec<_>>()
                                .join("\\n")
                    )?;
                    writeln!(writer, "\"{parent}\" -> \"{node}\";")?;
                }

                parent = node;
            }
        }
        writeln!(writer, "}}")?;
        Ok(())
    }

    fn tree_ordering(&self, data: &Vec<Matching>) -> Vec<(usize, usize)> {
        let mut tab = vec![HashSet::new(); self.map_a.len()];
        for p in data {
            for (i, js) in p.iter().enumerate() {
                tab[i].insert(js);
            }
        }

        let mut ordering: Vec<_> = tab.iter().enumerate().map(|(i, x)| (i, x.len())).collect();
        match &self.tree_top {
            Some(ts) => {
                let t = self.lut_a[ts];
                ordering.sort_unstable_by_key(|(i, x)| {
                    // x values will always be positive, 1 will be the minimum / value for already
                    // fixed matches
                    // with (x-1)*2 we move that minimum to 0 and spread the values.
                    // In effect the value 1 will be unused. To sort the specified tree_top right
                    // below the already fixed matches this level is mapped to the value 1
                    // Why so complicated? To avoid using floats here, while still ensuring the
                    // order as specified.
                    if *i == t {
                        1
                    } else {
                        ((*x) - 1) * 2
                    }
                })
            }
            None => {
                ordering.sort_unstable_by_key(|(_, x)| *x);
            }
        }
        ordering
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The path to the file to read
    yaml_path: PathBuf,

    #[arg(short = 'c', long = "color")]
    colored: bool,

    #[arg(short = 'o', long = "output")]
    stem: PathBuf,
}

fn main() {
    let args = Cli::parse();
    let mut g = Game::new_from_yaml(&args.yaml_path, &args.stem).expect("Parsing failed");

    let start = Instant::now();
    g.sim().unwrap();
    println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
}

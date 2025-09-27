use crate::game::foreach_unwrapped_matching;
use crate::game::IterState;
use crate::DumpMode;
use crate::Game;

use comfy_table::presets::NOTHING;
use comfy_table::{Row, Table};

use std::fs::File;

use anyhow::{Context, Result};

use crate::constraint::Constraint;
use crate::Rem;

impl Game {
    pub fn sim(
        &mut self,
        print_transposed: bool,
        dump_mode: Option<DumpMode>,
        full: bool,
        use_cache: &Option<String>,
    ) -> Result<()> {
        let input_file = if use_cache.is_some() {
            &self.cache_file
        } else {
            &None
        };

        let perm_amount =
            self.rule_set
                .get_perms_amount(self.map_a.len(), self.map_b.len(), input_file)?;

        let mut is = IterState::new(
            dump_mode.is_some(),
            perm_amount,
            self.constraints_orig.clone(),
            &self.query_matchings,
            if use_cache.is_some() {
                &self.final_cache_hash
            } else {
                &None
            },
            (self.map_a.len(), self.map_b.len()),
        )?;
        self.rule_set
            .iter_perms(&self.lut_a, &self.lut_b, &mut is, true, input_file)?;

        // fix is so that it can't be mutated anymore
        let is = &is;

        // track table indices
        let mut tab_idx = 0;
        let mut md_tables: Vec<(String, u16, bool, bool)> = vec![];

        // generate additional tables
        if is.query_matchings.iter().any(|(_, x)| x.is_some()) {
            println!("Trace at which point a particular matching was elimiated:");
            for (q, id) in &is.query_matchings {
                if let Some(id) = id {
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
                    let mut rows = vec![Row::new(); q.len()];
                    for (a, b) in q.iter().enumerate() {
                        let ass = self.map_a.get(a).unwrap();
                        let bs = b
                            .iter()
                            .map(|b| self.map_b.get(*b as usize).unwrap())
                            .collect::<Vec<_>>();
                        rows[a].add_cell(ass.into());
                        rows[a].add_cell(format!("{:?}", bs).into());
                    }
                    tab.add_rows(rows);
                    tab.column_mut(0)
                        .context("no 0th column in table found")?
                        .set_padding((0, 1));
                    println!("{tab}");
                    println!("=> Eliminated in {}", id);
                    tab_idx += 1;
                }
            }
            println!();
        }

        let mut rem: Rem = (is.each.clone(), is.total);
        if print_transposed {
            self.print_rem_generic(&rem, &self.map_b, &self.map_a, |v, h| (h, v))
                .context("Error printing")?;
        } else {
            self.print_rem_generic(&rem, &self.map_a, &self.map_b, |v, h| (v, h))
                .context("Error printing")?;
        }
        md_tables.push(("tab-start".to_owned(), tab_idx, false, false));
        tab_idx += 1;
        println!();

        let mut constr = vec![];
        let mut to_merge = vec![]; // collect hidden constraints to merge them down
        let mut past_constraints: Vec<&Constraint> = Vec::default();
        for c_ in &is.constraints {
            if c_.should_merge() {
                to_merge.push(c_);
            } else {
                let mut c = c_.clone();
                // merge down previous hidden constraints
                while !to_merge.is_empty() {
                    c.merge(to_merge.pop().unwrap())?;
                }
                rem = c.apply_to_rem(rem).context("Apply to rem failed")?;
                c.print_hdr(&past_constraints)?;
                if c.show_rem_table() {
                    let tree = c.build_tree(
                        self.dir
                            .join(format!("{}_{}_tree", self.stem, tab_idx))
                            .with_extension("dot"),
                        &self.map_a,
                        &self.map_b,
                    )?;
                    if print_transposed {
                        self.print_rem_generic(&rem, &self.map_b, &self.map_a, |v, h| (h, v))
                            .context("Error printing")?;
                        c.ruleset_data.print(
                            full,
                            &self.rule_set,
                            &self.map_a,
                            &self.map_b,
                            &self.lut_b,
                            rem.1,
                        )?;
                        md_tables.push((c.md_title(), tab_idx, tree, true));
                        tab_idx += 1;
                    } else {
                        self.print_rem_generic(&rem, &self.map_a, &self.map_b, |v, h| (v, h))
                            .context("Error printing")?;
                        c.ruleset_data.print(
                            full,
                            &self.rule_set,
                            &self.map_a,
                            &self.map_b,
                            &self.lut_b,
                            rem.1,
                        )?;
                        md_tables.push((c.md_title(), tab_idx, tree, true));
                        tab_idx += 1;
                    }
                }

                past_constraints.push(c_);
                println!();
                constr.push(c);
            }
        }

        let md_path = self.dir.join(self.stem.clone()).with_extension("md");
        self.md_output(&mut File::create(md_path.clone())?, &md_tables)?;

        if let Some(d) = dump_mode {
            match d {
                DumpMode::Full => {
                    for p in is.left_poss.iter() {
                        println!("{:?}", p.iter().enumerate().collect::<Vec<_>>())
                    }
                }
                DumpMode::FullNames => {
                    for p in is.left_poss.iter() {
                        println!(
                            "{:?}",
                            p.iter()
                                .enumerate()
                                .map(|(a, bs)| (
                                    &self.map_a[a],
                                    bs.iter()
                                        .map(|b| &self.map_b[*b as usize])
                                        .collect::<Vec<_>>()
                                ))
                                .collect::<Vec<_>>()
                        )
                    }
                }
                DumpMode::Winning => {
                    for p in is.left_poss.iter() {
                        foreach_unwrapped_matching(p, |m| println!("{:?}", m));
                    }
                }
                DumpMode::WinningNames => {
                    for p in is.left_poss.iter() {
                        foreach_unwrapped_matching(p, |m| {
                            println!(
                                "{:?}",
                                m.into_iter()
                                    .enumerate()
                                    .map(|(a, b)| (&self.map_a[a], &self.map_b[*b as usize]))
                                    .collect::<Vec<_>>()
                            )
                        });
                    }
                }
            }
        }

        self.do_statistics(is.total as f64, &constr)?;

        println!(
            "Total permutations: {}  Permutations left: {}  Initial combinations for each pair: {}",
            is.total,
            is.total - is.eliminated,
            is.each[0][0]
        );
        Ok(())
    }
}

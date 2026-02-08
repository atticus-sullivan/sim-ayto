use std::fs::File;

use anyhow::Result;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};

use std::io::Write;

use crate::Rem;

use crate::game::Game;
use crate::{COLOR_ALT_BG, COLOR_COL_MAX, COLOR_ROW_MAX, COLOR_BOTH_MAX};

impl Game {
    pub fn write_page_md(&self, out: &mut File, md_tables: &[(String, u16, bool, bool)]) -> Result<()> {
        writeln!(out, "---")?;
        writeln!(out, "{}", serde_yaml::to_string(&self.frontmatter)?)?;
        writeln!(out, "---")?;

        let stem = &self.stem;

        writeln!(out, "\n{{{{% translateHdr \"tab-current\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
        writeln!(out, "{{{{% details closed=\"true\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/{stem}/{stem}_tab.png\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/{stem}/{stem}_sum.png\" %}}}}")?;
        writeln!(out, "{{{{% /details %}}}}")?;

        writeln!(out, "\n{{{{% translateHdr \"tab-individual\" %}}}}")?;
        for (name, idx, tree, detail) in md_tables.iter() {
            if *detail {
                writeln!(
                    out,
                    "\n{{{{% details title=\"{name}\" closed=\"true\" %}}}}"
                )?;
            } else {
                writeln!(out, "\n{{{{% translatedDetails \"{name}\" %}}}}")?;
            }

            writeln!(out, "{{{{% img src=\"/{stem}/{stem}_{idx}.png\" %}}}}")?;
            if *tree {
                writeln!(out, "{{{{% img src=\"/{stem}/{stem}_{idx}_tree.png\" %}}}}")?;
            }

            if *detail {
                writeln!(out, "{{{{% /details %}}}}")?;
            } else {
                writeln!(out, "{{{{% /translatedDetails %}}}}")?;
            }
        }

        writeln!(out, "\n{{{{% translateHdr \"tab-everything\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
        writeln!(out, "{{{{% details closed=\"true\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/{stem}/{stem}.col.png\" %}}}}")?;
        writeln!(out, "{{{{% /details %}}}}")?;

        Ok(())
    }

    pub fn print_rem_generic(
        &self,
        rem: &Rem,
        map_vert: &[String],
        map_hor: &[String],
        norm_idx: fn(v: usize, h: usize) -> (usize, usize),
    ) -> Result<()> {
        let table_content = map_vert
            .iter()
            .enumerate()
            .map(|(vert_idx, vert_name)| {
                (
                    vert_name,
                    map_hor
                        .iter()
                        .enumerate()
                        .map(|(hor_idx, _)| {
                            let (vert_idx, hor_idx) = norm_idx(vert_idx, hor_idx);
                            if self.rule_set.ignore_pairing(vert_idx, hor_idx) {
                                None // ignore/empty
                            } else {
                                // calculate remaining percentage
                                let x = rem.0[vert_idx][hor_idx];
                                let val = (x as f64) / (rem.1 as f64) * 100.0;
                                Some(val)
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        // mapping of y-coord to indices with maximum
        let hor_max = table_content
            .iter()
            .map(|(_, vals)| {
                vals.iter()
                    .enumerate()
                    // iterator over the values in current row (skips the None entries)
                    .filter_map(|(col_idx, value)| value.map(|value| (col_idx, value)))
                    // find all indices containing the maximum
                    .fold((vec![], 0.0), |mut acc, (col_idx, value)| {
                        if acc.1 < value {
                            acc.0 = vec![col_idx];
                            acc.1 = value;
                        } else if acc.1 == value {
                            acc.0.push(col_idx);
                        }
                        acc
                    })
            })
            .collect::<Vec<_>>();

        // mapping of x-coord to indices with maximum
        let vert_max = (0..table_content[0].1.len())
            // loop over columns
            .map(|col_idx| {
                table_content
                    .iter()
                    .enumerate()
                    // iterator over the values in current column (skips the None entries)
                    .filter_map(|(row_idx, row)| row.1[col_idx].map(|value| (row_idx, value)))
                    // find all indices containing the maximum
                    .fold((vec![], 0.0), |mut acc, (row_idx, value)| {
                        if acc.1 < value {
                            acc.0 = vec![row_idx];
                            acc.1 = value;
                        } else if acc.1 == value {
                            acc.0.push(row_idx);
                        }
                        acc
                    })
            })
            .collect::<Vec<_>>();

        let mut table = Table::new();
        {
            let mut hdr = vec![Cell::new("")];
            hdr.extend(map_hor.iter().enumerate().map(|(i, x)| {
                if vert_max[i].1 == 100.0 {
                    Cell::new(x).fg(Color::Green)
                } else {
                    Cell::new(x)
                }
                .set_alignment(comfy_table::CellAlignment::Center)
                .bg(COLOR_COL_MAX)
            }));

            table
                .force_no_tty()
                .enforce_styling()
                .load_preset(UTF8_FULL_CONDENSED)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_header(hdr);
        }

        for (j, i) in table_content.iter().enumerate() {
            let mut row = vec![];
            row.push(
                if hor_max[j].1 == 100.0 {
                    Cell::new(i.0).fg(Color::Green)
                } else {
                    Cell::new(i.0)
                }
                .bg(COLOR_ROW_MAX),
            );

            // put formatted table content into the table
            row.extend(
                i.1.iter()
                    .enumerate()
                    // format according to value (sets the foreground)
                    .map(|(idx, val)| {
                        match val {
                            Some(val) => {
                                let val = *val;
                                if 79.0 < val && val < 101.0 {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    )
                                    .fg(Color::Green))
                                } else if 55.0 <= val {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    )
                                    .fg(Color::Cyan))
                                } else if 45.0 < val {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    )
                                    .fg(Color::Yellow))
                                } else if 1.0 < val {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    ))
                                } else if -1.0 < val {
                                    Ok(Cell::new(
                                        format!("{:6.3}", val)
                                            .trim_end_matches('0')
                                            .trim_end_matches('.'),
                                    )
                                    .fg(Color::Red))
                                } else {
                                    Ok(Cell::new(""))
                                }
                            }
                            None => Ok(Cell::new("")),
                        }
                        .map(|cell| {
                            // format according to row and maxima (uses background)
                            let max_h = hor_max[j].0.contains(&idx);
                            let max_v = vert_max[idx].0.contains(&j);
                            if max_h {
                                if max_v {
                                    // max for both
                                    cell.bg(COLOR_BOTH_MAX)
                                } else {
                                    // row max
                                    cell.bg(COLOR_ROW_MAX)
                                }
                            } else {
                                #[allow(clippy::collapsible_else_if)]
                                if max_v {
                                    // column max
                                    cell.bg(COLOR_COL_MAX)
                                } else {
                                    // neither
                                    if j % 2 == 0 {
                                        cell.bg(COLOR_ALT_BG)
                                    } else {
                                        cell
                                    }
                                }
                            }
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            );

            table.add_row(row);
        }

        println!("{table}");
        println!(
            "{} left -> {} bits left",
            rem.1,
            format!("{:.4}", (rem.1 as f64).log2())
                .trim_end_matches('0')
                .trim_end_matches('.')
        );
        Ok(())
    }
}

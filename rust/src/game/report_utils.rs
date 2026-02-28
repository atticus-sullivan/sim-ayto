//! This module provides all functionalities required for printing a table of the remaining
//! probabilities of a match.
//! For the outside the computation is split into two steps:
//! 1. generate the data: `print_rem_generic`
//! 2. print the data via the Display trait of the returned struct (`RemTable`)
//!
//! On the inside the computation is further split up into:
//! 1. Translating the remaining counts to percentages/probabilities
//! 2. Calculating row/col maxima
//! 3. Generating a styled (comfy)table

use std::fmt::Display;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, Table};

use crate::Rem;
use crate::{COLOR_ALT_BG, COLOR_BOTH_MAX, COLOR_COL_MAX, COLOR_ROW_MAX};

pub(super) struct RemTable {
    tab: Table,
    footer: String,
}

impl Display for RemTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.tab)?;
        write!(f, "{}", self.footer)?;
        Ok(())
    }
}

/// Render a generic remaining-percentage table (or transposed).
///
/// `norm_idx` maps the table coordinates used in `rem` to the visible (vert, hor)
/// ordering. The function highlights maxima with background colors and prints the table.
pub(super) fn print_rem_generic<F>(
    rem: &Rem,
    map_vert: &[String],
    map_hor: &[String],
    norm_idx: fn(usize, usize) -> (usize, usize),
    ignore_pairing: F,
) -> RemTable
where
    F: Fn(usize, usize) -> bool,
{
    let matrix = build_percentage_matrix(rem, map_vert, map_hor, norm_idx, ignore_pairing);
    let max = find_maxima(&matrix);
    let table = render_table(&max, map_hor, &matrix);

    RemTable {
        tab: table,
        footer: format!(
            "{} left -> {} bits left",
            rem.1,
            format!("{:.4}", (rem.1 as f64).log2())
                .trim_end_matches('0')
                .trim_end_matches('.')
        ),
    }
}

/// Converts the `Rem` to a matrix of percentages. Based on ignore_pairing, some entries might be
/// absent in this matrix.
/// In the returned value, each row is associated with its "header" (aka first column)
///
/// # Notes
/// - the dimensions of map_vert, map_hor and the matrix in rem need to fit (this is not explicitly
///   checked)
fn build_percentage_matrix<'a, F>(
    rem: &Rem,
    map_vert: &'a [String],
    map_hor: &[String],
    norm_idx: fn(v: usize, h: usize) -> (usize, usize),
    ignore_pairing: F,
) -> Vec<(&'a String, Vec<Option<f64>>)>
where
    F: Fn(usize, usize) -> bool,
{
    map_vert
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
                        if ignore_pairing(vert_idx, hor_idx) {
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
        .collect::<Vec<_>>()
}

// Small helper to find the maximum (potentially multiple ones) as well as the associated
// index/indices
#[derive(Debug, Clone, Default, PartialEq)]
struct TabMax<T: PartialOrd> {
    // multiple indices might contain the same maximum -> use a vector
    idxs: Vec<usize>,
    max: T,
}

impl<T: PartialOrd> TabMax<T> {
    fn update(mut self, idx: usize, val: T) -> Self {
        if self.max < val {
            self.idxs.clear();
            self.max = val;
            self.idxs.push(idx);
        } else if self.max == val {
            self.idxs.push(idx);
        }
        self
    }
}

// collects maximas in different directions to make the code more readable (no tuple indexing,
// descriptive words instead)
#[derive(Debug, PartialEq)]
struct TabFullMaxima<T: PartialOrd> {
    // (rowIdx -> maxima)
    hor: Vec<TabMax<T>>,
    // (colIdx -> maxima)
    vert: Vec<TabMax<T>>,
}

/// returns a mappings: row/col idx to TabMax
fn find_maxima(matrix: &[(&String, Vec<Option<f64>>)]) -> TabFullMaxima<f64> {
    // mapping of y-coord to indices with maximum
    let hor_max = matrix
        .iter()
        .map(|(_, vals)| {
            vals.iter()
                .enumerate()
                // iterator over the values in current row (skips the None entries)
                .filter_map(|(col_idx, value)| value.map(|value| (col_idx, value)))
                // find all indices containing the maximum
                .fold(TabMax::default(), |acc, (col_idx, value)| {
                    acc.update(col_idx, value)
                })
        })
        .collect::<Vec<_>>();

    // mapping of x-coord to indices with maximum
    let vert_max = (0..matrix[0].1.len())
        // loop over columns
        .map(|col_idx| {
            matrix
                .iter()
                .enumerate()
                // iterator over the values in current column (skips the None entries)
                .filter_map(|(row_idx, row)| row.1[col_idx].map(|value| (row_idx, value)))
                // find all indices containing the maximum
                .fold(TabMax::default(), |acc, (row_idx, value)| {
                    acc.update(row_idx, value)
                })
        })
        .collect::<Vec<_>>();

    TabFullMaxima {
        hor: hor_max,
        vert: vert_max,
    }
}

// turn a matrix of percentages to a comfy table.
// Style values according to their value and the hor/vert maxima
fn render_table(
    max: &TabFullMaxima<f64>,
    map_hor: &[String],
    matrix: &[(&String, Vec<Option<f64>>)],
) -> Table {
    let mut table = Table::new();
    {
        let mut hdr = vec![Cell::new("")];
        hdr.extend(map_hor.iter().enumerate().map(|(i, x)| {
            if max.vert[i].max == 100.0 {
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

    for (j, i) in matrix.iter().enumerate() {
        let mut row = vec![];
        row.push(
            if max.hor[j].max == 100.0 {
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
                    let cell = match val {
                        Some(val) => {
                            let val = *val;
                            if 79.0 < val && val < 101.0 {
                                Cell::new(
                                    format!("{:6.3}", val)
                                        .trim_end_matches('0')
                                        .trim_end_matches('.'),
                                )
                                .fg(Color::Green)
                            } else if 55.0 <= val {
                                Cell::new(
                                    format!("{:6.3}", val)
                                        .trim_end_matches('0')
                                        .trim_end_matches('.'),
                                )
                                .fg(Color::Cyan)
                            } else if 45.0 < val {
                                Cell::new(
                                    format!("{:6.3}", val)
                                        .trim_end_matches('0')
                                        .trim_end_matches('.'),
                                )
                                .fg(Color::Yellow)
                            } else if 1.0 < val {
                                Cell::new(
                                    format!("{:6.3}", val)
                                        .trim_end_matches('0')
                                        .trim_end_matches('.'),
                                )
                            } else if -1.0 < val {
                                Cell::new(
                                    format!("{:6.3}", val)
                                        .trim_end_matches('0')
                                        .trim_end_matches('.'),
                                )
                                .fg(Color::Red)
                            } else {
                                Cell::new("")
                            }
                        }
                        None => Cell::new(""),
                    };
                    // format according to row and maxima (uses background)
                    let max_h = max.hor[j].idxs.contains(&idx);
                    let max_v = max.vert[idx].idxs.contains(&j);
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
                .collect::<Vec<_>>(),
        );

        table.add_row(row);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // Stub normaliser - identity
    fn identity_norm(v: usize, h: usize) -> (usize, usize) {
        (v, h)
    }

    #[test]
    fn build_percentage_matrix_simple() {
        // 2x3 matrix:
        // [10, 20, 30]
        // [40, 50, 60]
        let rem = (vec![vec![10, 20, 30], vec![40, 50, 60]], 210);
        let map_vert = vec!["a", "b"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let map_hor = vec!["A", "B", "C"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();

        let matrix =
            build_percentage_matrix(&rem, &map_vert, &map_hor, identity_norm, |_, _| false);
        assert_eq!(
            matrix,
            vec![
                (
                    &"a".to_string(),
                    vec![
                        Some(10.0 / 210.0 * 100.0),
                        Some(20.0 / 210.0 * 100.0),
                        Some(30.0 / 210.0 * 100.0)
                    ]
                ),
                (
                    &"b".to_string(),
                    vec![
                        Some(40.0 / 210.0 * 100.0),
                        Some(50.0 / 210.0 * 100.0),
                        Some(60.0 / 210.0 * 100.0)
                    ]
                ),
            ]
        );
    }

    #[test]
    fn build_percentage_matrix_empty() {
        let rem = (vec![], 210);
        let map_vert = vec![];
        let map_hor = vec![];

        let matrix =
            build_percentage_matrix(&rem, &map_vert, &map_hor, identity_norm, |_, _| false);
        assert_eq!(matrix, vec![]);

        let rem = (vec![vec![], vec![]], 210);
        let map_vert = vec!["a", "b"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let map_hor = vec![];

        let matrix =
            build_percentage_matrix(&rem, &map_vert, &map_hor, identity_norm, |_, _| false);
        assert_eq!(
            matrix,
            vec![(&"a".to_string(), vec![]), (&"b".to_string(), vec![]),]
        );
    }

    #[test]
    fn build_percentage_matrix_ignore() {
        // 2x3 matrix:
        // [10, 20, 30]
        // [40, 50, 60]
        let rem = (vec![vec![10, 20, 30], vec![40, 50, 60]], 210);
        let map_vert = vec!["a", "b"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let map_hor = vec!["A", "B", "C"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();

        let matrix = build_percentage_matrix(&rem, &map_vert, &map_hor, identity_norm, |x, y| {
            x == 1 && y == 1
        });
        assert_eq!(
            matrix,
            vec![
                (
                    &"a".to_string(),
                    vec![
                        Some(10.0 / 210.0 * 100.0),
                        Some(20.0 / 210.0 * 100.0),
                        Some(30.0 / 210.0 * 100.0)
                    ]
                ),
                (
                    &"b".to_string(),
                    vec![Some(40.0 / 210.0 * 100.0), None, Some(60.0 / 210.0 * 100.0)]
                ),
            ]
        );

        // ignore all
        let matrix = build_percentage_matrix(&rem, &map_vert, &map_hor, identity_norm, |_, _| true);
        assert_eq!(
            matrix,
            vec![
                (&"a".to_string(), vec![None, None, None]),
                (&"b".to_string(), vec![None, None, None]),
            ]
        );
    }

    #[test]
    fn find_maxima_simple() {
        let map_vert = vec!["a", "b", "c"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let matrix = vec![
            (
                &map_vert[0],
                vec![
                    Some(10.0 / 210.0 * 100.0),
                    Some(20.0 / 210.0 * 100.0),
                    Some(30.0 / 210.0 * 100.0),
                ],
            ),
            (
                &map_vert[1],
                vec![
                    Some(60.0 / 210.0 * 100.0),
                    Some(50.0 / 210.0 * 100.0),
                    Some(60.0 / 210.0 * 100.0),
                ],
            ),
            (
                &map_vert[2],
                vec![
                    Some(70.0 / 210.0 * 100.0),
                    Some(20.0 / 210.0 * 100.0),
                    Some(90.0 / 210.0 * 100.0),
                ],
            ),
        ];

        let max = find_maxima(&matrix);
        assert_eq!(
            max,
            TabFullMaxima {
                hor: vec![
                    TabMax {
                        idxs: vec![2],
                        max: 30.0 / 210.0 * 100.0,
                    },
                    TabMax {
                        idxs: vec![0, 2],
                        max: 60.0 / 210.0 * 100.0,
                    },
                    TabMax {
                        idxs: vec![2],
                        max: 90.0 / 210.0 * 100.0,
                    },
                ],
                vert: vec![
                    TabMax {
                        idxs: vec![2],
                        max: 70.0 / 210.0 * 100.0,
                    },
                    TabMax {
                        idxs: vec![1],
                        max: 50.0 / 210.0 * 100.0,
                    },
                    TabMax {
                        idxs: vec![2],
                        max: 90.0 / 210.0 * 100.0,
                    },
                ],
            }
        );
    }

    #[test]
    fn compute_max_simple() {
        let mut engine = TabMax::default();
        engine = engine.update(1, 10);
        assert_eq!(
            engine,
            TabMax {
                idxs: vec![1],
                max: 10
            }
        );

        engine = engine.update(22, 10);
        assert_eq!(
            engine,
            TabMax {
                idxs: vec![1, 22],
                max: 10
            }
        );

        engine = engine.update(0, 42);
        assert_eq!(
            engine,
            TabMax {
                idxs: vec![0],
                max: 42
            }
        );
    }
}

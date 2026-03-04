// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides all functionality to display the constraint in detail. Usually this is
//! used as a header to the table showing the matching possibilities left in all possible
//! solutions.

use core::fmt;

use comfy_table::{presets::NOTHING, Row, Table};

use crate::constraint::{CheckType, Constraint, ConstraintGetters};
use crate::{LightCnt, MapS};

/// a renderer for the check type associated with the constraint
struct CheckTypeRender<'a> {
    /// the check-type which is to be rendered
    check: &'a CheckType,
    /// the distribution of the information gain over the possible outcomes/lights
    i: Option<Vec<(LightCnt, f64)>>,
    /// the expeced value of the information
    e: Option<f64>,
}

impl fmt::Display for CheckTypeRender<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.check {
            CheckType::Eq => write!(f, "Eq ")?,
            CheckType::Nothing | CheckType::Sold => write!(f, "Nothing ")?,
            CheckType::Lights(l, _) => {
                // information theory
                if let Some(is) = &self.i {
                    writeln!(
                        f,
                        "-> I[l]/bits: {{{}}}",
                        is.iter()
                            .map(|(l, i)| format!("{}: {:.2}", l, i))
                            .collect::<Vec<_>>()
                            .join(", "),
                    )?;
                }
                if let Some(e) = self.e {
                    writeln!(f, "-> E[I]/bits: {:.2} = H", -e)?;
                }
                write!(f, "{} lights ", l)?;
            }
        }
        Ok(())
    }
}

/// a renderer for the map associated with the constraint
struct MapSRender<'a> {
    /// the map which shall be rendered here
    map: &'a MapS,
    /// the past constraints
    past_constraints: &'a [Constraint],
    /// whether to show how many this 1:1 matching was seen in the past
    show_past_cnt: bool,
}

impl fmt::Display for MapSRender<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        let mut rows = vec![("", Row::new()); self.map.len()];
        for (i, (k, v)) in self.map.iter().enumerate() {
            if self.show_past_cnt {
                let cnt = self
                    .past_constraints
                    .iter()
                    .filter(|&c| c.show_past_cnt() && c.map_s.get(k).is_some_and(|v2| v2 == v))
                    .count();
                rows[i].0 = k;
                rows[i].1.add_cell(format!("{}x {}", cnt, k).into());
                rows[i].1.add_cell(v.into());
            } else {
                rows[i].0 = k;
                rows[i].1.add_cell(k.into());
                rows[i].1.add_cell(v.into());
            }
        }
        rows.sort_by_key(|i| i.0);
        tab.add_rows(rows.into_iter().map(|i| i.1).collect::<Vec<_>>());
        tab.column_mut(0).ok_or(fmt::Error)?.set_padding((0, 1));
        write!(f, "{tab}")
    }
}

/// An intermediate representation produced by evaluating the constraint.
///
/// Can be displayed in the process of reporting.
pub(crate) struct ReportData<'a> {
    /// a header printed above the report
    hdr: String,
    /// the map associated with the constraint
    map_s: MapSRender<'a>,
    /// data on the check-type involved
    check_type: CheckTypeRender<'a>,
    /// a footer to be printed at the end of the the report
    footer: String,
}

impl<'a> fmt::Display for ReportData<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.hdr)?;
        writeln!(f, "{}", self.map_s)?;
        writeln!(f, "---")?;

        write!(f, "{}", self.check_type)?;
        write!(f, "{}", self.footer)?;
        Ok(())
    }
}

impl Constraint {
    /// generate an intermediate representation for the header describing this constraint.
    ///
    /// The result can then be displayed/printed in the process of reporting
    pub(crate) fn generate_hdr_report<'a>(
        &'a self,
        past_constraints: &'a [Constraint],
    ) -> ReportData<'a> {
        ReportData {
            hdr: format!("{} {}", self.type_str(), self.comment()),
            map_s: MapSRender {
                map: &self.map_s,
                show_past_cnt: self.show_past_cnt(),
                past_constraints,
            },
            check_type: CheckTypeRender {
                check: &self.check,
                i: if self.show_lights_information() {
                    self.check.calc_information_gain()
                } else {
                    None
                },
                e: if self.show_expected_information() {
                    self.check.calc_expected_value()
                } else {
                    None
                },
            },
            footer: format!(
                "=> I = {} bits",
                format!("{:.4}", self.information.unwrap_or(f64::INFINITY))
                    .trim_end_matches('0')
                    .trim_end_matches('.')
            ),
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use pretty_assertions::assert_eq;
    use rust_decimal::dec;

    use crate::constraint::ConstraintType;

    use super::*;

    #[test]
    fn check_type_render_simple() {
        let ctr = CheckTypeRender {
            check: &CheckType::Eq {},
            i: Some(vec![(0, 0.0)]),
            e: Some(0.0),
        };
        assert_eq!(ctr.to_string(), "Eq ");

        let ctr = CheckTypeRender {
            check: &CheckType::Sold {},
            i: Some(vec![(0, 1.0)]),
            e: Some(0.0),
        };
        assert_eq!(ctr.to_string(), "Nothing ");

        let ctr = CheckTypeRender {
            check: &CheckType::Nothing {},
            i: Some(vec![(0, 1.0)]),
            e: Some(0.0),
        };
        assert_eq!(ctr.to_string(), "Nothing ");

        let ctr = CheckTypeRender {
            check: &CheckType::Lights(3, Default::default()),
            i: Some(vec![(0, 1.0)]),
            e: Some(-0.5),
        };
        assert_eq!(
            ctr.to_string(),
            "-> I[l]/bits: {0: 1.00}\n-> E[I]/bits: 0.50 = H\n3 lights "
        );

        let ctr = CheckTypeRender {
            check: &CheckType::Lights(3, Default::default()),
            i: Some(vec![(0, 1.0)]),
            e: None,
        };
        assert_eq!(ctr.to_string(), "-> I[l]/bits: {0: 1.00}\n3 lights ");

        let ctr = CheckTypeRender {
            check: &CheckType::Lights(3, Default::default()),
            i: None,
            e: None,
        };
        assert_eq!(ctr.to_string(), "3 lights ");

        let ctr = CheckTypeRender {
            check: &CheckType::Lights(3, Default::default()),
            i: None,
            e: Some(-0.5),
        };
        assert_eq!(ctr.to_string(), "-> E[I]/bits: 0.50 = H\n3 lights ");
    }

    #[test]
    fn map_s_render_simple() {
        let msr = MapSRender {
            map: &vec![("bbb", "B"), ("a", "A"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            past_constraints: &[],
            show_past_cnt: false,
        };
        assert_eq!(
            msr.to_string(),
            r#"a   → A 
bbb → B 
c   → C "#
        );

        let msr = MapSRender {
            map: &vec![("bbb", "B"), ("a", "A"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            past_constraints: &[],
            show_past_cnt: true,
        };
        assert_eq!(
            msr.to_string(),
            r#"0x a   → A 
0x bbb → B 
0x c   → C "#
        );

        let c1 = Constraint {
            r#type: ConstraintType::Box {
                num: dec![1],
                comment: "".to_string(),
                offer: None,
            },
            map_s: vec![("a", "A")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            ..Default::default()
        };
        assert!(!c1.show_past_cnt());

        let c2 = Constraint {
            r#type: ConstraintType::Night {
                num: dec![1],
                comment: "".to_string(),
                offer: None,
            },
            map_s: vec![("a", "A"), ("b", "C"), ("c", "B")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            ..Default::default()
        };
        assert!(c2.show_past_cnt());

        let c3 = Constraint {
            r#type: ConstraintType::Night {
                num: dec![1],
                comment: "".to_string(),
                offer: None,
            },
            map_s: vec![("a", "A"), ("b", "D"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            ..Default::default()
        };
        assert!(c3.show_past_cnt());

        let msr = MapSRender {
            map: &vec![("b", "B"), ("a", "A"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            past_constraints: &[c1, c2, c3],
            show_past_cnt: true,
        };
        assert_eq!(
            msr.to_string(),
            r#"2x a → A 
0x b → B 
1x c → C "#
        );
    }
}

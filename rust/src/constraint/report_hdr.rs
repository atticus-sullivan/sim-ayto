// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides all functionality to display the constraint in detail. Usually this is
//! used as a header to the table showing the matching possibilities left in all possible
//! solutions.

use core::fmt;
use std::collections::HashMap;

use comfy_table::{presets::NOTHING, Row, Table};

use crate::constraint::{CheckType, Constraint, ConstraintGetters};
use crate::{prob_comfy_cell, LightCnt, Lut, MapS, Rem};

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
            CheckType::HintCntMatch(x) => write!(f, "Xcnt({x}) ")?,
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
    /// whether to show the keys of the map
    show_keys: bool,
    /// whether to show the values of the map
    show_values: bool,
    /// the probabilities of the matching before and optionally also after the constraint was
    /// applied
    /// after:None if the probability did not change
    probs: Option<HashMap<&'a String, (f64, Option<f64>)>>,
}

/// a small internal enum for the columns used in the hdr table
enum TabCol {
    /// this column shows how often this matching was observed in the past
    PastCounts,
    /// this column shows the key of the map
    Keys,
    /// this column shows an arrow for pretty-printing the map
    Arrow,
    /// this column shows the value of the map
    Values,
    /// this column shows the x signaling "times"
    Times,
    /// this column shows the probabiliy of the 1:1 matching before the constraint
    ProbBefore,
    /// this column shows the probabiliy of the 1:1 matching after the constraint
    ProbAfter,
    /// this column conditionally shows an arrow for the probabilities
    PArrow,
}

impl TabCol {
    /// determine the padding for a certain column
    fn padding(&self) -> (u16, u16) {
        match self {
            TabCol::PastCounts => (0, 0),
            TabCol::Keys => (0, 0),
            TabCol::Arrow => (1, 1),
            TabCol::PArrow => (1, 1),
            TabCol::Values => (0, 0),
            TabCol::Times => (0, 1),
            TabCol::ProbBefore => (4, 0),
            TabCol::ProbAfter => (0, 0),
        }
    }
}

impl fmt::Display for MapSRender<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tab = Table::new();
        tab.force_no_tty().enforce_styling().load_preset(NOTHING);

        let mut rows = vec![("", Row::new()); self.map.len()];

        let cols: Vec<_> = [
            self.show_past_cnt.then_some(TabCol::PastCounts),
            self.show_past_cnt.then_some(TabCol::Times),
            self.show_keys.then_some(TabCol::Keys),
            (self.show_keys && self.show_values).then_some(TabCol::Arrow),
            self.show_values.then_some(TabCol::Values),
            self.probs.as_ref().map(|_| TabCol::ProbBefore),
            self.probs.as_ref().map(|_| TabCol::PArrow),
            self.probs.as_ref().map(|_| TabCol::ProbAfter),
        ]
        .into_iter()
        .flatten()
        .collect();

        for (i, (k, v)) in self.map.iter().enumerate() {
            rows[i].0 = k;
            for c in &cols {
                match c {
                    TabCol::PastCounts => {
                        let cnt = self
                            .past_constraints
                            .iter()
                            .filter(|&c| {
                                c.show_past_cnt() && c.map_s.get(k).is_some_and(|v2| v2 == v)
                            })
                            .count();
                        rows[i].1.add_cell(cnt.into());
                    }
                    TabCol::Keys => {
                        rows[i].1.add_cell(k.into());
                    }
                    TabCol::Arrow => {
                        rows[i].1.add_cell('\u{2192}'.into());
                        // '\u{2192}'
                        // '\u{21D2}'
                        // '\u{21E8}'
                        // '\u{21FE}'
                    }
                    TabCol::Values => {
                        rows[i].1.add_cell(v.into());
                    }
                    TabCol::Times => {
                        rows[i].1.add_cell('x'.into());
                    }
                    TabCol::ProbBefore => {
                        // no danger due to unwrap, ProbBefore is only set if it is some
                        rows[i].1.add_cell(prob_comfy_cell(self.probs.as_ref().unwrap()[k].0));
                    }
                    TabCol::ProbAfter => {
                        // no danger due to unwrap, ProbAfter is only set if it is some
                        if let Some(x) = self.probs.as_ref().unwrap()[k].1 {
                            rows[i].1.add_cell(prob_comfy_cell(x));
                        }
                    }
                    TabCol::PArrow => {
                        // only show the arrow if it actually points to something
                        // no danger due to unwrap, PArrow is only set if it is some
                        if self.probs.as_ref().unwrap()[k].1.is_some() {
                            rows[i].1.add_cell('\u{2192}'.into());
                        }
                    }
                }
            }
        }
        rows.sort_by_key(|i| i.0);
        tab.add_rows(rows.into_iter().map(|i| i.1).collect::<Vec<_>>());
        for (i, c) in cols.iter().enumerate() {
            tab.column_mut(i)
                .ok_or(fmt::Error)?
                .set_padding(c.padding());
        }
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
        rem_before: &Rem,
        rem_after: &Rem,
        lut_a: &Lut,
        lut_b: &Lut,
    ) -> ReportData<'a> {
        let probs = self.show_probs().then(|| {
            let probs_before = self.map_s.iter().map(|(k,v)| {
                (k, rem_before.0[*lut_a.get(k).unwrap()][*lut_b.get(v).unwrap()] as f64 * 100.0 / rem_before.1 as f64)
            }).collect::<Vec<_>>();

            let probs_after = self.map_s.iter().map(|(k,v)| {
                (k, rem_after.0[*lut_a.get(k).unwrap()][*lut_b.get(v).unwrap()] as f64 * 100.0 / rem_after.1 as f64)
            }).collect::<Vec<_>>();

            probs_before
                .iter()
                .zip(probs_after)
                .map(|(before, after)| {
                    (before.0, (before.1, (before.1 != after.1).then_some(after.1)))
                })
                .collect::<HashMap<_,_>>()
        });

        ReportData {
            hdr: format!("{} {}", self.type_str(), self.comment()),
            map_s: MapSRender {
                map: &self.map_s,
                show_past_cnt: self.show_past_cnt(),
                past_constraints,
                show_keys: self.check.is_relevant_map_keys(),
                show_values: self.check.is_relevant_map_values(),
                probs,
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
            check: &CheckType::HintCntMatch(2),
            i: Some(vec![(0, 0.0)]),
            e: Some(0.0),
        };
        assert_eq!(ctr.to_string(), "Xcnt(2) ");

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
            show_keys: true,
            show_values: true,
        };
        assert_eq!(
            msr.to_string(),
            r#"a   → A
bbb → B
c   → C"#
        );
    }

    #[test]
    fn map_s_render_no_keys() {
        let msr = MapSRender {
            map: &vec![("bbb", "B"), ("a", "A"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            past_constraints: &[],
            show_past_cnt: false,
            show_keys: false,
            show_values: true,
        };
        assert_eq!(
            msr.to_string(),
            r#"A
B
C"#
        );
    }

    #[test]
    fn map_s_render_show_past() {
        let msr = MapSRender {
            map: &vec![("bbb", "B"), ("a", "A"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            past_constraints: &[],
            show_past_cnt: true,
            show_keys: true,
            show_values: true,
        };
        assert_eq!(
            msr.to_string(),
            r#"0x a   → A
0x bbb → B
0x c   → C"#
        );
    }

    #[test]
    fn map_s_render_roundtrip() {
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
            show_keys: true,
            show_values: true,
        };
        assert_eq!(
            msr.to_string(),
            r#"2x a → A
0x b → B
1x c → C"#
        );
    }
}

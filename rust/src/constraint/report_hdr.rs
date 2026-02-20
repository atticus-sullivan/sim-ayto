/// This module provides all functionality to display the constraint in detail. Usually this is
/// used as a header to the table showing the matching possibilities left in all possible
/// solutions.
use core::fmt;

use anyhow::Result;
use comfy_table::{presets::NOTHING, Row, Table};

use crate::constraint::{CheckType, Constraint};
use crate::MapS;

struct CheckTypeRender<'a> {
    check: &'a CheckType,
    i: Option<Vec<(u8, f64)>>,
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

struct MapSRender<'a, 'b> {
    map: &'a MapS,
    past_constraints: &'b [&'b Constraint],
    show_past_cnt: bool,
}

impl fmt::Display for MapSRender<'_, '_> {
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
        writeln!(f, "{tab}")
    }
}

impl Constraint {
    pub fn print_hdr(&self, past_constraints: &[&Constraint]) -> Result<()> {
        println!("{}", self.type_str());

        println!();

        println!(
            "{}",
            MapSRender {
                map: &self.map_s,
                show_past_cnt: self.show_past_cnt(),
                past_constraints,
            }
        );

        println!("---");

        println!(
            "{}",
            CheckTypeRender {
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
            }
        );
        println!(
            "=> I = {} bits",
            format!("{:.4}", self.information.unwrap_or(f64::INFINITY))
                .trim_end_matches('0')
                .trim_end_matches('.')
        );
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
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
c   → C 
"#
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
0x c   → C 
"#
        );

        let mut c1 = Constraint::default();
        c1.r#type = ConstraintType::Box {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        c1.map_s = vec![("a", "A")]
            .into_iter()
            .map(|(i, j)| (i.to_string(), j.to_string()))
            .collect::<MapS>();
        assert!(!c1.show_past_cnt());

        let mut c2 = Constraint::default();
        c2.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        c2.map_s = vec![("a", "A"), ("b", "C"), ("c", "B")]
            .into_iter()
            .map(|(i, j)| (i.to_string(), j.to_string()))
            .collect::<MapS>();
        assert!(c2.show_past_cnt());

        let mut c3 = Constraint::default();
        c3.r#type = ConstraintType::Night {
            num: dec![1],
            comment: "".to_string(),
            offer: None,
        };
        c3.map_s = vec![("a", "A"), ("b", "D"), ("c", "C")]
            .into_iter()
            .map(|(i, j)| (i.to_string(), j.to_string()))
            .collect::<MapS>();
        assert!(c3.show_past_cnt());

        let msr = MapSRender {
            map: &vec![("b", "B"), ("a", "A"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<MapS>(),
            past_constraints: &[&c1, &c2, &c3],
            show_past_cnt: true,
        };
        assert_eq!(
            msr.to_string(),
            r#"2x a → A 
0x b → B 
1x c → C 
"#
        );
    }
}

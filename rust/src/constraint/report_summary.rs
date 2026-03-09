// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module renders one row for a comfy_table table showing a summary of this constraint (type,
//! map, information, etc)

use core::fmt;

use comfy_table::Cell;

use crate::constraint::evaluate_predicates::ConstraintEval;
use crate::constraint::{CheckType, Constraint, ConstraintGetters, ConstraintType};
use crate::matching_repr::bitset::Bitset;
use crate::LightCnt;

/// A struct representing a row in the summary table. The idea is this is produced by the
/// evaluation. Then this can be displayed in the process of reporting.
#[derive(Clone, Debug)]
pub(crate) struct SummaryRow {
    /// a label for this row
    label: String,
    /// lights produced by the constraint, value attached with meaning
    light_status: (LightCell, LightSemantic),
    /// "entries"/names in this constraint (the other side is the header in the row) attached with
    /// meaning
    entries: Vec<(EntryCell, EntrySemantic)>,
    /// the information gain by this constraint
    info: Option<f64>,
    /// how many new 1:1 matchings in this constraint
    new_count: Option<usize>,
    /// to which other constraint the distance is at its minimum (distance + label of the
    /// constraint)
    min_dist: Option<(String, usize)>,
}

impl SummaryRow {
    /// render the [`SummaryRow`] to a row so it can be used by comfy_table
    pub(crate) fn render<F>(&self, style: F) -> Vec<Cell>
    where
        F: Fn(Cell) -> Cell,
    {
        let mut ret = vec![];
        ret.push(Cell::new(self.label.clone()));
        ret.push(
            self.light_status
                .1
                .style(Cell::new(self.light_status.0.to_string())),
        );
        ret.extend(
            self.entries
                .iter()
                .map(|x| x.1.style(Cell::new(x.0.to_string()))),
        );

        ret.push(Cell::new(""));

        ret.push(Cell::new(
            self.info
                .map(|x| {
                    format!("{:6.4}", x)
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_owned()
                })
                .unwrap_or("".to_string()),
        ));
        ret.push(Cell::new(
            self.new_count
                .map(|x| x.to_string())
                .unwrap_or("".to_string()),
        ));
        ret.push(Cell::new(
            self.min_dist
                .clone()
                .map(|x| format!("{}/{}", x.1, x.0))
                .unwrap_or("".to_string()),
        ));
        // apply the style specified from the outside
        ret.into_iter().map(style).collect::<Vec<_>>()
    }
}

/// attach a meaning to the amount of lights so we can style based on meaning, not on value
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LightSemantic {
    /// we know this was a match
    Match, // green
    /// we know this was not a match
    NoMatch, // red
    /// we don't gain information by this
    NoGain, // yellow
    /// we did gain information, but nothing certain
    Neutral,
}

impl LightSemantic {
    /// style the cell `c` based on the [`LightSemantic`]
    fn style(&self, c: Cell) -> Cell {
        match self {
            LightSemantic::Match => c.fg(comfy_table::Color::Green),
            LightSemantic::NoMatch => c.fg(comfy_table::Color::Red),
            LightSemantic::NoGain => c.fg(comfy_table::Color::Yellow),
            LightSemantic::Neutral => c,
        }
    }
}

/// a cell with a name in the summary row describing the amount of lights achieved by this
/// constraint
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LightCell {
    /// we don't know how many lights were produced by this constraint
    Unknown,
    /// this constraint did not produce lights, it has Eq check-type
    Equal,
    /// this constraint produced [`crate::LightCnt`] new lights
    Value(LightCnt),
    /// we get to known an individual is a match together with X other individuals of the same set
    Xcnt,
}

impl fmt::Display for LightCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LightCell::Unknown => write!(f, "?"),
            LightCell::Xcnt => write!(f, "X"),
            LightCell::Equal => write!(f, "E"),
            LightCell::Value(value) => write!(f, "{value}"),
        }
    }
}

/// attach a meaning to entries/cells so we can style based on meaning, not on value
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntrySemantic {
    /// this entry produced a new known match
    Match,
    /// this entry produced a new known no-match
    NoMatch,
    /// we don't know anything about the outcome produced by this entry
    Unknown,
}

impl EntrySemantic {
    /// style the cell `c` based on the [`EntrySemantic`]
    fn style(&self, c: Cell) -> Cell {
        match self {
            EntrySemantic::Match => c.fg(comfy_table::Color::Green),
            EntrySemantic::NoMatch => c.fg(comfy_table::Color::Red),
            EntrySemantic::Unknown => c,
        }
    }
}

/// a cell with a name in the summary row describing this constraint
#[derive(Clone, Debug, PartialEq, Eq)]
struct EntryCell {
    /// the name to be placed in this cell
    value: String,
    /// whether this is 1:1 match
    is_new: bool,
    /// whether the 1:1 match should be shown
    show_new: bool,
}

impl fmt::Display for EntryCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)?;
        if self.show_new && self.is_new {
            write!(f, "*")?;
        }
        Ok(())
    }
}

impl Constraint {
    /// evaluate and produce summary data regarding this constraint
    pub(crate) fn summary_row_data(
        &self,
        transpose: bool,
        map_hor: &[String],
        past: &[Constraint],
    ) -> SummaryRow {
        let map_s = if transpose {
            &self
                .map_s
                .iter()
                .map(|(k, v)| (v.clone(), k.clone()))
                .collect()
        } else {
            &self.map_s
        };

        let mut match_sem = None;

        let light_status = if self.result_unknown {
            (LightCell::Unknown, LightSemantic::Neutral)
        } else {
            match &self.check {
                CheckType::Eq => (LightCell::Equal, LightSemantic::Neutral),
                CheckType::HintCntMatch(..) => (LightCell::Xcnt, LightSemantic::Neutral),
                CheckType::Nothing => (LightCell::Unknown, LightSemantic::Neutral),
                CheckType::Sold => (LightCell::Unknown, LightSemantic::NoGain),
                CheckType::Lights(lights, _) => {
                    let lights = *lights;
                    match self.r#type {
                        ConstraintType::Night { .. } => {
                            (LightCell::Value(lights), LightSemantic::Neutral)
                        }
                        ConstraintType::Box { .. } => {
                            if self.is_match_found() {
                                match_sem = Some(EntrySemantic::Match);
                                (LightCell::Value(lights), LightSemantic::Match)
                            } else {
                                match_sem = Some(EntrySemantic::NoMatch);
                                (LightCell::Value(lights), LightSemantic::NoMatch)
                            }
                        }
                    }
                }
            }
        };

        let entries = map_hor
            .iter()
            .map(|v1| {
                map_s
                    .get(v1)
                    .map(|v2| {
                        let (a, b) = if !transpose { (v1, v2) } else { (v2, v1) };
                        (
                            EntryCell {
                                value: v2.to_string(),
                                is_new: !past.iter().any(|c| {
                                    c.adds_new() && c.map_s.get(a).is_some_and(|v2| v2 == b)
                                }),
                                show_new: self.show_new(),
                            },
                            match_sem.unwrap_or(EntrySemantic::Unknown),
                        )
                    })
                    .unwrap_or((
                        EntryCell {
                            value: "".to_string(),
                            is_new: false,
                            show_new: self.show_new(),
                        },
                        match_sem.unwrap_or(EntrySemantic::Unknown),
                    ))
            })
            .collect::<Vec<_>>();

        let info = if self.result_unknown {
            None
        } else {
            match &self.check {
                CheckType::Eq | CheckType::HintCntMatch(..) | CheckType::Lights(..) => {
                    Some(self.information.unwrap_or(f64::INFINITY))
                }
                CheckType::Nothing | CheckType::Sold => None,
            }
        };

        let min_dist = if self.show_past_dist() {
            past.iter()
                .filter(|&c| c.show_past_dist())
                .map(|c| (c.type_str(), self.distance(c).unwrap_or(usize::MAX)))
                .min_by_key(|i| i.1)
        } else {
            None
        };

        SummaryRow {
            label: self.type_str(),
            light_status,
            entries,
            info,
            new_count: self.new_matches(past),
            min_dist,
        }
    }

    /// how many 1:1 matches are new in this constraint
    fn new_matches(&self, past: &[Constraint]) -> Option<usize> {
        if self.result_unknown {
            None
        } else if let ConstraintType::Night { .. } = self.r#type {
            let cnt = self
                .map
                .iter()
                .enumerate()
                .filter(|&(k, v)| {
                    !v.is_empty()
                        && !past.iter().any(|c| {
                            c.adds_new()
                                && c.map
                                    .slot_mask(k)
                                    .unwrap_or(&Bitset::empty())
                                    .contains_any(v)
                        })
                })
                .count();
            Some(cnt)
        } else {
            None
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use std::collections::HashMap;

    use pretty_assertions::assert_eq;
    use rust_decimal::dec;

    use crate::matching_repr::MaskedMatching;

    use super::*;

    #[test]
    fn summary_row_render_simple() {
        let sr = SummaryRow {
            label: "label".to_string(),
            light_status: (LightCell::Value(5), LightSemantic::Neutral),
            entries: vec![(
                EntryCell {
                    value: "abc".to_string(),
                    is_new: true,
                    show_new: true,
                },
                EntrySemantic::Unknown,
            )],
            info: Some(1.0),
            new_count: Some(5),
            min_dist: Some(("MN1".to_string(), 5)),
        };
        let cells = sr.render(|c| c);
        assert_eq!(cells.len(), 7);
    }

    #[test]
    fn lightcell_display_simple() {
        assert_eq!(LightCell::Unknown.to_string(), "?");
        assert_eq!(LightCell::Value(10).to_string(), "10");
        assert_eq!(LightCell::Equal.to_string(), "E");
        assert_eq!(LightCell::Xcnt.to_string(), "X");
    }

    #[test]
    fn entrycell_display_simple() {
        assert_eq!(
            EntryCell {
                value: "abc".to_string(),
                is_new: true,
                show_new: true,
            }
            .to_string(),
            "abc*"
        );

        assert_eq!(
            EntryCell {
                value: "abc".to_string(),
                is_new: false,
                show_new: true,
            }
            .to_string(),
            "abc"
        );

        assert_eq!(
            EntryCell {
                value: "abc".to_string(),
                is_new: true,
                show_new: false,
            }
            .to_string(),
            "abc"
        );

        assert_eq!(
            EntryCell {
                value: "abc".to_string(),
                is_new: false,
                show_new: false,
            }
            .to_string(),
            "abc"
        );
    }

    #[test]
    fn row_data_simple() {
        let c1 = Constraint {
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            map_s: vec![("a", "A"), ("b", "B"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<HashMap<_, _>>(),
            r#type: ConstraintType::Night {
                num: dec![2.5],
                comment: "abc".to_string(),
                offer: None,
            },
            check: CheckType::Lights(3, Default::default()),
            result_unknown: false,
            information: Some(1.5),
            ..Default::default()
        };
        assert!(c1.adds_new());

        let c2 = Constraint {
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1]]),
            map_s: vec![("a", "A"), ("b", "B")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<HashMap<_, _>>(),
            r#type: ConstraintType::Night {
                num: dec![0.5],
                comment: "ihg".to_string(),
                offer: None,
            },
            check: CheckType::Eq,
            result_unknown: true,
            information: Some(1.5),
            ..Default::default()
        };
        assert!(!c2.adds_new());

        let c3 = Constraint {
            map: MaskedMatching::from_matching_ref(&[vec![], vec![1]]),
            map_s: vec![("b", "B")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<HashMap<_, _>>(),
            r#type: ConstraintType::Box {
                num: dec![10],
                comment: "xyz".to_string(),
                offer: None,
            },
            check: CheckType::Lights(1, Default::default()),
            result_unknown: false,
            information: None,
            ..Default::default()
        };
        assert!(c3.adds_new());

        let sr = c1.summary_row_data(
            false,
            &["a", "b", "c"]
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>(),
            &[c2.clone(), c3.clone()],
        );
        let ref_sr = SummaryRow {
            label: "MN#2.5".to_string(),
            light_status: (LightCell::Value(3), LightSemantic::Neutral),
            entries: vec![
                (
                    EntryCell {
                        value: "A".to_string(),
                        is_new: true,
                        show_new: true,
                    },
                    EntrySemantic::Unknown,
                ),
                (
                    EntryCell {
                        value: "B".to_string(),
                        is_new: false,
                        show_new: true,
                    },
                    EntrySemantic::Unknown,
                ),
                (
                    EntryCell {
                        value: "C".to_string(),
                        is_new: true,
                        show_new: true,
                    },
                    EntrySemantic::Unknown,
                ),
            ],
            info: Some(0.5),
            new_count: Some(2),
            min_dist: None,
        };
        assert_eq!(sr.label, ref_sr.label);
        assert_eq!(sr.light_status, ref_sr.light_status);
        assert_eq!(sr.entries, ref_sr.entries);
        assert_eq!(sr.info.is_some(), ref_sr.info.is_some());
        assert_eq!(sr.new_count, ref_sr.new_count);
        assert_eq!(sr.min_dist, ref_sr.min_dist);

        let sr = c2.summary_row_data(
            false,
            &["a", "b", "c"]
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>(),
            &[c1.clone(), c3.clone()],
        );
        let ref_sr = SummaryRow {
            label: "MN#0.5".to_string(),
            light_status: (LightCell::Unknown, LightSemantic::Neutral),
            entries: vec![
                (
                    EntryCell {
                        value: "A".to_string(),
                        is_new: false,
                        show_new: false,
                    },
                    EntrySemantic::Unknown,
                ),
                (
                    EntryCell {
                        value: "B".to_string(),
                        is_new: false,
                        show_new: false,
                    },
                    EntrySemantic::Unknown,
                ),
                (
                    EntryCell {
                        value: "".to_string(),
                        is_new: false,
                        show_new: false,
                    },
                    EntrySemantic::Unknown,
                ),
            ],
            info: None,
            new_count: None,
            min_dist: None,
        };
        assert_eq!(sr.label, ref_sr.label);
        assert_eq!(sr.light_status, ref_sr.light_status);
        assert_eq!(sr.entries, ref_sr.entries);
        assert_eq!(sr.info.is_some(), ref_sr.info.is_some());
        assert_eq!(sr.new_count, ref_sr.new_count);
        assert_eq!(sr.min_dist, ref_sr.min_dist);

        let sr = c3.summary_row_data(
            false,
            &["a", "b", "c"]
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>(),
            &[c1, c2],
        );
        let ref_sr = SummaryRow {
            label: "MB#10".to_string(),
            light_status: (LightCell::Value(1), LightSemantic::Match),
            entries: vec![
                (
                    EntryCell {
                        value: "".to_string(),
                        is_new: false,
                        show_new: false,
                    },
                    EntrySemantic::Match,
                ),
                (
                    EntryCell {
                        value: "B".to_string(),
                        is_new: false,
                        show_new: false,
                    },
                    EntrySemantic::Match,
                ),
                (
                    EntryCell {
                        value: "".to_string(),
                        is_new: false,
                        show_new: false,
                    },
                    EntrySemantic::Match,
                ),
            ],
            info: Some(0.5),
            new_count: None,
            min_dist: None,
        };
        assert_eq!(sr.label, ref_sr.label);
        assert_eq!(sr.light_status, ref_sr.light_status);
        assert_eq!(sr.entries, ref_sr.entries);
        assert_eq!(sr.info.is_some(), ref_sr.info.is_some());
        assert_eq!(sr.new_count, ref_sr.new_count);
        assert_eq!(sr.min_dist, ref_sr.min_dist);
    }

    #[test]
    fn new_matches_simple() {
        let c1 = Constraint {
            map: MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            map_s: vec![("a", "A"), ("b", "B"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<HashMap<_, _>>(),
            r#type: ConstraintType::Night {
                num: dec![2.5],
                comment: "abc".to_string(),
                offer: None,
            },
            ..Default::default()
        };

        let c2 = Constraint {
            map: MaskedMatching::from_matching_ref(&[vec![1], vec![0], vec![2]]),
            map_s: vec![("a", "B"), ("b", "A"), ("c", "C")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<HashMap<_, _>>(),
            r#type: ConstraintType::Night {
                num: dec![0.5],
                comment: "xyz".to_string(),
                offer: None,
            },
            ..Default::default()
        };

        let c3 = Constraint {
            map: MaskedMatching::from_matching_ref(&[vec![2], vec![1], vec![0]]),
            map_s: vec![("a", "C"), ("b", "B"), ("c", "A")]
                .into_iter()
                .map(|(i, j)| (i.to_string(), j.to_string()))
                .collect::<HashMap<_, _>>(),
            r#type: ConstraintType::Night {
                num: dec![10.5],
                comment: "jkl".to_string(),
                offer: None,
            },
            ..Default::default()
        };

        let n = c1.new_matches(&[c2.clone(), c3.clone()]);
        assert_eq!(n, Some(1));
    }
}

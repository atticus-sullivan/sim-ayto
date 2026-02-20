/// This module renders one row for a comfy_table table showing a summary of this constraint (type,
/// map, information, etc)

use core::fmt;

use comfy_table::Cell;

use crate::constraint::{CheckType, Constraint, ConstraintType};
use crate::matching_repr::bitset::Bitset;

#[derive(Clone, Debug)]
pub(crate) struct SummaryRow {
    label: String,
    light_status: (LightCell, LightSemantic),
    entries: Vec<(EntryCell, EntrySemantic)>,
    info: Option<f64>,
    new_count: Option<usize>,
    min_dist: Option<(String, usize)>,
}

impl SummaryRow {
    pub(crate) fn render<F>(&self, style: F) -> Vec<Cell>
    where
        F: Fn(Cell) -> Cell
    {
        let mut ret = vec![];
        ret.push(Cell::new(self.label.clone()));
        ret.push(
            self.light_status.1.style(Cell::new(self.light_status.0.to_string()))
        );
        ret.extend(
            self.entries.iter().map(|x| x.1.style(Cell::new(x.0.to_string())))
        );

        ret.push(Cell::new(""));

        ret.push(Cell::new(self.info
            .map(|x| format!("{:6.4}", x).trim_end_matches('0').trim_end_matches('.').to_owned())
            .unwrap_or("".to_string())
        ));
        ret.push(Cell::new(self.new_count
            .map(|x| x.to_string())
            .unwrap_or("".to_string())
        ));
        ret.push(Cell::new(self.min_dist
            .clone()
            .map(|x| format!("{}/{}", x.1, x.0))
            .unwrap_or("".to_string())
        ));
        // apply the style specified from the outside
        ret.into_iter().map(style).collect::<Vec<_>>()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LightSemantic {
    Match,    // green
    NoMatch, // red
    NoGain,  // yellow
    Neutral,
}

impl LightSemantic {
    fn style(&self, c: Cell) -> Cell {
        match self {
            LightSemantic::Match => c.fg(comfy_table::Color::Green),
            LightSemantic::NoMatch => c.fg(comfy_table::Color::Red),
            LightSemantic::NoGain => c.fg(comfy_table::Color::Yellow),
            LightSemantic::Neutral => c,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LightCell {
    Unknown,
    Equal,
    Value(u8),
}

impl fmt::Display for LightCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LightCell::Unknown => write!(f, "?"),
            LightCell::Equal => write!(f, "E"),
            LightCell::Value(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntrySemantic {
    Match,
    NoMatch,
    Unknown,
}

impl EntrySemantic {
    fn style(&self, c: Cell) -> Cell {
        match self {
            EntrySemantic::Match => c.fg(comfy_table::Color::Green),
            EntrySemantic::NoMatch => c.fg(comfy_table::Color::Red),
            EntrySemantic::Unknown => c,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EntryCell {
    value: String,
    is_new: bool,
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
    pub(crate) fn summary_row_data(&self, transpose: bool, map_hor: &[String], past: &[&Constraint]) -> SummaryRow {
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
                CheckType::Nothing => (LightCell::Unknown, LightSemantic::Neutral),
                CheckType::Sold => (LightCell::Unknown, LightSemantic::NoGain),
                CheckType::Lights(lights, _) => {
                    let lights = *lights;
                    match self.r#type {
                        ConstraintType::Night { .. } => (LightCell::Value(lights), LightSemantic::Neutral),
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
            .map(|v1| map_s.get(v1).map(|v2| {
                let (a,b) = if !transpose {
                    (v1, v2)
                } else {
                    (v2, v1)
                };
                (
                    EntryCell{
                        value: v2.to_string(),
                        is_new: past
                            .iter()
                            .any(|&c| c.adds_new() && c.map_s.get(a).is_some_and(|v2| v2 == b)),
                        show_new: self.show_new(),
                    },
                    match_sem.unwrap_or(EntrySemantic::Unknown)
                )
            }).unwrap_or((EntryCell{
                    value: "".to_string(),
                    is_new: false,
                    show_new: self.show_new(),
                }, match_sem.unwrap_or(EntrySemantic::Unknown))))
            .collect::<Vec<_>>()
        ;

        let info = match &self.check {
            CheckType::Eq | CheckType::Lights(..) => {
                Some(self.information.unwrap_or(f64::INFINITY))
            },
            CheckType::Nothing | CheckType::Sold => None,
        };

        let min_dist = if self.show_past_dist() {
            past
                .iter()
                .filter(|&c| c.show_past_dist())
                .map(|&c| (c.type_str(), self.distance(c).unwrap_or(usize::MAX)))
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

    fn new_matches(&self, past: &[&Constraint]) -> Option<usize> {
        if let ConstraintType::Night { .. } = self.r#type {
            let cnt = self
                .map
                .iter()
                .enumerate()
                .filter(|&(k, v)| {
                    !v.is_empty()
                        && !past.iter().any(|&c| {
                            c.adds_new()
                                && c.map
                                    .slot_mask(k)
                                    .unwrap_or(&Bitset::empty())
                                    .contains_any(&v)
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
mod tests {
    use std::collections::HashMap;

    use rust_decimal::dec;

    use crate::matching_repr::MaskedMatching;

    use super::*;

    #[test]
    fn summary_row_render_simple() {
        let sr = SummaryRow{
            label: "label".to_string(),
            light_status: (LightCell::Value(5), LightSemantic::Neutral),
            entries: vec![(EntryCell{value: "abc".to_string(), is_new: true, show_new: true}, EntrySemantic::Unknown)],
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
    }

    #[test]
    fn entrycell_display_simple() {
        assert_eq!(EntryCell{
            value: "abc".to_string(),
            is_new: true,
            show_new: true,
        }.to_string(), "abc*");

        assert_eq!(EntryCell{
            value: "abc".to_string(),
            is_new: false,
            show_new: true,
        }.to_string(), "abc");

        assert_eq!(EntryCell{
            value: "abc".to_string(),
            is_new: true,
            show_new: false,
        }.to_string(), "abc");

        assert_eq!(EntryCell{
            value: "abc".to_string(),
            is_new: false,
            show_new: false,
        }.to_string(), "abc");
    }

    #[test]
    fn row_data_simple() {
        let mut c1 = Constraint::default();
        c1.map = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]);
        c1.map_s = vec![("a", "A"), ("b", "B"), ("c", "C")].into_iter().map(|(i,j)| (i.to_string(),j.to_string())).collect::<HashMap<_,_>>();
        c1.r#type = ConstraintType::Night { num: dec![2.5], comment: "abc".to_string(), offer: None };
        c1.check = CheckType::Lights(3, Default::default());
        c1.result_unknown = false;
        c1.information = Some(1.5);

        let mut c2 = Constraint::default();
        c2.map = MaskedMatching::from_matching_ref(&[vec![0], vec![1]]);
        c2.map_s = vec![("a", "A"), ("b", "B")].into_iter().map(|(i,j)| (i.to_string(),j.to_string())).collect::<HashMap<_,_>>();
        c2.r#type = ConstraintType::Night { num: dec![0.5], comment: "ihg".to_string(), offer: None };
        c2.check = CheckType::Eq;
        c2.result_unknown = true;
        c2.information = Some(1.5);


        let mut c3 = Constraint::default();
        c3.map = MaskedMatching::from_matching_ref(&[vec![], vec![1]]);
        c3.map_s = vec![("b", "B")].into_iter().map(|(i,j)| (i.to_string(),j.to_string())).collect::<HashMap<_,_>>();
        c3.r#type = ConstraintType::Box { num: dec![10], comment: "xyz".to_string(), offer: None };
        c3.check = CheckType::Lights(1, Default::default());
        c3.result_unknown = false;
        c3.information = None;


        let sr = c1.summary_row_data(false, &["a", "b", "c"].iter().map(|i| i.to_string()).collect::<Vec<_>>(), &vec![&c2, &c3]);
        let ref_sr = SummaryRow{
            label: "MN#2.5".to_string(),
            light_status: (LightCell::Value(3), LightSemantic::Neutral),
            entries: vec![
                (EntryCell{value: "A".to_string(), is_new: true, show_new: true}, EntrySemantic::Unknown),
                (EntryCell{value: "B".to_string(), is_new: false, show_new: true}, EntrySemantic::Unknown),
                (EntryCell{value: "C".to_string(), is_new: true, show_new: true}, EntrySemantic::Unknown),
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

        let sr = c2.summary_row_data(false, &["a", "b", "c"].iter().map(|i| i.to_string()).collect::<Vec<_>>(), &vec![&c1, &c3]);
        let ref_sr = SummaryRow{
            label: "MN#2.5".to_string(),
            light_status: (LightCell::Equal, LightSemantic::Neutral),
            entries: vec![
                (EntryCell{value: "A".to_string(), is_new: true, show_new: false}, EntrySemantic::Unknown),
                (EntryCell{value: "B".to_string(), is_new: true, show_new: false}, EntrySemantic::Unknown),
                (EntryCell{value: "".to_string(), is_new: false, show_new: false}, EntrySemantic::Unknown),
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

        let ref_sr = SummaryRow{
            label: "MB#10".to_string(),
            light_status: (LightCell::Value(1), LightSemantic::Match),
            entries: vec![
                (EntryCell{value: "".to_string(), is_new: true, show_new: false}, EntrySemantic::Unknown),
                (EntryCell{value: "B".to_string(), is_new: true, show_new: false}, EntrySemantic::Unknown),
                (EntryCell{value: "".to_string(), is_new: false, show_new: false}, EntrySemantic::Unknown),
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
        let mut c1 = Constraint::default();
        c1.map = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]);
        c1.map_s = vec![("a", "A"), ("b", "B"), ("c", "C")].into_iter().map(|(i,j)| (i.to_string(),j.to_string())).collect::<HashMap<_,_>>();
        c1.r#type = ConstraintType::Night { num: dec![2.5], comment: "abc".to_string(), offer: None };

        let mut c2 = Constraint::default();
        c2.map = MaskedMatching::from_matching_ref(&[vec![1], vec![0], vec![2]]);
        c2.map_s = vec![("a", "B"), ("b", "A"), ("c", "C")].into_iter().map(|(i,j)| (i.to_string(),j.to_string())).collect::<HashMap<_,_>>();
        c2.r#type = ConstraintType::Night { num: dec![0.5], comment: "xyz".to_string(), offer: None };

        let mut c3 = Constraint::default();
        c3.map = MaskedMatching::from_matching_ref(&[vec![2], vec![1], vec![0]]);
        c3.map_s = vec![("a", "C"), ("b", "B"), ("c", "A")].into_iter().map(|(i,j)| (i.to_string(),j.to_string())).collect::<HashMap<_,_>>();
        c3.r#type = ConstraintType::Night { num: dec![10.5], comment: "jkl".to_string(), offer: None };

        let n = c1.new_matches(&[&c2, &c2]);
        assert_eq!(n, Some(1));
    }
}

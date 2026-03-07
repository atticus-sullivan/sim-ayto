// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module parses a ruleset. It transfers a RuleSetParse to a ready to use Ruleset.

use crate::ruleset::RuleSet;
use serde::Deserialize;

/// An enum defining all the different rulesets which can be applied to the game.
/// This enum is only for parsing such a ruleset from file
#[derive(Deserialize, Debug)]
pub enum RuleSetParse {
    /// A ruleset where X duplicates exist. One of the two individuals forming the dup might be
    /// known (`Some(name)`) or not (`None`).
    /// The dups have to exist on the set_b side.
    XTimesDup(Vec<Option<String>>),
    /// A ruleset where exactly one triple exists. None of the individuals of the triple is known.
    /// The triple has to exist on the set_b side.
    SomeoneIsTrip,
    /// A ruleset where exactly one triple exists. One of three individuals of the triple is known
    /// The triple has to exist on the set_b side.
    FixedTrip(String),
    /// A ruleset where essentially N:N players play. But there are not really fixed sets a and b.
    /// Instead everyone can match everyone, but it is still a strict 1:1 matching
    NToN,
    /// A ruleset where N:N players play so each individual from set_a matches exactly one
    /// individual from set_b
    Eq,
}

impl RuleSetParse {
    /// finalizes the parsing by consuming the `RuleSetParse` and producing the final `RuleSet`
    pub fn finalize_parsing(self) -> RuleSet {
        match self {
            RuleSetParse::SomeoneIsTrip => RuleSet::SomeoneIsTrip,
            RuleSetParse::NToN => RuleSet::NToN,
            RuleSetParse::FixedTrip(s) => RuleSet::FixedTrip(s),
            RuleSetParse::Eq => RuleSet::Eq,
            RuleSetParse::XTimesDup(s) => {
                let nc = s.iter().filter(|s| s.is_none()).count();
                let ss = s.into_iter().flatten().collect::<Vec<_>>();
                RuleSet::XTimesDup((nc, ss))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruleset::RuleSet;

    #[test]
    fn finalize_parsing_someone_is_trip_simple() {
        let parsed = RuleSetParse::SomeoneIsTrip;
        let result = parsed.finalize_parsing();
        assert_eq!(result, RuleSet::SomeoneIsTrip);
    }

    #[test]
    fn finalize_parsing_fixed_trip_simple() {
        let parsed = RuleSetParse::FixedTrip("x".to_string());
        let result = parsed.finalize_parsing();
        assert_eq!(result, RuleSet::FixedTrip("x".to_string()));
    }

    #[test]
    fn finalize_parsing_eq_simple() {
        let parsed = RuleSetParse::Eq;
        let result = parsed.finalize_parsing();
        assert_eq!(result, RuleSet::Eq);
    }

    #[test]
    fn finalize_parsing_n_to_n_simple() {
        let parsed = RuleSetParse::NToN;
        let result = parsed.finalize_parsing();
        assert_eq!(result, RuleSet::NToN);
    }

    #[test]
    fn finalize_parsing_x_times_dup_all_none_simple() {
        let parsed = RuleSetParse::XTimesDup(vec![None, None, None]);
        let result = parsed.finalize_parsing();
        match result {
            RuleSet::XTimesDup((cnt, vec)) => {
                assert_eq!(cnt, 3);
                assert!(vec.is_empty());
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn finalize_parsing_x_times_dup_mix_simple() {
        let parsed = RuleSetParse::XTimesDup(vec![
            Some("a".to_string()),
            None,
            Some("b".to_string()),
            None,
        ]);
        let result = parsed.finalize_parsing();
        match result {
            RuleSet::XTimesDup((cnt, vec)) => {
                assert_eq!(cnt, 2);
                assert_eq!(vec, vec!["a".to_string(), "b".to_string()]);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn finalize_parsing_x_times_dup_no_none_simple() {
        let parsed = RuleSetParse::XTimesDup(vec![Some("x".to_string()), Some("y".to_string())]);
        let result = parsed.finalize_parsing();
        match result {
            RuleSet::XTimesDup((cnt, vec)) => {
                assert_eq!(cnt, 0);
                assert_eq!(vec, vec!["x".to_string(), "y".to_string()]);
            }
            _ => panic!("unexpected variant"),
        }
    }
}

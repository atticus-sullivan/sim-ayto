use crate::ruleset::RuleSet;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub enum RuleSetParse {
    SomeoneIsTrip,
    XTimesDup(Vec<Option<String>>),
    NToN,
    FixedTrip(String),
    Eq,
}

impl RuleSetParse {
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

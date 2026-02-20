use crate::{constraint::Constraint, Rem};


struct ReportEvent<'a> {
    rem: Rem,
    constraint: &'a Constraint,
}

type Trail<'a> = (Rem, Vec<ReportEvent<'a>>);

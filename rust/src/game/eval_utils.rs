use anyhow::Result;

use crate::constraint::Constraint;

pub(super) fn merge_constraints(constraints: &[Constraint]) -> Result<Vec<Constraint>> {
    let mut merged = vec![];
    let mut needs_merging = vec![];
    for c in constraints {
        if c.should_merge() {
            needs_merging.push(c);
            continue;
        }
        let mut d = c.clone();
        while !needs_merging.is_empty() {
            d.merge(needs_merging.pop().unwrap())?;
        }
        merged.push(d);
    }
    Ok(merged)
}

//! This module contains standalone helpers to faciliate turning a parsed (from yaml) game to a
//! ready-to-use `Game`.

use anyhow::{bail, Result};

use crate::constraint::parse::ConstraintParse;
use crate::constraint::Constraint;
use crate::ignore_ops::IgnoreOps;
use crate::ruleset::RuleSet;
use crate::{LightCnt, Lut, Rename};

/// Build lookup tables (`Lut`) for the left-hand side (`setA`) and right-hand
/// side (`setB`).  Each unique name is mapped to its index in the original
/// vector.
///
/// # Errors
/// Returns an error if a duplicate name is found in either input slice,
/// indicating which set (`setA` or `setB`) contained the clash.
/// ```
pub(super) fn build_luts(map_a: &[String], map_b: &[String]) -> Result<(Lut, Lut)> {
    let mut lut_a = Lut::default();
    let mut lut_b = Lut::default();

    for (lut, map, id) in [(&mut lut_a, &map_a, "setA"), (&mut lut_b, &map_b, "setB")] {
        for (idx, name) in map.iter().enumerate() {
            // `insert` returns the previous value, which we can use to detect dupes.
            if lut.insert(name.clone(), idx).is_some() {
                bail!("duplicate entry '{}'in {}", name, id);
            }
        }
    }
    Ok((lut_a, lut_b))
}

/// Transform a list of raw `ConstraintParse` objects into the final
/// `Constraint` representations used by the solver.
///
/// The conversion respects:
/// * Global ignore flags (`ignore`)
/// * Name-to-index lookup tables (`lut_a`, `lut_b`)
/// * Rule-set-driven behaviours (sorting, exclusion, etc.)
/// * Per-side rename tables (`rename_a`, `rename_b`)
///
/// Returns a tuple containing:
/// * `Vec<Constraint>` - the processed constraints in the same order as the
///   input (minus any that were ignored).
/// * `LightCnt` - the cumulative number of *known lights* added while processing,
///   useful for later bookkeeping.
///
/// # Errors
/// Propagates any error from `ConstraintParse::finalize_parsing` or from the
/// rule-set initialisation.
#[allow(clippy::too_many_arguments)]
pub(super) fn process_constraints(
    raw: Vec<ConstraintParse>,
    ignore: &IgnoreOps,
    lut_a: &Lut,
    lut_b: &Lut,
    rule_set: &RuleSet,
    rename_a: &Rename,
    rename_b: &Rename,
    map_b: &[String],
) -> Result<(Vec<Constraint>, LightCnt)> {
    let mut out = Vec::with_capacity(raw.len());
    let mut known_lights: LightCnt = 0;

    for cp in raw {
        if cp.ignore_on(ignore) {
            continue;
        }

        let added = cp.added_known_lights();
        let finished = cp.finalize_parsing(
            lut_a,
            lut_b,
            rule_set.constr_map_len(lut_a.len(), lut_b.len()),
            map_b,
            rule_set.must_add_exclude(),
            rule_set.must_sort_constraint(),
            (rename_a, rename_b),
            rule_set.init_data()?,
            known_lights,
        )?;
        out.push(finished);
        known_lights += added;
    }
    Ok((out, known_lights))
}

/// Apply rename mappings to the left‑hand (`map_a`) and right‑hand (`map_b`)
/// name vectors.  For each entry, the corresponding `Rename` table is consulted;
/// if a mapping exists the name is replaced, otherwise it stays unchanged.
///
/// This function mutates the supplied slices in place and does not allocate
/// new vectors.
/// ```
pub(super) fn apply_renames(
    map_a: &mut [String],
    map_b: &mut [String],
    rename_a: &Rename,
    rename_b: &Rename,
) {
    for name in map_a.iter_mut() {
        *name = rename_a.get(name).unwrap_or(name).to_owned();
    }
    for name in map_b.iter_mut() {
        *name = rename_b.get(name).unwrap_or(name).to_owned();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn build_luts_happy_path() -> Result<()> {
        let left = vec!["A".to_string(), "B".to_string()];
        let right = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        let (lut_a, lut_b) = build_luts(&left, &right)?;

        // Verify that each name maps to its index.
        assert_eq!(
            lut_a,
            vec![("A", 0), ("B", 1)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<Lut>()
        );
        assert_eq!(
            lut_b,
            vec![("a", 0), ("b", 1), ("c", 2)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<Lut>()
        );
        Ok(())
    }

    #[test]
    fn build_luts_detects_duplicate_in_set_a() {
        let left = vec!["dup".to_string(), "dup".to_string()];
        let right = vec!["unique".to_string()];

        let err = build_luts(&left, &right).expect_err("expected duplicate error");
        // The error message is produced by `bail!` inside the helper.
        assert!(
            err.to_string().contains("duplicate entry 'dup'"),
            "error message should mention the duplicated name"
        );
        assert!(
            err.to_string().contains("setA"),
            "error message should indicate the offending set"
        );
    }

    #[test]
    fn build_luts_detects_duplicate_in_set_b() {
        let left = vec!["unique".to_string()];
        let right = vec!["dup".to_string(), "dup".to_string()];

        let err = build_luts(&left, &right).expect_err("expected duplicate error");
        assert!(
            err.to_string().contains("duplicate entry 'dup'"),
            "error message should mention the duplicated name"
        );
        assert!(
            err.to_string().contains("setB"),
            "error message should indicate the offending set"
        );
    }

    #[test]
    fn apply_renames_renames_only_mapped_names() {
        // Input vectors - some names have a rename entry, others do not.
        let mut left = vec!["old_a".to_string(), "keep_a".to_string()];
        let mut right = vec!["keep_b".to_string(), "old_b".to_string()];

        // Build a tiny rename table: old_a -> new_a, old_b -> new_b.
        let rename = vec![("old_a", "new_a"), ("old_b", "new_b")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<Rename>();

        // The second rename table is empty (no right‑hand renames besides the one above).
        let empty_rename = Rename::default();

        // only rename the left side
        apply_renames(&mut left, &mut right, &rename, &empty_rename);

        // a on the left side was renamed
        assert_eq!(left, vec!["new_a".to_string(), "keep_a".to_string()]);
        // left side was not renamed (passed empty_rename)
        assert_eq!(right, vec!["keep_b".to_string(), "old_b".to_string()]);
    }
}

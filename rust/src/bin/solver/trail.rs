//! This module offers the functionality to make the order of MBs/MNs customizable

/// A struct for determine only whether a constraint-type should be a box or a night. This avoids
/// having to fill all the fields usually needed when creating a ConstraintType
#[derive(PartialEq)]
pub(super) enum CT {
    /// the shortform for [`ayto::constraint::ConstraintType::Box`]
    Box,
    /// the shortform for [`ayto::constraint::ConstraintType::Night`]
    Night,
}

/// a function which determines when which constraint type is generated
///
/// Idea is to make this configurable later on
pub(super) fn constraint_type_order(i: usize) -> CT {
    if i.is_multiple_of(2) {
        CT::Box
    } else {
        CT::Night
    }
}

//! This module implements helper-utilities to use when evaluating the series of constraints.

use anyhow::Result;

use crate::constraint::evaluate::ConstraintMerge;

pub(super) fn merge_constraints<T: ConstraintMerge + Clone>(constraints: &[T]) -> Result<Vec<T>> {
    let mut merged = vec![];
    let mut needs_merging = vec![];
    for c in constraints {
        if c.should_merge() {
            needs_merging.push(c);
            continue;
        }
        let mut d = c.to_owned();

        while let Some(to_merge) = needs_merging.pop() {
            d.merge(to_merge)?;
        }
        merged.push(d);
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Clone, Debug, PartialEq)]
    struct MockConstraint {
        value: i32,
        mergeable: bool,
    }

    impl ConstraintMerge for MockConstraint {
        fn should_merge(&self) -> bool {
            self.mergeable
        }

        fn merge(&mut self, other: &Self) -> Result<()> {
            self.value += other.value;
            Ok(())
        }
    }

    #[test]
    fn merge_constraints_simple() {
        let input: Vec<MockConstraint> = vec![];
        let result = merge_constraints(&input).unwrap();
        assert!(result.is_empty());

        let input = vec![
            MockConstraint {
                value: 1,
                mergeable: false,
            },
            MockConstraint {
                value: 2,
                mergeable: false,
            },
        ];
        let result = merge_constraints(&input).unwrap();
        assert_eq!(
            result,
            vec![
                MockConstraint {
                    value: 1,
                    mergeable: false
                },
                MockConstraint {
                    value: 2,
                    mergeable: false
                },
            ]
        );

        let input = vec![
            MockConstraint {
                value: 1,
                mergeable: true,
            },
            MockConstraint {
                value: 2,
                mergeable: true,
            },
            MockConstraint {
                value: 10,
                mergeable: false,
            },
        ];
        let result = merge_constraints(&input).unwrap();
        // LIFO merge: 10 + 2 + 1
        assert_eq!(
            result,
            vec![MockConstraint {
                value: 13,
                mergeable: false
            },]
        );

        let input = vec![
            MockConstraint {
                value: 1,
                mergeable: true,
            },
            MockConstraint {
                value: 10,
                mergeable: false,
            },
            MockConstraint {
                value: 2,
                mergeable: true,
            },
            MockConstraint {
                value: 20,
                mergeable: false,
            },
            MockConstraint {
                value: 2,
                mergeable: true,
            },
        ];
        let result = merge_constraints(&input).unwrap();
        assert_eq!(
            result,
            vec![
                MockConstraint {
                    value: 11,
                    mergeable: false
                },
                MockConstraint {
                    value: 22,
                    mergeable: false
                },
            ]
        );
    }
}

use std::collections::HashSet;
use std::io::Write;

use anyhow::Result;

use crate::matching_repr::MaskedMatching;

/// Write a Graphviz DOT representation of the tree to any `Write`.
///
/// This is the testable, generic helper. `data` are the possible partial solutions
/// (one `MaskedMatching` per leaf path). `ordering` controls which `A` indices are
/// serialized in which order (pairs of `(index, something)` — only the index is used).
///
/// `title` is placed in the graph label. `map_a`/`map_b` are used to render readable labels.
pub fn dot_tree<W: Write>(
    data: &Vec<MaskedMatching>,
    ordering: &Vec<(usize, usize)>,
    title: &str,
    writer: &mut W,
    map_a: &[String],
    map_b: &[String],
) -> Result<()> {
    let mut nodes: HashSet<String> = HashSet::new();
    writeln!(
        writer,
        "digraph D {{ labelloc=\"b\"; label=\"Stand: {}\"; ranksep=0.8;",
        title
    )?;
    for p in data {
        let mut parent = String::from("root");
        for (i, _) in ordering {
            let mut node = parent.clone();
            node.push('/');
            node.push_str(&format!("{:b}", p.slot_mask(*i).unwrap()));

            if nodes.insert(node.clone()) {
                // if node is new -- write node label or empty label
                if p.slot_mask(*i).unwrap().count() == 0 {
                    writeln!(writer, "\"{node}\"[label=\"\"]")?;
                } else {
                    writeln!(
                        writer,
                        "\"{node}\"[label=\"{}\"]",
                        map_a[*i].clone()
                            + "\\n"
                            + &p.slot_mask(*i)
                                .unwrap()
                                .iter()
                                .map(|b| map_b[b as usize].clone())
                                .collect::<Vec<_>>()
                                .join("\\n")
                    )?;
                }
                writeln!(writer, "\"{parent}\" -> \"{node}\";")?;
            }

            parent = node;
        }
    }
    writeln!(writer, "}}")?;
    Ok(())
}

/// Tells to how many different masks (i.1) someone from set_a (i.0) is mapped to.
/// The returned vector is sorted by the amount (i.1) ascending.
///
/// This is used to decide a sensible ordering for the tree layers.
pub fn tree_ordering(data: &[MaskedMatching], map_a: &[String]) -> Vec<(usize, usize)> {
    // tab maps people from set_a -> possible matches (set -> no duplicates)
    let mut tab = vec![HashSet::new(); map_a.len()];
    for p in data {
        for (i, js) in p.iter().enumerate() {
            // if js[0] != u8::MAX {
            tab[i].insert(js.0);
            // }
        }
    }

    // pairs people of set_a with amount of different matches
    let mut ordering: Vec<_> = tab
        .iter()
        .enumerate()
        .filter_map(|(i, x)| {
            if x.is_empty() || x.iter().all(|y| y.count_ones() == 1) {
                None
            } else {
                Some((i, x.len()))
            }
        })
        .collect();

    ordering.sort_unstable_by_key(|(_, x)| *x);
    ordering
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching_repr::bitset::Bitset;
    use crate::matching_repr::MaskedMatching;
    use std::io::Cursor;

    #[test]
    fn tree_ordering_detects_variable_slots() {
        // build two masked matchings such that index 0 has same mask, index 1 differs
        let p1 = MaskedMatching::from_masks(vec![Bitset::from_word(1), Bitset::from_word(2)]);
        let p2 = MaskedMatching::from_masks(vec![Bitset::from_word(1), Bitset::from_word(3)]);
        let data = vec![p1, p2];
        let map_a = vec!["A".to_string(), "B".to_string()];
        let ordering = tree_ordering(&data, &map_a);
        // only index 1 should appear since it has 2 different masks
        assert_eq!(ordering.len(), 1);
        assert_eq!(ordering[0].0, 1usize);
    }

    #[test]
    fn dot_tree_writer_emits_dot() {
        let p1 = MaskedMatching::from_masks(vec![Bitset::from_word(1), Bitset::from_word(2)]);
        let p2 = MaskedMatching::from_masks(vec![Bitset::from_word(1), Bitset::from_word(3)]);
        let data = vec![p1, p2];
        let map_a = vec!["A".to_string(), "B".to_string()];
        let map_b = vec![
            "b0".to_string(),
            "b1".to_string(),
            "b2".to_string(),
            "b3".to_string(),
        ];
        let ordering = tree_ordering(&data, &map_a);
        let mut buf = Cursor::new(Vec::<u8>::new());
        dot_tree(&data, &ordering, "TITLE", &mut buf, &map_a, &map_b).unwrap();
        let s = String::from_utf8(buf.into_inner()).unwrap();
        assert!(s.contains("digraph"));
        assert!(s.contains("TITLE"));
        // expect at least one arrow edge
        assert!(s.contains("->"));
    }
}

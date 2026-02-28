//! This module offers the functionality to generate a tree of a collection of matchings. For every
//! pairing it adds a node. The ordering of the levels so pairings with high probability are placed
//! higher and pairings with lower probability are placed lower.

use std::collections::HashSet;
use std::io::Write;

use anyhow::{Context, Result};

use crate::matching_repr::bitset::Bitset;
use crate::matching_repr::MaskedMatching;

/// This is the testable, generic helper. `data` are the possible partial solutions
/// (one `MaskedMatching` per leaf path). `ordering` controls which `A` indices are
/// serialized in which order (pairs of `(index, something)` - only the index is used).
///
/// `title` is placed in the graph label. `map_a`/`map_b` are used to render readable labels.
pub(crate) fn dot_tree<W: Write>(
    writer: &mut W,
    data: &[MaskedMatching],
    ordering: &[(usize, usize)],
    title: &str,
    map_a: &[String],
    map_b: &[String],
) -> Result<()> {
    write_header(writer, title)?;

    let mut builder = DotBuilder::new(writer, map_a, map_b);

    for p in data {
        let mut parent = "root".to_owned();
        for &(i, _) in ordering {
            let mask = p
                .slot_mask(i)
                .with_context(|| format!("slot {i} missing in matching"))?;
            let node = builder.ensure_node(&parent, i, mask)?;
            builder.write_edge(&parent, &node)?;
            parent = node;
        }
    }
    write_footer(writer)?;
    Ok(())
}

struct DotBuilder<'a, W: Write> {
    writer: &'a mut W,
    seen: HashSet<String>,
    map_a: &'a [String],
    map_b: &'a [String],
}

impl<'a, W: Write> DotBuilder<'a, W> {
    fn new(writer: &'a mut W, map_a: &'a [String], map_b: &'a [String]) -> Self {
        Self {
            writer,
            seen: HashSet::new(),
            map_a,
            map_b,
        }
    }

    /// Ensure a node exists and return its identifier.
    /// The method also records the edge from `parent` -> `node`
    /// **but does not write the edge yet** - it just stores it.
    fn ensure_node(&mut self, parent: &str, idx: usize, mask: &Bitset) -> Result<String> {
        let mut node = parent.to_string();
        node.push('/');
        node.push_str(&format!("{:b}", mask));

        if self.seen.insert(node.clone()) {
            // if node is new -- write node label or empty label
            if mask.count() == 0 {
                writeln!(self.writer, "\"{node}\"[label=\"\"]")?;
            } else {
                writeln!(
                    self.writer,
                    "\"{node}\"[label=\"{}\"]",
                    self.map_a[idx].clone()
                        + "\\n"
                        + &mask
                            .iter()
                            .map(|b| self.map_b[b as usize].clone())
                            .collect::<Vec<_>>()
                            .join("\\n")
                )?;
            }
        }
        Ok(node)
    }

    /// Flush a single edge after we know both ends.
    fn write_edge(&mut self, parent: &str, child: &str) -> Result<()> {
        writeln!(self.writer, "\"{parent}\" -> \"{child}\";")?;
        Ok(())
    }
}

fn write_header<W: Write>(writer: &mut W, title: &str) -> Result<()> {
    writeln!(
        writer,
        "digraph D {{ labelloc=\"b\"; label=\"Stand: {}\"; ranksep=0.8;",
        title
    )?;
    Ok(())
}

fn write_footer<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "}}")?;
    Ok(())
}

/// Tells to how many different masks (i.1) someone from set_a (i.0) is mapped to.
/// The returned vector is sorted by the amount (i.1) ascending.
///
/// This is used to decide a sensible ordering for the tree layers.
pub(crate) fn tree_ordering(data: &[MaskedMatching], map_a: &[String]) -> Vec<(usize, usize)> {
    // tab maps people from set_a -> possible matches (set -> no duplicates)
    let mut tab = vec![HashSet::new(); map_a.len()];
    for p in data {
        for (i, js) in p.iter().enumerate() {
            tab[i].insert(js.0);
        }
    }

    // pairs people of set_a with amount of different matches
    let mut ordering: Vec<_> = tab
        .iter()
        .enumerate()
        .filter_map(|(i, x)| {
            if x.is_empty() {
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

    use crate::matching_repr::MaskedMatching;

    use pretty_assertions::assert_eq;

    fn fixture_data() -> (
        Vec<MaskedMatching>, // data
        Vec<String>,         // map_a
        Vec<String>,         // map_b
    ) {
        let p1 = MaskedMatching::from_matching_ref(&[vec![0], vec![1]]);
        let p2 = MaskedMatching::from_matching_ref(&[vec![0], vec![0, 1]]);

        (
            vec![p1, p2],
            vec!["A", "B"]
                .into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c", "d"]
                .into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn write_header_produces_exact_output() -> Result<()> {
        let mut buf = Vec::new();
        write_header(&mut buf, "MyTitle")?;
        let got = String::from_utf8(buf)?;

        let expected = r#"digraph D { labelloc="b"; label="Stand: MyTitle"; ranksep=0.8;"#;

        assert_eq!(got, format!("{expected}\n"));

        Ok(())
    }

    #[test]
    fn write_footer_produces_exact_output() -> Result<()> {
        let mut buf = Vec::new();
        write_footer(&mut buf)?;
        let got = String::from_utf8(buf)?;

        assert_eq!(got, "}\n");

        Ok(())
    }

    #[test]
    fn ensure_node_writes_new_node_with_label() -> Result<()> {
        let (data, map_a, map_b) = fixture_data();
        let mask = data[0].slot_mask(1).context("mask missing")?;
        let mut buf = Vec::new();
        let mut builder = DotBuilder::new(&mut buf, &map_a, &map_b);

        let node_id = builder.ensure_node("root", 1, mask)?;
        let got = String::from_utf8(buf)?;

        let expected = "\"root/10\"[label=\"B\\nb\"]\n";

        assert_eq!(node_id, "root/10");
        assert_eq!(got, expected);

        Ok(())
    }

    #[test]
    fn ensure_node_does_not_duplicate_existing_node() -> Result<()> {
        let (data, map_a, map_b) = fixture_data();
        let mask = data[0].slot_mask(1).context("mask missing")?;

        let mut buf = Vec::new();
        let mut builder = DotBuilder::new(&mut buf, &map_a, &map_b);

        let node_id = builder.ensure_node("root", 1, mask)?;
        assert_eq!(node_id, "root/10");

        let node_id = builder.ensure_node("root", 1, mask)?;
        assert_eq!(node_id, "root/10");

        let got = String::from_utf8(buf)?;

        let expected = "\"root/10\"[label=\"B\\nb\"]\n";

        assert_eq!(got, expected);
        Ok(())
    }

    #[test]
    fn write_edge_outputs_correct_arrow() -> Result<()> {
        let mut buf = Vec::new();
        let mut builder = DotBuilder::new(&mut buf, &[], &[]);

        builder.write_edge("root", "root/10")?;
        let got = String::from_utf8(buf)?;

        let expected = "\"root\" -> \"root/10\";\n";

        assert_eq!(got, expected);

        Ok(())
    }

    #[test]
    fn tree_ordering_returns_expected_pairs() {
        let (data, map_a, _) = fixture_data();

        let ordering = tree_ordering(&data, &map_a);

        // node-id 0 has one instance
        // node-id 1 has two instances
        assert_eq!(ordering, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn dot_tree_produces_complete_dot_output() -> Result<()> {
        let (data, map_a, map_b) = fixture_data();
        let ordering = tree_ordering(&data, &map_a);

        let mut buf = Vec::new();
        dot_tree(&mut buf, &data, &ordering, "FULL_GRAPH", &map_a, &map_b)?;

        let got = String::from_utf8(buf)?;

        // Build the exact expected DOT representation.
        //
        // Header
        let expected = r#"digraph D { labelloc="b"; label="Stand: FULL_GRAPH"; ranksep=0.8;
"root/1"[label="A\na"]
"root" -> "root/1";
"root/1/10"[label="B\nb"]
"root/1" -> "root/1/10";
"root" -> "root/1";
"root/1/11"[label="B\na\nb"]
"root/1" -> "root/1/11";
}
"#;

        assert_eq!(got, expected);

        Ok(())
    }
}

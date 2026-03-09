// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module offers the functionality to generate a tree of a collection of matchings. For every
//! pairing it adds a node. The ordering of the levels so pairings with high probability are placed
//! higher and pairings with lower probability are placed lower.

use std::collections::HashSet;
use std::io::Write;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::matching_repr::bitset::Bitset;
use crate::matching_repr::{IdBase, MaskedMatching};
use crate::Lut;

/// Parsing struct for [`TreeConfig`], can be converted to this via the [`TreeConfigParse::finalize`] function
#[derive(Clone, Deserialize, Debug)]
pub(crate) struct TreeConfigParse {
    /// an id which can be used as a component in the filename
    id: String,
    /// something which can be shown as part of the title in the image
    #[serde(default)]
    title: String,
    /// these individuals from set_b will be ignored when drawing this tree
    #[serde(default, rename = "ignoreB")]
    ignore_b: Vec<String>,
    /// the layers associated with these individuals from set_a will be moved up right below the
    /// layers where the PM has already been found. The order specified here will be the order in
    /// the output.
    /// Especially useful when using ignore_b as tree_ordering is determined on the unfiltered
    /// matchings
    #[serde(default, rename = "moveUpA")]
    move_up_a: Vec<String>,
}

impl TreeConfigParse {
    /// convert the parsing struct to the real thing. Consumes itself.
    ///
    /// Needs the luts to convert the names to indices
    pub(crate) fn finalize(self, lut_a: &Lut, lut_b: &Lut) -> Result<TreeConfig> {
        let ignore_b = self
            .ignore_b
            .iter()
            .map(|b| {
                lut_b
                    .get(b)
                    .with_context(|| format!("{b} not found in lut_b for TreeConfig {}", self.id))
                    .map(|i| *i as IdBase)
            })
            .collect::<Result<Vec<_>>>()?;

        let move_up_a = self
            .move_up_a
            .iter()
            .map(|a| {
                lut_a
                    .get(a)
                    .with_context(|| format!("{a} not found in lut_a for TreeConfig {}", self.id))
                    .map(|i| *i as IdBase)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(TreeConfig {
            id: self.id,
            title: self.title,
            ignore_b,
            move_up_a,
        })
    }
}

/// A configuration for drawing a tree
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TreeConfig {
    /// an id which can be used as a component in the filename
    id: String,
    /// something which can be shown as part of the title in the image
    title: String,
    /// these individuals from set_b will be ignored when drawing this tree
    ignore_b: Vec<IdBase>,
    /// the layers associated with these individuals from set_a will be moved up right below the
    /// layers where the PM has already been found. The order specified here will be the order in
    /// the output.
    /// Especially useful when using ignore_b as tree_ordering is determined on the unfiltered
    /// matchings
    move_up_a: Vec<IdBase>,
}

impl TreeConfig {
    /// getter for the id of this config. Can be used as part of a filename/path
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// Visualize a list of MaskedMatchings as a tree. The ordering of the layers needs to be
    /// calculated beforehand.
    ///
    /// This only writes a `.dot` file. The file needs to be rendered to e.g. `pdf`/`png` manually
    /// afterwards.
    ///
    /// Arguments:
    /// - `writer` is where the `.dot` file is written to
    /// - `data` are the possible solutions (one [`crate::matching_repr::MaskedMatching`] per leaf path)
    /// - `ordering` controls the order of the layers/levels
    /// - `title` is placed in the graph label
    /// - `map_a`/`map_b` are used to render readable labels.
    pub(crate) fn dot_tree<W: Write>(
        &self,
        writer: &mut W,
        data: &[MaskedMatching],
        ordering: &[(IdBase, usize)],
        title: &str,
        map_a: &[String],
        map_b: &[String],
    ) -> Result<()> {
        if self.title.is_empty() {
            write_header(writer, title)?;
        } else {
            write_header(writer, &format!("{} | {}", title, self.title))?;
        }

        let ordering = ordering_move(ordering, &self.move_up_a);

        let mut builder = DotBuilder::new(writer, map_a, map_b);

        for p in data {
            let mut parent = "root".to_owned();
            for &(i, _) in ordering.iter() {
                let mut mask = *p
                    .slot_mask(i as usize)
                    .with_context(|| format!("slot {i} missing in matching"))?;
                for c in &self.ignore_b {
                    mask.clear_bit(*c);
                }
                let node = builder.ensure_node(&parent, i as usize, &mask)?;
                if node.0 {
                    builder.write_edge(&parent, &node.1)?;
                }
                parent = node.1;
            }
        }
        write_footer(writer)?;
        Ok(())
    }
}

/// A generic builder for .dot files in this context
struct DotBuilder<'a, W: Write> {
    /// the writer to which to write the .dot file to
    writer: &'a mut W,
    /// stores which nodes have already been seen (only draw nodes once)
    seen: HashSet<String>,
    /// convert idx_a to name
    map_a: &'a [String],
    /// convert idx_b to name
    map_b: &'a [String],
}

impl<'a, W: Write> DotBuilder<'a, W> {
    /// Create a new [`DotBuilder`] which writes to `W`
    ///
    /// - `map_a`/`map_b` the maps to convert indices to names
    fn new(writer: &'a mut W, map_a: &'a [String], map_b: &'a [String]) -> Self {
        Self {
            writer,
            seen: HashSet::new(),
            map_a,
            map_b,
        }
    }

    /// Ensure a node exists and return its identifier.
    ///
    /// If the node does not exist yet, this function creates the node in the `.dot` output (with
    /// the correct id and label).
    /// If the node did already exists this is indicated with the first value in the tuple
    ///
    /// Returns Ok((new:bool, nodeId/nodeName: String)) on success
    fn ensure_node(&mut self, parent: &str, idx: usize, mask: &Bitset) -> Result<(bool, String)> {
        let mut node = parent.to_string();
        node.push('/');
        node.push_str(&format!("{:b}", mask));

        let new = self.seen.insert(node.clone());

        if new {
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
        Ok((new, node))
    }

    /// Write an edge from `parent` to `child` in the `.dot` output.
    fn write_edge(&mut self, parent: &str, child: &str) -> Result<()> {
        writeln!(self.writer, "\"{parent}\" -> \"{child}\";")?;
        Ok(())
    }
}

/// Write the header for the .dot file to `W`
fn write_header<W: Write>(writer: &mut W, title: &str) -> Result<()> {
    writeln!(
        writer,
        "digraph D {{ labelloc=\"b\"; label=\"Stand: {}\"; ranksep=0.8;",
        title
    )?;
    Ok(())
}

/// Write the footer for the .dot file to `W`
fn write_footer<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "}}")?;
    Ok(())
}

/// Calculate an ordering for the layers/levels of the tree
///
/// A layer is identified by the id in `set_a`.
/// The layers are ordered so the amount of outgoing edges of a (complete) layer is minimized.
///
/// Returns a sorted list of layers to be drawn: [(id in set_a, num outgoing edges)]
pub(crate) fn tree_ordering(data: &[MaskedMatching], map_a: &[String]) -> Vec<(IdBase, usize)> {
    // tab maps people from set_a -> possible matches (set -> no duplicates)
    let mut tab = vec![HashSet::new(); map_a.len()];
    for p in data {
        for (i, js) in p.iter().enumerate() {
            tab[i].insert(js.as_word());
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
                Some((i as IdBase, x.len()))
            }
        })
        .collect();

    ordering.sort_unstable_by_key(|(_, x)| *x);
    ordering
}

/// adjust the ordering so the ids are moved up.
///
/// The resulting orering will be as follows:
/// 1. perfect matches (only one possibility left for this 1:1 match
/// 2. ids in the order from `ids`
/// 3. the remaining ones in the order from `ordering`
pub(crate) fn ordering_move(ordering: &[(IdBase, usize)], ids: &[IdBase]) -> Vec<(IdBase, usize)> {
    let mut ret = Vec::with_capacity(ordering.len());

    // insert the layers with just one outgoing edge
    for &(id, cnt) in ordering {
        if cnt <= 1 {
            ret.push((id, cnt));
        }
    }

    // insert the layers specified
    for id in ids {
        // obtain the item from ordering with the specified id
        if let Some(&(oid, cnt)) = ordering.iter().find(|(j, _)| j == id) {
            // ensure ret does not already contain this id
            if !ret.iter().any(|(r_id, _)| r_id == &oid) {
                ret.push((oid, cnt))
            }
        }
    }
    // insert the remaining elements
    for &(id, cnt) in ordering {
        // ensure ret does not already contain this id
        if !ret.iter().any(|(r_id, _)| r_id == &id) {
            ret.push((id, cnt));
        }
    }

    ret
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

        assert_eq!(node_id.1, "root/10");
        assert!(node_id.0);
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
        assert_eq!(node_id.1, "root/10");
        assert!(node_id.0);

        let node_id = builder.ensure_node("root", 1, mask)?;
        assert_eq!(node_id.1, "root/10");
        assert!(!node_id.0);

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
    fn finalize_ok() -> Result<()> {
        let cfg_p = TreeConfigParse {
            id: "abc".to_string(),
            title: "def".to_string(),
            ignore_b: vec!["a".to_string(), "c".to_string()],
            move_up_a: vec!["A".to_string(), "C".to_string(), "B".to_string()],
        };
        let lut_a = Lut::from_iter([
            ("A".to_string(), 0),
            ("B".to_string(), 1),
            ("C".to_string(), 2),
        ]);
        let lut_b = Lut::from_iter([
            ("a".to_string(), 0),
            ("b".to_string(), 1),
            ("c".to_string(), 2),
        ]);

        let cfg = cfg_p.finalize(&lut_a, &lut_b)?;

        let expected = TreeConfig {
            id: "abc".to_string(),
            title: "def".to_string(),
            ignore_b: vec![0, 2],
            move_up_a: vec![0, 2, 1],
        };

        assert_eq!(cfg, expected);
        Ok(())
    }

    #[test]
    fn finalize_err() -> Result<()> {
        let lut_a = Lut::from_iter([
            ("A".to_string(), 0),
            ("B".to_string(), 1),
            ("C".to_string(), 2),
        ]);
        let lut_b = Lut::from_iter([
            ("a".to_string(), 0),
            ("b".to_string(), 1),
            ("c".to_string(), 2),
        ]);

        let cfg_p = TreeConfigParse {
            id: "abc".to_string(),
            title: "def".to_string(),
            ignore_b: vec!["a".to_string(), "d".to_string()],
            move_up_a: vec!["A".to_string(), "C".to_string(), "B".to_string()],
        };
        assert!(cfg_p.finalize(&lut_a, &lut_b).is_err());

        let cfg_p = TreeConfigParse {
            id: "abc".to_string(),
            title: "def".to_string(),
            ignore_b: vec!["a".to_string(), "c".to_string()],
            move_up_a: vec!["A".to_string(), "D".to_string(), "B".to_string()],
        };
        assert!(cfg_p.finalize(&lut_a, &lut_b).is_err());

        Ok(())
    }

    #[test]
    fn dot_tree_produces_complete_dot_output() -> Result<()> {
        let data = vec![
            MaskedMatching::from_matching_ref(&[vec![3], vec![0, 1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![3], vec![2], vec![0, 1]]),
            MaskedMatching::from_matching_ref(&[vec![3], vec![4], vec![0, 1]]),
        ];

        let map_a = vec!["A", "B", "C"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let map_b = vec!["a", "b", "c", "d", "e"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();

        let ordering = tree_ordering(&data, &map_a);

        println!("{:?}", ordering);

        let cfg = TreeConfig {
            id: "abcI".to_string(),
            title: "abcT".to_string(),
            ignore_b: vec![1],
            move_up_a: vec![1],
        };

        let mut buf = Vec::new();
        cfg.dot_tree(&mut buf, &data, &ordering, "FULL_GRAPH", &map_a, &map_b)?;

        let got = String::from_utf8(buf)?;

        // Build the exact expected DOT representation.
        let expected = r#"digraph D { labelloc="b"; label="Stand: FULL_GRAPH | abcT"; ranksep=0.8;
"root/1000"[label="A\nd"]
"root" -> "root/1000";
"root/1000/1"[label="B\na"]
"root/1000" -> "root/1000/1";
"root/1000/1/100"[label="C\nc"]
"root/1000/1" -> "root/1000/1/100";
"root/1000/100"[label="B\nc"]
"root/1000" -> "root/1000/100";
"root/1000/100/1"[label="C\na"]
"root/1000/100" -> "root/1000/100/1";
"root/1000/10000"[label="B\ne"]
"root/1000" -> "root/1000/10000";
"root/1000/10000/1"[label="C\na"]
"root/1000/10000" -> "root/1000/10000/1";
}
"#;

        assert_eq!(got, expected);

        Ok(())
    }

    #[test]
    fn ordering_move_simple() {
        let ordering = vec![(10, 1), (5, 1), (11, 1), (1, 5), (3, 10), (2, 11), (4, 15)];

        // 99 does not exist -> ignored
        // 10 already inserted before -> ignore
        let o = ordering_move(&ordering, &[2, 3, 99, 10]);

        let expected = vec![(10, 1), (5, 1), (11, 1), (2, 11), (3, 10), (1, 5), (4, 15)];

        assert_eq!(o, expected)
    }
}

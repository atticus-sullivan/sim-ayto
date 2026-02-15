/*
sim_ayto
Copyright (C) 2025  Lukas Heindl

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use std::io::Write;
use std::{collections::HashSet, fs::File};

use anyhow::Result;

use crate::matching_repr::MaskedMatching;

pub fn dot_tree(
    data: &Vec<MaskedMatching>,
    ordering: &Vec<(usize, usize)>,
    title: &str,
    writer: &mut File,
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
            node.push_str(
                &format!("{:b}", p.slot_mask(*i).unwrap())
            );

            if nodes.insert(node.clone()) {
                // if node is new -- what???
                if p.slot_mask(*i).unwrap().count() == 0 {
                    writeln!(writer, "\"{node}\"[label=\"\"]")?;
                } else {
                    // only put content in that node if there is something meaning-full
                    // don't just skip the whole node since this would mess up the layering
                    writeln!(
                        writer,
                        "\"{node}\"[label=\"{}\"]",
                        map_a[*i].clone()
                            + "\\n"
                            + &p.slot_mask(*i).unwrap().iter()
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

/// Tells to how many different masks (i.1) someone from set_a (i.0) is mapped to
/// The vector is already sorted by the amount (i.1)
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

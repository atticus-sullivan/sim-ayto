// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements different ways to dump the remaining possible solutions

use crate::matching_repr::MaskedMatching;
use std::io;

/// select how the remaining possible solutions should be dumped
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum DumpMode {
    /// all full-matches (a -> bs) should be shown as ids
    Full,
    /// all full-matches (a -> bs) should be shown with the ids translated to names
    FullNames,
    /// all *winning* matchings (a -> b) should be shown as ids
    Winning,
    /// all *winning* matchings (a -> b) should be shown with the ids translated to names
    WinningNames,
}

impl DumpMode {
    /// dump the `left_poss` to `W` according to this DumpMode.
    ///
    /// Depending on the DumpMode, ids need to be translated to names, `map_a`/`map_b` will be used
    /// in this case.
    pub(super) fn dump<W: io::Write>(
        &self,
        mut out: W,
        left_poss: &[MaskedMatching],
        map_a: &[String],
        map_b: &[String],
    ) -> io::Result<()> {
        match self {
            DumpMode::Full => {
                for p in left_poss.iter() {
                    writeln!(out, "{:?}", p.prepare_debug_print())?;
                }
            }
            DumpMode::FullNames => {
                for p in left_poss.iter() {
                    writeln!(
                        out,
                        "{:?}",
                        p.prepare_debug_print_names(map_a, map_b)
                            .map_err(io::Error::other)?
                    )?;
                }
            }
            DumpMode::Winning => {
                for p in left_poss.iter() {
                    for pw in p.iter_unwrapped() {
                        writeln!(out, "{:?}", pw.prepare_debug_print())?;
                    }
                }
            }
            DumpMode::WinningNames => {
                for p in left_poss.iter() {
                    for pw in p.iter_unwrapped() {
                        writeln!(
                            out,
                            "{:?}",
                            pw.prepare_debug_print_names(map_a, map_b)
                                .map_err(io::Error::other)?
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn dump_full_simple() {
        let data = vec![
            MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![2], vec![1], vec![0]]),
            MaskedMatching::from_matching_ref(&[vec![0], vec![2, 3], vec![1]]),
        ];
        let mut buf = Vec::new();

        DumpMode::Full.dump(&mut buf, &data, &[], &[]).unwrap();

        let output = String::from_utf8(buf).unwrap();
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "[[0], [1], [2]]");
        assert_eq!(lines[1], "[[2], [1], [0]]");
        assert_eq!(lines[2], "[[0], [2, 3], [1]]");
        assert_eq!(lines.len(), 3)
    }

    #[test]
    fn dump_full_names_simple() {
        let map_a = vec!["A", "B", "C", "D", "E", "F"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let map_b = vec!["a", "b", "c", "d", "e", "f"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();

        let data = vec![
            MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![2], vec![1], vec![0]]),
            MaskedMatching::from_matching_ref(&[vec![0], vec![2, 3], vec![1]]),
        ];
        let mut buf = Vec::new();

        DumpMode::FullNames
            .dump(&mut buf, &data, &map_a, &map_b)
            .unwrap();

        let output = String::from_utf8(buf).unwrap();
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(
            lines[0],
            "[(\"A\", [\"a\"]), (\"B\", [\"b\"]), (\"C\", [\"c\"])]"
        );
        assert_eq!(
            lines[1],
            "[(\"A\", [\"c\"]), (\"B\", [\"b\"]), (\"C\", [\"a\"])]"
        );
        assert_eq!(
            lines[2],
            "[(\"A\", [\"a\"]), (\"B\", [\"c\", \"d\"]), (\"C\", [\"b\"])]"
        );
        assert_eq!(lines.len(), 3)
    }

    #[test]
    fn dump_winning_simple() {
        let map_a = vec!["A", "B", "C", "D", "E", "F"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let map_b = vec!["a", "b", "c", "d", "e", "f"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();

        let data = vec![
            MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![2], vec![1], vec![0]]),
            MaskedMatching::from_matching_ref(&[vec![0], vec![2, 3], vec![1]]),
        ];
        let mut buf = Vec::new();

        DumpMode::Winning
            .dump(&mut buf, &data, &map_a, &map_b)
            .unwrap();

        let output = String::from_utf8(buf).unwrap();
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "[[0], [1], [2]]");
        assert_eq!(lines[1], "[[2], [1], [0]]");
        assert_eq!(lines[2], "[[0], [2], [1]]");
        assert_eq!(lines[3], "[[0], [3], [1]]");
        assert_eq!(lines.len(), 4)
    }

    #[test]
    fn dump_winning_names_simple() {
        let map_a = vec!["A", "B", "C", "D", "E", "F"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let map_b = vec!["a", "b", "c", "d", "e", "f"]
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();

        let data = vec![
            MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![2]]),
            MaskedMatching::from_matching_ref(&[vec![2], vec![1], vec![0]]),
            MaskedMatching::from_matching_ref(&[vec![0], vec![2, 3], vec![1]]),
        ];
        let mut buf = Vec::new();

        DumpMode::WinningNames
            .dump(&mut buf, &data, &map_a, &map_b)
            .unwrap();

        let output = String::from_utf8(buf).unwrap();
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(
            lines[0],
            "[(\"A\", [\"a\"]), (\"B\", [\"b\"]), (\"C\", [\"c\"])]"
        );
        assert_eq!(
            lines[1],
            "[(\"A\", [\"c\"]), (\"B\", [\"b\"]), (\"C\", [\"a\"])]"
        );
        assert_eq!(
            lines[2],
            "[(\"A\", [\"a\"]), (\"B\", [\"c\"]), (\"C\", [\"b\"])]"
        );
        assert_eq!(
            lines[3],
            "[(\"A\", [\"a\"]), (\"B\", [\"d\"]), (\"C\", [\"b\"])]"
        );
        assert_eq!(lines.len(), 4)
    }
}

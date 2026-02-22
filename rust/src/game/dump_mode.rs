use crate::matching_repr::MaskedMatching;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum DumpMode {
    Full,
    FullNames,
    Winning,
    WinningNames,
}

impl DumpMode {
    pub(super) fn dump<W: std::io::Write>(&self, left_poss: &[MaskedMatching], map_a: &[String], map_b: &[String], mut out: W) -> std::io::Result<()> {
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
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn dump_full_simple() {
//         let data = vec![
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//         ];
//         let mut buf = Vec::new();
//
//         DumpMode::Full.dump(&data, &[], &[], &mut buf).unwrap();
//
//         let output = String::from_utf8(buf).unwrap();
//         let lines = output.lines().collect::<Vec<_>>();
//         assert_eq!(lines[0], "");
//         assert_eq!(lines[1], "");
//     }
//
//     #[test]
//     fn dump_full_names_simple() {
//         let map_a = vec!["A", "B", "C", "D", "E", "F"].into_iter().map(|x| x.to_string()).collect::<Vec<_>>();
//         let map_b = vec!["a", "b", "c", "d", "e", "f"].into_iter().map(|x| x.to_string()).collect::<Vec<_>>();
//
//         let data = vec![
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//         ];
//         let mut buf = Vec::new();
//
//         DumpMode::FullNames.dump(&data, &map_a, &map_b, &mut buf).unwrap();
//
//         let output = String::from_utf8(buf).unwrap();
//         let lines = output.lines().collect::<Vec<_>>();
//         assert_eq!(lines[0], "");
//         assert_eq!(lines[1], "");
//     }
//
//     #[test]
//     fn dump_winning_simple() {
//         let data = vec![
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//         ];
//         let mut buf = Vec::new();
//
//         DumpMode::WinningNames.dump(&data, &[], &[], &mut buf).unwrap();
//
//         let output = String::from_utf8(buf).unwrap();
//         let lines = output.lines().collect::<Vec<_>>();
//         assert_eq!(lines[0], "");
//         assert_eq!(lines[1], "");
//     }
//
//     #[test]
//     fn dump_winning_names_simple() {
//         let map_a = vec!["A", "B", "C", "D", "E", "F"].into_iter().map(|x| x.to_string()).collect::<Vec<_>>();
//         let map_b = vec!["a", "b", "c", "d", "e", "f"].into_iter().map(|x| x.to_string()).collect::<Vec<_>>();
//
//         let data = vec![
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//             MaskedMatching::from_matching_ref(&[vec![], vec![], vec![]]),
//         ];
//         let mut buf = Vec::new();
//
//         DumpMode::WinningNames.dump(&data, &map_a, &map_b, &mut buf).unwrap();
//
//         let output = String::from_utf8(buf).unwrap();
//         let lines = output.lines().collect::<Vec<_>>();
//         assert_eq!(lines[0], "");
//         assert_eq!(lines[1], "");
//     }
// }

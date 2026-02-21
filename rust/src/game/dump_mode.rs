use crate::matching_repr::MaskedMatching;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum DumpMode {
    Full,
    FullNames,
    Winning,
    WinningNames,
}

impl DumpMode {
    pub(super) fn dump(&self, left_poss: &[MaskedMatching], map_a: &[String], map_b: &[String]) {
        match self {
            DumpMode::Full => {
                for p in left_poss.iter() {
                    println!("{:?}", p.prepare_debug_print())
                }
            }
            DumpMode::FullNames => {
                for p in left_poss.iter() {
                    println!(
                        "{:?}",
                        p.prepare_debug_print_names(map_a, map_b)
                    );
                }
            }
            DumpMode::Winning => {
                for p in left_poss.iter() {
                    for pw in p.iter_unwrapped() {
                        println!("{:?}", pw.prepare_debug_print());
                    }
                }
            }
            DumpMode::WinningNames => {
                for p in left_poss.iter() {
                    for pw in p.iter_unwrapped() {
                        println!(
                            "{:?}",
                            pw.prepare_debug_print_names(map_a, map_b)
                        );
                    }
                }
            }
        }
    }
}

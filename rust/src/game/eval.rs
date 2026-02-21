/// TODO: write comment here
/// everything here is just orchestration -> no tests needed
use anyhow::Result;

use std::fs::File;

use crate::constraint::Constraint;
use crate::game::eval_utils::{gen_report_data, merge_constraints, MdTable, Trail};
use crate::game::{query_matchings, query_pairs, DumpMode};
use crate::game::Game;
use crate::iterstate::IterState;

impl Game {
    /// This function orchestrates the complete evaluation, reporting + comparison preparation
    pub fn eval(
        &mut self,
        print_transposed: bool,
        dump_mode: Option<DumpMode>,
        full: bool,
        is: &IterState,
        no_tree_output: bool,
    ) -> Result<()> {
        // EVALUATION
        // preprocess the constraints for printing
        let mut constraints = merge_constraints(&is.constraints)?;
        // process the constraints and derive the tables with how often each matching occurs
        let report_data = gen_report_data(&mut constraints, (is.each.clone(), is.total))?;

        // REPORT
        self.report(print_transposed, full, is, no_tree_output, report_data)?;
        self.report_finalize(dump_mode, &constraints, is)?;

        // COMPARISON
        // this is gethering data for a comparison at a later point in time
        let solutions = is.keep_rem.then_some(&is.left_poss);
        self.do_statistics(is.total as f64, &constraints, solutions)?;

        Ok(())
    }

    /// This does all the reporting based on the Trail => Generates the report for the trail of the
    /// constraints
    /// (other parts of the report are split off, so they can borrow the constraints again)
    fn report(
        &mut self,
        print_transposed: bool,
        full: bool,
        is: &IterState,
        no_tree_output: bool,
        data: Trail,
    ) -> Result<()> {
        // track table indices
        let mut tab_idx = 0;
        let mut md_tables: Vec<MdTable> = vec![];

        // generate additional tables
        {
            let m_data = query_matchings::MatchingReport::new(&is.query_matchings, &self.map_a, &self.map_b)?;
            if let Some(m_data) = m_data {
                println!("{m_data}");
                // need to generate an "offset" so the generated pngs match the numbers used in the
                // markdown code
                tab_idx += m_data.tab_cnt();
            }
            let p_data = query_pairs::QueryPairReport::new(&is.query_pair, &self.map_a, &self.map_b)?;
            print!("{p_data}");
        }

        // this function prints the report which was generated before
        // it also collects the tables which shall be included in the markdown file, for this it
        // appends to md_tables
        self.gen_report(&data, print_transposed, full, no_tree_output, tab_idx, &mut md_tables)?;

        let md_path = self.dir.join(self.stem.clone()).with_extension("md");
        self.write_page_md(&mut File::create(md_path.clone())?, &md_tables)?;

        Ok(())
    }

    /// Writes the final part of the report.
    ///
    /// Split off so it can borrow the constraints again
    fn report_finalize(
        &mut self,
        dump_mode: Option<DumpMode>,
        constraints: &[Constraint],
        is: &IterState,
        ) -> Result<()> {
        if let Some(d) = dump_mode {
            d.dump(&is.left_poss, &self.map_a, &self.map_b);
        }

        println!("{}", self.summary_table(false, constraints)?);
        println!("{}", self.summary_table(true, constraints)?);

        println!(
            "Total permutations: {}  Permutations left: {}  Initial combinations for each pair: {}",
            is.total,
            is.total - is.eliminated,
            is.each[0][0]
        );

        Ok(())
    }
}

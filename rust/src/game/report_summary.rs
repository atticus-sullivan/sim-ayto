use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::{UTF8_FULL_CONDENSED};
use comfy_table::{Cell, Table};

use anyhow::Result;

use crate::constraint::Constraint;
use crate::game::Game;

impl Game {
    pub(super) fn summary_table(&self, transpose: bool, merged_constraints: &[Constraint]) -> Result<Table> {
        // let map_vert;
        let map_hor = if !transpose {
            &self.map_a
            // map_vert = &self.map_b;
        } else {
            &self.map_b
            // map_vert = &self.map_a;
        };

        let mut hdr = vec![
            Cell::new(""),
            Cell::new("L").set_alignment(comfy_table::CellAlignment::Center),
        ];
        hdr.extend(
            map_hor
                .iter()
                .map(|x| Cell::new(x).set_alignment(comfy_table::CellAlignment::Center)),
        );
        hdr.push(Cell::new("").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("I").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("#new").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("min dist").set_alignment(comfy_table::CellAlignment::Center));

        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);

        let mut past_constraints: Vec<&Constraint> = Vec::default();
        for (i, c) in merged_constraints.iter().enumerate() {
            let row = c.summary_row_data(transpose, map_hor, &past_constraints);
            let style = if i % 2 == 0 {
                |cell: Cell| cell.bg(crate::COLOR_ALT_BG)
            } else {
                |cell: Cell| cell
            };
            table.add_row(row.render(style));

            past_constraints.push(c);
        }
        Ok(table)
    }
}

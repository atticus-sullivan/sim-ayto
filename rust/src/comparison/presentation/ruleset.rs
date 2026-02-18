/// This module allows rendering an overview over the different rulesets used for the various
/// seasons.

use crate::comparison::data::CmpData;

/// Render ruleset metadata as a Markdown table.
///
/// `cmp_data` is the comparison data per ruleset; the returned string is a
/// Markdown table (pipe-separated) ready for insertion into the site content.
pub fn tab_md(cmp_data: &Vec<(String, CmpData)>) -> String {
    let mut tab_lines = vec![
        r#"| {{< i18n "season" >}} | {{< i18n "players" >}} | {{< i18n "rulesetShort" >}} | {{< i18n "rulesetDesc" >}} |"#.to_owned(),
        "| --- | --- |:---:| --- |".to_owned(),
    ];

    for (name, cd) in cmp_data {
        let r = cd.game.ruleset_str();
        tab_lines.push(format!(
            "| {} | {} | {} | {{{{< i18n \"{}\" >}}}} |",
            name,
            cd.game.players_str(),
            r.1,
            r.0
        ));
    }
    tab_lines[2..].sort();
    tab_lines.join("\n")
}

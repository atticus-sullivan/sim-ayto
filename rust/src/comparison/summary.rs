use std::collections::HashMap;

use crate::{comparison::CmpData, constraint::eval::SumCounts};

pub fn summary_tab_md(cmp_data: &HashMap<String, CmpData>) -> String {
    let mut total_counts = SumCounts {
        blackouts: 0,
        sold: 0,
        sold_but_match: 0,
        sold_but_match_active: true,
        matches_found: 0,
        won: false,
    };

    let mut tab_lines = vec![
        r#"| {{{{< i18n "season" >}}}} | {{{{< i18n "blackouts" >}}}} | {{{{< i18n "sold" >}}}} | {{{{< i18n "soldButGood" >}}}} | {{{{< i18n "matchesFound" >}}}} | {{{{< i18n "won" >}}}} |"#.to_owned(),
        "| --- | --- | --- | --- | --- | --- |".to_owned(),
    ];

    for (name, cd) in cmp_data {
        tab_lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            name,
            cd.cnts.blackouts,
            cd.cnts.sold,
            if cd.cnts.sold_but_match_active {
                cd.cnts.sold_but_match.to_string()
            } else {
                "(0)".to_string()
            },
            cd.cnts.matches_found,
            cd.cnts.won,
        ));
        total_counts.add(&cd.cnts);
    }
    tab_lines[2..].sort();

    tab_lines.push("| | | | | | |".to_string());
    tab_lines.push(format!(
        "| {} | {} | {} | {} | {} | |",
        "{{< i18n \"total\" >}}",
        total_counts.blackouts,
        total_counts.sold,
        if total_counts.sold_but_match_active {
            total_counts.sold_but_match.to_string()
        } else {
            "(0)".to_string()
        },
        total_counts.matches_found,
    ));

    tab_lines.join("\n")
}

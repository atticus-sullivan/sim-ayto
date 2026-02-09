use crate::{
    comparison::{CmpData, Language},
    constraint::eval::SumCounts,
};
use num_format::ToFormattedString;

pub fn summary_tab_md(cmp_data: &Vec<(String, CmpData)>, lang: Language) -> String {
    let mut total_counts = SumCounts {
        blackouts: 0,
        sold: 0,
        sold_but_match: 0,
        sold_but_match_active: true,
        matches_found: 0,
        won: false,
        offers_noted: true,
        offer_and_match: 0,
        offered_money: 0,
        offers: 0,
    };

    let mut tab_lines = vec![
        r#"| {{< i18n "season" >}} | {{< i18n "won" >}} | {{< i18n "matchesFound" >}} | {{< i18n "blackouts" >}} | | {{< i18n "sold" >}} | {{< i18n "soldButGood" >}} | | {{< i18n "offers" >}} | {{< i18n "offerAndMatch" >}} | {{< i18n "offeredMoney" >}} |"#.to_owned(),
        "| --- |:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:| ---:|".to_owned(),
    ];

    for (name, cd) in cmp_data {
        tab_lines.push(format!(
            "| {} | {{{{< badge content=\"{}\" color=\"{}\" >}}}} | {} | {} | | {} | {} | | {} | {} | {} |",
            name,

            lang.format_bool_yes_no(cd.cnts.won),
            if cd.cnts.won { "green" } else { "red" },

            cd.cnts.matches_found,
            cd.cnts.blackouts,

            cd.cnts.sold,
            if cd.cnts.sold_but_match_active {
                cd.cnts.sold_but_match.to_string()
            } else {
                "".to_string()
            },

            if cd.cnts.offers_noted {
                cd.cnts.offers.to_string()
            } else {
                "".to_string()
            },
            if cd.cnts.offers_noted {
                cd.cnts.offer_and_match.to_string()
            } else {
                "".to_string()
            },
            if cd.cnts.offers_noted {
                cd.cnts.offered_money.to_formatted_string(&lang.number_formatting())
            } else {
                "".to_string()
            },
        ));
        total_counts.add(&cd.cnts);
    }
    tab_lines[2..].sort();

    tab_lines.push("| | | | | | | | | | | |".to_string());
    tab_lines.push(format!(
        "| {} | | {} | {} | | {} | {} | | {} | {} | {} |",
        "{{< i18n \"total\" >}}",
        total_counts.matches_found,
        total_counts.blackouts,
        total_counts.sold,
        if total_counts.sold_but_match_active {
            total_counts.sold_but_match.to_string()
        } else {
            "".to_string()
        },
        if total_counts.offers_noted {
            total_counts.offers.to_string()
        } else {
            "".to_string()
        },
        if total_counts.offers_noted {
            total_counts.offer_and_match.to_string()
        } else {
            "".to_string()
        },
        if total_counts.offers_noted {
            total_counts
                .offered_money
                .to_formatted_string(&lang.number_formatting())
        } else {
            "".to_string()
        },
    ));

    tab_lines.join("\n")
}

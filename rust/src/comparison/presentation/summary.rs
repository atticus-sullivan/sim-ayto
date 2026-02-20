/// This module renders overviews over complete seasons.
use num_format::ToFormattedString;

use crate::comparison::data::CmpData;
use crate::comparison::Language;
use crate::constraint::compare::{SumCounts, SumOffersMB, SumOffersMN};

/// Build a summary Markdown tab for all rulesets.
///
/// `cmp_data` is pairs of `(ruleset_name, CmpData)`. `lang` controls the i18n
/// labels (e.g. `Language::De` / `Language::En`). Returns a Markdown table string.
pub(crate) fn tab_md(cmp_data: &Vec<(String, CmpData)>, lang: Language) -> String {
    let mut total_counts = SumCounts {
        solvable_after: None,
        blackouts: 0,
        matches_found: 0,
        won: false,
        offers_mb: SumOffersMB {
            sold_cnt: 0,
            sold_but_match: 0,
            sold_but_match_active: true,
            offers_noted: true,
            offer_and_match: 0,
            offers: 0,
            offered_money: 0,
        },
        offers_mn: SumOffersMN {
            sold_cnt: 0,
            offers_noted: true,
            offers: 0,
            offered_money: 0,
        },
    };

    let mut tab_lines = vec![
        r#"| {{< i18n "season" >}} | {{< i18n "won" >}} | {{< i18n "solvable" >}} | {{< i18n "matchesFound" >}} | {{< i18n "blackouts" >}} | | {{< i18n "sold" >}} | {{< i18n "soldButGood" >}} | | {{< i18n "offers" >}} | {{< i18n "offerAndMatch" >}} | {{< i18n "offeredMoney" >}} |"#.to_owned(),
        "| --- |:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:| ---:|".to_owned(),
    ];

    for (name, cd) in cmp_data {
        tab_lines.push(format!(
            "| {} | {{{{< badge content=\"{}\" color=\"{}\" >}}}} | {{{{< badge content=\"{}\" color=\"{}\" >}}}} | {} | {} | | {} / {} | {} | | {} | {} | {} / {} |",
            name,

            lang.format_bool_yes_no(cd.cnts.won),
            if cd.cnts.won { "green" } else { "red" },
            if let Some(solv) = &cd.cnts.solvable_after {
                solv.1.clone()
            } else { lang.format_bool_yes_no(false).to_string() },
            if let Some(solv) = &cd.cnts.solvable_after {
                if solv.0 { "green" } else { "red" }
            } else { "red" },

            cd.cnts.matches_found,
            cd.cnts.blackouts,

            cd.cnts.offers_mb.sold_cnt,
            cd.cnts.offers_mn.sold_cnt,
            if cd.cnts.offers_mb.sold_but_match_active {
                cd.cnts.offers_mb.sold_but_match.to_string()
            } else {
                "".to_string()
            },

            if cd.cnts.offers_mb.offers_noted {
                cd.cnts.offers_mb.offers.to_string()
            } else {
                "".to_string()
            },
            if cd.cnts.offers_mb.offers_noted {
                cd.cnts.offers_mb.offer_and_match.to_string()
            } else {
                "".to_string()
            },
            if cd.cnts.offers_mb.offers_noted {
                cd.cnts.offers_mb.offered_money.to_formatted_string(&lang.number_formatting())
            } else {
                "".to_string()
            },
            if cd.cnts.offers_mn.offers_noted {
                cd.cnts.offers_mn.offered_money.to_formatted_string(&lang.number_formatting())
            } else {
                "".to_string()
            },
        ));
        total_counts.add(&cd.cnts);
    }
    tab_lines[2..].sort();

    tab_lines.push("| | | | | | | | | | | | |".to_string());
    tab_lines.push(format!(
        "| {} | | | {} | {} | | {} / {} | {} | | {} | {} | {} / {} |",
        "{{< i18n \"total\" >}}",
        total_counts.matches_found,
        total_counts.blackouts,
        total_counts.offers_mb.sold_cnt,
        total_counts.offers_mn.sold_cnt,
        if total_counts.offers_mb.sold_but_match_active {
            total_counts.offers_mb.sold_but_match.to_string()
        } else {
            "".to_string()
        },
        if total_counts.offers_mb.offers_noted {
            total_counts.offers_mb.offers.to_string()
        } else {
            "".to_string()
        },
        if total_counts.offers_mb.offers_noted {
            total_counts.offers_mb.offer_and_match.to_string()
        } else {
            "".to_string()
        },
        if total_counts.offers_mb.offers_noted {
            total_counts
                .offers_mb
                .offered_money
                .to_formatted_string(&lang.number_formatting())
        } else {
            "".to_string()
        },
        if total_counts.offers_mn.offers_noted {
            total_counts
                .offers_mn
                .offered_money
                .to_formatted_string(&lang.number_formatting())
        } else {
            "".to_string()
        },
    ));

    tab_lines.join("\n")
}

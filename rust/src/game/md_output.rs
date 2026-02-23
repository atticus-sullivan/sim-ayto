/// This module facilitates writing a markdown file which includes tables contained in the terminal
/// output. The markdown rendering depends on that terminal output split into the various tables
/// and converted to png files as specified. Similarly the tree (dot) files must have been
/// generated and rendered to png files.
use std::io::Write;

use anyhow::Result;

use crate::game::report_trail::MdTable;
use crate::game::Game;

impl Game {
    /// Write the main markdown output file (frontmatter + images/tabs).
    ///
    /// `md_tables` describes which generated plots / images will be embedded in the page.
    pub(super) fn write_page_md<W: Write>(&self, mut out: W, md_tables: &[MdTable]) -> Result<()> {
        writeln!(out, "---")?;
        writeln!(out, "{}", serde_yaml::to_string(&self.frontmatter)?)?;
        writeln!(out, "---")?;

        let stem = &self.stem;

        writeln!(out, "\n{{{{% translateHdr \"tab-current\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
        writeln!(out, "{{{{% details closed=\"true\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/{stem}/{stem}_tab.png\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/{stem}/{stem}_sum.png\" %}}}}")?;
        writeln!(out, "{{{{% /details %}}}}")?;

        writeln!(out, "\n{{{{% translateHdr \"tab-individual\" %}}}}")?;
        for tab in md_tables.iter() {
            if tab.detail {
                writeln!(
                    out,
                    "\n{{{{% details title=\"{}\" closed=\"true\" %}}}}",
                    tab.name
                )?;
            } else {
                writeln!(out, "\n{{{{% translatedDetails \"{}\" %}}}}", tab.name)?;
            }

            writeln!(
                out,
                "{{{{% img src=\"/{stem}/{stem}_{}.png\" %}}}}",
                tab.idx
            )?;
            if tab.tree {
                writeln!(
                    out,
                    "{{{{% img src=\"/{stem}/{stem}_{}_tree.png\" %}}}}",
                    tab.idx
                )?;
            }

            if tab.detail {
                writeln!(out, "{{{{% /details %}}}}")?;
            } else {
                writeln!(out, "{{{{% /translatedDetails %}}}}")?;
            }
        }

        writeln!(out, "\n{{{{% translateHdr \"tab-everything\" %}}}}\n:warning: {{{{< i18n \"spoiler-warning\" >}}}} :warning:")?;
        writeln!(out, "{{{{% details closed=\"true\" %}}}}")?;
        writeln!(out, "{{{{% img src=\"/{stem}/{stem}.col.png\" %}}}}")?;
        writeln!(out, "{{{{% /details %}}}}")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn write_page_md_empty() {
        let game = Game {
            stem: "stem".to_string(),
            frontmatter: serde_yaml::from_str("title: abc").unwrap(),
            ..Default::default()
        };
        let tabs = vec![];
        let mut buf = vec![];
        game.write_page_md(&mut buf, &tabs).unwrap();
        let output = String::from_utf8(buf).unwrap();

        let expected = r#"---
title: abc

---

{{% translateHdr "tab-current" %}}
:warning: {{< i18n "spoiler-warning" >}} :warning:
{{% details closed="true" %}}
{{% img src="/stem/stem_tab.png" %}}
{{% img src="/stem/stem_sum.png" %}}
{{% /details %}}

{{% translateHdr "tab-individual" %}}

{{% translateHdr "tab-everything" %}}
:warning: {{< i18n "spoiler-warning" >}} :warning:
{{% details closed="true" %}}
{{% img src="/stem/stem.col.png" %}}
{{% /details %}}
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn write_page_md_simple() {
        let game = Game {
            stem: "stem".to_string(),
            frontmatter: serde_yaml::from_str("title: abc").unwrap(),
            ..Default::default()
        };
        let tabs = vec![
            MdTable {
                name: "a".to_string(),
                idx: 1,
                tree: true,
                detail: true,
            },
            MdTable {
                name: "z".to_string(),
                idx: 10,
                tree: false,
                detail: false,
            },
            MdTable {
                name: "x".to_string(),
                idx: 23,
                tree: false,
                detail: true,
            },
        ];
        let mut buf = vec![];
        game.write_page_md(&mut buf, &tabs).unwrap();
        let output = String::from_utf8(buf).unwrap();

        let expected = r#"---
title: abc

---

{{% translateHdr "tab-current" %}}
:warning: {{< i18n "spoiler-warning" >}} :warning:
{{% details closed="true" %}}
{{% img src="/stem/stem_tab.png" %}}
{{% img src="/stem/stem_sum.png" %}}
{{% /details %}}

{{% translateHdr "tab-individual" %}}

{{% details title="a" closed="true" %}}
{{% img src="/stem/stem_1.png" %}}
{{% img src="/stem/stem_1_tree.png" %}}
{{% /details %}}

{{% translatedDetails "z" %}}
{{% img src="/stem/stem_10.png" %}}
{{% /translatedDetails %}}

{{% details title="x" closed="true" %}}
{{% img src="/stem/stem_23.png" %}}
{{% /details %}}

{{% translateHdr "tab-everything" %}}
:warning: {{< i18n "spoiler-warning" >}} :warning:
{{% details closed="true" %}}
{{% img src="/stem/stem.col.png" %}}
{{% /details %}}
"#;

        assert_eq!(output, expected);
    }
}

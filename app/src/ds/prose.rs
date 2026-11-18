//! Long-form text (licences), kept readable.

use dioxus::prelude::*;

/// A block of Markdown-lite: "## " headings, paragraphs (wrapped lines are
/// joined, `backticks` dropped), and 4-space indented preformatted text.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading(String),
    Para(String),
    Pre(String),
}

pub fn blocks(text: &str) -> Vec<Block> {
    let mut out = vec![];
    for chunk in text
        .split("\n\n")
        .map(|c| c.trim_end_matches('\n'))
        .filter(|c| !c.trim().is_empty())
    {
        if let Some(h) = chunk.trim_start().strip_prefix("## ") {
            out.push(Block::Heading(h.trim().to_string()));
        } else if chunk
            .lines()
            .all(|l| l.starts_with("    ") || l.trim().is_empty())
        {
            let pre: Vec<&str> = chunk
                .lines()
                .map(|l| l.strip_prefix("    ").unwrap_or(l))
                .collect();
            match out.last_mut() {
                // Consecutive indented chunks (separated by blank lines) form one block.
                Some(Block::Pre(prev)) => {
                    prev.push_str("\n\n");
                    prev.push_str(&pre.join("\n"));
                }
                _ => out.push(Block::Pre(pre.join("\n"))),
            }
        } else {
            let para = chunk
                .lines()
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(" ")
                .replace('`', "");
            out.push(Block::Para(para));
        }
    }
    out
}

#[component]
pub fn Prose(#[props(into)] text: String) -> Element {
    rsx! {
        div { class: "prose",
            for (i, block) in blocks(&text).into_iter().enumerate() {
                match block {
                    Block::Heading(h) => rsx! { h2 { key: "{i}", class: "prose__heading", "{h}" } },
                    Block::Para(p) => rsx! { p { key: "{i}", class: "prose__body", "{p}" } },
                    Block::Pre(t) => rsx! { pre { key: "{i}", class: "prose__pre", "{t}" } },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn splits_markdown_lite_into_blocks() {
        let text = "Intro line one\nline two.\n\n## Fonts: here\n\nA `code` word.\n\n    Copyright (c) X\n      indented more\n";
        assert_eq!(
            blocks(text),
            vec![
                Block::Para("Intro line one line two.".into()),
                Block::Heading("Fonts: here".into()),
                Block::Para("A code word.".into()),
                Block::Pre("Copyright (c) X\n  indented more".into()),
            ]
        );
    }

    #[test]
    fn renders_headings_and_preformatted_blocks() {
        let html = render(move || rsx! { Prose { text: "## Data\n\n    licence text\n" } });
        assert!(
            html.contains("<h2")
                && html.contains("Data")
                && html.contains("<pre")
                && html.contains("licence text"),
            "{html}"
        );
        assert!(!html.contains("##"), "{html}");
    }

    #[test]
    fn renders_text() {
        let html = render(move || rsx! { Prose { text: "Line one\nLine two" } });
        assert!(
            html.contains("prose") && html.contains("Line one Line two"),
            "{html}"
        );
    }
}

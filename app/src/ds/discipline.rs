//! Guards for the design system: only tokens.css may hold raw values.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    fn styles() -> Vec<(String, String)> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut out = vec![(
            "assets/components.css".to_string(),
            fs::read_to_string(root.join("assets/components.css")).unwrap(),
        )];
        for dir in fs::read_dir(root.join("src/components")).unwrap().flatten() {
            let p = dir.path().join("style.css");
            if let Ok(css) = fs::read_to_string(&p) {
                out.push((p.strip_prefix(root).unwrap().display().to_string(), css));
            }
        }
        out
    }

    /// CSS with comments removed.
    fn code(css: &str) -> String {
        let mut out = String::new();
        let mut rest = css;
        while let Some(i) = rest.find("/*") {
            out.push_str(&rest[..i]);
            rest = rest[i..]
                .find("*/")
                .map(|j| &rest[i + j + 2..])
                .unwrap_or("");
        }
        out + rest
    }

    #[test]
    fn no_raw_colours_outside_tokens() {
        for (name, css) in styles() {
            let css = code(&css);
            for bad in ["#", "rgb(", "rgba(", "hsl("] {
                let hits: Vec<&str> = css
                    .lines()
                    .filter(|l| l.contains(bad) && !l.trim_start().starts_with('['))
                    .collect();
                assert!(hits.is_empty(), "{name} has raw colour ({bad}): {hits:?}");
            }
        }
    }

    /// Nothing in the app scrolls sideways: lists scroll down, rows wrap.
    #[test]
    fn nothing_scrolls_sideways() {
        for (name, css) in styles() {
            let css = code(&css);
            let hits: Vec<&str> = css
                .lines()
                .filter(|l| {
                    let l: String = l.chars().filter(|c| !c.is_whitespace()).collect();
                    [
                        "overflow:auto",
                        "overflow:scroll",
                        "overflow-x:auto",
                        "overflow-x:scroll",
                    ]
                    .iter()
                    .any(|bad| l.contains(bad))
                })
                .collect();
            assert!(hits.is_empty(), "{name} can scroll sideways: {hits:#?}");
        }
    }

    /// The crossword's colours are set once, outside the light and dark
    /// palettes, so the grid looks the same in both.
    #[test]
    fn grid_colours_are_the_same_in_light_and_dark() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let tokens = code(&fs::read_to_string(root.join("assets/tokens.css")).unwrap());
        for token in [
            "--grid-paper:",
            "--grid-block:",
            "--grid-ink:",
            "--grid-number:",
            "--grid-answer:",
            "--grid-current:",
            "--grid-current-ink:",
            "--grid-break:",
        ] {
            assert_eq!(
                tokens.matches(token).count(),
                1,
                "{token} must be set exactly once"
            );
            // Set in the shared block, after every palette.
            let at = tokens.find(token).unwrap();
            let palettes = tokens.rfind("color-scheme: dark").unwrap();
            assert!(at > palettes, "{token} is inside a light or dark palette");
        }
        // And the grid uses nothing but its own colours.
        let css = code(&fs::read_to_string(root.join("assets/components.css")).unwrap());
        let start = css.find(".xw-grid {").expect("the grid's styles");
        let end = start + css[start..].find("\n\n").unwrap();
        let grid = &css[start..end];
        for themed in [
            "--canvas",
            "--surface",
            "--raised",
            "--line",
            "--text",
            "--muted",
            "--faint",
            "--invert",
            "--p1",
            "--p2",
            "--p3",
            "--p4",
            "--p5",
            "--def",
            "--ind",
            "--danger",
            "--shadow",
        ] {
            assert!(
                !grid.contains(&format!("var({themed})"))
                    && !grid.contains(&format!("var({themed}-")),
                "the grid uses {themed}, which changes with the theme"
            );
        }
    }

    #[test]
    fn components_css_uses_only_token_sizes() {
        let all = styles();
        let (name, css) = &all[0];
        let css = code(css);
        let hits: Vec<&str> = css.lines().filter(|l| l.contains("px")).collect();
        assert!(hits.is_empty(), "{name} has raw px sizes: {hits:#?}");
    }
}

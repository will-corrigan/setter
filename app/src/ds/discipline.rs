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

    #[test]
    fn components_css_uses_only_token_sizes() {
        let all = styles();
        let (name, css) = &all[0];
        let css = code(css);
        let hits: Vec<&str> = css.lines().filter(|l| l.contains("px")).collect();
        assert!(hits.is_empty(), "{name} has raw px sizes: {hits:#?}");
    }
}

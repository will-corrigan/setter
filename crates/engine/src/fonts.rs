//! The app's two typefaces, named here and nowhere else. The app and the
//! exported web page refer to them only as "Setter Sans" and "Setter Serif",
//! so trying another font means changing [`SANS`] or [`SERIF`] (and its
//! licence line in data/NOTICE.md). `scripts/subset_font.sh` makes a font
//! file small enough to bundle.

/// One font file: what it is, and its bytes.
pub struct Font {
    /// Its real name, for the credits.
    pub name: &'static str,
    /// "normal" or "italic".
    pub style: &'static str,
    /// The weights it covers, as CSS writes them ("100 900").
    pub weights: &'static str,
    pub woff2: &'static [u8],
}

/// Buttons, labels, the grid's letters: an upright sans with tall capitals.
pub const SANS: Font = Font {
    name: "Inter",
    style: "normal",
    weights: "100 900",
    woff2: include_bytes!("../../../app/assets/fonts/inter-latin-var-normal.woff2"),
};

/// Clues and headings.
pub const SERIF: Font = Font {
    name: "Newsreader",
    style: "normal",
    weights: "200 800",
    woff2: include_bytes!("../../../app/assets/fonts/newsreader-latin-opsz-normal.woff2"),
};

pub const SERIF_ITALIC: Font = Font {
    name: "Newsreader",
    style: "italic",
    weights: "200 800",
    woff2: include_bytes!("../../../app/assets/fonts/newsreader-latin-opsz-italic.woff2"),
};

/// What CSS calls them.
pub const SANS_FAMILY: &str = "Setter Sans";
pub const SERIF_FAMILY: &str = "Setter Serif";

/// One `@font-face` rule with the font inlined, so nothing is fetched.
pub fn face(family: &str, font: &Font) -> String {
    format!(
        "@font-face{{font-family:'{family}';font-style:{};font-weight:{};font-display:block;src:url(data:font/woff2;base64,{}) format('woff2')}}",
        font.style,
        font.weights,
        base64(font.woff2)
    )
}

/// The `@font-face` rules for the app.
pub fn app_faces() -> String {
    [
        face(SANS_FAMILY, &SANS),
        face(SERIF_FAMILY, &SERIF),
        face(SERIF_FAMILY, &SERIF_ITALIC),
    ]
    .concat()
}

/// The typefaces' names, for the credits ("Inter and Newsreader").
pub fn names() -> Vec<&'static str> {
    let mut names = vec![SANS.name, SERIF.name, SERIF_ITALIC.name];
    names.dedup();
    names
}

pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_values() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn the_app_s_faces_use_the_generic_names_only() {
        let css = app_faces();
        assert_eq!(css.matches("@font-face").count(), 3);
        assert_eq!(css.matches("'Setter Sans'").count(), 1);
        assert_eq!(css.matches("'Setter Serif'").count(), 2);
        assert!(!css.contains(SANS.name));
        // Real fonts: woff2 files start "wOF2".
        for f in [&SANS, &SERIF, &SERIF_ITALIC] {
            assert_eq!(&f.woff2[..4], b"wOF2", "{}", f.name);
        }
    }

    #[test]
    fn credits_name_each_typeface_once() {
        assert_eq!(names(), vec!["Inter", "Newsreader"]);
    }
}

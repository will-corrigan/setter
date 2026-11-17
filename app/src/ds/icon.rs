//! Small line icons, drawn inline so they take the colour of their text.
//! The shapes are from Lucide (ISC licence; see the credits).

use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconName {
    /// A wand with sparkles: fill something in.
    Wand,
    /// An eraser: clear something.
    Eraser,
}

impl IconName {
    pub fn id(self) -> &'static str {
        match self {
            IconName::Wand => "wand",
            IconName::Eraser => "eraser",
        }
    }

    fn paths(self) -> &'static [&'static str] {
        match self {
            IconName::Wand => &[
                "m21.64 3.64-1.28-1.28a1.21 1.21 0 0 0-1.72 0L2.36 18.64a1.21 1.21 0 0 0 0 1.72l1.28 1.28a1.2 1.2 0 0 0 1.72 0L21.64 5.36a1.2 1.2 0 0 0 0-1.72",
                "m14 7 3 3",
                "M5 6v4",
                "M19 14v4",
                "M10 2v2",
                "M7 8H3",
                "M21 16h-4",
                "M11 3H9",
            ],
            IconName::Eraser => &[
                "m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21",
                "M22 21H7",
                "m5 11 9 9",
            ],
        }
    }
}

/// A decorative icon. Whatever it sits in (a button, say) carries the name.
#[component]
pub fn Icon(name: IconName) -> Element {
    rsx! {
        svg {
            class: "icon",
            "data-icon": name.id(),
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            for (i, d) in name.paths().iter().enumerate() {
                path { key: "{i}", d: *d }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn icons_are_inline_svg_hidden_from_screen_readers() {
        let wand = render(move || rsx! { Icon { name: IconName::Wand } });
        assert!(
            wand.contains("<svg") && wand.contains(r#"class="icon""#),
            "{wand}"
        );
        assert!(wand.contains(r#"data-icon="wand""#), "{wand}");
        assert!(wand.contains(r#"aria-hidden="true""#), "{wand}");
        assert!(wand.contains(r#"stroke="currentColor""#), "{wand}");
        assert_eq!(wand.matches("<path").count(), 8, "{wand}");
        let eraser = render(move || rsx! { Icon { name: IconName::Eraser } });
        assert!(eraser.contains(r#"data-icon="eraser""#), "{eraser}");
        assert_eq!(eraser.matches("<path").count(), 3, "{eraser}");
    }
}

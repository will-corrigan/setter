//! Small line icons, drawn inline so they take the colour of their text.
//! The shapes are from Lucide (ISC licence; see the credits).

use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconName {
    /// A wand with sparkles: fill something in.
    Wand,
    /// An eraser: clear something.
    Eraser,
    /// An arrow curling back: undo.
    Undo,
    /// An arrow curling forward: redo.
    Redo,
    /// A gear: settings.
    Settings,
    /// A magnifier: search.
    Search,
}

impl IconName {
    pub fn id(self) -> &'static str {
        match self {
            IconName::Wand => "wand",
            IconName::Eraser => "eraser",
            IconName::Undo => "undo",
            IconName::Redo => "redo",
            IconName::Settings => "settings",
            IconName::Search => "search",
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
            IconName::Undo => &["M9 14 4 9l5-5", "M4 9h10.5a5.5 5.5 0 0 1 0 11H11"],
            IconName::Redo => &["m15 14 5-5-5-5", "M20 9H9.5a5.5 5.5 0 0 0 0 11H13"],
            IconName::Settings => &[
                "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z",
                "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0",
            ],
            IconName::Search => &["m21 21-4.34-4.34", "M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0"],
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

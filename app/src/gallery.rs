//! Component gallery: every design-system component, in light and dark,
//! rendered with real engine data. Run with `setter --gallery`.

use crate::components::button::{Button, ButtonVariant};
use crate::ds::answer_nav::AnswerNav;
use crate::ds::chip::Chip;
use crate::ds::clue_field::ClueField;
use crate::ds::empty_state::EmptyState;
use crate::ds::note::Note;
use crate::ds::notice::Notice;
use crate::ds::reads_as::ReadsAs;
use crate::ds::segmented::Segmented;
use crate::ds::side_nav::{NavItem, SideNav};
use crate::ds::tiles::Tiles;
use crate::ds::top_bar::TopBar;
use crate::ds::word_column::WordColumn;
use crate::highlight::{checks, Checks};
use dioxus::prelude::*;
use std::sync::LazyLock;
use xw_engine::bank::{self, DeviceKind, DeviceView, Tone};

/// Real engine output for ORCHESTRA, so the gallery shows true data shapes.
static SAMPLE: LazyLock<Vec<DeviceView>> = LazyLock::new(|| {
    let data = crate::files::load_engine_data_from(&std::env::temp_dir().join("setter-gallery"));
    bank::build(
        xw_engine::Ctx::new(&data.lex, &data.abbr),
        &data.ind,
        &data.th,
        "orchestra",
    )
});

fn device(kind: DeviceKind) -> &'static DeviceView {
    SAMPLE
        .iter()
        .find(|d| d.kind == kind)
        .expect("every device is built")
}

#[component]
pub fn Gallery() -> Element {
    // Dev hook for screenshots: SETTER_GALLERY_SCROLL=<px> scrolls the gallery.
    use_effect(|| {
        if let Some(px) = std::env::var("SETTER_GALLERY_SCROLL")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
        {
            document::eval(&format!(
                "setTimeout(() => document.querySelector('.gallery').scrollTop = {px}, 300)"
            ));
        }
    });
    rsx! {
        div { class: "gallery",
            div { class: "gallery__columns",
                Showcase { scope: "light" }
                Showcase { scope: "dark" }
            }
        }
    }
}

#[component]
fn Section(#[props(into)] title: String, children: Element) -> Element {
    rsx! {
        section { class: "gallery__section",
            h3 { class: "gallery__title", "{title}" }
            {children}
        }
    }
}

#[component]
fn Showcase(scope: &'static str) -> Element {
    let mut mode = use_signal(|| 1usize);
    let mut option = use_signal(|| 0usize);
    let mut nav = use_signal(|| "Charade".to_string());
    let mut clue = use_signal(|| "Gold box by artist for band".to_string());
    let mut pick = use_signal(|| Some("box".to_string()));
    let mut answer = use_signal(|| 0usize);
    let mut notice = use_signal(|| true);
    let answers = [
        ("ORCHESTRA", "10a · 9"),
        ("OR·HE·TRA", "4d · 9"),
        ("TEA", "9d · 3"),
    ];
    let charade = device(DeviceKind::Charade);
    let split = charade
        .options
        .iter()
        .find(|o| o.label == "OR + CHEST + RA")
        .unwrap_or(&charade.options[0]);
    let deletion = device(DeviceKind::Deletion);
    let picks = vec![
        ("gold".to_string(), Tone::P1),
        ("box".to_string(), Tone::P2),
        ("artist".to_string(), Tone::P3),
        ("band".to_string(), Tone::Definition),
        ("by".to_string(), Tone::Indicator),
    ];
    let c: Checks = checks(&clue.read(), &picks[..3], Some("band"));
    let nav_items: Vec<NavItem> = SAMPLE
        .iter()
        .map(|d| NavItem {
            key: d.kind.name().into(),
            label: d.kind.name().into(),
            count: d.count,
            group: d.kind.group().name().into(),
        })
        .collect();
    rsx! {
        div { class: "gallery__theme", "data-theme-scope": scope,
            Section { title: "Top bar",
                TopBar {
                    brand: "Setter",
                    start: rsx! { span { "…" } },
                    end: rsx! { Note { text: "11 of 28 clued" } },
                }
            }
            Section { title: "Buttons ({scope})",
                div { class: "gallery__row",
                    Button { variant: ButtonVariant::Primary, "Save" }
                    Button { variant: ButtonVariant::Secondary, "Open" }
                    Button { variant: ButtonVariant::Outline, "New sample" }
                    Button { variant: ButtonVariant::Ghost, "Cancel" }
                }
            }
            Section { title: "Segmented control",
                Segmented {
                    label: "Mode",
                    options: vec!["Build grid".to_string(), "Write clues".to_string()],
                    selected: mode(),
                    on_change: move |i| mode.set(i),
                }
            }
            Section { title: "Answer navigator",
                AnswerNav {
                    word: answers[answer()].0,
                    label: answers[answer()].1,
                    on_step: move |delta: i32| answer.set((answer() as i32 + delta).rem_euclid(3) as usize),
                }
            }
            Section { title: "Notice",
                if notice() {
                    Notice { message: "abbreviations.toml: expected a table at line 4", on_dismiss: move |_| notice.set(false) }
                } else {
                    Button { variant: ButtonVariant::Ghost, onclick: move |_| notice.set(true), "Show the notice" }
                }
            }
            Section { title: "Option pills",
                Segmented {
                    label: "Splits",
                    pills: true,
                    options: charade.options.iter().map(|o| o.label.clone()).collect::<Vec<_>>(),
                    selected: option(),
                    on_change: move |i| option.set(i),
                }
            }
            Section { title: "Answer tiles",
                Tiles { tiles: split.tiles.clone() }
                Tiles { tiles: deletion.options[0].tiles.clone() }
                Tiles { tiles: vec![xw_engine::bank::Tile { letters: "OR?HE?TRA".into(), tone: None, struck: false }] }
            }
            Section { title: "Chips",
                div { class: "gallery__row",
                    Chip { label: "band", tone: Some(Tone::Definition), pressed: true }
                    Chip { label: "gold", tone: Some(Tone::P1), pressed: true }
                    Chip { label: "trunk", tone: Some(Tone::P2) }
                    Chip { label: "wildly", tone: Some(Tone::Indicator) }
                }
            }
            Section { title: "Clue field",
                ClueField {
                    text: clue(),
                    picks: picks.clone(),
                    enumeration: "9".to_string(),
                    on_input: move |t| clue.set(t),
                }
                ReadsAs { text: split.reads_as.clone(), checks: c }
            }
            Section { title: "Device navigation",
                div { style: "height: 330px; display: grid; grid-template-columns: var(--sidebar-width);",
                    SideNav { items: nav_items, current: nav(), on_select: move |k| nav.set(k) }
                }
            }
            Section { title: "Word bank columns",
                div { class: "word-bank",
                    for (i, col) in split.columns.iter().enumerate() {
                        WordColumn {
                            key: "{i}",
                            column: col.clone(),
                            selected: if i == 1 { pick() } else { None },
                            on_pick: move |w| pick.set(Some(w)),
                        }
                    }
                }
            }
            Section { title: "Empty state",
                EmptyState { message: "This answer has empty squares." }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn gallery_shows_every_component_in_both_themes() {
        let html = render(move || rsx! { Gallery {} });
        for class in [
            "chip",
            "tiles__tile",
            "word-column",
            "clue-field",
            "side-nav",
            "reads-as",
            "empty-state",
            "answer-nav",
            "notice",
            "dx-toggle",
            "dx-button",
            "topbar__logo",
            "class=\"note\"",
        ] {
            assert!(html.contains(class), "missing {class}");
        }
        assert!(
            html.contains(r#"data-theme-scope="light""#)
                && html.contains(r#"data-theme-scope="dark""#)
        );
    }
}

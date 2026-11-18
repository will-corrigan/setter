//! Component gallery: every design-system component, in light and dark,
//! rendered with real engine data. Run with `setter --gallery`.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::answer_bar::AnswerBar;
use crate::ds::answer_nav::AnswerNav;
use crate::ds::chip::Chip;
use crate::ds::clue_field::ClueField;
use crate::ds::clue_sheet::ClueSheet;
use crate::ds::crossword_grid::{CrosswordGrid, GridKey};
use crate::ds::empty_state::EmptyState;
use crate::ds::icon::{Icon, IconName};
use crate::ds::inline_edit::InlineEdit;
use crate::ds::menu_button::{MenuButton, MenuItem};
use crate::ds::note::Note;
use crate::ds::notice::Notice;
use crate::ds::prose::Prose;
use crate::ds::reads_as::ReadsAs;
use crate::ds::segmented::Segmented;
use crate::ds::side_nav::{NavItem, SideNav};
use crate::ds::status_line::{StatusAction, StatusItem, StatusLine, StatusTone};
use crate::ds::template_picker::{GridThumb, TemplatePicker};
use crate::ds::tiles::Tiles;
use crate::ds::top_bar::TopBar;
use crate::ds::word_column::WordColumn;
use crate::ds::word_popover::{Placement, Side, WordEntry, WordPopover};
use crate::grid_edit::{self, Cursor};
use crate::grid_fill;
use crate::grid_view;
use crate::highlight::{checks, Checks};
use dioxus::prelude::*;
use std::sync::LazyLock;
use xw_engine::bank::{self, DeviceKind, DeviceView, Tone};
use xw_engine::grid::{Dir, Grid};
use xw_engine::puzzle::Puzzle;
use xw_engine::rules::light_name;
use xw_engine::templates::TEMPLATES;

/// Real engine output for ORCHESTRA, so the gallery shows true data shapes.
static SAMPLE: LazyLock<Vec<DeviceView>> = LazyLock::new(|| {
    let data = crate::files::load_engine_data_from(&crate::files::scratch_dir("gallery"));
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

/// A small part-filled puzzle with a two-word answer, a hyphened answer and
/// two clues, for the Build grid components.
fn sample_puzzle() -> Puzzle {
    let mut p = Puzzle::new(7, 7);
    p.grid = Grid::from_rows(&[
        "CARTHOP", ".#.#.#.", "ICEPACK", ".#.#.#.", "CATCH..", ".#.#.#.", ".......",
    ]);
    grid_edit::set_split(&mut p, "2,0,a", "3,4");
    grid_edit::set_split(&mut p, "4,0,a", "5-2");
    p.clues.entry("0,0,a".into()).or_default().text =
        "Wagon with a jump for a dance in the barn".into();
    p.clues.entry("0,2,d".into()).or_default().text = "Shy about one in a row".into();
    p
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
    let mut puzzle = use_signal(sample_puzzle);
    let mut cursor = use_signal(|| Cursor {
        cell: 14,
        dir: Dir::Across,
    });
    let mut query = use_signal(String::new);
    let mut words = use_signal(|| 0usize);
    let mut picked = use_signal(|| None::<String>);
    let mut added = use_signal(|| false);
    let (cells, columns) = {
        let p = puzzle.read();
        (grid_view::grid_cells(&p, cursor()), p.grid.width)
    };
    let (across, down) = grid_view::clue_rows(&puzzle.read());
    let light = grid_edit::current_light(&puzzle.read().grid, cursor());
    let (bar_label, bar_pattern, bar_split) = match &light {
        Some(l) => {
            let p = puzzle.read();
            (
                light_name(l),
                grid_view::shown_pattern(&p.grid.pattern(l), grid_fill::split_of(&p, &l.id)),
                grid_view::split_text(&p, l),
            )
        }
        None => Default::default(),
    };
    let current = light.as_ref().map(|l| l.id.clone());
    let entries: Vec<WordEntry> = [
        ("ice pack", Some("3,4")),
        ("iceberg", None),
        ("ice-cold", Some("3-4")),
        ("iceland", None),
        ("ice floe", Some("3,4")),
    ]
    .into_iter()
    .filter(|(text, _)| text.starts_with(query.read().as_str()))
    .map(|(text, split)| WordEntry {
        text: text.to_string(),
        split: split.map(str::to_string),
        rare: false,
    })
    .collect();
    let mut status = vec![
        StatusItem::new(StatusTone::Plain, "3 of 12 answers filled"),
        StatusItem::new(StatusTone::Ok, "everything still fillable"),
        StatusItem::new(StatusTone::Warn, "repeated: ICE"),
    ];
    if !added() {
        status.insert(
            1,
            StatusItem {
                tone: StatusTone::Warn,
                text: "CARTHOP (1 across) is not in the word list".into(),
                action: Some(StatusAction {
                    label: grid_view::ADD_WORD.into(),
                    key: "0,0,a".into(),
                }),
            },
        );
    }
    let thumbs: Vec<GridThumb> = TEMPLATES
        .iter()
        .map(|t| GridThumb {
            name: t.name.to_string(),
            rows: t.rows.iter().map(|r| r.to_string()).collect(),
        })
        .collect();
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
            Section { title: "Menu button",
                div { class: "gallery__row",
                    MenuButton {
                        label: "Export",
                        items: vec![
                            MenuItem::new("Open…").keys("Ctrl+O"),
                            MenuItem::new("Save as…").keys("Ctrl+Shift+S"),
                            MenuItem::new("Export web page…").separated(),
                            MenuItem::new("Show in folder").disabled(true),
                            MenuItem::heading("Recent"),
                            MenuItem::new("Prize puzzle 12"),
                        ],
                        on_pick: move |_| {},
                    }
                }
            }
            Section { title: "Inline edit",
                div { class: "gallery__row",
                    InlineEdit { value: "Prize puzzle 12", placeholder: "Untitled", label: "Puzzle title", on_change: move |_| {} }
                    InlineEdit { value: "", placeholder: "Setter", label: "Setter", on_change: move |_| {} }
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
            Section { title: "Icons",
                div { class: "gallery__row",
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::IconSm,
                        title: "Fill just this answer",
                        aria_label: "Fill just this answer",
                        Icon { name: IconName::Wand }
                    }
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::IconSm,
                        title: "Clear this answer",
                        aria_label: "Clear this answer",
                        Icon { name: IconName::Eraser }
                    }
                    Icon { name: IconName::Wand }
                    Icon { name: IconName::Eraser }
                    Icon { name: IconName::Undo }
                    Icon { name: IconName::Redo }
                    Icon { name: IconName::Settings }
                    Icon { name: IconName::Search }
                }
            }
            Section { title: "Answer bar",
                AnswerBar {
                    label: bar_label,
                    pattern: bar_pattern,
                    split: bar_split,
                    on_split: move |text: String| {
                        if let Some(id) = &current {
                            grid_edit::set_split(&mut puzzle.write(), id, &text);
                        }
                    },
                }
            }
            Section { title: "Crossword grid (the same in light and dark)",
                CrosswordGrid {
                    cells,
                    columns,
                    on_select: move |cell| {
                        let next = grid_edit::click(&puzzle.read().grid, cursor(), cell);
                        cursor.set(next);
                    },
                    on_words: move |_| {},
                    on_key: move |key| {
                        let c = cursor();
                        let next = match key {
                            GridKey::Letter(ch) => grid_edit::type_letter(&mut puzzle.write(), c, ch),
                            GridKey::Backspace => grid_edit::backspace(&mut puzzle.write(), c),
                            GridKey::Delete => grid_edit::delete(&mut puzzle.write(), c),
                            GridKey::Move(rows, columns) => {
                                grid_edit::arrow(&puzzle.read().grid, c, rows, columns)
                            }
                            GridKey::Turn => grid_edit::click(&puzzle.read().grid, c, c.cell),
                            GridKey::NextAnswer => grid_edit::next_light(&puzzle.read().grid, c, 1),
                            GridKey::PreviousAnswer => grid_edit::next_light(&puzzle.read().grid, c, -1),
                            GridKey::Escape => c,
                        };
                        cursor.set(next);
                    },
                }
            }
            Section { title: "Status line",
                StatusLine { items: status, on_action: move |_| added.set(true) }
            }
            Section { title: "Words popover",
                div { class: "gallery__stage",
                    WordPopover {
                        label: "5 across",
                        pattern: "ICE ····",
                        at: Placement { side: Side::Right, column: 0, row: 0 },
                        query: query(),
                        words: words(),
                        entries: Some(entries),
                        on_query: move |text| query.set(text),
                        on_words: move |i| words.set(i),
                        on_place: move |text| picked.set(Some(text)),
                        on_fill: move |_| picked.set(Some("iceberg".to_string())),
                        on_clear: move |_| picked.set(None),
                        on_close: move |_| {},
                    }
                }
                if let Some(word) = picked() {
                    Note { text: "Placed: {word}" }
                }
            }
            Section { title: "Clue sheet",
                ClueSheet {
                    across,
                    down,
                    current: light.as_ref().map(|l| l.id.clone()),
                    on_select: move |id: String| {
                        if let Some(next) = grid_fill::cursor_on(&puzzle.read().grid, &id) {
                            cursor.set(next);
                        }
                    },
                }
            }
            Section { title: "Template picker",
                div { class: "gallery__stage", "data-tall": "true",
                    TemplatePicker { title: "New puzzle", grids: thumbs, on_pick: move |_| {}, on_close: move |_| {} }
                }
            }
            Section { title: "Prose",
                Prose { title: "Credits", text: "Open English WordNet 2025, CC BY 4.0.\nMoby Thesaurus II, public domain." }
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
            "prose__body",
            "class=\"icon\"",
            "answer-bar",
            "xw-grid__cell",
            "status-line__item",
            "word-popover__item",
            "clue-sheet__row",
            "template-picker__thumb",
            "menu-button__trigger",
            "inline-edit",
        ] {
            assert!(html.contains(class), "missing {class}");
        }
        assert!(
            html.contains(r#"data-theme-scope="light""#)
                && html.contains(r#"data-theme-scope="dark""#)
        );
    }
}

//! Write clues: the grid and devices on the left; the answer, its clue card
//! (clue, checks, definition switch, explanation) and the word bank.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::answer_nav::AnswerNav;
use crate::ds::clue_field::ClueField;
use crate::ds::empty_state::EmptyState;
use crate::ds::inline_edit::InlineEdit;
use crate::ds::mini_grid::{MiniGrid, MiniSize};
use crate::ds::note::Note;
use crate::ds::reads_as::ReadsAs;
use crate::ds::segmented::Segmented;
use crate::ds::side_nav::{NavItem, SideNav};
use crate::ds::tiles::Tiles;
use crate::ds::word_column::WordColumn;
use crate::highlight;
use crate::keys;
use crate::state::{self, AppState};
use dioxus::prelude::*;
use xw_engine::bank::DeviceKind;

#[component]
pub fn WriteClues() -> Element {
    let state = use_context::<AppState>();
    if (state.answer)().is_none() {
        return rsx! {
            EmptyState { message: "This puzzle has no answers." }
        };
    }
    rsx! {
        div { class: "shell__body write-clues-body",
            aside { class: "write-clues__rail",
                GridRail {}
                Devices {}
            }
            main { class: "shell__main write-clues",
                Head {}
                Clue {}
                Bank {}
            }
        }
    }
}

/// The whole grid, small, with the answer on screen lit; a square goes to
/// its answer.
#[component]
fn GridRail() -> Element {
    let state = use_context::<AppState>();
    let puzzle = state.puzzle.read();
    let Some(p) = puzzle.as_ref() else {
        return rsx! {};
    };
    let light = state.selection.read().light.clone();
    let lit: Vec<usize> = p
        .grid
        .lights()
        .into_iter()
        .find(|l| l.id == light)
        .map(|l| l.cells)
        .unwrap_or_default();
    let blocks: Vec<bool> = p.grid.cells.iter().map(|c| c.block).collect();
    let (clued, total) = (state.clued)();
    rsx! {
        div { class: "write-clues__grid", style: "--side: {p.grid.width.max(p.grid.height)}",
            MiniGrid {
                columns: p.grid.width,
                blocks,
                size: MiniSize::Medium,
                current: lit.first().copied(),
                lit,
                on_cell: move |cell: usize| state.select_cell(cell),
            }
            span { class: "write-clues__count", "{clued} of {total} clued" }
        }
    }
}

fn nav_item(kind: DeviceKind, count: usize) -> NavItem {
    NavItem {
        key: kind.name().into(),
        label: kind.name().into(),
        count,
        group: kind.group().name().into(),
    }
}

/// The 13 devices with their counts. Until the word bank is built, the names alone.
#[component]
fn Devices() -> Element {
    let state = use_context::<AppState>();
    let shown = state.shown();
    let items: Vec<NavItem> = match &shown {
        Some(s) => s
            .devices
            .iter()
            .map(|d| nav_item(d.kind, d.count))
            .collect(),
        None => DeviceKind::ALL.iter().map(|k| nav_item(*k, 0)).collect(),
    };
    let current = shown
        .as_ref()
        .map(|s| s.device.name().to_string())
        .unwrap_or_default();
    rsx! {
        SideNav {
            items,
            current,
            busy: shown.is_none(),
            on_select: move |name: String| {
                if let Some(kind) = state::device_named(&name) {
                    state.select_device(kind);
                }
            },
        }
    }
}

/// "10a" → "10 across".
pub fn spoken(label: &str) -> String {
    match label.char_indices().last() {
        Some((i, 'a')) => format!("{} across", &label[..i]),
        Some((i, 'd')) => format!("{} down", &label[..i]),
        _ => label.to_string(),
    }
}

/// Which answer, its tiles and option pills; then moving between answers.
#[component]
fn Head() -> Element {
    let state = use_context::<AppState>();
    let Some(answer) = (state.answer)() else {
        return rsx! {};
    };
    let shown = state.shown();
    let view = shown.as_ref().and_then(|s| s.view());
    let tiles = shown
        .as_ref()
        .and_then(|s| s.option())
        .map(|o| o.tiles.clone())
        .unwrap_or_else(|| state::plain_tiles(&answer.pattern));
    let labels: Vec<String> = view
        .map(|d| d.options.iter().map(|o| o.label.clone()).collect())
        .unwrap_or_default();
    let device = shown.as_ref().map(|s| s.device);
    let selected = shown.as_ref().map_or(0, |s| s.option);
    let (position, total) = {
        let puzzle = state.puzzle.read();
        puzzle.as_ref().map_or((0, 0), |p| {
            let all = crate::answers::answers(p);
            let at = all.iter().position(|a| a.light_id == answer.light_id);
            (at.map_or(0, |i| i + 1), all.len())
        })
    };
    let (clued, _) = (state.clued)();
    rsx! {
        div { class: "clue-head",
            span { class: "clue-head__answer", "{spoken(&answer.label)}" }
            Tiles { tiles }
            if answer.pattern.contains('?') {
                Note { text: "This answer has empty squares." }
            }
            if let Some(device) = device.filter(|_| !labels.is_empty()) {
                Segmented {
                    label: "Options",
                    pills: true,
                    options: labels,
                    selected,
                    on_change: move |i| state.select_option(device, i),
                }
            }
            div { class: "clue-head__nav",
                AnswerNav {
                    label: "{position} of {total}",
                    on_step: move |delta| state.step(delta),
                }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    title: keys::tip("Next answer without a clue", "Ctrl+J"),
                    disabled: clued == total,
                    onclick: move |_| state.next_unclued(),
                    "Next unclued"
                    kbd { class: "clue-head__keys", "{keys::label(\"Ctrl+J\")}" }
                }
            }
        }
    }
}

/// The clue card: the clue, how it reads and its checks, where the
/// definition goes, and the explanation.
#[component]
fn Clue() -> Element {
    let state = use_context::<AppState>();
    let Some(answer) = (state.answer)() else {
        return rsx! {};
    };
    let text = state.clue_text();
    let shown = state.shown();
    let option = shown.as_ref().and_then(|s| s.option());
    let marks = match (&shown, option) {
        (Some(s), Some(o)) => state::marks(
            &state.picks.read(),
            &answer.light_id,
            s.device,
            s.option,
            &o.columns,
        ),
        _ => state::Marks::default(),
    };
    let required = match (&shown, option) {
        (Some(s), Some(o)) => xw_engine::bank::required_parts(s.device, o),
        _ => 0,
    };
    let checks = highlight::checks_for(&text, &marks.parts, required, marks.definition.as_deref());
    let has_definition = option.is_some_and(|o| {
        o.columns
            .iter()
            .any(|c| c.tone == xw_engine::bank::Tone::Definition)
    });
    let definition_last = state.settings.read().definition_last;
    rsx! {
        div { class: "clue-card",
            ClueField {
                text,
                picks: marks.all,
                enumeration: answer.enumeration,
                on_input: move |t| state.set_clue_text(t),
                on_arrange: move |t| state.arrange_clue(t),
            }
            div { class: "clue-card__foot",
                if let Some(o) = option {
                    ReadsAs { text: o.reads_as.clone(), checks }
                }
                if has_definition {
                    div { class: "clue-card__definition",
                        span { class: "clue-card__label", "Definition" }
                        Segmented {
                            label: "Where the definition goes",
                            pills: true,
                            options: vec!["Start".to_string(), "End".to_string()],
                            selected: usize::from(definition_last),
                            on_change: move |i: usize| state.set_definition_last(i == 1),
                        }
                    }
                }
            }
            div { class: "clue-note",
                span { class: "clue-note__label", "Explanation" }
                InlineEdit {
                    key: "{answer.light_id}",
                    value: state.clue_note(),
                    placeholder: "none yet",
                    label: "Explanation, shown to solvers once they finish",
                    on_change: move |note| state.set_clue_note(note),
                }
            }
        }
    }
}

/// One column of words per part of the chosen option, under one filter.
#[component]
fn Bank() -> Element {
    let state = use_context::<AppState>();
    let mut filter = use_signal(String::new);
    // A new answer starts with the filter cleared.
    let light = state.answer.read().as_ref().map(|a| a.light_id.clone());
    let mut seen = use_signal(|| light.clone());
    if *seen.peek() != light {
        seen.set(light);
        filter.set(String::new());
    }
    let Some(answer) = (state.answer)() else {
        return rsx! {};
    };
    let Some(shown) = state.shown() else {
        return rsx! {
            div { class: "word-bank-pending",
                EmptyState { message: "Finding words…" }
            }
        };
    };
    let Some(option) = shown.option() else {
        // An answer with empty squares already says why there is nothing.
        if answer.pattern.contains('?') {
            return rsx! {};
        }
        return rsx! {
            EmptyState { message: "Nothing found for this answer." }
        };
    };
    let picks = state.picks.read();
    let columns = option.columns.iter().enumerate().map(|(i, column)| {
        let key = state::pick_key(&answer.light_id, shown.device, shown.option, i);
        (i, column.clone(), picks.get(&key).cloned(), key)
    });
    let typed = filter();
    rsx! {
        div { class: "word-bank-bar",
            input {
                class: "word-bank-bar__filter",
                r#type: "search",
                placeholder: "Filter every column",
                "aria-label": "Filter every column",
                value: "{typed}",
                oninput: move |e| filter.set(e.value()),
            }
        }
        div { class: "word-bank",
            for (i, column, selected, key) in columns {
                WordColumn {
                    // A new device or option starts each column afresh.
                    key: "{shown.device:?}/{shown.option}/{i}",
                    column,
                    selected,
                    filter: typed.clone(),
                    on_pick: move |item: String| state.toggle_pick(key.clone(), &item),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;
    use crate::files::Settings;
    use crate::state::{build_bank, pick_key, toggle_pick, use_app_state, Selection};
    use std::rc::Rc;
    use xw_engine::bank::DeviceKind;
    use xw_engine::grid::Grid;
    use xw_engine::puzzle::Puzzle;

    fn puzzle(rows: &[&str]) -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.grid = Grid::from_rows(rows);
        p
    }

    /// Render the page for a puzzle, after `setup` has arranged the state.
    fn page(p: Puzzle, setup: impl FnOnce(AppState) + 'static) -> String {
        render(move || {
            let state = use_app_state(Settings::default);
            use_hook(|| {
                state.show(p, None);
                setup(state);
            });
            rsx! { WriteClues {} }
        })
    }

    fn with_bank(mut state: AppState, pattern: &str) {
        state
            .banks
            .write()
            .insert(pattern.to_string(), Rc::new(build_bank(pattern)));
    }

    #[test]
    fn the_clue_s_explanation_shows_and_can_be_edited() {
        let mut p = puzzle(&["cat", "a#o", "bye"]);
        p.clues.entry("0,0,a".into()).or_default().annotation = "C + AT".into();
        let html = page(p, |state| with_bank(state, "cat"));
        let note = html.find("clue-note").expect("note");
        assert!(html[note..].contains(">C + AT<"), "{html}");
        assert!(
            html.contains(r#"aria-label="Explanation, shown to solvers once they finish""#),
            "{html}"
        );
    }

    #[test]
    fn shows_device_nav_tiles_clue_field_and_word_bank() {
        let mut p = puzzle(&["cat", "a#o", "bye"]);
        p.clues.entry("0,0,a".into()).or_default().text = "Pet".into();
        let html = page(p, |mut state| {
            with_bank(state, "cat");
            let anagram = state.selection.peek().with_device(DeviceKind::Anagram);
            state.selection.set(anagram);
        });
        // Device nav: all 13 devices in two groups, one current.
        assert_eq!(html.matches("side-nav__count").count(), 13, "{html}");
        assert!(html.contains(">Wordplay<") && html.contains(">Meaning only<"));
        for kind in DeviceKind::ALL {
            assert!(
                html.contains(&format!(">{}<", kind.name())),
                "{}",
                kind.name()
            );
        }
        assert_eq!(html.matches(r#"aria-current="true""#).count(), 1, "{html}");
        // Tiles: the answer, coloured for the option.
        assert_eq!(html.matches("tiles__tile").count(), 3, "{html}");
        assert!(html.contains("clue-head"), "{html}");
        // Option pills.
        assert!(html.contains(r#"data-variant="pills""#), "{html}");
        // Clue field bound to the clue, with the enumeration.
        assert!(html.contains("clue-field__editor"), "{html}");
        assert!(
            html.contains(r#"value="Pet""#) && html.contains("(3)"),
            "{html}"
        );
        // Reads-as line and one word column per column.
        assert!(html.contains("reads-as"), "{html}");
        assert!(html.contains("word-bank"), "{html}");
        assert!(
            html.matches(r#"class="word-column""#).count() >= 3,
            "{html}"
        );
        assert!(html.contains(">Definition<") && html.contains(">Mixing words<"));
    }

    #[test]
    fn the_grid_stays_in_view_with_the_answer_lit() {
        let html = page(puzzle(&["cat", "a#o", "bye"]), |state| {
            with_bank(state, "cat")
        });
        let rail = html.find("write-clues__rail").expect("rail");
        let grid = &html[rail..];
        assert!(grid.contains("mini-grid--medium"), "{html}");
        assert_eq!(grid.matches("is-current").count(), 1, "{html}");
        assert_eq!(
            grid.matches("is-lit").count(),
            2,
            "the rest of 1 across: {html}"
        );
        assert!(html.contains("0 of 4 clued"), "{html}");
        // The rail holds the devices too.
        let devices = html.find("side-nav").unwrap();
        assert!(devices > rail, "{html}");
    }

    #[test]
    fn the_head_names_the_answer_and_offers_the_next_unclued_one() {
        let html = page(puzzle(&["cat", "a#o", "bye"]), |state| {
            with_bank(state, "cat")
        });
        assert!(
            html.contains(r#"class="clue-head__answer">1 across<"#),
            "{html}"
        );
        assert!(html.contains(">1 of 4<"), "{html}");
        assert!(html.contains(">Next unclued"), "{html}");
        assert!(html.contains(&crate::keys::label("Ctrl+J")), "{html}");
    }

    #[test]
    fn the_card_holds_clue_checks_definition_switch_and_explanation() {
        let html = page(puzzle(&["cat", "a#o", "bye"]), |mut state| {
            with_bank(state, "cat");
            let anagram = state.selection.peek().with_device(DeviceKind::Anagram);
            state.selection.set(anagram);
        });
        let card = html.find(r#"class="clue-card""#).expect("card");
        let bank = html.find("word-bank-bar").expect("bank filter");
        let inside = &html[card..bank];
        for piece in [
            "clue-field__editor",
            "reads-as",
            "clue-card__definition",
            ">Start<",
            ">End<",
            "clue-note",
        ] {
            assert!(inside.contains(piece), "{piece}: {inside}");
        }
    }

    #[test]
    fn one_filter_serves_every_column() {
        let html = page(puzzle(&["cat", "a#o", "bye"]), |mut state| {
            with_bank(state, "cat");
            let anagram = state.selection.peek().with_device(DeviceKind::Anagram);
            state.selection.set(anagram);
        });
        assert_eq!(
            html.matches(r#"placeholder="Filter every column""#).count(),
            1,
            "{html}"
        );
        assert!(!html.contains("word-column__filter"), "{html}");
        assert!(
            html.contains("--weight:"),
            "columns sized to their words: {html}"
        );
    }

    #[test]
    fn answer_labels_read_aloud() {
        assert_eq!(spoken("10a"), "10 across");
        assert_eq!(spoken("3d"), "3 down");
    }

    #[test]
    fn default_device_is_the_first_with_something() {
        let html = page(puzzle(&["cat", "a#o", "bye"]), |state| {
            with_bank(state, "cat")
        });
        let devices = build_bank("cat");
        let first = devices
            .iter()
            .find(|d| d.count > 0)
            .expect("cat has devices");
        let current = html
            .split("aria-current=\"true\"")
            .nth(1)
            .and_then(|rest| rest.split("</button>").next())
            .unwrap_or_default();
        assert!(current.contains(first.kind.name()), "{current}");
    }

    #[test]
    fn picks_colour_the_clue_and_select_the_word() {
        let devices = build_bank("cat");
        let anagram = devices
            .iter()
            .find(|d| d.kind == DeviceKind::Anagram)
            .unwrap();
        let cols = &anagram.options[0].columns;
        let definition = cols.iter().position(|c| c.title == "Definition").unwrap();
        let word = cols[definition].groups[0].1[0].clone();
        let mut p = puzzle(&["cat", "a#o", "bye"]);
        p.clues.entry("0,0,a".into()).or_default().text = format!("Odd {word}");
        let picked = word.clone();
        let html = page(p, move |mut state| {
            with_bank(state, "cat");
            state
                .selection
                .set(Selection::at("0,0,a").with_device(DeviceKind::Anagram));
            toggle_pick(
                &mut state.picks.write(),
                pick_key("0,0,a", DeviceKind::Anagram, 0, definition),
                &picked,
            );
        });
        // The editor is told which words are coloured pieces.
        let unescaped = html.replace("&#34;", "\"");
        assert!(
            unescaped.contains(&format!(r#"{{"t":"{word}","tone":"def"}}"#)),
            "{html}"
        );
        assert_eq!(html.matches(r#"aria-selected="true""#).count(), 1, "{html}");
        assert!(html.contains("definition at an end"), "{html}");
    }

    #[test]
    fn while_the_bank_is_building_tiles_and_clue_field_still_show() {
        let html = page(puzzle(&["cat", "a#o", "bye"]), |_| {});
        assert!(html.contains(r#"aria-busy="true""#), "{html}");
        assert_eq!(html.matches("side-nav__item").count(), 13, "{html}");
        assert_eq!(html.matches("tiles__tile").count(), 3, "{html}");
        assert!(html.contains("clue-field__editor"), "{html}");
        assert!(!html.contains(r#"class="word-column""#), "{html}");
        assert!(html.contains("Finding words…"), "{html}");
    }

    #[test]
    fn answers_with_blanks_show_the_pattern_and_say_so() {
        let html = page(puzzle(&["c?t", "a#o", "bye"]), |state| {
            with_bank(state, "c?t")
        });
        assert!(html.contains("This answer has empty squares."), "{html}");
        assert_eq!(html.matches("tiles__tile").count(), 3, "{html}");
        assert_eq!(html.matches("is-blank").count(), 1, "{html}");
        assert!(html.contains("clue-field__editor"), "{html}");
        assert_eq!(html.matches("is-empty").count(), 13, "{html}");
        assert!(!html.contains(r#"class="word-column""#), "{html}");
    }

    #[test]
    fn a_device_with_nothing_says_so() {
        let devices = build_bank("cat");
        let Some(empty) = devices.iter().find(|d| d.count == 0) else {
            return;
        };
        let kind = empty.kind;
        let html = page(puzzle(&["cat", "a#o", "bye"]), move |mut state| {
            with_bank(state, "cat");
            state
                .selection
                .set(Selection::at("0,0,a").with_device(kind));
        });
        assert!(html.contains("Nothing found for this answer."), "{html}");
        assert_eq!(html.matches("tiles__tile").count(), 3, "{html}");
        assert!(html.contains("clue-field__editor"), "{html}");
    }

    #[test]
    fn a_puzzle_with_no_answers_says_so() {
        let html = page(puzzle(&["###", "###", "###"]), |_| {});
        assert!(html.contains("This puzzle has no answers."), "{html}");
    }
}

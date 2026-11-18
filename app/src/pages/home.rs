//! Home: start a puzzle of any size, or carry on with one. Reached from the
//! logo or File ▸ Home; on launch the last puzzle opens as before.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::puzzle_list::{PuzzleList, PuzzleRow, SizeChoice, SizeTiles};
use crate::files;
use crate::library::{self, Summary};
use crate::pages::build_grid::Picker;
use crate::shell;
use crate::state::AppState;
use dioxus::prelude::*;
use std::path::PathBuf;
use std::time::SystemTime;
use xw_engine::templates::TEMPLATES;

/// The sizes Home offers, each pictured by its first standard grid.
pub fn size_choices() -> Vec<SizeChoice> {
    let mut out: Vec<SizeChoice> = vec![];
    for t in TEMPLATES {
        let side = t.rows.len();
        if out.iter().any(|s| s.side == side) {
            continue;
        }
        let blocks = t
            .rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        out.push(SizeChoice { side, blocks });
    }
    out.sort_by_key(|s| s.side);
    out
}

/// A puzzle's row, as of `now`.
pub fn row(s: &Summary, now: SystemTime) -> PuzzleRow {
    let setter = if s.author.is_empty() {
        "No setter"
    } else {
        &s.author
    };
    PuzzleRow {
        title: s.title.clone(),
        detail: format!("{setter} · {}×{}", s.width, s.height),
        columns: s.width,
        blocks: s.blocks.clone(),
        filled: s.filled,
        clued: s.clued,
        edited: library::ago(s.modified, now),
    }
}

/// Copy a puzzle into the Setter folder as "<title> copy".
fn duplicate(s: &Summary) -> Result<PathBuf, String> {
    let mut p = files::load(&s.path)?;
    p.title = format!("{} copy", s.title);
    let path = files::new_file_in(&files::library_dir(), &p.title);
    files::save(&path, &p)?;
    Ok(path)
}

#[component]
pub fn Home() -> Element {
    let mut state = use_context::<AppState>();
    let mut query = use_signal(String::new);
    // Bumped to read the folder again (after Duplicate).
    let mut refresh = use_signal(|| 0u32);
    let puzzles = use_resource(move || {
        let recent = state.settings.read().recent.clone();
        refresh();
        async move {
            tokio::task::spawn_blocking(move || library::list(&files::library_dir(), &recent))
                .await
                .unwrap_or_default()
        }
    });
    let sizes = size_choices();
    let sides: Vec<usize> = sizes.iter().map(|s| s.side).collect();
    let all = puzzles.read().clone().unwrap_or_default();
    let shown: Vec<Summary> = all
        .into_iter()
        .filter(|s| library::matches(s, &query()))
        .collect();
    let now = SystemTime::now();
    let rows: Vec<PuzzleRow> = shown.iter().map(|s| row(s, now)).collect();
    let paths: Vec<Summary> = shown.clone();
    let any = puzzles.read().as_ref().is_some_and(|p| !p.is_empty());
    rsx! {
        div { class: "home",
            div { class: "home__inner",
                h2 { class: "home__title", "New puzzle" }
                SizeTiles {
                    sizes,
                    on_pick: move |i: usize| {
                        state.build.picker_size.set(sides.get(i).copied());
                        state.build.picker_open.set(true);
                    },
                }
                if any {
                    div { class: "home__list-head",
                        h2 { class: "home__title", "Puzzles" }
                        input {
                            class: "home__filter",
                            r#type: "search",
                            "aria-label": "Filter puzzles",
                            placeholder: "Filter",
                            value: "{query}",
                            oninput: move |e| query.set(e.value()),
                        }
                        Button {
                            variant: ButtonVariant::Outline,
                            size: ButtonSize::Sm,
                            onclick: move |_| shell::open_from_home(state),
                            "Open…"
                        }
                    }
                    PuzzleList {
                        rows,
                        on_open: {
                            let paths = paths.clone();
                            move |i: usize| {
                                if let Some(s) = paths.get(i) {
                                    shell::open_path(state, s.path.clone());
                                }
                            }
                        },
                        on_action: move |(i, action): (usize, usize)| {
                            let Some(s) = paths.get(i) else {
                                return;
                            };
                            match action {
                                0 => shell::open_path(state, s.path.clone()),
                                1 => match duplicate(s) {
                                    Ok(_) => refresh += 1,
                                    Err(e) => state.notice.set(Some(e)),
                                },
                                _ => {
                                    if let Err(e) = files::reveal(&s.path) {
                                        state.notice.set(Some(format!("Couldn't show the folder: {e}")));
                                    }
                                }
                            }
                        },
                    }
                } else {
                    div { class: "home__list-head",
                        span {}
                        Button {
                            variant: ButtonVariant::Outline,
                            size: ButtonSize::Sm,
                            onclick: move |_| shell::open_from_home(state),
                            "Open…"
                        }
                    }
                }
            }
        }
        Picker {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn every_size_is_offered_once_smallest_first() {
        let sides: Vec<usize> = size_choices().iter().map(|s| s.side).collect();
        assert_eq!(sides, [5, 7, 9, 11, 13, 15]);
        let five = &size_choices()[0];
        assert_eq!(five.blocks.len(), 25);
    }

    #[test]
    fn a_row_names_the_setter_size_and_when() {
        let now = SystemTime::now();
        let s = Summary {
            path: Path::new("/x/p.xword").to_path_buf(),
            title: "Prize 12".into(),
            author: String::new(),
            width: 15,
            height: 15,
            blocks: vec![false; 225],
            filled: (36, 36),
            clued: (30, 36),
            modified: now - Duration::from_secs(120),
        };
        let r = row(&s, now);
        assert_eq!(r.detail, "No setter · 15×15");
        assert_eq!(r.edited, "2 minutes ago");
        assert_eq!(r.clued, (30, 36));
    }
}

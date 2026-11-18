//! About Setter: the version, where the source is, and the credits and
//! licences for the bundled data, fonts and components. Its own page from the
//! gear menu; each licence opens in place.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::prose::Prose;
use crate::files;
use crate::shell;
use crate::state::{update_line, AppState, UpdateAction};
use dioxus::prelude::*;
use std::collections::HashSet;

/// The licence notices, compiled in so they ship with every build.
static NOTICE: &str = include_str!("../../../data/NOTICE.md");

const ICON: Asset = asset!("/icons/128x128@2x.png");

/// One thing Setter is built on.
#[derive(Clone, PartialEq, Debug)]
pub struct Credit {
    pub name: &'static str,
    pub what: &'static str,
    /// The licence's short name, as a tag.
    pub licence: &'static str,
    /// Its full notice, from data/NOTICE.md.
    pub text: String,
}

/// (NOTICE.md heading it starts with, group, name, what it's for, licence)
const SOURCES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Word list",
        "Words and meanings",
        "UKACD18, via Exet's Lufz lexicon",
        "The word list, with popularity and CMUdict pronunciations",
        "BSD-style",
    ),
    (
        "Thesaurus: wordnet",
        "Words and meanings",
        "Open English WordNet",
        "Meanings and broader words, for definitions and synonyms",
        "CC BY 4.0",
    ),
    (
        "Thesaurus: moby",
        "Words and meanings",
        "Moby Thesaurus II",
        "Related words, by Grady Ward",
        "Public domain",
    ),
    (
        "Word quality",
        "Words and meanings",
        "Nediger list",
        "How good each word is in a grid",
        "MIT",
    ),
    (
        "Fonts",
        "Type and parts",
        "Inter and Newsreader",
        "The typefaces",
        "SIL Open Font Licence",
    ),
    (
        "Components",
        "Type and parts",
        "Dioxus components",
        "Buttons and toggles, restyled",
        "MIT / Apache-2.0",
    ),
    ("Icons", "Type and parts", "Lucide", "Icons", "ISC"),
];

/// The credits by group, each with its notice from data/NOTICE.md.
pub fn credits() -> Vec<(&'static str, Vec<Credit>)> {
    let sections: Vec<(&str, &str)> = NOTICE
        .split("\n## ")
        .skip(1)
        .filter_map(|s| s.split_once('\n'))
        .collect();
    let mut out: Vec<(&str, Vec<Credit>)> = vec![];
    for &(heading, group, name, what, licence) in SOURCES {
        let text = sections
            .iter()
            .find(|(h, _)| h.starts_with(heading))
            .map(|(_, body)| body.trim().to_string())
            .unwrap_or_default();
        let credit = Credit {
            name,
            what,
            licence,
            text,
        };
        match out.iter_mut().find(|(g, _)| *g == group) {
            Some((_, list)) => list.push(credit),
            None => out.push((group, vec![credit])),
        }
    }
    out
}

/// Where updating has got to, and the one thing to do about it: Check now,
/// Update, or Restart to update. On About and in Settings → General.
#[component]
pub fn UpdateStatus() -> Element {
    let state = use_context::<AppState>();
    let (line, action) = update_line(
        &(state.update)(),
        (state.update_checking)(),
        (state.update_checked)(),
        std::time::SystemTime::now(),
    );
    rsx! {
        span { class: "update-status",
            span { class: "update-status__line", "{line}" }
            match action {
                UpdateAction::Check => rsx! {
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        onclick: move |_| shell::check_for_updates(state, true),
                        "Check now"
                    }
                },
                UpdateAction::Wait => rsx! {
                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm, disabled: true, "Check now" }
                },
                UpdateAction::Update(next) => rsx! {
                    Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        onclick: move |_| shell::download_update(state, next.clone(), true),
                        "Update"
                    }
                },
                UpdateAction::Restart => rsx! {
                    Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        onclick: move |_| shell::restart_to_update(state),
                        "Restart to update"
                    }
                },
            }
        }
    }
}

const REPO: &str = "https://github.com/will-corrigan/setter";

#[component]
pub fn About() -> Element {
    let mut state = use_context::<AppState>();
    let mut open = use_signal(HashSet::<&'static str>::new);
    let version = env!("CARGO_PKG_VERSION");
    let back = crate::pages::settings::back_label(state);
    let link = move |url: &'static str| {
        move |_| {
            if let Err(e) = files::open_url(url) {
                state
                    .notice
                    .set(Some(format!("Couldn't open the browser: {e}")));
            }
        }
    };
    rsx! {
        div { class: "about",
            div { class: "about__column",
                button { class: "settings-nav__back", r#type: "button", onclick: move |_| state.credits.set(false),
                    span { "aria-hidden": "true", "‹" }
                    "{back}"
                }
                header { class: "about__hero",
                    img { class: "about__icon", src: ICON, alt: "" }
                    div { class: "about__name",
                        h1 { "Setter" }
                        div { class: "about__meta",
                            "Version {version}"
                            UpdateStatus {}
                        }
                    }
                    div { class: "about__links",
                        Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm, onclick: link(REPO), "Source on GitHub" }
                        Button {
                            variant: ButtonVariant::Outline,
                            size: ButtonSize::Sm,
                            onclick: link("https://github.com/will-corrigan/setter/blob/main/LICENSE"),
                            "MIT licence"
                        }
                    }
                }
                p { class: "about__blurb",
                    "For setting British cryptic crosswords. Setter is built on the work of the people and projects below."
                }
                for (group , list) in credits() {
                    section { key: "{group}", class: "settings-group",
                        h2 { class: "settings-group__title", "{group}" }
                        div { class: "settings-group__card",
                            for credit in list {
                                div {
                                    key: "{credit.name}",
                                    class: if open().contains(credit.name) { "credit is-open" } else { "credit" },
                                    div { class: "credit__head",
                                        div { class: "credit__name",
                                            b { "{credit.name}" }
                                            span { "{credit.what}" }
                                        }
                                        span { class: "credit__licence", "{credit.licence}" }
                                        button {
                                            class: "list-editor__link",
                                            r#type: "button",
                                            "aria-expanded": "{open().contains(credit.name)}",
                                            onclick: move |_| {
                                                let mut set = open.write();
                                                if !set.remove(credit.name) {
                                                    set.insert(credit.name);
                                                }
                                            },
                                            if open().contains(credit.name) { "Hide" } else { "Licence" }
                                        }
                                    }
                                    if open().contains(credit.name) {
                                        div { class: "credit__text", Prose { text: credit.text.clone() } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credits_cover_every_bundled_source() {
        let all: String = credits()
            .iter()
            .flat_map(|(_, list)| {
                list.iter()
                    .map(|c| format!("{} {} {}", c.name, c.what, c.text))
            })
            .collect::<Vec<_>>()
            .join("\n");
        for name in [
            "UKACD18",
            "Exet",
            "CMUdict",
            "Open English WordNet",
            "Princeton",
            "Moby",
            "DioxusLabs",
            "Lucide",
            "Nediger",
        ] {
            assert!(all.contains(name), "missing {name}");
        }
        for name in xw_engine::fonts::names() {
            assert!(all.contains(name), "missing font {name}");
        }
    }

    #[test]
    fn every_credit_has_its_licence_text() {
        for (_, list) in credits() {
            for c in list {
                assert!(c.text.len() > 40, "{} has no notice", c.name);
            }
        }
    }
}

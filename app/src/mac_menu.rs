//! The menu bar along the top of a Mac's screen: Setter, File, Edit, Puzzle
//! and Window, as Mac apps have. (Windows and Linux keep the menus in the
//! window's own top bar.) The items show the setter's current keys and run
//! the same actions as the in-window menus. The Edit menu is the system's
//! own, which is what makes ⌘C, ⌘V and ⌘A work in text boxes.

use crate::bindings::{self, Combo};
use crate::export::ExportFormat;
use crate::state::Shortcut;
use dioxus::desktop::muda::accelerator::{Accelerator, Code, Modifiers};
use dioxus::desktop::muda::{Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu, SubmenuBuilder};
use std::cell::RefCell;

/// What a menu item asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pick {
    Shortcut(Shortcut),
    Export(ExportFormat),
    NewSample,
    Home,
    About,
}

const PREFIX: &str = "setter:";

fn id_of(pick: Pick) -> String {
    let name = match pick {
        Pick::Shortcut(s) => s.id(),
        Pick::Export(ExportFormat::WebPage) => return id_of(Pick::Shortcut(Shortcut::Export)),
        Pick::Export(ExportFormat::Puz) => "export-puz",
        Pick::Export(ExportFormat::Ipuz) => "export-ipuz",
        Pick::NewSample => "new-sample",
        Pick::Home => "home",
        Pick::About => "about",
    };
    format!("{PREFIX}{name}")
}

/// The action for a menu event's id, if it's one of ours.
pub fn pick_for(id: &MenuId) -> Option<Pick> {
    let name = id.0.strip_prefix(PREFIX)?;
    Some(match name {
        "export-puz" => Pick::Export(ExportFormat::Puz),
        "export-ipuz" => Pick::Export(ExportFormat::Ipuz),
        "new-sample" => Pick::NewSample,
        "home" => Pick::Home,
        "about" => Pick::About,
        other => Pick::Shortcut(Shortcut::ALL.into_iter().find(|s| s.id() == other)?),
    })
}

/// The physical key for a keydown's `key` (as [`Combo`] keeps it).
fn code_of(key: &str) -> Option<Code> {
    let name = match key {
        k if k.len() == 1 && k.as_bytes()[0].is_ascii_lowercase() => {
            format!("Key{}", k.to_ascii_uppercase())
        }
        k if k.len() == 1 && k.as_bytes()[0].is_ascii_digit() => format!("Digit{k}"),
        "," => "Comma".into(),
        "." => "Period".into(),
        "/" => "Slash".into(),
        ";" => "Semicolon".into(),
        "'" => "Quote".into(),
        "[" => "BracketLeft".into(),
        "]" => "BracketRight".into(),
        "-" => "Minus".into(),
        "=" => "Equal".into(),
        "`" => "Backquote".into(),
        "\\" => "Backslash".into(),
        " " => "Space".into(),
        "arrowleft" => "ArrowLeft".into(),
        "arrowright" => "ArrowRight".into(),
        "arrowup" => "ArrowUp".into(),
        "arrowdown" => "ArrowDown".into(),
        "enter" => "Enter".into(),
        "tab" => "Tab".into(),
        "backspace" => "Backspace".into(),
        "delete" => "Delete".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "pageup" => "PageUp".into(),
        "pagedown" => "PageDown".into(),
        k if k.len() > 1 && k.starts_with('f') && k[1..].parse::<u8>().is_ok() => k.to_uppercase(),
        _ => return None,
    };
    name.parse().ok()
}

/// The menu's key equivalent for a binding: its "Ctrl" is ⌘.
pub fn accelerator(combo: &Combo) -> Option<Accelerator> {
    let mut mods = Modifiers::empty();
    if combo.ctrl {
        mods |= Modifiers::META;
    }
    if combo.alt {
        mods |= Modifiers::ALT;
    }
    if combo.shift {
        mods |= Modifiers::SHIFT;
    }
    Some(Accelerator::new(Some(mods), code_of(&combo.key)?))
}

thread_local! {
    /// The items that carry a binding, to update when the keys change.
    static BOUND: RefCell<Vec<(Shortcut, MenuItem)>> = const { RefCell::new(vec![]) };
}

fn item(text: &str, pick: Pick) -> MenuItem {
    let keys = match pick {
        Pick::Shortcut(s) => accelerator(&bindings::combo(s)),
        _ => None,
    };
    let item = MenuItem::with_id(id_of(pick), text, true, keys);
    if let Pick::Shortcut(s) = pick {
        BOUND.with(|b| b.borrow_mut().push((s, item.clone())));
    }
    item
}

fn shortcut(text: &str, s: Shortcut) -> MenuItem {
    item(text, Pick::Shortcut(s))
}

/// The whole menu bar.
pub fn menu_bar() -> Menu {
    BOUND.with(|b| b.borrow_mut().clear());
    let sep = PredefinedMenuItem::separator;
    let app = SubmenuBuilder::new()
        .text("Setter")
        .item(&item("About Setter", Pick::About))
        .item(&sep())
        .item(&shortcut("Settings…", Shortcut::Settings))
        .item(&sep())
        .item(&PredefinedMenuItem::services(None))
        .item(&sep())
        .item(&PredefinedMenuItem::hide(None))
        .item(&PredefinedMenuItem::hide_others(None))
        .item(&PredefinedMenuItem::show_all(None))
        .item(&sep())
        .item(&PredefinedMenuItem::quit(None))
        .build()
        .expect("app menu");
    let file = SubmenuBuilder::new()
        .text("File")
        .item(&item("Home", Pick::Home))
        .item(&sep())
        .item(&shortcut("New Puzzle…", Shortcut::New))
        .item(&item("New Filled Sample", Pick::NewSample))
        .item(&shortcut("Open…", Shortcut::Open))
        .item(&sep())
        .item(&PredefinedMenuItem::close_window(None))
        .item(&shortcut("Save", Shortcut::Save))
        .item(&shortcut("Save As…", Shortcut::SaveAs))
        .item(&sep())
        .item(&shortcut("Export Web Page…", Shortcut::Export))
        .item(&item(
            "Export Across Lite (.puz)…",
            Pick::Export(ExportFormat::Puz),
        ))
        .item(&item("Export ipuz…", Pick::Export(ExportFormat::Ipuz)))
        .build()
        .expect("file menu");
    // The system's own: these act on whatever has the focus, so they work in
    // every text box. On the grid, the page takes ⌘Z first for its own undo.
    let edit = SubmenuBuilder::new()
        .text("Edit")
        .item(&PredefinedMenuItem::undo(None))
        .item(&PredefinedMenuItem::redo(None))
        .item(&sep())
        .item(&PredefinedMenuItem::cut(None))
        .item(&PredefinedMenuItem::copy(None))
        .item(&PredefinedMenuItem::paste(None))
        .item(&PredefinedMenuItem::select_all(None))
        .build()
        .expect("edit menu");
    let puzzle = SubmenuBuilder::new()
        .text("Puzzle")
        .item(&shortcut("Search…", Shortcut::Palette))
        .item(&sep())
        .item(&shortcut("Previous Answer", Shortcut::PreviousAnswer))
        .item(&shortcut("Next Answer", Shortcut::NextAnswer))
        .item(&shortcut(
            "Next Answer Without a Clue",
            Shortcut::NextUnclued,
        ))
        .item(&sep())
        .item(&shortcut("Words That Fit", Shortcut::Words))
        .build()
        .expect("puzzle menu");
    let window: Submenu = SubmenuBuilder::new()
        .text("Window")
        .item(&PredefinedMenuItem::minimize(None))
        .item(&PredefinedMenuItem::maximize(None))
        .item(&PredefinedMenuItem::fullscreen(None))
        .item(&sep())
        .item(&PredefinedMenuItem::bring_all_to_front(None))
        .build()
        .expect("window menu");
    #[cfg(target_os = "macos")]
    window.set_as_windows_menu_for_nsapp();
    let bar = Menu::new();
    bar.append_items(&[&app, &file, &edit, &puzzle, &window])
        .expect("menu bar");
    bar
}

/// Show the setter's current keys in the menus (after they change some).
pub fn refresh() {
    BOUND.with(|b| {
        for (s, item) in b.borrow().iter() {
            let _ = item.set_accelerator(accelerator(&bindings::combo(*s)));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pick_comes_back_from_its_id() {
        let mut picks: Vec<Pick> = Shortcut::ALL.into_iter().map(Pick::Shortcut).collect();
        picks.extend([
            Pick::Export(ExportFormat::Puz),
            Pick::Export(ExportFormat::Ipuz),
            Pick::NewSample,
            Pick::Home,
            Pick::About,
        ]);
        for p in picks {
            assert_eq!(pick_for(&MenuId::new(id_of(p))), Some(p), "{p:?}");
        }
        assert_eq!(pick_for(&MenuId::new("dioxus-float-top")), None);
    }

    #[test]
    fn every_default_binding_has_a_menu_key() {
        for s in Shortcut::ALL {
            let combo = Combo::parse(s.default_keys()).unwrap();
            assert!(accelerator(&combo).is_some(), "{s:?}");
        }
    }

    #[test]
    fn ctrl_becomes_command() {
        let a = accelerator(&Combo::parse("Ctrl+Shift+S").unwrap()).unwrap();
        assert_eq!(
            a,
            Accelerator::new(Some(Modifiers::META | Modifiers::SHIFT), Code::KeyS)
        );
        let a = accelerator(&Combo::parse("Alt+←").unwrap()).unwrap();
        assert_eq!(a, Accelerator::new(Some(Modifiers::ALT), Code::ArrowLeft));
    }
}

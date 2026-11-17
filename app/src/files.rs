//! Files: the user's config folder, puzzle files and settings.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use xw_engine::fill::{autofill, FillIndex, FillOptions};
use xw_engine::grid::Grid;
use xw_engine::puzzle::Puzzle;
use xw_engine::thesaurus::Thesaurus;
use xw_engine::{text, Abbreviations, Indicators, Lexicon};

/// Config-file templates written to the user's config folder on first run.
pub const USER_FILES: &[(&str, &str)] = &[
    (
        "words.txt",
        "# Extra words and phrases, one per line. These are treated as the most popular\n\
         # entries, so they come first in fill suggestions and wordplay results.\n",
    ),
    (
        "banned.txt",
        "# Words to never suggest, one per line (letters only, any case).\n",
    ),
    (
        "abbreviations.toml",
        "# Your abbreviations, merged with the built-in list.\n\
         # An empty list removes a built-in entry.\n\
         [abbreviations]\n\
         # zz = [\"sleep\", \"snore\"]\n",
    ),
    (
        "indicators.toml",
        "# Your indicator words, appended to the built-in lists.\n\
         # [anagram]\n\
         # words = [\"discombobulated\"]\n",
    ),
    (
        "theme.css",
        "/* Your CSS, loaded after the app's own styles. Every colour and font is a
\
            CSS variable; override any of them here, e.g.
\
         :root { --canvas: #f7f4ee; --p1: #0b5cad; }
\
         :root[data-theme=dark] { --canvas: #10141c; }
\
         */
",
    ),
];

fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// The user's config folder: `$SETTER_CONFIG_DIR`, else `<OS config dir>/setter`.
/// Tests get a scratch folder of their own, so they never touch the user's.
pub fn config_dir() -> PathBuf {
    if cfg!(test) {
        let d = std::env::temp_dir().join(format!("setter-tests-{}", std::process::id()));
        let _ = fs::create_dir_all(&d);
        return d;
    }
    let d = std::env::var_os("SETTER_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("setter")
        });
    let _ = fs::create_dir_all(&d);
    d
}

/// Write each config-file template that doesn't exist yet.
pub fn ensure_user_files() {
    ensure_user_files_in(&config_dir());
}

pub fn ensure_user_files_in(dir: &Path) {
    let _ = fs::create_dir_all(dir);
    for (name, template) in USER_FILES {
        let p = dir.join(name);
        if !p.exists() {
            let _ = fs::write(p, template);
        }
    }
}

/// Add a word or phrase of the user's own to `words.txt` (once).
pub fn add_own_word(text: &str) -> Result<(), String> {
    add_own_word_in(&config_dir(), text)
}

pub fn add_own_word_in(dir: &Path, text: &str) -> Result<(), String> {
    let path = dir.join("words.txt");
    let entry = text.trim().to_lowercase();
    if entry.is_empty() || read_lines(&path).iter().any(|l| l.to_lowercase() == entry) {
        return Ok(());
    }
    let mut contents = fs::read_to_string(&path).unwrap_or_default();
    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(&entry);
    contents.push('\n');
    fs::write(&path, contents).map_err(|e| format!("Couldn't add the word: {e}"))
}

/// Everything the engine needs, with the user's own lists merged in.
pub struct EngineData {
    pub lex: Lexicon,
    pub abbr: Abbreviations,
    pub ind: Indicators,
    pub th: Thesaurus,
    /// Problems reading the user's files, shown once in the UI.
    pub warnings: Vec<String>,
}

pub fn load_engine_data() -> EngineData {
    load_engine_data_from(&config_dir())
}

pub fn load_engine_data_from(dir: &Path) -> EngineData {
    let mut warnings = vec![];
    let extra = read_lines(&dir.join("words.txt"));
    let banned: HashSet<String> = read_lines(&dir.join("banned.txt"))
        .iter()
        .map(|w| text::key(w))
        .collect();
    let lex = Lexicon::bundled(&extra, &banned);
    let mut abbr = Abbreviations::bundled();
    if let Ok(src) = fs::read_to_string(dir.join("abbreviations.toml")) {
        if let Err(e) = abbr.merge_toml(&src) {
            warnings.push(format!("abbreviations.toml: {e}"));
        }
    }
    let mut ind = Indicators::bundled();
    if let Ok(src) = fs::read_to_string(dir.join("indicators.toml")) {
        if let Err(e) = ind.merge_toml(&src) {
            warnings.push(format!("indicators.toml: {e}"));
        }
    }
    EngineData {
        lex,
        abbr,
        ind,
        th: Thesaurus::bundled(),
        warnings,
    }
}

/// The user's extra CSS (empty if none).
pub fn user_css() -> String {
    fs::read_to_string(config_dir().join("theme.css")).unwrap_or_default()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub last_file: Option<PathBuf>,
    /// Put on new puzzles and shown in exports.
    #[serde(default)]
    pub setter_name: String,
}

/// A `<head>` script applying a saved Light/Dark theme before the first paint
/// (System needs nothing: the CSS follows the OS).
pub fn theme_head(theme: Theme) -> String {
    match theme {
        Theme::System => String::new(),
        Theme::Light => "<script>document.documentElement.dataset.theme='light'</script>".into(),
        Theme::Dark => "<script>document.documentElement.dataset.theme='dark'</script>".into(),
    }
}

pub fn load_settings() -> Settings {
    load_settings_from(&config_dir())
}

pub fn load_settings_from(dir: &Path) -> Settings {
    fs::read_to_string(dir.join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(s: &Settings) {
    save_settings_to(&config_dir(), s);
}

pub fn save_settings_to(dir: &Path, s: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = fs::write(dir.join("settings.json"), json);
    }
}

/// Read a `.xword` puzzle file.
pub fn load(path: &Path) -> Result<Puzzle, String> {
    let s =
        fs::read_to_string(path).map_err(|e| format!("Couldn't open {}: {e}", path.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("Not a puzzle file: {e}"))
}

/// Write a `.xword` puzzle file (pretty JSON).
pub fn save(path: &Path, p: &Puzzle) -> Result<(), String> {
    let s = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
    write_whole(path, s.as_bytes()).map_err(|e| format!("Couldn't save {}: {e}", path.display()))
}

/// Write a file so that a crash part-way leaves the old one intact: write a
/// temporary file beside it, then rename it into place.
pub fn write_whole(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".saving");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// The British 15×15 template used for sample puzzles.
pub const BRITISH_15: &[&str] = xw_engine::templates::TEMPLATES[0].rows;

/// A filled 15×15 puzzle to start clueing straight away.
pub fn sample_puzzle(lex: &Lexicon) -> Result<Puzzle, String> {
    sample_puzzle_with(lex, BRITISH_15, Duration::from_secs(20))
}

/// Autofill `rows` into a new puzzle, or say why it couldn't be filled.
pub fn sample_puzzle_with(lex: &Lexicon, rows: &[&str], limit: Duration) -> Result<Puzzle, String> {
    let mut p = Puzzle::new(rows[0].len(), rows.len());
    p.title = "Sample puzzle".into();
    p.grid = Grid::from_rows(rows);
    let index = FillIndex::build(lex, FillOptions::default());
    let cancel = AtomicBool::new(false);
    let result = autofill(lex, &index, &p.grid, limit, 0.4, &cancel);
    if !result.ok {
        return Err(format!("Couldn't fill a sample grid: {}.", result.message));
    }
    p.grid = result.grid;
    for (id, entry) in result.entries {
        let en = text::enumeration(&entry);
        if en.contains([',', '-']) {
            p.clues.entry(id).or_default().enumeration = en;
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("setter-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn saved_theme_is_set_before_first_paint() {
        assert_eq!(theme_head(Theme::System), "");
        assert!(theme_head(Theme::Dark).contains("dataset.theme='dark'"));
        assert!(theme_head(Theme::Light).contains("dataset.theme='light'"));
    }

    #[test]
    fn own_words_are_added_once_and_count_as_words() {
        let d = temp_dir("ownwords");
        ensure_user_files_in(&d);
        add_own_word_in(&d, "zob rist").unwrap();
        add_own_word_in(&d, "zob rist").unwrap();
        let text = fs::read_to_string(d.join("words.txt")).unwrap();
        assert_eq!(text.lines().filter(|l| *l == "zob rist").count(), 1);
        assert!(load_engine_data_from(&d).lex.contains("zobrist"));
    }

    #[test]
    fn puzzle_round_trips() {
        let d = temp_dir("roundtrip");
        let mut p = xw_engine::puzzle::Puzzle::new(3, 3);
        p.title = "Test".into();
        let path = d.join("t.xword");
        save(&path, &p).unwrap();
        assert_eq!(load(&path).unwrap(), p);
    }

    #[test]
    fn junk_files_are_rejected() {
        let d = temp_dir("junk");
        let path = d.join("bad.xword");
        fs::write(&path, "not json").unwrap();
        assert!(load(&path).unwrap_err().contains("Not a puzzle file"));
    }

    #[test]
    fn user_files_customise_engine_data() {
        let d = temp_dir("engine");
        ensure_user_files_in(&d);
        assert!(d.join("theme.css").exists());
        fs::write(d.join("banned.txt"), "orchestra\n").unwrap();
        fs::write(d.join("abbreviations.toml"), "this is = not [toml").unwrap();
        let data = load_engine_data_from(&d);
        assert_eq!(data.lex.lookup("orchestra"), None);
        assert_eq!(data.warnings.len(), 1);
    }

    #[test]
    fn setter_name_is_remembered() {
        let d = temp_dir("settername");
        let s = Settings {
            setter_name: "Gussalufz".into(),
            ..Default::default()
        };
        save_settings_to(&d, &s);
        assert_eq!(load_settings_from(&d).setter_name, "Gussalufz");
    }

    #[test]
    fn settings_default_on_garbage() {
        let d = temp_dir("settings");
        fs::write(d.join("settings.json"), "{oops").unwrap();
        assert_eq!(load_settings_from(&d).theme, Theme::System);
        let s = Settings {
            theme: Theme::Dark,
            last_file: Some("x.xword".into()),
            ..Default::default()
        };
        save_settings_to(&d, &s);
        assert_eq!(load_settings_from(&d), s);
    }

    #[test]
    fn unfillable_sample_reports_an_error() {
        let lex = Lexicon::from_words(&["cat"]);
        let err =
            sample_puzzle_with(&lex, BRITISH_15, std::time::Duration::from_secs(2)).unwrap_err();
        assert!(err.contains("Couldn't fill a sample grid"), "{err}");
    }

    #[test]
    fn sample_puzzle_is_fully_filled_with_real_words() {
        let data = load_engine_data_from(&temp_dir("sample"));
        let p = sample_puzzle(&data.lex).unwrap();
        assert!(p.grid.cells.iter().all(|c| c.block || c.letter.is_some()));
        for l in p.grid.lights() {
            assert!(
                data.lex.contains(&p.grid.pattern(&l)),
                "{}",
                p.grid.pattern(&l)
            );
        }
    }

    #[test]
    fn saving_replaces_the_file_whole() {
        let d = std::env::temp_dir().join(format!("setter-whole-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        let path = d.join("p.xword");
        fs::write(&path, "old").unwrap();
        let mut p = Puzzle::new(3, 3);
        p.title = "New".into();
        save(&path, &p).unwrap();
        assert_eq!(load(&path).unwrap().title, "New");
        // Nothing left over beside it.
        let names: Vec<_> = fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("p.xword")]);
        let _ = fs::remove_dir_all(&d);
    }


    #[test]
    fn tests_never_use_the_user_s_config_folder() {
        let d = config_dir();
        assert!(d.starts_with(std::env::temp_dir()), "{}", d.display());
        assert!(!d.ends_with("setter"));
    }

}

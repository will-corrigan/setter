# Write Clues Screen (Dioxus) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Dioxus desktop app whose "Write clues" screen is the approved word-bank design: a device list, a colour-coded clue box, and wide word-bank columns for every part of every device. It is driven by the existing `xw-engine` plus online synonyms.

**Architecture:**
- All word logic stays in `crates/engine`. A new `bank` module turns finder results into a framework-free view model (devices → options → columns), which is unit-tested.
- A new `app` crate (Dioxus 0.7 desktop) renders that model with plain CSS. It fills the online columns from Datamuse and owns the puzzle file and settings.
- The Svelte/Tauri scaffolding is removed.

**Tech Stack:**
- Rust 1.99 (Nix flake), Dioxus 0.7 desktop (wry/WebKitGTK on Linux, WebView2 on Windows), `dx` CLI 0.7.
- `reqwest` (rustls) for Datamuse, `rfd` for file dialogs, `dirs` for the config folder.
- Bundled fonts: Newsreader and Instrument Sans (woff2, OFL).

**Spec:** `docs/superpowers/specs/2026-11-16-write-clues-screen-design.md`. The visual reference is `docs/design/write-clues-mockup.html` (clickable) and `docs/design/devices-gallery-mockup.html`. Copy the CSS tokens and classes from the mockup rather than reinventing them.

## Global Constraints

- **Devices:** two groups, *Wordplay* then *Meaning only*, each sorted **alphabetically**:
  - Wordplay: Alternate letters, Anagram, Change a letter, Charade, Container, Deletion, First letters, Hidden, Reversal, Sounds like, Spoonerism.
  - Meaning only: Cryptic definition, Double definition.
- Every device row shows a count. Devices with zero options are greyed but still selectable.
- **No hand-holding:** no numbered steps, no explanatory paragraphs, no intro text on the screen.
- **Copy:**
  - Don't use "surface" or "recipe" anywhere in the UI. "Device" and "Definition" are fine.
  - Column titles follow the mockup, e.g. "Definition", "Joining words", "Mixing words", "Hiding words", "Putting-in words", "Removal words", "Backwards words", "Sound words", "Swap words", "Every-other words", "Initial words", "Spooner words".
- **Part colours, in order:** p1 blue, p2 violet, p3 amber, p4 teal, p5 pink. These are shared by the tiles, the column header and the matching words in the clue.
- **Definition colours:** green. Indicator/link: dotted grey.
- **Light and dark themes:** follow the system by default, with a toggle in the top bar. Every colour is a CSS variable. The user's `theme.css` in the config folder loads last.
- Each word-bank column has a filter box, a count, grouped items, and its own scroll. Clicking an item selects it in the column's colour.
- Online columns, when offline, show exactly one line: `Needs an internet connection.` Everything else works offline.
- **Build commands:** builds and runs go through `dx` (bundled assets only load under `dx`). Pin `dioxus = "0.7"`.
- **Platforms:** Windows is the target; development happens on NixOS-in-WSL. Linux needs `WEBKIT_DISABLE_DMABUF_RENDERER=1`.

## Review Focus

1. **Answer with empty squares**, or not in the lexicon. The screen shows the tiles with gaps, all device counts read 0, and the clue box still works. No panic. *(Test in Task 4.)*
2. **Multi-word answers** like IT IS (2,2). Analysis uses letters only, and the enumeration comes from the clue's own enumeration. *(Test in Task 4.)*
3. **Offline.** Online columns show the one-line notice, and offline columns still populate. *(Test in Task 6.)*
4. **Clue text with case, punctuation and possessives** ("Artist's", "BOX,"). Colouring matches whole words, case-insensitively, and ignores trailing punctuation. *(Test in Task 8.)*
5. **3-letter and 15-letter answers.** Finders return without panic. Tiles and columns don't overflow the window: columns scroll horizontally once there are more than 7. *(Test in Task 4; visual check in Task 9.)*

---

## File Structure

```
Cargo.toml                       workspace: members = ["crates/engine", "app"]
flake.nix                        + dioxus-cli, clang linker, mold, runtime libs, wsl lib path
crates/engine/src/wordplay/spoonerism.rs   new finder
crates/engine/src/wordplay/initials.rs     words by first/last letter
crates/engine/src/bank/mod.rs    view-model types + build()
crates/engine/src/bank/devices.rs one fn per device: finder result -> DeviceView
app/Cargo.toml, app/Dioxus.toml
app/assets/main.css              tokens + components (from the mockup)
app/assets/fonts/*.woff2         Newsreader + Instrument Sans (latin, variable)
app/src/main.rs                  launch, window config, root component
app/src/state.rs                 AppState: puzzle, path, current answer, picks, settings
app/src/answers.rs               list/navigate answers from a Puzzle (pure, tested)
app/src/online.rs                Datamuse client + cache + response parsing (tested)
app/src/highlight.rs             clue-text colouring + checks (pure, tested)
app/src/files.rs                 open/save .xword, sample puzzle, config dir, settings.json
app/src/ui/{topbar,devices,head,cluebox,bank}.rs   components
scripts/winshot.ps1              window screenshot helper (from the benchmark)
```

Delete: `src-tauri/`, `ui/`.

---

### Task 1: Switch the workspace to Dioxus and show a themed empty window

**Files:**
- Delete: `src-tauri/`, `ui/` (copy the fonts out first, see Step 1)
- Modify: `Cargo.toml`, `flake.nix`, `.gitignore`, `data/NOTICE.md`
- Create: `app/Cargo.toml`, `app/Dioxus.toml`, `app/src/main.rs`, `app/assets/main.css`, `app/assets/fonts/*`, `scripts/winshot.ps1`

**Interfaces:**
- Produces: a binary crate `setter` (package name `setter`). `main()` launches `App` via `dioxus::LaunchBuilder::desktop().with_cfg(Config::new().with_menu(None).with_window(WindowBuilder::new().with_title("Setter").with_inner_size(LogicalSize::new(1440.0, 920.0))))`.

- [ ] **Step 1: Save the fonts, then delete the old frontends.**
  - Copy these into `app/assets/fonts/`:
    - `ui/node_modules/@fontsource-variable/newsreader/files/newsreader-latin-opsz-normal.woff2`
    - `ui/node_modules/@fontsource-variable/newsreader/files/newsreader-latin-opsz-italic.woff2`
    - `ui/node_modules/@fontsource-variable/instrument-sans/files/instrument-sans-latin-wght-normal.woff2`
  - Copy `/tmp/gui-spikes/tools/winshot.ps1` to `scripts/`. Copy the `USER_FILES` constant (the config-file templates) and the `read_lines` helper from `src-tauri/src/state.rs` into a new `app/src/files.rs`; Task 7 builds on it.
  - Then `rm -rf src-tauri ui`.
  - Add the SIL OFL 1.1 attribution for both font families to `data/NOTICE.md`.
- [ ] **Step 2: Update the workspace and the flake.**
  - Workspace members become `["crates/engine", "app"]`.
  - In `flake.nix`:
    - Drop `cargo-tauri` and `nodejs_22`.
    - Add `dioxus-cli`, `clang`, `mold`, plus the runtime libs `wayland libxkbcommon libGL fontconfig freetype xdotool` to `libs`.
    - Add to `shellHook`:
      - `export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath libs}:/usr/lib/wsl/lib:$LD_LIBRARY_PATH"`
      - `export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang` (gcc's collect2 hits "Argument list too long" in this shell)
      - `export RUSTFLAGS="-C link-arg=-fuse-ld=mold"`
  - Add `/app/target`, `/target/dx`, `/app/dist` to `.gitignore`.
- [ ] **Step 3: Create the `app` crate.**
  - Dependencies: `dioxus = { version = "0.7", features = ["desktop"] }`, `xw-engine = { path = "../crates/engine" }`, `serde`, `serde_json`, `reqwest = { version = "0.12", default-features = false, features = ["json","rustls-tls"] }`, `rfd = "0.15"`, `dirs = "6"`, `tokio = { version = "1", features = ["time"] }`.
  - `Dioxus.toml`: `[application] name = "setter"`, `default_platform = "desktop"`.
  - `main.css`:
    - Copy the `:root`/`.app` token block **and** the `.dark` block from `docs/design/write-clues-mockup.html`. Rename `.app.dark` to `:root[data-theme=dark]`, and also apply it under `@media (prefers-color-scheme: dark) { :root:not([data-theme=light]) {…} }`.
    - Add `@font-face` rules pointing at the three fonts via `asset!`. Build them in a `style {}` element in Rust, as the benchmark spike did with `FONT`.
  - The `App` component renders the top bar shell: logo "Setter", mode switch "Build grid | Write clues", theme button.
- [ ] **Step 4: Build and launch.**
  - Run `nix develop --command bash -c 'cd app && dx build --platform desktop'`. Expected: a successful build.
  - Then launch the built binary. Screenshot it with `powershell.exe -File scripts/winshot.ps1 -Title Setter -Out C:\\Users\\Public\\setter.png`. Expected: the top bar in Newsreader/Instrument Sans.
  - `cargo test -p xw-engine` still passes.
- [ ] **Step 5: Commit** — `git add -A && git commit -m "build: replace Tauri/Svelte with Dioxus desktop app shell"`

---

### Task 2: Spoonerism finder

**Files:**
- Create: `crates/engine/src/wordplay/spoonerism.rs`; add `pub mod spoonerism;` to `wordplay/mod.rs`

**Interfaces:**
- Produces:
  - `pub struct Spoonerism { pub answer_parts: [String; 2], pub from: [WordRef; 2] }` (Serialize)
  - `pub fn find(ctx: Ctx, target: &str) -> Vec<Spoonerism>`, sorted by `max(rank)` ascending, capped at 60.
- Algorithm (spelling-based):
  - For each split of the target key into `x + y`, where both are component words of 2+ letters:
    - Let `onset(w)` be the leading run of consonants, treating `qu` as a consonant cluster. If either onset is empty, skip the split.
    - Form `a = onset(y) + rest(x)` and `b = onset(x) + rest(y)`.
  - Keep the split if both `a` and `b` are component words, with `a != x`.

- [ ] **Step 1: Write the failing tests.**
```rust
#[test] fn butterfly_is_flutter_by() {
    let s = find(ctx(), "butterfly");
    assert!(s.iter().any(|s| s.from[0].key == "flutter" && s.from[1].key == "by"
        && s.answer_parts == ["butter".to_string(), "fly".to_string()]));
}
#[test] fn short_or_vowel_only_returns_empty() {
    assert!(find(ctx(), "an").is_empty());
    assert!(find(ctx(), "aeiou").is_empty());
}
```
- [ ] **Step 2:** Run `nix develop --command cargo test -p xw-engine spoonerism`. Expected: FAIL (module missing).
- [ ] **Step 3:** Implement `find` as specified.
- [ ] **Step 4:** Run the same command. Expected: PASS.
- [ ] **Step 5: Commit** — `feat(engine): spoonerism finder`

---

### Task 3: Words by first / last letter

**Files:**
- Create: `crates/engine/src/wordplay/initials.rs`; register it in `wordplay/mod.rs`

**Interfaces:**
- Produces:
  - `pub fn starting_with(ctx: Ctx, letter: char, limit: usize) -> Vec<WordRef>`
  - `pub fn ending_with(ctx: Ctx, letter: char, limit: usize) -> Vec<WordRef>`
  - Both return component words with `id < ctx.lex.common_cutoff`, most popular first.

- [ ] **Step 1: Write the failing test.**
```rust
#[test] fn initials() {
    let t = starting_with(ctx(), 't', 40);
    assert_eq!(t.len(), 40);
    assert!(t.iter().all(|w| w.key.starts_with('t')));
    assert!(t.iter().any(|w| w.key == "the"));
    assert!(ending_with(ctx(), 'o', 20).iter().all(|w| w.key.ends_with('o')));
    assert!(starting_with(ctx(), '7', 10).is_empty());
}
```
- [ ] **Step 2:** Run `cargo test -p xw-engine initials`. Expected: FAIL.
- [ ] **Step 3:** Implement by scanning ids `0..common_cutoff`.
- [ ] **Step 4:** Run it again. Expected: PASS.
- [ ] **Step 5: Commit** — `feat(engine): words by first/last letter`

---

### Task 4: Word-bank view model (`bank`), offline devices

**Files:**
- Create: `crates/engine/src/bank/mod.rs`, `crates/engine/src/bank/devices.rs`; add `pub mod bank;` to `lib.rs`

**Interfaces:**
- Consumes: every finder in `wordplay::*` (existing), plus `spoonerism::find` (Task 2) and `initials::*` (Task 3).
- Produces:
```rust
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Group { Wordplay, MeaningOnly }
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DeviceKind { AlternateLetters, Anagram, ChangeALetter, Charade, Container, Deletion,
    FirstLetters, Hidden, Reversal, SoundsLike, Spoonerism, CrypticDefinition, DoubleDefinition }
impl DeviceKind { pub fn name(self) -> &'static str; pub fn group(self) -> Group; pub const ALL: [DeviceKind; 13]; }
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone { P1, P2, P3, P4, P5, Definition, Indicator }
#[derive(Serialize, Clone, Debug)] pub struct Tile { pub letters: String, pub tone: Option<Tone>, pub struck: bool }
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub enum Online { Meanings(String) /* synonyms+means-like of a word */, Definitions(String),
    Senses(String) /* double definition */, Linked(String) /* cryptic definition */ }
#[derive(Serialize, Clone, Debug)]
pub struct Column { pub title: String, pub tone: Tone, pub wide: bool,
    pub groups: Vec<(String, Vec<String>)>, pub online: Option<Online>,
    /// item text -> highlighted letter ranges (hidden / alternate letters)
    pub marks: Vec<(String, Vec<(usize, usize)>)> }
#[derive(Serialize, Clone, Debug)]
pub struct DeviceOption { pub label: String, pub tiles: Vec<Tile>, pub columns: Vec<Column>, pub reads_as: String }
#[derive(Serialize, Clone, Debug)]
pub struct DeviceView { pub kind: DeviceKind, pub count: usize, pub options: Vec<DeviceOption> }
pub fn build(ctx: Ctx, ind: &Indicators, answer: &str) -> Vec<DeviceView>
```
- **`build` ordering and options:**
  - It returns all 13 devices in `DeviceKind::ALL` order (the Global Constraints order), even when `count == 0`. In that case `options` is empty.
  - Up to 12 options per device. The option label is the pill text from the mockup: the split, e.g. `"OR + CHEST + RA"`; `"One word"`, `"Two words"`, `"Three words"` for anagrams; `"ORCHESTRA(L) − L"` for deletions.
- **Columns:** each option's columns follow the spec's device table. Part columns take P1…P5 in order.
- **Definition column:** every device has one, with `online: Some(Online::Definitions(answer))` and empty `groups`.
- **Part columns:**
  - Item groups: `"Abbreviation"` (from `Piece::Abbrev`) first, then `"Word"` (the word itself).
  - `online: Some(Online::Meanings(word))` when the part is a word.
- **Indicator columns:** filled from `Indicators` lists:
  - charade → `charade`
  - anagram → `anagram`
  - hidden → `hidden`
  - container → `container` + `insertion` (two groups)
  - deletion → `deletion_first` / `deletion_last` / `deletion_middle` / `deletion_general`
  - reversal → `reversal` + `reversal_down`
  - sounds like → `homophone`
  - change a letter → `shift`
  - alternate letters → `alternate`
  - first letters → `initial` + `final`
  - spoonerism → `spoonerism`
- **Count:** the number of finder results, so 412 anagrams and so on. Option lists are trimmed; counts are not.
- **Answers that can't be analysed:** if the answer contains `?`, or has fewer than 2 letters, every device has count 0.
- **First letters:** one column per answer letter, items from `initials::starting_with(letter, 40)`, cycling tones P1–P5.
- **Double definition and Cryptic definition:** count 1 when the answer is in the lexicon, else 0.
  - Double definition: one column, `Online::Senses(answer)`.
  - Cryptic definition: one column, `Online::Linked(answer)`.

- [ ] **Step 1: Write the failing tests** (in `bank/mod.rs`).
```rust
#[test] fn orchestra_devices() {
    let v = build(ctx(), &Indicators::bundled(), "orchestra");
    let names: Vec<_> = v.iter().map(|d| d.kind.name()).collect();
    assert_eq!(names, ["Alternate letters","Anagram","Change a letter","Charade","Container","Deletion",
        "First letters","Hidden","Reversal","Sounds like","Spoonerism","Cryptic definition","Double definition"]);
    let ch = v.iter().find(|d| d.kind == DeviceKind::Charade).unwrap();
    let opt = ch.options.iter().find(|o| o.label == "OR + CHEST + RA").expect("split");
    let titles: Vec<_> = opt.columns.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["OR","CHEST","RA","Definition","Joining words"]);
    assert!(opt.columns[0].groups.iter().any(|(_, items)| items.iter().any(|i| i == "gold")));
    assert_eq!(opt.columns[1].online, Some(Online::Meanings("chest".into())));
    assert!(v.iter().find(|d| d.kind == DeviceKind::Anagram).unwrap().count > 100);
    assert_eq!(v.iter().find(|d| d.kind == DeviceKind::Container).unwrap().count, 0);
}
#[test] fn blanks_and_tiny_answers_are_all_zero() {
    for a in ["orch?stra", "a", ""] {
        assert!(build(ctx(), &Indicators::bundled(), a).iter().all(|d| d.count == 0 && d.options.is_empty()));
    }
}
#[test] fn phrase_answers_use_letters_only() {
    let v = build(ctx(), &Indicators::bundled(), "it is");
    assert_eq!(v.len(), 13); // no panic; analysed as "itis"
}
#[test] fn long_answers_do_not_panic() {
    let v = build(ctx(), &Indicators::bundled(), "counterrevolution");
    assert_eq!(v.len(), 13);
}
#[test] fn first_letters_has_column_per_letter() {
    let v = build(ctx(), &Indicators::bundled(), "tempo");
    let fl = v.iter().find(|d| d.kind == DeviceKind::FirstLetters).unwrap();
    let cols = &fl.options[0].columns;
    assert_eq!(cols.iter().filter(|c| c.title.ends_with('…')).count(), 5);
}
```
- [ ] **Step 2:** Run `cargo test -p xw-engine bank`. Expected: FAIL.
- [ ] **Step 3:** Implement the types in `bank/mod.rs` and one `fn <device>(ctx, ind, key) -> DeviceView` per device in `bank/devices.rs`.
  - Hidden and alternate-letters items: put the display text in `groups` and the highlight ranges in `marks`, using the spaced key text as in the mockup.
  - Deletion tiles: the source word with `struck: true` on the removed chunk.
  - Container tiles: outer, inner, outer.
  - Reversal tiles: the answer, followed by a tile reading `← <REVERSED>` with `tone: None`.
- [ ] **Step 4:** Run the tests again. Expected: PASS. Also run `cargo test -p xw-engine`. Expected: all pass.
- [ ] **Step 5: Commit** — `feat(engine): word-bank view model for all 13 devices`

---

### Task 5: Answers and navigation (pure)

**Files:**
- Create: `app/src/answers.rs`

**Interfaces:**
- Consumes: `xw_engine::puzzle::Puzzle`, `xw_engine::grid::{Grid, Light, Dir}`
- Produces:
  - `pub struct Answer { pub light_id: String, pub label: String /* "10a" */, pub pattern: String /* lowercase, '?' for blank */, pub enumeration: String }`
  - `pub fn answers(p: &Puzzle) -> Vec<Answer>`: all acrosses, then all downs, in number order. The enumeration is the clue's if non-empty, else the length.
  - `pub fn step(list: &[Answer], current: &str, delta: i32) -> Option<String>`: wraps around, returns the light id.
  - `pub fn clued_count(p: &Puzzle) -> (usize, usize)`: (clues with non-blank text, total lights).

- [ ] **Step 1: Write the failing tests.**
  - On `Grid::from_rows(&["cat","a#o","bye"])`:
    - labels are `["1a","3a","1d","2d"]` and patterns `["cat","bye","cab","toe"]` (1 across CAT, 3 across BYE, 1 down CAB, 2 down TOE).
    - `step(&list, "0,2,d", 1)` returns `Some("0,0,a")`.
    - `step(&list, "0,0,a", -1)` returns `Some("0,2,d")`.
    - An empty puzzle gives `(0, n)`.
- [ ] **Step 2:** Run `cargo test -p setter answers`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run it again. Expected: PASS.
- [ ] **Step 5: Commit** — `feat(app): answer list and navigation`

---

### Task 6: Datamuse client with cache and offline handling

**Files:**
- Create: `app/src/online.rs`

**Interfaces:**
- Consumes: `xw_engine::bank::Online`
- Produces:
  - `pub enum Fetched { Groups(Vec<(String, Vec<String>)>), Offline }`
  - `pub async fn fetch(q: &Online) -> Fetched`, which caches by `format!("{q:?}")` in a `static OnceLock<Mutex<HashMap<…>>>`. It never caches `Offline`.
  - Pure parsers (tested), each taking a Datamuse JSON array as a `serde_json::Value`:
    - `pub fn parse_meanings(syn: &Value, ml: &Value) -> Vec<(String, Vec<String>)>`: groups `"Synonyms"` (rel_syn) then `"Related"` (ml minus synonyms), deduplicated, at most 60 in total.
    - `pub fn parse_definitions(ml: &Value) -> Vec<(String, Vec<String>)>`: one group, `"Means <WORD>"`.
    - `pub fn parse_senses(defs: &Value, ml_with_defs: &Value) -> Vec<(String, Vec<String>)>`: one group per sense of the answer.
      - The answer's senses come from the first element's `defs` (`"n\tgloss"`). The title is `"<pos>: <first 4 words of gloss>"`.
      - Each `ml` result (requested with `md=d`) goes to the first sense whose gloss shares a word of 4+ letters with any of that result's own defs; otherwise to `"Other"`.
      - At most 5 senses plus Other.
    - `pub fn parse_linked(trg: &Value) -> Vec<(String, Vec<String>)>`: one group, `"Linked ideas"`, at most 60.
- **URLs:** base `https://api.datamuse.com/words?`, then:
  - `rel_syn={w}&max=60`
  - `ml={w}&max=80`
  - `sp={w}&md=d&max=1`
  - `ml={w}&md=d&max=80`
  - `rel_trg={w}&max=60`
- **Client:** reqwest with a 10 s timeout and User-Agent `setter-crossword-app/0.1`. Any network error maps to `Offline`.

- [ ] **Step 1: Write the failing tests**, using inline JSON fixtures:
```rust
#[test] fn meanings_merge_and_dedupe() {
    let syn = json!([{"word":"box"},{"word":"trunk"}]);
    let ml = json!([{"word":"trunk"},{"word":"bust"},{"word":"coffer"}]);
    assert_eq!(parse_meanings(&syn, &ml), vec![
        ("Synonyms".into(), vec!["box".into(),"trunk".into()]),
        ("Related".into(), vec!["bust".into(),"coffer".into()])]);
}
#[test] fn senses_group_by_shared_gloss_words() {
    let defs = json!([{"word":"pitch","defs":["v\tthrow or cast forward","n\ta dark sticky substance like tar"]}]);
    let ml = json!([{"word":"toss","defs":["v\tthrow carelessly"]},{"word":"bitumen","defs":["n\tsticky substance used for roads"]},{"word":"spiel","defs":["n\ta plausible glib talk"]}]);
    let g = parse_senses(&defs, &ml);
    assert_eq!(g[0].1, vec!["toss".to_string()]);
    assert_eq!(g[1].1, vec!["bitumen".to_string()]);
    assert_eq!(g.last().unwrap().0, "Other");
}
#[tokio::test] async fn unreachable_host_is_offline() {
    assert!(matches!(get_json("http://127.0.0.1:9/words?ml=x").await, Err(_)));
}
```
  Expose `async fn get_json(url: &str) -> Result<Value, String>` as `pub(crate)` for this test. Add `tokio` with the `macros` and `rt` features to dev-dependencies.
- [ ] **Step 2:** Run `cargo test -p setter online`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run it again. Expected: PASS.
- [ ] **Step 5: Commit** — `feat(app): Datamuse lookups with cache and offline handling`

---

### Task 7: Puzzle files, sample puzzle, settings

**Files:**
- Create: `app/src/files.rs`

**Interfaces:**
- Consumes: `xw_engine::{puzzle::Puzzle, fill::{FillIndex, FillOptions, autofill}, grid::Grid}`
- Produces:
  - `pub fn config_dir() -> PathBuf`: `dirs::config_dir()/setter`, created on first use.
  - `pub struct Settings { pub theme: Theme /* System|Light|Dark */, pub last_file: Option<PathBuf> }`: Serialize/Deserialize to `settings.json`, defaults on any error. Also `pub fn load_settings() -> Settings` and `pub fn save_settings(&Settings)`.
  - `pub fn load(path: &Path) -> Result<Puzzle, String>` and `pub fn save(path: &Path, p: &Puzzle) -> Result<(), String>`. Pretty JSON, extension `.xword`.
  - `pub fn sample_puzzle(lex: &Lexicon) -> Puzzle`:
    - Uses the British 15×15 template rows below and autofills with `FillOptions::default()`, randomness 0.4, a 20 s limit.
    - Sets the enumerations of phrase entries from `AutofillResult.entries` via `text::enumeration`.
    - Template rows: `".....#.........", ".#.#.#.#.#.#.#.", ".........#.....", ".#.#.#####.#.#.", ".......#.......", "##.#.#.#.#.###.", "...#.......#...", ".#.###.#.###.#.", "...#.......#...", ".###.#.#.#.#.##", ".......#.......", ".#.#.#####.#.#.", ".....#.........", ".#.#.#.#.#.#.#.", ".........#....."`
  - `pub fn user_css() -> String`: the contents of `config_dir()/theme.css`, or an empty string.
  - `pub fn ensure_user_files()`: writes each `USER_FILES` template into `config_dir()` if it's missing.
  - `pub fn load_engine_data() -> EngineData`, where `pub struct EngineData { pub lex: Lexicon, pub abbr: Abbreviations, pub ind: Indicators, pub warnings: Vec<String> }`. It builds `Lexicon::bundled(words.txt lines, banned.txt keys)` and the bundled lists plus `merge_toml` of the user files. TOML errors become warnings, shown once as a dismissible line under the top bar.
  - `config_dir()` reads the `SETTER_CONFIG_DIR` env var first, so tests can point it at a temp dir.

- [ ] **Step 1: Write the failing tests.**
  - `load_engine_data()` with a temp config dir whose `banned.txt` is `orchestra` gives `lex.lookup("orchestra") == None`, and an invalid `abbreviations.toml` gives exactly one warning.
  - A save/load round trip through `std::env::temp_dir()` gives an equal `Puzzle`.
  - Loading a file containing `"not json"` returns `Err` containing `"Not a puzzle file"`.
  - `sample_puzzle(&lex)` has no `None` letters in non-block cells, and every light's pattern is in the lexicon. Mark this test `#[ignore]` if it takes longer than 10 s in debug, and run it in release.
- [ ] **Step 2:** Run `cargo test -p setter files`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `cargo test -p setter files -- --include-ignored --release`. Expected: PASS.
- [ ] **Step 5: Commit** — `feat(app): puzzle files, sample puzzle, settings`

---

### Task 8: Clue-text colouring and checks (pure)

**Files:**
- Create: `app/src/highlight.rs`

**Interfaces:**
- Consumes: `xw_engine::bank::Tone`
- Produces:
  - `pub fn spans(text: &str, picks: &[(String, Tone)]) -> Vec<(String, Option<Tone>)>`: splits `text` into consecutive spans. A span gets the tone of the first pick it matches. Matching is case-insensitive and only on word boundaries. Multi-word picks ("next to") are matched as a phrase. Concatenating the span texts reproduces `text` exactly.
  - `pub struct Checks { pub adds_up: bool, pub definition_at_end: bool }`
  - `pub fn checks(text: &str, parts: &[(String, Tone)], definition: Option<&str>) -> Checks`:
    - `adds_up` is true when every part pick appears in the text.
    - `definition_at_end` is true when the definition appears as the first or last words, ignoring punctuation and a trailing enumeration like `(9)`.

- [ ] **Step 1: Write the failing tests.**
```rust
#[test] fn colours_whole_words_case_insensitively() {
    let s = spans("Gold box by Artist's band (9)", &[("gold".into(),Tone::P1),("box".into(),Tone::P2),("artist".into(),Tone::P3),("band".into(),Tone::Definition),("by".into(),Tone::Indicator)]);
    assert_eq!(s.iter().map(|x| x.0.as_str()).collect::<String>(), "Gold box by Artist's band (9)");
    assert!(s.contains(&("Gold".into(), Some(Tone::P1))));
    assert!(s.contains(&("Artist".into(), Some(Tone::P3))));
    assert!(!s.iter().any(|x| x.0 == "o" && x.1.is_some())); // "gold" doesn't colour inside other words
}
#[test] fn phrases_and_checks() {
    let s = spans("Gold next to box", &[("next to".into(), Tone::Indicator)]);
    assert!(s.contains(&("next to".into(), Some(Tone::Indicator))));
    let c = checks("Gold box by artist for band (9)", &[("gold".into(),Tone::P1),("box".into(),Tone::P2),("artist".into(),Tone::P3)], Some("band"));
    assert!(c.adds_up && c.definition_at_end);
    assert!(!checks("Gold box", &[("artist".into(),Tone::P3)], Some("band")).adds_up);
}
```
- [ ] **Step 2:** Run `cargo test -p setter highlight`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run it again. Expected: PASS.
- [ ] **Step 5: Commit** — `feat(app): clue colouring and checks`

---

### Task 9: The Write clues screen

**Files:**
- Create: `app/src/state.rs`, `app/src/ui/mod.rs`, `app/src/ui/topbar.rs`, `app/src/ui/devices.rs`, `app/src/ui/head.rs`, `app/src/ui/cluebox.rs`, `app/src/ui/bank.rs`
- Modify: `app/src/main.rs`, `app/assets/main.css`

**Interfaces:**
- Consumes:
  - `bank::build` (Task 4); `answers::{answers, step, clued_count}` (Task 5); `online::fetch` (Task 6); `files::*` (Task 7); `highlight::{spans, checks}` (Task 8).
- Produces:
  - `AppState`, provided by `use_context_provider` in `App` and holding:
    - `puzzle: Signal<Puzzle>`, `path: Signal<Option<PathBuf>>`, `current: Signal<String>` (light id)
    - `device: Signal<DeviceKind>`, `option: Signal<usize>`
    - `picks: Signal<HashMap<(String, DeviceKind, usize, usize), String>>`: (light id, device, option, column) → picked item
    - `settings: Signal<Settings>`
  - Engine data lives in a `static DATA: LazyLock<EngineData>` from `files::load_engine_data()` (Task 7). Call `files::ensure_user_files()` first.

- [ ] **Step 1: State and loading.**
  - On start: if `settings.last_file` loads, use it; otherwise use `sample_puzzle`. Generate the sample in `spawn_blocking`, showing a centred "Filling a sample grid…" in the meantime.
  - The current answer is the first in `answers()`.
  - `bank::build` runs in a `use_memo` keyed on the answer pattern. The `device` and `option` signals reset when the answer changes. The default device is the first with `count > 0`.
- [ ] **Step 2: Top bar** (`topbar.rs`):
  - Logo, then the mode switch. "Build grid" shows a centred one-liner, `Grid building comes next. Use New sample to get a filled grid.`, with no other content.
  - Answer navigator: `‹`, the answer letters (blanks shown as `·`), `<label> · <enum>`, `›`. Alt+← / Alt+→ also navigate, via `onkeydown` on the root.
  - `"<clued> of <total> clued"`.
  - Buttons: Open, Save (Ctrl+S), New sample, theme toggle cycling System → Light → Dark.
  - Theme sets `document::eval("document.documentElement.dataset.theme = '…'")`, or removes the attribute for System.
  - Open/Save use `rfd::AsyncFileDialog` with a `.xword` filter.
- [ ] **Step 3: Device list** (`devices.rs`):
  - Group headings "Wordplay" / "Meaning only".
  - Rows show the name and count. `.zero` styling when the count is 0; `.on` for the selected row.
  - Classes as in the mockup's `.side`, `.dev`.
- [ ] **Step 4: Head row** (`head.rs`):
  - Tiles from `DeviceOption.tiles` (classes `grp g-p1`…, `g-x` when struck).
  - Option pills, from `DeviceView.options[*].label`.
  - When the answer has blanks: the tiles only, plus `This answer has empty squares.`
- [ ] **Step 5: Clue box** (`cluebox.rs`):
  - A `textarea` overlaid on a mirror `div`. Both share the same font, padding and wrapping. The textarea has `color: transparent; caret-color: var(--text); background: transparent`.
  - The mirror renders `highlight::spans(text, picks_for_current)`, using classes `w p1`…`p5`, `wd` and `wi`.
  - Edits write `puzzle.clues[light_id].text`.
  - Beside the box: the enumeration (`(<enum>)`).
  - Below it: `Reads as: <reads_as>`, then `✓ adds up` and `✓ definition at an end`. Each check shows only when true. When false it shows nothing.
- [ ] **Step 6: Word bank** (`bank.rs`):
  - One `.col` per `Column`, with the tone class, the `wide` flag and a header showing the title and count.
  - A filter input, then the grouped items. Hidden and alternate-letter items render `marks` as `<u>` highlights.
  - Clicking an item sets the pick (`.it.on`).
  - Online columns: on mount, `use_resource(fetch(q))`.
    - While loading: `Loading…` in the column.
    - On `Offline`: `Needs an internet connection.`
    - On `Groups`: these are appended after any offline groups.
  - The `.bank` row scrolls horizontally when columns × 150 px exceed its width.
- [ ] **Step 7: Run the whole workspace's tests:** `nix develop --command cargo test --workspace`. Expected: all pass.
- [ ] **Step 8: Visual check.**
  - Run `dx serve --platform desktop`.
  - Navigate to ORCHESTRA. If the sample grid lacks it, type `orchestra` into a 9-letter light via a temporary `--answer` CLI flag; that flag is dev-only and is removed before the commit.
  - Screenshot every device in light and dark mode with `scripts/winshot.ps1`.
  - Compare against `docs/design/devices-gallery-mockup.html`. Expected: the same structure, colours and copy, no overflow at 1440×920, and none at a 15-letter answer.
- [ ] **Step 9: Commit** — `feat(app): Write clues screen`

---

### Task 10: README and Windows build notes

**Files:**
- Create: `README.md`

- [ ] **Step 1:** Write the README. It covers:
  - What the app is.
  - `nix develop` then `cd app && dx serve --platform desktop`.
  - The config folder files: `words.txt`, `banned.txt`, `abbreviations.toml`, `indicators.toml`, `theme.css`. They're created on first run with commented templates.
  - The Windows build: install Rust (MSVC) and `cargo install dioxus-cli --version 0.7.*`, then `dx bundle --platform desktop --release` on Windows. This produces an MSI/NSIS installer, since `dx bundle` only targets the host OS.
  - Data credits, pointing to `data/NOTICE.md`.
- [ ] **Step 2:** Run `cargo test --workspace` and `dx build --platform desktop --release`. Expected: success.
- [ ] **Step 3: Commit** — `docs: README with dev and Windows build instructions`

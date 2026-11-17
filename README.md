# Setter

A desktop app for setting **cryptic crosswords**. You fill a grid, then write the clues
with help that goes well beyond online tools: for every answer it lays out each wordplay
device, the pieces the answer splits into, and long, colour-coded lists of words to clue
each piece. The clue you type is coloured to match.

It works entirely offline. The word list, pronunciations, thesaurus, abbreviations and
indicators are all built in.

## What's in it

- **Build grid**:
  - **New puzzle** offers six standard British grids (15×15 and 13×13).
  - Click a square and type. Space switches direction; Tab moves to the next answer.
  - Right-click or Ctrl+Space shows the words that fit, filtered by how many words
    the answer splits into. The wand fills just that answer; the eraser clears it.
  - Type a split such as `(4,5)` or `(3-4)` beside the answer. Bars and hyphens
    show in the grid, and fills respect them.
  - **Fill the rest** autofills every empty square, keeping your letters. It can be
    stopped.
  - The line under the grid flags non-words, repeats, unfillable answers and British
    checking rules. A word that isn't in the list can be added to your words.
  - Undo and Redo, also Ctrl+Z and Ctrl+Shift+Z.
  - The clue list on the right shows which answers still need a clue.
  - Click the title or setter in the top bar to edit them. The setter's name is
    remembered for new puzzles.
- **Write clues**:
  - A device list with live counts for each answer. Thirteen devices: alternate letters,
    anagram, change a letter, charade, container, deletion, first letters, hidden,
    reversal, sounds like, spoonerism, cryptic definition and double definition.
  - Answer tiles coloured by part, with the alternative splits as pills.
  - A clue box that colours the words standing for each part, and checks that the
    wordplay adds up and the definition sits at an end.
  - Wide word-bank columns: abbreviations, synonyms, broader terms and loose
    associations, with the most common words first.
- **Engine** (`crates/engine`):
  - A 287k-entry popularity-ranked lexicon with pronunciations.
  - The wordplay finders and the word-bank model.
  - An offline thesaurus.
  - The grid model, fill suggestions that keep crossings fillable, autofill, the
    `.xword` file format and ipuz export.
- **Design system** (`app/src/ds`, `app/assets/tokens.css`):
  - Typed components on top of the official Dioxus components, styled only through
    tokens.
  - Light and dark modes.
  - See every component with `setter --gallery`.
- **Saving and sharing**:
  - **Save** writes a `.xword` file (Ctrl+S).
  - Work is autosaved a second after each change and recovered if Setter closes
    before you save.
  - **Export** writes:
    - a **web page** (`.html`): one self-contained file anyone can open in a
      browser to solve. It has check and reveal by letter, word or grid, a timer,
      an anagram helper, confetti and the solve time when finished, saved
      progress, print and a phone layout. Once solved, each clue shows how it
      works, written from the words you picked in the word bank.
    - **Across Lite** (`.puz`), once every square is filled.
    - **ipuz** (`.ipuz`).

## Developing

[mise](https://mise.jdx.dev) sets everything up, the same way on every OS. `mise.toml`
pins Rust and the Dioxus CLI (`dx`), lists the system libraries each OS needs, and
names the everyday commands.

```bash
mise install                      # Rust 1.99.1 and dx 0.7.10
mise bootstrap packages apply     # Linux only: WebKitGTK etc. via apt, dnf, pacman or zypper
mise run app                      # run Setter with live reload
mise run test                     # every test
mise run lint                     # rustfmt and clippy
mise run gallery                  # every design-system component
mise run bundle                   # an installer for this OS
```

What each OS needs besides mise:

| OS | System libraries |
|---|---|
| macOS | Xcode's command-line tools (`xcode-select --install`); the webview is built in |
| Windows | Microsoft C++ Build Tools ("Desktop development with C++"); WebView2 is built into Windows 10 and 11 |
| Debian, Ubuntu, Fedora, Arch, openSUSE | `mise bootstrap packages apply` (it uses sudo) |
| NixOS | nothing: `flake.nix` provides them, and the tasks use it automatically through `scripts/in-env.sh`. Rust from mise needs nix-ld enabled. |

Always build the app through `dx` (the tasks do), not plain `cargo run`. The bundled
fonts and stylesheets only load under `dx`. To run a built binary yourself on NixOS,
go through the same wrapper: `scripts/in-env.sh target/dx/setter/debug/linux/app/setter`.
Outside it, WebKitGTK can't find its schemas and renders a broken page.

`dx bundle` only builds installers for the OS it runs on, so build each one on its own
OS: `.msi`/`.exe` on Windows, `.app`/`.dmg` on macOS, `.deb`/`.AppImage` on Linux. They
land under `target/dx/setter/bundle/`. An unsigned Mac app needs right-click → Open the
first time.

Regenerating the word data (`scripts/build_lexicon.py`, `scripts/build_thesaurus.py`)
needs any Python 3; they use the standard library only.

Dev-only hooks, for screenshots and driving the app in WSLg, where synthetic input
doesn't reach windows:

- `SETTER_CONFIG_DIR=<dir>`: use a scratch config folder.
- `SETTER_GALLERY_SCROLL=<px>`: scroll the gallery.
- `SETTER_DEV_SCRIPT=<file.js>`: run a script in the window (debug builds only). See
  `scripts/dev/`.
- `scripts/winshot.ps1`: capture a window from WSL via Windows PowerShell, e.g.
  `-Title Setter -Out "$(wslpath -w .scratch)\shot.png"`.

Scratch files stay in the repo. Screenshots and dev scripts go in `.scratch/`, which git
ignores. Tests write under `target/scratch`.

## Making it yours

On first run, Setter creates a config folder:

- Windows: `%APPDATA%\setter`
- Linux: `~/.config/setter`

It contains:

| File | What it does |
|---|---|
| `words.txt` | Extra words and phrases, treated as most common. |
| `banned.txt` | Words never to suggest. |
| `abbreviations.toml` | Your abbreviations, merged with the built-in ~280. An empty list removes a built-in one. |
| `indicators.toml` | Your indicator words, added to the built-in lists. |
| `theme.css` | Your CSS, loaded last. Every colour, font, size and radius is a variable in `app/assets/tokens.css`, so you can override any of them. |

## Data and credits

Built-in data and its licences are listed in [`data/NOTICE.md`](data/NOTICE.md), which is also shown in the app under **Credits**:

- The Lufz lexicon (from Exet; UKACD18 and CMUdict).
- Open English WordNet 2025 (CC BY 4.0).
- Moby Thesaurus II (public domain).
- Word quality for fills, from the hand-scored Nediger list (MIT), adjusted for British spellings.
- Inter and Newsreader fonts (OFL). To try another typeface, change `crates/engine/src/fonts.rs`.

Regenerate the data with `scripts/build_lexicon.py`, `scripts/build_thesaurus.py` and `scripts/build_quality.py` (which needs the Nediger list's `nediger list.txt`).

# Setter

A desktop app for setting **cryptic crosswords**. You fill a grid, then write the clues
with help that goes well beyond online tools: for every answer it lays out each wordplay
device, the pieces the answer splits into, and long, colour-coded lists of words to clue
each piece. The clue you type is coloured to match.

It works entirely offline. The word list, pronunciations, thesaurus, abbreviations and
indicators are all built in.

## What's in it

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
- **Build grid**: designed next. For now **New sample** gives you an autofilled
  15×15 British grid to clue.

## Developing (NixOS / WSL)

The Nix flake provides Rust, the `dx` CLI and the WebKitGTK libraries.

```bash
nix develop
cargo test --workspace                     # engine + app tests
cd app && dx serve --platform desktop      # run with live reload
dx serve --platform desktop -- --gallery   # the component gallery
```

Always build and run the app through `dx`, not plain `cargo run`. The bundled fonts
and stylesheets only load under `dx`.

Dev-only hooks, for screenshots and driving the app in WSLg, where synthetic input
doesn't reach windows:

- `SETTER_CONFIG_DIR=<dir>`: use a scratch config folder.
- `SETTER_GALLERY_SCROLL=<px>`: scroll the gallery.
- `SETTER_DEV_SCRIPT=<file.js>`: run a script in the window (debug builds only). See
  `scripts/dev/`.
- `scripts/winshot.ps1`: capture a window from WSL via Windows PowerShell.

## Building for Windows

`dx bundle` only builds installers for the OS it runs on, so build on Windows itself:

1. Install Rust with the MSVC toolchain (<https://rustup.rs>) and WebView2. WebView2 is
   preinstalled on Windows 10 and 11.
2. Install the Dioxus CLI: `cargo install dioxus-cli --version 0.7.10 --locked`.
3. In the repo: `cd app` then `dx bundle --platform desktop --release`. This writes an
   MSI/NSIS installer under `target/dx/setter/bundle/`.

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
- Newsreader and Instrument Sans fonts (OFL).

Regenerate the data with `scripts/build_lexicon.py` and `scripts/build_thesaurus.py`.

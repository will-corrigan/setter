# Write clues screen — design

Status: direction approved by the user 2026-11-16 ("build this and refine it after").
Mockups: `docs/design/write-clues-mockup.html` (clickable) and
`docs/design/devices-gallery-mockup.html` (every device stacked).

## Intent

Setter is a desktop app for setting cryptic crosswords, for the user's own use.
Their workflow is two stages: build and fill the grid, then write the clues.
Writing clues is the heart of it ("a good clue is the best feeling"), and
fodder.ing helped "SO much" by showing ways to fit an answer into different
wordplay. This screen is where clues get written. It must give at least what
fodder gives, plus the common cryptic abbreviations and short words that fit
inside the answer, and a wide breadth of words to choose from.

## Goals

- Pick an answer, pick a device, see a broad bank of words for every part of the
  wordplay, and write the clue, all on one screen.
- Make it obvious which words in the clue stand for which letters of the answer.
- Be fast to use repeatedly: no tutorial-style steps.

## Non-goals

- Writing clues for the user. The app supplies material; the user writes the sentence.
- Mobile layouts. This is a desktop app (Windows target).
- Teaching cryptic crosswords.

## Anti-goals (what would make it a failure even if it works)

- Hand-holding: numbered steps, explanations and intro text. The user rejected these
  as "would get annoying after a few times".
- Setter jargon the user doesn't use. "Surface" and "recipe" confused them;
  "device" and "def" are their own words and are fine.
- Narrow choices. Five synonyms per part is too few; breadth matters.
- Unexplained panels. Every area's purpose must be obvious from what it shows.
- Monotone visuals. Colour must carry meaning.

## Layout (approved)

```
┌ Setter │ [Build grid | Write clues] │ ‹ ORCHESTRA 10a · 9 › │ 11 of 28 clued │ theme ┐
├──────────────┬────────────────────────────────────────────────────────────────┤
│ Wordplay     │ [O R][C H E S T][R A]   (OR + CHEST + RA) (ORC + HE + STRA) …   │
│  Alternate…  │ ┌──────────────────────────────────────────────┐ (9)            │
│  Anagram 412 │ │ Gold box by artist for band                  │                 │
│  Change a…   │ └──────────────────────────────────────────────┘                 │
│  Charade 9   │ Reads as: gold + box + artist   ✓ adds up                       │
│  …           │ ┌ OR ──┐┌ CHEST ──┐┌ RA ──┐┌ Definition ┐┌ Joining words ┐       │
│ Meaning only │ │gold  ││box      ││artist││band        ││with           │       │
│  Cryptic def │ │men   ││trunk    ││…     ││players     ││by             │       │
│  Double def  │ │…     ││… 21     ││      ││…           ││…              │       │
└──────────────┴────────────────────────────────────────────────────────────────┘
```

- **Top bar:** mode switch (Build grid / Write clues), answer navigator (‹ › and
  keyboard), progress, light/dark toggle.
- **Device list (left):** two groups, *Wordplay* and *Meaning only*, each sorted
  **alphabetically**, each device with a count of options for the current answer.
  Devices with no options are greyed out but still selectable.
- **Head row:** answer tiles coloured by part, and the alternative splits or options
  for the chosen device as pills.
- **Clue box:** a large serif editable field with the enumeration beside it. Below
  it, a quiet "Reads as" line and checks (adds up, definition at an end).
- **Word bank:** one column per part of the wordplay, then a Definition column,
  then an indicator column for the chosen device. Each column has a filter box, a
  count, grouped items (Abbreviation / Synonyms / Phrases…) and scrolls on its own.
  Clicking an item selects it in that part's colour.

## Colour language

- Each wordplay part has its own colour: blue, violet, amber, teal, pink, in that
  order. The colour is shared by the answer tiles, that part's word-bank column,
  and the words in the clue that clue it.
- Green is always the definition. Dotted grey is an indicator or linking word.
- Light and dark themes, following the system by default. All colours are CSS
  variables so the user can retheme them.

## Devices

| Group | Device | Parts / columns |
|---|---|---|
| Wordplay | Alternate letters | source phrases (letters highlighted) · Definition · every-other words |
| Wordplay | Anagram | letters to mix (one / two / three words, wide) · Definition · mixing words |
| Wordplay | Change a letter | source word · letter out · letter in · Definition · swap words |
| Wordplay | Charade | one column per part · Definition · joining words |
| Wordplay | Container | outer · inner · Definition · putting-in words (outer holds / inner goes in) |
| Wordplay | Deletion | longer word · removed part · Definition · removal words (by position) |
| Wordplay | First letters | one column per letter (words starting with it) · Definition · initial words |
| Wordplay | Hidden | phrases that hide it (highlighted, wide) · Definition · hiding words |
| Wordplay | Reversal | reversed word · Definition · backwards words (across / down) |
| Wordplay | Sounds like | sound-alike · Definition · sound words |
| Wordplay | Spoonerism | first word · second word · Definition · Spooner words |
| Meaning only | Cryptic definition | linked ideas · words with a second sense · Definition |
| Meaning only | Double definition | one column per meaning |

The pills in the head row hold the device's alternatives: the different splits
for a charade, single/two/three-word for an anagram, each longer word for a
deletion, and so on.

## Data needed per column

| Source | Status |
|---|---|
| Wordplay finders: anagram, hidden, container, charade, deletion, reversal, homophone, substitution, alternate letters, letter shift | Built in `crates/engine` |
| Abbreviations and indicator lists, user-extendable | Built (`data/*.toml` plus user config) |
| Synonyms and meanings per part and for the definition | Datamuse online (cached); needs a breadth target of ~20–60 per column |
| First letters: common words starting with each letter | New. Offline, from the lexicon's popular words |
| Spoonerisms | New engine finder (swap initial consonant clusters of two-word splits, check both halves; ideally phonetic via CMUdict) |
| Double definition: separate meanings of the answer | New. Datamuse `md=d` definitions, grouped by sense |
| Cryptic definition: associations and words with a second sense | New. Datamuse `rel_trg` / `ml`, plus words with several senses |
| Abbreviations and short words inside the answer (user request) | Engine `Ctx::part` exists; surface it in the Charade/Container splits and as an "all pieces" view (placement to be decided in refinement) |

Wherever the app can't confirm that a column applies, offline still works.
Online-only columns say so in one line when offline.

## Approaches considered

- **Guided steps** (split → meanings → definition → sentence): clear on first use,
  but rejected as too hand-holdy.
- **Recipe slots, drill-down columns, grid overlay, highlighter, clue sheet**: shown
  as variations V2–V6; all rejected, except that V2's device list was kept.
- **Word bank (chosen):** device list plus columns of words per part. It gives
  breadth and stays fast after repeated use.

## Decided by the user

- Two stages, Build grid then Write clues, as separate modes.
- Word bank layout, colour coding, dark mode.
- Alphabetical device order.
- Build first, refine after.

## Open / left to the builder

- **GUI framework: Dioxus 0.7 desktop (decided by the user, 2026-11-16)**, after the
  benchmark and report in `reports/Rust desktop GUI options.md`. It is all Rust and
  renders real HTML/CSS, so the mockups' CSS carries over; the UI calls `xw-engine`
  directly. Build with the `dx` CLI (bundled assets need it). Pin 0.7.x. If it
  disappoints, the CSS moves to Tauri + Svelte.
- **Build grid screen.** Not yet designed with the user; to be designed next.
- Keyboard shortcuts, column widths, and how many items to load per column (lazy
  loading).
- Whether the clue box offers a starting sentence. Asked, not yet answered;
  default is an empty box.

## Later (user request, 2026-11-17, "only after it works")

- Export / share so people can play together: candidates are a self-contained
  playable HTML page, `.ipuz` (engine already exports), PDF, and a temporary
  share link or live LAN session for solving together. Design when the core works.

## Data decision (user, 2026-11-17)

- Fully offline: no Datamuse or any online service. Synonyms, senses and linked
  ideas come from a bundled thesaurus (Open English WordNet + Moby Thesaurus).

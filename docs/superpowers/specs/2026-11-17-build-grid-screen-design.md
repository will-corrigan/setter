# Build grid screen: design

Status: draft for approval (2026-11-17). Interactive mockup:
`docs/design/build-grid-mockup.html`.

## Intent

The first stage of setting: pick a standard grid and fill it with words,
quickly and pleasantly, before writing clues on the Write clues screen. The
user said they want to start from standard patterns, click a square and type,
and get words that fit the letters already in place on demand.

## Goals

- Pick a standard British pattern (15×15 or 13×13) and start filling at once.
- Fill by clicking and typing, with the selected answer clearly highlighted.
- See, at all times, the words that fit the selected answer. They respect
  crossing letters, can be filtered by word split, and are ranked by
  everyday-ness and by how much room they leave the crossing answers.
- Mark how each answer splits into words, and show the breaks in the grid.
- Know at a glance whether the grid is still sound.

## Non-goals

- Drawing your own block patterns. The user said "maybe someday … not now".
- Barred grids, theme or nina tools, answer locks, rejecting words for
  autofill. The user was indifferent; leave for later.

## Anti-goals

- Unexplained numbers, jargon or notes under suggestions in the UI (the user
  rejected "26 left", then the plain-English notes too).
- Inverted crossword colours in dark mode.

## Layout (approved through the mockup)

```
┌ Setter │ [Build grid | Write clues] │ New puzzle │      n of m filled │ Fill the rest │ Undo │ …┐
├───────────────────────────────────────────────┬───────────────────────────────────┤
│ 3 down  ·A·· ·E·T·  (4,5)  Ctrl+Space…         │ Fits 3 down · ·A·· ·E·T·          │
│ ┌───────── grid ─────────┐                     │ [ search ]  [Any|1|2|3+ words]    │
│ │ white squares, black   │                     │ cart horse (4,5)                  │
│ │ blocks, thick bars at  │                     │ dark horse (4,5)                  │
│ │ word breaks            │                     │ …                                 │
│ └────────────────────────┘                     │ …                                 │
│ status: 12 of 36 filled · every answer is a word · everything still fillable · …   │
└───────────────────────────────────────────────┴───────────────────────────────────┘
```

- **Top bar:** the shared TopBar.
  - "New puzzle" opens the pattern picker: thumbnails of the standard grids.
  - "Fill the rest" autofills every empty square and keeps the user's letters.
  - Undo and Redo, also Ctrl+Z and Ctrl+Shift+Z.
  - A filled count.
- **Answer bar** (above the grid): the label (e.g. "3 down"), the pattern with
  word breaks, and an editable split box "(4,5)" or "(3-4)". A split that
  doesn't add up to the answer's length is ignored.
- **Grid:**
  - Click a square to select it and light up its answer. Click it again, or
    press Space, to switch between across and down.
  - Type A–Z to fill and advance. Backspace deletes and steps back. Arrow keys
    move, setting the direction. Tab and Shift+Tab move to the next and
    previous answer.
  - Word breaks are a thick bar between squares; a hyphen is a short dash
    across the border.
  - **Fixed crossword colours in both themes:** white squares, black blocks,
    dark letters, light-blue answer, blue current square. Only the app around
    the grid follows the theme.
- **Right-click a square:** a menu with exactly "Clear this answer" and "Fill
  just this answer".
- **Side panel** (always visible): "Fits <answer> · <pattern>".
  - A search box. Ctrl+Space focuses it.
  - Chips: Any · 1 word · 2 words · 3+ words.
  - The list of fitting entries, including phrases, each with its split.
  - Click to place. Placing a phrase sets the answer's split.
  - **No notes under words** (user, 2026-11-17). Words that would leave a
    crossing answer with nothing that fits are not listed at all, and tighter
    words rank lower. Everything shown is safe to place.
- **Status line** (under the grid):
  - "n of m answers filled"
  - "every answer is a word", or the ones that aren't
  - "everything still fillable", or "nothing in the word list fits 3 down"
  - "no repeats", or the repeats
  - British checking rules met, or what's wrong

## Engine

- Use the existing `xw_engine::fill`:
  - `FillIndex` for pattern matching.
  - `suggestions` for crossing viability. Extend it to report *which* crossing
    is tightest and its count.
  - `autofill` for "Fill the rest" and "Fill just this answer".
- Add split filtering to suggestions: an enumeration filter and a word-count
  filter. Use the full 287k lexicon, with phrases.
- Splits are stored in the puzzle as clue enumerations (`Clue.enumeration`), so
  the Write clues screen already sees them.
- New in the engine: British grid checks, as pure tested functions.
  - Every light has at least half of its letters checked.
  - No two consecutive unchecked squares.
  - The grid is connected.
- Templates: the standard British 15×15 and 13×13 patterns, symmetric and
  checked by those rules.

## Decided by the user

- Standard patterns only.
- Click and type with answer highlight.
- An always-on side panel of fitting words.
- A right-click menu with clear and fill-this-answer only.
- Word splits, with clearly marked separators.
- No notes under suggestions; blocking words are hidden instead.
- Normal crossword colours in dark mode.

## Calls made for the user (correct me)

- Undo and Redo are included. Locks, theme words and rejecting words come later.
- The template list starts with 4× 15×15 and 2× 13×13. More can be added later
  as data.
- "Fill the rest" uses the same word-list depth as the side panel: everyday
  through wide.

## Left to the builder

Keyboard focus handling between the grid and the panel, the exact picker
layout, and performance tuning (suggestions must update within about 100 ms).

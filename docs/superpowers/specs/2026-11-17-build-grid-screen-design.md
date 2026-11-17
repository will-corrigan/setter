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
- **Horizontal scrollbars anywhere in the app** (user, 2026-11-17). Lists only
  scroll vertically; rows wrap instead of overflowing. This also applies to the
  Write clues word bank, which must wrap its columns rather than scroll sideways.
- Instruction text on screen ("Click to place", "Ctrl+Space jumps here" and the
  like). Shortcuts go in button tooltips only. (User, 2026-11-17.)

## Layout (approved through the mockup, final 2026-11-17)

```
┌ Setter │ [Build grid | Write clues] │ New puzzle │      n of m filled │ Fill the rest │ Undo │ …┐
├──────────────────────────────┬──────────────────────────────────────────────────────┤
│ 3 down  ·A·· ·E·T·  (4,5)    │ Across                    │ Down                      │
│ ┌──────── grid ────────┐     │  1 Reset badly for some   │  1 Bill with the French   │
│ │ white squares, black │ ┌─────────────┐ woodland (5)   │    put forward (5)        │
│ │ blocks, word breaks  │ │ 3 down ✦ ⌫  │                │  2 Gloomy yet strangely…  │
│ │                      │ │ [search]    │ …              │ 12 no clue                │
│ └──────────────────────┘ │ words…      │                │                           │
│ status line              └─────────────┘                │                           │
└──────────────────────────────┴──────────────────────────────────────────────────────┘
```

- **Top bar:** the shared TopBar.
  - "New puzzle" opens the pattern picker: thumbnails of the standard grids.
  - "Fill the rest" autofills every empty square and keeps the user's letters.
  - Undo and Redo, also Ctrl+Z and Ctrl+Shift+Z.
  - A filled count.
- **Left column:** the answer bar, then the grid, then the status line, all
  top-aligned.
  - **Answer bar:** the label, the pattern with word breaks, and an editable
    split box "(4,5)" or "(3-4)". A split that doesn't add up to the answer's
    length is ignored.
  - **Grid:**
    - Click a square to select it and light up its answer. Click it again, or
      press Space, to switch between across and down.
    - Type A–Z to fill and advance. Backspace deletes and steps back. Arrows
      move. Tab and Shift+Tab go to the next and previous answer.
    - Word breaks are a thick bar between squares; a hyphen is a short dash
      across the border.
    - **Fixed crossword colours in both themes:** white squares, black blocks,
      dark letters, light-blue answer, blue current square.
- **Words popover:** right-click a square, or press Ctrl+Space, to open a
  popover beside the selected answer. It flips to the other side near the
  window edge. It contains:
  - a header with the answer label and two icon buttons, with tooltips only:
    a wand for "Fill just this answer" and an eraser for "Clear this answer".
    Clearing keeps letters that belong to complete crossing answers.
  - a search box, and chips: Any · 1 word · 2 words · 3+ words.
  - the list of fitting entries, including phrases with their split. **No notes
    under words.** Words that would leave a crossing answer with nothing that
    fits are not listed; tighter words rank lower.

  Placing a word, pressing Escape or clicking elsewhere closes it. Placing a
  phrase sets the answer's split. There is **no toggle button and no other
  right-click menu.**
- **Right: the clue list**, typeset like a printed puzzle in two fixed columns,
  Across and Down.
  - Each row is the **number** (bold sans), the **clue** (serif) and the
    **enumeration** (small, muted). The answer itself is not shown.
  - A row with no clue yet shows its number and an amber **"no clue"** tag.
  - The selected answer's row has a blue left edge and a blue number. Clicking
    a row selects that answer.
  - Lists only scroll vertically, with a thin scrollbar visible only on hover.
- **Status line:**
  - "n of m answers filled"
  - "every answer is a word", or the ones that aren't, each with "Add to my
    words"
  - "everything still fillable", or "nothing in the word list fits 3 down"
  - "no repeats", or the repeats
  - British checking rules met, or what's wrong
- **No horizontal scrollbars anywhere**, and **no instruction text** on screen.

## Your own words (user, 2026-11-17)

- Any letters can be typed, in or out of the dictionary, and the split box
  works on any answer.
- An answer that isn't in the word list is named in the status line, with an
  **Add to my words** action. It appends the answer to the user's `words.txt`,
  shaped by its split (e.g. "zob rist" for (3,4)), and reloads the word list.
  The word then counts as a word everywhere: status, suggestions and autofill.
- "Fill the rest" never changes letters already in the grid.

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
- A words popover on right-click or Ctrl+Space, with fill and clear icons (no side panel, no toggle).
- A clue list beside the grid: number, clue and enumeration; amber "no clue" tag when missing.
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

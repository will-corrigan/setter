# Setter UX audit (2026-11-17)

Screens checked from the running app: Write clues (empty, with picks, dark,
1100 px wide), Build grid with the words popover, the template picker, the
Export menu. Findings are grouped by area. Research and agreed changes are
recorded at the end.

## Whole app

1. **The top bar wraps when the window is narrower.** At 1100 px, Theme and
   Credits drop to a second row. There are 13 controls in one row on Build
   grid.
2. **The top bar changes between pages.** Build grid has New puzzle, the
   title, Fill, Undo and Redo. Write clues has the answer navigator and *New
   sample*. Two different "new" actions sit in two places, and the title can
   only be edited on Build grid.
3. **Rare actions take prime space.** Credits and the theme are top-level
   buttons used once in a blue moon. Open, Save and Export are three buttons
   for what is one File menu elsewhere.
4. **"Theme: light" is a state shown as a label.** It isn't clear whether
   clicking it sets light or leaves it. A menu with the three choices and a
   check mark is the standard pattern.
5. **The window title is always "Setter".** It doesn't show the puzzle's
   name or that there are unsaved changes, so the taskbar and window switcher
   don't help.
6. **There's no home.** On start you land in the last puzzle or a sample.
   Finding other puzzles means the OS file dialog, and nothing shows progress
   (filled, clued) across puzzles.
7. **Editable text doesn't look editable.** The title and setter in the top
   bar look like labels until hovered.
8. **The keyboard can't reach everything.** There are a few shortcuts but no
   overview and no way to run any command by name.

## Write clues

9. **You lose your place in the grid.** The page shows the answer's tiles
   but not where it sits or what crosses it, and the other clues are hidden,
   so repeated indicators and similar surfaces go unnoticed.
10. **Progress isn't visible.** "32 of 36 clued" is the only signal. There's
    no jump to the next unclued answer, and ‹ › steps through every answer in
    order.
11. **The clue area is loosely composed.** The Definition Start/End switch
    floats at the far right, away from the clue box. "Reads as" and
    "Explanation" are small grey lines below it. The enumeration sits outside
    the box.
12. **The word bank's widths don't follow its content.** A device with two
    candidates (alternate letters) gets a huge half-empty column, while the
    long lists are cramped. Every column repeats its own Filter box.
13. **Some groups hold one item** ("Word: tree"), which costs a header and a
    row for something already shown in the tiles.

## Build grid

14. **Clue rows can't be edited where they're seen.** Clicking a row selects
    the answer but doesn't offer a way to write that clue.
15. **The template picker has no visible close.** Escape and clicking outside
    work, but nothing on screen says so.

## Research (summary; sources in the linked pages)

- **Start-up and home.** Reopen the last puzzle by default, and keep a home
  screen one click away (the brand mark). List puzzles in rows with a small
  grid thumbnail and their progress: crossword grids look alike, so big
  thumbnail cards help little (NN/g). Rename, Duplicate, Show in folder and
  Delete go on each row. Pinning and type-to-filter. An empty home is a New
  puzzle action and grid choices, with no paragraphs.
  [Apple: Launching](https://developer.apple.com/design/human-interface-guidelines/launching),
  [NN/g thumbnails](https://www.nngroup.com/articles/mobile-list-thumbnail/),
  [Exet](https://github.com/viresh-ratnakar/exet)
- **Command palette.** Register every action once (name, shortcut, whether it
  works now), and drive the menus, buttons and palette from that one list.
  Fuzzy matching with aliases. Recent items when the box is empty. Shortcuts
  shown on every row. A second step for things like "Open puzzle…". ARIA
  combobox. A visible "Ctrl K" key label on a search button is how people find
  it, without instruction text.
  [Superhuman](https://blog.superhuman.com/how-to-build-a-remarkable-command-palette/),
  [VS Code](https://code.visualstudio.com/api/ux-guidelines/command-palette),
  [GitHub](https://docs.github.com/en/get-started/accessibility/github-command-palette),
  [W3C ARIA combobox](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-autocomplete-list/)
- **Top bar and menus.** At most three groups: identity and title, navigation,
  primary actions. Document actions go in a File menu in the standard order
  (New, Open, Open Recent, Duplicate, Rename, Export ▸, Print). App settings,
  including the theme, go in a settings menu, with About (Credits) there too.
  Unavailable items are greyed out, not hidden. Use the standard shortcuts.
  [Apple: Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars),
  [Apple: Menu bar](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar)
- **Writing clues.** Next/previous and "next unclued" keys, as in Crossword
  Compiler (Ctrl+N/P) and Exet. Draft or done status per clue. The grid stays
  in view with the current answer and its crossings. Tools follow the
  selection. Repeated words across clues get a quiet whole-puzzle flag.
  [Crossword Compiler clue editor](https://crossword-compiler.com/en/help/html/clueeditor.htm),
  [Exet](https://github.com/viresh-ratnakar/exet)
- **Heuristics.** Nielsen's 10, plus visible keyboard focus, tooltips on focus
  as well as hover, autosave and restore, and greyed-out unavailable commands.
  Skipping tutorials doesn't hurt success (NN/g), which backs the "no
  instruction text" rule.
  [NN/g heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/),
  [NN/g onboarding](https://www.nngroup.com/articles/onboarding-tutorials/)

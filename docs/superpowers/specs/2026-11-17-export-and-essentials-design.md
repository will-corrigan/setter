# Export and essentials: design

Status: autonomous follow-up, requested by the user on 2026-11-17 before bed:
"look into any UX that might benefit the app or critical features that are
missing (like saving etc) and work on those. When I wake up I expect to be able
to fill in a crossword, do the clues and save it to either an html file so
someone can open it in their browser and play it interactively, or one of those
filetypes … that are popular."

## What's missing for that morning workflow

1. **Export.** The engine can produce:
   - a self-contained **web page** (Exolve player inlined; verified in headless
     Chromium),
   - **.puz** (Across Lite; checksums verified with puzpy),
   - **.ipuz**.

   The app has no way to save these yet.
2. **Puzzle title and setter name.** Exports show them, but the app never lets
   you set them: the title is "Sample puzzle" and the setter is blank.
3. **Losing work.** There's no autosave and no warning on quit, so a crash or a
   closed window loses everything since the last Save.
4. **Starting a fresh puzzle.** Handled by the Build grid page's "New puzzle"
   (template picker).

## Decisions (made autonomously; the user can revisit)

- **Export menu** in the top bar: one "Export" button opens a small menu with
  three items:
  - "Web page (.html)"
  - "Across Lite (.puz)"
  - "ipuz (.ipuz)"

  Each opens a save dialog with the right extension, defaulting to the puzzle
  title as the file name.
  - If .puz isn't possible because squares are empty, a Notice gives the
    reason: "Fill every square before exporting .puz."
  - The web page and ipuz export unfinished grids too; Exolve shows "?"
    squares, and ipuz uses null.
  - After a successful export, a short Notice confirms it: "Exported
    <name>.html".
- **Puzzle details:** click the title in the top bar to edit it, plus the
  setter's name. The setter name is remembered in settings and used for new
  puzzles.
- **Autosave:** every edit to the puzzle is written, debounced by about a
  second, to `autosave.xword` in the config folder. On startup, if an autosave
  is newer than `last_file`, or there's no last file, it's opened, with a
  Notice: "Recovered unsaved work." Saving to a real file keeps the autosave in
  step.
- **Annotations in exports:** a clue's annotation field already goes into the
  Exolve page, which reveals it with "Reveal all".

## Out of scope tonight

- Live "play together" sessions or hosting.
- PDF layout (Exolve's page has Print).
- Recent-files list.

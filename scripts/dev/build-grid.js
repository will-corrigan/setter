// Dev script for a debug build of Setter: drives the Build grid page (and the
// Write clues word bank) for screenshots.
//
//   (echo 'const SCENE = "phrase";'; cat scripts/dev/build-grid.js) > /tmp/scene.js
//   SETTER_DEV_SCRIPT=/tmp/scene.js target/dx/setter/debug/linux/app/setter
//
// Synthetic input can't be sent to WSLg windows, so this uses DOM events
// instead. The app prints "dev script finished" to stderr when it is done.
// The hook that runs it is not in release builds.
//
// Scenes (SCENE, default "words"):
//   words  - Build grid, part-way through: the long answer along the top
//            right and its crossings emptied, and its words popover open.
//            (Start the app on scripts/dev/sample-finished.xword for a
//            puzzle with clues: put its path in settings.json as last_file.)
//   phrase - a new empty grid; a two-word phrase placed in a nine-letter
//            answer and a hyphened one typed in, so the grid shows a
//            word-break bar and a hyphen.
//   keys   - a new empty grid; typing, arrows, Tab, the split box, undo and
//            redo, the popover's keys and buttons, reported as text (a check
//            that everything works).
//   picker - the template picker, open.
//   low    - the popover for the answer along the bottom row (it moves up
//            rather than hang below the grid).
//   flip   - the popover for a short answer at the right-hand edge. With
//            SETTER_DEV_SIZE=800x900 there is no room on its right, so it
//            opens on its left.
//   bank   - Write clues, on the answer and device with the most word-bank
//            columns, to show them wrapping.

const scene = typeof SCENE === "undefined" ? "words" : SCENE;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const until = async (test, ms = 20000) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const value = test();
    if (value) return value;
    await sleep(50);
  }
  return null;
};
const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];
const button = (label) => $$("button").find((b) => b.textContent.trim() === label);
// The pills of the option row inside `within` (the toggle group's own class is hashed).
const pills = (within) => $$(`${within} [data-variant="pills"] button`);
const square = (i) => $(`.xw-grid__cell[data-cell="${i}"]`);
const rightClick = (el) =>
  el.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, button: 2 }));
const press = (key, more = {}) =>
  (document.activeElement || document.body).dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...more }),
  );
const letters = () =>
  $$(".xw-grid__cell")
    .map((c) => (c.classList.contains("is-block") ? "#" : c.lastChild?.nodeType === 3 ? c.lastChild.textContent : "."))
    .join("");
const bar = () => $(".answer-bar")?.textContent.replace(/\s+/g, " ").trim();
const split = () => $(".answer-bar__input")?.value;

// The squares of the across answers, longest first: [[cell, ...], ...].
const acrossAnswers = () => {
  const cells = $$(".xw-grid__cell");
  const columns = Number($(".xw-grid").style.getPropertyValue("--columns"));
  const runs = [];
  for (let row = 0; row * columns < cells.length; row++) {
    let run = [];
    for (let col = 0; col <= columns; col++) {
      const i = row * columns + col;
      if (col < columns && !cells[i].classList.contains("is-block")) run.push(i);
      else {
        if (run.length > 1) runs.push(run);
        run = [];
      }
    }
  }
  return runs.sort((a, b) => b.length - a.length);
};

const toBuildGrid = async () => {
  await until(() => button("Build grid"));
  button("Build grid").click();
  return until(() => $(".xw-grid"));
};

const newPuzzle = async (name) => {
  button("New puzzle").click();
  const thumb = await until(() =>
    $$(".template-picker__thumb").find((t) => t.textContent.trim() === name),
  );
  thumb.click();
  await until(() => !$(".template-picker") && /^[.#]+$/.test(letters()));
};

const openWords = async (cell) => {
  rightClick(square(cell));
  await until(() => $(".word-popover"));
  return until(() => $(".word-popover__item") || $(".word-popover__note:not(.is-pending)"));
};

if (scene === "words") {
  await toBuildGrid();
  // A puzzle part-way through: empty the long answer along the top right and
  // every answer that crosses it, then ask for the words that fit it.
  const columns = Number($(".xw-grid").style.getPropertyValue("--columns"));
  const answer = acrossAnswers().find((run) => run.length >= 7 && run[0] < columns && run[0] > 0);
  const open = (i) => square(i) !== null;
  const doomed = new Set(answer);
  for (const top of answer) {
    for (let i = top + columns; i < columns * columns && open(i); i += columns) doomed.add(i);
  }
  $(".xw-grid").focus();
  for (const i of doomed) {
    square(i).click();
    await until(() => square(i).classList.contains("is-current"));
    if (letters()[i] !== ".") press("Backspace");
    await sleep(15);
  }
  // A square with no down answer through it, so the across answer is the one selected.
  const acrossOnly = answer.find((i) => !open(i + columns));
  square(acrossOnly).click();
  await until(() => square(answer[0]).classList.contains("is-answer"));
  await openWords(answer[0]);
  await until(() => $$(".word-popover__item").length > 1, 3000);
  return `${bar()} · ${$$(".word-popover__item").length} words · ${$(".status-line").textContent}`;
}

if (scene === "phrase") {
  await toBuildGrid();
  await newPuzzle("British 15 · A");
  // Select an across answer at its first square, whichever way the cursor points.
  const selectAcross = async (run) => {
    $(".xw-grid").focus();
    if (!square(run[0]).classList.contains("is-current")) square(run[0]).click();
    await until(() => square(run[0]).classList.contains("is-current"));
    if (!bar().includes("across")) press(" ");
    await until(() => bar().includes("across"));
  };
  const twoWords = () => {
    const items = $$(".word-popover__item");
    return items.length > 0 && items.every((i) => /\(\d+[,-]\d+\)$/.test(i.textContent));
  };
  const nine = acrossAnswers().find((run) => run.length === 9);
  await selectAcross(nine);
  await openWords(nine[0]);
  pills(".word-popover").find((chip) => chip.textContent.trim() === "2 words").click();
  await until(twoWords);
  // The first one written as two words (a space, so a bar), not hyphened.
  const phrase = $$(".word-popover__item").find((i) => /\(\d+,\d+\)$/.test(i.textContent));
  const text = phrase.textContent;
  phrase.click();
  await until(() => !$(".word-popover"));
  // A hyphened answer too, typed in by hand with its split set in the box.
  const seven = acrossAnswers().find((run) => run.length === 7);
  await selectAcross(seven);
  for (const ch of "CATCHUP") {
    press(ch);
    await sleep(30);
  }
  const box = $(".answer-bar__input");
  box.value = "5-2";
  box.dispatchEvent(new Event("change", { bubbles: true }));
  await until(() => $('[data-break-right="hyphen"]'), 3000);
  // Back on the phrase, so the answer bar shows it with its break.
  await selectAcross(nine);
  await until(() => $(".status-line").textContent.startsWith("2 of"), 3000);
  return `${text} · ${bar()} (${split()}) · bars ${$$('[data-break-right="word"]').length} · hyphens ${$$('[data-break-right="hyphen"]').length}`;
}

if (scene === "keys") {
  await toBuildGrid();
  await newPuzzle("British 13 · A");
  const report = [];
  const first = acrossAnswers().sort((a, b) => a[0] - b[0])[0];
  const row = () => letters().slice(first[0], first[0] + first.length);
  const at = () => $(".is-current").dataset.cell;
  const ctrl = (key, more = {}) =>
    document.dispatchEvent(new KeyboardEvent("keydown", { key, ctrlKey: true, bubbles: true, ...more }));
  // The first square is already selected: a second click would turn to down.
  square(first[1]).click();
  $(".xw-grid").focus();
  await sleep(100);
  report.push(`click: ${bar()} at ${at()}`);
  press("ArrowLeft");
  await sleep(40);
  for (const ch of "cab") {
    press(ch);
    await sleep(40);
  }
  report.push(`typed cab: ${row()} at ${at()}`);
  press("Backspace");
  await sleep(40);
  press("Backspace");
  await sleep(40);
  report.push(`backspace twice: ${row()} at ${at()}`);
  ctrl("z");
  await sleep(80);
  report.push(`undo: ${row()} at ${at()}`);
  ctrl("z");
  await sleep(80);
  report.push(`undo: ${row()} at ${at()}`);
  ctrl("Z", { shiftKey: true });
  await sleep(80);
  report.push(`redo: ${row()} at ${at()}`);
  report.push(`undo/redo buttons: ${button("Undo").disabled ? "off" : "on"}/${button("Redo").disabled ? "off" : "on"}`);
  press("ArrowRight");
  await sleep(40);
  report.push(`right: ${at()}`);
  press(" ");
  await sleep(40);
  report.push(`space: ${bar()}`);
  press(" ");
  await sleep(40);
  press("Tab");
  await sleep(40);
  report.push(`space, tab: ${bar()}`);
  press("Tab", { shiftKey: true });
  await sleep(40);
  report.push(`shift+tab: ${bar()}`);
  // A split that doesn't add up is ignored; one that does is kept.
  const change = async (text) => {
    const box = $(".answer-bar__input");
    box.value = text;
    box.dispatchEvent(new Event("change", { bubbles: true }));
    await sleep(150);
    return `${text} -> ${split()} "${bar()}"`;
  };
  report.push(`bad split: ${await change("1,1")}`);
  report.push(`good split: ${await change("2,3")} bars ${$$('[data-break-right="word"]').length}`);
  report.push(`plain length: ${await change(String(first.length))} bars ${$$('[data-break-right="word"]').length}`);
  // Ctrl+Space opens the popover; Escape closes it; a click elsewhere closes it.
  ctrl(" ");
  await until(() => $(".word-popover"), 3000);
  report.push(`ctrl+space: ${$(".word-popover") ? "open" : "closed"}, focus ${document.activeElement.className}`);
  await until(() => $(".word-popover__item"), 3000);
  // Search: every word listed starts with the letters typed.
  const search = $(".word-popover__search");
  const count = $$(".word-popover__item").length;
  search.value = "cabi";
  search.dispatchEvent(new Event("input", { bubbles: true }));
  await until(() => $$(".word-popover__item").every((i) => i.textContent.startsWith("cabi")), 3000);
  report.push(`search cabi: ${count} -> ${$$(".word-popover__item").map((i) => i.textContent).join(", ")}`);
  press("ArrowDown");
  await sleep(60);
  report.push(`arrow down: ${$('.word-popover__item[aria-selected="true"]')?.textContent}`);
  press("Escape");
  await until(() => !$(".word-popover"), 3000);
  report.push(`escape: ${$(".word-popover") ? "open" : "closed"}, focus ${document.activeElement.className}`);
  await openWords(first[0]);
  $(".clue-sheet__title").click();
  await until(() => !$(".word-popover"), 3000);
  report.push(`click elsewhere: ${$(".word-popover") ? "open" : "closed"}`);
  // Enter places the word under the arrow keys.
  await openWords(first[0]);
  const top = $(".word-popover__item").textContent;
  press("Enter");
  await until(() => !$(".word-popover"), 3000);
  report.push(`enter: ${top} -> ${row()}, then ${bar()}`);
  // The wand fills one answer; the eraser clears one.
  await openWords(at());
  const name = $(".word-popover__label").textContent;
  $('.word-popover [aria-label="Fill just this answer"]').click();
  await until(() => !$(".word-popover"), 5000);
  report.push(`wand on ${name}: ${letters().replace(/[#.]/g, "").length} letters in the grid, then ${bar()}`);
  await openWords(first[0]);
  $('.word-popover [aria-label="Clear this answer"]').click();
  await until(() => !$(".word-popover"), 3000);
  report.push(`eraser: ${row()}`);
  // Clicking a clue row selects its answer.
  $$(".clue-sheet__row")[3].click();
  await sleep(80);
  report.push(`clue row 4: ${bar()}, marked ${$('.clue-sheet__row[aria-current="true"] b').textContent}`);
  button("Fill the rest").click();
  await until(() => !letters().includes("."), 25000);
  report.push(`fill the rest: ${letters().includes(".") ? "gaps left" : "full"} · ${$(".note").textContent}`);
  await until(() => $(".status-line").textContent.startsWith("30 of 30"), 5000);
  await until(() => $(".status-line__item"), 3000);
  report.push(`status: ${$$(".status-line__item").map((i) => i.textContent).join(" | ")}`);
  report.push(`sideways scroll: ${$$("*").filter((el) => el.scrollWidth > el.clientWidth + 1 && getComputedStyle(el).overflowX !== "visible" && getComputedStyle(el).overflowX !== "hidden").map((el) => el.className).join(",") || "none"}`);
  return report.join("\n");
}

if (scene === "picker") {
  await toBuildGrid();
  button("New puzzle").click();
  await until(() => $(".template-picker__thumb"));
  await sleep(200);
  return $$(".template-picker__thumb").map((t) => t.textContent.trim()).join(", ");
}

if (scene === "low" || scene === "flip") {
  await toBuildGrid();
  const columns = Number($(".xw-grid").style.getPropertyValue("--columns"));
  const answers = acrossAnswers();
  const answer =
    scene === "low"
      ? answers.find((run) => run[0] === columns * (columns - 1))
      : answers.find((run) => run.length <= 5 && run[run.length - 1] % columns === columns - 1);
  const acrossOnly = answer.find((i) => square(i + columns) === null && square(i - columns) === null);
  $(".xw-grid").focus();
  square(acrossOnly).click();
  await until(() => square(answer[0]).classList.contains("is-answer"));
  await openWords(answer[0]);
  await sleep(200);
  const pop = $(".word-popover").getBoundingClientRect();
  const grid = $(".xw-grid").getBoundingClientRect();
  const first = square(answer[0]).getBoundingClientRect();
  const last = square(answer[answer.length - 1]).getBoundingClientRect();
  const round = (n) => Math.round(n);
  return [
    `${bar()} · side ${$(".word-popover-anchor").dataset.side}`,
    `popover x ${round(pop.left)}-${round(pop.right)} y ${round(pop.top)}-${round(pop.bottom)}`,
    `answer x ${round(first.left)}-${round(last.right)} y ${round(first.top)}-${round(first.bottom)}`,
    `grid x ${round(grid.left)}-${round(grid.right)} y ${round(grid.top)}-${round(grid.bottom)}`,
    `window ${document.documentElement.clientWidth}x${document.documentElement.clientHeight}`,
  ].join("\n");
}

if (scene === "bank") {
  const ready = () => !$(".side-nav[aria-busy]") && $(".side-nav__count");
  const label = () => $(".answer-nav__label")?.textContent;
  const tracks = () =>
    $$(".word-column").reduce((n, c) => n + (c.dataset.wide === "true" ? 2 : 1), 0);
  let best = { tracks: 0 };
  for (let answer = 0; answer < 30 && best.tracks < 8; answer++) {
    await until(ready);
    const devices = $$(".side-nav__item").filter((d) => !d.classList.contains("is-empty"));
    for (let d = 0; d < devices.length && best.tracks < 8; d++) {
      const name = devices[d].firstElementChild.textContent;
      devices[d].click();
      await until(() => devices[d].getAttribute("aria-current") === "true");
      await sleep(60);
      const options = pills(".clue-head").length || 1;
      for (let o = 0; o < options && best.tracks < 8; o++) {
        pills(".clue-head")[o]?.click();
        await sleep(60);
        if (tracks() > best.tracks) best = { tracks: tracks(), answer, name, option: o, label: label() };
      }
    }
    if (best.tracks < 8) {
      const before = label();
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", altKey: true }));
      await until(() => label() !== before);
    }
  }
  // Go back to the best one found.
  while (label() !== best.label) {
    const before = label();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", altKey: true }));
    await until(() => label() !== before);
  }
  await until(ready);
  const device = $$(".side-nav__item").find((d) => d.firstElementChild.textContent === best.name);
  device.click();
  await until(() => device.getAttribute("aria-current") === "true");
  await sleep(100);
  pills(".clue-head")[best.option]?.click();
  await sleep(300);
  const bank = $(".word-bank");
  const rows = new Set($$(".word-column").map((c) => Math.round(c.getBoundingClientRect().top))).size;
  const main = $(".shell__main");
  return `${best.label} ${best.name} option ${best.option}: ${$$(".word-column").length} columns in ${rows} rows · bank ${bank.scrollWidth}/${bank.clientWidth} · main ${main.scrollWidth}/${main.clientWidth}`;
}

return `unknown scene: ${scene}`;

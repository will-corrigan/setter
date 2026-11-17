// Dev script for a debug build of Setter:
//
//   SETTER_DEV_SCRIPT=scripts/dev/charade-picks.js target/dx/setter/debug/linux/app/setter
//
// Synthetic input can't be sent to WSLg windows, so this drives the Write
// clues page with DOM events instead. It steps through the answers (Alt+→)
// until one has a charade with words in every column, picks the first word
// in each, and writes a clue from the picks. The app prints
// "dev script finished" to stderr when it is done. Used for screenshots;
// the hook that runs it is not in release builds.

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
const bankReady = () =>
  !document.querySelector(".side-nav[aria-busy]") && document.querySelector(".side-nav__count");
const device = (name) =>
  [...document.querySelectorAll(".side-nav__item")].find(
    (item) => item.firstElementChild.textContent === name,
  );
const label = () => document.querySelector(".answer-nav__label")?.textContent;
const columns = () => [...document.querySelectorAll(".word-column")];
const firstWord = (column) => column.querySelector(".word-column__item");

// Step through the answers until one can be clued as a charade from the bank alone.
let found = false;
for (let i = 0; i < 80 && !found; i++) {
  await until(bankReady);
  if (!device("Charade").classList.contains("is-empty")) {
    device("Charade").click();
    await until(() => device("Charade").getAttribute("aria-current") === "true");
    await until(() => columns().length > 0);
    found = columns().every(firstWord);
  }
  if (!found) {
    const before = label();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", altKey: true }));
    await until(() => label() !== before);
  }
}
if (!found) return "no charade found";

// Pick the first word in every column.
const picks = [];
for (const column of columns()) {
  const item = firstWord(column);
  item.click();
  picks.push([column.dataset.tone, item.textContent]);
  await sleep(60);
}

// Write a clue from the picks: the parts, a joining word, then the definition.
const words = (tone) => picks.filter(([t]) => t === tone).map(([, word]) => word);
const parts = picks.filter(([t]) => /^p\d$/.test(t)).map(([, word]) => word);
const [joining] = words("ind");
const [definition] = words("def");
const wordplay = [parts[0], joining, ...parts.slice(1)].join(" ");
const clue = `${wordplay} for ${definition}`;
const field = document.querySelector(".clue-field__input");
field.value = clue.charAt(0).toUpperCase() + clue.slice(1);
field.dispatchEvent(new Event("input", { bubbles: true }));
await until(() => document.querySelector(".check"), 2000);
return `${label()}: ${field.value}`;

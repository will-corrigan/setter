// Setter player: solve a crossword in the browser. Self-contained, no network.
(function () {
  "use strict";
  const D = JSON.parse(document.getElementById("setter-data").textContent);
  const N = D.w * D.h;
  const KEY = "setter:" + D.id;
  const $ = (s) => document.querySelector(s);
  const open = (i) => D.cells[i] !== "#";
  const blank = () => ({ letters: Array(N).fill(""), elapsed: 0, revealed: [], solved: false, reveals: 0 });
  let S = load();
  let cur = D.cells.findIndex((c, i) => open(i) && !S.letters[i]);
  if (cur < 0) cur = D.cells.findIndex((c, i) => open(i));
  let dir = "a";
  let wrong = new Set();
  let running = false, last = 0;

  // ---------- data helpers ----------
  const at = {}; // cell -> {a: clue index, d: clue index}
  D.clues.forEach((c, k) => c.cells.forEach((i) => ((at[i] ??= {})[c.dir] = k)));
  const clueAt = (i, d) => (at[i] && at[i][d] !== undefined ? at[i][d] : null);
  const light = () => clueAt(cur, dir) ?? clueAt(cur, dir === "a" ? "d" : "a");
  const full = (k) => D.clues[k].cells.every((i) => S.letters[i]);

  function load() {
    try {
      const s = JSON.parse(localStorage.getItem(KEY));
      if (s && Array.isArray(s.letters) && s.letters.length === N) return Object.assign(blank(), s);
    } catch (e) {}
    return blank();
  }
  function save() {
    try { localStorage.setItem(KEY, JSON.stringify(S)); } catch (e) {}
  }

  // ---------- build ----------
  const grid = $("#grid");
  const size = () => Math.max(24, Math.min(40, Math.floor((Math.min(window.innerWidth, 1240) - (window.innerWidth > 920 ? 560 : 48)) / D.w)));
  grid.style.gridTemplateColumns = `repeat(${D.w}, var(--cell))`;
  const cellEls = [];
  for (let i = 0; i < N; i++) {
    const el = document.createElement("div");
    el.className = "cell" + (open(i) ? "" : " block") + (D.breaks[i] ? " " + D.breaks[i] : "");
    if (open(i)) {
      el.dataset.i = i;
      el.innerHTML = (D.numbers[i] ? `<span class="n">${D.numbers[i]}</span>` : "") + `<span class="ch"></span><span class="br-r"></span><span class="br-b"></span>`;
    }
    grid.appendChild(el);
    cellEls.push(el);
  }
  const keys = document.createElement("input");
  keys.className = "keys";
  keys.setAttribute("aria-label", "Type letters");
  keys.autocomplete = "off";
  keys.autocapitalize = "characters";
  grid.appendChild(keys);
  function fit() { grid.style.setProperty("--cell", size() + "px"); }
  addEventListener("resize", fit);
  fit();

  const lists = { a: $("#across"), d: $("#down") };
  const clueEls = D.clues.map((c, k) => {
    const el = document.createElement("div");
    el.className = "clue";
    el.dataset.k = k;
    el.innerHTML = `<b>${c.n}</b><span>${esc(c.text)}${c.text ? " " : ""}<span class="en">(${esc(c.en)})</span></span>` + (c.anno ? `<span class="anno" hidden>${esc(c.anno)}</span>` : "");
    lists[c.dir].appendChild(el);
    return el;
  });
  function esc(s) { return String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c])); }

  // ---------- render ----------
  function render() {
    const k = light();
    const lit = new Set(k === null ? [] : D.clues[k].cells);
    for (let i = 0; i < N; i++) {
      if (!open(i)) continue;
      const el = cellEls[i];
      el.classList.toggle("lit", lit.has(i) && i !== cur);
      el.classList.toggle("cur", i === cur);
      el.classList.toggle("wrong", wrong.has(i));
      el.classList.toggle("revealed", S.revealed.includes(i));
      el.querySelector(".ch").textContent = S.letters[i] || "";
    }
    clueEls.forEach((el, j) => {
      el.classList.toggle("on", j === k);
      el.classList.toggle("done", full(j));
      const a = el.querySelector(".anno");
      if (a) a.hidden = !S.solved;
    });
    if (k !== null) {
      const c = D.clues[k];
      $("#current").innerHTML = `<b>${c.n} ${c.dir === "a" ? "across" : "down"}</b>${esc(c.text)} <span class="en">(${esc(c.en)})</span>`;
      if (focusClue) clueEls[k].scrollIntoView({ block: "nearest" });
    }
    const filled = D.cells.filter((c, i) => open(i) && S.letters[i]).length;
    const total = D.cells.filter((c, i) => open(i)).length;
    $("#progress").textContent = `${filled} of ${total} squares`;
    showTime();
    helperCount();
  }
  let focusClue = true;

  // ---------- timer ----------
  function fmt(ms) {
    const t = Math.floor(ms / 1000), h = Math.floor(t / 3600), m = Math.floor((t % 3600) / 60), s = t % 60;
    return (h ? h + ":" + String(m).padStart(2, "0") : m) + ":" + String(s).padStart(2, "0");
  }
  // `last` is when time was last added to S.elapsed. Only accrue() moves it,
  // so redrawing the clock (showTime, on every keystroke) never drops the
  // time since the last whole-second tick.
  function accrue() {
    const now = Date.now();
    if (running) S.elapsed += now - last;
    last = now;
  }
  function showTime() {
    const t = $("#timer");
    t.textContent = fmt(S.elapsed);
    t.classList.toggle("done", S.solved);
  }
  function start() {
    if (!running && !S.solved) { running = true; last = Date.now(); }
  }
  setInterval(() => {
    if (document.hidden) return; // paused: hiding already banked the time
    accrue(); showTime(); if (running) save();
  }, 1000);
  // Pause while the page is hidden: bank the time up to hiding, then restart
  // the count from when it's shown again.
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) { accrue(); if (running) save(); }
    else last = Date.now();
  });
  addEventListener("pagehide", () => { if (!document.hidden) accrue(); if (running) save(); });

  // ---------- editing ----------
  function select(i, d) {
    if (!open(i)) return;
    if (i === cur && d === undefined && clueAt(i, dir === "a" ? "d" : "a") !== null) dir = dir === "a" ? "d" : "a";
    cur = i;
    if (d) dir = d;
    if (clueAt(cur, dir) === null) dir = dir === "a" ? "d" : "a";
    render();
  }
  function step(delta) {
    const k = light();
    if (k === null) return;
    const cells = D.clues[k].cells, p = cells.indexOf(cur) + delta;
    if (p >= 0 && p < cells.length) cur = cells[p];
  }
  // A solved grid is locked: typing and erasing do nothing until Clear.
  function unreveal(i) { S.revealed = S.revealed.filter((j) => j !== i); }
  function type(ch) {
    if (S.solved || !open(cur)) return;
    start();
    S.letters[cur] = ch.toUpperCase();
    unreveal(cur); // the solver's own letter now, not a revealed one
    wrong.delete(cur);
    step(1);
    save();
    render();
    checkSolved();
  }
  function erase() {
    if (S.solved) return;
    if (!S.letters[cur]) step(-1);
    S.letters[cur] = "";
    unreveal(cur);
    wrong.delete(cur);
    save();
    render();
  }
  function move(dr, dc) {
    dir = dr ? "d" : "a";
    let r = Math.floor(cur / D.w), c = cur % D.w;
    for (;;) {
      r += dr; c += dc;
      if (r < 0 || c < 0 || r >= D.h || c >= D.w) break;
      if (open(r * D.w + c)) { cur = r * D.w + c; break; }
    }
    if (clueAt(cur, dir) === null) dir = dir === "a" ? "d" : "a";
    render();
  }
  function nextClue(delta) {
    let k = light() ?? 0;
    k = (k + delta + D.clues.length) % D.clues.length;
    const c = D.clues[k];
    dir = c.dir;
    cur = c.cells.find((i) => !S.letters[i]) ?? c.cells[0];
    render();
  }

  // ---------- check, reveal, clear ----------
  function scope(what) {
    if (what === "letter") return [cur];
    if (what === "word") { const k = light(); return k === null ? [] : D.clues[k].cells; }
    return D.cells.map((c, i) => i).filter(open);
  }
  function check(what) {
    if (!D.hasSolution) return;
    for (const i of scope(what)) if (S.letters[i] && S.letters[i] !== D.cells[i]) wrong.add(i);
    render();
  }
  function reveal(what) {
    if (!D.hasSolution || S.solved) return;
    let n = 0;
    for (const i of scope(what)) {
      if (S.letters[i] !== D.cells[i]) {
        S.letters[i] = D.cells[i];
        if (!S.revealed.includes(i)) S.revealed.push(i);
        n++;
      }
      wrong.delete(i);
    }
    if (n) { S.reveals++; start(); }
    save();
    render();
    checkSolved();
  }
  // Clear keeps revealed letters. After a solve it's how to play again:
  // - Clear grid starts afresh: every letter, revealed or not, the timer and
  //   the reveal count all reset, as if the puzzle had never been opened.
  // - Clear word un-solves if it takes any letter away; the timer then
  //   carries on from the solved time when the solver next types.
  function clear(what) {
    if (S.solved && what === "grid") {
      S = blank();
      wrong.clear();
      running = false;
      $("#solved").classList.remove("show");
    } else {
      let changed = false;
      for (const i of scope(what)) {
        if (!S.revealed.includes(i)) { changed ||= !!S.letters[i]; S.letters[i] = ""; wrong.delete(i); }
      }
      if (S.solved && changed) { S.solved = false; $("#solved").classList.remove("show"); }
    }
    save();
    render();
  }
  function checkSolved() {
    if (!D.hasSolution || S.solved) return;
    if (!D.cells.every((c, i) => !open(i) || S.letters[i] === c)) return;
    accrue();
    S.solved = true;
    running = false;
    save();
    render();
    celebrate();
  }

  // ---------- celebration ----------
  function celebrate() {
    $("#solved-time").textContent = fmt(S.elapsed);
    $("#solved-help").textContent = S.reveals ? ` with ${S.reveals} reveal${S.reveals > 1 ? "s" : ""}` : "";
    $("#solved").classList.add("show");
    $("#solved button").focus();
    if (!matchMedia("(prefers-reduced-motion: reduce)").matches) confetti();
  }
  function confetti() {
    const cv = document.createElement("canvas");
    cv.className = "confetti";
    document.body.appendChild(cv);
    const ctx = cv.getContext("2d");
    const dpr = window.devicePixelRatio || 1;
    const W = (cv.width = innerWidth * dpr), H = (cv.height = innerHeight * dpr);
    const colours = ["#1f63d6", "#7445d6", "#b25e00", "#0f7a6b", "#c2366f", "#178045", "#ffd21f"];
    const bits = Array.from({ length: 220 }, () => ({
      x: W / 2 + (Math.random() - 0.5) * W * 0.3, y: H * 0.35,
      vx: (Math.random() - 0.5) * 22 * dpr, vy: (-Math.random() * 18 - 6) * dpr,
      w: (6 + Math.random() * 6) * dpr, h: (8 + Math.random() * 8) * dpr,
      a: Math.random() * 6.28, va: (Math.random() - 0.5) * 0.3, c: colours[(Math.random() * colours.length) | 0],
    }));
    const t0 = performance.now();
    (function frame(t) {
      const age = t - t0;
      ctx.clearRect(0, 0, W, H);
      for (const b of bits) {
        b.vy += 0.45 * dpr; b.vx *= 0.99; b.x += b.vx; b.y += b.vy; b.a += b.va;
        ctx.save(); ctx.translate(b.x, b.y); ctx.rotate(b.a);
        ctx.globalAlpha = Math.max(0, 1 - age / 4200);
        ctx.fillStyle = b.c; ctx.fillRect(-b.w / 2, -b.h / 2, b.w, b.h * Math.abs(Math.cos(b.a * 2)));
        ctx.restore();
      }
      if (age < 4200) requestAnimationFrame(frame); else cv.remove();
    })(t0);
  }

  // ---------- anagram helper ----------
  const helper = $("#helper"), wheel = $("#wheel"), letters = $("#ana-letters"), attempt = $("#ana-try");
  let order = [];
  function shuffle() {
    order = letters.value.toUpperCase().replace(/[^A-Z]/g, "").split("");
    for (let i = order.length - 1; i > 0; i--) { const j = (Math.random() * (i + 1)) | 0; [order[i], order[j]] = [order[j], order[i]]; }
    drawWheel();
  }
  function drawWheel() {
    const used = attempt.value.toUpperCase().replace(/[^A-Z]/g, "").split("");
    const pool = [...used];
    wheel.innerHTML = order.map((ch, i) => {
      const a = (i / Math.max(1, order.length)) * Math.PI * 2 - Math.PI / 2, r = order.length > 1 ? 78 : 0;
      const hit = pool.indexOf(ch);
      if (hit >= 0) pool.splice(hit, 1);
      return `<span class="${hit >= 0 ? "used" : ""}" style="left:${105 + r * Math.cos(a)}px;top:${105 + r * Math.sin(a)}px">${ch}</span>`;
    }).join("");
    const left = [...order];
    const over = [];
    for (const ch of used) { const p = left.indexOf(ch); if (p >= 0) left.splice(p, 1); else over.push(ch); }
    $("#ana-left").textContent = left.join(" ") || "none";
    $("#ana-over").textContent = over.length ? `not in the letters: ${over.join(" ")}` : "";
    helperCount();
  }
  function helperCount() {
    const k = light();
    const n = letters.value.replace(/[^A-Za-z]/g, "").length;
    $("#ana-count").textContent = `${n} letter${n === 1 ? "" : "s"}` + (k !== null ? `, ${D.clues[k].n} ${D.clues[k].dir === "a" ? "across" : "down"} needs ${D.clues[k].cells.length}` : "");
  }
  letters.addEventListener("input", shuffle);
  attempt.addEventListener("input", drawWheel);
  $("#ana-shuffle").addEventListener("click", shuffle);
  $("#ana-toggle").addEventListener("click", (e) => {
    const on = !helper.classList.contains("open");
    helper.classList.toggle("open", on);
    e.currentTarget.setAttribute("aria-pressed", on);
    if (on) {
      const sel = String(window.getSelection() || "").trim();
      if (sel) { letters.value = sel; shuffle(); }
      letters.focus();
    }
  });

  // ---------- events ----------
  grid.addEventListener("click", (e) => {
    const c = e.target.closest(".cell[data-i]");
    if (!c) return;
    select(+c.dataset.i);
    keys.focus({ preventScroll: true });
  });
  document.addEventListener("click", (e) => {
    const cl = e.target.closest(".clue");
    if (cl) {
      const c = D.clues[+cl.dataset.k];
      focusClue = false;
      select(c.cells.find((i) => !S.letters[i]) ?? c.cells[0], c.dir);
      focusClue = true;
      keys.focus({ preventScroll: true });
    }
    const m = e.target.closest(".menu > .btn");
    document.querySelectorAll(".menu.open").forEach((x) => { if (!m || x !== m.parentElement) x.classList.remove("open"); });
    if (m) m.parentElement.classList.toggle("open");
    const act = e.target.closest("[data-act]");
    if (act) {
      const [verb, what] = act.dataset.act.split("-");
      ({ check, reveal, clear })[verb](what);
      act.closest(".menu").classList.remove("open");
      keys.focus({ preventScroll: true });
    }
  });
  $("#print").addEventListener("click", () => window.print());
  $("#solved button").addEventListener("click", () => $("#solved").classList.remove("show"));
  // Mobile keyboards often send keydown "Unidentified", so letters arrive
  // here. The input holds one space so Backspace always has something to
  // delete and fires beforeinput. Backspace in this input, from any
  // keyboard, is handled only there (keydown lets it through), so it can
  // never erase twice.
  const SPACER = " ";
  function resetKeys() { keys.value = SPACER; keys.setSelectionRange(1, 1); }
  keys.addEventListener("beforeinput", (e) => {
    if (e.inputType === "deleteContentBackward") { e.preventDefault(); erase(); }
  });
  keys.addEventListener("input", () => {
    const v = keys.value.replace(/[^A-Za-z]/g, "");
    resetKeys();
    if (v) type(v[v.length - 1]);
  });
  keys.addEventListener("focus", resetKeys);
  document.addEventListener("keydown", (e) => {
    if (e.target.closest && e.target.closest(".helper")) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key === "Escape") { $("#solved").classList.remove("show"); return; }
    if (e.target !== keys && e.target !== document.body) return;
    if (/^[a-zA-Z]$/.test(e.key)) { e.preventDefault(); type(e.key); }
    else if (e.key === "Backspace" && e.target === keys) resetKeys(); // erased in beforeinput
    else if (e.key === "Backspace" || e.key === "Delete") { e.preventDefault(); erase(); }
    else if (e.key === "ArrowLeft") { e.preventDefault(); move(0, -1); }
    else if (e.key === "ArrowRight") { e.preventDefault(); move(0, 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); move(-1, 0); }
    else if (e.key === "ArrowDown") { e.preventDefault(); move(1, 0); }
    else if (e.key === " ") { e.preventDefault(); select(cur); }
    else if (e.key === "Tab") { e.preventDefault(); nextClue(e.shiftKey ? -1 : 1); }
  });

  if (!D.hasSolution) document.querySelectorAll("[data-needs-solution]").forEach((el) => (el.hidden = true));
  render();
  resetKeys();
  keys.focus({ preventScroll: true });
  window.setterPlayer = { type, reveal, check, clear, select, state: () => S };
})();

// The clue editor's behaviour, installed once per window. The editor is a
// contenteditable div that Rust never fills: Rust sets data-segments (the
// clue as pieces, coloured or plain) and this script draws them, keeping the
// caret where it was. Typing is sent back through the hidden sink input;
// dragging a coloured piece moves it to the nearest gap between words and is
// sent through the hidden arrange input.
(() => {
  if (window.__setterClueEditor) return;
  window.__setterClueEditor = true;
  const EDITOR = ".clue-field__editor";

  const isChip = (n) => n.nodeType === 1 && n.classList.contains("clue-chip");
  const isMarker = (n) => n.nodeType === 1 && n.classList.contains("clue-drop");

  // The clue as plain text.
  const textOf = (ed) => {
    let s = "";
    for (const n of ed.childNodes) {
      if (isMarker(n)) continue;
      if (n.nodeName === "BR") s += " ";
      else s += n.textContent;
    }
    return s.replace(/ /g, " ").replace(/[\r\n]+/g, " ");
  };

  // The pieces on screen, in the same shape as data-segments.
  const piecesOf = (ed) => {
    const out = [];
    for (const n of ed.childNodes) {
      if (isMarker(n)) continue;
      const tone = isChip(n) ? n.dataset.tone : null;
      const t = n.textContent.replace(/ /g, " ");
      const last = out[out.length - 1];
      if (!tone && last && !last.tone) last.t += t;
      else if (t) out.push({ t, tone });
    }
    return out;
  };

  const caretOffset = (ed) => {
    const sel = getSelection();
    if (!sel.rangeCount || !ed.contains(sel.anchorNode)) return null;
    const r = sel.getRangeAt(0);
    const pre = document.createRange();
    pre.selectNodeContents(ed);
    pre.setEnd(r.endContainer, r.endOffset);
    return pre.toString().length;
  };

  const setCaret = (ed, offset) => {
    const r = document.createRange();
    let left = offset;
    for (const n of ed.childNodes) {
      const len = n.textContent.length;
      if (left <= len) {
        if (n.nodeType === 3) r.setStart(n, left);
        else if (left === 0) r.setStartBefore(n);
        else r.setStartAfter(n);
        r.collapse(true);
        const sel = getSelection();
        sel.removeAllRanges();
        sel.addRange(r);
        return;
      }
      left -= len;
    }
    r.selectNodeContents(ed);
    r.collapse(false);
    getSelection().removeAllRanges();
    getSelection().addRange(r);
  };

  const draw = (ed) => {
    if (ed.__composing || ed.__dragging) return;
    let want = [];
    try { want = JSON.parse(ed.dataset.segments || "[]"); } catch (_) {}
    if (JSON.stringify(want) === JSON.stringify(piecesOf(ed))) return;
    const offset = document.activeElement === ed ? caretOffset(ed) : null;
    ed.textContent = "";
    for (const s of want) {
      if (s.tone) {
        const chip = document.createElement("span");
        chip.className = "clue-chip";
        chip.contentEditable = "false";
        chip.dataset.tone = s.tone;
        chip.textContent = s.t;
        ed.appendChild(chip);
      } else {
        ed.appendChild(document.createTextNode(s.t));
      }
    }
    if (offset !== null) setCaret(ed, offset);
  };

  const send = (ed, sinkClass, value) => {
    const sink = ed.closest(".clue-field").querySelector(sinkClass);
    if (!sink) return;
    sink.value = value;
    sink.dispatchEvent(new Event("input", { bubbles: true }));
  };

  // ---- typing ----
  document.addEventListener("input", (e) => {
    const ed = e.target.closest && e.target.closest(EDITOR);
    if (!ed || ed.__composing) return;
    send(ed, ".clue-field__sink", textOf(ed));
  });
  document.addEventListener("compositionstart", (e) => {
    const ed = e.target.closest && e.target.closest(EDITOR);
    if (ed) ed.__composing = true;
  });
  document.addEventListener("compositionend", (e) => {
    const ed = e.target.closest && e.target.closest(EDITOR);
    if (!ed) return;
    ed.__composing = false;
    send(ed, ".clue-field__sink", textOf(ed));
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && e.target.closest && e.target.closest(EDITOR)) e.preventDefault();
  });
  document.addEventListener("paste", (e) => {
    const ed = e.target.closest && e.target.closest(EDITOR);
    if (!ed) return;
    e.preventDefault();
    const text = (e.clipboardData.getData("text/plain") || "").replace(/\s+/g, " ");
    document.execCommand("insertText", false, text);
  });

  // ---- dragging a piece ----
  let drag = null;

  // The gap between words nearest a point in the editor, as a range.
  const gapAt = (ed, x, y) => {
    const r = document.caretRangeFromPoint && document.caretRangeFromPoint(x, y);
    if (!r || !ed.contains(r.startContainer)) return null;
    const node = r.startContainer;
    if (node.nodeType === 3) {
      const s = node.data;
      let o = r.startOffset;
      const gap = (i) => i <= 0 || i >= s.length || /\s/.test(s[i - 1]) || /\s/.test(s[i]);
      let a = o, b = o;
      while (!gap(a)) a--;
      while (!gap(b)) b++;
      o = o - a <= b - o ? a : b;
      r.setStart(node, o);
      r.collapse(true);
    }
    return r;
  };

  const spaceAround = (chip) => {
    const fix = (side) => {
      const n = side === "before" ? chip.previousSibling : chip.nextSibling;
      if (!n) return;
      const t = n.textContent;
      const touching = side === "before" ? t && !/\s$/.test(t) : t && !/^[\s,.;:!?)]/.test(t);
      if (touching || isChip(n)) chip[side](document.createTextNode(" "));
    };
    fix("before");
    fix("after");
  };

  document.addEventListener("pointerdown", (e) => {
    const chip = e.target.closest && e.target.closest(".clue-chip");
    const ed = chip && chip.closest(EDITOR);
    if (!ed || e.button !== 0) return;
    e.preventDefault();
    drag = { chip, ed, x: e.clientX, y: e.clientY, moved: false, ghost: null, marker: null };
  });

  document.addEventListener("pointermove", (e) => {
    if (!drag) return;
    if (!drag.moved && Math.hypot(e.clientX - drag.x, e.clientY - drag.y) < 4) return;
    if (!drag.moved) {
      drag.moved = true;
      drag.ed.__dragging = true;
      const box = drag.chip.getBoundingClientRect();
      drag.dx = drag.x - box.left;
      drag.dy = drag.y - box.top;
      drag.ghost = drag.chip.cloneNode(true);
      drag.ghost.classList.add("clue-chip--ghost");
      document.body.appendChild(drag.ghost);
      drag.chip.classList.add("is-dragging");
      drag.marker = document.createElement("span");
      drag.marker.className = "clue-drop";
    }
    drag.ghost.style.left = e.clientX - drag.dx + "px";
    drag.ghost.style.top = e.clientY - drag.dy + "px";
    const gap = gapAt(drag.ed, e.clientX, e.clientY);
    if (gap && !drag.chip.contains(gap.startContainer)) gap.insertNode(drag.marker);
    else drag.marker.remove();
  });

  const finish = (drop) => {
    const { chip, ed, ghost, marker, moved } = drag;
    drag = null;
    if (!moved) {
      // A click: put the caret after the piece.
      ed.focus();
      const r = document.createRange();
      r.setStartAfter(chip);
      r.collapse(true);
      getSelection().removeAllRanges();
      getSelection().addRange(r);
      return;
    }
    ghost.remove();
    chip.classList.remove("is-dragging");
    ed.__dragging = false;
    if (drop && marker.parentNode) {
      marker.replaceWith(chip);
      spaceAround(chip);
      ed.normalize();
      send(ed, ".clue-field__arrange", textOf(ed));
    } else {
      marker.remove();
      ed.normalize();
    }
  };
  document.addEventListener("pointerup", () => drag && finish(true));
  document.addEventListener("pointercancel", () => drag && finish(false));
  document.addEventListener("keydown", (e) => {
    if (drag && e.key === "Escape") finish(false);
  });

  // ---- drawing: now, for editors added later, and when Rust changes one ----
  document.querySelectorAll(EDITOR).forEach(draw);
  new MutationObserver((records) => {
    for (const rec of records) {
      if (rec.type === "attributes" && rec.target.matches(EDITOR)) draw(rec.target);
      for (const n of rec.addedNodes || []) {
        if (n.nodeType !== 1) continue;
        if (n.matches(EDITOR)) draw(n);
        n.querySelectorAll && n.querySelectorAll(EDITOR).forEach(draw);
      }
    }
  }).observe(document.body, {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: ["data-segments"],
  });
})();

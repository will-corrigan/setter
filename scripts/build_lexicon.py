#!/usr/bin/env python3
"""Download Exet's Lufz English lexicon and convert it to data/lexicon.tsv.gz.

Output format (one entry per line, ordered by popularity, most popular first):
    entry<TAB>pronunciations
where pronunciations are CMUdict phoneme strings separated by '|',
e.g. "and\tAE N D|AH N D".

Usage: python3 scripts/build_lexicon.py [path-to-lufz-en-lexicon.js]
"""
import gzip, json, sys, urllib.request
from pathlib import Path

URL = "https://raw.githubusercontent.com/viresh-ratnakar/exet/master/lufz-en-lexicon.js"
OUT = Path(__file__).resolve().parent.parent / "data" / "lexicon.tsv.gz"


def load_blocks(text):
    merged, pos = {}, 0
    while (i := text.find("JSON.parse(`", pos)) >= 0:
        i += len("JSON.parse(`")
        j = text.find("`)", i)
        merged.update(json.loads(text[i:j]))
        pos = j
    return merged


def main():
    if len(sys.argv) > 1:
        text = Path(sys.argv[1]).read_text(encoding="utf-8")
    else:
        with urllib.request.urlopen(URL) as r:
            text = r.read().decode("utf-8")
    d = load_blocks(text)
    lex, phones = d["lexicon"], d["phones"]
    seen, n = set(), 0
    with gzip.open(OUT, "wt", encoding="utf-8", compresslevel=9) as f:
        f.write(f"# {d['id']} — derived from UKACD18 (c) J Ross Beresford, Exet (c) Viresh Ratnakar, CMUdict. See data/NOTICE.md\n")
        for entry, ph in zip(lex, phones):
            entry = entry.strip()
            if not entry or "\t" in entry or entry in seen:
                continue
            seen.add(entry)
            f.write(entry + "\t" + "|".join(" ".join(p) for p in ph) + "\n")
            n += 1
    print(f"wrote {n} entries to {OUT}")


if __name__ == "__main__":
    main()

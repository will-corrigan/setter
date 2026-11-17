#!/usr/bin/env python3
"""Build data/quality.tsv.gz: how good each word in the lexicon is as grid fill.

    python3 scripts/build_quality.py path/to/"nediger list.txt"

The Nediger list (https://codeberg.org/bewilderingly/Nediger-list, MIT) is a
hand-scored list of crossword entries: 25 = only if necessary, 49 = likely too
racy, 51 = usable, 99 = good. It is North American, so British spellings score
low; here a British spelling takes the score of its American twin
(colour = color). Partial phrases made only of function words ("as to",
"for her") are rare however they score, since British cryptics avoid them.

Classes, one per lexicon key:
  g  good  - Nediger 51/99, directly or via its American twin
  f  fair  - Nediger 25, or not in Nediger but among the 30,000 most popular
  r  rare  - everything else: racy (49), partials, obscure words
Standard library only.
"""

import gzip
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEXICON = ROOT / "data" / "lexicon.tsv.gz"
OUT = ROOT / "data" / "quality.tsv.gz"
POPULAR_ENOUGH = 30_000

# Words that make a phrase a partial ("as to", "for her", "the same").
FUNCTION = set("""
a an the this that these those some any each every all both either neither no
i me my mine you your yours he him his she her hers it its we us our ours they
them their theirs one ones oneself myself yourself himself herself itself
ourselves themselves who whom whose which what
to of in on at by for from with without into onto upon about above below over
under up down out off as than then so such too very more most less least much
many few other another same own
and or nor but if when while whilst though although because since unless until
be is am are was were been being do does did done have has had having
will would shall should can could may might must not
there here where how why
""".split())


def key(text):
    return re.sub(r"[^a-z]", "", text.lower())


def american_twins(word):
    """Likely American spellings of a British word (colour -> color)."""
    w = word.lower()
    out = set()
    swaps = [
        ("our", "or"), ("tre", "ter"), ("ise", "ize"), ("isa", "iza"),
        ("yse", "yze"), ("ysi", "yzi"), ("ogue", "og"), ("mme", "m"),
        ("ence", "ense"), ("lled", "led"), ("lling", "ling"), ("ller", "ler"),
        ("aeo", "eo"), ("ae", "e"), ("oe", "e"), ("ium", "um"),
        ("eue", "ue"), ("que", "ck"),
    ]
    for uk, us in swaps:
        start = 0
        while (i := w.find(uk, start)) != -1:
            out.add(w[:i] + us + w[i + len(uk):])
            start = i + 1
    special = {"grey": "gray", "tyre": "tire", "kerb": "curb", "plough": "plow",
               "cheque": "check", "aeroplane": "airplane", "jewellery": "jewelry",
               "mould": "mold", "moult": "molt", "storey": "story", "pyjamas": "pajamas"}
    for uk, us in special.items():
        if uk in w:
            out.add(w.replace(uk, us))
    out.discard(w)
    return {key(x) for x in out}


def load_nediger(path):
    scores = {}
    for line in open(path, encoding="utf-8"):
        line = line.rstrip("\n\r\t ")
        if ";" not in line:
            continue
        text, score = line.rsplit(";", 1)
        try:
            score = int(score.strip())
        except ValueError:
            continue
        k = key(text)
        if k:
            scores[k] = max(scores.get(k, 0), score)
    return scores


def classify(text, rank, scores):
    words = re.findall(r"[a-z']+", text.lower())
    if len(words) > 1 and all(w.strip("'") in FUNCTION for w in words):
        return "r"
    k = key(text)
    score = scores.get(k)
    twin = max((scores.get(t, 0) for t in american_twins(text)), default=0)
    if twin > (score or 0) and twin != 49:
        score = twin
    if score is None:
        return "f" if rank <= POPULAR_ENOUGH else "r"
    if score >= 51:
        return "g"
    if score == 25:
        return "f"
    return "r"  # 49: likely too racy for many venues


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    scores = load_nediger(sys.argv[1])
    classes = {}
    rank = 0
    for line in gzip.open(LEXICON, "rt", encoding="utf-8"):
        if line.startswith("#"):
            continue
        rank += 1
        text = line.split("\t")[0]
        k = key(text)
        if k and k not in classes:
            classes[k] = classify(text, rank, scores)
    with gzip.open(OUT, "wt", encoding="utf-8", compresslevel=9) as f:
        f.write("# Fill quality per lexicon key: g good, f fair, r rare. Built by\n")
        f.write("# scripts/build_quality.py from the Nediger list (MIT); see data/NOTICE.md.\n")
        for k, c in classes.items():
            f.write(f"{k}\t{c}\n")
    counts = {c: sum(1 for v in classes.values() if v == c) for c in "gfr"}
    print(f"wrote {OUT.relative_to(ROOT)}: {counts}")
    for w in ["embus", "whoom", "nemesia", "as to", "for her", "the same", "colour",
              "centre", "realise", "programme", "faeces", "kerb", "grey", "small",
              "cricket", "oboe", "etui", "osier", "anything", "whilst", "town centre"]:
        print(f"  {w}: {classes.get(key(w), '-')}")


if __name__ == "__main__":
    main()

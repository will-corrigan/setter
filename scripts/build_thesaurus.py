#!/usr/bin/env python3
"""Build the bundled offline thesaurus files.

Inputs (download first):
  - Open English WordNet 2025 JSON:
    https://github.com/globalwordnet/english-wordnet/releases/download/2025-edition/english-wordnet-2025-json.zip
  - Moby Thesaurus II (public domain), comma-separated lines "head,syn,syn,...":
    https://raw.githubusercontent.com/words/moby/master/words.txt

Outputs:
  data/wordnet.tsv.gz  one line per synset:
      pos <TAB> members (|) <TAB> gloss <TAB> hypernyms <TAB> hyponyms <TAB> parts
      <TAB> part_of <TAB> members_of_group <TAB> member_of <TAB> related
    where relation fields are comma-separated synset line numbers (0-based).
  data/wordnet-index.tsv.gz  lemma <TAB> synset line numbers, most common sense first.
  data/moby.tsv.gz     the Moby lines, unchanged apart from trimming.

Usage: python3 scripts/build_thesaurus.py OEWN_JSON_ZIP MOBY_TXT
"""
import gzip, json, sys, zipfile
from pathlib import Path

DATA = Path(__file__).resolve().parent.parent / "data"


SLUR_MARKERS = ("ethnic slur", "offensive term", "offensive name", "offensive word", "(offensive)",
                "disparaging term", "(disparaging)", "derogatory term", "(derogatory)", "offensive slang")


def main(oewn_zip, moby_txt):
    z = zipfile.ZipFile(oewn_zip)
    synsets = {}
    for name in z.namelist():
        if name.startswith("entries-") or not name.endswith(".json"):
            continue
        data = json.loads(z.read(name))
        if isinstance(data, dict):
            synsets.update({k: v for k, v in data.items() if isinstance(v, dict) and "members" in v})
    # Drop senses WordNet itself marks as slurs or disparaging terms, and words
    # that exist only as such (they're also removed from Moby below).
    def is_slur(syn):
        gloss = " ".join(syn.get("definition", [])).lower()
        return any(m in gloss for m in SLUR_MARKERS)

    slur_ids = {sid for sid, syn in synsets.items() if is_slur(syn)}
    slur_lemmas = {m.replace("_", " ").lower() for sid in slur_ids for m in synsets[sid].get("members", [])}
    clean_lemmas = {m.replace("_", " ").lower() for sid, syn in synsets.items() if sid not in slur_ids for m in syn.get("members", [])}
    slur_only = slur_lemmas - clean_lemmas
    for sid in slur_ids:
        del synsets[sid]
    print(f"dropped {len(slur_ids)} slur senses; {len(slur_only)} slur-only words")

    ids = sorted(synsets)
    index = {sid: i for i, sid in enumerate(ids)}

    rel = {k: [[] for _ in ids] for k in
           ("hyper", "hypo", "parts", "part_of", "members", "member_of", "related")}

    def link(kind, a, b):
        if a in index and b in index:
            rel[kind][index[a]].append(index[b])

    for sid, s in synsets.items():
        for h in s.get("hypernym", []) + s.get("instance_hypernym", []):
            link("hyper", sid, h)
            link("hypo", h, sid)
        for m in s.get("mero_part", []) + s.get("mero_substance", []):
            link("parts", sid, m)
            link("part_of", m, sid)
        for m in s.get("mero_member", []):
            link("members", sid, m)
            link("member_of", m, sid)
        for r in s.get("similar", []) + s.get("also", []) + s.get("domain_topic", []) + s.get("attribute", []):
            link("related", sid, r)

    def field(xs):
        return ",".join(str(x) for x in sorted(set(xs)))

    with gzip.open(DATA / "wordnet.tsv.gz", "wt", encoding="utf-8", compresslevel=9) as f:
        f.write("# Open English WordNet 2025 (CC BY 4.0) — see data/NOTICE.md\n")
        for i, sid in enumerate(ids):
            s = synsets[sid]
            members = "|".join(m.replace("_", " ") for m in s.get("members", []))
            gloss = (s.get("definition") or [""])[0].replace("\t", " ").replace("\n", " ")
            f.write("\t".join([s.get("partOfSpeech", "n"), members, gloss] +
                              [field(rel[k][i]) for k in ("hyper", "hypo", "parts", "part_of", "members", "member_of", "related")]) + "\n")
    print(f"wrote {len(ids)} synsets")

    # Per-lemma sense order (WordNet lists a word's senses most-common first).
    n = 0
    with gzip.open(DATA / "wordnet-index.tsv.gz", "wt", encoding="utf-8", compresslevel=9) as f:
        f.write("# lemma <TAB> synset line numbers, most common sense first\n")
        for name in sorted(z.namelist()):
            if not name.startswith("entries-"):
                continue
            for lemma, by_pos in json.loads(z.read(name)).items():
                order = [index[sense["synset"]] for pos in by_pos.values()
                         for sense in pos.get("sense", []) if sense.get("synset") in index]
                if order:
                    f.write(lemma.replace("_", " ").replace("\t", " ") + "\t" + ",".join(map(str, order)) + "\n")
                    n += 1
    print(f"wrote {n} lemmas")

    n = 0
    with open(moby_txt, encoding="latin-1") as src, gzip.open(DATA / "moby.tsv.gz", "wt", encoding="utf-8", compresslevel=9) as f:
        f.write("# Moby Thesaurus II (public domain) — see data/NOTICE.md\n")
        for line in src:
            words = [w for w in line.strip().split(",") if w and w.lower() not in slur_only]
            if len(words) > 1 and words[0].lower() not in slur_only:
                f.write(",".join(words) + "\n")
                n += 1
    print(f"wrote {n} moby entries")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])

#!/usr/bin/env bash
# Cut a font down to Latin and write it as woff2, small enough to bundle.
#   scripts/subset_font.sh SomeFont[wght].ttf app/assets/fonts/somefont-latin-var-normal.woff2
# Then point SANS or SERIF in crates/engine/src/fonts.rs at the new file and
# update its licence line in data/NOTICE.md.
set -euo pipefail
in="$1"; out="$2"
nix shell --impure --expr 'with import <nixpkgs> {}; python3.withPackages (p: [p.fonttools p.brotli])' \
  --command python3 -m fontTools.subset "$in" \
  --unicodes='U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+2000-206F,U+2074,U+20AC,U+2122,U+2190-2193,U+2212,U+2215,U+FEFF,U+FFFD' \
  --layout-features='*' --flavor=woff2 --output-file="$out"
ls -la "$out"

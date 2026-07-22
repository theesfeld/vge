#!/usr/bin/env bash
# Build CMFD boards with pcbnew, run DRC, export Gerbers/drill/pos via kicad-cli.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="$(cd "$ROOT/.." && pwd)"
cd "$REPO"

# Locate kicad tools (nix-shell or PATH)
if ! command -v kicad-cli >/dev/null 2>&1; then
  echo "kicad-cli not on PATH. Run: nix-shell -p kicad --run 'bash hardware/tools/kicad_export.sh'" >&2
  exit 1
fi

KICAD_BASE="$(dirname "$(dirname "$(command -v kicad-cli)")")"
# python site-packages for pcbnew often lives in kicad-base
for sp in /nix/store/*-kicad-base-*/lib/python*/site-packages; do
  if [[ -e "$sp/_pcbnew.so" || -e "$sp/pcbnew.py" ]]; then
    export PYTHONPATH="$sp${PYTHONPATH:+:$PYTHONPATH}"
    break
  fi
done

FP_DIR="$(ls -d /nix/store/*-kicad-footprints-*/share/kicad/footprints 2>/dev/null | tail -1 || true)"
if [[ -n "${FP_DIR:-}" ]]; then
  export KICAD_FOOTPRINT_DIR="$FP_DIR"
fi

echo "== pcbnew build =="
python3 "$ROOT/tools/kicad_build.py"

BOARD_A="$ROOT/elec/bezel-mcu/cmfd-board-a.kicad_pcb"
BOARD_B="$ROOT/elec/carrier-som/cmfd-board-b.kicad_pcb"
FAB="$ROOT/elec/fab"
mkdir -p "$FAB/board-a-bezel" "$FAB/board-b-carrier"

export_one() {
  local board="$1"
  local outdir="$2"
  local tag="$3"
  mkdir -p "$outdir"
  rm -f "$outdir"/*.{gbr,gbl,gtl,gts,gbs,gto,gbo,gtp,gbp,gm1,drl,pos,csv,pdf,png} 2>/dev/null || true
  # clear previous exports but keep README if any we rewrite

  echo "== DRC $tag =="
  kicad-cli pcb drc \
    --output "$FAB/kicad-drc-${tag}.rpt" \
    --format report \
    --units mm \
    --severity-error \
    --severity-warning \
    --refill-zones \
    --save-board \
    "$board" || true
  # also json for tooling
  kicad-cli pcb drc \
    --output "$FAB/kicad-drc-${tag}.json" \
    --format json \
    --units mm \
    --severity-error \
    --severity-warning \
    --refill-zones \
    "$board" || true

  echo "== Gerbers $tag =="
  kicad-cli pcb export gerbers \
    --output "$outdir" \
    --layers "F.Cu,B.Cu,F.Paste,B.Paste,F.SilkS,B.SilkS,F.Mask,B.Mask,Edge.Cuts" \
    --subtract-soldermask \
    --no-protel-ext \
    --check-zones \
    "$board"

  echo "== Drill $tag =="
  kicad-cli pcb export drill \
    --output "$outdir" \
    --format excellon \
    --excellon-zeros-format decimal \
    --excellon-units mm \
    --generate-map \
    --map-format pdf \
    "$board" || kicad-cli pcb export drill --output "$outdir" --format excellon "$board"

  echo "== Position $tag =="
  kicad-cli pcb export pos \
    --output "$outdir/${tag}-pos.csv" \
    --format csv \
    --units mm \
    --side both \
    "$board" || true

  # zip
  (
    cd "$outdir"
    zip -q -r "$FAB/cmfd-${tag}-kicad-gerbers.zip" .
  )
  echo "Zip: $FAB/cmfd-${tag}-kicad-gerbers.zip"
}

export_one "$BOARD_A" "$FAB/board-a-bezel" "board-a"
export_one "$BOARD_B" "$FAB/board-b-carrier" "board-b"

# summary
echo
echo "== DRC error counts =="
for f in "$FAB"/kicad-drc-board-*.rpt; do
  echo "--- $(basename "$f") ---"
  rg -n "error|Error|ERROR|violation|Violations" "$f" | head -40 || true
  wc -l "$f"
done

echo
echo "Done. Boards: $BOARD_A $BOARD_B"
echo "Fab dir: $FAB"
ls -la "$FAB"/*.zip 2>/dev/null || true

# Send these files to a PCB house

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)  
**Primary path:** **KiCad headless** (`kicad-cli` + `pcbnew`) — not the older pure-Python Gerber writer.

## Rebuild (reproducible)

```bash
nix-shell -p kicad zip --run 'bash hardware/tools/kicad_export.sh'
```

This:

1. Builds `cmfd-board-a.kicad_pcb` / `cmfd-board-b.kicad_pcb` via `hardware/tools/kicad_build.py`  
2. Runs `kicad-cli pcb drc --refill-zones`  
3. Exports Gerbers, Excellon drill, pick-and-place CSV  
4. Zips fab folders  

## Upload these zips

| Board | Zip |
|-------|-----|
| Board A — bezel MCU | `cmfd-board-a-kicad-gerbers.zip` |
| Board B — SoM carrier | `cmfd-board-b-kicad-gerbers.zip` |

Also present (legacy geometry-only, **do not prefer**):

- `cmfd-board-a-bezel-gerbers.zip` / `cmfd-board-b-carrier-gerbers.zip` from early `gen_pcbs.py`

## Fab options

| Setting | Value |
|---------|--------|
| Layers | **2** |
| Thickness | **1.6 mm** |
| Copper | 1 oz |
| Surface | HASL lead-free or ENIG |
| Min track/space | 0.15 / 0.15 mm class |
| Qty first spin | 5 of each |

## Source projects (open in KiCad 10)

| Board | Path |
|-------|------|
| A | `hardware/elec/bezel-mcu/cmfd-board-a.kicad_pcb` |
| B | `hardware/elec/carrier-som/cmfd-board-b.kicad_pcb` |

## DRC status (honest)

Reports: `kicad-drc-board-a.rpt`, `kicad-drc-board-b.rpt`.

| Board | Expectation |
|-------|-------------|
| **A** | **OSB 4×5 matrix is routed** (ROW/COL nets + vias + MCU pins). See `bezel-mcu/cmfd-board-a-pinmap.md`. Ratsnest for matrix is largely complete (~10 residual unconnected, often power). Remaining **shorts/crossings** are dense-frame geometry (channels share a 17.5 mm ring) — clean in KiCad GUI before SMT. Silk/mask noise from vias is expected. |
| **B** | USB-C / RJ45 **edge-mount** footprints intentionally violate generic hole/copper-edge rules; courtyard density on connectors is normal for those library parts. |

**v1 purpose:** mechanical fit-check + **matrix connectivity** + land validation.  
**Before dense paid assembly:** open Board A in KiCad GUI, fix remaining shorting_items / tracks_crossing, pour GND/3V3, re-run DRC.

## Assembly

- Position files: `board-a-bezel/board-a-pos.csv`, `board-b-carrier/board-b-pos.csv`  
- SoM module is **not** on SMT BOM — seat after board fab  
- OSB switches: 6×6 mm THT (`SW_PUSH_6mm_*`) under printed caps  

## Print case

See `hardware/mech/print/cmfd-print-files.zip`.

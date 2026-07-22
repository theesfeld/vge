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
| **A** | Mostly silk / text / residual copper-edge near display cutout and OSB frame. **Unconnected** OSB nets until matrix is routed (v1 is placement + mechanical). |
| **B** | USB-C / RJ45 **edge-mount** footprints intentionally violate generic hole/copper-edge rules; courtyard density on connectors is normal for those library parts. |

**v1 purpose:** mechanical fit-check (case, OSB pitch, standoffs, port windows) + SMT land validation.  
**Before dense paid assembly:** open the `.kicad_pcb` in KiCad GUI, pour GND, route matrix / power, re-run DRC to **0 errors** (edge-connector exceptions documented).

## Assembly

- Position files: `board-a-bezel/board-a-pos.csv`, `board-b-carrier/board-b-pos.csv`  
- SoM module is **not** on SMT BOM — seat after board fab  
- OSB switches: 6×6 mm THT (`SW_PUSH_6mm_*`) under printed caps  

## Print case

See `hardware/mech/print/cmfd-print-files.zip`.

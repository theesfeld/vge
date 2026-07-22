# CMFD studio stills (build-accurate)

**Rule:** only show frames after visual QA. Geometry comes from **KiCad** and **OpenSCAD STLs**, not freestyle AI.

## KiCad raytrace (real PCB)

Generated with `kicad-cli pcb render` from the fab boards.

| File | Content |
|------|---------|
| `kicad-board-a-top.png` | Board A passive switch frame, top |
| `kicad-board-a-iso.png` | Board A isometric |
| `kicad-board-b-top.png` | Board B carrier, top |
| `kicad-board-b-iso.png` | Board B isometric |

## Blender Cycles (STLs + layout-true boards)

Script: `hardware/tools/blender_render_cmfd.py`

```bash
nix-shell -p blender --run 'blender -b -P hardware/tools/blender_render_cmfd.py'
```

| File | Content |
|------|---------|
| `render-exploded.png` | Full stack explode: rear, battery, Board B, Board A, bezel, OSB, rockers |
| `render-closed.png` | Closed product |
| `render-front-detail.png` | Front 3/4: OSB ring, rockers, glass |
| `render-case.png` | Rear shell + front bezel STLs |
| `render-board-a-lcd.png` | Board A frame + LCD in cutout (layout match) |
| `render-board-b.png` | Board B carrier layout |
| `render-battery.png` | Battery tray STL + two 18650 cells |
| `render-buttons.png` | OSB cap + rocker STLs on switch bodies |

## Stack order (must match build)

```
front bezel + OSB caps + rockers
Board A FR4 frame (102 mm cutout)
LCD / glass in the cutout
Board B carrier
18650 tray
rear shell
```

## Do not ship

- Freestyle AI product art
- Three.js screenshots “polished” into fake boards
- Frames with screen under a solid PCB

# MLU M1 Pilot’s Guide — Visual OCR of CMFD figures

**Source PDF:** `docs/246416220-F16-MLU-M1-Pilot-s-Manual.pdf` (16PR14341)  
**Method:** `pdftoppm` render @ 150 dpi → multimodal read of figure pages  
**Renders (local):** `docs/reference/mlu-m1-ocr/p-*.png`, `fcr-*.png`, `sms2-*.png` (gitignored if large)

This document is the **visual** companion to [`mlu-m1-cmfd.md`](mlu-m1-cmfd.md). Text extraction alone misses symbology shapes and color callouts.

---

## Figure index (CMFD-critical)

| Fig | Title | PDF ≈ | What to implement |
|-----|--------|-------|-------------------|
| 1-14 | CMFD Controls | p-027 | Bezel, OSB 1–20, rockers, FCR example page |
| 1-15 | Format Selection Master Menu | p-028 | Menu layout: FCR/TGP/WPN/TFR/FLIR · SMS/HSD/DTE/TEST/FLCS · BLANK/RCCE/RESET |
| 1-16 | Reset Menu + Master Menu | p-029 | SBC DAY/NIGHT SET/RESET, NVIS OVRD |
| 1-1 | **Color Symbology table** | p-030 | Palette + graphic shapes |
| 1-17 | Color symbology FCR | p-031 | CYAN / WHITE / YELLOW callouts on FCR |
| 1-18 | Color symbology HSD | p-031 | CYAN / WHITE / YELLOW callouts on HSD |
| 1-19 | HSD Base Page | p-032 | DEP/DCPL/NORM/CNTL, FRZ, ownship, bullseye, range |
| 1-33 | SMS Inventory (NAV) | p-048 | Stations 1–9 layout, STBY/CLR, S-J |
| 1-34 | SMS MDDE | p-048 | Menu-driven data entry |
| 1-35 | SMS AAM | p-049 | AAM SPOT INV, SLAVE, RDY, stations |
| 1-36 | SMS GUN EEGS | p-049 | GUN EEGS INV, SCORE ON |
| 1-37 | SMS DGFT | sms2-050 | Combined gun + missile |
| 1-38 | SMS A-G | sms2-050 | CCRP, AD/BA, PROF, PAIR, RP |
| 3-2 | FCR A-A Base Page | fcr-136 | Full CRM/RWS symbology callouts |

---

## Hardware visual (Fig 1-14)

```
        GAIN                              SYM
     [1] [2] [3] [4] [5]
 [20]                      [6]
 [19]                      [7]
 [18]      [ GLASS ]       [8]
 [17]                      [9]
 [16]                      [10]
        BRT               CON
     [15][14][13][12][11]
```

| Bezel corner | Control |
|--------------|---------|
| Upper left rocker | **GAIN** (sensor gain) |
| Upper right rocker | **SYM** (symbology intensity) |
| Lower left rocker | **BRT** |
| Lower right rocker | **CON** |

**Bottom row L→R:** OSB **15 SWAP** · **14 active format** (highlighted box, e.g. FCR) · **13** · **12** next formats · **11 DCLT**

**Active format** is boxed/highlighted on OSB 14/13/12.

---

## Page change (Figs 1-15, 1-16) — visual flow

```
[ any format page ]  --press active OSB 12/13/14-->  [ MASTER MENU ]
[ MASTER MENU ]      --pick FCR/SMS/HSD/...------>  [ that format on slot ]
[ MASTER MENU ]      --OSB 5 RESET MENU---------->  [ SBC day/night set ]
```

### Master Menu OSB map (from figure)

| Side | Labels (as drawn) |
|------|-------------------|
| Top | BLANK · (empty) · RCCE · RESET MENU |
| Right | SMS · HSD · DTE · TEST · FLCS |
| Left | FCR · TGP · WPN · TFR · FLIR |
| Bottom | SWAP · (slot) · (slot) · (slot) · DCLT |

---

## Table 1-1 Color Symbology (OCR of figure p-030)

Palette on glass: **black** background. Named colors used on symbols:

### 1 Safety cursors — **CYAN**
| Name | Graphic (schematic) |
|------|---------------------|
| FCR cursor | Cross / brackets |
| HSD ghost AA | Cross |
| HSD ghost AG | Cross |

### 2 IFF
| Name | Color | Graphic |
|------|-------|---------|
| Friend IFF | **Green** | Circle |
| Unk IFF | **Yellow** | Square |
| Break X | **Red** | X |

### 3 Datalink
| Name | Color | Graphic |
|------|-------|---------|
| DLNK friend | **Green** | Aircraft-like |
| DLNK unk | **Yellow** | Aircraft-like |
| DLNK MKPT | **Yellow** | X |
| DLNK cursor | **Yellow** | * |
| DLNK Outside FOV | **Yellow** | ▶ |
| DLNK Peng | **Yellow** | P |

### 4 FCR
| Name | Color | Graphic |
|------|-------|---------|
| Unk bug | **Yellow** | Circle with cross |
| Unk sys trk | **Green** | Small box |
| Tank tgt | **Green** | Tank icon |

### 5 Ownship data
| Name | Color | Graphic |
|------|-------|---------|
| Own Peng | **Yellow** | P |
| Own MKPT | **Yellow** | X |
| Preplan Thrt | **Yellow** | Digit (e.g. 8) |
| **HSD ownship** | **Cyan** | Stick aircraft **+** |
| **FCR volume** | **Cyan** | Wedge / pie outline |
| **Bullseye data** | **Cyan** | Concentric rings ◎ |
| Tgt’s | **White** | △ |
| IP’s | **White** | □ |
| Lines | **White** | Wave / polyline |
| Routes | **White** | ○—○—○ |
| Text | **Green** | e.g. SMS |

### 6 Misc
| Name | Color | Graphic |
|------|-------|---------|
| Range rings | **Green** | Concentric circles |
| FCR ownship | **Cyan** | + |
| SOI | **Green** | Box corner |
| Sensor video | (video) | Camera-like icon |

### Callout boxes on Figs 1-17 / 1-18 (simpler training summary)

**FCR (1-17)**  
- **CYAN:** Bullseye position; Brg/Rng from bullseye to cursor; ownship bullseye rng & brg  
- **WHITE:** Gain gauge, radar returns, hot lines, radar cursor, radar search altitudes  
- **YELLOW:** Radar tracks, track altitudes, bugged target, bugged target altitude  

**HSD (1-18)**  
- **CYAN:** Bullseye; radar search pattern; **ownship**; bullseye brg/rng  
- **WHITE:** Range rings, nav route, target, IP, selected STPT, lines, ghost A/A cursor  
- **YELLOW:** Preplanned threats, DL mark points, DL A/G cursor, bugged target, bugged alt, DL responses  

**Correction for our library:** HSD **ownship is cyan**, not white. White is for routes/STPT/text class items.

---

## HSD Base page (Fig 1-19) — widgets & OSBs

### OSB mnemonics (as drawn)

| OSB | Mnemonic |
|-----|----------|
| 1 | **DEP** (or CEN) — centered/depressed rotary |
| 2 | **DCPL** (or CPL) — FCR range coupling |
| 3 | **NORM** (expand rotary when SOI: NORM/EXP1/EXP2) |
| 4 | (blank or MSG when text) |
| 5 | **CNTL** — control page |
| 7 | **FRZ** — freeze |
| 11 | **S-J** |
| 12 | **SMS** (format option) |
| 13 | empty / next option |
| 14 | **HSD** highlighted (active) |
| 15 | **SWAP** |
| 19–20 | Range INC/DEC arrows + scale number (e.g. 60) |

### Glass symbols (non-declutterable set)

| Symbol | Color | Notes |
|--------|-------|-------|
| Ownship stick aircraft | **Cyan** | Centered vs depressed (¾ down) |
| Bullseye ◎ | **Cyan** | Three concentric rings |
| Bullseye brg/rng digroup | lower left | With tic on outer ring |
| Range rings | **Green** | |
| Ownship markpoint | **Yellow X** | |
| Bugged FCR target | **Yellow** | Relative to FCR volume |
| HSD cursor | **White** | Only when HSD is SOI |
| Ghost FCR cursors | **White** (callout) / cyan safety table | When HSD not SOI |
| Selected STPT / route | **White** | |
| Preplanned threat | **Yellow** digit + ring | |

---

## FCR A-A Base page (Fig 3-2) — widgets

### OSB row (top): CRM · RWS · NORM · OVRD · CNTL  
### Bottom: SWAP · **FCR** · (slot) · DTE · DCLT  

### Symbology callouts on figure

| Widget / mark | Location / role |
|---------------|-----------------|
| Acquisition cursor | Center-right, primary aim |
| Sensitivity indicator | Left of scope |
| Range scale / INC / DEC | Left OSBs 19–20 area |
| Antenna elevation tic | Left scale |
| Azimuth scan width | Left (A2 / A3 / A6 style) |
| Elevation bar scan | Left |
| Azimuth scan limit lines | Vertical bounds on scope |
| Antenna azimuth tic | Bottom of scope |
| Range marks | Horizontal ticks |
| Horizon line | Across scope |
| Hot line | |
| Bugged target | Box + vector |
| Bug aspect / closure / CAS / altitude | Data block near bug |
| Secondary target (system track) | |
| Steerpoint symbol | |
| Bull’s-eye bearing/range to cursor | Lower left |
| Relative brg/rng to bull’s-eye | Lower left digroup |
| AIFF mode | Left |
| Min/max search altitudes | Right of cursor |
| DMD | Upper right of glass |
| RDY | Bottom center of glass |
| Mode/submode FOV | Top of glass data |

Library **widgets** to extract from this figure:  
`acq_cursor`, `bscope_frame`, `az_scan_gates`, `bug_target`, `bullseye_los`, `data_block`, `range_scale_osb`, `horizon_line`.

---

## SMS pages — OSB maps change per page

### Table 1-3 SMS pages by master mode

| Master mode | SMS pages |
|-------------|-----------|
| NAV | INV, MDDE* (STBY under OSB1) |
| A-A | INV, MDDE*, AAM, GUN (EEGS) |
| A-G | INV, MDDE*, A-G, GUN (STRF) |
| DGFT | DGFT, INV, MDDE* |
| MSL OVRD | MSL, INV, MDDE* |
| S-J | S-J only |
| E-J | E-J only (emerg button) |

### Fig 1-33 INV (NAV) — visual layout

```
        STBY                    CLR
   [sta4]     51 GUN            [sta5 tank]
   [sta3]     PGU28
   [sta2]  racks/weapons        [sta6..8]
   [sta1]  A-9NP                [sta9]
        SWAP  SMS   DTE   S-J
```

- Stations **1–4 left wing**, **5 center**, **6–9 right wing**  
- Order **1→9 clockwise from lower left**  
- Empty station = **dashed** box  
- Highlighted active station = solid/bright green fill  
- OSB 11 = **S-J**, OSB 14 = **SMS** active  

### Fig 1-35 AAM page OSBs

| Area | Labels |
|------|--------|
| Top | **AAM** · **SPOT** · **INV** |
| Left | **SLAVE** · **BP** · station digits **1 2 3** |
| Right | **RDY** · **5 A-9LM** · **COOL** · stations **4 8 9** |
| Bottom | SWAP · SMS · TEST · S-J |
| Center | RDY |

### Fig 1-36 GUN EEGS

Top: **GUN** · **EEGS** · **INV** · SCORE ON · 51 GUN · RDY  

### Fig 1-37 DGFT

Top: **DGFT** · **EEGS** · **SPOT** · **INV** — gun + missile combined  

### Fig 1-38 A-G

| OSB role | Example mnemonic |
|----------|------------------|
| Mode | **A-G** **CCRP** |
| Submode / INV / CNTL | **INV** **CNTL** |
| RBS | **RBS** |
| Fuse | **NOSE** · AD/BA |
| Weapon count | **8 M82** |
| Profile | **PROF2** |
| Release | **2 PAIR** · **50FT** · **RP 2** |
| Bottom | SWAP SMS TEST S-J |

**Key lesson:** SMS is many **pages**, each with a **different OSB map** — not one station grid forever.

---

## Widget catalog (from visuals → library targets)

| Widget ID | Source figure | Color |
|-----------|---------------|-------|
| `osb_chrome` + per-page map | 1-14… | Green mnemonics; highlight box on active format |
| `ownship_hsd` | 1-18, 1-19 | **Cyan** stick aircraft |
| `ownship_fcr` | 1-1, 3-2 | **Cyan** + |
| `bullseye_rings` | 1-18, 1-19 | **Cyan** ◎ |
| `bullseye_los` | 1-19, 3-2 | Cyan digroup + tic |
| `range_rings` | 1-18 | **Green** |
| `acq_cursor` | 1-17, 3-2 | Cyan safety / white radar cursor (role split) |
| `bug_target` | 1-17, 3-2 | **Yellow** |
| `track_symbol` | 1-17 | **Yellow** |
| `stpt_route` | 1-18 | **White** ○—○ |
| `threat_preplan` | 1-18 | **Yellow** digit + ring |
| `markpoint` | 1-19 | **Yellow** X |
| `sms_station_cell` | 1-33 | Green box / dashed empty / fill selected |
| `sms_inv_layout` | 1-33 | Stations 1–9 geometry |
| `data_block` | 3-2 | Multi-line numeric |
| `format_select_slots` | 1-14 | OSB 12/13/14 highlight |

---

## Implementation notes (for agents)

1. **Do not** invent a single global OSB bank.  
2. Each **layout** returns `OsbMap[1..=20]`.  
3. SMS **sub-pages** (INV/AAM/GUN/DGFT/A-G/S-J) each own a map (F4-SMS pattern).  
4. Colors must follow Table 1-1 **per symbol**, not “everything green except one accent.”  
5. Renders in `mlu-m1-ocr/` are for agent vision; re-run:  
   `pdftoppm -png -r 150 -f 26 -l 55 docs/246416220-F16-MLU-M1-Pilot-s-Manual.pdf docs/reference/mlu-m1-ocr/p`

---

*Visual OCR complete for Section 1 CMFD + SMS figures + FCR Fig 3-2. Expand with 1-20…1-23 (expand/freeze/control) and Section 5 GM/SEA as needed.*

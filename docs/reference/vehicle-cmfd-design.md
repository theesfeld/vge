# Vehicle CMFD design (Lockheed-class)

**Status:** product law for `cmfd`  
**Issues:** [#103](https://github.com/theesfeld/mfd/issues/103) · [#105](https://github.com/theesfeld/mfd/issues/105)  
**Source:** Lockheed-style design review + operator hard requirements

## Operator hard requirements

| # | Requirement | Implementation |
|---|-------------|----------------|
| 1 | **Dedicated fault code page** | Format **DTC** — always when link ready; empty = honest `NONE`; display-only (no clear) |
| 2 | **Dedicated ATT page** | Format **ATT** — horizon ball + compass/heading; always when link ready |
| 3 | **Tach + speedo gauges** | **ENG** = large RPM tach; **DRV** = large speedo + tach pair |

## Principles

1. **Muscle memory** — stable OSB slots when a function exists.  
2. **Format vs options** — format identity vs controls for that format.  
3. **Unlabeled = no function** — blank, not grey “disabled.”  
4. **Active format highlighted** (OSB 12 / 13 / 14).  
5. **Declutter** — DCLT reduces density; do not paint every PID.  
6. **Power-on honesty** — black → blank chrome → formats when probe ready.  
7. **Fail-soft** — omit or “—”; never invent bus data; display-only.  
8. **Labels are page-owned** — options change with format.

## Format decision table

| Format | Always when link ready? | Probe may omit? | Primary glass |
|--------|-------------------------|-----------------|---------------|
| **ENG** | Yes | No | **Tach (RPM)** gauge + sparse numerics |
| **DRV** | Yes | No | **Speedo + tach** gauges |
| **ATT** | Yes (operator hard) | No | Horizon ball + heading/compass |
| **DTC** | Yes (operator hard) | No (omit only if no link) | Fault list; empty = NONE |
| FUEL | Yes | Rare | Fuel % + level tape |
| FLUD | Yes | Rare | Key temp gauges (ECT/OIL/TFT) |
| ELEC | Yes | Rare | Battery gauge |
| CHAS | No | Yes (TPMS/ABS) | Tire grid when GO |
| BODY · LITE | Default on truck | May tighten later | Status grids |
| CLIM · CAM · RNG · MAP | No | Yes | Only if GO |
| BUS | Yes (shop support) | No | Dense channel dump |
| OWN · SET | Yes | No | Identity / setup |

## OSB policy (frozen)

```
        1   2   3   4   5     top — options for **active format only**
   20                       6
   19                       7     left / right — options
   18      [  GLASS  ]      8
   17                       9
   16                      10
       15  14  13  12  11     bottom
       OWN  A   B   C  DCLT
```

| Side | OSB | POC keys | Role |
|------|-----|----------|------|
| **Top** | 1–5 | `1`–`5` | **Page options** (Lights: LO HI FOG DRL INT) |
| **Right** | 6–10 | `6`–`9` `0` | **Page options** |
| **Bottom** | 15–11 | `y` `u` `i` `o` `p` (legacy qwert) | OWN · **\*fmtA** · fmtB · fmtC · DCLT |
| **Left** | 16–20 | `a` `s` `d` `f` `g` | DTC · · · SET · BUS |

**Hard rule:** top/right keys are **never** format jumps. Format change only via bottom slots 12–14 + Master Menu.

**MLU:** lit `*` format slot = Master Menu. Other slots = select. On-glass cue always.

**Example — LIGHTS:** `1`=LO · `2`=HI · `3`=FOG · `4`=DRL · `5`=INT. Not ENG/FUEL.

**Default slots after probe:** ENG · DRV · ATT.

**Blank-not-repack:** missing formats omitted from Master Menu; never slide labels.

## Widgets (discipline)

| Use as real widgets | Dense numeric only |
|---------------------|--------------------|
| RPM tach, speedo, fuel level tape | Secondary PIDs |
| Battery gauge, key temps | BUS dump, identity |
| ATT ball + heading | — |
| Tire grid (if TPMS GO) | Door/belt status |

Do not fill the face with tiny gauges for every channel.

## Power-on

1. Black face  
2. Blank content + chrome: `OWN · · · DCLT` (slots empty until probe)  
3. Probe off-glass → seed slots (ENG/DRV/ATT) → systems formats  

No GO/NOGO splash on glass. DTC is a format, not cold-power BIT.

## Master Menu

GO formats only. Pick assigns into the slot that opened the menu. Duplicate on another slot → blank other (MLU habit).

## Hardware I/O freeze (production)

**Must have on the face:**

| I/O | Role |
|-----|------|
| **20 OSB** | Softkeys — all format / option / menu navigation |
| **BRT rocker** | Brightness |
| **CON rocker** | Contrast |
| **SYM rocker** | Symbology intensity (optional use) |
| **GAIN rocker** | CAM/FLIR gain when GO; else no-op |
| **ALS** (optional) | Auto brightness bias |

**Forbidden as dedicated extra face buttons:**

| Forbidden | Why |
|-----------|-----|
| n/p next-prev format keys | Not on CMFD; use slots + Master Menu |
| Color mode dedicated button | SET → **PAL** option OSB |
| Speed unit dedicated button | SET/DRV → **UNIT** option OSB |
| Touch/mouse as primary | Gloved OSB muscle memory |

**POC keyboard only:** maps 1:1 onto OSB/rockers (`1`–`5` top, `6`–`0` right, `qwert` bottom, `asdfg` left).  
`n`/`p`/`m`/`c`/`u` are **lab aliases** — product acceptance = GPIO bezel only.

### Reachability (hardware only)

| Function | Path |
|----------|------|
| ENG / DRV / ATT | Default slots 14/13/12, or active slot → Master Menu |
| Any other GO format | Active format OSB → Master Menu → pick |
| Master Menu | Press **lit** format slot (12/13/14) |
| DCLT | OSB 11 |
| OWN | OSB 15 |
| DTC | OSB 16 |
| SET | OSB 19 |
| BUS | OSB 20 |
| UNIT | SET or DRV/ENG OSB **1–2** |
| Color palette | SET OSB **3–4** **PAL/MODE** |
| BRT | BRT rocker |

**Yes — everything product-critical is reachable with 20 OSB + rockers.** No dependence on n/p/color hardware.

## Active format SOI (must)

| Rule | Detail |
|------|--------|
| **Lit OSB** | Underline + bright label on the OSB that **owns the glass** |
| **Format on slot** | Light that slot (12/13/14) |
| **OWN / DTC / SET / BUS** | Light 15 / 16 / 19 / 20 |
| **Source of truth** | Displayed page, not last button press |
| **Title** | `MNEM  ·  FULL TITLE` in white under chrome |

## Warnings (Lockheed freeze)

| Layer | Role |
|-------|------|
| **Master strip** (top of content) | On **every** format when any caution/warning is active |
| **Format-local fields** | BINGO title, PARK flash, tire cell, … on owning page |
| **Format-slot flash** | Only **warning-class** and only when owning format is on a slot **and not displayed** |
| **No slot flash** | DTC present, low batt (caution) — strip + DTC OSB only |

Owner map: BINGO→FUEL · PARK→DRV · TIRE→CHAS · DOOR→BODY · LOW BATT→ELEC (strip only).

## Anti-patterns (rejected)

- Reshuffle OSB after probe  
- Grey disabled labels  
- Hollow formats for missing equipment  
- Marketing-dense glass  
- Write vehicle bus  
- Permanent car-radio of every category on top/sides  
- Extra permanent face buttons for scroll / color / units  

# Bezel wiring (Board A)

Each line is one Dupont wire (or harness).
Headers are **right-angle** (horizontal), pin toward outer edge — not taller than the 4.3 mm switch.

| Button | Pin on board | Connect to MCU |
|--------|--------------|----------------|
| OSB 1 | **J1** | any free GPIO (see firmware map) |
| OSB 2 | **J2** | any free GPIO (see firmware map) |
| OSB 3 | **J3** | any free GPIO (see firmware map) |
| OSB 4 | **J4** | any free GPIO (see firmware map) |
| OSB 5 | **J5** | any free GPIO (see firmware map) |
| OSB 6 | **J6** | any free GPIO (see firmware map) |
| OSB 7 | **J7** | any free GPIO (see firmware map) |
| OSB 8 | **J8** | any free GPIO (see firmware map) |
| OSB 9 | **J9** | any free GPIO (see firmware map) |
| OSB 10 | **J10** | any free GPIO (see firmware map) |
| OSB 15 | **J15** | any free GPIO (see firmware map) |
| OSB 14 | **J14** | any free GPIO (see firmware map) |
| OSB 13 | **J13** | any free GPIO (see firmware map) |
| OSB 12 | **J12** | any free GPIO (see firmware map) |
| OSB 11 | **J11** | any free GPIO (see firmware map) |
| OSB 20 | **J20** | any free GPIO (see firmware map) |
| OSB 19 | **J19** | any free GPIO (see firmware map) |
| OSB 18 | **J18** | any free GPIO (see firmware map) |
| OSB 17 | **J17** | any free GPIO (see firmware map) |
| OSB 16 | **J16** | any free GPIO (see firmware map) |
| GAIN | **J_GAIN** | GPIO |
| SYM | **J_SYM** | GPIO |
| BRT | **J_BRT** | GPIO |
| CON | **J_CON** | GPIO |
| GND | **J_GND** | MCU GND |

## How to use (no KiCad)

1. Order the board from the gerber zip (see ORDER-THIS.md).
2. Solder the 6×6 switches and pin headers (or order SMT+THT assembly).
3. Plug Dupont wires: each `J#` → one MCU GPIO, `J_GND` → ground.
4. Firmware: pin low = button pressed (internal pull-up on MCU).

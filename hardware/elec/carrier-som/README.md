# Board B — SoM carrier

**Fab package:** [`../fab/board-b-carrier/`](../fab/board-b-carrier/)  
**Size:** 120 × 90 mm, 2-layer  
**SoM:** RK3566-class mezzanine (2 GB RAM target)  
**Role:** Linux host, panel FPC, USB-C charge/data, Eth, CAN, UART, audio, battery, RF

## Port map (see also docs/hardware/cmfd-connector-icd.md)

All external connectors sit on the **front board edge** (`y ≈ 0`) so they line up with the rear-shell port windows.  
M12 bulkheads are **panel-mount on the case wall** and wire to `J_M12`.

| Ref | Board XY (mm) | Function |
|-----|---------------|----------|
| J3 | 16, 6.5 | USB-C primary — PD sink + data + flash |
| J4 | 34, 6.5 | USB-C aux |
| J5 | 56, 12 | Ethernet RJ45 |
| J_M12 | 81, 5 | Harness to 3× M12 panel bulkheads (power / CAN / sensor) |
| J6 | 96, 5 | CAN-H/L + UART + GND |
| J7 | 108, 5 | Audio out / mic |
| J_BEZEL | 40, 78 | OSB 1–20 from Board A |
| J_RK | 95, 78 | Rocker lines |
| J10 / J11 | 30,45 / 30,62 | SoM mezzanine |

## Power

USB-C → BQ25895-class charger → 1S/2S 18650 → system buck → SoM 5V/3.3V.  
Optional DC jack pads for vehicle adapter (TVS + fuse).

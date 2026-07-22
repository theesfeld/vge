# Board A pin map (matrix)

| MCU pin | Net | Function |
|--------:|-----|----------|
| 10 | ROW_TOP | OSB 1–5 |
| 11 | ROW_RIGHT | OSB 6–10 |
| 12 | ROW_BOT | OSB 15–11 L→R |
| 13 | ROW_LEFT | OSB 20–16 T→B |
| 14–18 | COL0–4 | column index on each side |
| 19–22 | RK_* | GAIN SYM BRT CON |
| 29–30 | UART_TX/RX | J1.5 / J1.6 |
| 34 / 37 | SWDIO / SWCLK | J2.1 / J2.2 |

Routing: matrix on **B.Cu** buses; vias at each switch; MCU fanout on bottom **F.Cu**.
Firmware: scan COL drive / ROW sense (or reverse). Debounce ≥ 20 ms.

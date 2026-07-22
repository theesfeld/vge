# CMFD warnings (visual + aural)

**Hardware:** CMFD box includes a **speaker**.  
**Policy:** display-only bus; warnings are **local** glass + speaker.

## Visual

| Behavior | When |
|----------|------|
| **Master strip** | Any active caution/warning — top of content; red field pulses |
| **Red flash field** | Discrete items (e.g. **PARK** brake) — full cell red + white text at ~3.5 Hz |
| **BINGO label** | Fuel page title becomes BINGO when fuel ≤ 15% |

## Aural (speaker)

| Callout | Trigger | Pattern |
|---------|---------|---------|
| **BINGO** | Fuel ≤ `BINGO_FUEL` (0.15) | Double falling tones (repeat ~8 s) |
| **ALERT** | Park brake while moving, tire alert, door ajar moving | Paired urgent tones (~4 s) |
| **Caution** | Low battery, DTC present | Soft chirp (~12 s) |

Play path: synthesize PCM → `aplay` (ALSA). Hardware can replace with I2S.

| Env | Meaning |
|-----|---------|
| `MFD_AUDIO=0` | Mute |
| `MFD_APLAY=aplay` | Player binary |

## Code

| Path | Role |
|------|------|
| `mfd::warn` | Evaluate + flash timing + `WarningEngine` |
| `mfd::audio` | Callout tones |
| `mfd::widget::alert` | Flash cells / master strip |

## Demo

Without OBD, demo briefly forces park-brake-while-moving and low fuel so flash + aural can be checked. Use headphones/speaker; install `aplay` (alsa-utils) on the host.

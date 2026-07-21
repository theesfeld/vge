# VGE — project facts only

Global process: `~/.config/agents/AGENTS.md` (wins on conflict).

## Commands

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
VGE_FORCE_C=1 cargo test   # portable C path
```

## Layout

- `asm/x86_64/vge.s` — hot path (plot/line/circle/clear)
- `include/vge.h` — C ABI
- `c/vge_portable.c` — transforms + fallback raster
- `src/lib.rs` — Rust API

## Product rule

Geometry → individual pixels. Do not turn the library into a bitmap/sprite blitter.

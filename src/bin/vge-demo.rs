//! **Demo only** — loads pure-asm **libvge** and draws instrument vectors.
//!
//! Layout (first real use):
//! - Large **2019 Ford F-150** style tachometer (0–7000 RPM)
//! - Tape gauges: fuel, coolant temp, transmission temp, battery
//!
//! Prefer **1px** strokes. Needle tip uses library lifespan for a short trail.
//!
//! ```text
//! make
//! cargo run --release --bin vge-demo
//! VGE_TTL=14 cargo run --release --bin vge-demo   # tip trail length (frames)
//! ```
//!
//! Quit: `q` / Esc / Ctrl+C.

use std::f32::consts::PI;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vge::frame::FramePacer;
use vge::stroke::DisplayList;
use vge::term::{
    detect_backend, enter_overlay, leave_overlay, present_at_state, surface_size_for_viewport,
    terminal_cells, OverlayState, Viewport,
};
use vge::{
    engine_version, using_assembly, Color, Surface, AMBER, CYAN, GREEN, GREEN_DIM, RED, WHITE,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

// --- 2019 F-150 style tach (cluster scale, not a dyno sheet) ---
/// Face full scale (×1000 markings 0…7).
const RPM_MAX: f32 = 7000.0;
/// Approximate red-zone start on 2018–2020 F-150 clusters (~5.5k).
/// Engine cut varies by powertrain (3.5 EcoBoost ~6k, 5.0 higher).
const REDLINE_RPM: f32 = 5500.0;
/// Arc sweep for 0 → RPM_MAX (270°, lower-left through top to lower-right).
const TACH_SWEEP: f32 = PI * 1.5;
/// Angle at 0 RPM (screen: y down). Lower-left.
const TACH_ANG0: f32 = PI * 0.75;

fn main() -> io::Result<()> {
    let ver = engine_version();
    if !using_assembly() {
        eprintln!("error: demo requires pure-asm libvge (x86_64)");
        std::process::exit(1);
    }
    eprintln!("loaded libvge {ver} (assembly) · 2019 F-150 tach + tapes");

    install_sigint();
    let hz = std::env::var("VGE_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120u32);
    // Tip trail frames (default on — needle must have a tip trail).
    let tip_ttl = std::env::var("VGE_TTL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(16u32)
        .max(1);

    let backend = detect_backend();
    let (tc, tr) = terminal_cells();
    let vp = Viewport {
        col: 0,
        row: 1,
        cols: tc.max(1),
        rows: tr.saturating_sub(2).max(1),
    };
    let (w, h) = surface_size_for_viewport(backend, vp);

    let mut scanout = Surface::new(w, h);
    let mut tip_trail = DisplayList::with_capacity(64);
    let mut ostate = OverlayState::new();
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };

    enter_overlay()?;
    {
        let mut out = io::stdout().lock();
        write!(
            out,
            "\x1b[1;1H\x1b[2K\x1b[32mlibvge\x1b[0m {ver} · F-150 tach 0–7k · redline {REDLINE_RPM:.0}+ · tip ttl={tip_ttl} · {backend:?} · q quit"
        )?;
        out.flush()?;
    }

    let t0 = Instant::now();
    let mut last_status = Instant::now();
    let mut n = 0u32;
    let mut beam_sum = Duration::ZERO;
    let mut present_sum = Duration::ZERO;

    while RUNNING.load(Ordering::Relaxed) {
        if poll_quit()? {
            break;
        }
        let t = t0.elapsed().as_secs_f32();

        // Demo motion only (no OBD).
        let rpm = 800.0 + 4200.0 * (0.5 + 0.5 * (t * 0.55).sin());
        let fuel = 0.62 + 0.08 * (t * 0.12).cos();
        let coolant = 0.45 + 0.12 * (t * 0.18).sin(); // ~normal band mid-scale
        let trans = 0.40 + 0.15 * (t * 0.22).cos();
        let battery = 0.55 + 0.08 * (t * 0.3).sin(); // ~13–14 V band on 10–16 scale

        let tb = Instant::now();
        scanout.clear_transparent();

        // Layout: large tach left/center; four 1px tapes stacked on the right.
        let m = (w.min(h) as f32 * 0.04).max(8.0) as i32;
        let tape_w = ((w as f32) * 0.16).max(36.0) as i32;
        let gap = m / 2;
        let tach_right = w as i32 - m - tape_w - gap;
        let tach_cx = m + (tach_right - m) / 2;
        let tach_cy = h as i32 / 2;
        let tach_r = ((tach_right - m).min(h as i32 - 2 * m) / 2 - 4).max(40);

        draw_f150_tach(&mut scanout, tach_cx, tach_cy, tach_r, rpm);

        // 1px needle + tip trail (library lifespan).
        let tip = needle_tip(tach_cx, tach_cy, tach_r, rpm);
        draw_needle_1px(&mut scanout, tach_cx, tach_cy, tach_r, rpm);
        tip_trail.tick();
        tip_trail.set_lifespan(tip_ttl);
        tip_trail.set_color(RED);
        // Short cross at tip so the trail is visible as 1px marks.
        tip_trail.line(tip.0 - 1, tip.1, tip.0 + 1, tip.1);
        tip_trail.line(tip.0, tip.1 - 1, tip.0, tip.1 + 1);
        tip_trail.stroke_life(&mut scanout, true);

        // Four vertical tapes on the right.
        let tape_x = w as i32 - m - tape_w;
        let tape_top = m;
        let tape_h_total = h as i32 - 2 * m;
        let n_tapes = 4i32;
        let tape_gap = gap.max(4);
        let tape_h = (tape_h_total - tape_gap * (n_tapes - 1)) / n_tapes;
        let tapes: [(f32, Color, &str); 4] = [
            (fuel.clamp(0.0, 1.0), AMBER, "FUEL"),
            (coolant.clamp(0.0, 1.0), CYAN, "COOL"),
            (trans.clamp(0.0, 1.0), GREEN, "TRNS"),
            (battery.clamp(0.0, 1.0), WHITE, "BATT"),
        ];
        for (i, &(val, color, _name)) in tapes.iter().enumerate() {
            let y = tape_top + i as i32 * (tape_h + tape_gap);
            draw_tape_1px(&mut scanout, tape_x, y, tape_w, tape_h, val, color);
        }

        beam_sum += tb.elapsed();

        let tp = Instant::now();
        present_at_state(&scanout, backend, vp, Some(&mut ostate))?;
        present_sum += tp.elapsed();

        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }

        n += 1;
        if last_status.elapsed() >= Duration::from_millis(250) {
            let d = (beam_sum / n.max(1)).as_micros();
            let p = (present_sum / n.max(1)).as_micros();
            let fps = pacer.as_ref().map(|x| x.fps).unwrap_or(0.0);
            let mut out = io::stdout().lock();
            write!(
                out,
                "\x1b[1;1H\x1b[2K\x1b[32mlibvge\x1b[0m {ver} · F-150 · {rpm:.0} rpm · beam={d}µs present={p}µs fps={fps:.0} · q quit"
            )?;
            out.flush()?;
            n = 0;
            beam_sum = Duration::ZERO;
            present_sum = Duration::ZERO;
            last_status = Instant::now();
        }
    }

    leave_overlay()?;
    eprintln!("demo done · libvge {ver}");
    Ok(())
}

#[inline]
fn rpm_to_angle(rpm: f32) -> f32 {
    let v = (rpm / RPM_MAX).clamp(0.0, 1.0);
    TACH_ANG0 + v * TACH_SWEEP
}

fn needle_tip(cx: i32, cy: i32, r: i32, rpm: f32) -> (i32, i32) {
    let a = rpm_to_angle(rpm);
    let len = r as f32 * 0.88;
    (cx + (len * a.cos()) as i32, cy + (len * a.sin()) as i32)
}

/// Static face: 1px outline, ticks, redline **arc** inside the radius.
fn draw_f150_tach(s: &mut Surface, cx: i32, cy: i32, r: i32, _rpm: f32) {
    let r = r.max(24);
    // 1px bezel (outline only).
    s.circle(cx, cy, r, GREEN_DIM);

    // Major ticks every 1000 RPM (0…7).
    for k in 0..=7 {
        let rpm = k as f32 * 1000.0;
        let a = rpm_to_angle(rpm);
        let (c, sn) = (a.cos(), a.sin());
        let outer = r as f32 * 0.98;
        // Longer ticks at 0 and every 2k.
        let inner = r as f32 * if k % 2 == 0 { 0.86 } else { 0.90 };
        s.line_aa(
            cx + (outer * c) as i32,
            cy + (outer * sn) as i32,
            cx + (inner * c) as i32,
            cy + (inner * sn) as i32,
            if rpm >= REDLINE_RPM { RED } else { GREEN_DIM },
        );
    }

    // Minor ticks every 500 between majors.
    for k in 0..14 {
        if k % 2 == 0 {
            continue;
        }
        let rpm = k as f32 * 500.0;
        let a = rpm_to_angle(rpm);
        let (c, sn) = (a.cos(), a.sin());
        let outer = r as f32 * 0.98;
        let inner = r as f32 * 0.93;
        s.line_aa(
            cx + (outer * c) as i32,
            cy + (outer * sn) as i32,
            cx + (inner * c) as i32,
            cy + (inner * sn) as i32,
            if rpm >= REDLINE_RPM { RED } else { GREEN_DIM },
        );
    }

    // Redline arc: 1px chain inside the gauge radius (not on the bezel).
    draw_redline_arc(s, cx, cy, (r as f32 * 0.94) as i32, REDLINE_RPM, RPM_MAX);

    // Small 1px hub ring (static).
    s.circle(cx, cy, (r / 28).max(2), GREEN_DIM);
}

/// Redline as a 1px arc of short AA segments from `rpm0` to `rpm1`.
fn draw_redline_arc(s: &mut Surface, cx: i32, cy: i32, r_arc: i32, rpm0: f32, rpm1: f32) {
    let r = r_arc.max(8) as f32;
    let a0 = rpm_to_angle(rpm0);
    let a1 = rpm_to_angle(rpm1);
    // Chord length ~2px → segment count.
    let arc_len = (a1 - a0).abs() * r;
    let segs = ((arc_len / 2.0).ceil() as i32).clamp(8, 256);
    let mut prev = (cx + (r * a0.cos()) as i32, cy + (r * a0.sin()) as i32);
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let a = a0 + (a1 - a0) * t;
        let cur = (cx + (r * a.cos()) as i32, cy + (r * a.sin()) as i32);
        s.line_aa(prev.0, prev.1, cur.0, cur.1, RED);
        prev = cur;
    }
}

/// 1px needle from hub to tip (AA hairline).
fn draw_needle_1px(s: &mut Surface, cx: i32, cy: i32, r: i32, rpm: f32) {
    let a = rpm_to_angle(rpm);
    let (c, sn) = (a.cos(), a.sin());
    let tip = r as f32 * 0.88;
    let tail = r as f32 * 0.12;
    let x0 = cx + (-tail * c) as i32;
    let y0 = cy + (-tail * sn) as i32;
    let x1 = cx + (tip * c) as i32;
    let y1 = cy + (tip * sn) as i32;
    s.line_aa(x0, y0, x1, y1, GREEN);
    // 1px hub dot.
    s.plot(cx, cy, WHITE);
}

/// Vertical 1px tape: frame, ticks, value bar as a single AA line + index.
fn draw_tape_1px(s: &mut Surface, x: i32, y: i32, w: i32, h: i32, value01: f32, color: Color) {
    let v = value01.clamp(0.0, 1.0);
    let x1 = x + w;
    let y1 = y + h;
    // 1px frame.
    s.polyline(&[(x, y), (x1, y), (x1, y1), (x, y1), (x, y)], GREEN_DIM);

    let mid_x = x + w / 2;
    // Scale ticks (1px).
    let n = 9;
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let yy = y1 - ((h as f32) * t) as i32;
        let half = if i % 4 == 0 { w / 4 } else { w / 8 };
        s.line_aa(mid_x - half, yy, mid_x + half, yy, GREEN_DIM);
    }

    // Value as 1px vertical bar (not a filled rect).
    let fill_h = ((h as f32) * v) as i32;
    if fill_h > 0 {
        s.line_aa(mid_x, y1, mid_x, y1 - fill_h, color);
    }

    // Index: 1px horizontal hairline + short chevron polyline.
    let iy = y1 - fill_h;
    let arm = (w / 3).max(4);
    s.line_aa(mid_x - arm, iy, mid_x + arm, iy, color);
    s.polyline(
        &[
            (mid_x - arm, iy),
            (mid_x - arm / 2, iy - 2),
            (mid_x - arm / 2, iy + 2),
            (mid_x - arm, iy),
        ],
        color,
    );
}

fn poll_quit() -> io::Result<bool> {
    #[cfg(unix)]
    unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 0 {
            return Ok(false);
        }
        let mut fds = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        if libc::poll(&mut fds as *mut _, 1, 0) > 0 && (fds.revents & libc::POLLIN) != 0 {
            let mut buf = [0u8; 16];
            let r = libc::read(libc::STDIN_FILENO, buf.as_mut_ptr() as *mut _, buf.len());
            if r > 0 {
                for &b in &buf[..r as usize] {
                    if b == b'q' || b == b'Q' || b == 0x1b {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}

fn install_sigint() {
    #[cfg(unix)]
    unsafe {
        extern "C" fn on_sigint(_: libc::c_int) {
            RUNNING.store(false, Ordering::Relaxed);
        }
        #[allow(unknown_lints, function_casts_as_integer)]
        let handler = on_sigint as *const () as libc::sighandler_t;
        libc::signal(libc::SIGINT, handler);
    }
}

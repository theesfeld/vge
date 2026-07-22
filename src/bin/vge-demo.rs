//! **MFD instrument panel** — pure-asm libvge + bitmap text in the terminal.
//!
//! Full alternate screen. Black glass. High-contrast strokes and legends.
//! This is an instrument face, not a transparent wireframe overlay.
//!
//! ```text
//! make
//! cargo run --release --bin vge-demo
//! VGE_HZ=60 cargo run --release --bin vge-demo
//! ```
//!
//! Quit: `q` / Esc / Ctrl+C.

use std::f32::consts::PI;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vge::font::{draw_text, draw_text_centered, text_height, text_width};
use vge::frame::FramePacer;
use vge::term::{
    detect_backend, enter_fullscreen, leave_fullscreen, present_at, surface_size_for_viewport,
    terminal_cells, Viewport,
};
use vge::{engine_version, using_assembly, Color, Surface, AMBER, BLACK, CYAN, GREEN, RED, WHITE};

static RUNNING: AtomicBool = AtomicBool::new(true);

// High-contrast MFD ink (full alpha).
const INK: Color = GREEN;
const INK_DIM: Color = vge::GREEN_DIM;
const INK_CYAN: Color = CYAN;
const INK_AMBER: Color = AMBER;
const INK_RED: Color = RED;
const INK_WHITE: Color = WHITE;

const RPM_MAX: f32 = 7000.0;
const REDLINE_RPM: f32 = 5500.0;
const TACH_SWEEP: f32 = PI * 1.5;
const TACH_ANG0: f32 = PI * 0.75;

fn main() -> io::Result<()> {
    let ver = engine_version();
    if !using_assembly() {
        eprintln!("error: demo requires pure-asm libvge (x86_64)");
        std::process::exit(1);
    }
    eprintln!("loaded libvge {ver} · MFD panel (black glass, bitmap text)");

    install_sigint();
    let hz = std::env::var("VGE_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60u32);

    let backend = detect_backend();
    let (tc, tr) = terminal_cells();
    // Full terminal face — no chrome row outside the panel.
    let vp = Viewport {
        col: 0,
        row: 0,
        cols: tc.max(1),
        rows: tr.max(1),
    };
    let (w, h) = surface_size_for_viewport(backend, vp);

    let mut panel = Surface::new(w, h);
    let mut pacer = if hz == 0 {
        None
    } else {
        Some(FramePacer::new(hz))
    };

    enter_fullscreen()?;

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

        let rpm = 800.0 + 4200.0 * (0.5 + 0.5 * (t * 0.55).sin());
        let fuel = (0.62 + 0.08 * (t * 0.12).cos()).clamp(0.0, 1.0);
        let coolant = (0.45 + 0.12 * (t * 0.18).sin()).clamp(0.0, 1.0);
        let trans = (0.40 + 0.15 * (t * 0.22).cos()).clamp(0.0, 1.0);
        let battery = (0.55 + 0.08 * (t * 0.3).sin()).clamp(0.0, 1.0);

        let tb = Instant::now();
        // Black glass — always.
        panel.clear(BLACK);
        draw_mfd(
            &mut panel, w as i32, h as i32, rpm, fuel, coolant, trans, battery,
        );
        beam_sum += tb.elapsed();

        let tp = Instant::now();
        present_at(&panel, backend, vp)?;
        present_sum += tp.elapsed();

        if let Some(p) = pacer.as_mut() {
            p.wait_next();
        }

        n += 1;
        if last_status.elapsed() >= Duration::from_millis(500) {
            let d = (beam_sum / n.max(1)).as_micros();
            let p = (present_sum / n.max(1)).as_micros();
            let fps = pacer.as_ref().map(|x| x.fps).unwrap_or(0.0);
            // Status is drawn on-panel; also log occasionally to stderr if needed.
            let _ = (d, p, fps, ver);
            n = 0;
            beam_sum = Duration::ZERO;
            present_sum = Duration::ZERO;
            last_status = Instant::now();
        }
    }

    leave_fullscreen()?;
    eprintln!("demo done · libvge {ver}");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_mfd(
    s: &mut Surface,
    w: i32,
    h: i32,
    rpm: f32,
    fuel: f32,
    cool: f32,
    trans: f32,
    batt: f32,
) {
    let m = (w.min(h) as f32 * 0.03).max(6.0) as i32;
    let scale = if w.min(h) >= 500 { 2 } else { 1 };
    let th = text_height(scale);
    let top_bar = th + m * 2;
    let bot_bar = th + m * 2;

    // Outer bezel (1px).
    s.line_fast(m / 2, m / 2, w - m / 2, m / 2, INK_DIM);
    s.line_fast(w - m / 2, m / 2, w - m / 2, h - m / 2, INK_DIM);
    s.line_fast(w - m / 2, h - m / 2, m / 2, h - m / 2, INK_DIM);
    s.line_fast(m / 2, h - m / 2, m / 2, m / 2, INK_DIM);

    // Softkey row (MFD chrome).
    let keys = ["NORM", "RWS", "CRM", "CNTL", "OVRD"];
    let slot = (w - 2 * m) / keys.len() as i32;
    for (i, k) in keys.iter().enumerate() {
        let cx = m + slot * i as i32 + slot / 2;
        draw_text_centered(s, cx, m + th / 2, k, INK, scale);
    }
    // Divider under softkeys.
    s.line_fast(m, top_bar - 2, w - m, top_bar - 2, INK_DIM);

    // Layout: tach left, tapes right.
    let tape_w = ((w as f32) * 0.22).max(50.0) as i32;
    let gap = m;
    let face_top = top_bar + m / 2;
    let face_bot = h - bot_bar;
    let face_h = (face_bot - face_top).max(40);
    let tach_right = w - m - tape_w - gap;
    let tach_cx = m + (tach_right - m) / 2;
    let tach_cy = face_top + face_h / 2;
    let tach_r = ((tach_right - m).min(face_h) / 2 - 8).max(48);

    draw_tach(s, tach_cx, tach_cy, tach_r, rpm, scale);

    // Four tapes.
    let n_tapes = 4i32;
    let tape_gap = (gap / 2).max(4);
    let tape_h = (face_h - tape_gap * (n_tapes - 1)) / n_tapes;
    let tape_x = w - m - tape_w;
    let tapes: [(&str, f32, Color); 4] = [
        ("FUEL", fuel, INK_AMBER),
        ("COOL", cool, INK_CYAN),
        ("TRNS", trans, INK),
        ("BATT", batt, INK_WHITE),
    ];
    for (i, &(name, val, col)) in tapes.iter().enumerate() {
        let y = face_top + i as i32 * (tape_h + tape_gap);
        draw_tape(s, tape_x, y, tape_w, tape_h, name, val, col, scale);
    }

    // Bottom status strip.
    s.line_fast(m, face_bot + 2, w - m, face_bot + 2, INK_DIM);
    let rpm_s = format!("RPM {}", rpm.round() as i32);
    let st = format!("F150 TACH 0-7K  RL {}", REDLINE_RPM as i32);
    draw_text(s, m + 2, h - bot_bar + m / 2, &rpm_s, INK_WHITE, scale);
    let st_x = w - m - 2 - text_width(&st, scale);
    draw_text(s, st_x, h - bot_bar + m / 2, &st, INK_DIM, scale);
}

fn rpm_to_angle(rpm: f32) -> f32 {
    let v = (rpm / RPM_MAX).clamp(0.0, 1.0);
    TACH_ANG0 + v * TACH_SWEEP
}

fn draw_tach(s: &mut Surface, cx: i32, cy: i32, r: i32, rpm: f32, scale: i32) {
    let r = r.max(32);

    // Outer arc track (not a full ring floating — still circle for structure,
    // plus bold tick field). Hard pixels for bezel.
    s.circle(cx, cy, r, INK_DIM);
    s.circle(cx, cy, r - 1, INK_DIM);

    // Major ticks + numerals every 1000 RPM.
    for k in 0..=7 {
        let rv = k as f32 * 1000.0;
        let a = rpm_to_angle(rv);
        let (c, sn) = (a.cos(), a.sin());
        let outer = r as f32 - 2.0;
        let inner = r as f32 * if k % 2 == 0 { 0.82 } else { 0.88 };
        let col = if rv >= REDLINE_RPM { INK_RED } else { INK };
        // Crisp ticks: aliased line on black.
        s.line_fast(
            cx + (outer * c) as i32,
            cy + (outer * sn) as i32,
            cx + (inner * c) as i32,
            cy + (inner * sn) as i32,
            col,
        );
        // Numerals just inside ticks.
        let lx = cx + ((r as f32 * 0.68) * c) as i32;
        let ly = cy + ((r as f32 * 0.68) * sn) as i32;
        let label = format!("{k}");
        draw_text_centered(s, lx, ly, &label, col, scale);
    }

    // Minor ticks every 500.
    for k in 0..14 {
        if k % 2 == 0 {
            continue;
        }
        let rv = k as f32 * 500.0;
        let a = rpm_to_angle(rv);
        let (c, sn) = (a.cos(), a.sin());
        let outer = r as f32 - 2.0;
        let inner = r as f32 * 0.92;
        let col = if rv >= REDLINE_RPM { INK_RED } else { INK_DIM };
        s.line_fast(
            cx + (outer * c) as i32,
            cy + (outer * sn) as i32,
            cx + (inner * c) as i32,
            cy + (inner * sn) as i32,
            col,
        );
    }

    // Redline arc inside radius (bright, solid chords).
    draw_arc(
        s,
        cx,
        cy,
        (r as f32 * 0.94) as i32,
        REDLINE_RPM,
        RPM_MAX,
        INK_RED,
    );

    // Title.
    draw_text_centered(s, cx, cy + r / 3, "X1000", INK_DIM, scale);
    draw_text_centered(
        s,
        cx,
        cy + r / 3 + text_height(scale) + 2,
        "RPM",
        INK_DIM,
        scale,
    );

    // Needle: bright AA hairline + short tail; solid hub.
    let a = rpm_to_angle(rpm);
    let (c, sn) = (a.cos(), a.sin());
    let tip = r as f32 * 0.86;
    let tail = r as f32 * 0.14;
    let x0 = cx + (-tail * c) as i32;
    let y0 = cy + (-tail * sn) as i32;
    let x1 = cx + (tip * c) as i32;
    let y1 = cy + (tip * sn) as i32;
    // Double pass: aliased core + AA edge for both weight and crispness.
    s.line_fast(x0, y0, x1, y1, INK_WHITE);
    s.line_aa(x0, y0, x1, y1, INK);
    s.circle(cx, cy, 3, INK_WHITE);
    s.circle(cx, cy, 1, BLACK);
    s.plot(cx, cy, INK_WHITE);
}

fn draw_arc(s: &mut Surface, cx: i32, cy: i32, r_arc: i32, rpm0: f32, rpm1: f32, color: Color) {
    let r = r_arc.max(8) as f32;
    let a0 = rpm_to_angle(rpm0);
    let a1 = rpm_to_angle(rpm1);
    let arc_len = (a1 - a0).abs() * r;
    let segs = ((arc_len / 1.5).ceil() as i32).clamp(16, 512);
    let mut prev = (cx + (r * a0.cos()) as i32, cy + (r * a0.sin()) as i32);
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let a = a0 + (a1 - a0) * t;
        let cur = (cx + (r * a.cos()) as i32, cy + (r * a.sin()) as i32);
        s.line_fast(prev.0, prev.1, cur.0, cur.1, color);
        prev = cur;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_tape(
    s: &mut Surface,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    name: &str,
    value01: f32,
    color: Color,
    scale: i32,
) {
    let v = value01.clamp(0.0, 1.0);
    let x1 = x + w;
    let y1 = y + h;
    let th = text_height(scale);

    // Label above rail.
    draw_text(s, x + 2, y + 1, name, color, scale);

    let rail_top = y + th + 3;
    let rail_bot = y1 - 2;
    let rail_h = (rail_bot - rail_top).max(8);
    let mid = x + w / 2;

    // Frame.
    s.line_fast(x, rail_top, x1, rail_top, INK_DIM);
    s.line_fast(x1, rail_top, x1, rail_bot, INK_DIM);
    s.line_fast(x1, rail_bot, x, rail_bot, INK_DIM);
    s.line_fast(x, rail_bot, x, rail_top, INK_DIM);

    // Scale ticks.
    let n = 11;
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let yy = rail_bot - ((rail_h as f32) * t) as i32;
        let half = if i % 5 == 0 { w / 5 } else { w / 10 };
        s.line_fast(mid - half, yy, mid + half, yy, INK_DIM);
    }

    // Value bar: bright 1px vertical + index.
    let fill_h = ((rail_h as f32) * v) as i32;
    if fill_h > 0 {
        s.line_fast(mid, rail_bot, mid, rail_bot - fill_h, color);
        // Second column for slight weight (still 1–2 px total).
        s.line_fast(mid + 1, rail_bot, mid + 1, rail_bot - fill_h, color);
    }
    let iy = rail_bot - fill_h;
    let arm = (w / 3).max(6);
    s.line_fast(mid - arm, iy, mid + arm, iy, color);
    // Index bug.
    s.line_fast(mid - arm, iy, mid - arm + 3, iy - 3, color);
    s.line_fast(mid - arm, iy, mid - arm + 3, iy + 3, color);

    // Numeric percent.
    let pct = format!("{}", (v * 100.0).round() as i32);
    draw_text_centered(s, mid, rail_top + th / 2 + 1, &pct, INK_WHITE, scale);
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

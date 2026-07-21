//! Live terminal demo: true vector geometry (assembly engine) → pixels → terminal.
//!
//! ```text
//! cargo run --release --bin vge-demo
//! VGE_TERM=half cargo run --release --bin vge-demo
//! VGE_TERM=ascii cargo run --release --bin vge-demo
//! ```
//!
//! Quit: `q` / Esc / Ctrl+C.

use std::f32::consts::PI;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use vge::term::{
    detect_backend, enter_fullscreen, leave_fullscreen, present, suggested_surface_size,
    TermBackend,
};
use vge::{Surface, Xform, AMBER, BLACK, CYAN, GREEN, GREEN_DIM, RED, WHITE};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> io::Result<()> {
    install_sigint();

    let backend = detect_backend();
    let (w, h) = suggested_surface_size(backend);
    let mut surf = Surface::new(w, h);

    enter_fullscreen()?;
    let start = Instant::now();
    let mut frame: u64 = 0;
    let mut last_fps = Instant::now();
    let mut fps_count = 0u32;
    let mut fps = 0u32;

    let result = (|| -> io::Result<()> {
        while RUNNING.load(Ordering::Relaxed) {
            if poll_quit()? {
                break;
            }

            let t = start.elapsed().as_secs_f32();
            draw_scene(&mut surf, t, backend, fps);
            present(&surf, backend)?;

            frame += 1;
            fps_count += 1;
            if last_fps.elapsed() >= Duration::from_secs(1) {
                fps = fps_count;
                fps_count = 0;
                last_fps = Instant::now();
            }

            // Engine is instant; sleep only paces the eyes (~60 Hz).
            thread::sleep(Duration::from_millis(16));
        }
        Ok(())
    })();

    leave_fullscreen()?;
    let _ = writeln!(
        io::stderr(),
        "vge-demo done · backend={backend:?} · surface={w}x{h} · asm={} · frames={frame}",
        vge::using_assembly()
    );
    result
}

fn draw_scene(surf: &mut Surface, t: f32, backend: TermBackend, fps: u32) {
    let w = surf.width() as i32;
    let h = surf.height() as i32;
    let cx = w / 2;
    let cy = h / 2;
    surf.clear(BLACK);

    // FOV brackets
    let m = (w.min(h) / 40).max(8);
    let bracket = m * 2;
    surf.line_thick(m, m, m + bracket, m, GREEN_DIM, 2);
    surf.line_thick(m, m, m, m + bracket, GREEN_DIM, 2);
    surf.line_thick(w - m, m, w - m - bracket, m, GREEN_DIM, 2);
    surf.line_thick(w - m, m, w - m, m + bracket, GREEN_DIM, 2);
    surf.line_thick(m, h - m, m + bracket, h - m, GREEN_DIM, 2);
    surf.line_thick(m, h - m, m, h - m - bracket, GREEN_DIM, 2);
    surf.line_thick(w - m, h - m, w - m - bracket, h - m, GREEN_DIM, 2);
    surf.line_thick(w - m, h - m, w - m, h - m - bracket, GREEN_DIM, 2);

    // Rotating star (transform → Bresenham pixels)
    let arm = (w.min(h) as f32) * 0.28;
    let rot = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate(t * 1.2)
        .translate(-cx as f32, -cy as f32);
    for i in 0..6 {
        let a = i as f32 * PI / 3.0;
        let x1 = cx as f32 + arm * a.cos();
        let y1 = cy as f32 + arm * a.sin();
        surf.line_xf(&rot, cx as f32, cy as f32, x1, y1, GREEN);
    }

    // Orbiting circle
    let orbit_r = arm * 0.85;
    let ox = cx as f32 + orbit_r * (t * 2.0).cos();
    let oy = cy as f32 + orbit_r * (t * 2.0).sin();
    surf.circle(ox as i32, oy as i32, (arm * 0.12).max(4.0) as i32, CYAN);

    // Pitch ladder (synthetic roll)
    let roll = (t * 0.4).sin() * 12.0;
    let ladder = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate_deg(roll)
        .translate(-cx as f32, -cy as f32);
    for step in -3..=3 {
        if step == 0 {
            continue;
        }
        let y = cy as f32 + step as f32 * (h as f32 * 0.06);
        let half = w as f32 * 0.12;
        let gap = w as f32 * 0.04;
        let col = if step > 0 { GREEN } else { GREEN_DIM };
        surf.line_xf(&ladder, cx as f32 - half, y, cx as f32 - gap, y, col);
        surf.line_xf(&ladder, cx as f32 + gap, y, cx as f32 + half, y, col);
    }
    surf.line_xf(
        &ladder,
        cx as f32 - w as f32 * 0.2,
        cy as f32,
        cx as f32 + w as f32 * 0.2,
        cy as f32,
        AMBER,
    );

    // Gun cross
    let g = (w.min(h) / 25).max(6);
    surf.line(cx - g * 2, cy, cx - g / 2, cy, GREEN);
    surf.line(cx + g / 2, cy, cx + g * 2, cy, GREEN);
    surf.line(cx, cy - g, cx, cy - g / 3, GREEN);

    // Radar PPI
    let rcx = w - w / 5;
    let rcy = h - h / 5;
    let rr = (w.min(h) / 8).max(12);
    for ring in 1..=3 {
        surf.circle(rcx, rcy, rr * ring / 3, GREEN_DIM);
    }
    let sweep = t * 2.5;
    surf.line_thick(
        rcx,
        rcy,
        rcx + (rr as f32 * sweep.cos()) as i32,
        rcy + (rr as f32 * sweep.sin()) as i32,
        GREEN,
        2,
    );

    // Spinning square
    let sq = (w.min(h) as f32) * 0.08;
    let bx = w as f32 * 0.18;
    let by = h as f32 * 0.22;
    let box_xf = Xform::identity()
        .translate(bx, by)
        .rotate(t * -1.8)
        .translate(-bx, -by);
    let corners = [
        (bx - sq, by - sq),
        (bx + sq, by - sq),
        (bx + sq, by + sq),
        (bx - sq, by + sq),
        (bx - sq, by - sq),
    ];
    for win in corners.windows(2) {
        surf.line_xf(&box_xf, win[0].0, win[0].1, win[1].0, win[1].1, RED);
    }

    // Status bars: fps width + backend code
    let label_y = 4;
    surf.rect_fill(4, label_y, 4 + (fps.min(120) as i32), label_y + 3, WHITE);
    let be = match backend {
        TermBackend::Kitty => 3,
        TermBackend::HalfBlock => 2,
        TermBackend::Ascii => 1,
    };
    surf.rect_fill(4, label_y + 6, 4 + be * 8, label_y + 9, CYAN);
}

fn poll_quit() -> io::Result<bool> {
    #[cfg(unix)]
    {
        unsafe {
            let mut fds = libc::pollfd {
                fd: libc::STDIN_FILENO,
                events: libc::POLLIN,
                revents: 0,
            };
            let n = libc::poll(&mut fds as *mut libc::pollfd, 1, 0);
            if n > 0 && (fds.revents & libc::POLLIN) != 0 {
                let mut buf = [0u8; 16];
                let r = libc::read(
                    libc::STDIN_FILENO,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                );
                if r > 0 {
                    for &b in &buf[..r as usize] {
                        if b == b'q' || b == b'Q' || b == 0x1b {
                            return Ok(true);
                        }
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
        extern "C" fn on_sigint(_: i32) {
            RUNNING.store(false, Ordering::Relaxed);
        }
        libc::signal(libc::SIGINT, on_sigint as *const () as usize);
    }
}

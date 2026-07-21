//! Live vector demo — fast RAM raster + single present per frame.
//!
//! ```text
//! cargo run --release --bin vge-demo              # any terminal
//! cargo run --release --bin vge-demo -- --fb      # Linux video RAM
//! VGE_HZ=120 cargo run --release --bin vge-demo  # target frame rate
//! VGE_PHOSPHOR=1 cargo run --release --bin vge-demo  # CRT-style trail
//! ```
//!
//! Quit: `q` / Esc / Ctrl+C.

use std::f32::consts::PI;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use vge::frame::FramePacer;
use vge::term::{
    detect_backend, enter_fullscreen, leave_fullscreen, present, suggested_surface_size,
    TermBackend,
};
use vge::{Color, Surface, Xform, AMBER, BLACK, CYAN, GREEN, GREEN_DIM, RED, WHITE};

static RUNNING: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy, Debug)]
enum PresentMode {
    Framebuffer,
    Terminal(TermBackend),
}

fn main() -> io::Result<()> {
    install_sigint();
    let hz = std::env::var("VGE_HZ")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60u32);
    let phosphor =
        std::env::var_os("VGE_PHOSPHOR").is_some() || std::env::args().any(|a| a == "--phosphor");
    let decay = std::env::var("VGE_DECAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(230u32); // factor_256; only used if phosphor

    match select_mode() {
        PresentMode::Framebuffer => run_fb(hz, phosphor, decay),
        PresentMode::Terminal(backend) => run_term(backend, hz, phosphor, decay),
    }
}

fn select_mode() -> PresentMode {
    let args: Vec<String> = std::env::args().collect();
    let flag_fb = args.iter().any(|a| a == "--fb" || a == "-f");
    let flag_term = args.iter().any(|a| a == "--term" || a == "-t");
    let env = std::env::var("VGE_PRESENT")
        .unwrap_or_default()
        .to_ascii_lowercase();

    if flag_term {
        return PresentMode::Terminal(detect_backend());
    }
    if flag_fb || env == "fb" || env == "framebuffer" || env == "tty" {
        return PresentMode::Framebuffer;
    }
    if is_linux_vt() && fb_available() {
        return PresentMode::Framebuffer;
    }
    PresentMode::Terminal(detect_backend())
}

fn fb_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        vge::fb::Framebuffer::available()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn is_linux_vt() -> bool {
    let term = std::env::var("TERM").unwrap_or_default();
    if term == "linux" {
        return true;
    }
    #[cfg(unix)]
    {
        unsafe {
            let mut buf = [0i8; 64];
            if libc::ttyname_r(libc::STDIN_FILENO, buf.as_mut_ptr(), buf.len()) == 0 {
                let name = std::ffi::CStr::from_ptr(buf.as_ptr());
                if let Ok(s) = name.to_str() {
                    if let Some(rest) = s.strip_prefix("/dev/tty") {
                        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Draw in system RAM, blit once to the frame buffer. This is the fast path:
/// uncached FB writes per pixel are slow; one bulk present is smooth.
#[cfg(target_os = "linux")]
fn run_fb(hz: u32, phosphor: bool, decay: u32) -> io::Result<()> {
    use vge::fb::Framebuffer;

    let mut fb = Framebuffer::open_default().map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "open framebuffer failed: {e}\n\
                 Use a real VT (Ctrl+Alt+F3) or drop --fb for terminal mode."
            ),
        )
    })?;

    // Match FB size so present is a tight copy.
    let mut back = Surface::new(fb.width(), fb.height());
    let mut pacer = FramePacer::new(hz);

    eprintln!(
        "VGE fast path · FB {}x{} · RAM back-buffer · target {hz} Hz · asm={} · phosphor={}",
        fb.width(),
        fb.height(),
        vge::using_assembly(),
        phosphor
    );
    eprintln!("Quit: q / Ctrl+C");

    let mut t: f32 = 0.0;
    let mut frame: u64 = 0;
    let dt = 1.0 / hz as f32;

    while RUNNING.load(Ordering::Relaxed) {
        if poll_quit()? {
            break;
        }
        pacer.begin();

        if phosphor {
            back.decay(decay);
        } else {
            back.clear(BLACK);
        }
        draw_scene(&mut back, t, pacer.fps as u32, 3);
        fb.present_from(&back);

        pacer.end();
        t += dt;
        frame += 1;
        if frame % (hz as u64) == 0 {
            eprint!("\r  {:.0} FPS   ", pacer.fps);
            let _ = io::stderr().flush();
        }
    }
    eprintln!();
    let _ = writeln!(
        io::stderr(),
        "vge-demo FB done · frames={frame} · last_fps={:.1}",
        pacer.fps
    );
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn run_fb(_hz: u32, _phosphor: bool, _decay: u32) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "framebuffer present is Linux-only",
    ))
}

fn run_term(backend: TermBackend, hz: u32, phosphor: bool, decay: u32) -> io::Result<()> {
    let (w, h) = suggested_surface_size(backend);
    let mut back = Surface::new(w, h);
    let mut pacer = FramePacer::new(hz);

    eprintln!(
        "VGE terminal · {backend:?} · {w}x{h} · target {hz} Hz · asm={} · phosphor={}",
        vge::using_assembly(),
        phosphor
    );
    eprintln!("Quit: q / Esc / Ctrl+C");

    enter_fullscreen()?;
    let mut t: f32 = 0.0;
    let mut frame: u64 = 0;
    let dt = 1.0 / hz as f32;
    let be = match backend {
        TermBackend::Kitty => 3,
        TermBackend::HalfBlock => 2,
        TermBackend::Ascii => 1,
    };

    let result = (|| -> io::Result<()> {
        while RUNNING.load(Ordering::Relaxed) {
            if poll_quit()? {
                break;
            }
            pacer.begin();
            if phosphor {
                back.decay(decay);
            } else {
                back.clear(BLACK);
            }
            draw_scene(&mut back, t, pacer.fps as u32, be);
            present(&back, backend)?;
            pacer.end();
            t += dt;
            frame += 1;
        }
        Ok(())
    })();

    leave_fullscreen()?;
    let _ = writeln!(
        io::stderr(),
        "vge-demo term done · frames={frame} · last_fps={:.1}",
        pacer.fps
    );
    result
}

fn draw_scene(c: &mut Surface, t: f32, fps: u32, backend_code: i32) {
    let w = c.width() as i32;
    let h = c.height() as i32;
    let cx = w / 2;
    let cy = h / 2;

    let m = (w.min(h) / 40).max(8);
    let bracket = m * 2;
    let th = if w > 1000 { 3 } else { 2 };
    c.line_thick(m, m, m + bracket, m, GREEN_DIM, th);
    c.line_thick(m, m, m, m + bracket, GREEN_DIM, th);
    c.line_thick(w - m, m, w - m - bracket, m, GREEN_DIM, th);
    c.line_thick(w - m, m, w - m, m + bracket, GREEN_DIM, th);
    c.line_thick(m, h - m, m + bracket, h - m, GREEN_DIM, th);
    c.line_thick(m, h - m, m, h - m - bracket, GREEN_DIM, th);
    c.line_thick(w - m, h - m, w - m - bracket, h - m, GREEN_DIM, th);
    c.line_thick(w - m, h - m, w - m, h - m - bracket, GREEN_DIM, th);

    let arm = (w.min(h) as f32) * 0.28;
    let rot = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate(t * 1.2)
        .translate(-(cx as f32), -(cy as f32));
    for i in 0..6 {
        let a = i as f32 * PI / 3.0;
        c.line_xf(
            &rot,
            cx as f32,
            cy as f32,
            cx as f32 + arm * a.cos(),
            cy as f32 + arm * a.sin(),
            GREEN,
        );
    }

    let orbit_r = arm * 0.85;
    c.circle(
        (cx as f32 + orbit_r * (t * 2.0).cos()) as i32,
        (cy as f32 + orbit_r * (t * 2.0).sin()) as i32,
        (arm * 0.12).max(4.0) as i32,
        CYAN,
    );

    let roll = (t * 0.4).sin() * 12.0;
    let ladder = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate_deg(roll)
        .translate(-(cx as f32), -(cy as f32));
    for step in -3..=3 {
        if step == 0 {
            continue;
        }
        let y = cy as f32 + step as f32 * (h as f32 * 0.06);
        let half = w as f32 * 0.12;
        let gap = w as f32 * 0.04;
        let col = if step > 0 { GREEN } else { GREEN_DIM };
        c.line_xf(&ladder, cx as f32 - half, y, cx as f32 - gap, y, col);
        c.line_xf(&ladder, cx as f32 + gap, y, cx as f32 + half, y, col);
    }
    c.line_xf(
        &ladder,
        cx as f32 - w as f32 * 0.2,
        cy as f32,
        cx as f32 + w as f32 * 0.2,
        cy as f32,
        AMBER,
    );

    let g = (w.min(h) / 25).max(6);
    c.line(cx - g * 2, cy, cx - g / 2, cy, GREEN);
    c.line(cx + g / 2, cy, cx + g * 2, cy, GREEN);
    c.line(cx, cy - g, cx, cy - g / 3, GREEN);

    let rcx = w - w / 5;
    let rcy = h - h / 5;
    let rr = (w.min(h) / 8).max(12);
    for ring in 1..=3 {
        c.circle(rcx, rcy, rr * ring / 3, GREEN_DIM);
    }
    let sweep = t * 2.5;
    c.line_thick(
        rcx,
        rcy,
        rcx + (rr as f32 * sweep.cos()) as i32,
        rcy + (rr as f32 * sweep.sin()) as i32,
        GREEN,
        th,
    );

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
        c.line_xf(&box_xf, win[0].0, win[0].1, win[1].0, win[1].1, RED);
    }

    c.rect_fill(4, 4, 4 + (fps.min(200) as i32), 7, WHITE);
    c.rect_fill(4, 10, 4 + backend_code * 8, 13, CYAN);
    let _ = Color::default;
}

fn poll_quit() -> io::Result<bool> {
    #[cfg(unix)]
    {
        unsafe {
            if libc::isatty(libc::STDIN_FILENO) == 0 {
                return Ok(false);
            }
            let mut fds = libc::pollfd {
                fd: libc::STDIN_FILENO,
                events: libc::POLLIN,
                revents: 0,
            };
            if libc::poll(&mut fds as *mut libc::pollfd, 1, 0) > 0
                && (fds.revents & libc::POLLIN) != 0
            {
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
        extern "C" fn on_sigint(_: libc::c_int) {
            RUNNING.store(false, Ordering::Relaxed);
        }
        #[allow(unknown_lints, function_casts_as_integer)]
        let handler = on_sigint as *const () as libc::sighandler_t;
        libc::signal(libc::SIGINT, handler);
    }
}

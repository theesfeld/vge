//! Micro-benchmark: clear + scene + (optional) blit.
//!
//! ```bash
//! cargo run --release --example bench
//! cargo run --release --example bench -- --fb
//! ```

use std::env;
use std::f32::consts::PI;
use std::time::Instant;
use vge::{Surface, Xform, BLACK, CYAN, GREEN, GREEN_DIM};

fn main() {
    let fb_mode = env::args().any(|a| a == "--fb");
    let frames: u32 = env::var("VGE_BENCH_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);

    let (w, h) = if fb_mode {
        #[cfg(target_os = "linux")]
        {
            match vge::fb::Framebuffer::open_default() {
                Ok(fb) => (fb.width(), fb.height()),
                Err(_) => (1280, 720),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            (1280, 720)
        }
    } else {
        (1280, 720)
    };

    let mut back = Surface::new(w, h);
    println!(
        "bench · {}x{} · frames={frames} · asm={}",
        w,
        h,
        vge::using_assembly()
    );

    // Warmup
    for i in 0..10 {
        scene(&mut back, i as f32 * 0.05);
    }

    let t0 = Instant::now();
    for i in 0..frames {
        back.clear(BLACK);
        scene(&mut back, i as f32 * 0.05);
    }
    let draw = t0.elapsed();

    #[cfg(target_os = "linux")]
    let blit_time = if fb_mode {
        if let Ok(mut fb) = vge::fb::Framebuffer::open_default() {
            let t1 = Instant::now();
            for i in 0..frames {
                back.clear(BLACK);
                scene(&mut back, i as f32 * 0.05);
                fb.present_from(&back);
            }
            Some(t1.elapsed())
        } else {
            None
        }
    } else {
        None
    };
    #[cfg(not(target_os = "linux"))]
    let blit_time: Option<std::time::Duration> = None;

    let fps_draw = frames as f64 / draw.as_secs_f64();
    println!(
        "draw-only  total={draw:?}  per_frame={:?}  fps={fps_draw:.1}",
        draw / frames
    );
    if let Some(bt) = blit_time {
        let fps = frames as f64 / bt.as_secs_f64();
        println!(
            "draw+blit  total={bt:?}  per_frame={:?}  fps={fps:.1}",
            bt / frames
        );
    }
}

fn scene(s: &mut Surface, t: f32) {
    let w = s.width() as i32;
    let h = s.height() as i32;
    let cx = w / 2;
    let cy = h / 2;
    let arm = w.min(h) as f32 * 0.3;
    let rot = Xform::identity()
        .translate(cx as f32, cy as f32)
        .rotate(t)
        .translate(-(cx as f32), -(cy as f32));
    for i in 0..16 {
        let a = i as f32 * PI / 8.0;
        s.line_xf(
            &rot,
            cx as f32,
            cy as f32,
            cx as f32 + arm * a.cos(),
            cy as f32 + arm * a.sin(),
            GREEN,
        );
    }
    s.circle(cx, cy, (arm * 0.5) as i32, CYAN);
    s.circle(cx, cy, (arm * 0.8) as i32, GREEN_DIM);
    for k in 0..8 {
        let y = h / 10 + k * h / 12;
        s.line(w / 8, y, w - w / 8, y, GREEN_DIM);
    }
}

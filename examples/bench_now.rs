use std::time::Instant;
use vge::fb::Framebuffer;
use vge::{Xform, BLACK, GREEN};
fn main() {
    let mut fb = Framebuffer::open_default().unwrap();
    let n = 60;
    let t0 = Instant::now();
    for i in 0..n {
        fb.clear(BLACK);
        let t = i as f32 * 0.05;
        let w = fb.width() as i32;
        let h = fb.height() as i32;
        let cx = w / 2;
        let cy = h / 2;
        let m = Xform::identity()
            .translate(cx as f32, cy as f32)
            .rotate(t)
            .translate(-(cx as f32), -(cy as f32));
        for k in 0..12 {
            let a = k as f32 * std::f32::consts::TAU / 12.0;
            fb.line_xf(
                &m,
                cx as f32,
                cy as f32,
                cx as f32 + 400.0 * a.cos(),
                cy as f32 + 400.0 * a.sin(),
                GREEN,
            );
        }
        fb.circle(cx, cy, 200, GREEN);
    }
    let dt = t0.elapsed();
    println!(
        "frames={n} total={:?} fps={:.1} per_frame={:?}",
        n,
        n as f64 / dt.as_secs_f64(),
        dt / n
    );
}

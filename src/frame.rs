//! Frame pacing for smooth motion.
//!
//! Target a fixed frame period. Sleep only the remaining slack so the
//! draw path stays as short as the hardware allows.

use std::thread;
use std::time::{Duration, Instant};

/// Fixed-rate frame clock.
pub struct FramePacer {
    target: Duration,
    frame_start: Instant,
    last_report: Instant,
    frames: u32,
    /// Smoothed FPS for display.
    pub fps: f32,
}

impl FramePacer {
    /// Target frames per second (e.g. 60, 120, 144). Clamped to 1..=1000.
    pub fn new(target_hz: u32) -> Self {
        let hz = target_hz.clamp(1, 1000);
        let target = Duration::from_secs_f64(1.0 / hz as f64);
        let now = Instant::now();
        Self {
            target,
            frame_start: now,
            last_report: now,
            frames: 0,
            fps: hz as f32,
        }
    }

    /// Call at the start of each frame.
    pub fn begin(&mut self) {
        self.frame_start = Instant::now();
    }

    /// Call after draw+present. Sleeps only if the frame finished early.
    pub fn end(&mut self) {
        self.frames += 1;
        let elapsed = self.frame_start.elapsed();
        if elapsed < self.target {
            // Sleep almost all remaining time; tiny busy wait for last 200µs
            // keeps scheduling jitter down without spinning the whole period.
            let remain = self.target - elapsed;
            if remain > Duration::from_micros(200) {
                thread::sleep(remain - Duration::from_micros(200));
            }
            while self.frame_start.elapsed() < self.target {
                std::hint::spin_loop();
            }
        }
        if self.last_report.elapsed() >= Duration::from_secs(1) {
            let secs = self.last_report.elapsed().as_secs_f32().max(0.001);
            self.fps = self.frames as f32 / secs;
            self.frames = 0;
            self.last_report = Instant::now();
        }
    }

    pub fn target_hz(&self) -> f32 {
        1.0 / self.target.as_secs_f32()
    }
}

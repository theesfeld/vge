//! CMFD **speaker** path — short tone callouts (BINGO / ALERT / caution).
//!
//! Default: synthesize 16-bit mono PCM and play via `aplay` when present
//! (Linux ALSA). Hardware CMFD can replace this with I2S / codec DAC later.
//!
//! Env:
//! - `MFD_AUDIO=0` — silence
//! - `MFD_APLAY=aplay` — override player binary

use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;

/// Aural callout type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callout {
    /// Low fuel — distinctive double pattern (“bingo”)
    Bingo,
    /// Master-class alert (park brake, tire, door while moving)
    Alert,
    /// Soft caution chirp
    Caution,
}

/// Fire-and-forget callout (background thread so glass never stalls).
pub fn play(callout: Callout) {
    if std::env::var("MFD_AUDIO").as_deref() == Ok("0")
        || std::env::var("MFD_AUDIO").as_deref() == Ok("off")
    {
        return;
    }
    thread::Builder::new()
        .name("mfd-audio".into())
        .spawn(move || {
            let pcm = render(callout);
            if play_pcm(&pcm, 22_050).is_err() {
                // Silent fail — glass stays primary
            }
        })
        .ok();
}

fn render(callout: Callout) -> Vec<u8> {
    match callout {
        Callout::Bingo => {
            // Two falling tones + gap (readable as BINGO-class)
            let mut v = tone_burst(880.0, 0.18, 0.7);
            v.extend(silence(0.08));
            v.extend(tone_burst(660.0, 0.22, 0.7));
            v.extend(silence(0.12));
            v.extend(tone_burst(880.0, 0.12, 0.55));
            v.extend(tone_burst(550.0, 0.28, 0.75));
            v
        }
        Callout::Alert => {
            // “Alert alert” — two identical urgent pairs
            let mut v = Vec::new();
            for _ in 0..2 {
                v.extend(tone_burst(1200.0, 0.12, 0.85));
                v.extend(silence(0.06));
                v.extend(tone_burst(1200.0, 0.12, 0.85));
                v.extend(silence(0.18));
            }
            v
        }
        Callout::Caution => {
            let mut v = tone_burst(740.0, 0.09, 0.45);
            v.extend(silence(0.05));
            v.extend(tone_burst(740.0, 0.09, 0.35));
            v
        }
    }
}

fn silence(secs: f32) -> Vec<u8> {
    let n = (22_050.0 * secs) as usize;
    vec![0u8; n * 2]
}

fn tone_burst(freq_hz: f32, secs: f32, amp: f32) -> Vec<u8> {
    let rate = 22_050.0_f32;
    let n = (rate * secs) as usize;
    let mut out = Vec::with_capacity(n * 2);
    let amp = amp.clamp(0.0, 1.0);
    for i in 0..n {
        // Simple envelope to avoid clicks
        let env = {
            let a = (i as f32 / (rate * 0.01)).min(1.0);
            let r = ((n - i) as f32 / (rate * 0.02)).min(1.0);
            a.min(r)
        };
        let t = i as f32 / rate;
        let s = (t * freq_hz * std::f32::consts::TAU).sin() * amp * env;
        let sample = (s * 16000.0) as i16;
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

fn play_pcm(pcm: &[u8], rate: u32) -> Result<(), ()> {
    let bin = std::env::var("MFD_APLAY").unwrap_or_else(|_| "aplay".into());
    let mut child = Command::new(&bin)
        .args([
            "-q",
            "-t",
            "raw",
            "-f",
            "S16_LE",
            "-c",
            "1",
            "-r",
            &rate.to_string(),
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(pcm);
    }
    let _ = child.wait();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bingo_not_empty() {
        assert!(!render(Callout::Bingo).is_empty());
        assert!(!render(Callout::Alert).is_empty());
        assert!(!render(Callout::Caution).is_empty());
    }
}

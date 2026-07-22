//! Fixed **bitmap font** for instrument faces (MFD-style labels and digits).
//!
//! Glyphs are solid pixels (no AA). On a black face this reads crisp, like
//! panel legends on glass.
//!
//! Cell size: 5×7 pixels per glyph at `scale = 1`, plus 1px inter-char gap.

use crate::{Color, Surface};

/// Glyph width in pixels at scale 1.
pub const GLYPH_W: i32 = 5;
/// Glyph height in pixels at scale 1.
pub const GLYPH_H: i32 = 7;
/// Horizontal gap between glyphs at scale 1.
pub const GLYPH_GAP: i32 = 1;

/// Pixel width of `text` at the given scale (including gaps).
pub fn text_width(text: &str, scale: i32) -> i32 {
    let s = scale.max(1);
    let n = text.chars().count() as i32;
    if n == 0 {
        return 0;
    }
    n * (GLYPH_W * s) + (n - 1) * (GLYPH_GAP * s)
}

/// Pixel height of a line at the given scale.
pub fn text_height(scale: i32) -> i32 {
    GLYPH_H * scale.max(1)
}

/// Draw a single line of text. Unknown glyphs are skipped (space advances).
pub fn draw_text(surface: &mut Surface, x: i32, y: i32, text: &str, color: Color, scale: i32) {
    let s = scale.max(1);
    let mut cx = x;
    for ch in text.chars() {
        if ch == ' ' {
            cx += (GLYPH_W + GLYPH_GAP) * s;
            continue;
        }
        if let Some(rows) = glyph(ch) {
            blit_glyph(surface, cx, y, rows, color, s);
        }
        cx += (GLYPH_W + GLYPH_GAP) * s;
    }
}

/// Center text on `(cx, cy)` (center of the text box).
pub fn draw_text_centered(
    surface: &mut Surface,
    cx: i32,
    cy: i32,
    text: &str,
    color: Color,
    scale: i32,
) {
    let s = scale.max(1);
    let w = text_width(text, s);
    let h = text_height(s);
    draw_text(surface, cx - w / 2, cy - h / 2, text, color, s);
}

fn blit_glyph(surface: &mut Surface, x: i32, y: i32, rows: [u8; 7], color: Color, scale: i32) {
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..5 {
            // MSB = left pixel.
            if (bits >> (4 - col)) & 1 == 1 {
                let px = x + col * scale;
                let py = y + row as i32 * scale;
                if scale == 1 {
                    surface.plot(px, py, color);
                } else {
                    surface.rect_fill(px, py, px + scale - 1, py + scale - 1, color);
                }
            }
        }
    }
}

/// 5×7 bitmaps: each `u8` holds 5 bits (top bit unused). Row 0 is top.
fn glyph(ch: char) -> Option<[u8; 7]> {
    let c = if ch.is_ascii_lowercase() {
        ch.to_ascii_uppercase()
    } else {
        ch
    };
    Some(match c {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00001, 0b00001, 0b00001, 0b00001, 0b10001, 0b10001, 0b01110,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00100, 0b00100, 0b01000, 0b10000, 0b10000,
        ],
        '%' => [
            0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011,
        ],
        '#' => [
            0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010,
        ],
        '*' => [
            0b00000, 0b00100, 0b10101, 0b01110, 0b10101, 0b00100, 0b00000,
        ],
        '<' => [
            0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010,
        ],
        '>' => [
            0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000,
        ],
        '=' => [
            0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000,
        ],
        '\'' => [
            0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{alpha, GREEN};

    #[test]
    fn digit_lights_pixels() {
        let mut s = Surface::new(40, 20);
        s.clear(0);
        draw_text(&mut s, 1, 1, "RPM", GREEN, 1);
        // At least a few green pixels in the glyph area.
        let mut lit = 0u32;
        for y in 1..10 {
            for x in 1..30 {
                if s.get(x, y).map(alpha).unwrap_or(0) > 0 {
                    lit += 1;
                }
            }
        }
        assert!(lit > 20, "expected glyph pixels, got {lit}");
    }

    #[test]
    fn width_scales() {
        assert_eq!(text_width("A", 1), 5);
        assert_eq!(text_width("AB", 1), 5 + 1 + 5);
        assert_eq!(text_width("A", 2), 10);
    }
}

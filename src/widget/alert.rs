//! Flashing **warning field** (red background) for discrete alerts on glass.

use crate::color::{rgb, RED, WHITE};
use crate::font::{draw_text, draw_text_centered, text_width};
use crate::geom::Rect;
use crate::{Color, Surface};

/// Red field behind text when flash phase is on.
pub fn flash_label(
    s: &mut Surface,
    x: f32,
    y: f32,
    text: &str,
    font_px: f32,
    flash_on: bool,
    normal: Color,
) {
    if flash_on {
        let tw = text_width(text, font_px);
        let pad = 3.0;
        let x0 = (x - pad) as i32;
        let y0 = (y - pad) as i32;
        let x1 = (x + tw + pad) as i32;
        let y1 = (y + font_px + pad) as i32;
        s.rect_fill(x0, y0, x1, y1, RED);
        draw_text(s, x, y, text, WHITE, font_px);
    } else {
        draw_text(s, x, y, text, normal, font_px);
    }
}

/// Centered flash label.
pub fn flash_label_centered(
    s: &mut Surface,
    cx: f32,
    cy: f32,
    text: &str,
    font_px: f32,
    flash_on: bool,
    normal: Color,
) {
    let tw = text_width(text, font_px);
    let x = cx - tw * 0.5;
    let y = cy - font_px * 0.5;
    flash_label(s, x, y, text, font_px, flash_on, normal);
}

/// Status cell with optional red flash fill (park brake, etc.).
#[allow(clippy::too_many_arguments)]
pub fn status_cell_flash(
    s: &mut Surface,
    rect: Rect,
    label: &str,
    on: bool,
    flash_on: bool,
    on_color: Color,
    off_color: Color,
    font_px: f32,
) {
    let fh = font_px.clamp(8.0, 14.0);
    if flash_on && on {
        s.rect_fill(rect.x, rect.y, rect.right(), rect.bottom(), RED);
        // White border
        s.line_aa(rect.x, rect.y, rect.right(), rect.y, WHITE);
        s.line_aa(rect.right(), rect.y, rect.right(), rect.bottom(), WHITE);
        s.line_aa(rect.right(), rect.bottom(), rect.x, rect.bottom(), WHITE);
        s.line_aa(rect.x, rect.bottom(), rect.x, rect.y, WHITE);
        draw_text_centered(
            s,
            rect.center().0 as f32,
            rect.center().1 as f32,
            label,
            WHITE,
            fh,
        );
    } else {
        let c = if on { on_color } else { off_color };
        s.line_aa(rect.x, rect.y, rect.right(), rect.y, c);
        s.line_aa(rect.right(), rect.y, rect.right(), rect.bottom(), c);
        s.line_aa(rect.right(), rect.bottom(), rect.x, rect.bottom(), c);
        s.line_aa(rect.x, rect.bottom(), rect.x, rect.y, c);
        if on {
            s.line_aa(rect.x + 2, rect.y + 2, rect.x + 2, rect.bottom() - 2, c);
            s.line_aa(rect.x + 3, rect.y + 2, rect.x + 3, rect.bottom() - 2, c);
        }
        draw_text_centered(
            s,
            rect.center().0 as f32,
            rect.center().1 as f32,
            label,
            c,
            fh,
        );
    }
}

/// Master caution / warning strip across content top.
pub fn master_warn_strip(s: &mut Surface, rect: Rect, text: &str, flash_on: bool, font_px: f32) {
    let bg = if flash_on { RED } else { rgb(80, 20, 20) };
    s.rect_fill(rect.x, rect.y, rect.right(), rect.bottom(), bg);
    draw_text_centered(
        s,
        rect.center().0 as f32,
        rect.center().1 as f32,
        text,
        WHITE,
        font_px,
    );
}

/* SPDX-License-Identifier: MIT
 * Portable C implementation of VGE hot path.
 * Used when the build is not x86_64 Linux assembly, and always for
 * transform helpers + export + version.
 */

#include "vge.h"

#include <math.h>
#include <string.h>

const char *vge_version(void) { return "0.1.0-dev.1"; }

/* ---- always compiled: transforms + export + polyline ---- */

void vge_xform_identity(VgeXform *m) {
    if (!m)
        return;
    m->a = 1.f;
    m->b = 0.f;
    m->tx = 0.f;
    m->c = 0.f;
    m->d = 1.f;
    m->ty = 0.f;
}

/* Each op is post-multiply: M := M * Op (apply Op after current map). */

void vge_xform_translate(VgeXform *m, float tx, float ty) {
    if (!m)
        return;
    m->tx = m->a * tx + m->b * ty + m->tx;
    m->ty = m->c * tx + m->d * ty + m->ty;
}

void vge_xform_scale(VgeXform *m, float sx, float sy) {
    if (!m)
        return;
    m->a *= sx;
    m->c *= sx;
    m->b *= sy;
    m->d *= sy;
}

void vge_xform_rotate(VgeXform *m, float radians) {
    if (!m)
        return;
    float cs = cosf(radians);
    float sn = sinf(radians);
    float a = m->a, b = m->b, c = m->c, d = m->d;
    /* M = M * R, R = [cs -sn; sn cs] */
    m->a = a * cs + b * sn;
    m->b = -a * sn + b * cs;
    m->c = c * cs + d * sn;
    m->d = -c * sn + d * cs;
}

void vge_xform_apply(const VgeXform *m, float x, float y, float *ox, float *oy) {
    if (!m || !ox || !oy)
        return;
    *ox = m->a * x + m->b * y + m->tx;
    *oy = m->c * x + m->d * y + m->ty;
}

void vge_line_xf(VgeSurface *s, const VgeXform *m, float x0, float y0, float x1,
                 float y1, vge_color color) {
    float ax, ay, bx, by;
    if (!m) {
        vge_line(s, (int32_t)lroundf(x0), (int32_t)lroundf(y0), (int32_t)lroundf(x1),
                 (int32_t)lroundf(y1), color);
        return;
    }
    vge_xform_apply(m, x0, y0, &ax, &ay);
    vge_xform_apply(m, x1, y1, &bx, &by);
    vge_line(s, (int32_t)lroundf(ax), (int32_t)lroundf(ay), (int32_t)lroundf(bx),
             (int32_t)lroundf(by), color);
}

void vge_polyline(VgeSurface *s, const int32_t *xy, int32_t n, vge_color color) {
    int32_t i;
    if (!s || !xy || n < 2)
        return;
    for (i = 0; i < n - 1; i++) {
        vge_line(s, xy[i * 2], xy[i * 2 + 1], xy[(i + 1) * 2], xy[(i + 1) * 2 + 1],
                 color);
    }
}

void vge_export_rgb24(const VgeSurface *s, uint8_t *dest) {
    uint32_t x, y;
    if (!s || !s->pixels || !dest)
        return;
    for (y = 0; y < s->height; y++) {
        const uint8_t *row = s->pixels + (size_t)y * s->stride;
        for (x = 0; x < s->width; x++) {
            uint32_t p;
            memcpy(&p, row + (size_t)x * 4, 4);
            *dest++ = (uint8_t)((p >> 16) & 0xFF); /* R */
            *dest++ = (uint8_t)((p >> 8) & 0xFF);  /* G */
            *dest++ = (uint8_t)(p & 0xFF);         /* B */
        }
    }
}

/* ---- always in C: blit + phosphor decay + composed ops ---- */

void vge_blit(VgeSurface *dst, const VgeSurface *src) {
    uint32_t y, w, h, row_bytes;
    if (!dst || !src || !dst->pixels || !src->pixels)
        return;
    w = dst->width < src->width ? dst->width : src->width;
    h = dst->height < src->height ? dst->height : src->height;
    row_bytes = w * 4u;
    for (y = 0; y < h; y++) {
        memcpy(dst->pixels + (size_t)y * dst->stride,
               src->pixels + (size_t)y * src->stride, row_bytes);
    }
}

void vge_decay(VgeSurface *s, uint32_t factor_256) {
    uint32_t x, y;
    if (!s || !s->pixels)
        return;
    if (factor_256 > 256)
        factor_256 = 256;
    for (y = 0; y < s->height; y++) {
        uint8_t *row = s->pixels + (size_t)y * s->stride;
        for (x = 0; x < s->width; x++) {
            uint32_t p;
            uint32_t r, g, b;
            memcpy(&p, row + (size_t)x * 4, 4);
            r = ((p >> 16) & 0xFF) * factor_256 >> 8;
            g = ((p >> 8) & 0xFF) * factor_256 >> 8;
            b = (p & 0xFF) * factor_256 >> 8;
            p = (r << 16) | (g << 8) | b;
            memcpy(row + (size_t)x * 4, &p, 4);
        }
    }
}

/* ---- always in C: composed ops on top of plot/line ---- */

void vge_line_thick(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                    vge_color color, int32_t thickness) {
    int32_t t = thickness < 1 ? 1 : thickness;
    int32_t half = t / 2;
    int32_t o;
    if (t == 1) {
        vge_line(s, x0, y0, x1, y1, color);
        return;
    }
    for (o = -half; o <= half; o++) {
        vge_line(s, x0 + o, y0, x1 + o, y1, color);
        vge_line(s, x0, y0 + o, x1, y1 + o, color);
    }
}

void vge_rect_fill(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                   vge_color color) {
    int32_t x, y, t;
    if (!s)
        return;
    if (x0 > x1) {
        t = x0;
        x0 = x1;
        x1 = t;
    }
    if (y0 > y1) {
        t = y0;
        y0 = y1;
        y1 = t;
    }
    if (x0 < 0)
        x0 = 0;
    if (y0 < 0)
        y0 = 0;
    if (x1 >= (int32_t)s->width)
        x1 = (int32_t)s->width - 1;
    if (y1 >= (int32_t)s->height)
        y1 = (int32_t)s->height - 1;
    for (y = y0; y <= y1; y++)
        for (x = x0; x <= x1; x++)
            vge_plot(s, x, y, color);
}

/* ---- portable raster (when assembly is not linked) ---- */
#if !defined(VGE_USE_ASM)

void vge_plot(VgeSurface *s, int32_t x, int32_t y, vge_color color) {
    uint8_t *dst;
    if (!s || !s->pixels || x < 0 || y < 0)
        return;
    if ((uint32_t)x >= s->width || (uint32_t)y >= s->height)
        return;
    dst = s->pixels + (size_t)y * s->stride + (size_t)x * 4;
    memcpy(dst, &color, 4);
}

void vge_clear(VgeSurface *s, vge_color color) {
    uint32_t y, x;
    if (!s || !s->pixels)
        return;
    for (y = 0; y < s->height; y++) {
        uint8_t *row = s->pixels + (size_t)y * s->stride;
        for (x = 0; x < s->width; x++) {
            memcpy(row + (size_t)x * 4, &color, 4);
        }
    }
}

void vge_line(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
              vge_color color) {
    int32_t dx = (x1 > x0) ? (x1 - x0) : (x0 - x1);
    int32_t sx = x0 < x1 ? 1 : -1;
    int32_t dy = (y1 > y0) ? (y0 - y1) : (y1 - y0);
    int32_t sy = y0 < y1 ? 1 : -1;
    int32_t err = dx + dy;
    int32_t x = x0, y = y0;

    for (;;) {
        vge_plot(s, x, y, color);
        if (x == x1 && y == y1)
            break;
        {
            int32_t e2 = err * 2;
            if (e2 >= dy) {
                err += dy;
                x += sx;
            }
            if (e2 <= dx) {
                err += dx;
                y += sy;
            }
        }
    }
}

void vge_circle(VgeSurface *s, int32_t cx, int32_t cy, int32_t r, vge_color color) {
    int32_t x, y, err;
    if (r <= 0)
        return;
    x = r;
    y = 0;
    err = 1 - r;
    while (x >= y) {
        vge_plot(s, cx + x, cy + y, color);
        vge_plot(s, cx + y, cy + x, color);
        vge_plot(s, cx - y, cy + x, color);
        vge_plot(s, cx - x, cy + y, color);
        vge_plot(s, cx - x, cy - y, color);
        vge_plot(s, cx - y, cy - x, color);
        vge_plot(s, cx + y, cy - x, color);
        vge_plot(s, cx + x, cy - y, color);
        y++;
        if (err < 0) {
            err += 2 * y + 1;
        } else {
            x--;
            err += 2 * (y - x) + 1;
        }
    }
}

#endif /* !VGE_USE_ASM */

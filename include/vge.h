/* SPDX-License-Identifier: MIT
 * VGE — true vector graphics engine
 *
 * Geometry → individual pixels. No bitmap sprite path.
 * C ABI for C and Rust (and any language that can call C).
 */
#ifndef VGE_H
#define VGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Packed color: 0x00RRGGBB (top byte unused). */
typedef uint32_t vge_color;

/** Pixel surface: XRGB8888 little-endian words, row-major. */
typedef struct VgeSurface {
    uint32_t width;  /* pixels */
    uint32_t height; /* pixels */
    uint32_t stride; /* bytes per row; must be >= width * 4 */
    uint32_t _pad;
    uint8_t *pixels; /* length >= stride * height */
} VgeSurface;

/** 2D affine transform: [x'] = [a b tx] [x]
 *                       [y']   [c d ty] [y]
 *                                       [1] */
typedef struct VgeXform {
    float a, b, tx;
    float c, d, ty;
} VgeXform;

/* --- Surface / clear / plot --- */

/** Fill every pixel with color. */
void vge_clear(VgeSurface *s, vge_color color);

/** Light one pixel if (x,y) is inside the surface. */
void vge_plot(VgeSurface *s, int32_t x, int32_t y, vge_color color);

/* --- Integer screen-space vectors (hot path) --- */

/** Bresenham line: light every pixel from (x0,y0) to (x1,y1). */
void vge_line(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
              vge_color color);

/** Thick line (integer thickness, multi-pass). */
void vge_line_thick(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                    vge_color color, int32_t thickness);

/** Midpoint circle outline. */
void vge_circle(VgeSurface *s, int32_t cx, int32_t cy, int32_t r, vge_color color);

/** Filled axis-aligned rect [x0..x1]×[y0..y1] inclusive ends (clamped). */
void vge_rect_fill(VgeSurface *s, int32_t x0, int32_t y0, int32_t x1, int32_t y1,
                   vge_color color);

/* --- Transform helpers (portable C; apply then call line) --- */

void vge_xform_identity(VgeXform *m);
void vge_xform_translate(VgeXform *m, float tx, float ty);
void vge_xform_scale(VgeXform *m, float sx, float sy);
/** Rotate counter-clockwise by radians around origin, then current matrix. */
void vge_xform_rotate(VgeXform *m, float radians);
void vge_xform_apply(const VgeXform *m, float x, float y, float *ox, float *oy);

/** Transformed float endpoints → integer Bresenham on surface. */
void vge_line_xf(VgeSurface *s, const VgeXform *m, float x0, float y0, float x1,
                 float y1, vge_color color);

/** Polyline through n points (screen ints). n>=2. */
void vge_polyline(VgeSurface *s, const int32_t *xy, int32_t n, vge_color color);

/** Export RGB888 tightly packed (for display protocols). dest len = w*h*3. */
void vge_export_rgb24(const VgeSurface *s, uint8_t *dest);

/** Engine version string (static). */
const char *vge_version(void);

#ifdef __cplusplus
}
#endif

#endif /* VGE_H */

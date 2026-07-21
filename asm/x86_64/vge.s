/* SPDX-License-Identifier: MIT
 * VGE hot path — System V AMD64 ABI (GNU as, intel syntax)
 *
 * VgeSurface:
 *   0  width   u32
 *   4  height  u32
 *   8  stride  u32
 *  12  _pad    u32
 *  16  pixels  u64
 *
 * Pixel: 4-byte dword color 0x00RRGGBB.
 * Geometry lights individual pixels. No bitmap blit path.
 */
        .text
        .intel_syntax noprefix
        .file   "vge.s"

/*--------------------------------------------------------------------
 * vge_plot(VgeSurface *s, int32_t x, int32_t y, vge_color color)
 * rdi=s  esi=x  edx=y  ecx=color
 * Clobbers: rax, r8, r9 (caller-saved only)
 *------------------------------------------------------------------*/
        .globl  vge_plot
        .type   vge_plot, @function
vge_plot:
        test    rdi, rdi
        jz      .Lp_ret
        test    esi, esi
        js      .Lp_ret
        test    edx, edx
        js      .Lp_ret
        cmp     esi, dword ptr [rdi]
        jae     .Lp_ret
        cmp     edx, dword ptr [rdi + 4]
        jae     .Lp_ret
        mov     r8, qword ptr [rdi + 16]
        test    r8, r8
        jz      .Lp_ret
        mov     r9d, dword ptr [rdi + 8]
        mov     eax, edx
        imul    eax, r9d
        mov     edx, esi
        shl     edx, 2
        add     eax, edx
        mov     dword ptr [r8 + rax], ecx
.Lp_ret:
        ret
        .size   vge_plot, .-vge_plot

/*--------------------------------------------------------------------
 * vge_clear(VgeSurface *s, vge_color color)
 * rdi=s  esi=color
 *------------------------------------------------------------------*/
        .globl  vge_clear
        .type   vge_clear, @function
vge_clear:
        push    rbx
        push    r12
        push    r13
        push    r14
        test    rdi, rdi
        jz      .Lc_done
        mov     r12, qword ptr [rdi + 16]
        test    r12, r12
        jz      .Lc_done
        mov     r13d, dword ptr [rdi]          /* width */
        mov     r14d, dword ptr [rdi + 4]      /* height */
        mov     ebx, dword ptr [rdi + 8]       /* stride */
        test    r13d, r13d
        jz      .Lc_done
        test    r14d, r14d
        jz      .Lc_done
        mov     eax, esi                       /* color for stosd */
        xor     r8d, r8d                       /* y */
.Lc_y:
        cmp     r8d, r14d
        jae     .Lc_done
        mov     ecx, r8d
        imul    ecx, ebx
        mov     rdi, r12
        add     rdi, rcx
        mov     ecx, r13d
        rep     stosd
        inc     r8d
        jmp     .Lc_y
.Lc_done:
        pop     r14
        pop     r13
        pop     r12
        pop     rbx
        ret
        .size   vge_clear, .-vge_clear

/*--------------------------------------------------------------------
 * vge_line — Bresenham (callee-saved state across vge_plot)
 * rdi=s  esi=x0  edx=y0  ecx=x1  r8d=y1  r9d=color
 *
 * Stack frame (rbp-relative):
 *   -4   x
 *   -8   y
 *   -12  x1
 *   -16  y1
 *   -20  dx
 *   -24  dy
 *   -28  sx
 *   -32  sy
 *   -36  err
 *   -40  color
 *   r12  surface*
 *------------------------------------------------------------------*/
        .globl  vge_line
        .type   vge_line, @function
vge_line:
        push    rbp
        mov     rbp, rsp
        push    r12
        sub     rsp, 56

        test    rdi, rdi
        jz      .Ll_done
        mov     r12, rdi
        mov     dword ptr [rbp - 4], esi       /* x = x0 */
        mov     dword ptr [rbp - 8], edx       /* y = y0 */
        mov     dword ptr [rbp - 12], ecx      /* x1 */
        mov     dword ptr [rbp - 16], r8d      /* y1 */
        mov     dword ptr [rbp - 40], r9d      /* color */

        /* dx = abs(x1-x0), sx */
        mov     eax, dword ptr [rbp - 12]
        sub     eax, dword ptr [rbp - 4]
        mov     edx, 1
        jge     1f
        neg     eax
        mov     edx, -1
1:      mov     dword ptr [rbp - 20], eax      /* dx */
        mov     dword ptr [rbp - 28], edx      /* sx */

        /* dy = -abs(y1-y0), sy */
        mov     eax, dword ptr [rbp - 16]
        sub     eax, dword ptr [rbp - 8]
        mov     edx, 1
        jge     2f
        neg     eax
        mov     edx, -1
2:      neg     eax
        mov     dword ptr [rbp - 24], eax      /* dy */
        mov     dword ptr [rbp - 32], edx      /* sy */

        /* err = dx + dy */
        mov     eax, dword ptr [rbp - 20]
        add     eax, dword ptr [rbp - 24]
        mov     dword ptr [rbp - 36], eax

.Ll_loop:
        /* plot(s, x, y, color) */
        mov     rdi, r12
        mov     esi, dword ptr [rbp - 4]
        mov     edx, dword ptr [rbp - 8]
        mov     ecx, dword ptr [rbp - 40]
        call    vge_plot

        /* if x==x1 && y==y1 break */
        mov     eax, dword ptr [rbp - 4]
        cmp     eax, dword ptr [rbp - 12]
        jne     3f
        mov     eax, dword ptr [rbp - 8]
        cmp     eax, dword ptr [rbp - 16]
        je      .Ll_done
3:
        /* e2 = 2*err */
        mov     eax, dword ptr [rbp - 36]
        add     eax, eax
        mov     edx, eax                       /* e2 */

        /* if e2 >= dy: err += dy; x += sx */
        cmp     edx, dword ptr [rbp - 24]
        jl      4f
        mov     eax, dword ptr [rbp - 36]
        add     eax, dword ptr [rbp - 24]
        mov     dword ptr [rbp - 36], eax
        mov     eax, dword ptr [rbp - 4]
        add     eax, dword ptr [rbp - 28]
        mov     dword ptr [rbp - 4], eax
4:
        /* if e2 <= dx: err += dx; y += sy */
        cmp     edx, dword ptr [rbp - 20]
        jg      5f
        mov     eax, dword ptr [rbp - 36]
        add     eax, dword ptr [rbp - 20]
        mov     dword ptr [rbp - 36], eax
        mov     eax, dword ptr [rbp - 8]
        add     eax, dword ptr [rbp - 32]
        mov     dword ptr [rbp - 8], eax
5:
        jmp     .Ll_loop

.Ll_done:
        add     rsp, 56
        pop     r12
        pop     rbp
        ret
        .size   vge_line, .-vge_line

/*--------------------------------------------------------------------
 * vge_circle — midpoint, 8-way
 * rdi=s  esi=cx  edx=cy  ecx=r  r8d=color
 *------------------------------------------------------------------*/
        .globl  vge_circle
        .type   vge_circle, @function
vge_circle:
        push    rbp
        mov     rbp, rsp
        push    r12
        push    r13
        push    r14
        push    r15
        push    rbx
        sub     rsp, 24

        test    rdi, rdi
        jz      .Lci_done
        test    ecx, ecx
        jle     .Lci_done

        mov     r12, rdi
        mov     r13d, esi                      /* cx */
        mov     r14d, edx                      /* cy */
        mov     r15d, r8d                      /* color */
        mov     ebx, ecx                       /* x = r */
        xor     eax, eax
        mov     dword ptr [rbp - 48], eax      /* y = 0 */
        mov     eax, 1
        sub     eax, ecx
        mov     dword ptr [rbp - 52], eax      /* err = 1-r */

.Lci_loop:
        cmp     ebx, dword ptr [rbp - 48]
        jl      .Lci_done

        /* 8 plots */
        mov     rdi, r12
        mov     esi, r13d
        add     esi, ebx
        mov     edx, r14d
        add     edx, dword ptr [rbp - 48]
        mov     ecx, r15d
        call    vge_plot

        mov     rdi, r12
        mov     esi, r13d
        add     esi, dword ptr [rbp - 48]
        mov     edx, r14d
        add     edx, ebx
        mov     ecx, r15d
        call    vge_plot

        mov     rdi, r12
        mov     esi, r13d
        sub     esi, dword ptr [rbp - 48]
        mov     edx, r14d
        add     edx, ebx
        mov     ecx, r15d
        call    vge_plot

        mov     rdi, r12
        mov     esi, r13d
        sub     esi, ebx
        mov     edx, r14d
        add     edx, dword ptr [rbp - 48]
        mov     ecx, r15d
        call    vge_plot

        mov     rdi, r12
        mov     esi, r13d
        sub     esi, ebx
        mov     edx, r14d
        sub     edx, dword ptr [rbp - 48]
        mov     ecx, r15d
        call    vge_plot

        mov     rdi, r12
        mov     esi, r13d
        sub     esi, dword ptr [rbp - 48]
        mov     edx, r14d
        sub     edx, ebx
        mov     ecx, r15d
        call    vge_plot

        mov     rdi, r12
        mov     esi, r13d
        add     esi, dword ptr [rbp - 48]
        mov     edx, r14d
        sub     edx, ebx
        mov     ecx, r15d
        call    vge_plot

        mov     rdi, r12
        mov     esi, r13d
        add     esi, ebx
        mov     edx, r14d
        sub     edx, dword ptr [rbp - 48]
        mov     ecx, r15d
        call    vge_plot

        /* y++ */
        mov     eax, dword ptr [rbp - 48]
        inc     eax
        mov     dword ptr [rbp - 48], eax

        cmp     dword ptr [rbp - 52], 0
        jge     .Lci_epos
        /* err += 2y+1 */
        mov     eax, dword ptr [rbp - 48]
        add     eax, eax
        inc     eax
        add     dword ptr [rbp - 52], eax
        jmp     .Lci_loop
.Lci_epos:
        dec     ebx
        /* err += 2(y-x)+1 */
        mov     eax, dword ptr [rbp - 48]
        sub     eax, ebx
        add     eax, eax
        inc     eax
        add     dword ptr [rbp - 52], eax
        jmp     .Lci_loop

.Lci_done:
        add     rsp, 24
        pop     rbx
        pop     r15
        pop     r14
        pop     r13
        pop     r12
        pop     rbp
        ret
        .size   vge_circle, .-vge_circle

/* rect_fill + line_thick live in C (call asm plot/line). */

        .section .note.GNU-stack, "", @progbits

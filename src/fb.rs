//! Linux framebuffer present — **draw directly into video memory**.
//!
//! On a real TTY (virtual console), map `/dev/fb0` and point the assembly
//! engine at that mapping. `vge_plot` / `vge_line` store pixels into the
//! hardware frame buffer. No escape codes. No Kitty. No character cells.
//!
//! Requires read/write on the FB device (often group `video` / `nogroup`).
//! On exit, restore text mode (`KD_TEXT`) if we switched the console.
//!
//! Env: `VGE_FB=/dev/fb0` (default `/dev/fb0`).

use crate::{Color, VgeSurface, Xform};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

/// Live mapping of a Linux frame buffer device.
pub struct Framebuffer {
    _fd: OwnedFd,
    map: *mut u8,
    map_len: usize,
    width: u32,
    height: u32,
    stride: u32,
    /// We entered KD_GRAPHICS; must restore KD_TEXT.
    kd_graphics: bool,
    /// Console fd used for KDSETMODE, if any.
    tty_fd: Option<OwnedFd>,
    /// Use stdin for KD restore when tty_fd is None but kd_graphics is set.
    kd_on_stdin: bool,
    /// Saved pixels so the screen is not left black after exit.
    saved: Option<Vec<u8>>,
}

const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;
const FBIOGET_FSCREENINFO: libc::c_ulong = 0x4602;
const KDSETMODE: libc::c_ulong = 0x4B3A;
const KD_TEXT: libc::c_int = 0x00;
const KD_GRAPHICS: libc::c_int = 0x01;

#[repr(C)]
#[derive(Clone, Copy)]
struct FbBitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FbVarScreeninfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: FbBitfield,
    green: FbBitfield,
    blue: FbBitfield,
    transp: FbBitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FbFixScreeninfo {
    id: [u8; 16],
    smem_start: usize,
    smem_len: u32,
    type_: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: usize,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

impl Framebuffer {
    /// Open and map a frame buffer. Default path: `VGE_FB` or `/dev/fb0`.
    pub fn open_default() -> io::Result<Self> {
        let path = std::env::var_os("VGE_FB")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/dev/fb0"));
        Self::open(&path)
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let fd = unsafe {
            let raw = libc::open(c_path.as_ptr(), libc::O_RDWR);
            if raw < 0 {
                return Err(io::Error::last_os_error());
            }
            OwnedFd::from_raw_fd(raw)
        };

        let mut var: FbVarScreeninfo = unsafe { std::mem::zeroed() };
        let mut fix: FbFixScreeninfo = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(fd.as_raw_fd(), FBIOGET_VSCREENINFO as _, &mut var as *mut _) } < 0
        {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::ioctl(fd.as_raw_fd(), FBIOGET_FSCREENINFO as _, &mut fix as *mut _) } < 0
        {
            return Err(io::Error::last_os_error());
        }

        if var.bits_per_pixel != 32 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "framebuffer bpp={} (need 32 for direct dword stores)",
                    var.bits_per_pixel
                ),
            ));
        }

        // Engine color is 0x00RRGGBB (R@16 G@8 B@0).
        if var.red.offset != 16
            || var.green.offset != 8
            || var.blue.offset != 0
            || var.red.length != 8
            || var.green.length != 8
            || var.blue.length != 8
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "FB color layout R@{} G@{} B@{} (need R@16 G@8 B@0)",
                    var.red.offset, var.green.offset, var.blue.offset
                ),
            ));
        }

        let map_len = fix.smem_len as usize;
        if map_len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "framebuffer smem_len is 0",
            ));
        }

        let map = unsafe {
            libc::mmap(
                ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd.as_raw_fd(),
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        let width = var.xres;
        let height = var.yres;
        let stride = fix.line_length;
        let usable = (stride as usize)
            .saturating_mul(height as usize)
            .min(map_len);

        let saved = {
            let mut v = vec![0u8; usable];
            unsafe {
                ptr::copy_nonoverlapping(map as *const u8, v.as_mut_ptr(), v.len());
            }
            Some(v)
        };

        let (tty_fd, kd_graphics, kd_on_stdin) = try_enter_graphics_mode();

        Ok(Self {
            _fd: fd,
            map: map as *mut u8,
            map_len,
            width,
            height,
            stride,
            kd_graphics,
            tty_fd,
            kd_on_stdin,
            saved,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn stride(&self) -> u32 {
        self.stride
    }

    /// True if the default FB device can be opened for RW.
    pub fn available() -> bool {
        let path = std::env::var_os("VGE_FB")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/dev/fb0"));
        let Ok(c) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        unsafe {
            let fd = libc::open(c.as_ptr(), libc::O_RDWR);
            if fd < 0 {
                return false;
            }
            libc::close(fd);
            true
        }
    }

    /// FFI surface pointing **at video memory**. Assembly draws here.
    pub fn surface(&mut self) -> VgeSurface {
        VgeSurface {
            width: self.width,
            height: self.height,
            stride: self.stride,
            _pad: 0,
            pixels: self.map,
        }
    }

    pub fn clear(&mut self, color: Color) {
        let mut s = self.surface();
        unsafe { crate::vge_clear(&mut s, color) };
    }

    pub fn plot(&mut self, x: i32, y: i32, color: Color) {
        let mut s = self.surface();
        unsafe { crate::vge_plot(&mut s, x, y, color) };
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut s = self.surface();
        unsafe { crate::vge_line(&mut s, x0, y0, x1, y1, color) };
    }

    pub fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color, t: i32) {
        let mut s = self.surface();
        unsafe { crate::vge_line_thick(&mut s, x0, y0, x1, y1, color, t) };
    }

    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, color: Color) {
        let mut s = self.surface();
        unsafe { crate::vge_circle(&mut s, cx, cy, r, color) };
    }

    pub fn rect_fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut s = self.surface();
        unsafe { crate::vge_rect_fill(&mut s, x0, y0, x1, y1, color) };
    }

    pub fn line_xf(&mut self, m: &Xform, x0: f32, y0: f32, x1: f32, y1: f32, color: Color) {
        let mut s = self.surface();
        unsafe { crate::vge_line_xf(&mut s, &m.0, x0, y0, x1, y1, color) };
    }

    /// Present a system-RAM surface onto the frame buffer (one blit per frame).
    /// Draw into RAM, then call this — much faster than per-primitive FB stores.
    pub fn present_from(&mut self, src: &crate::Surface) {
        let mut dst = self.surface();
        src.blit_to_raw(&mut dst);
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if let Some(ref saved) = self.saved {
            let n = saved.len().min(self.map_len);
            unsafe {
                ptr::copy_nonoverlapping(saved.as_ptr(), self.map, n);
            }
        }
        if self.kd_graphics {
            if let Some(ref tty) = self.tty_fd {
                unsafe {
                    let _ = libc::ioctl(tty.as_raw_fd(), KDSETMODE as _, KD_TEXT);
                }
            } else if self.kd_on_stdin {
                unsafe {
                    let _ = libc::ioctl(libc::STDIN_FILENO, KDSETMODE as _, KD_TEXT);
                }
            }
        }
        unsafe {
            libc::munmap(self.map as *mut libc::c_void, self.map_len);
        }
    }
}

/// Switch controlling tty to graphics mode so the kernel text renderer
/// does not fight our pixels. Best on a real VT (Ctrl+Alt+F3).
fn try_enter_graphics_mode() -> (Option<OwnedFd>, bool, bool) {
    for path in ["/dev/tty", "/dev/console"] {
        let c = match std::ffi::CString::new(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let raw = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_NOCTTY) };
        if raw < 0 {
            continue;
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let r = unsafe { libc::ioctl(fd.as_raw_fd(), KDSETMODE as _, KD_GRAPHICS) };
        if r == 0 {
            return (Some(fd), true, false);
        }
        drop(fd);
    }
    unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 1
            && libc::ioctl(libc::STDIN_FILENO, KDSETMODE as _, KD_GRAPHICS) == 0
        {
            return (None, true, true);
        }
    }
    (None, false, false)
}

unsafe impl Send for Framebuffer {}

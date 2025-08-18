use core::slice;

use spin::{Mutex, MutexGuard, Once};

use crate::limine;
use crate::memory::align_up;

static DRIVER: Once<FramebufferDriver> = Once::new();

pub struct FramebufferDriver {
    info: FramebufferInfo,
    device: Mutex<Framebuffer<'static>>,
}

pub fn init() {
    let (ptr, info) = limine::framebuffer_information()
        .next()
        .expect("No valid framebuffers found");

    let buffer_size = info.pitch * info.height;
    let buffer = unsafe { slice::from_raw_parts_mut(ptr as *mut u32, buffer_size) };

    let primary_framebuffer = Framebuffer { info, buffer };
    DRIVER.call_once(|| FramebufferDriver {
        info,
        device: Mutex::new(primary_framebuffer),
    });
}

pub fn get() -> &'static FramebufferDriver {
    DRIVER.get().unwrap()
}

impl FramebufferDriver {
    #[inline]
    pub fn width(&self) -> usize {
        self.info.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.info.height
    }

    #[inline]
    pub fn pitch(&self) -> usize {
        self.info.pitch
    }

    #[inline]
    pub fn buffer_len(&self) -> usize {
        self.info.height * self.info.pitch
    }

    pub fn buffer<'a>(&'a self) -> MutexGuard<'a, Framebuffer<'static>> {
        self.device.lock()
    }
}

unsafe impl Send for FramebufferDriver {}
unsafe impl Sync for FramebufferDriver {}

#[derive(Clone, Copy)]
pub struct FramebufferInfo {
    pitch: usize,
    width: usize,
    height: usize,
}

impl FramebufferInfo {
    pub fn from(buffer: ::limine::framebuffer::Framebuffer) -> Option<(*mut u8, Self)> {
        let bytes_per_pixel = align_up(buffer.bpp() as usize, 8) / 8;

        // Ensure the framebuffer uses 4 byte RGB values
        if buffer.memory_model() != ::limine::framebuffer::MemoryModel::RGB || bytes_per_pixel != 4
        {
            return None;
        }

        // Ensure each color uses a single byte
        for mask_size in [
            buffer.red_mask_size(),
            buffer.green_mask_size(),
            buffer.blue_mask_size(),
        ] {
            if mask_size != 8 {
                return None;
            }
        }

        // Ensure colors use the standard 0RGB layout
        if buffer.red_mask_shift() != 16
            || buffer.green_mask_shift() != 8
            || buffer.blue_mask_shift() != 0
        {
            return None;
        }

        // Ensure potential padding is pixel sized
        if !buffer.pitch().is_multiple_of(bytes_per_pixel as u64) {
            return None;
        }

        Some((
            buffer.addr(),
            Self {
                pitch: buffer.pitch() as usize / bytes_per_pixel,
                width: buffer.width() as usize,
                height: buffer.height() as usize,
            },
        ))
    }
}

pub struct Framebuffer<'a> {
    info: FramebufferInfo,
    buffer: &'a mut [u32],
}

impl<'a> Framebuffer<'a> {
    #[inline(always)]
    pub fn set_pixel_value(&mut self, x: usize, y: usize, color: RGB) {
        self.buffer[x + y * self.info.pitch] = color.into();
    }

    #[inline(always)]
    pub fn fill(&mut self, color: RGB) {
        self.buffer.fill(color.into())
    }

    #[inline(always)]
    pub fn update_from_slice(&mut self, slice: &[u32]) {
        self.buffer.copy_from_slice(slice);
    }

    pub fn update_range_from_slice(&mut self, start: usize, end: usize, slice: &[u32]) {
        (&mut self.buffer[start..end]).copy_from_slice(slice);
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.info.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.info.height
    }

    #[inline]
    pub fn pitch(&self) -> usize {
        self.info.pitch
    }

    #[inline]
    pub fn buffer_len(&self) -> usize {
        self.info.height * self.info.pitch
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy)]
pub struct RGB(u32);

impl RGB {
    #[inline(always)]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self((r as u32) << 16 | (g as u32) << 8 | b as u32)
    }

    #[inline(always)]
    pub const fn from_hex(hex: u32) -> Self {
        assert!(hex <= 0xFFFFFF);
        Self(hex)
    }

    pub const fn red(&self) -> u8 {
        (self.0 >> 16 & 0xFF) as u8
    }

    pub const fn green(&self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    pub const fn blue(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    pub const fn alpha_blend(fg: RGB, bg: RGB, alpha: u8) -> RGB {
        let alpha = alpha as u64;
        let red = (fg.red() as u64 * alpha + (255 - alpha) * bg.red() as u64) / 255;
        let green = (fg.green() as u64 * alpha + (255 - alpha) * bg.green() as u64) / 255;
        let blue = (fg.blue() as u64 * alpha + (255 - alpha) * bg.blue() as u64) / 255;
        RGB::new(red as u8, green as u8, blue as u8)
    }
}

impl RGB {
    pub const WHITE: RGB = RGB::new(255, 255, 255);
    pub const BLACK: RGB = RGB::new(0, 0, 0);
    pub const RED: RGB = RGB::new(255, 0, 0);
    pub const GREEN: RGB = RGB::new(0, 255, 0);
    pub const BLUE: RGB = RGB::new(0,0,255);
    pub const CYAN: RGB = RGB::new(0, 255, 255);
    pub const YELLOW: RGB = RGB::new(255, 255, 0);
    pub const MAGENTA: RGB = RGB::new(255, 0, 255);
}

impl From<RGB> for u32 {
    #[inline(always)]
    fn from(value: RGB) -> Self {
        value.0
    }
}

impl From<u32> for RGB {
    #[inline(always)]
    fn from(value: u32) -> Self {
        Self(value)
    }
}

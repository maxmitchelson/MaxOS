use core::ops::{Bound, RangeBounds};
use core::{fmt, slice};

use spin::{Mutex, MutexGuard, Once};

use crate::memory::{align_up, frame_allocator};
use crate::terminal::logger;
use crate::terminal::tty::TERMINAL;
use crate::{LOGGER, limine};

static DRIVER: Once<FramebufferDriver> = Once::new();

pub struct FramebufferDriver {
    info: FramebufferInfo,
    device: Mutex<Framebuffer<'static>>,
}

pub fn init() {
    let (front_ptr, info) = limine::framebuffer_information()
        .next()
        .expect("No valid framebuffers found");

    let buffer_size = info.pitch * info.height;
    let back_buffer = unsafe {
        let back_ptr = frame_allocator::allocate(buffer_size * 4)
            .to_virtual()
            .to_ptr::<u32>();

        slice::from_raw_parts_mut(back_ptr, buffer_size)
    };

    let front_buffer = unsafe { slice::from_raw_parts_mut(front_ptr as *mut u32, buffer_size) };

    let primary_framebuffer = Framebuffer {
        info,
        back_buffer_cursor: 0,
        front_buffer,
        back_buffer,
    };
    DRIVER.call_once(|| FramebufferDriver {
        info,
        device: Mutex::new(primary_framebuffer),
    });
}

pub fn driver() -> &'static FramebufferDriver {
    DRIVER.get().unwrap()
}

impl FramebufferDriver {
    #[inline]
    pub fn info(&self) -> FramebufferInfo {
        self.info
    }

    pub fn device<'a>(&'a self) -> MutexGuard<'a, Framebuffer<'static>> {
        self.device.lock()
    }
}

unsafe impl Send for FramebufferDriver {}
unsafe impl Sync for FramebufferDriver {}

#[derive(Clone, Copy, Debug)]
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

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn pitch(&self) -> usize {
        self.pitch
    }

    #[inline]
    pub fn buffer_len(&self) -> usize {
        self.height * self.pitch
    }
}

pub struct Framebuffer<'a> {
    info: FramebufferInfo,
    back_buffer_cursor: usize,
    back_buffer: &'a mut [u32],
    front_buffer: &'a mut [u32],
}

impl<'a> Framebuffer<'a> {
    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: RGB) {
        self.back_buffer
            [(x + y * self.info.pitch + self.back_buffer_cursor) % self.back_buffer.len()] =
            color.into();
    }

    #[inline(always)]
    pub fn update_from_slice(&mut self) -> &mut [u32] {
        self.back_buffer
    }

    #[inline(always)]
    pub fn fill(&mut self, color: RGB) {
        self.back_buffer.fill(color.into())
    }

    pub fn partial_fill(&mut self, range: impl RangeBounds<usize>, color: RGB) {
        let start = match range.start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(&i) => i + 1,
            Bound::Excluded(&i) => i,
            Bound::Unbounded => self.back_buffer.len(),
        };

        assert!(start <= end);
        assert!(end <= self.back_buffer.len());

        let (head, tail) = self.back_buffer.split_at_mut(self.back_buffer_cursor);

        if start < tail.len() {
            let tail_end = end.min(tail.len());
            tail[start..tail_end].fill(color.into());
        }

        if end > tail.len() {
            let head_start = start.saturating_sub(tail.len());
            let head_end = end - tail.len();
            head[head_start..head_end].fill(color.into());
        }
    }

    #[inline(always)]
    pub fn refresh(&mut self) {
        let (head, tail) = self.back_buffer.split_at(self.back_buffer_cursor);
        self.front_buffer[..tail.len()].copy_from_slice(tail);
        self.front_buffer[tail.len()..].copy_from_slice(head);
    }

    pub fn scroll(&mut self, height: usize) {
        self.back_buffer_cursor = (self.back_buffer_cursor + self.info.pitch * height) % self.back_buffer.len();
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
    pub const BLUE: RGB = RGB::new(0, 0, 255);
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

impl fmt::Debug for RGB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RGB")
            .field(&self.red())
            .field(&self.green())
            .field(&self.blue())
            .finish()
    }
}

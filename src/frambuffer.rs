pub struct FramebufferInfo {
    pub pitch: usize,
    pub width: usize,
    pub height: usize,
    pub bytes_per_pixel: usize,
}

impl FramebufferInfo {
    pub fn from_limine_framebuffer(buffer: &limine::framebuffer::Framebuffer) -> Self {
        let bytes_per_pixel = ((buffer.bpp() as usize + 7) & !8) / 8; // align up to byte size

        Self {
            // convert from byte pitch to pixel pitch
            pitch: buffer.pitch() as usize / bytes_per_pixel,
            width: buffer.width() as usize,
            height: buffer.height() as usize,
            bytes_per_pixel,
        }
    }
}

pub struct Framebuffer<'buf> {
    pub info: FramebufferInfo,
    pub buffer: &'buf mut [u32],
}

#[allow(clippy::upper_case_acronyms)]
pub struct RGB(u32);

impl RGB {
    #[inline(always)]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self((r as u32) << 16 | (g as u32) << 8 | b as u32)
    }

    #[inline(always)]
    pub const fn from_hex(hex: u32) -> Self {
        assert!(hex < 0x1000000); // ensure max value is 0xFFFFFF
        Self(hex)
    }

    pub fn red(&self) -> u8 {
        (self.0 >> 16 & 0xFF) as u8
    }

    pub fn green(&self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    pub fn blue(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }
}

impl From<RGB> for u32 {
    fn from(value: RGB) -> Self {
        value.0
    }
}

impl From<u32> for RGB {
    fn from(value: u32) -> Self {
        RGB(value)
    }
}

impl<'buf> Framebuffer<'buf> {
    #[inline(always)]
    pub fn set_pixel_value(&mut self, x: usize, y: usize, value: RGB) {
        self.buffer[x + y * self.info.pitch] = value.0;
    }
}

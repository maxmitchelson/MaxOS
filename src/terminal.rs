use core::fmt::Write;

use crate::framebuffer::{Framebuffer, RGB};

use noto_sans_mono_bitmap::{FontWeight, RasterHeight, get_raster, get_raster_width};
use spin::RwLock;

const HORIZONTAL_MARGIN: usize = 20;
const VERTICAL_MARGIN: usize = 20;

const FONT_STYLE: FontWeight = FontWeight::Regular;
const FONT_SIZE: RasterHeight = RasterHeight::Size20;

pub struct TerminalDriver(pub RwLock<Terminal>);

pub struct Terminal {
    cursor_x: usize,
    cursor_y: usize,
    framebuffer: &'static RwLock<Framebuffer>,
}

impl Terminal {
    pub fn new(framebuffer: &'static RwLock<Framebuffer>) -> Self {
        framebuffer.write().buffer.fill(0x000000); // FIXME: raw color code

        Self {
            cursor_x: VERTICAL_MARGIN,
            cursor_y: HORIZONTAL_MARGIN,
            framebuffer,
        }
    }

    pub fn write_str(&mut self, str: &str) {
        for ch in str.chars() {
            if ch == '\n' {
                self.goto_next_line();
                continue;
            }

            let raster_width = get_raster_width(FONT_STYLE, FONT_SIZE);
            if self.cursor_x + raster_width + HORIZONTAL_MARGIN
                >= self.framebuffer.read().info.width
            {
                self.goto_next_line();
            }
            let raster = get_raster(ch, FONT_STYLE, FONT_SIZE).unwrap();

            let mut framebuffer = self.framebuffer.write();

            for (y, row) in raster.raster().iter().enumerate() {
                for (x, pixel) in row.iter().enumerate() {
                    framebuffer.set_pixel_value(
                        x + self.cursor_x,
                        y + self.cursor_y,
                        RGB::new(*pixel, *pixel, *pixel),
                    );
                }
            }

            self.cursor_x += raster_width;
        }
    }

    pub fn goto_next_line(&mut self) {
        self.cursor_x = VERTICAL_MARGIN;
        self.cursor_y += FONT_SIZE.val();
    }
}

impl Write for Terminal {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

impl TerminalDriver {
    pub fn write_fmt(&self, s: core::fmt::Arguments) -> core::fmt::Result {
        let _ = self.0.write().write_fmt(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let _ = $crate::TERMINAL.write_fmt(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($fmt:expr) => {
        $crate::print!(concat!($fmt, "\n"))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print!(concat!($fmt, "\n"), $($arg)*)
    };
}

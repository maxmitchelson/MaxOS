use core::fmt::Write;

use crate::drivers::framebuffer;
use crate::drivers::framebuffer::{Framebuffer, RGB};

use noto_sans_mono_bitmap::{FontWeight, RasterHeight, get_raster, get_raster_width};
use spin::{MutexGuard, Once, RwLock};

const HORIZONTAL_MARGIN: usize = 20;
const VERTICAL_MARGIN: usize = 20;

const FONT_STYLE: FontWeight = FontWeight::Regular;
const FONT_SIZE: RasterHeight = RasterHeight::Size20;

pub static _WRITER: Once<TerminalWriter> = Once::new();

pub struct TerminalWriter(RwLock<Terminal<'static>>);
impl TerminalWriter {
    /// Simply calls fmt::write_fmt on the Terminal. This wrapper is necessary because the
    /// write_fmt method requires a mutable borrow that would need to be acquired before calling
    /// it. In case of nested terminal writes, not using this would result in deadlocks.
    pub fn write_to_terminal(&self, s: core::fmt::Arguments) -> core::fmt::Result {
        self.0.write().write_fmt(s)
    }
}

pub fn init() {
    _WRITER.call_once(|| TerminalWriter(RwLock::new(Terminal::new())));
}

pub struct Terminal<'a> {
    cursor_x: usize,
    cursor_y: usize,
    theme: Theme,
    framebuffer: MutexGuard<'a, Framebuffer<'static>>,
}

impl<'a> Terminal<'a> {
    pub fn new() -> Self {
        let mut term = Self {
            cursor_x: VERTICAL_MARGIN,
            cursor_y: HORIZONTAL_MARGIN,
            theme: Theme::GRUVBOX,
            framebuffer: framebuffer::get().buffer(),
        };

        term.framebuffer.fill(term.theme.background);
        term
    }

    pub fn write_str(&mut self, str: &str) {
        for ch in str.chars() {
            if ch == '\n' {
                self.goto_next_line();
                continue;
            }

            let raster_width = get_raster_width(FONT_STYLE, FONT_SIZE);
            if self.cursor_x + raster_width + HORIZONTAL_MARGIN >= framebuffer::get().width() {
                self.goto_next_line();
            }
            let raster = get_raster(ch, FONT_STYLE, FONT_SIZE).unwrap().raster();

            for (y, row) in raster.iter().enumerate() {
                for (x, alpha) in row.iter().enumerate().filter(|(_, p)| **p != 0) {
                    self.framebuffer.set_pixel_value(
                        x + self.cursor_x,
                        y + self.cursor_y,
                        RGB::alpha_blend(self.theme.foreground, self.theme.background, *alpha),
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

impl<'a> Write for Terminal<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

struct Theme {
    foreground: RGB,
    background: RGB,
    selection_foreground: RGB,
    selection_background: RGB,
    cursor: RGB,
    cursor_text_color: RGB,
    ansi_colors: [RGB; 16],
}

impl Theme {
    const CATPPUCCIN: Theme = Self {
        foreground: RGB::from_hex(0xcdd6f4),
        background: RGB::from_hex(0x1e1e2e),
        selection_foreground: RGB::from_hex(0x1e1e2e),
        selection_background: RGB::from_hex(0xf5e0dc),
        cursor: RGB::from_hex(0xf5e0dc),
        cursor_text_color: RGB::from_hex(0x1e1e2e),
        ansi_colors: [
            RGB::from_hex(0x45475a),
            RGB::from_hex(0xf38ba8),
            RGB::from_hex(0xa6e3a1),
            RGB::from_hex(0xf9e2af),
            RGB::from_hex(0x89b4fa),
            RGB::from_hex(0xf5c2e7),
            RGB::from_hex(0x94e2d5),
            RGB::from_hex(0xbac2de),
            RGB::from_hex(0x585b70),
            RGB::from_hex(0xf38ba8),
            RGB::from_hex(0xa6e3a1),
            RGB::from_hex(0xf9e2af),
            RGB::from_hex(0x89b4fa),
            RGB::from_hex(0xf5c2e7),
            RGB::from_hex(0x94e2d5),
            RGB::from_hex(0xa6adc8),
        ],
    };
    const GRUVBOX: Theme = Self {
        foreground: RGB::from_hex(0xebdbb2),
        background: RGB::from_hex(0x282828),
        selection_foreground: RGB::from_hex(0x928374),
        selection_background: RGB::from_hex(0xebdbb2),
        cursor: RGB::from_hex(0x928374),
        cursor_text_color: RGB::from_hex(0x282828),
        ansi_colors: [
            RGB::from_hex(0x665c54),
            RGB::from_hex(0xcc241d),
            RGB::from_hex(0x98971a),
            RGB::from_hex(0xd79921),
            RGB::from_hex(0x458588),
            RGB::from_hex(0xb16286),
            RGB::from_hex(0x689d6a),
            RGB::from_hex(0xa89984),
            RGB::from_hex(0x7c6f64),
            RGB::from_hex(0xfb4934),
            RGB::from_hex(0xb8bb26),
            RGB::from_hex(0xfabd2f),
            RGB::from_hex(0x83a598),
            RGB::from_hex(0xd3869b),
            RGB::from_hex(0x8ec07c),
            RGB::from_hex(0xbdae93),
        ],
    };
}

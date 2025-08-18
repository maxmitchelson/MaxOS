use core::fmt;

use noto_sans_mono_bitmap::{get_raster, FontWeight, RasterHeight, RasterizedChar};
use spin::{Mutex, MutexGuard, Once};

use crate::{drivers::framebuffer::{self, Framebuffer, RGB}, terminal::themes::Theme};

const HORIZONTAL_MARGIN: usize = 20;
const VERTICAL_MARGIN: usize = 20;

const FONT_STYLE: FontWeight = FontWeight::Bold;
const FONT_SIZE: RasterHeight = RasterHeight::Size20;

pub static _WRITER: Once<Mutex<Terminal<'static>>> = Once::new();

pub struct TerminalWriter;
impl TerminalWriter {
    pub fn new() -> Self {
        Self {}
    }
}

impl fmt::Write for TerminalWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        _WRITER.get().unwrap().lock().write_str(s)
    }
}

pub fn init() {
    _WRITER.call_once(|| Mutex::new(Terminal::new()));
}

pub struct Terminal<'a> {
    cursor_x: usize,
    cursor_y: usize,
    fg_color: RGB,
    bg_color: RGB,
    theme: Theme,
    framebuffer: MutexGuard<'a, Framebuffer<'static>>,
}

impl<'a> Terminal<'a> {
    pub fn new() -> Self {
        let mut term = Self {
            cursor_x: VERTICAL_MARGIN,
            cursor_y: HORIZONTAL_MARGIN,
            fg_color: RGB::WHITE,
            bg_color: RGB::BLACK,
            theme: Theme::GRUVBOX,
            framebuffer: framebuffer::get().buffer(),
        };

        term.fg_color = term.theme.foreground;
        term.bg_color = term.theme.background;
        term.framebuffer.fill(term.theme.background);
        term
    }

    fn parse_ansi_sequence(&mut self, chars: &mut core::str::Chars) -> fmt::Result {
        match chars.next() {
            Some('[') => loop {
                let code = chars.take_while(|c| *c != ';');
                let mut numerical_code = 0;
                for digit in code {
                    if digit == 'm' {
                        self.parse_ansi_code(numerical_code)?;
                        return Ok(());
                    }
                    match digit.to_digit(10) {
                        Some(d) => {
                            numerical_code *= 10;
                            numerical_code += d;
                        }
                        None => return Err(fmt::Error),
                    }
                }
                self.parse_ansi_code(numerical_code)?;
            },
            _ => Err(fmt::Error),
        }
    }

    fn parse_ansi_code(&mut self, code: u32) -> fmt::Result {
        let code = code as usize;
        match code {
            0 => {
                self.fg_color = self.theme.foreground;
                self.bg_color = self.theme.background
            }
            30..38 => self.fg_color = self.theme.ansi_colors[code - 30],
            40..48 => self.bg_color = self.theme.ansi_colors[code - 40],
            90..98 => self.fg_color = self.theme.ansi_colors[code - 90 + 8],
            100..108 => self.bg_color = self.theme.ansi_colors[code - 100 + 8],
            _ => return Err(fmt::Error),
        }
        Ok(())
    }

    pub fn render_str(&mut self, str: &str) -> fmt::Result {
        let mut chars = str.chars();
        while let Some(c) = &chars.next() {
            match c {
                '\n' => self.jump_line(),
                '\t' => self.render_str("    ")?,
                '\x1b' => self.parse_ansi_sequence(&mut chars)?,
                c => self.render_char(*c)?,
            }
        }
        Ok(())
    }

    pub fn render_char(&mut self, ch: char) -> fmt::Result {
        self.render_raster(get_raster(ch, FONT_STYLE, FONT_SIZE).ok_or(fmt::Error)?);
        Ok(())
    }

    fn render_raster(&mut self, raster: RasterizedChar) {
        if self.cursor_x + raster.width() + HORIZONTAL_MARGIN >= framebuffer::get().width() {
            self.jump_line();
        }
        for (y, row) in raster.raster().iter().enumerate() {
            for (x, alpha) in row.iter().enumerate() {
                self.framebuffer.set_pixel_value(
                    x + self.cursor_x,
                    y + self.cursor_y,
                    RGB::alpha_blend(self.fg_color, self.bg_color, *alpha),
                );
            }
        }

        self.cursor_x += raster.width();
    }

    pub fn jump_line(&mut self) {
        self.cursor_x = VERTICAL_MARGIN;
        self.cursor_y += FONT_SIZE.val();
    }
}

impl<'a> fmt::Write for Terminal<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.render_str(s)
    }
}


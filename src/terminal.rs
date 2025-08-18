use core::fmt::{self, Display, Write};

use crate::drivers::framebuffer;
use crate::drivers::framebuffer::{Framebuffer, RGB};

use noto_sans_mono_bitmap::{FontWeight, RasterHeight, RasterizedChar, get_raster};
use spin::{Mutex, MutexGuard, Once};

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

impl Write for TerminalWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
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

impl<'a> Write for Terminal<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.render_str(s)
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Critical = 4,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            f.write_str(match self {
                Self::Debug => "\x1b[32mDEBUG\x1b[0m",
                Self::Info => "\x1b[32mINFO\x1b[0m",
                Self::Warn => "\x1b[33mWARN\x1b[0m",
                Self::Error => "\x1b[91mERROR\x1b[0m",
                Self::Critical => "\x1b[31mCRITICAL\x1b[0m",
            })
        } else {
            f.write_str(match self {
                Self::Debug => "DEBUG",
                Self::Info => "INFO",
                Self::Warn => "WARN",
                Self::Error => "ERROR",
                Self::Critical => "CRITICAL",
            })
        }
    }
}

pub struct Logger {
    level: LogLevel,
}

impl Logger {
    pub const fn new(level: LogLevel) -> Self {
        Self { level }
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        if level < self.level {
            return;
        }

        let mut writer = TerminalWriter::new();
        writeln!(writer, "[{:#}]: {}", level, message);
    }

    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn critical(&self, message: &str) {
        self.log(LogLevel::Critical, message);
    }
}

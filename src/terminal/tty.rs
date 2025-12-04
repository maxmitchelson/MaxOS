use core::{alloc::Layout, fmt, ptr, slice};

use spin::{Mutex, Once};

use crate::{
    drivers::framebuffer::{self, RGB},
    memory::{VirtualAddress, frame_allocator},
    terminal::{ansi::*, font, logger, themes::Theme},
};

const HORIZONTAL_MARGIN: usize = 20;
const VERTICAL_MARGIN: usize = 20;

static TERMINAL: Once<Mutex<Terminal>> = Once::new();

pub fn init() {
    TERMINAL.call_once(|| Mutex::new(Terminal::new()));
}

pub struct TerminalStdin {}

impl TerminalStdin {
    pub fn new() -> Self {
        Self {}
    }
}

impl fmt::Write for TerminalStdin {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        TERMINAL.get().unwrap().lock().write_str(s)
    }
}

#[derive(Debug, Clone, Copy)]
struct Pos {
    line: usize,
    column: usize,
}

impl Pos {
    fn origin() -> Self {
        Self { line: 0, column: 0 }
    }
}

impl Default for Pos {
    fn default() -> Self {
        Self::origin()
    }
}

#[derive(Debug, Clone, Copy)]
struct Style {
    foreground: AnsiColor,
    background: AnsiColor,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            foreground: AnsiColor::DefaultForeground,
            background: AnsiColor::DefaultBackground,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Selection {
    begin: Pos,
    end: Pos,
}

pub struct Terminal<'buf> {
    width: usize,
    height: usize,
    cursor: Pos,
    scroll: usize,
    selection: Option<Selection>,
    buffer: TerminalBuffer<'buf>,
    ansi_handler: AnsiHandler,
    style: Style,
    theme: Theme,
}

impl<'buf> Terminal<'buf> {
    pub fn new() -> Self {
        let display_info = framebuffer::driver().info();
        let height = (display_info.height() - 2 * VERTICAL_MARGIN) / font::HEIGHT;
        let width = (display_info.width() - 2 * HORIZONTAL_MARGIN) / font::WIDTH;

        let term = Self {
            width,
            height,
            cursor: Pos::origin(),
            scroll: 0,
            selection: None,
            buffer: TerminalBuffer::new(height * 30, width),
            ansi_handler: AnsiHandler::new(),
            style: Style::default(),
            theme: Theme::default(),
        };

        term.full_draw();
        term
    }

    /// Input function to the [`Terminal`].
    /// Manages the handling of ANSI sequences inside the input and adds regular text to the [`TerminalBuffer`]
    fn push_input(&mut self, input: impl IntoIterator<Item = char>) {
        let mut iterator = input.into_iter();

        if self.ansi_handler.is_active() {
            self.parse_ansi(iterator.by_ref());
        }

        let mut element = iterator.next();
        while let Some(ch) = element {
            match ch {
                '\n' => self.jump_line(),
                '\t' => self.send_to_buffer("    ".chars()),
                '\x1b' => self.parse_ansi(iterator.by_ref()),
                _ => self.send_char_to_buffer(ch),
            }

            element = iterator.next();
        }
        self.full_draw();
    }

    /// Start or continue parsing of an ANSI sequence using the ANSI handler.
    fn parse_ansi(&mut self, sequence: impl IntoIterator<Item = char>) {
        self.ansi_handler.try_start();
        let ansi_result = self.ansi_handler.continue_parse(sequence);

        match ansi_result {
            ParserResult::Valid(command) => {
                self.ansi_handler.reset();
                self.execute_ansi_command(command);
            }
            ParserResult::Incomplete => (),
            ParserResult::Error(ansi_error) => {
                self.ansi_handler.reset();
            }
        }
    }

    /// Send text to the buffer and adjust the cursor accordingly.
    #[inline]
    fn send_to_buffer(&mut self, text: impl IntoIterator<Item = char>) {
        let n_cells =
            self.buffer
                .write_formatted(text, self.cursor.line, self.cursor.column, self.style);
        self.advance_cursor_wrapping(n_cells);
    }

    /// Send char to the buffer and adjust the cursor accordingly.
    #[inline]
    fn send_char_to_buffer(&mut self, ch: char) {
        self.buffer
            .write_char(ch, self.cursor.line, self.cursor.column, self.style);
        self.advance_cursor_wrapping(1);
    }

    /// Advance the cursor by `len`, wrapping to the next line in case the end of the buffer
    /// for the current line is reached. Ensures the cursor is always in view by adjusting the
    /// scroll.
    fn advance_cursor_wrapping(&mut self, len: usize) {
        let new_col_with_overflow = self.cursor.column + len;
        self.cursor.column = (new_col_with_overflow) % self.buffer.max_columns;
        self.cursor.line += (new_col_with_overflow) / self.buffer.max_columns;

        if self.cursor.line > self.scroll + self.height {
            self.scroll = self.cursor.line - self.height;
        } else if self.cursor.line < self.scroll {
            self.scroll = self.cursor.line;
        }
    }

    /// Skips a line. Corresponds to the typical `'\n'` behavior.
    fn jump_line(&mut self) {
        self.cursor.column = 0;
        self.cursor.line += 1;
    }

    /// Executes the provided ANSI `command`
    fn execute_ansi_command(&mut self, command: AnsiCommand) {
        match command {
            AnsiCommand::CursorMoveAbsolute { line, column } => {
                self.move_cursor_absolute(line, column)
            }
            AnsiCommand::CursorMoveRelative { line, column } => {
                self.move_cursor_relative(line, column)
            }
            AnsiCommand::CursorMoveColumnAbsolute(column) => {
                self.move_cursor_absolute(self.cursor.line, column)
            }
            AnsiCommand::EraseDisplay {
                mode,
                preserve_offscreen,
            } => todo!(),
            AnsiCommand::EraseLine(erase_mode) => todo!(),
            AnsiCommand::ScrollRelative(delta) => self.scroll_relative(delta),
            AnsiCommand::SetBackground(ansi_color) => self.set_background(ansi_color),
            AnsiCommand::SetForeground(ansi_color) => self.set_foreground(ansi_color),
            AnsiCommand::ResetGraphicRendition => self.reset_style(),
        }
    }

    /// Moves the cursor to the specified line and column.
    /// Ensures the results are valid line and column.
    /// Note: The origin (0,0) is in the top-left corner and axes are positive to the right and downards.
    fn move_cursor_absolute(&mut self, line: usize, column: usize) {
        let line = self.scroll + line;
        self.cursor.line = line.clamp(self.scroll, self.scroll+self.height);
        self.cursor.column = column.clamp(0, self.buffer.get_line_length(self.cursor.line));
    }

    /// Moves the cursor according to the provided deltas.
    /// Ensures the results are valid line and column.
    /// Note: The origin (0,0) is in the top-left corner and axes are positive to the right and downards.
    fn move_cursor_relative(&mut self, line_delta: isize, column_delta: isize) {
        self.cursor.line = self
            .cursor
            .line
            .saturating_add_signed(line_delta)
            .min(self.buffer.max_lines);
        self.cursor.column = self
            .cursor
            .column
            .saturating_add_signed(column_delta)
            .min(self.buffer.get_line_length(self.cursor.line));
    }

    /// Scrolls downwards by delta if it's positive and upwards by -delta otherwise.
    /// Ensures the result is within the range of valid lines.
    fn scroll_relative(&mut self, delta: isize) {
        self.scroll = self
            .scroll
            .saturating_add_signed(delta)
            .min(self.buffer.max_lines);
    }

    fn set_background(&mut self, color: AnsiColor) {
        self.style.background = color;
    }

    fn set_foreground(&mut self, color: AnsiColor) {
        self.style.foreground = color;
    }

    /// Reset the style to the one set in `self.theme`
    fn reset_style(&mut self) {
        self.style.foreground = AnsiColor::DefaultForeground;
        self.style.background = AnsiColor::DefaultBackground;
    }

    /// Convert `ansi_color` to RGB according to the current theme
    fn ansi_to_rgb(&self, ansi_color: AnsiColor) -> RGB {
        match ansi_color {
            AnsiColor::ColorCode(code) => self.theme.ansi_colors[code as usize],
            AnsiColor::Rgb(r, g, b) => RGB::new(r, g, b),
            AnsiColor::DefaultForeground => self.theme.foreground,
            AnsiColor::DefaultBackground => self.theme.background,
        }
    }

    /// Draw the entire scroll view in the framebuffer;
    pub fn full_draw(&self) {
        let rows = self
            .buffer
            .get_view(self.scroll, self.height)
            .chunks(self.buffer.max_columns);

        let mut fb = framebuffer::driver().device();
        fb.fill(self.theme.background);

        let mut logical_x = 0;
        let mut logical_y = 0;
        'all: for row in rows {
            for cell in row.iter().flatten() {
                let raster = font::get_raster(cell.content).unwrap();
                let visual_x = HORIZONTAL_MARGIN + logical_x * font::WIDTH;
                let visual_y = VERTICAL_MARGIN + logical_y * font::HEIGHT;

                for (char_y, char_row) in raster.raster().iter().enumerate() {
                    for (char_x, alpha) in char_row.iter().enumerate() {
                        let fg_color = self.ansi_to_rgb(cell.style.foreground);
                        let bg_color = self.ansi_to_rgb(cell.style.background);
                        let color = RGB::alpha_blend(fg_color, bg_color, *alpha);

                        fb.set_pixel(char_x + visual_x, char_y + visual_y, color);
                    }
                }

                if logical_x + 1 == self.width {
                    logical_x = 0;
                    logical_y += 1;
                    if logical_y == self.height {
                        break 'all;
                    }
                } else {
                    logical_x += 1;
                }
            }
            logical_x = 0;
            logical_y += 1;
            if logical_y == self.height {
                break;
            }
        }

        fb.refresh();
    }
}

impl<'buf> fmt::Write for Terminal<'buf> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_input(s.chars());
        Ok(())
    }
}

/// Single text cell for the terminal buffer. Contains text and style information.
#[derive(Debug, Clone, Copy)]
struct TextCell {
    style: Style,
    content: char,
}

impl TextCell {
    fn empty() -> Option<Self> {
        None
    }
}

/// Buffer for a terminal. Owns an array of [`TextCell`]s.
/// This buffer is used to maintain style and text across scroll without maintaing the wasteful
/// rasterized view.
struct TerminalBuffer<'txt> {
    max_lines: usize,
    max_columns: usize,
    buffer: &'txt mut [Option<TextCell>],
    end_ptr: usize,
}

impl<'txt> TerminalBuffer<'txt> {
    fn new(lines: usize, columns: usize) -> Self {
        let length = lines * columns;
        let cells_buffer = unsafe {
            let cells_layout = Layout::array::<Option<TextCell>>(length).unwrap();
            let cells_ptr = frame_allocator::allocate(cells_layout.size())
                .to_virtual()
                .to_ptr::<Option<TextCell>>();

            for i in 0..length {
                ptr::write(cells_ptr.add(i), TextCell::empty());
            }

            slice::from_raw_parts_mut(cells_ptr, length)
        };

        Self {
            max_lines: lines,
            max_columns: columns,
            buffer: cells_buffer,
            end_ptr: 0,
        }
    }

    /// Write the specified `text` to the buffer using the provided `style` and position. Returns the number of cells occupied by the text.
    /// Note: This will overwrite existing cells if necessary
    #[inline]
    fn write_formatted<I>(&mut self, text: I, line: usize, column: usize, style: Style) -> usize
    where
        I: IntoIterator<Item = char>,
    {
        let ptr = line * self.max_columns + column;
        let mut offset = 0;
        for ch in text.into_iter() {
            if ptr + offset + 1 == self.buffer.len() {
                unsafe { self.grow_buffer() };
            }

            self.buffer[ptr + offset] = Some(TextCell { style, content: ch });
            offset += 1;
        }

        if ptr + offset > self.end_ptr {
            self.end_ptr = ptr;
        }

        offset
    }

    #[inline(always)]
    fn write_char(&mut self, ch: char, line: usize, column: usize, style: Style) {
        let pos = line * self.max_columns + column;
        if pos + 1 == self.buffer.len() {
            unsafe { self.grow_buffer() };
        }

        self.buffer[pos] = Some(TextCell { style, content: ch });
    }

    /// Compute the length of the specified line.
    /// The length is defined as the 1-indexed column of the last non-empty cell of the line.
    fn get_line_length(&self, line: usize) -> usize {
        let line = &self.buffer[line * self.max_columns..(line + 1) * self.max_columns];
        for (i, cell) in line.iter().enumerate().rev() {
            if cell.is_some() {
                return i + 1;
            }
        }
        0
    }

    /// Clear the specified range of cells
    fn clear_range(&mut self, start: usize, len: usize) {
        for i in start..start + len {
            self.buffer[i] = None;
        }
    }

    /// Returns the slice of TextCells between lines `start_line` and `start_line + height`.
    fn get_view(&self, start_line: usize, height: usize) -> &[Option<TextCell>] {
        &self.buffer[start_line * self.max_columns..(start_line + height) * self.max_columns]
    }

    unsafe fn grow_buffer(&mut self) {
        unsafe {
            let old_len = self.buffer.len();
            let new_len = self.buffer.len() * 2;
            let cells_layout = Layout::array::<TextCell>(new_len).unwrap();
            let ptr = frame_allocator::reallocate(
                VirtualAddress::from_ptr(self.buffer).to_physical(),
                cells_layout.size(),
            )
            .to_virtual()
            .to_ptr::<Option<TextCell>>();

            for i in old_len..new_len {
                ptr::write(ptr.add(i), None);
            }

            self.buffer = slice::from_raw_parts_mut(ptr, new_len);
        }
        self.max_lines *= 2;
    }
}

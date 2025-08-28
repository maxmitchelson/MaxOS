use core::ops::Range;

const ESC: char = '\x1b';
const BRACKET: char = '\x5b';

const PARAM_RANGE: Range<char> = '\x30'..'\x40';
const INTERMEDIATE_RANGE: Range<char> = '\x20'..'\x30';
const FINAL_RANGE: Range<char> = '\x40'..'\u{80}';

const BUFFER_SIZE: usize = 20;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum EraseMode {
    BeforeCursor,
    AfterCursor,
    All,
}

/// ANSI-compatible color codes.
/// * [`AnsiColor::ColorCode`] describes one of the 16 theme colors.
/// * [`AnsiColor::Rgb`] is a pure RGB value.
/// * [`AnsiColor::Reset`] describes a reset to the default theme color for the current element.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AnsiColor {
    ColorCode(u8),
    Rgb(u8, u8, u8),
    DefaultForeground,
    DefaultBackground,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AnsiCommand {
    CursorMoveAbsolute {
        line: usize,
        column: usize,
    },
    CursorMoveRelative {
        line: isize,
        column: isize,
    },
    CursorMoveColumnAbsolute(usize),
    EraseDisplay {
        mode: EraseMode,
        preserve_offscreen: bool,
    },
    EraseLine(EraseMode),
    ScrollRelative(isize),
    SetBackground(AnsiColor),
    SetForeground(AnsiColor),
    ResetGraphicRendition,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum AnsiError {
    Unsupported,
    InvalidParameters,
    BufferOverflow,
}

/// The current decoding stage of the [`AnsiHandler`].
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum AnsiStage {
    Escape,
    CtrlSequenceIdentifier,
    Parameters,
    Intermediate,
    Final,
}

impl AnsiStage {
    pub fn in_char_range(&self, ch: &char) -> bool {
        match self {
            AnsiStage::Escape => *ch == ESC,
            AnsiStage::CtrlSequenceIdentifier => *ch == BRACKET,
            AnsiStage::Parameters => PARAM_RANGE.contains(ch),
            AnsiStage::Intermediate => INTERMEDIATE_RANGE.contains(ch),
            AnsiStage::Final => FINAL_RANGE.contains(ch),
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            AnsiStage::Escape => Some(AnsiStage::CtrlSequenceIdentifier),
            AnsiStage::CtrlSequenceIdentifier => Some(AnsiStage::Parameters),
            AnsiStage::Parameters => Some(AnsiStage::Intermediate),
            AnsiStage::Intermediate => Some(AnsiStage::Final),
            AnsiStage::Final => None,
        }
    }
}

pub enum ParserResult {
    Valid(AnsiCommand),
    Incomplete,
    Error(AnsiError),
}

/// Handler for ANSI sequences. Keeps a buffer of parameters given as input for partial writes and
/// manages the state machine that dictates the current decoding stage.
pub struct AnsiHandler {
    buffer: [u8; BUFFER_SIZE],
    ptr: usize,
    stage: AnsiStage,
}

impl AnsiHandler {
    pub fn new() -> Self {
        Self {
            buffer: [0; 20],
            ptr: 0,
            stage: AnsiStage::Escape,
        }
    }

    /// Parse the ANSI sequence at the beginning of the provided string.
    ///
    /// If the provided command is valid and supported, returns a [`ParserResult::Valid`].
    /// In that case, the `unprocessed` &str corresponds to the remaining string after the ANSI sequence.
    ///
    /// If the provided command is incomplete but not yet identified as invalid or unsupported,
    /// returns [`ParserResult::Incomplete`]. In this case, further calls to continue_parse will
    /// attempt to continue the parsing from the current state.
    ///
    /// In the case that the command is invalid or unsupported, returns a [`ParserResult::Error`].
    pub fn continue_parse(&mut self, s: impl IntoIterator<Item = char>) -> ParserResult {
        let mut iterator = s.into_iter();
        let mut element = iterator.next();
        loop {
            if element.is_none() {
                break;
            }
            let ch = element.unwrap();

            match self.stage {
                AnsiStage::Escape | AnsiStage::CtrlSequenceIdentifier | AnsiStage::Final => {
                    if self.stage.in_char_range(&ch) {
                        if let Some(next) = self.stage.next() {
                            self.stage = next;
                        } else {
                            let result = self.parse_final(ch);
                            match result {
                                Ok(command) => return ParserResult::Valid(command),
                                Err(error) => return ParserResult::Error(error),
                            }
                        }
                    } else {
                        return ParserResult::Error(AnsiError::Unsupported);
                    }
                }
                AnsiStage::Parameters => {
                    if self.stage.in_char_range(&ch) {
                        if ch.len_utf8() != 1 {
                            return ParserResult::Error(AnsiError::InvalidParameters);
                        }
                        if self.ptr + ch.len_utf8() == BUFFER_SIZE {
                            return ParserResult::Error(AnsiError::BufferOverflow);
                        }
                        let bytes = ch.encode_utf8(&mut self.buffer[self.ptr..]);
                        self.ptr += 1;
                    } else {
                        self.stage = self.stage.next().unwrap();
                        continue;
                    }
                }
                AnsiStage::Intermediate => {
                    if self.stage.in_char_range(&ch) {
                        return ParserResult::Error(AnsiError::Unsupported);
                    } else {
                        self.stage = self.stage.next().unwrap();
                        continue;
                    }
                }
            };

            element = iterator.next();
        }
        ParserResult::Incomplete
    }

    /// Parses the ANSI sequence using data in `self.buffer` after having received the `final_char`
    /// that marks the end of the sequence.
    fn parse_final(&mut self, final_char: char) -> Result<AnsiCommand, AnsiError> {
        let s = str::from_utf8(&self.buffer).unwrap();
        let mut params = [0; 5];
        let mut n_params = 0;

        let s = s.trim_end_matches('\0');
        if s.is_empty() {
            n_params = 0;
        } else {
            for part in s.split(';') {
                if n_params == params.len() {
                    return Err(AnsiError::Unsupported);
                }
                if part.is_empty() {
                    params[n_params] = 0;
                } else {
                    params[n_params] = part.parse().unwrap();
                }
                n_params += 1;
            }
        }

        match final_char {
            'm' => parse_sgr(n_params, &params),
            'J' => parse_erase_display(n_params, &params),
            'K' => parse_erase_line(n_params, &params),
            'A' => parse_move_cursor_relative(n_params, &params, Direction::Up),
            'B' => parse_move_cursor_relative(n_params, &params, Direction::Down),
            'C' => parse_move_cursor_relative(n_params, &params, Direction::Right),
            'D' => parse_move_cursor_relative(n_params, &params, Direction::Left),
            'H' | 'f' => parse_move_cursor_absolute(n_params, &params),
            'G' => parse_move_cursor_column(n_params, &params),
            'S' => parse_scroll(n_params, &params, Direction::Up),
            'T' => parse_scroll(n_params, &params, Direction::Down),
            _ => Err(AnsiError::Unsupported),
        }
    }

    /// Returns true if in the process of parsing a sequence, false otherwise.
    pub fn is_active(&self) -> bool {
        self.stage != AnsiStage::Escape
    }

    /// Reset the parser, its buffer and its processing stage.
    pub fn reset(&mut self) {
        self.buffer.fill(0);
        self.ptr = 0;
        self.stage = AnsiStage::Escape;
    }

    pub fn try_start(&mut self) {
        if self.stage == AnsiStage::Escape {
            self.stage = AnsiStage::CtrlSequenceIdentifier;
        }
    }
}

fn parse_rgb_sgr(zone: i32, r: i32, g: i32, b: i32) -> Result<AnsiCommand, AnsiError> {
    let r: u8 = u8::try_from(r).map_err(|_| AnsiError::InvalidParameters)?;
    let g: u8 = u8::try_from(g).map_err(|_| AnsiError::InvalidParameters)?;
    let b: u8 = u8::try_from(b).map_err(|_| AnsiError::InvalidParameters)?;

    match zone {
        38 => Ok(AnsiCommand::SetForeground(AnsiColor::Rgb(r, g, b))),
        48 => Ok(AnsiCommand::SetBackground(AnsiColor::Rgb(r, g, b))),
        _ => Err(AnsiError::InvalidParameters),
    }
}
fn parse_256_sgr(zone: i32, color_code: i32) -> Result<AnsiCommand, AnsiError> {
    let color_code = u8::try_from(color_code).map_err(|_| AnsiError::InvalidParameters)?;
    let color = match color_code {
        0..16 => AnsiColor::ColorCode(color_code),
        16..232 => {
            let map = [0, 95, 135, 175, 215, 255];
            let val = color_code - 16;
            let r = (val / 36) as usize;
            let g = ((val / 6) % 6) as usize;
            let b = (val % 6) as usize;

            AnsiColor::Rgb(map[r], map[g], map[b])
        }
        232.. => {
            let l: u8 = 8 + (color_code - 232) * 10;
            AnsiColor::Rgb(l, l, l)
        }
    };

    match zone {
        38 => Ok(AnsiCommand::SetForeground(color)),
        48 => Ok(AnsiCommand::SetBackground(color)),
        _ => Err(AnsiError::InvalidParameters),
    }
}

fn parse_16_sgr(color_code: i32) -> Result<AnsiCommand, AnsiError> {
    let color_code = u8::try_from(color_code).map_err(|_| AnsiError::InvalidParameters)?;
    match color_code {
        0 => Ok(AnsiCommand::ResetGraphicRendition),
        30..38 => Ok(AnsiCommand::SetForeground(AnsiColor::ColorCode(
            color_code - 30,
        ))),
        40..48 => Ok(AnsiCommand::SetBackground(AnsiColor::ColorCode(
            color_code - 40,
        ))),
        90..98 => Ok(AnsiCommand::SetForeground(AnsiColor::ColorCode(
            color_code - 90 + 8,
        ))),
        100..108 => Ok(AnsiCommand::SetBackground(AnsiColor::ColorCode(
            color_code - 100 + 8,
        ))),
        _ => Err(AnsiError::InvalidParameters),
    }
}

fn parse_sgr(n_params: usize, params: &[i32]) -> Result<AnsiCommand, AnsiError> {
    match (params[1], n_params) {
        (_, 0) => Ok(AnsiCommand::ResetGraphicRendition),
        (2, 4 | 5) => parse_rgb_sgr(params[0], params[2], params[3], params[4]),
        (5, 2 | 3) => parse_256_sgr(params[0], params[2]),
        (_, 1) => parse_16_sgr(params[0]),
        _ => Err(AnsiError::InvalidParameters),
    }
}

fn parse_erase_display(n_params: usize, params: &[i32]) -> Result<AnsiCommand, AnsiError> {
    if n_params > 1 {
        return Err(AnsiError::InvalidParameters);
    }

    match params[0] {
        0 => Ok(AnsiCommand::EraseDisplay {
            mode: EraseMode::AfterCursor,
            preserve_offscreen: true,
        }),
        1 => Ok(AnsiCommand::EraseDisplay {
            mode: EraseMode::BeforeCursor,
            preserve_offscreen: true,
        }),
        2 => Ok(AnsiCommand::EraseDisplay {
            mode: EraseMode::All,
            preserve_offscreen: true,
        }),
        3 => Ok(AnsiCommand::EraseDisplay {
            mode: EraseMode::All,
            preserve_offscreen: false,
        }),
        _ => Err(AnsiError::InvalidParameters),
    }
}

fn parse_erase_line(n_params: usize, params: &[i32]) -> Result<AnsiCommand, AnsiError> {
    if n_params > 1 {
        return Err(AnsiError::InvalidParameters);
    }

    match params[0] {
        0 => Ok(AnsiCommand::EraseLine(EraseMode::AfterCursor)),
        1 => Ok(AnsiCommand::EraseLine(EraseMode::BeforeCursor)),
        2 => Ok(AnsiCommand::EraseLine(EraseMode::All)),
        _ => Err(AnsiError::InvalidParameters),
    }
}

fn parse_move_cursor_relative(
    n_params: usize,
    params: &[i32],
    direction: Direction,
) -> Result<AnsiCommand, AnsiError> {
    if n_params > 1 {
        return Err(AnsiError::InvalidParameters);
    }

    let distance = params[0] as isize;
    match direction {
        Direction::Up => Ok(AnsiCommand::CursorMoveRelative {
            line: -distance,
            column: 0,
        }),
        Direction::Down => Ok(AnsiCommand::CursorMoveRelative {
            line: distance,
            column: 0,
        }),
        Direction::Left => Ok(AnsiCommand::CursorMoveRelative {
            line: 0,
            column: -distance,
        }),
        Direction::Right => Ok(AnsiCommand::CursorMoveRelative {
            line: 0,
            column: distance,
        }),
    }
}

fn parse_move_cursor_absolute(n_params: usize, params: &[i32]) -> Result<AnsiCommand, AnsiError> {
    if n_params > 2 {
        Err(AnsiError::Unsupported)
    } else {
        let line = usize::try_from(params[0]).map_err(|_| AnsiError::InvalidParameters)?;
        let column = usize::try_from(params[1]).map_err(|_| AnsiError::InvalidParameters)?;
        Ok(AnsiCommand::CursorMoveAbsolute { line, column })
    }
}

fn parse_move_cursor_column(n_params: usize, params: &[i32]) -> Result<AnsiCommand, AnsiError> {
    if n_params > 1 {
        Err(AnsiError::InvalidParameters)
    } else {
        let col = usize::try_from(params[0]).map_err(|_| AnsiError::InvalidParameters)?;
        Ok(AnsiCommand::CursorMoveColumnAbsolute(col))
    }
}

fn parse_scroll(
    n_params: usize,
    params: &[i32],
    direction: Direction,
) -> Result<AnsiCommand, AnsiError> {
    if n_params > 1 {
        return Err(AnsiError::InvalidParameters);
    }

    let distance = if n_params == 0 { 1 } else { params[0] } as isize;
    match direction {
        Direction::Up => Ok(AnsiCommand::ScrollRelative(-distance)),
        Direction::Down => Ok(AnsiCommand::ScrollRelative(distance)),
        _ => Err(AnsiError::InvalidParameters),
    }
}

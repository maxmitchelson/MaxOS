use core::fmt;
use fmt::Write;
use crate::terminal::{tty::TerminalStdin};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Critical = 4,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

        let mut stdin = TerminalStdin::new();
        let _ = writeln!(stdin, "[{:#}]: {}", level, message);
    }

    pub fn log_args(&self, level: LogLevel, message: fmt::Arguments) {
        if level < self.level {
            return;
        }

        let mut stdin = TerminalStdin::new();
        let _ = writeln!(stdin, "[{:#}]: {}", level, message);
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

    pub fn debug_args(&self, message: fmt::Arguments) {
        self.log_args(LogLevel::Debug, message);
    }

    pub fn info_args(&self, message: fmt::Arguments) {
        self.log_args(LogLevel::Info, message);
    }

    pub fn warn_args(&self, message: fmt::Arguments) {
        self.log_args(LogLevel::Warn, message);
    }

    pub fn error_args(&self, message: fmt::Arguments) {
        self.log_args(LogLevel::Error, message);
    }

    pub fn critical_args(&self, message: fmt::Arguments) {
        self.log_args(LogLevel::Critical, message);
    }
}



macro_rules! debug {
    ($($arg:tt)*) => {{
        $crate::LOGGER.debug_args(format_args!($($arg)*));
    }};
}

macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::LOGGER.info_args(format_args!($($arg)*));
    }};
}

macro_rules! warning {
    ($($arg:tt)*) => {{
        $crate::LOGGER.warn_args(format_args!($($arg)*));
    }};
}

macro_rules! error {
    ($($arg:tt)*) => {{
        $crate::LOGGER.error_args(format_args!($($arg)*));
    }};
}

macro_rules! critical {
    ($($arg:tt)*) => {{
        $crate::LOGGER.critical_args(format_args!($($arg)*));
    }};
}

pub(crate) use debug;
pub(crate) use info;
pub(crate) use warning;
pub(crate) use error;
pub(crate) use critical;

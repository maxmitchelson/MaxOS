mod implementation;
pub mod logger;
mod themes;

pub use implementation::*;

macro_rules! print {
    ($($arg:tt)*) => {{
        let mut writer = $crate::terminal::TerminalWriter::new();
        use core::fmt::Write;
        let _ = write!(writer, $($arg)*);
    }};
}

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

pub(crate) use print;
pub(crate) use println;

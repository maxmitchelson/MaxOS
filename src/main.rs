#![no_std]
#![no_main]

mod framebuffer;
mod limine;
mod terminal;

use core::arch::asm;
use core::panic::PanicInfo;

use spin::{Lazy, RwLock};

use crate::framebuffer::Framebuffer;
use crate::terminal::Terminal;

static FRAMEBUFFER: Lazy<RwLock<Framebuffer>> = Lazy::new(|| RwLock::new(limine::get_framebuffer()));
static TERMINAL: Lazy<RwLock<Terminal>> = Lazy::new(|| RwLock::new(Terminal::new(&FRAMEBUFFER)));

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    limine::ensure_base_revision_support();

    println!("Hello MaxOS!");

    halt();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        print!(
            "Panic at {}:{}: {} \n",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        print!("Panic: {} \n", info.message())
    }
    halt()
}

fn halt() -> ! {
    loop {
        // loop over instruction in case CPU retakes control
        unsafe {
            asm!("hlt");
        }
    }
}

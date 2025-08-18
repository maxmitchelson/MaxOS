#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod cpu;
mod drivers;
mod limine;
mod memory;
mod terminal;

use core::arch::asm;
use core::panic::PanicInfo;

use crate::terminal::logger::{self, LogLevel, Logger};
pub static LOGGER: Logger = Logger::new(LogLevel::Debug);

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    limine::init();
    cpu::interrupts::init();
    drivers::framebuffer::init();
    memory::frame_allocator::init();
    terminal::init();

    logger::info!("Initialization sequence over!");

    for i in 0..100 {
        logger::info!("{}", i);
    }

    logger::info!("Exit!");

    halt();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        logger::critical!(
            "Panic at {}:{}: {} \n",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        logger::critical!("Panic: {} \n", info.message())
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

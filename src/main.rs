#![no_std]
#![no_main]
#![allow(unused)] // temporary fix to get rid of diagnostic spam

mod framebuffer;
mod limine;
mod memory;
mod terminal;

use core::arch::asm;
use core::fmt::Write;
use core::ops::Deref;
use core::panic::PanicInfo;

use spin::{Lazy, RwLock};
use terminal::TerminalDriver;

use crate::framebuffer::Framebuffer;
use crate::memory::frame_allocator;
use crate::terminal::Terminal;

static FRAMEBUFFER: Lazy<RwLock<Framebuffer>> =
    Lazy::new(|| RwLock::new(limine::get_framebuffer()));
static TERMINAL: Lazy<TerminalDriver> =
    Lazy::new(|| TerminalDriver(RwLock::new(Terminal::new(&FRAMEBUFFER))));

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    limine::ensure_base_revision_support();

    println!("Hello MaxOS!");
    println!("HHDM offset: {:#X}", *limine::HHDM_OFFSET);

    let mut fralloc = frame_allocator::FrameAllocator::new();

    let frame = fralloc.allocate();
    let frame = unsafe { &mut *frame.to_virtual().to_ptr::<[u8; 4096]>() };

    for byte in &mut *frame {
        *byte = 1;
    }

    println!("{}", frame.iter().map(|x| *x as u64).sum::<u64>());

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

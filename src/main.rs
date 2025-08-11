#![no_std]
#![no_main]

mod framebuffer;
mod limine;
mod memory;
mod terminal;

use core::arch::asm;
use core::panic::PanicInfo;

use memory::frame_allocator::BuddyAllocator;
use spin::{Lazy, RwLock};
use terminal::TerminalDriver;

use crate::framebuffer::Framebuffer;
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

    let balloc = BuddyAllocator::new_embedded(*limine::BOOT_MEMORY_MAP).unwrap();
    let frame = balloc.allocate(4096);
    let frame = unsafe { &mut *frame.to_virtual().to_ptr::<[u8; 4096]>() };

    println!("test");
    for byte in &mut *frame {
        *byte = 1;
    }

    println!("{}", frame.iter().map(|x| *x as u64).sum::<u64>());

    println!("Exit!");
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

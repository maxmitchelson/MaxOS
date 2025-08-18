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

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    limine::init();
    cpu::interrupts::init();
    drivers::framebuffer::init();
    terminal::init();
    memory::frame_allocator::init();

    println!("Hello MaxOS!");
    println!("HHDM offset: {:#X}", limine::hhdm_offset());
    println!("Exit!");

    // unsafe {
    //     asm!("int 0");
    // };

    // Terrible testing code, this behavior should be made much MUCH easier
    // let pt = memory::paging::get_active_level_4_table();
    //
    // 'out: for (i, third) in pt.entries_mut().enumerate() {
    //     if third.is_unused() {
    //         continue;
    //     }
    //     let third = unsafe { &mut *third.address().to_virtual().to_ptr::<PageTable>() };
    //     for (j, second) in third.entries_mut().enumerate() {
    //         if second.is_unused() {
    //             continue;
    //         }
    //         let second = unsafe { &mut *second.address().to_virtual().to_ptr::<PageTable>() };
    //         for (k, first) in second.entries_mut().enumerate() {
    //             if first.is_unused() {
    //                 continue;
    //             }
    //             let first = unsafe { &mut *first.address().to_virtual().to_ptr::<PageTable>() };
    //             for (l, page_table) in first.entries_mut().enumerate() {
    //                 if page_table.is_unused() {
    //                     page_table.set_flags(PageTableEntryFlags::PRESENT);
    //                     let addr = VirtualAddress::from(VirtualAddress::sign_extend_value(
    //                         i << 39 | j << 30 | k << 21 | l << 12,
    //                     ));
    //                     let x = unsafe { &mut *addr.to_ptr::<u8>() };
    //                     *x = 5;
    //                     println!("out");
    //                     break 'out;
    //                 }
    //             }
    //         }
    //     }
    // }

    // println!("Initializing frame allocator");
    // frame_allocator::init();
    // println!("Allocating single frame");
    // let frame = frame_allocator::allocate(4096);
    // let frame = unsafe { &mut *frame.to_virtual().to_ptr::<[u8; 4096]>() };
    //
    // println!("Writing to allocated frame");
    // for byte in &mut *frame {
    //     *byte = 1;
    // }
    //
    // println!("Stress testing the frame allocator");
    // frame_allocator::with_allocator(|a| a.stress());

    halt();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let _ = $crate::terminal::_WRITER.get().unwrap().write_to_terminal(format_args!($($arg)*));
    }};
}

#[macro_export]
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

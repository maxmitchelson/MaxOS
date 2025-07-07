#![no_std]
#![no_main]

mod frambuffer;
mod limine;

use core::arch::asm;
use core::panic::PanicInfo;

use crate::frambuffer::RGB;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    limine::ensure_base_revision_support();
    let mut framebuffer = limine::get_framebuffer();


    let h = framebuffer.info.height / 256;
    let gap = (framebuffer.info.height - (h * 256)) / 2;

    for x in 0..framebuffer.info.width {
        for y in 0..gap {
            framebuffer.set_pixel_value(x, y, RGB::from_hex(0x000000));
            framebuffer.set_pixel_value(
                x,
                framebuffer.info.height - y - 1,
                RGB::from_hex(0xFFFFFF),
            );
        }
    }

    for i in 0..256 {
        for x in 0..framebuffer.info.width {
            for y in i * h..(i + 1) * h {
                let color = i as u8;
                framebuffer.set_pixel_value(x, y + gap, RGB::new(color, color, color));
            }
        }
    }

    halt();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
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

#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;
// use core::ptr::NonNull;
//
// use acpi::fadt::Fadt;
// use acpi::{AcpiHandler, AcpiTables};
use limine::BaseRevision;
use limine::request::{
    FramebufferRequest, /* HhdmRequest, */ RequestsEndMarker, RequestsStartMarker, RsdpRequest,
};

macro_rules! limine_requests {
    ($($item:item)*) => {
        $(
            #[used]
            #[unsafe(link_section = ".limine_requests")]
            $item
        )*
    };
}

limine_requests! {
    static BASE_REVISION: BaseRevision = BaseRevision::new();
    static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();
    // static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();
    static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
}

// Define the start and end markers for Limine requests
#[used]
#[unsafe(link_section = ".limine_requests_start")]
static _LIMINE_REQUESTS_START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".limine_requests_end")]
static _LIMINE_REQUESTS_END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    assert!(BASE_REVISION.is_supported());

    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
            for i in 10..200_u64 {
                for j in 0..30 {
                    let pixel_offset = i * framebuffer.pitch() + (i + j - 3) * 4;
                    unsafe {
                        framebuffer
                            .addr()
                            .add(pixel_offset as usize)
                            .cast::<u32>()
                            .write(0xFFFFFFFF)
                    }
                }
            }
        }
    }

    halt();
}

// #[derive(Clone)]
// struct CustomAcpiHandler {
//     hhdm_offset: usize,
// }
//
// impl CustomAcpiHandler {
//     fn new(offset: u64) -> Self {
//         Self {hhdm_offset: offset as usize}
//     }
// }
//
// impl AcpiHandler for CustomAcpiHandler {
//     unsafe fn map_physical_region<T>(
//         &self,
//         physical_address: usize,
//         size: usize,
//     ) -> acpi::PhysicalMapping<Self, T> {
//         let virt = NonNull::new((physical_address + self.hhdm_offset) as *mut T).unwrap();
//         unsafe { 
//             acpi::PhysicalMapping::new(
//                 physical_address, 
//                 virt,
//                 size,
//                 size,
//                 self.clone(),
//             ) 
//         }
//     }
//
//     fn unmap_physical_region<T>(_region: &acpi::PhysicalMapping<Self, T>) {
//         // nothing to do, memory is permanently mapped
//     }
// }
//
// fn shutdown() {
//     let hhdm_offset = HHDM_REQUEST.get_response().unwrap().offset();
//     let acpi_handler = CustomAcpiHandler::new(hhdm_offset)
//
//     if let Some(rsdp_response) = RSDP_REQUEST.get_response() {
//
//         let t = unsafe {
//             AcpiTables::from_rsdp(acpi_handler, rsdp_response.address())
//         }.unwrap();
//
//
//         let fadt = t.find_table::<Fadt>().unwrap().get();
//     }
// }

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

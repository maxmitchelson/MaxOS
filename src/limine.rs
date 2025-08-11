use crate::framebuffer::{Framebuffer, FramebufferInfo};
use core::slice;
use limine::framebuffer::MemoryModel;
use limine::request::{FramebufferRequest, HhdmRequest, MemoryMapRequest};
use limine::request::{RequestsEndMarker, RequestsStartMarker};
use limine::{BaseRevision, memory_map};
use spin::Lazy;

/// Marks one or more static Limine bootloader request items to be placed in the
/// `.limine_requests` section of the binary.
///
/// This macro automatically applies the required attributes:
/// - `#[used]` to prevent the linker from discarding the symbol
/// - `#[link_section = ".limine_requests"]` (via `#[unsafe(...)]`) to ensure it is
///   placed in the section the Limine bootloader expects.
macro_rules! limine_request {
    ($($item:item)*) => {
        $(
            #[used]
            #[unsafe(link_section = ".limine_requests")]
            $item
        )*
    };
}

/// Marker for the beginning of the Limine bootloader request section
#[used]
#[unsafe(link_section = ".limine_requests_start")]
static _LIMINE_REQUESTS_START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

/// Marker for the end of the Limine bootloader request section
#[used]
#[unsafe(link_section = ".limine_requests_end")]
static _LIMINE_REQUESTS_END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

limine_request! {
    static BASE_REVISION: BaseRevision = BaseRevision::with_revision(3);
    static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();
    static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
    static MMAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
}

pub static HHDM_OFFSET: Lazy<usize> = Lazy::new(get_hhdm_offset);

/// The initial memory map provided by the bootloader.
///
/// Limine guarantees:
/// - Entries are sorted in increasing order of [`base`](memory_map::Entry::base) address.
/// - [`USABLE`](memory_map::EntryType::USABLE) and
///   [`BOOTLOADER_RECLAIMABLE`](memory_map::EntryType::BOOTLOADER_RECLAIMABLE) regions:
///   - Are non-overlapping.
///   - Have [`base`](memory_map::Entry::base) and [`length`](memory_map::Entry::length)
///     aligned to 4 KiB.
/// - No alignment or overlap guarantees are made for other [`EntryType`](memory_map::EntryType)
///   variants
pub static BOOT_MEMORY_MAP: Lazy<BootMemoryMap> = Lazy::new(get_memory_map);

pub fn ensure_base_revision_support() {
    assert!(BASE_REVISION.is_valid());
    assert!(BASE_REVISION.is_supported());
}

fn get_hhdm_offset() -> usize {
    HHDM_REQUEST.get_response().unwrap().offset() as usize
}

#[derive(Copy, Clone)]
pub struct BootMemoryMap(&'static [&'static memory_map::Entry]);

impl BootMemoryMap {
    pub fn entries(&self) -> &'static [&'static memory_map::Entry] {
        self.0
    }

    pub fn usable_entries(&self) -> impl DoubleEndedIterator<Item = &&'static memory_map::Entry> {
        self.0.iter().filter(|e| e.entry_type == memory_map::EntryType::USABLE)
    }
}

fn get_memory_map() -> BootMemoryMap {
    let mmap_response = MMAP_REQUEST.get_response().unwrap();
    BootMemoryMap(mmap_response.entries())
}

pub fn get_framebuffer() -> Framebuffer {
    let limine_framebuffers = FRAMEBUFFER_REQUEST.get_response().unwrap().framebuffers();

    for buffer in limine_framebuffers {
        let info = FramebufferInfo::from_limine_framebuffer(&buffer);
        let size = info.pitch * info.height;

        if buffer.memory_model() != MemoryModel::RGB || info.bytes_per_pixel != 4 {
            continue; // incompatible pixel layout
        }

        let buffer_slice = unsafe { slice::from_raw_parts_mut(buffer.addr() as *mut u32, size) };
        return Framebuffer {
            info,
            buffer: buffer_slice,
        };
    }

    panic!("No valid framebuffer found");
}

use crate::framebuffer::{Framebuffer, FramebufferInfo};
use core::slice;
use core::sync::atomic::AtomicBool;
use limine::framebuffer::MemoryModel;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker,
};
use limine::{BaseRevision, memory_map};

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

static mut HHDM_OFFSET: usize = 0;

pub fn init() {
    assert!(BASE_REVISION.is_valid());
    assert!(BASE_REVISION.is_supported());

    init_hhdm_offset();
}

fn init_hhdm_offset() {
    unsafe { HHDM_OFFSET = HHDM_REQUEST.get_response().unwrap().offset() as usize }
}

pub const fn hhdm_offset() -> usize {
    unsafe { HHDM_OFFSET }
}

#[derive(Copy, Clone)]
pub struct MemoryMap(&'static [&'static memory_map::Entry]);

impl MemoryMap {
    pub fn entries(&self) -> &'static [&'static memory_map::Entry] {
        self.0
    }

    pub fn usable_entries(&self) -> impl DoubleEndedIterator<Item = &&'static memory_map::Entry> {
        self.0
            .iter()
            .filter(|e| e.entry_type == memory_map::EntryType::USABLE)
    }
}

/// Obtain the initial memory map provided by the bootloader.
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
pub fn acquire_memory_map() -> Option<MemoryMap> {
    static MOVED: AtomicBool = AtomicBool::new(false);
    if MOVED.load(core::sync::atomic::Ordering::Acquire) {
        None
    } else {
        MOVED.store(true, core::sync::atomic::Ordering::Release);
        let mmap_response = MMAP_REQUEST.get_response().unwrap();
        Some(MemoryMap(mmap_response.entries()))
    }
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

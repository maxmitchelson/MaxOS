use core::arch::asm;
use core::fmt::Debug;
use core::marker::PhantomData;

use crate::cpu::interrupts::{
    DivergingHandler, DivergingHandlerWithError, Handler, HandlerWithError, PageFaultError,
    SegmentSelectorError,
};
use crate::cpu::segments;
use crate::cpu::{DescriptorTablePointer, PrivilegeLevel};
use crate::memory::VirtualAddress;

#[repr(C)]
#[derive(Debug)]
pub(super) struct InterruptDescriptorTable {
    pub(super) divide_error: Descriptor<Handler>,
    pub(super) debug: Descriptor<Handler>,
    pub(super) non_maskable_interrupt: Descriptor<Handler>,
    pub(super) breakpoint: Descriptor<Handler>,
    pub(super) overflow: Descriptor<Handler>,
    pub(super) bound_range_exceeded: Descriptor<Handler>,
    pub(super) invalid_opcode: Descriptor<Handler>,
    pub(super) device_not_available: Descriptor<Handler>,
    pub(super) double_fault: Descriptor<DivergingHandlerWithError<usize>>,
    _coprocessor_segment_overrun: Reserved,
    pub(super) invalid_tss: Descriptor<HandlerWithError<SegmentSelectorError>>,
    pub(super) segment_not_present: Descriptor<HandlerWithError<SegmentSelectorError>>,
    pub(super) stack_segment_fault: Descriptor<HandlerWithError<SegmentSelectorError>>,
    pub(super) general_protection_fault: Descriptor<HandlerWithError<SegmentSelectorError>>,
    pub(super) page_fault: Descriptor<HandlerWithError<PageFaultError>>,
    _reserved_0: Reserved,
    pub(super) x87_floating_point_exception: Descriptor<Handler>,
    pub(super) alignment_check: Descriptor<HandlerWithError<usize>>,
    pub(super) machine_check: Descriptor<DivergingHandler>,
    pub(super) simd_floating_point: Descriptor<Handler>,
    pub(super) virtualization_exception: Descriptor<Handler>,
    pub(super) control_protection_exception: Descriptor<HandlerWithError<usize>>,
    _reserved_10: [Reserved; 10],
    pub(super) _available: [Descriptor<Handler>; 256 - 32],
}

impl InterruptDescriptorTable {
    pub(super) const fn new() -> Self {
        Self {
            divide_error: Descriptor::missing(),
            debug: Descriptor::missing(),
            non_maskable_interrupt: Descriptor::missing(),
            breakpoint: Descriptor::missing(),
            overflow: Descriptor::missing(),
            bound_range_exceeded: Descriptor::missing(),
            invalid_opcode: Descriptor::missing(),
            device_not_available: Descriptor::missing(),
            double_fault: Descriptor::missing(),
            _coprocessor_segment_overrun: Reserved::new(),
            invalid_tss: Descriptor::missing(),
            segment_not_present: Descriptor::missing(),
            stack_segment_fault: Descriptor::missing(),
            general_protection_fault: Descriptor::missing(),
            page_fault: Descriptor::missing(),
            _reserved_0: Reserved::new(),
            x87_floating_point_exception: Descriptor::missing(),
            alignment_check: Descriptor::missing(),
            machine_check: Descriptor::missing(),
            simd_floating_point: Descriptor::missing(),
            virtualization_exception: Descriptor::missing(),
            control_protection_exception: Descriptor::missing(),
            _reserved_10: [Reserved::new(); 10],
            _available: [Descriptor::missing(); 256 - 32],
        }
    }

    /// SAFETY: Callers must ensure that the provided pointer is valid as long as the table is loaded
    pub(super) unsafe fn load(table: *const Self) {
        let idt_ptr = &DescriptorTablePointer {
            limit: (size_of::<Self>() - 1) as u16,
            base: VirtualAddress::from_ptr(table),
        };

        unsafe {
            asm!("lidt [{}]", in(reg) idt_ptr, options(readonly, nostack, preserves_flags));
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
struct Reserved(Descriptor<Self>);

impl Reserved {
    const fn new() -> Self {
        Self(Descriptor::missing())
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Descriptor<RoutineType> {
    address_1: u16,
    selector: segments::SegmentSelector,
    attributes: Attributes,
    address_2: u16,
    address_3: u32,
    zero: u32,
    _routine_type: PhantomData<RoutineType>,
}

impl<T> Descriptor<T> {
    pub(super) const fn missing() -> Self {
        Self {
            address_1: 0,
            selector: segments::selectors::CODE,
            attributes: Attributes::missing(),
            address_2: 0,
            address_3: 0,
            zero: 0,
            _routine_type: PhantomData,
        }
    }

    fn set_handler_address(&mut self, address: usize) -> &mut Attributes {
        self.address_1 = (address & 0xFFFF) as u16;
        self.address_2 = ((address >> 16) & 0xFFFF) as u16;
        self.address_3 = ((address >> 32) & 0xFFFF_FFFF) as u32;
        self.attributes.set_present();
        &mut self.attributes
    }

    pub(super) fn address(&self) -> VirtualAddress {
        VirtualAddress::from(
            self.address_1 as usize
                | (self.address_2 as usize) << 16
                | (self.address_3 as usize) << 32,
        )
    }
}

impl Descriptor<Handler> {
    #[inline]
    pub(super) fn set_handler(&mut self, handler: Handler) -> &mut Attributes {
        self.set_handler_address(handler as usize)
    }
}

impl<T> Descriptor<HandlerWithError<T>> {
    #[inline]
    pub(super) fn set_handler(&mut self, handler: HandlerWithError<T>) -> &mut Attributes {
        self.set_handler_address(handler as usize)
    }
}

impl Descriptor<DivergingHandler> {
    #[inline]
    pub(super) fn set_handler(&mut self, handler: DivergingHandler) -> &mut Attributes {
        self.set_handler_address(handler as usize)
    }
}

impl<T> Descriptor<DivergingHandlerWithError<T>> {
    #[inline]
    pub(super) fn set_handler(&mut self, handler: DivergingHandlerWithError<T>) -> &mut Attributes {
        self.set_handler_address(handler as usize)
    }
}

// impl_set_handler!(AbortISR);
// impl_set_handler!(AbortISRWithError);

impl<T> Debug for Descriptor<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Descriptor")
            .field("address", &self.address())
            .field("segment selector", &self.selector)
            .field("attributes", &self.attributes)
            .field("type", &self._routine_type)
            .finish()
    }
}

#[repr(u8)]
#[derive(Debug)]
pub(super) enum GateType {
    Interrupt = 0x0E,
    Trap = 0x0F,
}

impl TryFrom<u8> for GateType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0E => Ok(Self::Interrupt),
            0x0F => Ok(Self::Trap),
            _ => Err(()),
        }
    }
}

#[repr(u8)]
#[derive(Debug)]
pub(super) enum Presence {
    Missing = 0,
    Present = 1,
}

impl TryFrom<u8> for Presence {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Missing),
            1 => Ok(Self::Present),
            _ => Err(()),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Attributes {
    interrupt_stack_table: u8,
    attributes: u8,
}

impl Attributes {
    pub(super) const fn missing() -> Self {
        Self {
            interrupt_stack_table: 0,
            attributes: Self::attributes_from(
                Presence::Missing,
                PrivilegeLevel::Ring0,
                GateType::Interrupt,
            ),
        }
    }

    pub(super) const fn from(privilege_level: PrivilegeLevel, gate_type: GateType) -> Self {
        Self {
            interrupt_stack_table: 0,
            attributes: Self::attributes_from(Presence::Present, privilege_level, gate_type),
        }
    }

    const fn attributes_from(
        presence: Presence,
        privilege_level: PrivilegeLevel,
        gate_type: GateType,
    ) -> u8 {
        ((presence as u8) << 7) | ((privilege_level as u8) << 5) | gate_type as u8
    }

    pub(super) fn set_present(&mut self) -> &mut Self {
        self.attributes |= (Presence::Present as u8) << 7;
        self
    }

    pub(super) fn set_missing(&mut self) -> &mut Self {
        self.attributes &= (Presence::Missing as u8) << 7;
        self
    }

    pub(super) fn set_privilege_level(&mut self, privilege_level: PrivilegeLevel) -> &mut Self {
        self.attributes = (self.attributes & !(0b11 << 5)) | ((privilege_level as u8) << 5);
        self
    }

    pub(super) fn set_gate_type(&mut self, gate_type: GateType) -> &mut Self {
        self.attributes = (self.attributes & !(0b1111)) | gate_type as u8;
        self
    }

    pub(super) fn status(&self) -> Presence {
        (self.attributes >> 7).try_into().unwrap()
    }

    pub(super) fn privilege_level(&self) -> PrivilegeLevel {
        ((self.attributes >> 5) & 0b11).try_into().unwrap()
    }

    pub(super) fn gate_type(&self) -> GateType {
        (self.attributes & 0b1111).try_into().unwrap()
    }
}

impl Debug for Attributes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("attributes")
            .field("status", &self.status())
            .field("stack table", &self.interrupt_stack_table)
            .field("gate type", &self.gate_type())
            .field("privilege level", &self.privilege_level())
            .finish()
    }
}

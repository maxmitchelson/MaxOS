use core::arch::asm;
use core::fmt::Debug;
use core::marker::PhantomData;

use crate::cpu::PrivilegeLevel;
use crate::cpu::interrupts::{Isr, IsrWithError};
use crate::cpu::segments;
use crate::memory::VirtualAddress;

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: VirtualAddress,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct InterruptDescriptorTable {
    pub(super) divide_error: Descriptor<Isr>,
    pub(super) debug: Descriptor<Isr>,
    pub(super) non_maskable_interrupt: Descriptor<Isr>,
    pub(super) breakpoint: Descriptor<Isr>,
    pub(super) overflow: Descriptor<Isr>,
    pub(super) bound_range_exceeded: Descriptor<Isr>,
    pub(super) invalid_opcode: Descriptor<Isr>,
    pub(super) device_not_available: Descriptor<Isr>,
    pub(super) double_fault: Descriptor<IsrWithError /*AbortISRWithError*/>,
    pub(super) coprocessor_segment_overrun: Descriptor<Isr>,
    pub(super) invalid_tss: Descriptor<IsrWithError>,
    pub(super) segment_not_present: Descriptor<IsrWithError>,
    pub(super) stack_segment_fault: Descriptor<IsrWithError>,
    pub(super) general_protection_fault: Descriptor<IsrWithError>,
    pub(super) page_fault: Descriptor<IsrWithError>,
    _reserved_0: Reserved,
    pub(super) x87_floating_point_exception: Descriptor<Isr>,
    pub(super) alignment_check: Descriptor<IsrWithError>,
    pub(super) machine_check: Descriptor<Isr /* AbortISR */>,
    pub(super) simd_floating_point: Descriptor<Isr>,
    pub(super) virtualization_exception: Descriptor<Isr>,
    pub(super) control_protection_exception: Descriptor<IsrWithError>,
    _reserved_10: [Reserved; 10],
    pub(super) _available: [Descriptor<Isr>; 256 - 32],
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
            coprocessor_segment_overrun: Descriptor::missing(),
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

macro_rules! impl_set_handler {
    ($h:ty) => {
        impl Descriptor<$h> {
            #[inline]
            pub(super) fn set_handler(&mut self, handler: $h) -> &mut Attributes {
                self.set_handler_address(handler as usize)
            }
        }
    };
}

impl_set_handler!(Isr);
impl_set_handler!(IsrWithError);
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

#[repr(u8)]
#[derive(Debug)]
pub(super) enum Presence {
    Present = 0x80,
    Missing = 0x00,
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
        presence as u8 | ((privilege_level as u8) << 5) | gate_type as u8
    }

    pub(super) fn set_present(&mut self) -> &mut Self {
        self.attributes |= Presence::Present as u8;
        self
    }

    pub(super) fn set_absent(&mut self) -> &mut Self{
        self.attributes &= !(Presence::Present as u8);
        self
    }

    pub(super) fn set_privilege_level(&mut self, privilege_level: PrivilegeLevel) -> &mut Self {
        self.attributes = self.attributes & 0b1000_1111 | ((privilege_level as u8) << 5);
        self
    }

    pub(super) fn set_gate_type(&mut self, gate_type: GateType) -> &mut Self {
        self.attributes = self.attributes & 0b1110_0000 | gate_type as u8;
        self
    }

    pub(super) fn status(&self) -> Presence {
        match (self.attributes & 0x80) == 0x80 {
            true => Presence::Present,
            false => Presence::Missing,
        }
    }

    pub(super) fn privilege_level(&self) -> PrivilegeLevel {
        match (self.attributes >> 5) & 0b11 {
            0 => PrivilegeLevel::Ring0,
            1 => PrivilegeLevel::Ring1,
            2 => PrivilegeLevel::Ring2,
            3 => PrivilegeLevel::Ring3,
            _ => panic!("Invalid privilege level"),
        }
    }

    pub(super) fn gate_type(&self) -> GateType {
        match self.attributes & 0b1111 {
            0xE => GateType::Interrupt,
            0xF => GateType::Trap,
            _ => panic!("Invalid gate type"),
        }
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

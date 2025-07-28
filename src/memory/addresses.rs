use crate::limine;
use core::{
    fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysicalAddress(usize);

#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtualAddress(usize);

impl PhysicalAddress {
    #[inline(always)]
    pub const fn value(&self) -> usize {
        self.0
    }

    #[inline(always)]
    pub const fn from(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub fn to_virtual(self) -> VirtualAddress {
        VirtualAddress(*limine::HHDM_OFFSET + self.0)
    }
}

impl From<PhysicalAddress> for usize {
    #[inline(always)]
    fn from(value: PhysicalAddress) -> Self {
        value.value()
    }
}

impl From<usize> for PhysicalAddress {
    #[inline(always)]
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl Sub<usize> for PhysicalAddress {
    type Output = PhysicalAddress;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<usize> for PhysicalAddress {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl Add<usize> for PhysicalAddress {
    type Output = PhysicalAddress;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for PhysicalAddress {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub for PhysicalAddress {
    type Output = PhysicalAddress;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for PhysicalAddress {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

impl Add for PhysicalAddress {
    type Output = PhysicalAddress;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for PhysicalAddress {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Physical")
            .field(&format_args!("{:#015x}", self.0))
            .finish()
    }
}


impl VirtualAddress {
    #[inline(always)]
    pub const fn value(&self) -> usize {
        self.0
    }

    #[inline(always)]
    pub const fn null() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub const fn from(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub fn from_physical(address: usize) -> Self {
        PhysicalAddress::from(address).to_virtual()
    }

    #[inline(always)]
    pub fn from_ptr<T: ?Sized>(ptr: *const T) -> Self {
        Self(ptr.addr())
    }

    #[inline(always)]
    pub fn to_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    #[inline(always)]
    pub unsafe fn to_physical(self) -> PhysicalAddress {
        PhysicalAddress(self.0 - *limine::HHDM_OFFSET)
    }
}

impl From<VirtualAddress> for usize {
    #[inline(always)]
    fn from(value: VirtualAddress) -> Self {
        value.value()
    }
}

impl From<usize> for VirtualAddress {
    #[inline(always)]
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl<T> From<*const T> for VirtualAddress {
    #[inline(always)]
    fn from(value: *const T) -> Self {
        Self::from_ptr(value)
    }
}

impl<T> From<*mut T> for VirtualAddress {
    #[inline(always)]
    fn from(value: *mut T) -> Self {
        Self::from_ptr(value)
    }
}

impl Sub<usize> for VirtualAddress {
    type Output = VirtualAddress;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<usize> for VirtualAddress {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl Add<usize> for VirtualAddress {
    type Output = VirtualAddress;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for VirtualAddress {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub for VirtualAddress {
    type Output = VirtualAddress;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for VirtualAddress {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

impl Add for VirtualAddress {
    type Output = VirtualAddress;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for VirtualAddress {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Virtual")
            .field(&format_args!("{:#015x}", self.0))
            .finish()
    }
}

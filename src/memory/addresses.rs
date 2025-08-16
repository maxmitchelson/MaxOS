use crate::limine;
use core::{
    fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
    usize,
};

#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysicalAddress(usize);

#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtualAddress(usize);

impl PhysicalAddress {
    pub const fn null() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub const fn value(&self) -> usize {
        self.0
    }

    #[inline(always)]
    pub const fn from(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub const fn from_u64(address: u64) -> Self {
        Self(address as usize)
    }

    #[inline(always)]
    pub const fn to_virtual(self) -> VirtualAddress {
        VirtualAddress(limine::hhdm_offset() + self.0)
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

    #[inline(always)]
    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<usize> for PhysicalAddress {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl Add<usize> for PhysicalAddress {
    type Output = PhysicalAddress;

    #[inline(always)]
    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for PhysicalAddress {
    #[inline(always)]
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub for PhysicalAddress {
    type Output = PhysicalAddress;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for PhysicalAddress {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

impl Add for PhysicalAddress {
    type Output = PhysicalAddress;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for PhysicalAddress {
    #[inline(always)]
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
        let val = Self(address);
        assert!(val.is_canonical());
        val
    }

    /// SAFETY: The `addresss` should be a valid, canonical, virtual address.
    /// if necessary, use [`VirtualAddress::sign_extend_value`] to ensure canonical form.
    #[inline(always)]
    pub const unsafe fn from_unchecked(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub const fn is_canonical(&self) -> bool {
        let last_bit_set = self.0 & 1 << 47 != 0;
        match self.sign_extension() {
            0xFFFF => last_bit_set,
            0x0000 => !last_bit_set,
            _ => false,
        }
    }

    #[inline(always)]
    pub const fn sign_extension(&self) -> u16 {
        (self.0 >> 48) as u16
    }

    #[inline]
    pub const fn sign_extend_value(value: usize) -> usize {
        let last_bit_set = value & 1 << 47 != 0;
        match last_bit_set {
            true => 0xFFFF << 48 | value,
            false => !(0xFFFF << 48) & value
        }
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
    pub const fn to_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    #[inline(always)]
    pub const unsafe fn to_physical(self) -> PhysicalAddress {
        PhysicalAddress(self.0 - limine::hhdm_offset())
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
        Self::from(value)
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

    #[inline(always)]
    fn sub(self, rhs: usize) -> Self::Output {
        Self::from(self.0 - rhs)
    }
}

impl SubAssign<usize> for VirtualAddress {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
        assert!(self.is_canonical());
    }
}

impl Add<usize> for VirtualAddress {
    type Output = VirtualAddress;

    #[inline(always)]
    fn add(self, rhs: usize) -> Self::Output {
        Self::from(self.0 + rhs)
    }
}

impl AddAssign<usize> for VirtualAddress {
    #[inline(always)]
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
        assert!(self.is_canonical())
    }
}

impl Sub for VirtualAddress {
    type Output = VirtualAddress;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::from(self.0 - rhs.0)
    }
}

impl SubAssign for VirtualAddress {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        assert!(self.is_canonical());
    }
}

impl Add for VirtualAddress {
    type Output = VirtualAddress;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self::from(self.0 + rhs.0)
    }
}

impl AddAssign for VirtualAddress {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        assert!(self.is_canonical());
    }
}

impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Virtual")
            .field(&format_args!("{:#015x}", self.0))
            .finish()
    }
}

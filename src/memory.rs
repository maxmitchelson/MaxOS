mod addresses;
pub mod frame_allocator;
pub mod paging;

pub use addresses::*;

#[inline]
pub fn is_power_of_two(value: impl Into<usize>) -> bool {
    let value = value.into();
    value != 0 && (value & (value - 1)) == 0
}

#[inline]
pub fn is_aligned(value: impl Into<usize>, alignment: usize) -> bool {
    (value.into() & (alignment - 1)) == 0
}

#[inline]
pub fn align_up<T>(value: T, alignment: usize) -> T
where
    T: Into<usize> + From<usize>,
{
    ((value.into() + alignment - 1) & !(alignment - 1)).into()
}

#[inline]
pub fn align_down<T>(value: T, alignment: usize) -> T
where
    T: Into<usize> + From<usize>,
{
    (value.into() & !(alignment - 1)).into()
}

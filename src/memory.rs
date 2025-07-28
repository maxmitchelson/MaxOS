mod addresses;
pub mod frame_allocator;
pub mod paging;

pub use addresses::*;

pub fn is_aligned<T>(value: T, alignment: usize) -> bool
where
    T: Into<usize> + From<usize>,
{
    (value.into() & (alignment - 1)) == 0
}

pub fn align_up<T>(value: T, alignment: usize) -> T
where
    T: Into<usize> + From<usize>,
{
    ((value.into() + alignment - 1) & !(alignment - 1)).into()
}

pub fn align_down<T>(value: T, alignment: usize) -> T
where
    T: Into<usize> + From<usize>,
{
    (value.into() & !(alignment - 1)).into()
}

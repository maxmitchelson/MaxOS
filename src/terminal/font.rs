use noto_sans_mono_bitmap::{self as nsmb, FontWeight, RasterHeight, RasterizedChar};

pub const STYLE: FontWeight = noto_sans_mono_bitmap::FontWeight::Bold;
pub const SIZE: RasterHeight = RasterHeight::Size20;

pub const HEIGHT: usize = SIZE.val();
pub const WIDTH: usize = nsmb::get_raster_width(STYLE, SIZE);


pub fn get_raster(ch: char) -> Option<RasterizedChar> {
    nsmb::get_raster(ch, STYLE, SIZE)
}


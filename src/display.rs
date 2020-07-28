use crate::{
    geometry::{ScreenPoint, ScreenSize},
    pd_func_caller,
};
use anyhow::Error;
use core::ptr;
use euclid::{default::Vector2D, size2};

#[derive(Clone, Debug)]
pub struct Display(*mut crankstart_sys::playdate_display);

impl Display {
    pub(crate) fn new(display: *mut crankstart_sys::playdate_display) {
        unsafe {
            DISPLAY = Self(display);
        }
    }

    pub fn get() -> Self {
        unsafe { DISPLAY.clone() }
    }

    pub fn get_size(&self) -> Result<ScreenSize, Error> {
        Ok(size2(
            pd_func_caller!((*self.0).getWidth)?,
            pd_func_caller!((*self.0).getHeight)?,
        ))
    }

    pub fn set_inverted(&self, inverted: bool) -> Result<(), Error> {
        pd_func_caller!((*self.0).setInverted, inverted as i32)
    }

    pub fn set_scale(&self, scale_factor: u32) -> Result<(), Error> {
        pd_func_caller!((*self.0).setScale, scale_factor)
    }

    pub fn set_mosaic(&self, amount: Vector2D<u32>) -> Result<(), Error> {
        pd_func_caller!((*self.0).setMosaic, amount.x, amount.y)
    }

    pub fn set_offset(&self, offset: ScreenPoint) -> Result<(), Error> {
        pd_func_caller!((*self.0).setOffset, offset.x, offset.y)
    }

    pub fn set_refresh_rate(&self, rate: f32) -> Result<(), Error> {
        pd_func_caller!((*self.0).setRefreshRate, rate)
    }

    pub fn set_flipped(&self, flipped: Vector2D<bool>) -> Result<(), Error> {
        pd_func_caller!((*self.0).setFlipped, flipped.x as i32, flipped.y as i32)
    }
}

static mut DISPLAY: Display = Display(ptr::null_mut());

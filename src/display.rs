use crate::pd_func_caller;
use anyhow::Error;

pub struct Display(*mut crankstart_sys::playdate_display);

impl Display {
    pub fn new(display: *mut crankstart_sys::playdate_display) -> Self {
        Self(display)
    }

    pub fn set_refresh_rate(&self, rate: f32) -> Result<(), Error> {
        pd_func_caller!((*self.0).setRefreshRate, rate)
    }
}

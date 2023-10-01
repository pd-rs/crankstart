use {
    crate::pd_func_caller, alloc::format, anyhow::Error, core::ptr, crankstart_sys::ctypes::c_void,
    cstr_core::CString,
};

use crankstart_sys::ctypes::c_int;
pub use crankstart_sys::PDButtons;
use crankstart_sys::PDDateTime;

static mut SYSTEM: System = System(ptr::null_mut());

#[derive(Clone, Debug)]
pub struct System(*const crankstart_sys::playdate_sys);

impl System {
    pub(crate) fn new(system: *const crankstart_sys::playdate_sys) {
        unsafe {
            SYSTEM = Self(system);
        }
    }

    pub fn get() -> Self {
        unsafe { SYSTEM.clone() }
    }

    pub(crate) fn realloc(&self, ptr: *mut c_void, size: usize) -> *mut c_void {
        unsafe {
            let realloc_fn = (*self.0).realloc.expect("realloc");
            realloc_fn(ptr, size)
        }
    }

    pub fn set_update_callback(&self, f: crankstart_sys::PDCallbackFunction) -> Result<(), Error> {
        pd_func_caller!((*self.0).setUpdateCallback, f, ptr::null_mut())
    }

    pub fn get_button_state(&self) -> Result<(PDButtons, PDButtons, PDButtons), Error> {
        let mut current: PDButtons = PDButtons(0);
        let mut pushed: PDButtons = PDButtons(0);
        let mut released: PDButtons = PDButtons(0);
        pd_func_caller!(
            (*self.0).getButtonState,
            &mut current,
            &mut pushed,
            &mut released
        )?;
        Ok((current, pushed, released))
    }

    pub fn is_crank_docked(&self) -> Result<bool, Error> {
        let docked: bool = pd_func_caller!((*self.0).isCrankDocked)? != 0;
        Ok(docked)
    }
    pub fn get_crank_angle(&self) -> Result<f32, Error> {
        pd_func_caller!((*self.0).getCrankAngle,)
    }

    pub fn get_crank_change(&self) -> Result<f32, Error> {
        pd_func_caller!((*self.0).getCrankChange,)
    }

    pub fn set_crank_sound_disabled(&self, disable: bool) -> Result<bool, Error> {
        let last = pd_func_caller!((*self.0).setCrankSoundsDisabled, disable as i32)?;
        Ok(last != 0)
    }

    pub fn set_auto_lock_disabled(&self, disable: bool) -> Result<(), Error> {
        pd_func_caller!((*self.0).setAutoLockDisabled, disable as i32)
    }

    pub fn log_to_console(text: &str) {
        unsafe {
            if SYSTEM.0 != ptr::null_mut() {
                if let Ok(c_text) = CString::new(text) {
                    let log_to_console_fn = (*SYSTEM.0).logToConsole.expect("logToConsole");
                    log_to_console_fn(c_text.as_ptr() as *mut crankstart_sys::ctypes::c_char);
                }
            }
        }
    }

    pub fn log_to_console_raw(text: &str) {
        unsafe {
            if SYSTEM.0 != ptr::null_mut() {
                let log_to_console_fn = (*SYSTEM.0).logToConsole.expect("logToConsole");
                log_to_console_fn(text.as_ptr() as *mut crankstart_sys::ctypes::c_char);
            }
        }
    }

    pub fn error(text: &str) {
        unsafe {
            if SYSTEM.0 != ptr::null_mut() {
                if let Ok(c_text) = CString::new(text) {
                    let error_fn = (*SYSTEM.0).error.expect("error");
                    error_fn(c_text.as_ptr() as *mut crankstart_sys::ctypes::c_char);
                }
            }
        }
    }

    pub fn error_raw(text: &str) {
        unsafe {
            if SYSTEM.0 != ptr::null_mut() {
                let error_fn = (*SYSTEM.0).error.expect("error");
                error_fn(text.as_ptr() as *mut crankstart_sys::ctypes::c_char);
            }
        }
    }

    pub fn get_seconds_since_epoch(&self) -> Result<(usize, usize), Error> {
        let mut miliseconds = 0;
        let seconds = pd_func_caller!((*self.0).getSecondsSinceEpoch, &mut miliseconds)?;
        Ok((seconds as usize, miliseconds as usize))
    }

    pub fn get_current_time_milliseconds(&self) -> Result<usize, Error> {
        Ok(pd_func_caller!((*self.0).getCurrentTimeMilliseconds)? as usize)
    }

    pub fn get_timezone_offset(&self) -> Result<i32, Error> {
        pd_func_caller!((*self.0).getTimezoneOffset)
    }

    pub fn convert_epoch_to_datetime(&self, epoch: u32) -> Result<PDDateTime, Error> {
        let mut datetime = PDDateTime::default();
        pd_func_caller!((*self.0).convertEpochToDateTime, epoch, &mut datetime)?;
        Ok(datetime)
    }

    pub fn convert_datetime_to_epoch(&self, datetime: &mut PDDateTime) -> Result<usize, Error> {
        Ok(pd_func_caller!((*self.0).convertDateTimeToEpoch, datetime)? as usize)
    }

    pub fn should_display_24_hour_time(&self) -> Result<bool, Error> {
        Ok(pd_func_caller!((*self.0).shouldDisplay24HourTime)? != 0)
    }

    pub fn reset_elapsed_time(&self) -> Result<(), Error> {
        pd_func_caller!((*self.0).resetElapsedTime)
    }

    pub fn get_elapsed_time(&self) -> Result<f32, Error> {
        Ok(pd_func_caller!((*self.0).getElapsedTime)? as f32)
    }

    pub fn get_flipped(&self) -> Result<bool, Error> {
        Ok(pd_func_caller!((*self.0).getFlipped)? != 0)
    }

    pub fn get_reduced_flashing(&self) -> Result<bool, Error> {
        Ok(pd_func_caller!((*self.0).getReduceFlashing)? != 0)
    }

    pub fn draw_fps(&self, x: i32, y: i32) -> Result<(), Error> {
        pd_func_caller!((*self.0).drawFPS, x, y)
    }

    pub fn get_battery_percentage(&self) -> Result<f32, Error> {
        pd_func_caller!((*self.0).getBatteryPercentage)
    }

    pub fn get_battery_voltage(&self) -> Result<f32, Error> {
        pd_func_caller!((*self.0).getBatteryVoltage)
    }
}

use {
    crate::pd_func_caller, alloc::format, anyhow::Error, core::ptr, crankstart_sys::ctypes::c_void,
    cstr_core::CString,
};

use crankstart_sys::ctypes::c_int;
use core::mem;

pub use crankstart_sys::PDButtons;
use crankstart_sys::{PDDateTime, PDLanguage, PDPeripherals, PDMenuItem};

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

    pub fn add_menu_item(&self, title: &str, f: crankstart_sys::PDMenuItemCallbackFunction) -> Result<MenuItem, Error> {
        let c_title = CString::new(title).map_err(Error::msg)?;
        let item = pd_func_caller!(
            (*self.0).addMenuItem, 
            c_title.as_ptr(), 
            f, 
            ptr::null_mut()
        )?;
        Ok(MenuItem(item))
    }

    pub fn add_checkmark_menu_item(&self, title: &str, initially_checked: bool, f: crankstart_sys::PDMenuItemCallbackFunction) -> Result<MenuItem, Error> {
        let c_title = CString::new(title).map_err(Error::msg)?;
        let item = pd_func_caller!(
            (*self.0).addCheckmarkMenuItem, 
            c_title.as_ptr(), 
            if initially_checked { 1 } else { 0 },
            f, 
            ptr::null_mut()
        )?;
        Ok(MenuItem(item))
    }

    pub fn add_options_menu_item(&self, title: &str, options: Vec<&str>, f: crankstart_sys::PDMenuItemCallbackFunction) -> Result<MenuItem, Error> {
        let c_title = CString::new(title).map_err(Error::msg)?;

        let mut c_options = Vec::with_capacity(options.len());
        for option in options {
            let c_option = CString::new(option).map_err(Error::msg)?;
            let c_option_ptr = c_option.as_ptr();
            // Here, we need to forget our values or they won't live long enough
            // for Playdate OS to use them
            mem::forget(c_option);
            c_options.push(
                c_option_ptr
            )
        }

        let opt_ptr = c_options.as_mut_ptr();
        let opt_len = c_options.len();
        let opt_len_i32: i32 = opt_len.try_into().map_err(Error::msg)?;

        let item = pd_func_caller!(
            (*self.0).addOptionsMenuItem, 
            c_title.as_ptr(),
            opt_ptr,
            opt_len_i32,
            f,
            ptr::null_mut()
        )?;

        // After the call, we manually drop our forgotten values so as to not
        // leak memory.
        for c_option in c_options {
            mem::drop(c_option);
        }

        Ok(MenuItem(item))
    }

    pub fn remove_all_menu_items (&self) -> Result<(), Error> {
        pd_func_caller!((*self.0).removeAllMenuItems)
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

    pub fn set_peripherals_enabled(&self, peripherals: PDPeripherals) -> Result<(), Error> {
        pd_func_caller!((*self.0).setPeripheralsEnabled, peripherals)
    }

    pub fn get_accelerometer(&self) -> Result<(f32, f32, f32), Error> {
        let mut outx = 0.0;
        let mut outy = 0.0;
        let mut outz = 0.0;
        pd_func_caller!((*self.0).getAccelerometer, &mut outx, &mut outy, &mut outz)?;
        Ok((outx, outy, outz))
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
            if !SYSTEM.0.is_null() {
                if let Ok(c_text) = CString::new(text) {
                    let log_to_console_fn = (*SYSTEM.0).logToConsole.expect("logToConsole");
                    log_to_console_fn(c_text.as_ptr() as *mut crankstart_sys::ctypes::c_char);
                }
            }
        }
    }

    pub fn log_to_console_raw(text: &str) {
        unsafe {
            if !SYSTEM.0.is_null() {
                let log_to_console_fn = (*SYSTEM.0).logToConsole.expect("logToConsole");
                log_to_console_fn(text.as_ptr() as *mut crankstart_sys::ctypes::c_char);
            }
        }
    }

    pub fn error(text: &str) {
        unsafe {
            if !SYSTEM.0.is_null() {
                if let Ok(c_text) = CString::new(text) {
                    let error_fn = (*SYSTEM.0).error.expect("error");
                    error_fn(c_text.as_ptr() as *mut crankstart_sys::ctypes::c_char);
                }
            }
        }
    }

    pub fn error_raw(text: &str) {
        unsafe {
            if !SYSTEM.0.is_null() {
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
        pd_func_caller!((*self.0).getElapsedTime)
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

    pub fn get_language(&self) -> Result<PDLanguage, Error> {
        pd_func_caller!((*self.0).getLanguage)
    }
}

#[derive(Clone, Debug)]
pub struct MenuItem(*mut crankstart_sys::PDMenuItem);

impl MenuItem {
    pub fn remove (&self) -> Result<(), Error> {
        let system = System::get();
        pd_func_caller!((*system.0).removeMenuItem, self.0)
    }

    pub fn get_title (&self) -> Result<String, Error> {
        let system = System::get();
        let c_title = pd_func_caller!((*system.0).getMenuItemTitle, self.0)?;
        let title = unsafe {
            CStr::from_ptr(c_title).to_string_lossy().into_owned()
        };
        Ok(title)
    }

    pub fn set_title (&self, new_title: &str) -> Result<(), Error> {
        let system = System::get();
        let c_title = CString::new(new_title).map_err(Error::msg)?;
        pd_func_caller!((*system.0).setMenuItemTitle, self.0, c_title.as_ptr())
    }

    pub fn get_value (&self) -> Result<i32, Error> {
        let system = System::get();
        pd_func_caller!((*system.0).getMenuItemValue, self.0)
    }

    pub fn set_value (&self, new_value: i32) -> Result<(), Error> {
        let system = System::get();
        pd_func_caller!((*system.0).setMenuItemValue, self.0, new_value)
    }

    // For checkmark menu items
    pub fn get_checked (&self) -> Result<bool, Error> {
        Ok(self.get_value()? == 1)
    }

    pub fn set_checked (&self, new_value: bool) -> Result<(), Error> {
        self.set_value(if new_value { 1 } else { 0 })
    }
}

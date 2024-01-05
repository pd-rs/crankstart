use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

use anyhow::anyhow;

use crankstart_sys::ctypes::{c_char, c_int};
pub use crankstart_sys::PDButtons;
use crankstart_sys::{PDDateTime, PDLanguage, PDMenuItem, PDPeripherals};
use {
    crate::pd_func_caller, anyhow::Error, core::ptr, crankstart_sys::ctypes::c_void,
    cstr_core::CString,
};

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

    extern "C" fn menu_item_callback(user_data: *mut core::ffi::c_void) {
        unsafe {
            let callback = user_data as *mut Box<dyn Fn()>;
            (*callback)()
        }
    }

    /// Adds a option to the menu. The callback is called when the option is selected.
    pub fn add_menu_item(&self, title: &str, callback: Box<dyn Fn()>) -> Result<MenuItem, Error> {
        let c_text = CString::new(title).map_err(|e| anyhow!("CString::new: {}", e))?;
        let wrapped_callback = Box::new(callback);
        let raw_callback_ptr = Box::into_raw(wrapped_callback);
        let raw_menu_item = pd_func_caller!(
            (*self.0).addMenuItem,
            c_text.as_ptr() as *mut core::ffi::c_char,
            Some(Self::menu_item_callback),
            raw_callback_ptr as *mut c_void
        )?;
        Ok(MenuItem {
            inner: Rc::new(RefCell::new(MenuItemInner {
                item: raw_menu_item,
                raw_callback_ptr,
            })),
            kind: MenuItemKind::Normal,
        })
    }

    /// Adds a option to the menu that has a checkbox. The initial_checked_state is the initial
    /// state of the checkbox. Callback will only be called when the menu is closed, not when the
    /// option is toggled. Use `System::get_menu_item_value` to get the state of the checkbox when
    /// the callback is called.
    pub fn add_checkmark_menu_item(
        &self,
        title: &str,
        initial_checked_state: bool,
        callback: Box<dyn Fn()>,
    ) -> Result<MenuItem, Error> {
        let c_text = CString::new(title).map_err(|e| anyhow!("CString::new: {}", e))?;
        let wrapped_callback = Box::new(callback);
        let raw_callback_ptr = Box::into_raw(wrapped_callback);
        let raw_menu_item = pd_func_caller!(
            (*self.0).addCheckmarkMenuItem,
            c_text.as_ptr() as *mut core::ffi::c_char,
            initial_checked_state as c_int,
            Some(Self::menu_item_callback),
            raw_callback_ptr as *mut c_void
        )?;

        Ok(MenuItem {
            inner: Rc::new(RefCell::new(MenuItemInner {
                item: raw_menu_item,
                raw_callback_ptr,
            })),
            kind: MenuItemKind::Checkmark,
        })
    }

    /// Adds a option to the menu that has multiple values that can be cycled through. The initial
    /// value is the first element in `options`. Callback will only be called when the menu is
    /// closed, not when the option is toggled. Use `System::get_menu_item_value` to get the index
    /// of the options list when the callback is called, which can be used to lookup the value.
    pub fn add_options_menu_item(
        &self,
        title: &str,
        options: Vec<String>,
        callback: Box<dyn Fn()>,
    ) -> Result<MenuItem, Error> {
        let c_text = CString::new(title).map_err(|e| anyhow!("CString::new: {}", e))?;
        let options_count = options.len() as c_int;
        let c_options: Vec<CString> = options
            .iter()
            .map(|s| CString::new(s.clone()).map_err(|e| anyhow!("CString::new: {}", e)))
            .collect::<Result<Vec<CString>, Error>>()?;
        let c_options_ptrs: Vec<*const i8> = c_options.iter().map(|c| c.as_ptr()).collect();
        let c_options_ptrs_ptr = c_options_ptrs.as_ptr();
        let option_titles = c_options_ptrs_ptr as *mut *const c_char;
        let wrapped_callback = Box::new(callback);
        let raw_callback_ptr = Box::into_raw(wrapped_callback);
        let raw_menu_item = pd_func_caller!(
            (*self.0).addOptionsMenuItem,
            c_text.as_ptr() as *mut core::ffi::c_char,
            option_titles,
            options_count,
            Some(Self::menu_item_callback),
            raw_callback_ptr as *mut c_void
        )?;
        Ok(MenuItem {
            inner: Rc::new(RefCell::new(MenuItemInner {
                item: raw_menu_item,
                raw_callback_ptr,
            })),
            kind: MenuItemKind::Options(options),
        })
    }

    /// Returns the state of a given menu item. The meaning depends on the type of menu item.
    /// If it is the checkbox, the int represents the boolean checked state. If it's a option the
    /// int represents the index of the option array.
    pub fn get_menu_item_value(&self, item: &MenuItem) -> Result<usize, Error> {
        let value = pd_func_caller!((*self.0).getMenuItemValue, item.inner.borrow().item)?;
        Ok(value as usize)
    }

    /// set the value of a given menu item. The meaning depends on the type of menu item. Picking
    /// the right value is left up to the caller, but is protected by the `MenuItemKind` of the
    /// `item` passed
    pub fn set_menu_item_value(&self, item: &MenuItem, new_value: usize) -> Result<(), Error> {
        match &item.kind {
            MenuItemKind::Normal => {}
            MenuItemKind::Checkmark => {
                if new_value > 1 {
                    return Err(anyhow!(
                        "Invalid value ({}) for checkmark menu item",
                        new_value
                    ));
                }
            }
            MenuItemKind::Options(opts) => {
                if new_value >= opts.len() {
                    return Err(anyhow!(
                        "Invalid value ({}) for options menu item, must be between 0 and {}",
                        new_value,
                        opts.len() - 1
                    ));
                }
            }
        }
        pd_func_caller!(
            (*self.0).setMenuItemValue,
            item.inner.borrow().item,
            new_value as c_int
        )
    }

    /// Set the title of a given menu item
    pub fn set_menu_item_title(&self, item: &MenuItem, new_title: &str) -> Result<(), Error> {
        let c_text = CString::new(new_title).map_err(|e| anyhow!("CString::new: {}", e))?;
        pd_func_caller!(
            (*self.0).setMenuItemTitle,
            item.inner.borrow().item,
            c_text.as_ptr() as *mut c_char
        )
    }
    pub fn remove_menu_item(&self, item: MenuItem) -> Result<(), Error> {
        // Explicitly drops item. The actual calling of the removeMenuItem
        // (via `remove_menu_item_internal`) is done in the drop impl to avoid calling it multiple
        // times, even though that's been experimentally shown to be safe.
        drop(item);
        Ok(())
    }
    fn remove_menu_item_internal(&self, item_inner: &MenuItemInner) -> Result<(), Error> {
        pd_func_caller!((*self.0).removeMenuItem, item_inner.item)
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

/// The kind of menu item. See `System::add_{,checkmark_,options_}menu_item` for more details.
pub enum MenuItemKind {
    Normal,
    Checkmark,
    Options(Vec<String>),
}

pub struct MenuItemInner {
    item: *mut PDMenuItem,
    raw_callback_ptr: *mut Box<dyn Fn()>,
}

impl Drop for MenuItemInner {
    fn drop(&mut self) {
        // We must remove the menu item on drop to avoid a memory or having the firmware read
        // unmanaged memory.
        System::get().remove_menu_item_internal(self).unwrap();
        unsafe {
            // Recast into box to let Box deal with freeing the right memory
            let _ = Box::from_raw(self.raw_callback_ptr);
        }
    }
}

pub struct MenuItem {
    inner: Rc<RefCell<MenuItemInner>>,
    pub kind: MenuItemKind,
}

use {
    crate::pd_func_caller,
    alloc::string::String,
    anyhow::{anyhow, Error},
    core::ptr,
    crankstart_sys::{ctypes, lua_CFunction},
    cstr_core::{CStr, CString},
};

static mut LUA: Lua = Lua(ptr::null_mut());

#[derive(Clone, Debug)]
pub struct Lua(*const crankstart_sys::playdate_lua);

impl Lua {
    pub(crate) fn new(file: *const crankstart_sys::playdate_lua) {
        unsafe {
            LUA = Lua(file);
        }
    }

    pub fn get() -> Self {
        unsafe { LUA.clone() }
    }

    pub fn add_function(&self, f: lua_CFunction, name: &str) -> Result<(), Error> {
        let c_name = CString::new(name).map_err(Error::msg)?;
        let mut out_err: *const crankstart_sys::ctypes::c_char = ptr::null_mut();
        pd_func_caller!((*self.0).addFunction, f, c_name.as_ptr(), &mut out_err)?;
        if !out_err.is_null() {
            let err_msg = unsafe { CStr::from_ptr(out_err).to_string_lossy().into_owned() };
            Err(anyhow!(err_msg))
        } else {
            Ok(())
        }
    }

    pub fn get_arg_string(&self, pos: i32) -> Result<String, Error> {
        let c_arg_string = pd_func_caller!((*self.0).getArgString, pos as ctypes::c_int)?;
        unsafe {
            let arg_string = CStr::from_ptr(c_arg_string).to_string_lossy().into_owned();
            Ok(arg_string)
        }
    }
}

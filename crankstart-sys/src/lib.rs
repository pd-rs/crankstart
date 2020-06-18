#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod ctypes {
    pub type c_ulong = u64;
    pub type c_int = i32;
    pub type c_char = i8;
    pub type c_uint = u32;
    pub type c_void = core::ffi::c_void;
    pub type realloc_size = u64;
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
pub mod ctypes {
    pub type c_ulong = u64;
    pub type c_int = i32;
    pub type c_char = u8;
    pub type c_uchar = u8;
    pub type c_uint = u32;
    pub type c_ushort = u16;
    pub type c_short = i16;
    pub type c_void = core::ffi::c_void;
    pub type realloc_size = u32;
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
include!("bindings_x86.rs");
#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
include!("bindings_arm.rs");

use {
    crate::{log_to_console, pd_func_caller, pd_func_caller_log},
    alloc::{boxed::Box, format, string::String, vec::Vec},
    anyhow::{ensure, Error},
    core::ptr,
    crankstart_sys::{ctypes::c_void, size_t, FileOptions, PDButtons, SDFile},
    cstr_core::CStr,
    cstr_core::CString,
};

pub use crankstart_sys::FileStat;

fn ensure_filesystem_success(result: i32, function_name: &str) -> Result<(), Error> {
    if result < 0 {
        let file_sys = FileSystem::get();
        let err_result = pd_func_caller!((*file_sys.0).geterr)?;
        let err_string = unsafe { CStr::from_ptr(err_result) };

        Err(Error::msg(format!(
            "Error {} from {}: {:?}",
            result, function_name, err_string
        )))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct FileSystem(*const crankstart_sys::playdate_file);

extern "C" fn list_files_callback(
    filename: *const crankstart_sys::ctypes::c_char,
    userdata: *mut core::ffi::c_void,
) {
    unsafe {
        let path = CStr::from_ptr(filename).to_string_lossy().into_owned();
        let files_ptr: *mut Vec<String> = userdata as *mut Vec<String>;
        (*files_ptr).push(path);
    }
}

impl FileSystem {
    pub(crate) fn new(file: *const crankstart_sys::playdate_file) {
        unsafe {
            FILE_SYSTEM = FileSystem(file);
        }
    }

    pub fn get() -> Self {
        unsafe { FILE_SYSTEM.clone() }
    }

    pub fn listfiles(&self, path: &str) -> Result<Vec<String>, Error> {
        let mut files: Box<Vec<String>> = Box::new(Vec::new());
        let files_ptr: *mut Vec<String> = &mut *files;
        let c_path = CString::new(path).map_err(Error::msg)?;
        let result = pd_func_caller!(
            (*self.0).listfiles,
            c_path.as_ptr(),
            Some(list_files_callback),
            files_ptr as *mut core::ffi::c_void,
        )?;
        ensure_filesystem_success(result, "listfiles")?;
        Ok(*files)
    }

    pub fn stat(&self, path: &str) -> Result<FileStat, Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let mut file_stat = FileStat::default();
        let result = pd_func_caller!((*self.0).stat, c_path.as_ptr(), &mut file_stat)?;
        ensure_filesystem_success(result, "stat")?;
        Ok(file_stat)
    }

    pub fn mkdir(&self, path: &str) -> Result<(), Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let result = pd_func_caller!((*self.0).mkdir, c_path.as_ptr())?;
        ensure_filesystem_success(result, "mkdir")?;
        Ok(())
    }

    pub fn unlink(&self, path: &str, recursive: bool) -> Result<(), Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let result = pd_func_caller!((*self.0).unlink, c_path.as_ptr(), recursive as i32)?;
        ensure_filesystem_success(result, "unlink")?;
        Ok(())
    }

    pub fn rename(&self, from_path: &str, to_path: &str) -> Result<(), Error> {
        let c_from_path = CString::new(from_path).map_err(Error::msg)?;
        let c_to_path = CString::new(to_path).map_err(Error::msg)?;
        let result = pd_func_caller!((*self.0).rename, c_from_path.as_ptr(), c_to_path.as_ptr())?;
        ensure_filesystem_success(result, "rename")?;
        Ok(())
    }

    pub fn open(&self, path: &str, options: FileOptions) -> Result<File, Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let raw_file = pd_func_caller!((*self.0).open, c_path.as_ptr(), options)?;
        ensure!(
            raw_file != ptr::null_mut(),
            "Failed to open file at {} with options {:?}",
            path,
            options
        );
        Ok(File(raw_file))
    }

    pub fn read_file_as_string(&self, path: &str) -> Result<String, Error> {
        let stat = self.stat(path)?;
        let mut buffer = Vec::with_capacity(stat.size as usize);
        buffer.resize(stat.size as usize, 0);
        let sd_file = self.open(path, FileOptions::kFileRead | FileOptions::kFileReadData)?;
        sd_file.read(&mut buffer)?;
        Ok(String::from_utf8(buffer).map_err(Error::msg)?)
    }
}

static mut FILE_SYSTEM: FileSystem = FileSystem(ptr::null_mut());

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum Whence {
    Set = crankstart_sys::SEEK_SET as i32,
    Cur = crankstart_sys::SEEK_CUR as i32,
    End = crankstart_sys::SEEK_END as i32,
}

#[derive(Debug)]
pub struct File(*mut SDFile);

impl File {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let file_sys = FileSystem::get();
        let sd_file = self.0;
        let result = pd_func_caller!(
            (*file_sys.0).read,
            sd_file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            buf.len() as u32
        )?;
        ensure_filesystem_success(result, "read")?;
        Ok(result as usize)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, Error> {
        let file_sys = FileSystem::get();
        let sd_file = self.0;
        let result = pd_func_caller!(
            (*file_sys.0).write,
            sd_file,
            buf.as_ptr() as *mut core::ffi::c_void,
            buf.len() as u32
        )?;
        ensure_filesystem_success(result, "write")?;
        Ok(result as usize)
    }

    pub fn flush(&self) -> Result<(), Error> {
        let file_sys = FileSystem::get();
        let sd_file = self.0;
        let result = pd_func_caller!((*file_sys.0).flush, sd_file)?;
        ensure_filesystem_success(result, "flush")?;
        Ok(())
    }

    pub fn tell(&self) -> Result<i32, Error> {
        let file_sys = FileSystem::get();
        let sd_file = self.0;
        let result = pd_func_caller!((*file_sys.0).tell, sd_file)?;
        ensure_filesystem_success(result, "tell")?;
        Ok(result)
    }

    pub fn seek(&self, pos: i32, whence: Whence) -> Result<(), Error> {
        let file_sys = FileSystem::get();
        let sd_file = self.0;
        let result = pd_func_caller!((*file_sys.0).seek, sd_file, pos, whence as i32)?;
        ensure_filesystem_success(result, "seek")?;
        Ok(())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        let file_sys = FileSystem::get();
        let sd_file = self.0;
        pd_func_caller_log!((*file_sys.0).close, sd_file);
    }
}

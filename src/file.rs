use {
    crate::{log_to_console, pd_func_caller},
    alloc::{format, string::String, vec::Vec},
    anyhow::{ensure, Error},
    core::ptr,
    crankstart_sys::{ctypes::c_void, size_t, FileOptions, PDButtons, SDFile},
    cstr_core::CString,
};

pub use crankstart_sys::FileStat;

#[derive(Clone, Debug)]
pub struct FileSystem(*mut crankstart_sys::playdate_file);

impl FileSystem {
    pub fn new(file: *mut crankstart_sys::playdate_file) -> Self {
        Self(file)
    }

    pub fn open(&self, path: &str, options: FileOptions) -> Result<File, Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let raw_file = pd_func_caller!((*self.0).open, c_path.as_ptr(), options)?;
        Ok(File(self.clone(), raw_file))
    }

    pub fn stat(&self, path: &str) -> Result<FileStat, Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let mut file_stat = FileStat::default();
        let result = pd_func_caller!((*self.0).stat, c_path.as_ptr(), &mut file_stat)?;
        ensure!(result == 0, "Error: {} from stat", result);
        Ok(file_stat)
    }

    pub fn read_file_as_string(&self, path: &str) -> Result<String, Error> {
        let stat = self.stat(path)?;
        log_to_console!("stat = {:#?}", stat);
        let mut buffer = Vec::with_capacity(stat.size as usize);
        buffer.resize(stat.size as usize, 0);
        let sd_file = self.open(path, FileOptions::kFileRead | FileOptions::kFileReadData)?;
        sd_file.read(&mut buffer)?;
        Ok(String::from_utf8(buffer).map_err(Error::msg)?)
    }
}

pub struct File(FileSystem, *mut SDFile);

impl File {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let file_sys = &self.0;
        let sd_file = self.1;
        let result = pd_func_caller!(
            (*file_sys.0).read,
            sd_file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            buf.len() as u32
        )?;
        ensure!(result >= 0, "Error {} from read", result);
        Ok(result as usize)
    }
}

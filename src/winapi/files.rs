use std::ffi::{OsStr, OsString};
use std::fs::{remove_file, File};
use std::io::Error as IoError;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use windows::core::{Error, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, HANDLE};
use windows::Win32::Storage::FileSystem::{
    FindClose, FindFirstFileW, FindNextFileW, FILE_SHARE_READ, WIN32_FIND_DATAW,
};

pub struct Files;

impl Files {
    pub fn find(path: &OsStr) -> impl Iterator<Item = Result<OsString, Error>> {
        let mut find_data = WIN32_FIND_DATAW::default();
        let mut buf: Vec<u16> = path.encode_wide().collect();
        buf.push(0); // null-terminate
                     // SAFETY: find_data must not be used if the function returns error
        let result = unsafe { FindFirstFileW(PCWSTR(buf.as_ptr()), &mut find_data) };
        result.map_or_else(
            |err| FileIter::with_error(err),
            |handle| FileIter {
                handle,
                find_data,
                last_error: None,
            },
        )
    }

    pub fn delete(path: &OsStr) -> Result<(), IoError> {
        remove_file(path)
    }

    pub fn create(path: &OsStr) -> Result<File, IoError> {
        File::options()
            .create_new(true)
            .write(true)
            .share_mode(FILE_SHARE_READ.0)
            .open(&path)
    }
}

struct FileIter {
    last_error: Option<Error>,
    handle: HANDLE,
    find_data: WIN32_FIND_DATAW,
}

impl FileIter {
    pub fn with_error(error: Error) -> Self {
        FileIter {
            last_error: Some(error),
            handle: HANDLE::default(),
            find_data: WIN32_FIND_DATAW::default(),
        }
    }
}

impl Iterator for FileIter {
    type Item = Result<OsString, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(err) = self.last_error.clone() {
            if err == Error::from(ERROR_FILE_NOT_FOUND) || err == Error::from(ERROR_NO_MORE_FILES) {
                return None;
            }
            return Some(Err(err));
        }
        // copy the string out of find_data before calling FindNextFile
        let len = self
            .find_data
            .cFileName
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(self.find_data.cFileName.len());
        let file = OsString::from_wide(&self.find_data.cFileName[0..len]);
        // SAFETY: find_data must not be used if the function returns error
        self.last_error = unsafe { FindNextFileW(self.handle, &mut self.find_data) }.err();
        Some(Ok(file))
    }
}

impl Drop for FileIter {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            // SAFETY: We ensure that the handle is valid and FindClose is not called twice
            let result = unsafe { FindClose(self.handle) };
            if let Err(err) = result {
                error!("Failed to close the find operation: {}", err);
            }
        }
    }
}

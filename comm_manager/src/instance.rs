use std::{fs::File, io::{Read, Write}, os::windows::io::FromRawHandle};

use windows::Win32::Foundation::HANDLE;

pub struct CommManagerInstance {
    read_file: File,
    write_file: File,
}

impl CommManagerInstance {
    pub fn new(read_handle: HANDLE, write_handle: HANDLE) -> Self {
        unsafe {
            let read_file = File::from_raw_handle(read_handle.0 as _);
            let write_file = File::from_raw_handle(write_handle.0 as _);
            Self {
                read_file,
                write_file,
            }
        }
    }
}

impl Read for CommManagerInstance {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read_file.read(buf)
    }
}

impl Write for CommManagerInstance {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.write_file.flush()
    }
}

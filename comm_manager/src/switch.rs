use std::{fs::File, io::{ErrorKind, Read}, os::windows::io::FromRawHandle};

use windows::{core::HSTRING, Win32::{Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE}, Storage::FileSystem::{CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_NONE, OPEN_EXISTING}}};

use crate::instance::CommManagerInstance;
use std::io::Write;

pub struct CommManagerSwitch {
    pipe_file: File,
}

impl CommManagerSwitch {
    pub fn new() -> Option<Self> {
        let pipe_id = std::env::var("SITE_COMM_MANAGER_PIPE").ok();

        if pipe_id.is_none() {
            return None;
        }

        let pipe_id = pipe_id.unwrap();

        let pipe_name = format!(r"\\.\pipe\{}", pipe_id);

        let pipe_handle = unsafe {
            CreateFileW(
                &HSTRING::from(pipe_name),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            ).ok()?
        };
        let pipe_file = unsafe { File::from_raw_handle(pipe_handle.0 as _) };

        Some(Self { pipe_file })
    }

    pub fn acquire(&mut self, uuid: &[u8]) -> std::io::Result<CommManagerInstance> {

        if uuid.len() != 16 {
            return Err(std::io::Error::from(ErrorKind::InvalidInput));
        }

        self.pipe_file.write_all(uuid)?;

        let result = {
            let mut buffer = [0u8; 4];
            self.pipe_file.read_exact(&mut buffer)?;
            u32::from_le_bytes(buffer)
        };

        if result != 0 {
            return Err(std::io::Error::from(ErrorKind::Other));
        }

        let read_handle = {
            let mut buffer = [0u8; 8];
            self.pipe_file.read_exact(&mut buffer)?;
            HANDLE(isize::from_le_bytes(buffer))
        };

        let write_handle = {
            let mut buffer = [0u8; 8];
            self.pipe_file.read_exact(&mut buffer)?;
            HANDLE(isize::from_le_bytes(buffer))
        };

        Ok(CommManagerInstance::new(read_handle, write_handle))
    }
}
